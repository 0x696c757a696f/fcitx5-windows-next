#include "peer_verification.h"

#include <cstdint>
#include <utility>

namespace fcitx::windows::ipc {
namespace {

extern "C" std::uint8_t fcitx5_windows_common_same_principal_session_utf16(
    std::uint32_t peer_session_id,
    std::uint8_t peer_service_account,
    const std::uint16_t* peer_user_sid,
    std::size_t peer_user_sid_len,
    std::uint32_t current_session_id,
    const std::uint16_t* current_user_sid,
    std::size_t current_user_sid_len);
extern "C" std::uint8_t fcitx5_windows_common_verify_pipe_server_peer_utf16(
    void* pipe,
    std::uint8_t current_service_account,
    std::uint32_t current_session_id,
    std::uint8_t current_secure_desktop,
    const std::uint16_t* current_user_sid,
    std::size_t current_user_sid_len,
    std::uint32_t policy_mode,
    const std::uint16_t* expected_executable_path,
    std::size_t expected_executable_path_len,
    std::uint8_t development_exception_enabled);

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

bool samePrincipalAndSession(const platform::ProcessIdentity& peer,
                             const platform::RuntimeIdentity& current) noexcept {
    return fcitx5_windows_common_same_principal_session_utf16(
               peer.sessionId, peer.serviceAccount ? 1 : 0, wideData(peer.userSid),
               peer.userSid.size(), current.sessionId, wideData(current.userSid),
               current.userSid.size()) != 0;
}

bool developmentPeerExceptionEnabled() noexcept {
#if defined(FCITX_DEVELOPMENT_PEER_EXCEPTION)
    return true;
#else
    return false;
#endif
}

} // namespace

bool verifyPipeServer(HANDLE pipe, const platform::RuntimeIdentity& clientIdentity,
                      const PeerPolicy& policy) noexcept {
    return fcitx5_windows_common_verify_pipe_server_peer_utf16(
               pipe, clientIdentity.serviceAccount ? 1 : 0, clientIdentity.sessionId,
               clientIdentity.secureDesktop ? 1 : 0, wideData(clientIdentity.userSid),
               clientIdentity.userSid.size(), static_cast<std::uint32_t>(policy.mode),
               wideData(policy.expectedExecutablePath), policy.expectedExecutablePath.size(),
               developmentPeerExceptionEnabled() ? 1 : 0) != 0;
}

bool verifyPipeClient(HANDLE pipe, const platform::RuntimeIdentity& serverIdentity,
                      platform::ProcessIdentity* verifiedClient) noexcept {
    if (verifiedClient) *verifiedClient = {};
    if (!pipe || pipe == INVALID_HANDLE_VALUE || !serverIdentity.mayUseUserEngine()) return false;
    ULONG clientProcessId = 0;
    if (!GetNamedPipeClientProcessId(pipe, &clientProcessId) || clientProcessId == 0) return false;
    platform::ProcessIdentity client;
    if (!platform::queryProcessIdentity(clientProcessId, client) ||
        !samePrincipalAndSession(client, serverIdentity)) {
        return false;
    }
    if (verifiedClient) *verifiedClient = std::move(client);
    return true;
}

} // namespace fcitx::windows::ipc
