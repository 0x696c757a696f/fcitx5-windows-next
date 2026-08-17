#include "fcitx_dispatcher.h"
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

std::vector<std::uint8_t> handleRequest(std::span<const std::uint8_t> requestBytes,
                                        std::uint64_t engineEpoch,
                                        std::atomic<std::uint64_t>& nextResponseId,
                                        const platform::ProcessIdentity& clientIdentity,
                                        bool& handshakeComplete, std::uint64_t& lastRequestId,
                                        engine::FcitxDispatcher& dispatcher,
                                        engine::PresentationPublisher& presentation) {
    protocol::FrameView frame;
    if (!protocol::decodeFrame(requestBytes, frame) || frame.metadata.requestId <= lastRequestId) {
        return {};
    }
    if (frame.type == protocol::MessageType::helloRequest) {
        protocol::HelloRequest request;
        if (!protocol::decode(frame, request) || handshakeComplete ||
            request.metadata.sessionId != clientIdentity.sessionId ||
            request.clientProcessId != clientIdentity.processId) {
            return {};
        }
        handshakeComplete = true;
        lastRequestId = request.metadata.requestId;
        return protocol::encode(protocol::HelloResponse{
            protocol::Metadata{nextResponseId.fetch_add(1), request.metadata.requestId, engineEpoch,
                               request.metadata.sessionId, 0, 0, 0},
            protocol::Status::ok, 64});
    }
    if (frame.type != protocol::MessageType::keyRequest || !handshakeComplete)
        return {};
    protocol::KeyRequest request;
    if (!protocol::decode(frame, request) || request.metadata.engineEpoch != engineEpoch ||
        request.metadata.sessionId != clientIdentity.sessionId) {
        return {};
    }
    engine::RuntimeResult runtimeResult;
    if (!dispatcher.processKey(
            engine::ClientContextKey{clientIdentity.processId, request.metadata.contextId}, request,
            runtimeResult, std::chrono::milliseconds(100))) {
        return {};
    }
    lastRequestId = request.metadata.requestId;
    protocol::KeyResponse response;
    response.metadata = protocol::Metadata{
        nextResponseId.fetch_add(1), request.metadata.requestId, engineEpoch,
        request.metadata.sessionId,  request.metadata.contextId, runtimeResult.compositionId,
        runtimeResult.revision};
    response.status = protocol::Status::ok;
    response.handled = runtimeResult.handled;
    response.commitUtf8 = std::move(runtimeResult.commitUtf8);
    response.preeditUtf8 = std::move(runtimeResult.preeditUtf8);
    response.preeditCaretUtf8 = runtimeResult.preeditCaretUtf8;
    response.candidates = std::move(runtimeResult.candidates);
    response.selectedCandidate = runtimeResult.selectedCandidate;
    response.candidatePage = runtimeResult.candidatePage;
    response.candidateTotal = runtimeResult.candidateTotal;
    response.candidateVisibility = runtimeResult.candidateVisibility;
    response.candidatePageSize = runtimeResult.candidatePageSize;
    response.candidateBulk = runtimeResult.candidateBulk;
    response.candidateEnd = runtimeResult.candidateEnd;
    response.caret = request.caret;
    presentation.publish(response);
    return protocol::encode(response);
}

int serve(const std::wstring& pipeName, unsigned testClientCount,
          const std::wstring& readyEventName, const std::wstring& stopEventName, bool safeMode) {
    platform::RuntimeIdentity identity;
    platform::PipeSecurity pipeSecurity;
    if (!platform::queryCurrentIdentity(identity) || !identity.mayUseUserEngine() ||
        !platform::PipeSecurity::create(identity, pipeSecurity)) {
        return 4;
    }
    engine::FcitxDispatcher dispatcher;
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
    FILETIME now{};
    GetSystemTimeAsFileTime(&now);
    const std::uint64_t engineEpoch =
        (static_cast<std::uint64_t>(now.dwHighDateTime) << 32U) | now.dwLowDateTime;
    std::atomic<std::uint64_t> nextResponseId{1};
    std::atomic<bool> readinessSignaled{};
    std::atomic<int> serverError{};
    HANDLE stopEvent =
        stopEventName.empty() ? nullptr : OpenEventW(SYNCHRONIZE, FALSE, stopEventName.c_str());
    if (!stopEventName.empty() && !stopEvent)
        return 3;
    const unsigned workerCount = testClientCount == 0 ? 4U : testClientCount;
    std::vector<std::thread> workers;
    workers.reserve(workerCount);
    for (unsigned index = 0; index < workerCount; ++index) {
        workers.emplace_back([&] {
            unsigned completed = 0;
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
                    bool handshakeComplete = false;
                    std::uint64_t lastRequestId = 0;
                    std::vector<std::uint8_t> request;
                    while (readFrame(pipe, request, stopEvent)) {
                        auto response = handleRequest(request, engineEpoch, nextResponseId,
                                                      clientIdentity, handshakeComplete,
                                                      lastRequestId, dispatcher, presentation);
                        if (response.empty() || !transfer(pipe, true, response.data(),
                                                          response.size(), 100, stopEvent)) {
                            break;
                        }
                    }
                }
                DisconnectNamedPipe(pipe);
                CloseHandle(pipe);
                ++completed;
                if (testClientCount != 0 && completed == 1)
                    return;
            }
        });
    }
    for (auto& worker : workers)
        worker.join();
    if (stopEvent)
        CloseHandle(stopEvent);
    dispatcher.stop();
    return serverError.load();
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
        std::cout << "fcitx5-engine 0.1.0 protocol " << fcitx::windows::protocol::kVersion << '\n';
        return 0;
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
            if (!end || *end != L'\0' || parsed == 0 || parsed > 16)
                return 1;
            testClientCount = static_cast<unsigned>(parsed);
        } else if (argument == L"--pipe" && index + 1 < argc) {
            pipeName = argv[++index];
        } else if (argument == L"--ready-event" && index + 1 < argc) {
            readyEventName = argv[++index];
        } else if (argument == L"--stop-event" && index + 1 < argc) {
            stopEventName = argv[++index];
        } else {
            std::wcerr << L"Usage: fcitx5-engine [--test-once|--test-clients N] "
                          L"[--pipe NAME] [--ready-event NAME] [--stop-event NAME]\n";
            return 1;
        }
    }
    return serve(pipeName, testClientCount, readyEventName, stopEventName, safeMode);
}
