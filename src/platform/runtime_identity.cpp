#include "runtime_identity.h"

#include <fcitx5_windows/release_identity.h>

#include <sddl.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <cwctype>
#include <vector>

namespace fcitx::windows::platform {
namespace {

class Handle final {
  public:
    explicit Handle(HANDLE value = nullptr) noexcept : value_(value) {}
    ~Handle() {
        if (value_)
            CloseHandle(value_);
    }
    Handle(const Handle&) = delete;
    Handle& operator=(const Handle&) = delete;
    [[nodiscard]] HANDLE get() const noexcept { return value_; }
    [[nodiscard]] explicit operator bool() const noexcept { return value_ != nullptr; }

  private:
    HANDLE value_;
};

bool tokenSid(HANDLE process, std::wstring& sid, bool& serviceAccount) {
    HANDLE rawToken = nullptr;
    if (!OpenProcessToken(process, TOKEN_QUERY, &rawToken))
        return false;
    Handle token(rawToken);
    DWORD required = 0;
    GetTokenInformation(token.get(), TokenUser, nullptr, 0, &required);
    if (required == 0 || required > 64U * 1024U)
        return false;
    std::vector<std::uint8_t> buffer(required);
    if (!GetTokenInformation(token.get(), TokenUser, buffer.data(), required, &required)) {
        return false;
    }
    const auto* user = reinterpret_cast<const TOKEN_USER*>(buffer.data());
    LPWSTR rawSid = nullptr;
    if (!ConvertSidToStringSidW(user->User.Sid, &rawSid))
        return false;
    try {
        sid.assign(rawSid);
    } catch (...) {
        LocalFree(rawSid);
        throw;
    }
    LocalFree(rawSid);
    serviceAccount = IsWellKnownSid(user->User.Sid, WinLocalSystemSid) != FALSE ||
                     IsWellKnownSid(user->User.Sid, WinLocalServiceSid) != FALSE ||
                     IsWellKnownSid(user->User.Sid, WinNetworkServiceSid) != FALSE;
    return true;
}

bool processPath(HANDLE process, std::wstring& path) {
    path.assign(32768, L'\0');
    DWORD length = static_cast<DWORD>(path.size());
    if (!QueryFullProcessImageNameW(process, 0, path.data(), &length) || length == 0) {
        path.clear();
        return false;
    }
    path.resize(length);
    return true;
}

bool secureInputDesktop() {
    HDESK desktop = OpenInputDesktop(0, FALSE, DESKTOP_READOBJECTS);
    if (!desktop)
        return true;
    std::array<wchar_t, 256> name{};
    DWORD required = 0;
    const BOOL read = GetUserObjectInformationW(desktop, UOI_NAME, name.data(),
                                                static_cast<DWORD>(sizeof(name)), &required);
    CloseDesktop(desktop);
    if (!read)
        return true;
    std::wstring lowered(name.data());
    std::transform(lowered.begin(), lowered.end(), lowered.begin(),
                   [](wchar_t value) { return static_cast<wchar_t>(std::towlower(value)); });
    return lowered == L"winlogon" || lowered == L"disconnect";
}

bool validChannel(std::wstring_view channel) {
    if (channel.empty() || channel.size() > 32)
        return false;
    return std::all_of(channel.begin(), channel.end(), [](wchar_t value) {
        return (value >= L'a' && value <= L'z') || (value >= L'0' && value <= L'9') ||
               value == L'-';
    });
}

std::wstring normalized(std::wstring_view input) {
    if (input.empty() || input.size() >= 32768)
        return {};
    std::wstring source(input);
    std::wstring output(32768, L'\0');
    const DWORD length =
        GetFullPathNameW(source.c_str(), static_cast<DWORD>(output.size()), output.data(), nullptr);
    if (length == 0 || length >= output.size())
        return {};
    output.resize(length);
    while (output.size() > 3 && (output.back() == L'\\' || output.back() == L'/')) {
        output.pop_back();
    }
    return output;
}

} // namespace

bool mayLaunchUserEngine(const RuntimeIdentity& identity) noexcept {
    return !identity.serviceAccount && identity.sessionId != 0 && !identity.secureDesktop &&
           !identity.userSid.empty();
}

bool RuntimeIdentity::mayUseUserEngine() const noexcept { return mayLaunchUserEngine(*this); }

bool queryProcessIdentity(DWORD processId, ProcessIdentity& output) noexcept {
    output = {};
    try {
        Handle process(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, processId));
        if (!process)
            return false;
        ProcessIdentity result;
        result.processId = processId;
        if (!ProcessIdToSessionId(processId, &result.sessionId) ||
            !tokenSid(process.get(), result.userSid, result.serviceAccount) ||
            !processPath(process.get(), result.executablePath)) {
            return false;
        }
        output = std::move(result);
        return true;
    } catch (...) {
        output = {};
        return false;
    }
}

bool queryCurrentIdentity(RuntimeIdentity& output) noexcept {
    output = {};
    ProcessIdentity process;
    if (!queryProcessIdentity(GetCurrentProcessId(), process))
        return false;
    try {
        RuntimeIdentity result;
        static_cast<ProcessIdentity&>(result) = std::move(process);
        result.secureDesktop = secureInputDesktop();
        output = std::move(result);
        return true;
    } catch (...) {
        output = {};
        return false;
    }
}

std::wstring localTestNamespace() {
    std::array<wchar_t, 34> value{};
    const DWORD length = GetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", value.data(),
                                                  static_cast<DWORD>(value.size()));
    if (length == 0 || length >= value.size())
        return {};
    const std::wstring_view candidate(value.data(), length);
    return validChannel(candidate) ? std::wstring(candidate) : std::wstring{};
}

std::wstring makeLocalEndpointName(const RuntimeIdentity& identity, std::wstring_view channel) {
    if (identity.userSid.empty() || identity.sessionId == 0 || !validChannel(channel))
        return {};
    const std::wstring testNamespace = localTestNamespace();
    const std::wstring namespacePart =
        testNamespace.empty() ? std::wstring{} : L".Test." + testNamespace;
    return L"\\\\.\\pipe\\" + std::wstring(kReleaseIdentity.pipe_prefix) + L"." + identity.userSid +
           L".Session." + std::to_wstring(identity.sessionId) + namespacePart + L"." +
           std::wstring(channel);
}

std::wstring makeLocalObjectName(const RuntimeIdentity& identity, std::wstring_view channel) {
    if (identity.userSid.empty() || identity.sessionId == 0 || !validChannel(channel))
        return {};
    const std::wstring testNamespace = localTestNamespace();
    const std::wstring namespacePart =
        testNamespace.empty() ? std::wstring{} : L".Test." + testNamespace;
    return L"Local\\" + std::wstring(kReleaseIdentity.local_object_prefix) + L"." +
           identity.userSid + L".Session." + std::to_wstring(identity.sessionId) +
           namespacePart + L"." + std::wstring(channel);
}

bool pathsReferToSameFile(std::wstring_view left, std::wstring_view right) noexcept {
    try {
        const std::wstring normalizedLeft = normalized(left);
        const std::wstring normalizedRight = normalized(right);
        return !normalizedLeft.empty() && !normalizedRight.empty() &&
               CompareStringOrdinal(normalizedLeft.c_str(), static_cast<int>(normalizedLeft.size()),
                                    normalizedRight.c_str(),
                                    static_cast<int>(normalizedRight.size()), TRUE) == CSTR_EQUAL;
    } catch (...) {
        return false;
    }
}

} // namespace fcitx::windows::platform
