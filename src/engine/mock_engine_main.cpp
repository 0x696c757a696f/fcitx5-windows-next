#include "protocol.h"
#include "pipe_client.h"
#include "pipe_security.h"
#include "runtime_identity.h"

#include <fcitx5_windows/version.h>

#include <Windows.h>

#include <array>
#include <atomic>
#include <cstdlib>
#include <cstdint>
#include <iostream>
#include <span>
#include <string>
#include <thread>
#include <vector>

namespace {

using namespace fcitx::windows;

bool transfer(HANDLE pipe, bool write, void* data, std::size_t size, DWORD timeout,
              HANDLE stopEvent = nullptr) {
    auto* cursor = static_cast<std::uint8_t*>(data);
    std::size_t completed = 0;
    while (completed < size) {
        HANDLE event = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        if (!event) return false;
        OVERLAPPED operation{};
        operation.hEvent = event;
        DWORD transferred = 0;
        const DWORD requested = static_cast<DWORD>(size - completed);
        const BOOL immediate = write
                                   ? WriteFile(pipe, cursor + completed, requested, &transferred,
                                               &operation)
                                   : ReadFile(pipe, cursor + completed, requested, &transferred,
                                              &operation);
        bool success = immediate != FALSE;
        if (!success && GetLastError() == ERROR_IO_PENDING) {
            const std::array<HANDLE, 2> waits{event, stopEvent};
            if (WaitForMultipleObjects(stopEvent ? 2U : 1U, waits.data(), FALSE, timeout) ==
                WAIT_OBJECT_0) {
                success = GetOverlappedResult(pipe, &operation, &transferred, FALSE) != FALSE;
            } else {
                CancelIoEx(pipe, &operation);
                GetOverlappedResult(pipe, &operation, &transferred, TRUE);
                success = false;
            }
        }
        CloseHandle(event);
        if (!success || transferred == 0) {
            return false;
        }
        completed += transferred;
    }
    return true;
}

bool readAll(HANDLE pipe, void* data, std::size_t size, DWORD timeout, HANDLE stopEvent) {
    return transfer(pipe, false, data, size, timeout, stopEvent);
}

bool writeAll(HANDLE pipe, const void* data, std::size_t size, DWORD timeout,
              HANDLE stopEvent) {
    return transfer(pipe, true, const_cast<void*>(data), size, timeout, stopEvent);
}

bool connectClient(HANDLE pipe, HANDLE stopEvent) {
    HANDLE event = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    if (!event) return false;
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
        if (WaitForMultipleObjects(stopEvent ? 2U : 1U, waits.data(), FALSE, 60'000) ==
            WAIT_OBJECT_0) {
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

bool readFrame(HANDLE pipe, std::vector<std::uint8_t>& frameBytes, HANDLE stopEvent) {
    std::array<std::uint8_t, protocol::kHeaderSize> header{};
    if (!readAll(pipe, header.data(), header.size(), 60'000, stopEvent)) {
        return false;
    }
    protocol::MessageType type{};
    std::uint32_t bodySize = 0;
    protocol::Metadata metadata;
    if (!protocol::decodeHeader(header, type, bodySize, metadata)) {
        return false;
    }
    frameBytes.assign(header.begin(), header.end());
    frameBytes.resize(protocol::kHeaderSize + bodySize);
    return bodySize == 0 ||
           readAll(pipe, frameBytes.data() + protocol::kHeaderSize, bodySize, 100,
                   stopEvent);
}

std::vector<std::uint8_t> handle(std::span<const std::uint8_t> requestBytes,
                                 std::uint64_t engineEpoch,
                                 std::atomic<std::uint64_t>& nextResponseId,
                                 const platform::ProcessIdentity& clientIdentity,
                                 bool& handshakeComplete,
                                 std::uint64_t& lastRequestId,
                                 bool compositionTest) {
    protocol::FrameView frame;
    if (!protocol::decodeFrame(requestBytes, frame)) {
        return {};
    }
    if (frame.metadata.requestId <= lastRequestId) return {};
    if (frame.type == protocol::MessageType::helloRequest) {
        protocol::HelloRequest request;
        if (!protocol::decode(frame, request) ||
            handshakeComplete || request.metadata.sessionId != clientIdentity.sessionId ||
            request.clientProcessId != clientIdentity.processId) {
            return {};
        }
        lastRequestId = request.metadata.requestId;
        handshakeComplete = true;
        return protocol::encode(protocol::HelloResponse{
            protocol::Metadata{nextResponseId.fetch_add(1), request.metadata.requestId,
                               engineEpoch, request.metadata.sessionId, 0, 0, 0},
            protocol::Status::ok, static_cast<std::uint32_t>(sizeof(void*) * 8)});
    }
    if (frame.type == protocol::MessageType::keyRequest) {
        protocol::KeyRequest request;
        if (!protocol::decode(frame, request) || !handshakeComplete ||
            request.metadata.engineEpoch != engineEpoch ||
            request.metadata.sessionId != clientIdentity.sessionId) {
            return {};
        }
        lastRequestId = request.metadata.requestId;
        protocol::KeyResponse response;
        response.metadata = protocol::Metadata{
            nextResponseId.fetch_add(1), request.metadata.requestId, engineEpoch,
            request.metadata.sessionId, request.metadata.contextId,
            request.metadata.compositionId, request.metadata.revision + 1};
        response.status = protocol::Status::ok;
        response.caret = request.caret;
        if (compositionTest && request.virtualKey == 'N') {
            if (request.scanCode == 0 || request.keyboardLayout == 0 ||
                (!request.inputMethodUtf8.empty() && request.inputMethodUtf8 != "mozc") ||
                !request.surroundingTextValid) return {};
            response.handled = true;
            response.preeditUtf8 = "n";
            response.preeditCaretUtf8 = 1;
            response.candidates = {{101, "1", "\xe4\xbd\xa0", "ni"},
                                   {102, "2", "\xe5\x91\xa2", "ne"}};
            response.selectedCandidate = 0;
            response.candidateTotal = 2;
            response.candidateVisibility = 1;
            response.candidatePageSize = 2;
        } else if (compositionTest && request.virtualKey == VK_SPACE) {
            if (request.popupAllowed) return {};
            response.handled = true;
            response.commitUtf8 = "\xe4\xbd\xa0";
        } else if (request.virtualKey >= 'A' && request.virtualKey <= 'Z') {
            response.handled = true;
            response.commitUtf8.push_back(
                static_cast<char>('a' + (request.virtualKey - static_cast<std::uint32_t>('A'))));
        }
        return protocol::encode(response);
    }
    if (frame.type == protocol::MessageType::engineStatusRequest) {
        protocol::EngineStatusRequest request;
        if (!protocol::decode(frame, request) || !handshakeComplete ||
            request.metadata.engineEpoch != engineEpoch ||
            request.metadata.sessionId != clientIdentity.sessionId) {
            return {};
        }
        lastRequestId = request.metadata.requestId;
        return protocol::encode(protocol::EngineStatusResponse{
            protocol::Metadata{nextResponseId.fetch_add(1), request.metadata.requestId,
                               engineEpoch, request.metadata.sessionId, 0, 0, 0},
            protocol::Status::ok,
            "mock-pinyin",
            "Mock Pinyin",
            "\xe5\xb0\x8f\xe4\xbc\x81\xe9\xb9\x85",
            "\xe5\xb0\x8f"});
    }
    return {};
}

int serve(const std::wstring& pipeName, unsigned testClientCount,
          const std::wstring& readyEventName, const std::wstring& stopEventName,
          bool compositionTest) {
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
    FILETIME now{};
    GetSystemTimeAsFileTime(&now);
    const std::uint64_t engineEpoch =
        (static_cast<std::uint64_t>(now.dwHighDateTime) << 32U) | now.dwLowDateTime;
    std::atomic<std::uint64_t> nextResponseId{1};
    std::atomic<bool> readinessSignaled{};
    std::atomic<int> serverError{};
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
    const unsigned workerCount = testClientCount == 0 ? 4U : testClientCount;
    std::vector<std::thread> workers;
    workers.reserve(workerCount);
    for (unsigned workerIndex = 0; workerIndex < workerCount; ++workerIndex) {
        workers.emplace_back([&] {
            for (;;) {
                if (stopEvent && WaitForSingleObject(stopEvent, 0) == WAIT_OBJECT_0) return;
                HANDLE pipe = CreateNamedPipeW(
                    pipeName.c_str(), PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT |
                        PIPE_REJECT_REMOTE_CLIENTS,
                    workerCount, static_cast<DWORD>(protocol::kMaxFrameSize),
                    static_cast<DWORD>(protocol::kMaxFrameSize), 25,
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
                    if (stopEvent && WaitForSingleObject(stopEvent, 0) == WAIT_OBJECT_0) return;
                    continue;
                }
                platform::ProcessIdentity clientIdentity;
                if (ipc::verifyPipeClient(pipe, identity, &clientIdentity)) {
                    bool handshakeComplete = false;
                    std::uint64_t lastRequestId = 0;
                    std::vector<std::uint8_t> request;
                    while (readFrame(pipe, request, stopEvent)) {
                        const auto response =
                            handle(request, engineEpoch, nextResponseId, clientIdentity,
                                   handshakeComplete, lastRequestId, compositionTest);
                        if (response.empty() ||
                            !writeAll(pipe, response.data(), response.size(), 100, stopEvent)) {
                            break;
                        }
                    }
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
    for (auto& worker : workers) worker.join();
    if (stopEvent) CloseHandle(stopEvent);
    if (internalStopEvent && internalStopEvent != stopEvent) CloseHandle(internalStopEvent);
    return serverError.load();
#if defined(_MSC_VER)
#pragma warning(pop)
#endif
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
        std::cout << "fcitx5-mock-engine " << fcitx::windows::version()
                  << " protocol " << fcitx::windows::protocol::kVersion << '\n';
        return 0;
    }
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity) ||
        !identity.mayUseUserEngine()) {
        return 4;
    }
    std::wstring pipeName =
        fcitx::windows::platform::makeLocalEndpointName(identity, L"engine");
    std::wstring readyEventName;
    std::wstring stopEventName;
    unsigned testClientCount = 0;
    bool compositionTest = false;
    for (int index = 1; index < argc; ++index) {
        const std::wstring_view argument(argv[index]);
        if (argument == L"--test-once") {
            testClientCount = 1;
        } else if (argument == L"--safe-mode") {
            // Phase 2 mock has no addons; accepting the flag exercises launcher policy.
        } else if (argument == L"--composition-test") {
            compositionTest = true;
        } else if (argument == L"--test-clients" && index + 1 < argc) {
            wchar_t* end = nullptr;
            const unsigned long parsed = std::wcstoul(argv[++index], &end, 10);
            if (!end || *end != L'\0' || parsed == 0 || parsed > 64) return 1;
            testClientCount = static_cast<unsigned>(parsed);
        } else if (argument == L"--pipe" && index + 1 < argc) {
            pipeName = argv[++index];
        } else if (argument == L"--ready-event" && index + 1 < argc) {
            readyEventName = argv[++index];
        } else if (argument == L"--stop-event" && index + 1 < argc) {
            stopEventName = argv[++index];
        } else {
            std::wcerr << L"Usage: fcitx5-mock-engine [--test-once|--test-clients N] [--pipe NAME] "
                          L"[--ready-event NAME] [--stop-event NAME]\n";
            return 1;
        }
    }
    return serve(pipeName, testClientCount, readyEventName, stopEventName,
                 compositionTest);
}
