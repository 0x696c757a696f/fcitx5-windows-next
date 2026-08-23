#include "runtime_identity.h"

#include <cstdint>
#include <filesystem>
#include <string>

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
extern "C" std::size_t fcitx5_windows_common_local_test_namespace_utf16(
    std::uint16_t* output,
    std::size_t capacity);
extern "C" std::size_t fcitx5_windows_common_current_generation_utf16(
    std::uint16_t* output,
    std::size_t capacity);
struct Fcitx5WindowsCommonProcessExecutableIdentity {
    std::uint8_t status;
    std::uint8_t serviceAccount;
    std::uint8_t executableFileStatus;
    std::uint8_t executableFileContainsReparsePoint;
    std::uint32_t sessionId;
    std::uint32_t executableFileVolumeSerialNumber;
    std::uint32_t executableFileIndexHigh;
    std::uint32_t executableFileIndexLow;
    std::uint32_t executableFileNumberOfLinks;
    std::size_t userSidLen;
    std::size_t executablePathLen;
    std::size_t executableFinalPathLen;
};
struct Fcitx5WindowsCommonCurrentExecutableIdentity {
    std::uint8_t status;
    std::uint8_t serviceAccount;
    std::uint8_t secureDesktop;
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
extern "C" Fcitx5WindowsCommonProcessExecutableIdentity
fcitx5_windows_common_process_identity_with_executable_file_utf16(
    std::uint32_t process_id,
    std::uint16_t* user_sid_output,
    std::size_t user_sid_capacity,
    std::uint16_t* executable_path_output,
    std::size_t executable_path_capacity,
    std::uint16_t* executable_final_path_output,
    std::size_t executable_final_path_capacity);
extern "C" Fcitx5WindowsCommonCurrentExecutableIdentity
fcitx5_windows_common_current_identity_with_executable_file_utf16(
    std::uint16_t* user_sid_output,
    std::size_t user_sid_capacity,
    std::uint16_t* executable_path_output,
    std::size_t executable_path_capacity,
    std::uint16_t* executable_final_path_output,
    std::size_t executable_final_path_capacity);
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
extern "C" std::uint8_t fcitx5_windows_common_may_launch_user_engine_utf16(
    std::uint8_t service_account,
    std::uint32_t session_id,
    std::uint8_t secure_desktop,
    const std::uint16_t* user_sid,
    std::size_t user_sid_len);
extern "C" std::uint8_t fcitx5_windows_common_executable_files_match_utf16(
    std::uint32_t left_volume_serial_number,
    std::uint32_t left_file_index_high,
    std::uint32_t left_file_index_low,
    std::uint32_t left_number_of_links,
    std::uint8_t left_contains_reparse_point,
    const std::uint16_t* left_final_path,
    std::size_t left_final_path_len,
    std::uint32_t right_volume_serial_number,
    std::uint32_t right_file_index_high,
    std::uint32_t right_file_index_low,
    std::uint32_t right_number_of_links,
    std::uint8_t right_contains_reparse_point,
    const std::uint16_t* right_final_path,
    std::size_t right_final_path_len);
extern "C" std::uint8_t fcitx5_windows_common_paths_refer_to_same_file_utf16(
    const std::uint16_t* left_path,
    std::size_t left_path_len,
    const std::uint16_t* right_path,
    std::size_t right_path_len);
extern "C" std::uint8_t fcitx5_windows_common_executable_paths_match_utf16(
    const std::uint16_t* left_path,
    std::size_t left_path_len,
    const std::uint16_t* right_path,
    std::size_t right_path_len);
struct Fcitx5WindowsCommonExecutableFileIdentity {
    std::uint8_t status;
    std::uint8_t containsReparsePoint;
    std::uint32_t volumeSerialNumber;
    std::uint32_t fileIndexHigh;
    std::uint32_t fileIndexLow;
    std::uint32_t numberOfLinks;
    std::size_t finalPathLen;
};
extern "C" Fcitx5WindowsCommonExecutableFileIdentity
fcitx5_windows_common_executable_file_identity_utf16(const std::uint16_t* path,
                                                     std::size_t path_len,
                                                     std::uint16_t* final_path_output,
                                                     std::size_t final_path_capacity);

template <typename Producer>
std::wstring rustWide(Producer producer);

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

template <typename Identity>
bool copyExecutableFileIdentity(const Identity& identity,
                                std::wstring& finalPath,
                                ExecutableFileIdentity& output) {
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
    return fcitx5_windows_common_may_launch_user_engine_utf16(
               identity.serviceAccount ? 1 : 0, identity.sessionId,
               identity.secureDesktop ? 1 : 0, wideData(identity.userSid),
               identity.userSid.size()) != 0;
}

bool RuntimeIdentity::mayUseUserEngine() const noexcept { return mayLaunchUserEngine(*this); }

bool queryProcessIdentity(DWORD processId, ProcessIdentity& output) noexcept {
    output = {};
    try {
        const auto query = fcitx5_windows_common_process_identity_with_executable_file_utf16(
            processId, nullptr, 0, nullptr, 0, nullptr, 0);
        if (query.status == 0 || query.userSidLen == 0 || query.executablePathLen == 0) {
            return false;
        }
        ProcessIdentity result;
        result.userSid.assign(query.userSidLen, L'\0');
        result.executablePath.assign(query.executablePathLen, L'\0');
        std::wstring executableFinalPath(query.executableFinalPathLen, L'\0');
        const auto filled = fcitx5_windows_common_process_identity_with_executable_file_utf16(
            processId, reinterpret_cast<std::uint16_t*>(result.userSid.data()),
            result.userSid.size(), reinterpret_cast<std::uint16_t*>(result.executablePath.data()),
            result.executablePath.size(),
            executableFinalPath.empty()
                ? nullptr
                : reinterpret_cast<std::uint16_t*>(executableFinalPath.data()),
            executableFinalPath.size());
        if (filled.status == 0 || filled.userSidLen != result.userSid.size() ||
            filled.executablePathLen != result.executablePath.size()) {
            return false;
        }
        result.processId = processId;
        result.sessionId = filled.sessionId;
        result.serviceAccount = filled.serviceAccount != 0;
        result.executableFileVerified =
            copyExecutableFileIdentity(filled, executableFinalPath, result.executableFile);
        output = std::move(result);
        return true;
    } catch (...) {
        output = {};
        return false;
    }
}

bool queryCurrentIdentity(RuntimeIdentity& output) noexcept {
    output = {};
    try {
        const auto query = fcitx5_windows_common_current_identity_with_executable_file_utf16(
            nullptr, 0, nullptr, 0, nullptr, 0);
        if (query.status == 0 || query.userSidLen == 0 || query.executablePathLen == 0) {
            return false;
        }
        RuntimeIdentity result;
        result.userSid.assign(query.userSidLen, L'\0');
        result.executablePath.assign(query.executablePathLen, L'\0');
        std::wstring executableFinalPath(query.executableFinalPathLen, L'\0');
        const auto filled = fcitx5_windows_common_current_identity_with_executable_file_utf16(
            reinterpret_cast<std::uint16_t*>(result.userSid.data()), result.userSid.size(),
            reinterpret_cast<std::uint16_t*>(result.executablePath.data()),
            result.executablePath.size(),
            executableFinalPath.empty()
                ? nullptr
                : reinterpret_cast<std::uint16_t*>(executableFinalPath.data()),
            executableFinalPath.size());
        if (filled.status == 0 || filled.userSidLen != result.userSid.size() ||
            filled.executablePathLen != result.executablePath.size()) {
            return false;
        }
        result.processId = filled.processId;
        result.sessionId = filled.sessionId;
        result.serviceAccount = filled.serviceAccount != 0;
        result.secureDesktop = filled.secureDesktop != 0;
        result.executableFileVerified =
            copyExecutableFileIdentity(filled, executableFinalPath, result.executableFile);
        output = std::move(result);
        return true;
    } catch (...) {
        output = {};
        return false;
    }
}

std::wstring localTestNamespace() {
    return rustWide([](std::uint16_t* output, std::size_t capacity) {
        return fcitx5_windows_common_local_test_namespace_utf16(output, capacity);
    });
}

std::wstring currentRuntimeGeneration() {
    const std::wstring generation = rustWide([](std::uint16_t* output, std::size_t capacity) {
        return fcitx5_windows_common_current_generation_utf16(output, capacity);
    });
    return generation.empty() ? L"current" : generation;
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
        return fcitx5_windows_common_paths_refer_to_same_file_utf16(
                   wideData(left), left.size(), wideData(right), right.size()) != 0;
    } catch (...) {
        return false;
    }
}

