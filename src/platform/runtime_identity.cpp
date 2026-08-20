#include "runtime_identity.h"

#include <fcitx5_windows/release_identity.h>

#include <sddl.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <cwctype>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <string>
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

bool validGeneration(std::wstring_view generation) {
    if (generation.empty() || generation.size() > 32)
        return false;
    return std::all_of(generation.begin(), generation.end(), [](wchar_t value) {
        return (value >= L'a' && value <= L'z') || (value >= L'0' && value <= L'9') ||
               value == L'-' || value == L'_';
    });
}

std::wstring environmentGeneration() {
    std::array<wchar_t, 34> value{};
    const DWORD length = GetEnvironmentVariableW(L"FCITX5_RELEASE_GENERATION", value.data(),
                                                  static_cast<DWORD>(value.size()));
    if (length == 0 || length >= value.size())
        return {};
    const std::wstring_view candidate(value.data(), length);
    return validGeneration(candidate) ? std::wstring(candidate) : std::wstring{};
}

std::string readSmallTextFile(const std::filesystem::path& path) {
    std::error_code error;
    const auto size = std::filesystem::file_size(path, error);
    if (error || size > 64U * 1024U)
        return {};
    std::ifstream input(path, std::ios::binary);
    if (!input)
        return {};
    std::string bytes(std::istreambuf_iterator<char>(input), {});
    return (input.good() || input.eof()) ? bytes : std::string{};
}

std::wstring widenAscii(std::string_view value) {
    std::wstring result;
    result.reserve(value.size());
    for (const unsigned char character : value) {
        if (character > 0x7fU)
            return {};
        result.push_back(static_cast<wchar_t>(character));
    }
    return result;
}

std::wstring parsePlainGeneration(std::string_view bytes) {
    while (!bytes.empty() &&
           (bytes.back() == '\n' || bytes.back() == '\r' || bytes.back() == ' ' ||
            bytes.back() == '\t')) {
        bytes.remove_suffix(1);
    }
    while (!bytes.empty() &&
           (bytes.front() == '\n' || bytes.front() == '\r' || bytes.front() == ' ' ||
            bytes.front() == '\t')) {
        bytes.remove_prefix(1);
    }
    const std::wstring candidate = widenAscii(bytes);
    return validGeneration(candidate) ? candidate : std::wstring{};
}

std::wstring parseCurrentGeneration(std::string_view json) {
    constexpr std::string_view key = "\"current_generation\"";
    const std::size_t keyPosition = json.find(key);
    if (keyPosition == std::string_view::npos)
        return {};
    const std::size_t colon = json.find(':', keyPosition + key.size());
    if (colon == std::string_view::npos)
        return {};
    std::size_t quote = colon + 1;
    while (quote < json.size() &&
           (json[quote] == ' ' || json[quote] == '\t' || json[quote] == '\r' ||
            json[quote] == '\n')) {
        ++quote;
    }
    if (quote >= json.size() || json[quote] != '"')
        return {};
    const std::size_t begin = quote + 1;
    const std::size_t end = json.find('"', begin);
    if (end == std::string_view::npos || end == begin)
        return {};
    const std::wstring candidate = widenAscii(json.substr(begin, end - begin));
    return validGeneration(candidate) ? candidate : std::wstring{};
}

std::wstring leafLower(const std::filesystem::path& path) {
    std::wstring leaf = path.filename().wstring();
    std::transform(leaf.begin(), leaf.end(), leaf.begin(),
                   [](wchar_t value) { return static_cast<wchar_t>(std::towlower(value)); });
    return leaf;
}

std::filesystem::path installRootForModule(const std::filesystem::path& modulePath) {
    const auto directory = modulePath.parent_path();
    if (directory.empty())
        return {};
    const auto parentName = leafLower(directory);
    const auto grandParentName = leafLower(directory.parent_path());
    if ((parentName == L"x64" || parentName == L"x86") && grandParentName == L"tsf") {
        return directory.parent_path().parent_path();
    }
    if (parentName == L"bin" && validGeneration(directory.parent_path().filename().wstring()) &&
        leafLower(directory.parent_path().parent_path()) == L"runtime") {
        return directory.parent_path().parent_path().parent_path();
    }
    if (parentName == L"bin" || parentName == L"management") {
        return directory.parent_path();
    }
    if (validGeneration(directory.filename().wstring()) && grandParentName == L"runtime") {
        return directory.parent_path().parent_path();
    }
    return {};
}

