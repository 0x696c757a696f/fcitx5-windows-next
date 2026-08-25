#include "fcitx_dispatcher.h"
#include "fcitx_runtime.h"
#include "engine_core_ffi.h"
#include "peer_verification.h"
#include "pipe_security.h"
#include "presentation_publisher.h"
#include "protocol.h"
#include "runtime_identity.h"

#include <Windows.h>

#include <array>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <filesystem>
#include <iostream>
#include <span>
#include <string>
#include <thread>
#include <vector>

namespace {

using namespace fcitx::windows;

std::string jsonString(std::string_view value) {
    std::string output = "\"";
    for (const unsigned char character : value) {
        if (character == '\\' || character == '"')
            output.push_back('\\');
        if (character < 0x20U)
            return {};
        output.push_back(static_cast<char>(character));
    }
    output.push_back('"');
    return output;
}

bool transfer(HANDLE pipe, bool write, void* data, std::size_t size, DWORD timeout,
              HANDLE stopEvent) {
    auto* cursor = static_cast<std::uint8_t*>(data);
    std::size_t completed = 0;
    while (completed < size) {
        HANDLE event = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        if (!event)
            return false;
        OVERLAPPED operation{};
        operation.hEvent = event;
        DWORD transferred = 0;
        const DWORD requested = static_cast<DWORD>(size - completed);
        const BOOL immediate =
            write ? WriteFile(pipe, cursor + completed, requested, &transferred, &operation)
                  : ReadFile(pipe, cursor + completed, requested, &transferred, &operation);
        bool success = immediate != FALSE;
        if (!success && GetLastError() == ERROR_IO_PENDING) {
            const std::array<HANDLE, 2> waits{event, stopEvent};
            const DWORD waitResult =
                WaitForMultipleObjects(stopEvent ? 2U : 1U, waits.data(), FALSE, timeout);
            if (waitResult == WAIT_OBJECT_0) {
                success = GetOverlappedResult(pipe, &operation, &transferred, FALSE) != FALSE;
            } else {
                CancelIoEx(pipe, &operation);
                GetOverlappedResult(pipe, &operation, &transferred, TRUE);
                success = false;
            }
        }
        CloseHandle(event);
        if (!success || transferred == 0)
            return false;
        completed += transferred;
    }
    return true;
}

bool connectClient(HANDLE pipe, HANDLE stopEvent) {
    HANDLE event = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    if (!event)
        return false;
    OVERLAPPED operation{};
    operation.hEvent = event;
    const BOOL immediate = ConnectNamedPipe(pipe, &operation);
    bool connected = immediate != FALSE;
    if (!connected) {
        const DWORD error = GetLastError();
        if (error == ERROR_PIPE_CONNECTED) {
            connected = true;
        } else if (error == ERROR_IO_PENDING) {
            const std::array<HANDLE, 2> waits{event, stopEvent};
            const DWORD waitResult =
                WaitForMultipleObjects(stopEvent ? 2U : 1U, waits.data(), FALSE, 60'000);
            if (waitResult == WAIT_OBJECT_0) {
                DWORD transferred = 0;
                connected = GetOverlappedResult(pipe, &operation, &transferred, FALSE) != FALSE;
            } else {
                CancelIoEx(pipe, &operation);
                DWORD transferred = 0;
                GetOverlappedResult(pipe, &operation, &transferred, TRUE);
            }
        }
    }
    CloseHandle(event);
    return connected;
}

bool readFrame(HANDLE pipe, std::vector<std::uint8_t>& bytes, HANDLE stopEvent) {
    std::array<std::uint8_t, protocol::kHeaderSize> header{};
    if (!transfer(pipe, false, header.data(), header.size(), 60'000, stopEvent))
        return false;
    protocol::MessageType type{};
    std::uint32_t bodySize = 0;
    protocol::Metadata metadata;
    if (!protocol::decodeHeader(header, type, bodySize, metadata))
        return false;
    bytes.assign(header.begin(), header.end());
    bytes.resize(protocol::kHeaderSize + bodySize);
    return bodySize == 0 ||
           transfer(pipe, false, bytes.data() + protocol::kHeaderSize, bodySize, 100, stopEvent);
}

protocol::KeyResponse makeStateResponse(
    const protocol::Metadata& requestMetadata, std::uint64_t responseId,
    std::uint64_t engineEpoch, const engine::RuntimeResult& runtimeResult) {
    protocol::KeyResponse response;
    response.metadata = protocol::Metadata{
        responseId, requestMetadata.requestId, engineEpoch, requestMetadata.sessionId,
        requestMetadata.contextId, runtimeResult.compositionId, runtimeResult.revision};
    response.status = protocol::Status::ok;
    response.handled = runtimeResult.handled;
    response.commitUtf8 = runtimeResult.commitUtf8;
    response.preeditUtf8 = runtimeResult.preeditUtf8;
    response.preeditCaretUtf8 = runtimeResult.preeditCaretUtf8;
    response.candidates = runtimeResult.candidates;
    response.selectedCandidate = runtimeResult.selectedCandidate;
    response.candidatePage = runtimeResult.candidatePage;
    response.candidateTotal = runtimeResult.candidateTotal;
    response.candidateVisibility = runtimeResult.candidateVisibility;
    response.candidatePageSize = runtimeResult.candidatePageSize;
    response.candidateBulk = runtimeResult.candidateBulk;
    response.candidateEnd = runtimeResult.candidateEnd;
    response.deleteSurroundingText = runtimeResult.deleteSurroundingText;
    response.deleteSurroundingOffset = runtimeResult.deleteSurroundingOffset;
    response.deleteSurroundingSize = runtimeResult.deleteSurroundingSize;
    response.forwardKey = runtimeResult.forwardKey;
    response.forwardKeySym = runtimeResult.forwardKeySym;
    response.forwardKeyStates = runtimeResult.forwardKeyStates;
    response.forwardKeyCode = runtimeResult.forwardKeyCode;
    response.forwardKeyRelease = runtimeResult.forwardKeyRelease;
    response.caret = runtimeResult.caret;
    response.popupAllowed = runtimeResult.popupAllowed;
    response.contentLocaleUtf8 = runtimeResult.contentLocaleUtf8;
    return response;
}

protocol::EngineStatusResponse makeEngineStatusResponse(
    const protocol::Metadata& requestMetadata, std::uint64_t responseId,
    std::uint64_t engineEpoch, const engine::InputMethodStatus& status) {
    return protocol::EngineStatusResponse{
        protocol::Metadata{responseId, requestMetadata.requestId, engineEpoch,
                           requestMetadata.sessionId, 0, 0, 0},
        protocol::Status::ok,
        status.id,
        status.name,
        status.nativeName,
        status.shortLabel};
}

bool signalCandidateUpdate(const platform::RuntimeIdentity& identity,
                           std::uint32_t targetProcessId) noexcept {
    const std::wstring name = platform::makeLocalObjectName(
        identity, L"candidate-" + std::to_wstring(targetProcessId));
    if (name.empty()) return false;
    const HANDLE event = OpenEventW(EVENT_MODIFY_STATE, FALSE, name.c_str());
    if (!event) return false;
    const bool signaled = SetEvent(event) != FALSE;
    CloseHandle(event);
    return signaled;
}

std::vector<std::uint8_t> handleRequest(std::span<const std::uint8_t> requestBytes,
                                        std::uint64_t engineEpoch,
                                        std::atomic<std::uint64_t>& nextResponseId,
                                        const platform::ProcessIdentity& clientIdentity,
                                        std::uint64_t connectionId,
                                        void* session,
                                        engine::FcitxDispatcher& dispatcher,
                                        engine::PresentationPublisher& presentation,
                                        const platform::RuntimeIdentity& serverIdentity,
                                        const std::wstring& uiExecutable) {
    protocol::FrameView frame;
    if (!protocol::decodeFrame(requestBytes, frame)) {
        return {};
    }
    if (frame.type == protocol::MessageType::helloRequest) {
        protocol::HelloRequest request;
        // E4-3: the hello handshake is Rust-owned (repeat handshake, stale
        // request id, and session/process mismatch are rejected; on success
        // the session becomes handshake-complete and records the request id).
        if (!protocol::decode(frame, request) ||
            fcitx5_engine_core_session_begin_hello(
                session, request.metadata.requestId, request.metadata.sessionId,
                clientIdentity.sessionId, request.clientProcessId,
                clientIdentity.processId) == 0) {
            return {};
        }
        return protocol::encode(protocol::HelloResponse{
            protocol::Metadata{nextResponseId.fetch_add(1), request.metadata.requestId, engineEpoch,
                               request.metadata.sessionId, 0, 0, 0},
            protocol::Status::ok, 64});
    }
    // E4-3: every non-hello frame is validated by the Rust per-connection
    // session (handshake complete, engine-epoch match, session-id match, and
    // strictly-newer request id); the C++ handshakeComplete/lastRequestId
    // locals are deleted.
    if (fcitx5_engine_core_session_accept_frame(
            session, frame.metadata.requestId, frame.metadata.sessionId,
            clientIdentity.sessionId, frame.metadata.engineEpoch, engineEpoch) == 0) {
        return {};
    }

    if (frame.type == protocol::MessageType::keyRequest) {
        protocol::KeyRequest request;
        engine::RuntimeResult runtimeResult;
        const auto timeout = std::chrono::milliseconds(
            fcitx5_engine_core_key_request_timeout_ms(frame.metadata.revision));
        if (!protocol::decode(frame, request) ||
            !dispatcher.processKey(
                engine::ClientContextKey{clientIdentity.processId, connectionId,
                                         request.metadata.contextId},
                request, runtimeResult, timeout)) return {};
        fcitx5_engine_core_session_complete_request(session,
                                                    request.metadata.requestId);
        auto response = makeStateResponse(request.metadata, nextResponseId.fetch_add(1),
                                          engineEpoch, runtimeResult);
        presentation.publish(response);
        return protocol::encode(response);
    }

    if (frame.type == protocol::MessageType::candidateSelectRequest) {
        protocol::CandidateSelectRequest request;
        engine::RuntimeResult runtimeResult;
        if (!platform::pathsReferToSameFile(clientIdentity.executablePath, uiExecutable) ||
            !protocol::decode(frame, request) ||
            !dispatcher.selectCandidate(request.targetProcessId, request, runtimeResult,
                                        std::chrono::milliseconds(75))) return {};
        fcitx5_engine_core_session_complete_request(session,
                                                    request.metadata.requestId);
        auto state = makeStateResponse(request.metadata, nextResponseId.fetch_add(1),
                                       engineEpoch, runtimeResult);
        presentation.publish(state);
        const bool notified = signalCandidateUpdate(serverIdentity, request.targetProcessId);
        const protocol::CandidateSelectResponse response{
            protocol::Metadata{nextResponseId.fetch_add(1), request.metadata.requestId,
                               engineEpoch, request.metadata.sessionId,
                               request.metadata.contextId, runtimeResult.compositionId,
                               runtimeResult.revision},
            notified ? protocol::Status::ok : protocol::Status::unsupported};
        return protocol::encode(response);
    }

    if (frame.type == protocol::MessageType::stateRequest) {
        protocol::StateRequest request;
        engine::RuntimeResult runtimeResult;
        if (!protocol::decode(frame, request) ||
            !dispatcher.takePendingState(
                engine::ClientContextKey{clientIdentity.processId, connectionId,
                                         request.metadata.contextId},
                request, runtimeResult, std::chrono::milliseconds(75))) return {};
        fcitx5_engine_core_session_complete_request(session,
                                                    request.metadata.requestId);
        return protocol::encode(makeStateResponse(
            request.metadata, nextResponseId.fetch_add(1), engineEpoch, runtimeResult));
    }
    if (frame.type == protocol::MessageType::engineStatusRequest) {
        protocol::EngineStatusRequest request;
        engine::InputMethodStatus status;
        if (!protocol::decode(frame, request) ||
            !dispatcher.queryInputMethodStatus(status, std::chrono::milliseconds(75))) {
            return {};
        }
        fcitx5_engine_core_session_complete_request(session,
                                                    request.metadata.requestId);
        return protocol::encode(makeEngineStatusResponse(
            request.metadata, nextResponseId.fetch_add(1), engineEpoch, status));
    }
    return {};
}

int serve(const std::wstring& pipeName, unsigned testClientCount,
          const std::wstring& readyEventName, const std::wstring& stopEventName, bool safeMode) {
#if defined(_MSC_VER)
// The /analyze C6001 report below is a false positive: stopEvent is assigned
// on every path before the worker threads are created, and the threads only
// read it after that assignment.
#pragma warning(push)
#pragma warning(disable : 6001)
#endif
    platform::RuntimeIdentity identity;
    platform::PipeSecurity pipeSecurity;
    if (!platform::queryCurrentIdentity(identity) || !identity.mayUseUserEngine() ||
        !platform::PipeSecurity::create(identity, pipeSecurity)) {
        return 4;
    }
    engine::FcitxDispatcher dispatcher;
    if (const char* delay = std::getenv("FCITX5_TEST_DISPATCH_DELAY_MS"); delay) {
        dispatcher.setTestDispatchDelay(static_cast<std::uint32_t>(std::atoi(delay)));
    }
    if (!dispatcher.start(safeMode))
        return 5;
    std::wstring executable(32'768, L'\0');
    const DWORD executableSize =
        GetModuleFileNameW(nullptr, executable.data(), static_cast<DWORD>(executable.size()));
    if (executableSize == 0 || executableSize >= executable.size())
        return 6;
    executable.resize(executableSize);
    const auto uiExecutable =
        (std::filesystem::path(executable).parent_path() / "fcitx5-ui.exe").wstring();
    engine::PresentationPublisher presentation(
        platform::makeLocalEndpointName(identity, L"presentation"), uiExecutable);
    // E4: the engine-process session epoch is Rust-generated (100ns-since-1601
    // FILETIME value); the C++ shell only holds the value.
    const std::uint64_t engineEpoch = fcitx5_engine_core_generate_engine_epoch();
    std::atomic<std::uint64_t> nextResponseId{1};
    std::atomic<std::uint64_t> nextConnectionId{1};
    std::atomic<bool> readinessSignaled{};
    std::atomic<int> serverError{};
    // In --test-clients mode every worker must stop once the configured number
    // of client sessions has completed (not after the first one), so concurrent
    // multi-client stress tests can run N clients against N workers.
    std::atomic<unsigned> completedClients{0};
    HANDLE internalStopEvent = nullptr;
    HANDLE stopEvent = nullptr;
    if (testClientCount != 0) {
        internalStopEvent = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        if (!internalStopEvent) return 3;
    }
    if (!stopEventName.empty()) {
        stopEvent = OpenEventW(SYNCHRONIZE, FALSE, stopEventName.c_str());
        if (!stopEvent) {
            if (internalStopEvent) CloseHandle(internalStopEvent);
            return 3;
        }
    } else {
        stopEvent = internalStopEvent;
    }
    const unsigned workerCount = testClientCount == 0 ? 16U : testClientCount;
    std::vector<std::thread> workers;
    workers.reserve(workerCount);
    for (unsigned index = 0; index < workerCount; ++index) {
        workers.emplace_back([&] {
            for (;;) {
                if (stopEvent && WaitForSingleObject(stopEvent, 0) == WAIT_OBJECT_0)
                    return;
                HANDLE pipe = CreateNamedPipeW(
                    pipeName.c_str(), PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                    workerCount, static_cast<DWORD>(protocol::kMaxFrameSize),
                    static_cast<DWORD>(protocol::kMaxFrameSize), 25, pipeSecurity.attributes());
                if (pipe == INVALID_HANDLE_VALUE) {
                    serverError.store(2);
                    return;
                }
                if (!readinessSignaled.exchange(true) && !readyEventName.empty()) {
                    HANDLE readyEvent =
                        OpenEventW(EVENT_MODIFY_STATE, FALSE, readyEventName.c_str());
                    if (!readyEvent) {
                        CloseHandle(pipe);
                        serverError.store(3);
                        return;
                    }
                    SetEvent(readyEvent);
                    CloseHandle(readyEvent);
                }
                if (!connectClient(pipe, stopEvent)) {
                    CloseHandle(pipe);
                    if (stopEvent && WaitForSingleObject(stopEvent, 0) == WAIT_OBJECT_0)
                        return;
                    continue;
                }
                platform::ProcessIdentity clientIdentity;
                if (ipc::verifyPipeClient(pipe, identity, &clientIdentity)) {
                    const std::uint64_t connectionId =
                        nextConnectionId.fetch_add(1, std::memory_order_relaxed);
                    // E4-3: one Rust-owned session per connection
                    // (handshake completion + last accepted request id).
                    void* session = fcitx5_engine_core_session_create();
                    std::vector<std::uint8_t> request;
                    while (readFrame(pipe, request, stopEvent)) {
                        auto response = handleRequest(request, engineEpoch, nextResponseId,
                                                      clientIdentity, connectionId,
                                                      session, dispatcher, presentation,
                                                      identity, uiExecutable);
                        if (response.empty() || !transfer(pipe, true, response.data(),
                                                          response.size(), 100, stopEvent)) {
                            break;
                        }
                    }
                    fcitx5_engine_core_session_destroy(session);
                    dispatcher.forgetConnection(connectionId);
                }
                DisconnectNamedPipe(pipe);
                CloseHandle(pipe);
                if (testClientCount != 0 &&
                    completedClients.fetch_add(1, std::memory_order_acq_rel) + 1U ==
                        testClientCount) {
                    if (stopEvent) SetEvent(stopEvent);
                    return;
                }
            }
        });
    }
    for (auto& worker : workers)
        worker.join();
    if (stopEvent)
        CloseHandle(stopEvent);
    if (internalStopEvent && internalStopEvent != stopEvent)
        CloseHandle(internalStopEvent);
    dispatcher.stop();
    // Emitted for the REG-DISPATCH-LATE integration test: the count of queued
    // key requests that were dropped at their deadline check instead of being
    // executed after the caller timed out.
    std::cerr << "dispatcher-dropped=" << dispatcher.droppedCount() << '\n';
    return serverError.load();
#if defined(_MSC_VER)
#pragma warning(pop)
#endif
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
        std::cout << "fcitx5-engine 0.1.0 protocol " << fcitx::windows::protocol::kVersion << '\n';
        return 0;
    }
    if (argc == 2 && std::wstring_view(argv[1]) == L"--list-input-methods") {
        fcitx::windows::engine::FcitxRuntime runtime;
        if (!runtime.initialize(false))
            return 5;
        const auto methods = runtime.inputMethods();
        std::cout << "{\"format_version\":1,\"input_methods\":[";
        for (std::size_t index = 0; index < methods.size(); ++index) {
            if (index != 0)
                std::cout << ',';
            const auto& method = methods[index];
            std::cout << "{\"id\":" << jsonString(method.id)
                      << ",\"name\":" << jsonString(method.name)
                      << ",\"native_name\":" << jsonString(method.nativeName)
                      << ",\"selected\":" << (method.selected ? "true" : "false") << '}';
        }
        std::cout << "]}\n";
        return methods.empty() ? 6 : 0;
    }
    if (argc == 3 && std::wstring_view(argv[1]) == L"--set-input-method") {
        const int size = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, argv[2], -1, nullptr,
                                             0, nullptr, nullptr);
        if (size <= 1 || size > 66)
            return 2;
        std::string id(static_cast<std::size_t>(size), '\0');
        if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, argv[2], -1, id.data(), size,
                                nullptr, nullptr) != size)
            return 2;
        id.pop_back();
        fcitx::windows::engine::FcitxRuntime runtime;
        return runtime.initialize(false) && runtime.setDefaultInputMethod(id) ? 0 : 5;
    }
    for (int index = 1; index < argc; ++index) {
        if (std::wstring_view(argv[index]) == L"--generation" && index + 1 < argc) {
            const std::wstring generation = argv[++index];
            if (!SetEnvironmentVariableW(L"FCITX5_RELEASE_GENERATION", generation.c_str()) ||
                fcitx::windows::platform::currentRuntimeGeneration() != generation)
                return 1;
        }
    }
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity) || !identity.mayUseUserEngine()) {
        return 4;
    }
    std::wstring pipeName = fcitx::windows::platform::makeLocalEndpointName(identity, L"engine");
    std::wstring readyEventName;
    std::wstring stopEventName;
    unsigned testClientCount = 0;
    bool safeMode = false;
    for (int index = 1; index < argc; ++index) {
        const std::wstring_view argument(argv[index]);
        if (argument == L"--test-once") {
            testClientCount = 1;
        } else if (argument == L"--safe-mode") {
            safeMode = true;
        } else if (argument == L"--test-clients" && index + 1 < argc) {
            wchar_t* end = nullptr;
            const unsigned long parsed = std::wcstoul(argv[++index], &end, 10);
            if (!end || *end != L'\0' || parsed == 0 || parsed > 64)
                return 1;
            testClientCount = static_cast<unsigned>(parsed);
        } else if (argument == L"--pipe" && index + 1 < argc) {
            pipeName = argv[++index];
        } else if (argument == L"--ready-event" && index + 1 < argc) {
            readyEventName = argv[++index];
        } else if (argument == L"--stop-event" && index + 1 < argc) {
            stopEventName = argv[++index];
        } else if (argument == L"--generation" && index + 1 < argc) {
            ++index;
        } else {
            std::wcerr << L"Usage: fcitx5-engine [--test-once|--test-clients N] "
                          L"[--pipe NAME] [--ready-event NAME] [--stop-event NAME] "
                          L"[--generation GENERATION]\n";
            return 1;
        }
    }
    return serve(pipeName, testClientCount, readyEventName, stopEventName, safeMode);
}
