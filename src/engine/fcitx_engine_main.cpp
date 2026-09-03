#include "fcitx_dispatcher.h"
#include "fcitx_runtime.h"
#include "engine_core_ffi.h"
#include "peer_verification.h"
#include "pipe_security.h"
#include "protocol_ffi.h"
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

extern "C" std::uint64_t
fcitx5_windows_common_deadline_after_milliseconds(std::uint32_t milliseconds);
extern "C" std::uint8_t fcitx5_windows_common_pipe_transfer_with_stop(void* pipe,
                                                                      std::uint8_t write,
                                                                      void* data, std::size_t size,
                                                                      std::uint64_t deadline,
                                                                      void* stop_handle);
extern "C" std::uint8_t
fcitx5_windows_common_pipe_connect_client(void* pipe, std::uint64_t deadline, void* stop_handle);

namespace {

using namespace fcitx::windows;

constexpr std::size_t kWireHeaderSize = 64;
constexpr DWORD kWirePipeBufferSize = 256U * 1024U;
constexpr std::uint16_t kHelloRequestType = 1;
constexpr std::uint16_t kKeyRequestType = 3;
constexpr std::uint16_t kCandidateSelectRequestType = 7;
constexpr std::uint16_t kStateRequestType = 9;
constexpr std::uint16_t kEngineStatusRequestType = 10;
constexpr std::uint32_t kProtocolStatusOk = 0;
constexpr std::uint32_t kProtocolStatusUnsupported = 3;
constexpr std::uint16_t kWireProtocolVersion = 14;

struct WireFrameView {
    std::uint16_t type{};
    FcitxMetadataC metadata{};
    std::span<const std::uint8_t> body;
};

template <typename Message>
using DecodeMessage = std::uint8_t (*)(const FcitxMetadataC*, const std::uint8_t*, std::size_t,
                                       Message*, std::uint8_t*, std::size_t, std::size_t*);

template <typename Message>
using EncodeMessage = std::uint8_t (*)(const Message*, std::uint8_t*, std::size_t, std::size_t*);

bool decodeFrame(std::span<const std::uint8_t> bytes, WireFrameView& frame) {
    if (bytes.size() < kWireHeaderSize)
        return false;
    std::uint32_t bodySize = 0;
    if (fcitx5_protocol_core_decode_header(bytes.data(), kWireHeaderSize, &frame.type, &bodySize,
                                           &frame.metadata) == 0 ||
        bodySize != bytes.size() - kWireHeaderSize) {
        return false;
    }
    frame.body = bytes.subspan(kWireHeaderSize);
    return true;
}

template <typename Message>
bool decodeMessage(const WireFrameView& frame, DecodeMessage<Message> decode, Message& message,
                   std::vector<std::uint8_t>& strings) {
    std::size_t stringsNeeded = 0;
    if (decode(&frame.metadata, frame.body.data(), frame.body.size(), &message, nullptr, 0,
               &stringsNeeded) != 0) {
        message.metadata = frame.metadata;
        return true;
    }
    if (stringsNeeded == 0)
        return false;
    strings.resize(stringsNeeded);
    if (decode(&frame.metadata, frame.body.data(), frame.body.size(), &message, strings.data(),
               strings.size(), &stringsNeeded) == 0) {
        return false;
    }
    message.metadata = frame.metadata;
    return true;
}

template <typename Message>
std::vector<std::uint8_t> encodeMessage(EncodeMessage<Message> encode, const Message& message) {
    std::size_t bytesNeeded = 0;
    if (encode(&message, nullptr, 0, &bytesNeeded) != 0 || bytesNeeded == 0)
        return {};
    std::vector<std::uint8_t> bytes(bytesNeeded);
    if (encode(&message, bytes.data(), bytes.size(), &bytesNeeded) == 0 ||
        bytesNeeded != bytes.size()) {
        return {};
    }
    return bytes;
}

FcitxBytesC bytesFor(std::string_view value) {
    return FcitxBytesC{reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

class PresentationPublisher final {
  public:
    PresentationPublisher(std::wstring pipeName, std::wstring uiExecutable)
        : handle_(fcitx5_engine_core_presentation_publisher_create(
              reinterpret_cast<const std::uint16_t*>(pipeName.data()), pipeName.size(),
              reinterpret_cast<const std::uint16_t*>(uiExecutable.data()), uiExecutable.size())) {}
    ~PresentationPublisher() { fcitx5_engine_core_presentation_publisher_destroy(handle_); }
    PresentationPublisher(const PresentationPublisher&) = delete;
    PresentationPublisher& operator=(const PresentationPublisher&) = delete;

    bool publish(std::span<const std::uint8_t> frame) noexcept {
        return fcitx5_engine_core_presentation_publisher_publish(handle_, frame.data(),
                                                                 frame.size()) != 0;
    }

  private:
    void* handle_{};
};

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
    const std::uint64_t deadline = fcitx5_windows_common_deadline_after_milliseconds(timeout);
    return fcitx5_windows_common_pipe_transfer_with_stop(pipe, write ? 1 : 0, data, size, deadline,
                                                         stopEvent) != 0;
}

bool connectClient(HANDLE pipe, HANDLE stopEvent) {
    const std::uint64_t deadline = fcitx5_windows_common_deadline_after_milliseconds(60'000);
    return fcitx5_windows_common_pipe_connect_client(pipe, deadline, stopEvent) != 0;
}

bool readFrame(HANDLE pipe, std::vector<std::uint8_t>& bytes, HANDLE stopEvent) {
    std::array<std::uint8_t, kWireHeaderSize> header{};
    if (!transfer(pipe, false, header.data(), header.size(), 60'000, stopEvent))
        return false;
    std::uint16_t type = 0;
    std::uint32_t bodySize = 0;
    FcitxMetadataC metadata{};
    if (fcitx5_protocol_core_decode_header(header.data(), header.size(), &type, &bodySize,
                                           &metadata) == 0)
        return false;
    bytes.assign(header.begin(), header.end());
    bytes.resize(kWireHeaderSize + bodySize);
    return bodySize == 0 ||
           transfer(pipe, false, bytes.data() + kWireHeaderSize, bodySize, 100, stopEvent);
}

std::vector<std::uint8_t> encodeStateResponse(const FcitxMetadataC& requestMetadata,
                                              std::uint64_t responseId, std::uint64_t engineEpoch,
                                              const engine::RuntimeResult& runtimeResult) {
    std::vector<FcitxCandidateRecordC> candidates;
    candidates.reserve(runtimeResult.candidates.size());
    for (const auto& candidate : runtimeResult.candidates) {
        candidates.push_back(FcitxCandidateRecordC{candidate.id, bytesFor(candidate.labelUtf8),
                                                   bytesFor(candidate.textUtf8),
                                                   bytesFor(candidate.commentUtf8)});
    }
    FcitxKeyResponseC response{};
    response.metadata = FcitxMetadataC{responseId,
                                       requestMetadata.requestId,
                                       engineEpoch,
                                       requestMetadata.sessionId,
                                       requestMetadata.contextId,
                                       runtimeResult.compositionId,
                                       runtimeResult.revision};
    response.status = kProtocolStatusOk;
    response.handled = runtimeResult.handled ? 1U : 0U;
    response.commit = bytesFor(runtimeResult.commitUtf8);
    response.preedit = bytesFor(runtimeResult.preeditUtf8);
    response.preeditCaretUtf8 = runtimeResult.preeditCaretUtf8;
    response.selectedCandidate = runtimeResult.selectedCandidate;
    response.candidatePage = runtimeResult.candidatePage;
    response.candidateTotal = runtimeResult.candidateTotal;
    response.candidateVisibility = runtimeResult.candidateVisibility;
    response.candidatePageSize = runtimeResult.candidatePageSize;
    response.candidateBulk = runtimeResult.candidateBulk ? 1U : 0U;
    response.candidateEnd = runtimeResult.candidateEnd ? 1U : 0U;
    response.deleteSurroundingText = runtimeResult.deleteSurroundingText ? 1U : 0U;
    response.deleteSurroundingOffset = runtimeResult.deleteSurroundingOffset;
    response.deleteSurroundingSize = runtimeResult.deleteSurroundingSize;
    response.forwardKey = runtimeResult.forwardKey ? 1U : 0U;
    response.forwardKeySym = runtimeResult.forwardKeySym;
    response.forwardKeyStates = runtimeResult.forwardKeyStates;
    response.forwardKeyCode = runtimeResult.forwardKeyCode;
    response.forwardKeyRelease = runtimeResult.forwardKeyRelease ? 1U : 0U;
    response.caret = runtimeResult.caret;
    response.popupAllowed = runtimeResult.popupAllowed ? 1U : 0U;
    response.contentLocale = bytesFor(runtimeResult.contentLocaleUtf8);
    response.candidates = candidates.data();
    response.candidateCount = candidates.size();
    return encodeMessage(fcitx5_protocol_core_encode_key_response, response);
}

std::vector<std::uint8_t> encodeEngineStatusResponse(const FcitxMetadataC& requestMetadata,
                                                     std::uint64_t responseId,
                                                     std::uint64_t engineEpoch,
                                                     const engine::InputMethodStatus& status) {
    const FcitxEngineStatusResponseC response{FcitxMetadataC{responseId, requestMetadata.requestId,
                                                             engineEpoch, requestMetadata.sessionId,
                                                             0, 0, 0},
                                              kProtocolStatusOk,
                                              bytesFor(status.id),
                                              bytesFor(status.name),
                                              bytesFor(status.nativeName),
                                              bytesFor(status.shortLabel)};
    return encodeMessage(fcitx5_protocol_core_encode_engine_status_response, response);
}

bool signalCandidateUpdate(const platform::RuntimeIdentity& identity,
                           std::uint32_t targetProcessId) noexcept {
    const std::wstring name =
        platform::makeLocalObjectName(identity, L"candidate-" + std::to_wstring(targetProcessId));
    if (name.empty())
        return false;
    const HANDLE event = OpenEventW(EVENT_MODIFY_STATE, FALSE, name.c_str());
    if (!event)
        return false;
    const bool signaled = SetEvent(event) != FALSE;
    CloseHandle(event);
    return signaled;
}

std::vector<std::uint8_t>
handleRequest(std::span<const std::uint8_t> requestBytes, std::uint64_t engineEpoch,
              std::atomic<std::uint64_t>& nextResponseId,
              const platform::ProcessIdentity& clientIdentity, std::uint64_t connectionId,
              void* session, engine::FcitxDispatcher& dispatcher,
              PresentationPublisher& presentation, const platform::RuntimeIdentity& serverIdentity,
              const std::wstring& uiExecutable) {
    WireFrameView frame;
    if (!decodeFrame(requestBytes, frame)) {
        return {};
    }
    if (frame.type == kHelloRequestType) {
        FcitxHelloRequestC request{};
        std::vector<std::uint8_t> requestStrings;
        // E4-3: the hello handshake is Rust-owned (repeat handshake, stale
        // request id, and session/process mismatch are rejected; on success
        // the session becomes handshake-complete and records the request id).
        if (!decodeMessage(frame, fcitx5_protocol_core_decode_hello_request, request,
                           requestStrings) ||
            fcitx5_engine_core_session_begin_hello(
                session, request.metadata.requestId, request.metadata.sessionId,
                clientIdentity.sessionId, request.clientProcessId, clientIdentity.processId) == 0) {
            return {};
        }
        const FcitxHelloResponseC response{FcitxMetadataC{nextResponseId.fetch_add(1),
                                                          request.metadata.requestId, engineEpoch,
                                                          request.metadata.sessionId, 0, 0, 0},
                                           kProtocolStatusOk, 64};
        return encodeMessage(fcitx5_protocol_core_encode_hello_response, response);
    }
    // E4-3: every non-hello frame is validated by the Rust per-connection
    // session (handshake complete, engine-epoch match, session-id match, and
    // strictly-newer request id); the C++ handshakeComplete/lastRequestId
    // locals are deleted.
    if (fcitx5_engine_core_session_accept_frame(session, frame.metadata.requestId,
                                                frame.metadata.sessionId, clientIdentity.sessionId,
                                                frame.metadata.engineEpoch, engineEpoch) == 0) {
        return {};
    }

    if (frame.type == kKeyRequestType) {
        FcitxKeyRequestC request{};
        std::vector<std::uint8_t> requestStrings;
        engine::RuntimeResult runtimeResult;
        const auto timeout = std::chrono::milliseconds(
            fcitx5_engine_core_key_request_timeout_ms(frame.metadata.revision));
        if (!decodeMessage(frame, fcitx5_protocol_core_decode_key_request, request,
                           requestStrings) ||
            !dispatcher.processKey(engine::ClientContextKey{clientIdentity.processId, connectionId,
                                                            request.metadata.contextId},
                                   request, runtimeResult, timeout))
            return {};
        fcitx5_engine_core_session_complete_request(session, request.metadata.requestId);
        auto encodedResponse = encodeStateResponse(request.metadata, nextResponseId.fetch_add(1),
                                                   engineEpoch, runtimeResult);
        if (encodedResponse.empty())
            return {};
        presentation.publish(encodedResponse);
        return encodedResponse;
    }

    if (frame.type == kCandidateSelectRequestType) {
        FcitxCandidateSelectRequestC request{};
        std::vector<std::uint8_t> requestStrings;
        engine::RuntimeResult runtimeResult;
        if (!platform::pathsReferToSameFile(clientIdentity.executablePath, uiExecutable) ||
            !decodeMessage(frame, fcitx5_protocol_core_decode_candidate_select_request, request,
                           requestStrings) ||
            !dispatcher.selectCandidate(request.targetProcessId, request, runtimeResult,
                                        std::chrono::milliseconds(75)))
            return {};
        fcitx5_engine_core_session_complete_request(session, request.metadata.requestId);
        const auto encodedState = encodeStateResponse(request.metadata, nextResponseId.fetch_add(1),
                                                      engineEpoch, runtimeResult);
        if (encodedState.empty())
            return {};
        presentation.publish(encodedState);
        const bool notified = signalCandidateUpdate(serverIdentity, request.targetProcessId);
        const FcitxCandidateSelectResponseC response{
            FcitxMetadataC{nextResponseId.fetch_add(1), request.metadata.requestId, engineEpoch,
                           request.metadata.sessionId, request.metadata.contextId,
                           runtimeResult.compositionId, runtimeResult.revision},
            notified ? kProtocolStatusOk : kProtocolStatusUnsupported};
        return encodeMessage(fcitx5_protocol_core_encode_candidate_select_response, response);
    }

    if (frame.type == kStateRequestType) {
        FcitxStateRequestC request{};
        std::vector<std::uint8_t> requestStrings;
        engine::RuntimeResult runtimeResult;
        if (!decodeMessage(frame, fcitx5_protocol_core_decode_state_request, request,
                           requestStrings) ||
            !dispatcher.takePendingState(engine::ClientContextKey{clientIdentity.processId,
                                                                  connectionId,
                                                                  request.metadata.contextId},
                                         request, runtimeResult, std::chrono::milliseconds(75)))
            return {};
        fcitx5_engine_core_session_complete_request(session, request.metadata.requestId);
        return encodeStateResponse(request.metadata, nextResponseId.fetch_add(1), engineEpoch,
                                   runtimeResult);
    }
    if (frame.type == kEngineStatusRequestType) {
        FcitxEngineStatusRequestC request{};
        std::vector<std::uint8_t> requestStrings;
        engine::InputMethodStatus status;
        if (!decodeMessage(frame, fcitx5_protocol_core_decode_engine_status_request, request,
                           requestStrings) ||
            !dispatcher.queryInputMethodStatus(status, std::chrono::milliseconds(75))) {
            return {};
        }
        fcitx5_engine_core_session_complete_request(session, request.metadata.requestId);
        return encodeEngineStatusResponse(request.metadata, nextResponseId.fetch_add(1),
                                          engineEpoch, status);
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
    PresentationPublisher presentation(platform::makeLocalEndpointName(identity, L"presentation"),
                                       uiExecutable);
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
        if (!internalStopEvent)
            return 3;
    }
    if (!stopEventName.empty()) {
        stopEvent = OpenEventW(SYNCHRONIZE, FALSE, stopEventName.c_str());
        if (!stopEvent) {
            if (internalStopEvent)
                CloseHandle(internalStopEvent);
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
                    workerCount, kWirePipeBufferSize, kWirePipeBufferSize, 25,
                    pipeSecurity.attributes());
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
                        auto response = handleRequest(
                            request, engineEpoch, nextResponseId, clientIdentity, connectionId,
                            session, dispatcher, presentation, identity, uiExecutable);
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
                    if (stopEvent)
                        SetEvent(stopEvent);
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
        std::cout << "fcitx5-engine 0.1.0 protocol " << kWireProtocolVersion << '\n';
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
        const int size = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, argv[2], -1, nullptr, 0,
                                             nullptr, nullptr);
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
