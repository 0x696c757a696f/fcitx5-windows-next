#include "runtime_identity.h"

#include <sddl.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <cwctype>
#include <filesystem>
#include <string>
#include <vector>

namespace fcitx::windows::platform {
namespace {

extern "C" std::size_t fcitx5_windows_common_local_name_utf16(
    std::uint32_t kind,
    const std::uint16_t* user_sid,
    std::size_t user_sid_len,
    std::uint32_t session_id,
    const std::uint16_t* generation,
    std::size_t generation_len,
    const std::uint16_t* channel,
    std::size_t channel_len,
    const std::uint16_t* test_namespace,
    std::size_t test_namespace_len,
    std::uint16_t* output,
    std::size_t capacity);
extern "C" std::size_t fcitx5_windows_common_current_generation_for_module_utf16(
    const std::uint16_t* module_path,
    std::size_t module_path_len,
    std::uint16_t* output,
    std::size_t capacity);
extern "C" std::size_t fcitx5_windows_common_current_generation_from_install_root_utf16(
    const std::uint16_t* install_root,
    std::size_t install_root_len,
    std::uint16_t* output,
    std::size_t capacity);
extern "C" std::size_t fcitx5_windows_common_installation_root_for_module_utf16(
    const std::uint16_t* module_path,
    std::size_t module_path_len,
    std::uint16_t* output,
    std::size_t capacity);
extern "C" std::size_t fcitx5_windows_common_portable_data_root_for_module_utf16(
    const std::uint16_t* module_path,
    std::size_t module_path_len,
    std::uint16_t* output,
    std::size_t capacity);

class Handle final {
  public:
    explicit Handle(HANDLE value = nullptr) noexcept : value_(value) {}
    ~Handle() {
        if (valid())
            CloseHandle(value_);
    }
    Handle(const Handle&) = delete;
    Handle& operator=(const Handle&) = delete;
    [[nodiscard]] HANDLE get() const noexcept { return value_; }
    [[nodiscard]] bool valid() const noexcept {
        return value_ != nullptr && value_ != INVALID_HANDLE_VALUE;
    }
    [[nodiscard]] explicit operator bool() const noexcept { return valid(); }

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

struct BasicFileIdentity {
    DWORD volumeSerialNumber{};
    DWORD fileIndexHigh{};
    DWORD fileIndexLow{};
};

bool queryFileIdentity(std::wstring_view path, BasicFileIdentity& identity) {
    identity = {};
    if (path.empty() || path.size() >= 32768)
        return false;
    const std::wstring source(path);
    Handle file(CreateFileW(source.c_str(), FILE_READ_ATTRIBUTES,
                            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, nullptr,
                            OPEN_EXISTING,
                            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS, nullptr));
    if (!file)
        return false;
    BY_HANDLE_FILE_INFORMATION information{};
    if (!GetFileInformationByHandle(file.get(), &information))
        return false;
    identity.volumeSerialNumber = information.dwVolumeSerialNumber;
    identity.fileIndexHigh = information.nFileIndexHigh;
    identity.fileIndexLow = information.nFileIndexLow;
    return true;
}

bool pathIsReparsePoint(const std::filesystem::path& source) {
    const DWORD attributes = GetFileAttributesW(source.c_str());
    return attributes == INVALID_FILE_ATTRIBUTES ||
           (attributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0;
}

bool sameFinalPath(std::wstring_view left, std::wstring_view right) noexcept {
    return !left.empty() && !right.empty() &&
           CompareStringOrdinal(left.data(), static_cast<int>(left.size()), right.data(),
                                static_cast<int>(right.size()), TRUE) == CSTR_EQUAL;
}

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

template <typename Producer>
std::wstring rustWide(Producer producer) {
    const std::size_t required = producer(nullptr, 0);
    if (required == 0)
        return {};
    std::wstring result(required, L'\0');
    const std::size_t written =
        producer(reinterpret_cast<std::uint16_t*>(result.data()), result.size());
    if (written == 0 || written > result.size())
        return {};
    result.resize(written);
    return result;
}

std::wstring rustLocalName(std::uint32_t kind, const RuntimeIdentity& identity,
                           std::wstring_view generation, std::wstring_view channel) {
    const std::wstring testNamespace = localTestNamespace();
    return rustWide([&](std::uint16_t* output, std::size_t capacity) {
        return fcitx5_windows_common_local_name_utf16(
            kind, wideData(identity.userSid), identity.userSid.size(), identity.sessionId,
            wideData(generation), generation.size(), wideData(channel), channel.size(),
            wideData(testNamespace), testNamespace.size(), output, capacity);
    });
}

std::wstring rustCurrentGenerationForModule(std::wstring_view modulePath) {
    return rustWide([&](std::uint16_t* output, std::size_t capacity) {
        return fcitx5_windows_common_current_generation_for_module_utf16(
            wideData(modulePath), modulePath.size(), output, capacity);
    });
}

std::wstring rustCurrentGenerationFromInstallRoot(std::wstring_view installRoot) {
    return rustWide([&](std::uint16_t* output, std::size_t capacity) {
        return fcitx5_windows_common_current_generation_from_install_root_utf16(
            wideData(installRoot), installRoot.size(), output, capacity);
    });
}

std::filesystem::path rustInstallationRootForModule(std::wstring_view modulePath) {
    const std::wstring path = rustWide([&](std::uint16_t* output, std::size_t capacity) {
        return fcitx5_windows_common_installation_root_for_module_utf16(
            wideData(modulePath), modulePath.size(), output, capacity);
    });
    return path.empty() ? std::filesystem::path{} : std::filesystem::path(path);
}

std::filesystem::path rustPortableDataRootForModule(std::wstring_view modulePath) {
    const std::wstring path = rustWide([&](std::uint16_t* output, std::size_t capacity) {
        return fcitx5_windows_common_portable_data_root_for_module_utf16(
            wideData(modulePath), modulePath.size(), output, capacity);
    });
    return path.empty() ? std::filesystem::path{} : std::filesystem::path(path);
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
        result.executableFileVerified =
            queryExecutableFileIdentity(result.executablePath, result.executableFile);
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
    std::wstring path(32'768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    if (size > 0 && size < path.size()) {
        path.resize(size);
        if (const std::wstring generation = rustCurrentGenerationForModule(path);
            !generation.empty()) {
            return generation;
        }
    }
    return L"current";
}

std::wstring currentRuntimeGenerationForModule(std::wstring_view modulePath) {
    return rustCurrentGenerationForModule(modulePath);
}

std::wstring currentRuntimeGenerationFromInstallRoot(std::wstring_view installRoot) {
    return rustCurrentGenerationFromInstallRoot(installRoot);
}

std::filesystem::path installationRootForModule(std::wstring_view modulePath) {
    try {
        return rustInstallationRootForModule(modulePath);
    } catch (...) {
        return {};
    }
}

std::filesystem::path portableDataRootForModule(std::wstring_view modulePath) {
    try {
        return rustPortableDataRootForModule(modulePath);
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
    return rustLocalName(0, identity, generation, channel);
}

std::wstring makeLocalObjectName(const RuntimeIdentity& identity, std::wstring_view channel) {
    return makeLocalObjectName(identity, currentRuntimeGeneration(), channel);
}

std::wstring makeLocalObjectName(const RuntimeIdentity& identity,
                                 std::wstring_view generation,
                                 std::wstring_view channel) {
    return rustLocalName(1, identity, generation, channel);
}

bool pathsReferToSameFile(std::wstring_view left, std::wstring_view right) noexcept {
    try {
        BasicFileIdentity leftIdentity;
        BasicFileIdentity rightIdentity;
        return queryFileIdentity(left, leftIdentity) && queryFileIdentity(right, rightIdentity) &&
               leftIdentity.volumeSerialNumber == rightIdentity.volumeSerialNumber &&
               leftIdentity.fileIndexHigh == rightIdentity.fileIndexHigh &&
               leftIdentity.fileIndexLow == rightIdentity.fileIndexLow;
    } catch (...) {
        return false;
    }
}

bool queryExecutableFileIdentity(std::wstring_view path,
                                 ExecutableFileIdentity& output) noexcept {
    output = {};
    try {
        if (path.empty() || path.size() >= 32768)
            return false;
        const std::wstring source(path);
        Handle file(CreateFileW(source.c_str(), FILE_READ_ATTRIBUTES,
                                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, nullptr,
                                OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr));
        if (!file)
            return false;
        BY_HANDLE_FILE_INFORMATION information{};
        if (!GetFileInformationByHandle(file.get(), &information))
            return false;
        std::wstring finalPath(32768, L'\0');
        DWORD finalPathLength = GetFinalPathNameByHandleW(
            file.get(), finalPath.data(), static_cast<DWORD>(finalPath.size()),
            FILE_NAME_NORMALIZED | VOLUME_NAME_DOS);
        if (finalPathLength == 0 || finalPathLength >= finalPath.size())
            return false;
        finalPath.resize(finalPathLength);
        ExecutableFileIdentity result;
        result.volumeSerialNumber = information.dwVolumeSerialNumber;
        result.fileIndexHigh = information.nFileIndexHigh;
        result.fileIndexLow = information.nFileIndexLow;
        result.numberOfLinks = information.nNumberOfLinks;
        result.containsReparsePoint = pathIsReparsePoint(std::filesystem::path(source));
        result.finalPath = std::move(finalPath);
        output = std::move(result);
        return true;
    } catch (...) {
        output = {};
        return false;
    }
}

bool executableFilesMatch(const ExecutableFileIdentity& left,
                          const ExecutableFileIdentity& right) noexcept {
    return !left.containsReparsePoint && !right.containsReparsePoint &&
           left.numberOfLinks == 1 && right.numberOfLinks == 1 &&
           left.volumeSerialNumber == right.volumeSerialNumber &&
           left.fileIndexHigh == right.fileIndexHigh &&
           left.fileIndexLow == right.fileIndexLow &&
           sameFinalPath(left.finalPath, right.finalPath);
}

bool executablePathsMatch(std::wstring_view left, std::wstring_view right) noexcept {
    try {
        ExecutableFileIdentity leftIdentity;
        ExecutableFileIdentity rightIdentity;
        return queryExecutableFileIdentity(left, leftIdentity) &&
               queryExecutableFileIdentity(right, rightIdentity) &&
               executableFilesMatch(leftIdentity, rightIdentity);
    } catch (...) {
        return false;
    }
}

} // namespace fcitx::windows::platform
