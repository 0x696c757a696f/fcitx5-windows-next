#include "pipe_client.h"
#include "protocol_ffi.h"
#include "runtime_identity.h"

#include <Windows.h>

#include <array>
#include <atomic>
#include <cstdint>
#include <iostream>
#include <thread>
#include <vector>

namespace {

// Flat protocol-core frame bounds (Rust owns the wire contract).
constexpr std::size_t kHeaderSize = 64;
constexpr std::uint16_t kHelloRequestType = 1;
constexpr std::uint16_t kKeyRequestType = 3;

bool transfer(HANDLE pipe, bool write, void* data, std::size_t size, DWORD timeout) {
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
            if (WaitForSingleObject(event, timeout) == WAIT_OBJECT_0) {
                success = GetOverlappedResult(pipe, &operation, &transferred, FALSE) != FALSE;
            } else {
                CancelIoEx(pipe, &operation);
                GetOverlappedResult(pipe, &operation, &transferred, TRUE);
                success = false;
            }
        }
        CloseHandle(event);
        if (!success || transferred == 0) return false;
        completed += transferred;
    }
    return true;
}

bool connect(HANDLE pipe) {
    HANDLE event = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    if (!event) return false;
    OVERLAPPED operation{};
    operation.hEvent = event;
    const BOOL immediate = ConnectNamedPipe(pipe, &operation);
    const DWORD error = immediate ? ERROR_SUCCESS : GetLastError();
    bool success = immediate != FALSE || error == ERROR_PIPE_CONNECTED;
    if (!success && error == ERROR_IO_PENDING &&
        WaitForSingleObject(event, 2000) == WAIT_OBJECT_0) {
        DWORD ignored = 0;
        success = GetOverlappedResult(pipe, &operation, &ignored, FALSE) != FALSE;
    }
    if (!success && error == ERROR_IO_PENDING) {
        CancelIoEx(pipe, &operation);
        DWORD ignored = 0;
        GetOverlappedResult(pipe, &operation, &ignored, TRUE);
    }
    CloseHandle(event);
    return success;
}

// Reads one complete FCW4 frame (header + body) into `bytes`.
bool readFrame(HANDLE pipe, std::vector<std::uint8_t>& bytes) {
    std::array<std::uint8_t, kHeaderSize> header{};
    if (!transfer(pipe, false, header.data(), header.size(), 2000)) return false;
    std::uint16_t type = 0;
    std::uint32_t bodySize = 0;
    FcitxMetadataC metadata{};
    if (fcitx5_protocol_core_decode_header(header.data(), header.size(), &type, &bodySize,
                                           &metadata) == 0) {
        return false;
    }
    bytes.assign(header.begin(), header.end());
    bytes.resize(kHeaderSize + bodySize);
    return bodySize == 0 ||
           transfer(pipe, false, bytes.data() + kHeaderSize, bodySize, 2000);
}

bool writeFrame(HANDLE pipe, const std::vector<std::uint8_t>& bytes) {
    return !bytes.empty() &&
           transfer(pipe, true, const_cast<std::uint8_t*>(bytes.data()), bytes.size(), 2000);
}

HANDLE createPipe(const std::wstring& name) {
    return CreateNamedPipeW(name.c_str(), PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT |
                                PIPE_REJECT_REMOTE_CLIENTS,
                            1, 4096, 4096, 25, nullptr);
}

// Encodes one frame through the Rust codec into a byte vector.
template <typename Message, typename Encoder>
std::vector<std::uint8_t> encodeMessage(const Message& message, Encoder encoder) {
    std::array<std::uint8_t, 512> stack{};
    std::size_t written = 0;
    if (encoder(&message, stack.data(), stack.size(), &written)) {
        return std::vector<std::uint8_t>(stack.begin(), stack.begin() + written);
    }
    if (written == 0) {
        return {}; // rejected by validation
    }
    std::vector<std::uint8_t> out(written);
    std::size_t finalWritten = 0;
    if (!encoder(&message, out.data(), out.size(), &finalWritten)) {
        return {};
    }
    out.resize(finalWritten);
    return out;
}

