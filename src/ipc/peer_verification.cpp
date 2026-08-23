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
extern "C" std::uint8_t fcitx5_windows_common_peer_development_policy_allowed(
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

bool developmentPeerExceptionAllowed() noexcept {
#if defined(FCITX_DEVELOPMENT_PEER_EXCEPTION)
    return fcitx5_windows_common_peer_development_policy_allowed(1) != 0;
#else
    return fcitx5_windows_common_peer_development_policy_allowed(0) != 0;
#endif
}

} // namespace

bool verifyPipeServer(HANDLE pipe, const platform::RuntimeIdentity& clientIdentity,
                      const PeerPolicy& policy) noexcept {
    if (!pipe || pipe == INVALID_HANDLE_VALUE || !clientIdentity.mayUseUserEngine()) return false;
    ULONG serverProcessId = 0;
    if (!GetNamedPipeServerProcessId(pipe, &serverProcessId) || serverProcessId == 0) return false;
    platform::ProcessIdentity server;
    if (!platform::queryProcessIdentity(serverProcessId, server) ||
        !samePrincipalAndSession(server, clientIdentity)) {
        return false;
    }
    if (policy.mode == PeerVerificationMode::developmentSameUserSession) {
        return developmentPeerExceptionAllowed();
    }
    platform::ExecutableFileIdentity expected;
    return !policy.expectedExecutablePath.empty() && server.executableFileVerified &&
           platform::queryExecutableFileIdentity(policy.expectedExecutablePath, expected) &&
           platform::executableFilesMatch(server.executableFile, expected);
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