std::wstring runtimeGenerationForRuntimeModule(const std::filesystem::path& modulePath) {
    const auto directory = modulePath.parent_path();
    if (directory.empty())
        return {};
    if (leafLower(directory) == L"bin" &&
        validGeneration(directory.parent_path().filename().wstring()) &&
        leafLower(directory.parent_path().parent_path()) == L"runtime") {
        return directory.parent_path().filename().wstring();
    }
    if (leafLower(directory.parent_path()) != L"runtime")
        return {};
    const auto generation = directory.filename().wstring();
    return validGeneration(generation) ? generation : std::wstring{};
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

std::wstring currentRuntimeGeneration() {
    if (const std::wstring generation = environmentGeneration(); !generation.empty())
        return generation;
    std::wstring path(32'768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    if (size > 0 && size < path.size()) {
        path.resize(size);
        if (const std::wstring generation = currentRuntimeGenerationForModule(path);
            !generation.empty()) {
            return generation;
        }
    }
    return L"current";
}

std::wstring currentRuntimeGenerationForModule(std::wstring_view modulePath) {
    if (const std::wstring generation = environmentGeneration(); !generation.empty())
        return generation;
    const std::filesystem::path module{std::wstring(modulePath)};
    if (module.empty())
        return {};
    if (const std::wstring generation = runtimeGenerationForRuntimeModule(module);
        !generation.empty()) {
        return generation;
    }
    if (const std::wstring generation =
            parsePlainGeneration(readSmallTextFile(module.parent_path() /
                                                   L"fcitx5-tsf.generation"));
        !generation.empty()) {
        return generation;
    }
    const auto root = installRootForModule(module);
    return root.empty() ? std::wstring{} : currentRuntimeGenerationFromInstallRoot(root.wstring());
}

std::wstring currentRuntimeGenerationFromInstallRoot(std::wstring_view installRoot) {
    if (const std::wstring generation = environmentGeneration(); !generation.empty())
        return generation;
    const std::filesystem::path root{std::wstring(installRoot)};
    if (root.empty())
        return {};
    return parseCurrentGeneration(readSmallTextFile(root / L"current.json"));
}

std::filesystem::path installationRootForModule(std::wstring_view modulePath) {
    try {
        const std::filesystem::path module{std::wstring(modulePath)};
        if (module.empty())
            return {};
        return installRootForModule(module);
    } catch (...) {
        return {};
    }
}

std::filesystem::path portableDataRootForModule(std::wstring_view modulePath) {
    try {
        const std::filesystem::path module{std::wstring(modulePath)};
        if (module.empty())
            return {};
        const auto root = installRootForModule(module);
        if (!root.empty() && std::filesystem::exists(root / L"portable.flag"))
            return root / L"data";
        const auto directory = module.parent_path();
        if (!directory.empty() && std::filesystem::exists(directory / L"portable.flag"))
            return directory / L"data";
        if (!directory.empty() &&
            std::filesystem::exists(directory.parent_path() / L"portable.flag")) {
            return directory.parent_path() / L"data";
        }
        return {};
    } catch (...) {
        return {};
    }
}

std::wstring makeLocalEndpointName(const RuntimeIdentity& identity, std::wstring_view channel) {
    return makeLocalEndpointName(identity, currentRuntimeGeneration(), channel);
}

std::wstring makeLocalEndpointName(const RuntimeIdentity& identity,
                                   std::wstring_view generation,
                                   std::wstring_view channel) {
    if (identity.userSid.empty() || identity.sessionId == 0 || !validGeneration(generation) ||
        !validChannel(channel))
        return {};
    const std::wstring testNamespace = localTestNamespace();
    const std::wstring namespacePart =
        testNamespace.empty() ? std::wstring{} : L".Test." + testNamespace;
    return L"\\\\.\\pipe\\" + std::wstring(kReleaseIdentity.pipe_prefix) + L"." + identity.userSid +
           L".Session." + std::to_wstring(identity.sessionId) + L".Generation." +
           std::wstring(generation) + namespacePart + L"." + std::wstring(channel);
}

std::wstring makeLocalObjectName(const RuntimeIdentity& identity, std::wstring_view channel) {
    return makeLocalObjectName(identity, currentRuntimeGeneration(), channel);
}

std::wstring makeLocalObjectName(const RuntimeIdentity& identity,
                                 std::wstring_view generation,
                                 std::wstring_view channel) {
    if (identity.userSid.empty() || identity.sessionId == 0 || !validGeneration(generation) ||
        !validChannel(channel))
        return {};
    const std::wstring testNamespace = localTestNamespace();
    const std::wstring namespacePart =
        testNamespace.empty() ? std::wstring{} : L".Test." + testNamespace;
    return L"Local\\" + std::wstring(kReleaseIdentity.local_object_prefix) + L"." +
           identity.userSid + L".Session." + std::to_wstring(identity.sessionId) +
           L".Generation." + std::wstring(generation) + namespacePart + L"." +
           std::wstring(channel);
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
