#include "peer_verification.h"

#include <utility>

namespace fcitx::windows::ipc {
namespace {

bool samePrincipalAndSession(const platform::ProcessIdentity& peer,
                             const platform::RuntimeIdentity& current) noexcept {
    return peer.sessionId == current.sessionId && !peer.serviceAccount &&
           CompareStringOrdinal(peer.userSid.c_str(), static_cast<int>(peer.userSid.size()),
                                current.userSid.c_str(), static_cast<int>(current.userSid.size()),
                                TRUE) == CSTR_EQUAL;
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
#if defined(FCITX_DEVELOPMENT_PEER_EXCEPTION)
        return true;
#else
        return false;
#endif
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
