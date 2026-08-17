#include "pipe_client.h"

#include "protocol.h"

#include <algorithm>
#include <array>
#include <limits>
#include <span>
#include <vector>

namespace fcitx::windows::ipc {
namespace {

DWORD remainingMilliseconds(std::uint64_t deadline) noexcept {
    const auto now = GetTickCount64();
    if (now >= deadline) {
        return 0;
    }
    const auto remaining = deadline - now;
    return static_cast<DWORD>((std::min)(remaining, static_cast<std::uint64_t>(MAXDWORD - 1)));
}

bool utf8ToWide(std::string_view input, std::wstring& output) {
    output.clear();
    if (input.empty()) {
        return true;
    }
    const int size = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                                         static_cast<int>(input.size()), nullptr, 0);
    if (size <= 0) {
        return false;
    }
    output.resize(static_cast<std::size_t>(size));
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                               static_cast<int>(input.size()), output.data(), size) == size;
}

} // namespace

PipeClient::PipeClient() : PipeClient(kDefaultPipeName) {}

PipeClient::PipeClient(std::wstring pipeName) : pipeName_(std::move(pipeName)) {}

PipeClient::~PipeClient() { disconnect(); }

void PipeClient::disconnect() noexcept {
    if (pipe_ != INVALID_HANDLE_VALUE) {
        CancelIoEx(pipe_, nullptr);
        CloseHandle(pipe_);
        pipe_ = INVALID_HANDLE_VALUE;
    }
    handshakeComplete_ = false;
    engineEpoch_ = 0;
}

bool PipeClient::connect(std::uint64_t deadline) noexcept {
    if (pipe_ != INVALID_HANDLE_VALUE) {
        return true;
    }
    if (remainingMilliseconds(deadline) == 0) {
        return false;
    }
    pipe_ = CreateFileW(pipeName_.c_str(), GENERIC_READ | GENERIC_WRITE, 0, nullptr, OPEN_EXISTING,
                        FILE_FLAG_OVERLAPPED, nullptr);
    if (pipe_ == INVALID_HANDLE_VALUE) {
        return false;
    }
    return true;
}

bool PipeClient::transfer(bool write, void* data, std::size_t size,
                          std::uint64_t deadline) noexcept {
    auto* cursor = static_cast<std::uint8_t*>(data);
    std::size_t completed = 0;
    while (completed < size) {
        const DWORD wait = remainingMilliseconds(deadline);
        if (wait == 0 || size - completed > MAXDWORD) {
            return false;
        }
        HANDLE event = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        if (!event) {
            return false;
        }
        OVERLAPPED operation{};
        operation.hEvent = event;
        DWORD transferred = 0;
        const DWORD requested = static_cast<DWORD>(size - completed);
        const BOOL immediate = write
                                   ? WriteFile(pipe_, cursor + completed, requested, &transferred,
                                               &operation)
                                   : ReadFile(pipe_, cursor + completed, requested, &transferred,
                                              &operation);
        bool success = immediate != FALSE;
        if (!success && GetLastError() == ERROR_IO_PENDING) {
            const DWORD waitResult = WaitForSingleObject(event, wait);
            if (waitResult == WAIT_OBJECT_0) {
                success = GetOverlappedResult(pipe_, &operation, &transferred, FALSE) != FALSE;
            } else {
                CancelIoEx(pipe_, &operation);
                GetOverlappedResult(pipe_, &operation, &transferred, TRUE);
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

bool PipeClient::transact(const std::vector<std::uint8_t>& request,
                          std::vector<std::uint8_t>& response,
                          std::uint64_t deadline) noexcept {
    if (request.empty() || request.size() > protocol::kMaxFrameSize ||
        !transfer(true, const_cast<std::uint8_t*>(request.data()), request.size(), deadline)) {
        disconnect();
        return false;
    }
    std::array<std::uint8_t, protocol::kHeaderSize> header{};
    if (!transfer(false, header.data(), header.size(), deadline)) {
        disconnect();
        return false;
    }
    protocol::MessageType type{};
    std::uint32_t bodySize = 0;
    if (!protocol::decodeHeader(header, type, bodySize)) {
        disconnect();
        return false;
    }
    try {
        response.assign(header.begin(), header.end());
        response.resize(protocol::kHeaderSize + bodySize);
    } catch (...) {
        disconnect();
        return false;
    }
    if (bodySize > 0 &&
        !transfer(false, response.data() + protocol::kHeaderSize, bodySize, deadline)) {
        disconnect();
        return false;
    }
    return true;
}

bool PipeClient::handshake(std::uint64_t deadline) noexcept {
    try {
        if (handshakeComplete_) {
            return true;
        }
        const auto requestId = nextRequestId_.fetch_add(1, std::memory_order_relaxed);
        protocol::HelloRequest request{requestId, static_cast<std::uint32_t>(sizeof(void*) * 8)};
        std::vector<std::uint8_t> responseBytes;
        if (!transact(protocol::encode(request), responseBytes, deadline)) {
            return false;
        }
        protocol::FrameView frame;
        protocol::HelloResponse response;
        if (!protocol::decodeFrame(responseBytes, frame) || !protocol::decode(frame, response) ||
            response.responseTo != requestId || response.status != protocol::Status::ok) {
            disconnect();
            return false;
        }
        engineEpoch_ = response.engineEpoch;
        handshakeComplete_ = true;
        return true;
    } catch (...) {
        disconnect();
        return false;
    }
}

bool PipeClient::processKey(std::uint64_t contextId, std::uint32_t virtualKey,
                            std::uint32_t keyFlags, KeyResult& result) noexcept {
    result = {};
    try {
        const std::uint64_t deadline = GetTickCount64() + kInputDeadlineMilliseconds;
        if (!connect(deadline) || !handshake(deadline)) {
            disconnect();
            return false;
        }
        const auto requestId = nextRequestId_.fetch_add(1, std::memory_order_relaxed);
        protocol::KeyRequest request{requestId, contextId, virtualKey, keyFlags};
        std::vector<std::uint8_t> responseBytes;
        if (!transact(protocol::encode(request), responseBytes, deadline)) {
            return false;
        }
        protocol::FrameView frame;
        protocol::KeyResponse response;
        if (!protocol::decodeFrame(responseBytes, frame) || !protocol::decode(frame, response) ||
            response.responseTo != requestId || response.status != protocol::Status::ok ||
            !utf8ToWide(response.commitUtf8, result.commit)) {
            disconnect();
            return false;
        }
        result.handled = response.handled;
        return true;
    } catch (...) {
        disconnect();
        result.handled = false;
        result.commit.clear();
        return false;
    }
}

} // namespace fcitx::windows::ipc
