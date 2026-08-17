#include "launcher_client.h"

#include "protocol.h"

#include <Windows.h>

#include <algorithm>
#include <array>
#include <atomic>
#include <vector>

namespace fcitx::windows::ipc {
namespace {

DWORD remaining(std::uint64_t deadline) noexcept {
    const auto now = GetTickCount64();
    if (now >= deadline) return 0;
    return static_cast<DWORD>((std::min)(deadline - now,
                                         static_cast<std::uint64_t>(MAXDWORD - 1)));
}

bool transfer(HANDLE pipe, bool write, void* data, std::size_t size,
              std::uint64_t deadline) noexcept {
    auto* cursor = static_cast<std::uint8_t*>(data);
    std::size_t completed = 0;
    while (completed < size) {
        const DWORD wait = remaining(deadline);
        if (wait == 0 || size - completed > MAXDWORD) return false;
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
            if (WaitForSingleObject(event, wait) == WAIT_OBJECT_0) {
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

} // namespace

bool sendLauncherCommand(const platform::RuntimeIdentity& identity,
                         std::uint64_t absoluteDeadlineMilliseconds,
                         const PeerPolicy& peerPolicy, protocol::LauncherCommand command,
                         protocol::LauncherResponse& response) noexcept {
    response = {};
    if (!identity.mayUseUserEngine() || remaining(absoluteDeadlineMilliseconds) == 0) return false;
    const std::wstring endpoint = platform::makeLocalEndpointName(identity, L"launcher");
    HANDLE pipe = CreateFileW(endpoint.c_str(), GENERIC_READ | GENERIC_WRITE, 0, nullptr,
                              OPEN_EXISTING, FILE_FLAG_OVERLAPPED, nullptr);
    if (pipe == INVALID_HANDLE_VALUE) return false;
    bool success = verifyPipeServer(pipe, identity, peerPolicy);
    DWORD failure = success ? ERROR_SUCCESS : ERROR_ACCESS_DENIED;
    try {
        static std::atomic<std::uint64_t> nextRequestId{1};
        const auto requestId = nextRequestId.fetch_add(1, std::memory_order_relaxed);
        const protocol::LauncherRequest request{
            protocol::Metadata{requestId, 0, 0, identity.sessionId, 0, 0, 0},
            command};
        auto requestBytes = protocol::encode(request);
        std::array<std::uint8_t, protocol::kHeaderSize> header{};
        if (!success) {
            failure = ERROR_ACCESS_DENIED;
        } else if (requestBytes.empty()) {
            success = false;
            failure = ERROR_INVALID_DATA;
        } else if (!transfer(pipe, true, requestBytes.data(), requestBytes.size(),
                             absoluteDeadlineMilliseconds) ||
                   !transfer(pipe, false, header.data(), header.size(),
                             absoluteDeadlineMilliseconds)) {
            success = false;
            failure = ERROR_TIMEOUT;
        } else {
            protocol::MessageType type{};
            protocol::Metadata metadata;
            std::uint32_t bodySize = 0;
            if (!protocol::decodeHeader(header, type, bodySize, metadata)) {
                success = false;
                failure = ERROR_INVALID_DATA;
            } else {
                std::vector<std::uint8_t> responseBytes(header.begin(), header.end());
                responseBytes.resize(protocol::kHeaderSize + bodySize);
                if ((bodySize != 0 &&
                     !transfer(pipe, false,
                               responseBytes.data() + protocol::kHeaderSize, bodySize,
                               absoluteDeadlineMilliseconds))) {
                    success = false;
                    failure = ERROR_TIMEOUT;
                } else {
                    protocol::FrameView frame;
                    protocol::LauncherResponse decoded;
                    success = protocol::decodeFrame(responseBytes, frame) &&
                              protocol::decode(frame, decoded) &&
                              decoded.metadata.responseTo == requestId &&
                              decoded.metadata.sessionId == identity.sessionId;
                    if (success) response = decoded;
                    else failure = ERROR_INVALID_DATA;
                }
            }
        }
    } catch (...) {
        success = false;
        failure = ERROR_NOT_ENOUGH_MEMORY;
    }
    CancelIoEx(pipe, nullptr);
    CloseHandle(pipe);
    SetLastError(success ? ERROR_SUCCESS : failure);
    return success;
}

bool requestLauncherStart(const platform::RuntimeIdentity& identity,
                          std::uint64_t absoluteDeadlineMilliseconds,
                          const PeerPolicy& peerPolicy) noexcept {
    protocol::LauncherResponse response;
    return sendLauncherCommand(identity, absoluteDeadlineMilliseconds, peerPolicy,
                               protocol::LauncherCommand::startDemand, response) &&
           response.status == protocol::Status::ok;
}

} // namespace fcitx::windows::ipc
