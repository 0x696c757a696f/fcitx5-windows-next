#include "peer_verification.h"

#include <cstdint>
#include <utility>

namespace fcitx::windows::ipc {
namespace {

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
struct Fcitx5WindowsCommonVerifiedPipeClient {
    std::uint8_t status;
    std::uint8_t serviceAccount;
    std::uint8_t executableFileStatus;
    std::uint8_t executableFileContainsReparsePoint;
    std::uint32_t processId;
    std::uint32_t sessionId;
    std::uint32_t executableFileVolumeSerialNumber;
    std::uint32_t executableFileIndexHigh;
    std::uint32_t executableFileIndexLow;
    std::uint32_t executableFileNumberOfLinks;
    std::size_t userSidLen;
    std::size_t executablePathLen;
    std::size_t executableFinalPathLen;
};
extern "C" Fcitx5WindowsCommonVerifiedPipeClient
fcitx5_windows_common_verify_pipe_client_peer_utf16(
    void* pipe,
    std::uint8_t current_service_account,
    std::uint32_t current_session_id,
    std::uint8_t current_secure_desktop,
    const std::uint16_t* current_user_sid,
    std::size_t current_user_sid_len,
    std::uint16_t* user_sid_output,
    std::size_t user_sid_capacity,
    std::uint16_t* executable_path_output,
    std::size_t executable_path_capacity,
    std::uint16_t* executable_final_path_output,
    std::size_t executable_final_path_capacity);

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

bool developmentPeerExceptionEnabled() noexcept {
#if defined(FCITX_DEVELOPMENT_PEER_EXCEPTION)
    return true;
#else
    return false;
#endif
}

bool copyExecutableFileIdentity(const Fcitx5WindowsCommonVerifiedPipeClient& identity,
                                std::wstring& finalPath,
                                platform::ExecutableFileIdentity& output) {
    output = {};
    if (identity.executableFileStatus == 0) {
        return false;
    }
    if (identity.executableFinalPathLen == 0 ||
        identity.executableFinalPathLen > finalPath.size()) {
        return false;
    }
    finalPath.resize(identity.executableFinalPathLen);
    output.volumeSerialNumber = identity.executableFileVolumeSerialNumber;
    output.fileIndexHigh = identity.executableFileIndexHigh;
    output.fileIndexLow = identity.executableFileIndexLow;
    output.numberOfLinks = identity.executableFileNumberOfLinks;
    output.containsReparsePoint = identity.executableFileContainsReparsePoint != 0;
    output.finalPath = std::move(finalPath);
    return true;
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
    try {
        const auto query = fcitx5_windows_common_verify_pipe_client_peer_utf16(
            pipe, serverIdentity.serviceAccount ? 1 : 0, serverIdentity.sessionId,
            serverIdentity.secureDesktop ? 1 : 0, wideData(serverIdentity.userSid),
            serverIdentity.userSid.size(), nullptr, 0, nullptr, 0, nullptr, 0);
        if (query.status == 0 || query.userSidLen == 0 || query.executablePathLen == 0) {
            return false;
        }
        platform::ProcessIdentity client;
        client.userSid.assign(query.userSidLen, L'\0');
        client.executablePath.assign(query.executablePathLen, L'\0');
        std::wstring executableFinalPath(query.executableFinalPathLen, L'\0');
        const auto filled = fcitx5_windows_common_verify_pipe_client_peer_utf16(
            pipe, serverIdentity.serviceAccount ? 1 : 0, serverIdentity.sessionId,
            serverIdentity.secureDesktop ? 1 : 0, wideData(serverIdentity.userSid),
            serverIdentity.userSid.size(), reinterpret_cast<std::uint16_t*>(client.userSid.data()),
            client.userSid.size(), reinterpret_cast<std::uint16_t*>(client.executablePath.data()),
            client.executablePath.size(),
            executableFinalPath.empty()
                ? nullptr
                : reinterpret_cast<std::uint16_t*>(executableFinalPath.data()),
            executableFinalPath.size());
        if (filled.status == 0 || filled.userSidLen != client.userSid.size() ||
            filled.executablePathLen != client.executablePath.size()) {
            return false;
        }
        client.processId = filled.processId;
        client.sessionId = filled.sessionId;
        client.serviceAccount = filled.serviceAccount != 0;
        client.executableFileVerified =
            copyExecutableFileIdentity(filled, executableFinalPath, client.executableFile);
        if (verifiedClient) *verifiedClient = std::move(client);
        return true;
    } catch (...) {
        if (verifiedClient) *verifiedClient = {};
        return false;
    }
}

} // namespace fcitx::windows::ipc
