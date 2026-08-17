#include "protocol.h"
#include "pipe_client.h"

#include <Windows.h>

#include <array>
#include <cstdint>
#include <iostream>
#include <span>
#include <string>
#include <vector>

namespace {

using namespace fcitx::windows;

bool readAll(HANDLE pipe, void* data, std::size_t size) {
    auto* cursor = static_cast<std::uint8_t*>(data);
    std::size_t completed = 0;
    while (completed < size) {
        DWORD transferred = 0;
        if (!ReadFile(pipe, cursor + completed, static_cast<DWORD>(size - completed),
                      &transferred, nullptr) ||
            transferred == 0) {
            return false;
        }
        completed += transferred;
    }
    return true;
}

bool writeAll(HANDLE pipe, const void* data, std::size_t size) {
    const auto* cursor = static_cast<const std::uint8_t*>(data);
    std::size_t completed = 0;
    while (completed < size) {
        DWORD transferred = 0;
        if (!WriteFile(pipe, cursor + completed, static_cast<DWORD>(size - completed),
                       &transferred, nullptr) ||
            transferred == 0) {
            return false;
        }
        completed += transferred;
    }
    return true;
}

bool readFrame(HANDLE pipe, std::vector<std::uint8_t>& frameBytes) {
    std::array<std::uint8_t, protocol::kHeaderSize> header{};
    if (!readAll(pipe, header.data(), header.size())) {
        return false;
    }
    protocol::MessageType type{};
    std::uint32_t bodySize = 0;
    if (!protocol::decodeHeader(header, type, bodySize)) {
        return false;
    }
    frameBytes.assign(header.begin(), header.end());
    frameBytes.resize(protocol::kHeaderSize + bodySize);
    return bodySize == 0 ||
           readAll(pipe, frameBytes.data() + protocol::kHeaderSize, bodySize);
}

std::vector<std::uint8_t> handle(std::span<const std::uint8_t> requestBytes) {
    protocol::FrameView frame;
    if (!protocol::decodeFrame(requestBytes, frame)) {
        return {};
    }
    if (frame.type == protocol::MessageType::helloRequest) {
        protocol::HelloRequest request;
        if (!protocol::decode(frame, request) ||
            (request.clientArchitectureBits != 32 && request.clientArchitectureBits != 64)) {
            return {};
        }
        return protocol::encode(
            protocol::HelloResponse{request.requestId, protocol::Status::ok, 1});
    }
    if (frame.type == protocol::MessageType::keyRequest) {
        protocol::KeyRequest request;
        if (!protocol::decode(frame, request)) {
            return {};
        }
        protocol::KeyResponse response;
        response.responseTo = request.requestId;
        response.status = protocol::Status::ok;
        if (request.virtualKey >= 'A' && request.virtualKey <= 'Z') {
            response.handled = true;
            response.commitUtf8.push_back(
                static_cast<char>('a' + (request.virtualKey - static_cast<std::uint32_t>('A'))));
        }
        return protocol::encode(response);
    }
    return {};
}

int serve(const std::wstring& pipeName, bool testOnce, const std::wstring& readyEventName) {
    unsigned completedClients = 0;
    bool readinessSignaled = false;
    for (;;) {
        HANDLE pipe = CreateNamedPipeW(
            pipeName.c_str(), PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS, 1,
            static_cast<DWORD>(protocol::kMaxFrameSize),
            static_cast<DWORD>(protocol::kMaxFrameSize), 25, nullptr);
        if (pipe == INVALID_HANDLE_VALUE) {
            std::cerr << "CreateNamedPipeW failed: " << GetLastError() << '\n';
            return 2;
        }
        if (!readinessSignaled && !readyEventName.empty()) {
            HANDLE readyEvent = OpenEventW(EVENT_MODIFY_STATE, FALSE, readyEventName.c_str());
            if (!readyEvent) {
                CloseHandle(pipe);
                return 3;
            }
            SetEvent(readyEvent);
            CloseHandle(readyEvent);
            readinessSignaled = true;
        }
        const BOOL connected = ConnectNamedPipe(pipe, nullptr);
        if (!connected && GetLastError() != ERROR_PIPE_CONNECTED) {
            CloseHandle(pipe);
            continue;
        }
        std::vector<std::uint8_t> request;
        while (readFrame(pipe, request)) {
            const auto response = handle(request);
            if (response.empty() || !writeAll(pipe, response.data(), response.size())) {
                break;
            }
        }
        FlushFileBuffers(pipe);
        DisconnectNamedPipe(pipe);
        CloseHandle(pipe);
        ++completedClients;
        if (testOnce && completedClients == 1) {
            return 0;
        }
    }
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    std::wstring pipeName = fcitx::windows::ipc::kDefaultPipeName;
    std::wstring readyEventName;
    bool testOnce = false;
    for (int index = 1; index < argc; ++index) {
        const std::wstring_view argument(argv[index]);
        if (argument == L"--test-once") {
            testOnce = true;
        } else if (argument == L"--pipe" && index + 1 < argc) {
            pipeName = argv[++index];
        } else if (argument == L"--ready-event" && index + 1 < argc) {
            readyEventName = argv[++index];
        } else {
            std::wcerr << L"Usage: fcitx5-mock-engine [--test-once] [--pipe NAME] "
                          L"[--ready-event NAME]\n";
            return 1;
        }
    }
    return serve(pipeName, testOnce, readyEventName);
}