// Decodes a request body through the Rust codec (used only to confirm the
// frame is well-formed; request metadata comes from the header decode).
template <typename Message, typename Decoder>
bool decodeRequest(const std::vector<std::uint8_t>& frame, const FcitxMetadataC& metadata,
                   Message& message, Decoder decoder) {
    if (frame.size() < kHeaderSize) {
        return false;
    }
    std::array<std::uint8_t, 4096> strings{};
    std::size_t stringsNeeded = 0;
    if (decoder(&metadata, frame.data() + kHeaderSize, frame.size() - kHeaderSize, &message,
                strings.data(), strings.size(), &stringsNeeded)) {
        return true;
    }
    if (stringsNeeded == 0) {
        return false; // rejected by validation
    }
    std::vector<std::uint8_t> heap(stringsNeeded);
    std::size_t finalNeeded = 0;
    return decoder(&metadata, frame.data() + kHeaderSize, frame.size() - kHeaderSize, &message,
                   heap.data(), heap.size(), &finalNeeded) != 0;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    using namespace fcitx::windows;
    if (argc != 2) return 1;
    constexpr DWORD kContextStartSlackMilliseconds =
        ipc::kContextStartDeadlineMilliseconds + 1000;
    platform::RuntimeIdentity identity;
    if (!platform::queryCurrentIdentity(identity)) return 1;
    std::wstring executablePath(32768, L'\0');
    const DWORD executableLength = GetModuleFileNameW(
        nullptr, executablePath.data(), static_cast<DWORD>(executablePath.size()));
    if (executableLength == 0 || executableLength == executablePath.size()) return 1;
    executablePath.resize(executableLength);

    const std::wstring suffix = std::to_wstring(GetCurrentProcessId());
    const std::wstring mismatchPipeName =
        L"\\\\.\\pipe\\Fcitx5WindowsNext.PeerMismatch." + suffix;
    HANDLE mismatchPipe = createPipe(mismatchPipeName);
    if (mismatchPipe == INVALID_HANDLE_VALUE) return 1;
    std::atomic<bool> mismatchReceivedData{};
    std::thread mismatchServer([&] {
        const bool connected = connect(mismatchPipe);
        std::uint8_t unexpected = 0;
        mismatchReceivedData.store(
            connected && transfer(mismatchPipe, false, &unexpected, 1, 2000));
    });
    ipc::KeyResult mismatchResult;
    ipc::PipeClient mismatchClient(mismatchPipeName, ipc::PeerPolicy::exact(argv[1]));
    const bool mismatchAccepted = mismatchClient.processKey(1, 'A', 0, mismatchResult);
    mismatchServer.join();
    DisconnectNamedPipe(mismatchPipe);
    CloseHandle(mismatchPipe);
    if (mismatchAccepted || mismatchResult.handled || mismatchReceivedData.load()) {
        std::cerr << "mismatched server executable was not rejected\n";
        return 1;
    }

    const std::wstring pipeName = L"\\\\.\\pipe\\Fcitx5WindowsNext.Late." + suffix;
    HANDLE firstReady = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    HANDLE keyReceived = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    HANDLE releaseLate = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    HANDLE secondReady = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    HANDLE abruptKeyReceived = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    HANDLE thirdReady = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    if (!firstReady || !keyReceived || !releaseLate || !secondReady ||
        !abruptKeyReceived || !thirdReady) return 1;

    std::atomic<int> serverError{};
    std::thread server([&] {
        std::uint64_t nextResponseId = 1;
        for (unsigned connectionIndex = 0; connectionIndex < 3; ++connectionIndex) {
            HANDLE pipe = createPipe(pipeName);
            if (pipe == INVALID_HANDLE_VALUE) {
                serverError.store(10);
                return;
            }
            SetEvent(connectionIndex == 0   ? firstReady
                     : connectionIndex == 1 ? secondReady
                                            : thirdReady);
            if (!connect(pipe)) {
                CloseHandle(pipe);
                serverError.store(11);
                return;
            }
            std::vector<std::uint8_t> bytes;
            std::uint16_t type = 0;
            std::uint32_t bodySize = 0;
            FcitxMetadataC helloMetadata{};
            if (!readFrame(pipe, bytes) ||
                fcitx5_protocol_core_decode_header(bytes.data(), kHeaderSize, &type, &bodySize,
                                                   &helloMetadata) == 0 ||
                type != kHelloRequestType || bodySize != bytes.size() - kHeaderSize) {
                CloseHandle(pipe);
                serverError.store(12);
                return;
            }
            FcitxHelloRequestC hello{};
            if (!decodeRequest(bytes, helloMetadata, hello,
                               fcitx5_protocol_core_decode_hello_request)) {
                CloseHandle(pipe);
                serverError.store(12);
                return;
            }
            const std::uint64_t epoch = connectionIndex + 100;
            const FcitxHelloResponseC helloResponse{
                FcitxMetadataC{nextResponseId++, helloMetadata.requestId, epoch,
                               identity.sessionId, 0, 0, 0},
                0, // Status::ok
                static_cast<std::uint32_t>(sizeof(void*) * 8)};
            auto helloBytes = encodeMessage(helloResponse,
                                            fcitx5_protocol_core_encode_hello_response);
            if (helloBytes.empty() || !writeFrame(pipe, helloBytes)) {
                CloseHandle(pipe);
                serverError.store(131);
                return;
            }
            FcitxMetadataC keyMetadata{};
            if (!readFrame(pipe, bytes) ||
                fcitx5_protocol_core_decode_header(bytes.data(), kHeaderSize, &type, &bodySize,
                                                   &keyMetadata) == 0 ||
                type != kKeyRequestType || bodySize != bytes.size() - kHeaderSize) {
                CloseHandle(pipe);
                serverError.store(132);
                return;
            }
            FcitxKeyRequestC key{};
            if (!decodeRequest(bytes, keyMetadata, key,
                               fcitx5_protocol_core_decode_key_request)) {
                CloseHandle(pipe);
                serverError.store(133);
                return;
            }
            const char* commit = connectionIndex == 0 ? "late" : "a";
            FcitxKeyResponseC keyResponse{};
            keyResponse.metadata = FcitxMetadataC{
                nextResponseId++, keyMetadata.requestId, epoch, identity.sessionId,
                keyMetadata.contextId, keyMetadata.compositionId, keyMetadata.revision + 1};
            keyResponse.status = 0; // Status::ok
            keyResponse.handled = 1;
            keyResponse.commit = FcitxBytesC{
                reinterpret_cast<const std::uint8_t*>(commit), connectionIndex == 0 ? 4U : 1U};
            // The key response is a plain handshake-shaped acknowledgement (no
            // preedit/candidates); the client only checks handled/commit.
            keyResponse.selectedCandidate = UINT32_MAX;
            keyResponse.popupAllowed = 1;
            // An invalid caret still must carry the canonical default DPI for
            // the Rust payload validator.
            keyResponse.caret = FcitxCaretRectC{0, 0, 0, 0, 0, 96};
            auto keyBytes = encodeMessage(keyResponse, fcitx5_protocol_core_encode_key_response);
            if (keyBytes.empty()) {
                CloseHandle(pipe);
                serverError.store(134);
                return;
            }
            if (connectionIndex == 0) {
                SetEvent(keyReceived);
                if (WaitForSingleObject(releaseLate, kContextStartSlackMilliseconds) !=
                    WAIT_OBJECT_0) {
                    CloseHandle(pipe);
                    serverError.store(14);
                    return;
                }
                (void)writeFrame(pipe, keyBytes);
            } else if (connectionIndex == 1) {
                SetEvent(abruptKeyReceived);
            } else if (!writeFrame(pipe, keyBytes)) {
                CloseHandle(pipe);
                serverError.store(15);
                return;
            } else {
                std::uint8_t unexpected = 0;
                (void)transfer(pipe, false, &unexpected, 1, 2000);
            }
            DisconnectNamedPipe(pipe);
            CloseHandle(pipe);
        }
    });

    int result = 0;
    {
        ipc::PipeClient client(pipeName, ipc::PeerPolicy::exact(executablePath));
        ipc::KeyResult keyResult;
        if (WaitForSingleObject(firstReady, 2000) != WAIT_OBJECT_0 ||
            client.processKey(9, 'A', 0, keyResult) || keyResult.handled ||
            WaitForSingleObject(keyReceived, 2000) != WAIT_OBJECT_0) {
            result = 1;
        }
        SetEvent(releaseLate);
        if (WaitForSingleObject(secondReady, kContextStartSlackMilliseconds) != WAIT_OBJECT_0 ||
            client.processKey(9, 'A', 0, keyResult) || keyResult.handled ||
            WaitForSingleObject(abruptKeyReceived, 2000) != WAIT_OBJECT_0 ||
            WaitForSingleObject(thirdReady, kContextStartSlackMilliseconds) != WAIT_OBJECT_0 ||
            !client.processKey(9, 'A', 0, keyResult) || !keyResult.handled ||
            keyResult.commit != L"a") {
            result = 1;
        }
    }
    server.join();
    if (serverError.load() != 0) result = 1;

    CloseHandle(thirdReady);
    CloseHandle(abruptKeyReceived);
    CloseHandle(secondReady);
    CloseHandle(releaseLate);
    CloseHandle(keyReceived);
    CloseHandle(firstReady);
    if (result != 0) {
        std::cerr << "late response reconnect test failed, server error "
                  << serverError.load() << '\n';
    }
    return result;
}
