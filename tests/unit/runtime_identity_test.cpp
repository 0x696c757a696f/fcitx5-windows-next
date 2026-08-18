#include "pipe_security.h"
#include "runtime_identity.h"

#include <Windows.h>
#include <sddl.h>

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
    const std::wstring isolatedEndpoint = makeLocalEndpointName(identity, L"engine");
    const std::wstring isolatedObject = makeLocalObjectName(identity, L"candidate-42");
    SetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", nullptr);
    if (isolatedEndpoint.find(L".Test.contract-42.engine") == std::wstring::npos ||
        isolatedObject.find(L".Test.contract-42.candidate-42") == std::wstring::npos) {
        std::cerr << "test namespace isolation was not applied\n";
        return 1;
    }
    const std::wstring endpoint = makeLocalEndpointName(identity, L"engine");
    if (endpoint.find(identity.userSid) == std::wstring::npos ||
        endpoint.find(std::to_wstring(identity.sessionId)) == std::wstring::npos ||
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
        sddl.find(identity.userSid) == std::wstring::npos ||
        sddl.find(L"S:(ML;;NW;;;ME)") == std::wstring::npos) {
        std::cerr << "pipe DACL/SACL did not contain the required principals\n";
        return 1;
    }
    if (!pathsReferToSameFile(identity.executablePath, identity.executablePath) ||
        pathsReferToSameFile(identity.executablePath, L"C:\\definitely-not-this-file.exe")) {
        std::cerr << "path identity comparison failed\n";
        return 1;
    }
    return 0;
}
