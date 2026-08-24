#include "launcher_client.h"

#include "protocol.h"

#include <Windows.h>

#include <array>
#include <atomic>
#include <vector>

namespace fcitx::windows::ipc {
namespace {

extern "C" std::uint8_t fcitx5_windows_common_pipe_transfer(
    void* pipe,
    std::uint8_t write,
    std::uint8_t* data,
    std::size_t size,
    std::uint64_t deadline);
extern "C" void* fcitx5_windows_common_open_pipe_client_utf16(
    const std::uint16_t* pipe_name,
    std::size_t pipe_name_len,
    std::uint64_t deadline,
    std::uint8_t wait_when_busy);
extern "C" void fcitx5_windows_common_close_pipe_client(void* pipe);
extern "C" std::uint8_t fcitx5_windows_common_deadline_has_time(std::uint64_t deadline);

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

bool transfer(HANDLE pipe, bool write, void* data, std::size_t size,
              std::uint64_t deadline) noexcept {
    return fcitx5_windows_common_pipe_transfer(
               pipe, write ? 1 : 0, static_cast<std::uint8_t*>(data), size, deadline) != 0;
}

} // namespace

bool sendLauncherCommand(const platform::RuntimeIdentity& identity,
                         std::uint64_t absoluteDeadlineMilliseconds,
                         const PeerPolicy& peerPolicy, protocol::LauncherCommand command,
                         protocol::LauncherResponse& response) noexcept {
    return sendLauncherCommand(identity, platform::currentRuntimeGeneration(),
                               absoluteDeadlineMilliseconds, peerPolicy, command, response);
}

bool sendLauncherCommand(const platform::RuntimeIdentity& identity,
                         std::wstring_view generation,
                         std::uint64_t absoluteDeadlineMilliseconds,
                         const PeerPolicy& peerPolicy, protocol::LauncherCommand command,
                         protocol::LauncherResponse& response) noexcept {
    response = {};
    if (!identity.mayUseUserEngine() ||
        fcitx5_windows_common_deadline_has_time(absoluteDeadlineMilliseconds) == 0) {
        return false;
    }
    const std::wstring endpoint = platform::makeLocalEndpointName(identity, generation, L"launcher");
    HANDLE pipe = fcitx5_windows_common_open_pipe_client_utf16(
        wideData(endpoint), endpoint.size(), absoluteDeadlineMilliseconds, 0);
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
    fcitx5_windows_common_close_pipe_client(pipe);
    SetLastError(success ? ERROR_SUCCESS : failure);
    return success;
}

bool requestLauncherStart(const platform::RuntimeIdentity& identity,
                          std::uint64_t absoluteDeadlineMilliseconds,
                          const PeerPolicy& peerPolicy) noexcept {
    return requestLauncherStart(identity, platform::currentRuntimeGeneration(),
                                absoluteDeadlineMilliseconds, peerPolicy);
}

bool requestLauncherStart(const platform::RuntimeIdentity& identity,
                          std::wstring_view generation,
                          std::uint64_t absoluteDeadlineMilliseconds,
                          const PeerPolicy& peerPolicy) noexcept {
    protocol::LauncherResponse response;
    return sendLauncherCommand(identity, generation, absoluteDeadlineMilliseconds, peerPolicy,
                               protocol::LauncherCommand::startDemand, response) &&
           response.status == protocol::Status::ok;
}

} // namespace fcitx::windows::ipc
