#include "pipe_security.h"
#include "peer_verification.h"
#include "runtime_identity.h"

#include <Windows.h>
#include <sddl.h>

#include <filesystem>
#include <fstream>
#include <iostream>

int main() {
    using namespace fcitx::windows::platform;
    RuntimeIdentity identity;
    if (!queryCurrentIdentity(identity) || identity.processId != GetCurrentProcessId() ||
        identity.userSid.rfind(L"S-1-", 0) != 0 || identity.sessionId == 0 ||
        identity.executablePath.empty()) {
        std::cerr << "current process identity query failed\n";
        return 1;
    }
    RuntimeIdentity synthetic = identity;
    synthetic.serviceAccount = true;
    if (mayLaunchUserEngine(synthetic)) {
        std::cerr << "service identity was allowed to launch user engine\n";
        return 1;
    }
    synthetic = identity;
    synthetic.sessionId = 0;
    if (mayLaunchUserEngine(synthetic)) {
        std::cerr << "session zero identity was allowed to launch user engine\n";
        return 1;
    }
    synthetic = identity;
    synthetic.secureDesktop = true;
    if (mayLaunchUserEngine(synthetic)) {
        std::cerr << "secure desktop identity was allowed to launch user engine\n";
        return 1;
    }
    if (!SetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", L"../invalid") ||
        !localTestNamespace().empty() ||
        !SetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", L"contract-42") ||
        localTestNamespace() != L"contract-42") {
        std::cerr << "test namespace validation failed\n";
        return 1;
    }
    if (!SetEnvironmentVariableW(L"FCITX5_RELEASE_GENERATION", nullptr) ||
        currentRuntimeGeneration() != L"current" ||
        !SetEnvironmentVariableW(L"FCITX5_RELEASE_GENERATION", L"../bad") ||
        currentRuntimeGeneration() != L"current" ||
        !SetEnvironmentVariableW(L"FCITX5_RELEASE_GENERATION", L"00000042") ||
        currentRuntimeGeneration() != L"00000042") {
        std::cerr << "runtime generation validation failed\n";
        return 1;
    }
    const std::wstring isolatedEndpoint = makeLocalEndpointName(identity, L"engine");
    const std::wstring isolatedObject = makeLocalObjectName(identity, L"candidate-42");
    SetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", nullptr);
    SetEnvironmentVariableW(L"FCITX5_RELEASE_GENERATION", nullptr);
    const auto generationRoot =
        std::filesystem::temp_directory_path() /
        (L"fcitx5-runtime-identity-" + std::to_wstring(GetCurrentProcessId()));
    std::error_code cleanupError;
    std::filesystem::remove_all(generationRoot, cleanupError);
    std::filesystem::create_directories(generationRoot / L"tsf" / L"x64");
    std::filesystem::create_directories(generationRoot / L"runtime" / L"00000041" / L"bin");
    {
        std::ofstream portable(generationRoot / L"portable.flag", std::ios::binary);
        portable << "portable\n";
    }
    {
        std::ofstream current(generationRoot / L"current.json", std::ios::binary);
        current << "{\n"
                << "  \"format_version\": 1,\n"
                << "  \"current_generation\": \"00000042\",\n"
                << "  \"previous_generation\": \"00000041\",\n"
                << "  \"build_id\": \"build-42\"\n"
                << "}\n";
    }
    {
        std::ofstream sidecar(generationRoot / L"tsf" / L"x64" / L"fcitx5-tsf.generation",
                              std::ios::binary);
        sidecar << "00000044\n";
    }
    const auto tsfModule = generationRoot / L"tsf" / L"x64" / L"fcitx5-tsf.dll";
    const auto runtimeModule = generationRoot / L"runtime" / L"00000041" /
                               L"fcitx5-engine.exe";
    const auto runtimeBinModule = generationRoot / L"runtime" / L"00000041" / L"bin" /
                                  L"fcitx5-engine.exe";
    if (currentRuntimeGenerationFromInstallRoot(generationRoot.wstring()) != L"00000042" ||
        currentRuntimeGenerationForModule(tsfModule.wstring()) != L"00000044" ||
        currentRuntimeGenerationForModule(runtimeModule.wstring()) != L"00000041" ||
        currentRuntimeGenerationForModule(runtimeBinModule.wstring()) != L"00000041" ||
        installationRootForModule(runtimeBinModule.wstring()) != generationRoot ||
        portableDataRootForModule(runtimeBinModule.wstring()) != generationRoot / L"data") {
        std::filesystem::remove_all(generationRoot, cleanupError);
        std::cerr << "installed runtime generation discovery failed\n";
        return 1;
    }
    if (!SetEnvironmentVariableW(L"FCITX5_RELEASE_GENERATION", L"00000043") ||
        currentRuntimeGenerationForModule(tsfModule.wstring()) != L"00000043" ||
        currentRuntimeGenerationFromInstallRoot(generationRoot.wstring()) != L"00000043") {
        std::filesystem::remove_all(generationRoot, cleanupError);
        std::cerr << "explicit runtime generation override failed\n";
        return 1;
    }
    SetEnvironmentVariableW(L"FCITX5_RELEASE_GENERATION", nullptr);
    std::filesystem::remove_all(generationRoot, cleanupError);
    if (isolatedEndpoint.find(L".Test.contract-42.engine") == std::wstring::npos ||
        isolatedObject.find(L".Test.contract-42.candidate-42") == std::wstring::npos ||
        isolatedEndpoint.find(L".Generation.00000042.") == std::wstring::npos ||
        isolatedObject.find(L".Generation.00000042.") == std::wstring::npos) {
        std::cerr << "test namespace isolation was not applied\n";
        return 1;
    }
    const std::wstring endpoint = makeLocalEndpointName(identity, L"engine");
    const std::wstring generation41 = makeLocalEndpointName(identity, L"00000041", L"engine");
    const std::wstring generation42 = makeLocalEndpointName(identity, L"00000042", L"engine");
    if (endpoint.find(identity.userSid) == std::wstring::npos ||
        endpoint.find(std::to_wstring(identity.sessionId)) == std::wstring::npos ||
        endpoint.find(L".Generation.current.engine") == std::wstring::npos ||
        generation41 == generation42 ||
        generation41.find(L".Generation.00000041.engine") == std::wstring::npos ||
        generation42.find(L".Generation.00000042.engine") == std::wstring::npos ||
        !makeLocalEndpointName(identity, L"../bad", L"engine").empty() ||
        !makeLocalEndpointName(identity, L"../bad").empty() ||
        !makeLocalObjectName(identity, L"../bad").empty()) {
        std::cerr << "endpoint namespace validation failed\n";
        return 1;
    }
    PipeSecurity security;
    if (!PipeSecurity::create(identity, security) || !security.valid() ||
        !security.attributes()->lpSecurityDescriptor) {
        std::cerr << "pipe security descriptor creation failed\n";
        return 1;
    }
    SECURITY_DESCRIPTOR_CONTROL control = 0;
    DWORD revision = 0;
    LPWSTR rawSddl = nullptr;
    if (!GetSecurityDescriptorControl(
            security.attributes()->lpSecurityDescriptor, &control, &revision) ||
        (control & SE_DACL_PROTECTED) == 0 || security.attributes()->bInheritHandle ||
        !ConvertSecurityDescriptorToStringSecurityDescriptorW(
            security.attributes()->lpSecurityDescriptor, SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION | LABEL_SECURITY_INFORMATION, &rawSddl, nullptr)) {
        std::cerr << "pipe security controls could not be inspected\n";
        return 1;
    }
    const std::wstring sddl(rawSddl);
    LocalFree(rawSddl);
    if (sddl.find(L"(A;;GA;;;SY)") == std::wstring::npos ||
        sddl.find(L"S:(ML;;NW;;;ME)") == std::wstring::npos) {
        std::cerr << "pipe DACL/SACL did not contain the required principals\n";
        return 1;
    }
    // ConvertSecurityDescriptorToStringSecurityDescriptorW abbreviates
    // well-known SIDs (SY for S-1-5-18, LS for S-1-5-19, NS for S-1-5-20),
    // so the literal user SID string may not appear in the SDDL text on
    // service-style runners. Verify the user ACE by parsing the DACL and
    // comparing SIDs instead of relying on the textual form.
    {
        PACL dacl = nullptr;
        BOOL daclPresent = FALSE;
        BOOL daclDefaulted = FALSE;
        if (!GetSecurityDescriptorDacl(security.attributes()->lpSecurityDescriptor,
                                       &daclPresent, &dacl, &daclDefaulted) ||
            !daclPresent || !dacl) {
            std::cerr << "pipe security descriptor has no DACL\n";
            return 1;
        }
        ACL_SIZE_INFORMATION aclInfo{};
        if (!GetAclInformation(dacl, &aclInfo, sizeof(aclInfo), AclSizeInformation)) {
            std::cerr << "pipe DACL could not be enumerated\n";
            return 1;
        }
        PSID expectedSid = nullptr;
        if (!ConvertStringSidToSidW(identity.userSid.c_str(), &expectedSid)) {
            std::cerr << "user SID could not be parsed\n";
            return 1;
        }
        bool userAceFound = false;
        for (DWORD index = 0; index < aclInfo.AceCount; ++index) {
            LPVOID ace = nullptr;
            if (!GetAce(dacl, index, &ace)) continue;
            const auto* header = static_cast<const ACE_HEADER*>(ace);
            if (header->AceType == ACCESS_ALLOWED_ACE_TYPE ||
                header->AceType == ACCESS_ALLOWED_OBJECT_ACE_TYPE) {
                const auto* allowed = static_cast<const ACCESS_ALLOWED_ACE*>(ace);
                if (EqualSid(reinterpret_cast<PSID>(const_cast<DWORD*>(&allowed->SidStart)),
                             expectedSid)) {
                    userAceFound = true;
                    break;
                }
            }
        }
        LocalFree(expectedSid);
        if (!userAceFound) {
            std::cerr << "pipe DACL did not grant the current user\n";
            return 1;
        }
    }
    if (!pathsReferToSameFile(identity.executablePath, identity.executablePath) ||
        pathsReferToSameFile(identity.executablePath, L"C:\\definitely-not-this-file.exe")) {
        std::cerr << "path identity comparison failed\n";
        return 1;
    }
    ExecutableFileIdentity currentExecutable;
    if (!queryExecutableFileIdentity(identity.executablePath, currentExecutable) ||
        !executablePathsMatch(identity.executablePath, identity.executablePath)) {
        std::cerr << "current executable identity comparison failed\n";
        return 1;
    }
    const auto identityRoot =
        std::filesystem::temp_directory_path() /
        (L"fcitx5-peer-identity-" + std::to_wstring(GetCurrentProcessId()));
    std::filesystem::remove_all(identityRoot, cleanupError);
    std::filesystem::create_directories(identityRoot);
    const auto original = identityRoot / L"peer.exe";
    const auto hardlink = identityRoot / L"peer-hardlink.exe";
    const auto copy = identityRoot / L"peer-copy.exe";
    const auto symlink = identityRoot / L"peer-symlink.exe";
    {
        std::ofstream output(original, std::ios::binary);
        output << "peer identity fixture\n";
    }
    std::filesystem::copy_file(original, copy);
    if (!executablePathsMatch(original.wstring(), original.wstring()) ||
        executablePathsMatch(original.wstring(), copy.wstring())) {
        std::filesystem::remove_all(identityRoot, cleanupError);
        std::cerr << "REG-PEER-ID-001 executable identity did not distinguish a copied peer\n";
        return 1;
    }
    if (!CreateHardLinkW(hardlink.c_str(), original.c_str(), nullptr)) {
        std::filesystem::remove_all(identityRoot, cleanupError);
        std::cerr << "hardlink fixture creation failed\n";
        return 1;
    }
    if (!pathsReferToSameFile(original.wstring(), hardlink.wstring()) ||
        pathsReferToSameFile(original.wstring(), copy.wstring()) ||
        executablePathsMatch(original.wstring(), hardlink.wstring())) {
        std::filesystem::remove_all(identityRoot, cleanupError);
        std::cerr << "REG-PEER-ID-001 handle file identity comparison failed\n";
        return 1;
    }
    if (CreateSymbolicLinkW(symlink.c_str(), original.c_str(),
                            SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE) ||
        CreateSymbolicLinkW(symlink.c_str(), original.c_str(), 0)) {
        if (!pathsReferToSameFile(original.wstring(), symlink.wstring()) ||
            executablePathsMatch(original.wstring(), symlink.wstring())) {
            std::filesystem::remove_all(identityRoot, cleanupError);
            std::cerr << "REG-PEER-ID-001 reparse executable identity comparison failed\n";
            return 1;
        }
    }

    const std::wstring peerPipeName =
        L"\\\\.\\pipe\\Fcitx5WindowsNext.PeerIdentity.Unit." +
        std::to_wstring(GetCurrentProcessId());
    HANDLE peerPipe = CreateNamedPipeW(peerPipeName.c_str(), PIPE_ACCESS_DUPLEX,
                                       PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT, 1,
                                       4096, 4096, 0, nullptr);
    HANDLE peerClient = CreateFileW(peerPipeName.c_str(), GENERIC_READ | GENERIC_WRITE, 0,
                                    nullptr, OPEN_EXISTING, 0, nullptr);
    const bool peerConnected =
        peerPipe != INVALID_HANDLE_VALUE && peerClient != INVALID_HANDLE_VALUE &&
        (ConnectNamedPipe(peerPipe, nullptr) != FALSE ||
         GetLastError() == ERROR_PIPE_CONNECTED);
    const auto executableCopy = identityRoot / L"runtime-identity-copy.exe";
    std::filesystem::copy_file(identity.executablePath, executableCopy,
                               std::filesystem::copy_options::overwrite_existing);
    const auto executableSymlink = identityRoot / L"runtime-identity-symlink.exe";
    const bool executableSymlinkCreated =
        CreateSymbolicLinkW(executableSymlink.c_str(), identity.executablePath.c_str(),
                            SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE) ||
        CreateSymbolicLinkW(executableSymlink.c_str(), identity.executablePath.c_str(), 0);
    const bool exactPeerAccepted =
        peerConnected &&
        fcitx::windows::ipc::verifyPipeServer(
            peerClient, identity,
            fcitx::windows::ipc::PeerPolicy::exact(identity.executablePath));
    const bool copyPeerAccepted =
        peerConnected &&
        fcitx::windows::ipc::verifyPipeServer(
            peerClient, identity,
            fcitx::windows::ipc::PeerPolicy::exact(executableCopy.wstring()));
    const bool symlinkPeerAccepted =
        executableSymlinkCreated && peerConnected &&
        fcitx::windows::ipc::verifyPipeServer(
            peerClient, identity,
            fcitx::windows::ipc::PeerPolicy::exact(executableSymlink.wstring()));
    const auto executableHardlink = identityRoot / L"runtime-identity-hardlink.exe";
    const bool executableHardlinkCreated =
        CreateHardLinkW(executableHardlink.c_str(), identity.executablePath.c_str(), nullptr) !=
        FALSE;
    const bool hardlinkPeerAccepted =
        executableHardlinkCreated && peerConnected &&
        fcitx::windows::ipc::verifyPipeServer(
            peerClient, identity,
            fcitx::windows::ipc::PeerPolicy::exact(executableHardlink.wstring()));
    if (peerClient != INVALID_HANDLE_VALUE)
        CloseHandle(peerClient);
    if (peerPipe != INVALID_HANDLE_VALUE) {
        DisconnectNamedPipe(peerPipe);
        CloseHandle(peerPipe);
    }
    if (!exactPeerAccepted || copyPeerAccepted || symlinkPeerAccepted ||
        hardlinkPeerAccepted) {
        std::filesystem::remove_all(identityRoot, cleanupError);
        std::cerr << "REG-PEER-ID-001 exact peer executable verification failed\n";
        return 1;
    }
    std::filesystem::remove_all(identityRoot, cleanupError);
    return 0;
}