bool queryExecutableFileIdentity(std::wstring_view path,
                                 ExecutableFileIdentity& output) noexcept {
    output = {};
    try {
        std::wstring finalPath(32768, L'\0');
        const auto identity = fcitx5_windows_common_executable_file_identity_utf16(
            wideData(path), path.size(), reinterpret_cast<std::uint16_t*>(finalPath.data()),
            finalPath.size());
        if (identity.status == 0 || identity.finalPathLen == 0 ||
            identity.finalPathLen > finalPath.size()) {
            return false;
        }
        finalPath.resize(identity.finalPathLen);
        ExecutableFileIdentity result;
        result.volumeSerialNumber = identity.volumeSerialNumber;
        result.fileIndexHigh = identity.fileIndexHigh;
        result.fileIndexLow = identity.fileIndexLow;
        result.numberOfLinks = identity.numberOfLinks;
        result.containsReparsePoint = identity.containsReparsePoint != 0;
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
    return fcitx5_windows_common_executable_files_match_utf16(
               left.volumeSerialNumber, left.fileIndexHigh, left.fileIndexLow, left.numberOfLinks,
               left.containsReparsePoint ? 1 : 0, wideData(left.finalPath),
               left.finalPath.size(), right.volumeSerialNumber, right.fileIndexHigh,
               right.fileIndexLow, right.numberOfLinks, right.containsReparsePoint ? 1 : 0,
               wideData(right.finalPath), right.finalPath.size()) != 0;
}

bool executablePathsMatch(std::wstring_view left, std::wstring_view right) noexcept {
    try {
        return fcitx5_windows_common_executable_paths_match_utf16(
                   wideData(left), left.size(), wideData(right), right.size()) != 0;
    } catch (...) {
        return false;
    }
}

} // namespace fcitx::windows::platform
