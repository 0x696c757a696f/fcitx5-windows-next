#include "launcher_client.h"

#include "protocol.h"

#include <Windows.h>

#include <vector>

namespace fcitx::windows::ipc {
namespace {

struct Fcitx5WindowsCommonPipeTransactWithError {
    std::uint8_t status;
    std::uint32_t failureError;
    std::size_t responseLen;
};
extern "C" Fcitx5WindowsCommonPipeTransactWithError
fcitx5_windows_common_pipe_transact_with_error(
    void* pipe,
    const std::uint8_t* request,
    std::size_t request_len,
    std::uint8_t* response_output,
    std::size_t response_capacity,
    std::uint64_t deadline);
extern "C" void* fcitx5_windows_common_open_pipe_client_utf16(
    const std::uint16_t* pipe_name,
    std::size_t pipe_name_len,
    std::uint64_t deadline,
    std::uint8_t wait_when_busy);
extern "C" void fcitx5_windows_common_close_pipe_client(void* pipe);
extern "C" std::uint8_t fcitx5_windows_common_deadline_has_time(std::uint64_t deadline);
extern "C" std::uint8_t fcitx5_windows_common_accept_launcher_response(
    std::uint64_t response_to,
    std::uint32_t session_id,
    std::uint64_t expected_request_id,
    std::uint32_t expected_session_id);
extern "C" std::uint64_t fcitx5_windows_common_next_launcher_request_id();
extern "C" std::uint8_t fcitx5_windows_common_ipc_status_ok(std::uint32_t status);
extern "C" void fcitx5_windows_common_set_last_error(std::uint32_t error);

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
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
        const auto requestId = fcitx5_windows_common_next_launcher_request_id();
        const protocol::LauncherRequest request{
            protocol::Metadata{requestId, 0, 0, identity.sessionId, 0, 0, 0},
            command};
        auto requestBytes = protocol::encode(request);
        if (!success) {
            failure = ERROR_ACCESS_DENIED;
        } else {
            std::vector<std::uint8_t> responseBytes(protocol::kMaxFrameSize, 0);
            const auto transferred = fcitx5_windows_common_pipe_transact_with_error(
                pipe, requestBytes.data(), requestBytes.size(), responseBytes.data(),
                responseBytes.size(), absoluteDeadlineMilliseconds);
            if (transferred.status == 0) {
                success = false;
                failure = transferred.failureError == ERROR_SUCCESS ? ERROR_TIMEOUT
                                                                    : transferred.failureError;
            } else {
                responseBytes.resize(transferred.responseLen);
                protocol::FrameView frame;
                protocol::LauncherResponse decoded;
                success = protocol::decodeFrame(responseBytes, frame) &&
                          protocol::decode(frame, decoded) &&
                          fcitx5_windows_common_accept_launcher_response(
                              decoded.metadata.responseTo, decoded.metadata.sessionId,
                              requestId, identity.sessionId) != 0;
                if (success) response = decoded;
                else failure = ERROR_INVALID_DATA;
            }
        }
    } catch (...) {
        success = false;
        failure = ERROR_NOT_ENOUGH_MEMORY;
    }
    fcitx5_windows_common_close_pipe_client(pipe);
    fcitx5_windows_common_set_last_error(success ? ERROR_SUCCESS : failure);
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
           fcitx5_windows_common_ipc_status_ok(static_cast<std::uint32_t>(response.status)) != 0;
}

} // namespace fcitx::windows::ipc
