#include "config_model.h"
#include "fcitx5_windows/release_identity.h"
#include "fcitx5_windows/version.h"
#include "launcher_client.h"
#include "package_core.h"
#include "peer_verification.h"
#include "protocol.h"
#include "runtime_identity.h"

#include <Windows.h>

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <map>
#include <optional>
#include <set>
#include <span>
#include <string>
#include <string_view>
#include <utility>
#include <vector>
#include <filesystem>
#include <iostream>
#include <map>
#include <set>
#include <string>
#include <string_view>
#include <vector>

struct RepositorySequenceStateNative {
    std::uint8_t present{};
    std::uint8_t valid{};
    std::uint8_t reserved[6]{};
    std::uint64_t maximum{};
};
static_assert(sizeof(RepositorySequenceStateNative) == 16U);
static_assert(alignof(RepositorySequenceStateNative) == alignof(std::uint64_t));

extern "C" {
int fcitx5_repository_sequence_state_read_utf16(const wchar_t* data_root, std::size_t data_root_len,
                                                const wchar_t* channel, std::size_t channel_len,
                                                RepositorySequenceStateNative* out_state);
int fcitx5_repository_sequence_state_write_utf16(const wchar_t* data_root,
                                                 std::size_t data_root_len,
                                                 const wchar_t* channel,
                                                 std::size_t channel_len,
                                                 std::uint64_t maximum);
int fcitx5_update_read_update_owner_utf16(const std::uint16_t* root, std::size_t root_len,
                                          std::uint32_t* out_owner);

struct Fcitx5ControlUtf16 {
    const wchar_t* ptr;
    std::size_t len;
};
struct Fcitx5ControlUtf8 {
    const char* ptr;
    std::size_t len;
};
struct Fcitx5WindowsCommonUtf8ToWide {
    std::uint8_t status;
    std::size_t utf16Len;
};
struct Fcitx5WindowsCommonWideToUtf8 {
    std::uint8_t status;
    std::size_t utf8Len;
};
Fcitx5WindowsCommonUtf8ToWide fcitx5_windows_common_utf8_to_wide_utf16(
    const std::uint8_t* input, std::size_t inputLen, std::uint16_t* output,
    std::size_t capacity);
Fcitx5WindowsCommonWideToUtf8 fcitx5_windows_common_wide_utf16_to_utf8(
    const std::uint16_t* input, std::size_t inputLen, std::uint8_t* output,
    std::size_t capacity);
std::uint64_t fcitx5_windows_common_deadline_after_milliseconds(std::uint32_t milliseconds);
struct Fcitx5ControlPresentation {
    Fcitx5ControlUtf8 appearanceMode;
    Fcitx5ControlUtf8 theme;
    Fcitx5ControlUtf8 orientation;
    Fcitx5ControlUtf8 candidateFont;
    Fcitx5ControlUtf8 candidatePageSize;
    Fcitx5ControlUtf8 candidateMaxWidthDip;
    Fcitx5ControlUtf8 candidateScrollCellWidthDip;
    Fcitx5ControlUtf8 candidateFontSizeDip;
    Fcitx5ControlUtf8 candidateCornerRadiusDip;
    Fcitx5ControlUtf8 candidateOpacity;
    Fcitx5ControlUtf8 candidatePreeditMode;
    std::uint8_t candidateShadow;
    std::uint8_t scrollMode;
};
struct Fcitx5ControlStatus {
    std::uint8_t launcherReachable;
    std::int32_t launcherState;
    std::int32_t engineState;
    Fcitx5ControlUtf8 currentInputMethodId;
    Fcitx5ControlUtf8 currentInputMethodName;
    Fcitx5ControlUtf8 currentInputMethodNativeName;
    Fcitx5ControlUtf8 currentInputMethodShortLabel;
    std::uint8_t configValid;
    std::uint8_t tsfGuardDisabled;
    Fcitx5ControlUtf8 tsfGuardReason;
    Fcitx5ControlUtf8 dataRoot;
    Fcitx5ControlUtf8 updateOwner;
};
struct Fcitx5ControlTsfGuard {
    std::uint8_t disabled;
    Fcitx5ControlUtf8 reason;
    Fcitx5ControlUtf8 markerPath;
};
struct Fcitx5ControlPackageRepair {
    Fcitx5ControlUtf8 repositorySequenceState;
};
struct Fcitx5ControlAddonDescriptor {
    Fcitx5ControlUtf8 id;
    Fcitx5ControlUtf8 name;
    Fcitx5ControlUtf8 category;
    Fcitx5ControlUtf8 library;
    Fcitx5ControlUtf8 type;
    Fcitx5ControlUtf8 version;
    std::uint8_t configurable;
    std::uint8_t onDemand;
    std::uint8_t libraryPresent;
};
struct Fcitx5ControlThemeRecord {
    Fcitx5ControlUtf8 id;
    Fcitx5ControlUtf8 source;
    Fcitx5ControlUtf8 name;
    Fcitx5ControlUtf8 version;
    Fcitx5ControlUtf8 license;
    Fcitx5ControlUtf8 description;
};
struct Fcitx5ControlThemeDetail {
    Fcitx5ControlThemeRecord theme;
    std::uint8_t hasLightBranch;
    std::uint8_t hasDarkBranch;
};
struct Fcitx5ControlPackageSummary {
    Fcitx5ControlUtf8 id;
    Fcitx5ControlUtf8 title;
    Fcitx5ControlUtf8 summary;
    Fcitx5ControlUtf8 type;
    Fcitx5ControlUtf8 availableVersion;
    Fcitx5ControlUtf8 installedVersion;
    Fcitx5ControlUtf8 state;
    std::uint8_t updateAvailable;
};
struct Fcitx5ControlPackagesList {
    std::uint8_t repositoryAvailable;
    Fcitx5ControlUtf8 repositoryError;
    const Fcitx5ControlPackageSummary* packages;
    std::size_t packageCount;
};
struct Fcitx5ControlPackageDependency {
    Fcitx5ControlUtf8 id;
    Fcitx5ControlUtf8 version;
};
struct Fcitx5ControlPackageDetail {
    std::uint8_t repositoryAvailable;
    Fcitx5ControlUtf8 repositoryError;
    Fcitx5ControlUtf8 id;
    Fcitx5ControlUtf8 title;
    Fcitx5ControlUtf8 summary;
    Fcitx5ControlUtf8 type;
    Fcitx5ControlUtf8 availableVersion;
    Fcitx5ControlUtf8 installedVersion;
    Fcitx5ControlUtf8 state;
    std::uint8_t bundled;
    std::uint8_t updateAvailable;
    Fcitx5ControlUtf8 manifestSha256;
    Fcitx5ControlUtf8 sourceCommit;
    Fcitx5ControlUtf8 dependenciesJson;
    Fcitx5ControlUtf8 permissionsJson;
    Fcitx5ControlUtf8 configSurfaceJson;
};
struct Fcitx5ControlBundledPackageDescriptor {
    Fcitx5ControlUtf8 id;
    Fcitx5ControlUtf8 title;
};
struct Fcitx5ControlPathResult {
    int status;
    std::size_t pathLen;
};
int fcitx5_control_startup_query_utf16(Fcitx5ControlUtf16 executable_directory,
                                       Fcitx5ControlUtf16 registry_value,
                                       std::uint8_t* out_enabled);
int fcitx5_control_startup_set_utf16(Fcitx5ControlUtf16 executable_directory,
                                     Fcitx5ControlUtf16 registry_value,
                                     std::uint8_t enabled);
int fcitx5_control_atomic_write_utf8_file_utf16(Fcitx5ControlUtf16 destination,
                                                Fcitx5ControlUtf8 content);
int fcitx5_control_read_file_utf16(Fcitx5ControlUtf16 path, std::uint64_t maximum,
                                   char** out_ptr, std::size_t* out_len);
int fcitx5_control_read_optional_config_utf16(Fcitx5ControlUtf16 path, char** out_ptr,
                                              std::size_t* out_len);
int fcitx5_control_installed_manifest_bytes_utf16(Fcitx5ControlUtf16 package_root,
                                                  Fcitx5ControlUtf8 id,
                                                  Fcitx5ControlUtf8 version, char** out_ptr,
                                                  std::size_t* out_len);
std::size_t fcitx5_control_repository_cache_incoming_path_utf16(Fcitx5ControlUtf16 path,
                                                                wchar_t* output,
                                                                std::size_t capacity);
int fcitx5_control_repository_cache_prepare_utf16(Fcitx5ControlUtf16 index,
                                                  Fcitx5ControlUtf16 signature);
int fcitx5_control_repository_cache_cleanup_utf16(Fcitx5ControlUtf16 index,
                                                  Fcitx5ControlUtf16 signature);
int fcitx5_control_repository_cache_publish_utf16(Fcitx5ControlUtf16 index,
                                                  Fcitx5ControlUtf16 signature);
Fcitx5ControlPathResult fcitx5_control_package_archive_cache_prepare_utf16(
    Fcitx5ControlUtf16 data_root, Fcitx5ControlUtf8 id, Fcitx5ControlUtf8 version,
    std::uint8_t existing_hash_matches, wchar_t* output, std::size_t capacity);
std::size_t fcitx5_control_package_archive_cache_path_utf16(Fcitx5ControlUtf16 data_root,
                                                            Fcitx5ControlUtf8 id,
                                                            Fcitx5ControlUtf8 version,
                                                            wchar_t* output,
                                                            std::size_t capacity);
int fcitx5_control_schema_json_utf8(const char** out_ptr, std::size_t* out_len);
int fcitx5_control_usage_text_utf8(const char** out_ptr, std::size_t* out_len);
std::uint8_t fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 id);
int fcitx5_control_presentation_json_utf8(const Fcitx5ControlPresentation* presentation,
                                          char** out_ptr, std::size_t* out_len);
int fcitx5_control_status_json_utf8(const Fcitx5ControlStatus* status, char** out_ptr,
                                    std::size_t* out_len);
int fcitx5_control_diagnostics_plan_json_utf8(const Fcitx5ControlStatus* status, char** out_ptr,
                                              std::size_t* out_len);
int fcitx5_control_tsf_guard_json_utf8(const Fcitx5ControlTsfGuard* status, char** out_ptr,
                                       std::size_t* out_len);
int fcitx5_control_tsf_guard_reset_json_utf8(const char** out_ptr, std::size_t* out_len);
int fcitx5_control_startup_json_utf8(std::uint8_t enabled, char** out_ptr, std::size_t* out_len);
int fcitx5_control_launcher_action_sequence(std::uint32_t action,
                                            const std::uint32_t** out_ptr,
                                            std::size_t* out_len);
int fcitx5_control_package_repair_json_utf8(const Fcitx5ControlPackageRepair* repair,
                                            char** out_ptr, std::size_t* out_len);
std::uint32_t fcitx5_control_root_action_utf16(Fcitx5ControlUtf16 command, std::size_t argc,
                                               Fcitx5ControlUtf16 value);
std::uint32_t fcitx5_control_config_action_utf16(Fcitx5ControlUtf16 command, std::size_t argc);
std::uint32_t fcitx5_control_engine_management_action_utf16(Fcitx5ControlUtf16 command,
                                                            std::size_t argc);
std::uint32_t fcitx5_control_package_action_utf16(Fcitx5ControlUtf16 command, std::size_t argc,
                                                  Fcitx5ControlUtf16 state);
int fcitx5_control_addons_json_utf8(const Fcitx5ControlAddonDescriptor* addons,
                                    std::size_t addon_count, char** out_ptr,
                                    std::size_t* out_len);
int fcitx5_control_themes_json_utf8(const Fcitx5ControlThemeRecord* themes,
                                    std::size_t theme_count, char** out_ptr,
                                    std::size_t* out_len);
int fcitx5_control_theme_detail_json_utf8(const Fcitx5ControlThemeDetail* detail,
                                          char** out_ptr, std::size_t* out_len);
int fcitx5_control_packages_list_json_utf8(const Fcitx5ControlPackagesList* list,
                                           char** out_ptr, std::size_t* out_len);
int fcitx5_control_package_dependencies_json_utf8(
    const Fcitx5ControlPackageDependency* dependencies, std::size_t dependency_count,
    char** out_ptr, std::size_t* out_len);
int fcitx5_control_string_array_json_utf8(const Fcitx5ControlUtf8* values,
                                          std::size_t value_count, char** out_ptr,
                                          std::size_t* out_len);
int fcitx5_control_config_surfaces_json_utf8(Fcitx5ControlUtf8 owner,
                                             const Fcitx5ControlUtf8* kinds,
                                             std::size_t kind_count, char** out_ptr,
                                             std::size_t* out_len);
int fcitx5_control_package_config_surface_json_utf8(
    Fcitx5ControlUtf8 owner, std::uint32_t package_type,
    const Fcitx5ControlUtf8* permissions, std::size_t permission_count,
    const Fcitx5ControlUtf8* file_paths, std::size_t file_path_count, char** out_ptr,
    std::size_t* out_len);
int fcitx5_control_repository_error_utf8(Fcitx5ControlUtf8 error_code,
                                         Fcitx5ControlUtf16 keyring, char** out_ptr,
                                         std::size_t* out_len);
std::size_t fcitx5_control_bundled_package_count();
std::uint8_t fcitx5_control_bundled_package_descriptor(
    std::size_t index, Fcitx5ControlBundledPackageDescriptor* descriptor);
std::uint8_t fcitx5_control_bundled_package_present_utf16(Fcitx5ControlUtf16 install_root,
                                                          Fcitx5ControlUtf8 id);
Fcitx5ControlUtf8 fcitx5_control_package_type_name_utf8(std::uint32_t package_type);
Fcitx5ControlUtf8 fcitx5_control_native_package_architecture_utf8();
std::uint8_t fcitx5_control_package_architecture_matches_native_utf8(
    Fcitx5ControlUtf8 architecture);
std::uint8_t fcitx5_control_addon_metadata_section_is_addon_utf8(Fcitx5ControlUtf8 section);
std::uint32_t fcitx5_control_addon_metadata_key_utf8(Fcitx5ControlUtf8 key);
std::uint8_t fcitx5_control_addon_metadata_bool_utf8(Fcitx5ControlUtf8 value);
std::uint8_t fcitx5_control_package_update_available_utf8(
    std::uint8_t installed_present, Fcitx5ControlUtf8 installed_version,
    Fcitx5ControlUtf8 available_version);
std::uint8_t fcitx5_control_package_state_satisfies_dependency_utf8(Fcitx5ControlUtf8 state);
std::uint8_t fcitx5_control_package_state_keeps_installed_version_utf8(Fcitx5ControlUtf8 state);
std::uint64_t fcitx5_control_repository_max_release_sequence(const std::uint64_t* sequences,
                                                             std::size_t sequence_count);
std::size_t fcitx5_control_repository_metadata_url_utf16(Fcitx5ControlUtf16 base_url,
                                                         Fcitx5ControlUtf8 metadata_name,
                                                         wchar_t* output, std::size_t capacity);
std::size_t fcitx5_control_repository_default_base_url_utf16(Fcitx5ControlUtf8 channel,
                                                             wchar_t* output,
                                                             std::size_t capacity);
int fcitx5_control_package_transaction_id_utf8(Fcitx5ControlUtf8 sha256, char** out_ptr,
                                               std::size_t* out_len);
int fcitx5_control_package_detail_json_utf8(const Fcitx5ControlPackageDetail* detail,
                                            char** out_ptr, std::size_t* out_len);
void fcitx5_control_utf8_free(char* ptr, std::size_t len);
}

namespace fcitx::windows::tsf {

extern "C" {
std::size_t fcitx5_tsf_activation_guard_marker_path(const wchar_t* data_root,
                                                    std::size_t data_root_len,
                                                    wchar_t* out,
                                                    std::size_t capacity);
std::uint8_t fcitx5_tsf_activation_guard_status(const wchar_t* data_root,
                                                std::size_t data_root_len,
                                                std::uint8_t* reason_out,
                                                std::size_t reason_capacity,
                                                std::size_t* reason_len);
std::uint8_t fcitx5_tsf_activation_guard_clear(const wchar_t* data_root,
                                               std::size_t data_root_len);
}

struct ActivationGuardStatus {
    bool disabled{};
    std::filesystem::path markerPath;
    std::string reason;
};

[[nodiscard]] std::filesystem::path activationGuardMarkerPath(
    const std::filesystem::path& dataRoot) {
    const std::wstring root = dataRoot.wstring();
    const std::size_t required =
        fcitx5_tsf_activation_guard_marker_path(root.data(), root.size(), nullptr, 0);
    if (required == 0) return {};
    std::wstring buffer(required, L'\0');
    const std::size_t written = fcitx5_tsf_activation_guard_marker_path(
        root.data(), root.size(), buffer.data(), buffer.size());
    buffer.resize((std::min)(written, buffer.size()));
    return std::filesystem::path(buffer);
}

[[nodiscard]] ActivationGuardStatus activationGuardStatus(
    const std::filesystem::path& dataRoot) noexcept {
    ActivationGuardStatus status;
    try {
        status.markerPath = activationGuardMarkerPath(dataRoot);
        const std::wstring root = dataRoot.wstring();
        std::array<std::uint8_t, 512> reason{};
        std::size_t reasonLength = 0;
        status.disabled = fcitx5_tsf_activation_guard_status(
                              root.data(), root.size(), reason.data(), reason.size(),
                              &reasonLength) != 0;
        status.reason.assign(reinterpret_cast<const char*>(reason.data()),
                             (std::min)(reasonLength, reason.size()));
    } catch (...) {
        status = {};
    }
    return status;
}

[[nodiscard]] bool clearActivationGuard(const std::filesystem::path& dataRoot) noexcept {
    try {
        const std::wstring root = dataRoot.wstring();
        return fcitx5_tsf_activation_guard_clear(root.data(), root.size()) != 0;
    } catch (...) {
        return false;
    }
}

} // namespace fcitx::windows::tsf

namespace {

struct Fcitx5ProcessUtf16 {
    const char16_t* ptr;
    std::size_t len;
};

struct Fcitx5ProcessRunResult {
    std::uint8_t success;
    std::uint8_t reserved[7];
    char16_t* outputPtr;
    std::size_t outputLen;
};

extern "C" {
int fcitx5_process_run_utf16(Fcitx5ProcessUtf16 executable,
                             const Fcitx5ProcessUtf16* arguments,
                             std::size_t argumentCount, std::uint32_t timeoutMilliseconds,
                             std::size_t maxOutputBytes, Fcitx5ProcessRunResult* result);
void fcitx5_process_output_free(char16_t* ptr, std::size_t len);
}

Fcitx5ProcessUtf16 view(const std::wstring& value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(char16_t));
    return {reinterpret_cast<const char16_t*>(value.data()), value.size()};
}

} // namespace

namespace fcitx::windows::config {

bool runExecutable(const std::filesystem::path& executable,
                   const std::vector<std::wstring>& arguments,
                   std::wstring& output, unsigned timeoutMilliseconds = 120'000,
                   std::size_t maxOutputBytes = 2U * 1024U * 1024U) {
    output.clear();
    const std::wstring executableText = executable.wstring();
    std::vector<Fcitx5ProcessUtf16> argumentViews;
    argumentViews.reserve(arguments.size());
    for (const auto& argument : arguments) {
        argumentViews.push_back(view(argument));
    }
    Fcitx5ProcessRunResult result{};
    const int status =
        fcitx5_process_run_utf16(view(executableText), argumentViews.data(),
                                 argumentViews.size(), timeoutMilliseconds, maxOutputBytes,
                                 &result);
    if (status != 0) {
        return false;
    }
    if (result.outputPtr && result.outputLen > 0) {
        output.assign(reinterpret_cast<const wchar_t*>(result.outputPtr), result.outputLen);
    }
    fcitx5_process_output_free(result.outputPtr, result.outputLen);
    return result.success != 0;
}

} // namespace fcitx::windows::config

namespace {

namespace fs = std::filesystem;
using fcitx::windows::config::Config;
using fcitx::windows::config::ParseError;

enum class UpdateOwner : std::uint32_t {
    builtin = 0,
    chocolatey = 1,
    winget = 2,
    enterprise = 3,
    manual = 4,
};

[[nodiscard]] const std::uint16_t* utf16Data(const std::wstring& value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

[[nodiscard]] std::string_view updateOwnerName(UpdateOwner owner) noexcept {
    switch (owner) {
    case UpdateOwner::builtin:
        return "builtin";
    case UpdateOwner::chocolatey:
        return "chocolatey";
    case UpdateOwner::winget:
        return "winget";
    case UpdateOwner::enterprise:
        return "enterprise";
    case UpdateOwner::manual:
        return "manual";
    }
    return "manual";
}

[[nodiscard]] UpdateOwner readUpdateOwner(const fs::path& root) {
    std::uint32_t owner = static_cast<std::uint32_t>(UpdateOwner::manual);
    const std::wstring native = root.native();
    if (fcitx5_update_read_update_owner_utf16(utf16Data(native), native.size(), &owner) != 0 ||
        owner > static_cast<std::uint32_t>(UpdateOwner::manual)) {
        throw std::runtime_error("update owner schema is invalid");
    }
    return static_cast<UpdateOwner>(owner);
}

constexpr wchar_t kVisualConfigChangedMessage[] =
    L"Fcitx5WindowsNext.VisualConfigChanged.v1";
constexpr std::uint32_t kLauncherActionRestartEngine = 1;
constexpr std::uint32_t kLauncherActionShutdown = 2;
constexpr std::uint32_t kRootActionVersion = 1;
constexpr std::uint32_t kRootActionSchema = 2;
constexpr std::uint32_t kRootActionGetStartup = 3;
constexpr std::uint32_t kRootActionSetStartupEnabled = 4;
constexpr std::uint32_t kRootActionSetStartupDisabled = 5;
constexpr std::uint32_t kRootActionGetTsfGuard = 6;
constexpr std::uint32_t kRootActionResetTsfGuard = 7;
constexpr std::uint32_t kRootActionStatus = 8;
constexpr std::uint32_t kRootActionRestartEngine = 9;
constexpr std::uint32_t kRootActionShutdown = 10;
constexpr std::uint32_t kRootActionDiagnosticsPlan = 11;
constexpr int kControlFileReadInvalidFile = 1;
constexpr int kControlFileReadMissing = 3;
constexpr int kControlArchiveCacheInvalid = 1;
constexpr std::uint32_t kConfigActionValidate = 1;
constexpr std::uint32_t kConfigActionApply = 2;
constexpr std::uint32_t kConfigActionResetConfig = 3;
constexpr std::uint32_t kConfigActionResetPresentation = 4;
constexpr std::uint32_t kConfigActionGetPresentation = 5;
constexpr std::uint32_t kConfigActionSetPresentation = 6;
constexpr std::uint32_t kEngineActionGetInputMethods = 1;
constexpr std::uint32_t kEngineActionSetInputMethod = 2;
constexpr std::uint32_t kPackageActionPackagesList = 1;
constexpr std::uint32_t kPackageActionThemesList = 2;
constexpr std::uint32_t kPackageActionThemesDetail = 3;
constexpr std::uint32_t kPackageActionAddonsList = 4;
constexpr std::uint32_t kPackageActionPackagesDetail = 5;
constexpr std::uint32_t kPackageActionPackagesRefresh = 6;
constexpr std::uint32_t kPackageActionPackagesInstall = 7;
constexpr std::uint32_t kPackageActionPackagesUpdate = 8;
constexpr std::uint32_t kPackageActionPackagesState = 9;
constexpr std::uint32_t kPackageActionPackagesRemove = 10;
constexpr std::uint32_t kPackageActionPackagesRepair = 11;

std::string narrow(std::wstring_view value) {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    if (value.empty())
        return {};
    const auto* wide = reinterpret_cast<const std::uint16_t*>(value.data());
    const auto query =
        fcitx5_windows_common_wide_utf16_to_utf8(wide, value.size(), nullptr, 0);
    if (query.status == 0 || query.utf8Len == 0)
        return {};
    std::string result(query.utf8Len, '\0');
    const auto filled = fcitx5_windows_common_wide_utf16_to_utf8(
        wide, value.size(), reinterpret_cast<std::uint8_t*>(result.data()), result.size());
    return filled.status != 0 && filled.utf8Len == result.size() ? result : std::string{};
}

std::wstring widen(std::string_view value) {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    if (value.empty())
        return {};
    const auto* bytes = reinterpret_cast<const std::uint8_t*>(value.data());
    const auto query =
        fcitx5_windows_common_utf8_to_wide_utf16(bytes, value.size(), nullptr, 0);
    if (query.status == 0 || query.utf16Len == 0)
        return {};
    std::wstring result(query.utf16Len, L'\0');
    const auto filled = fcitx5_windows_common_utf8_to_wide_utf16(
        bytes, value.size(), reinterpret_cast<std::uint16_t*>(result.data()), result.size());
    return filled.status != 0 && filled.utf16Len == result.size() ? result : std::wstring{};
}

Fcitx5ControlUtf8 utf8View(std::string_view value) noexcept {
    return {value.data(), value.size()};
}

Fcitx5ControlUtf16 nativeView(std::wstring_view value) noexcept {
    return {value.data(), value.size()};
}

std::string takeRustUtf8(char* bytes, std::size_t length) {
    std::string result;
    if (bytes && length > 0) {
        result.assign(bytes, length);
    }
    fcitx5_control_utf8_free(bytes, length);
    return result;
}

std::string copyUtf8(Fcitx5ControlUtf8 value) {
    return value.ptr && value.len > 0 ? std::string(value.ptr, value.len) : std::string{};
}

template <typename Producer>
std::wstring takeRustWide(Producer producer) {
    const std::size_t required = producer(nullptr, 0);
    if (required == 0)
        return {};
    std::wstring result(required, L'\0');
    const std::size_t written = producer(result.data(), result.size());
    if (written == 0 || written > result.size())
        return {};
    result.resize(written);
    return result;
}

std::string presentationJson(const Fcitx5ControlPresentation& presentation) {
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_presentation_json_utf8(&presentation, &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::string statusJson(const Fcitx5ControlStatus& status) {
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_status_json_utf8(&status, &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::string diagnosticsPlanJson(const Fcitx5ControlStatus& status) {
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_diagnostics_plan_json_utf8(&status, &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::string tsfGuardJson(const Fcitx5ControlTsfGuard& status) {
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_tsf_guard_json_utf8(&status, &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::string tsfGuardResetJson() {
    const char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_tsf_guard_reset_json_utf8(&bytes, &length) != 0 || !bytes)
        return {};
    return {bytes, length};
}

std::string startupJson(bool enabled) {
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_startup_json_utf8(enabled ? 1 : 0, &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::string packageRepairJson(const Fcitx5ControlPackageRepair& repair) {
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_package_repair_json_utf8(&repair, &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::uint32_t rootAction(const std::vector<std::wstring_view>& arguments) {
    if (arguments.empty())
        return 0;
    const std::wstring_view command(arguments[0]);
    const std::wstring_view value = arguments.size() >= 2 ? std::wstring_view(arguments[1])
                                                          : std::wstring_view{};
    return fcitx5_control_root_action_utf16(
        {command.data(), command.size()}, arguments.size(),
        value.empty() ? Fcitx5ControlUtf16{} : Fcitx5ControlUtf16{value.data(), value.size()});
}

std::uint32_t configAction(std::wstring_view command, std::size_t argc) {
    return fcitx5_control_config_action_utf16({command.data(), command.size()}, argc);
}

std::uint32_t engineManagementAction(std::wstring_view command, std::size_t argc) {
    return fcitx5_control_engine_management_action_utf16({command.data(), command.size()}, argc);
}

std::uint32_t packageAction(const std::vector<std::wstring_view>& arguments) {
    if (arguments.empty())
        return 0;
    const std::wstring_view command(arguments[0]);
    const std::wstring_view state = arguments.size() >= 3 ? std::wstring_view(arguments[2])
                                                          : std::wstring_view{};
    return fcitx5_control_package_action_utf16(
        {command.data(), command.size()}, arguments.size(),
        state.empty() ? Fcitx5ControlUtf16{nullptr, 0} : Fcitx5ControlUtf16{state.data(), state.size()});
}

bool readUtf8Bounded(const fs::path& path, std::size_t maximum, std::string& text) {
    const std::wstring pathText = path.wstring();
    char* bytes = nullptr;
    std::size_t length = 0;
    const int status = fcitx5_control_read_file_utf16(
        {pathText.data(), pathText.size()}, maximum, &bytes, &length);
    if (status != 0)
        return false;
    text = takeRustUtf8(bytes, length);
    return true;
}

bool readUtf8(const fs::path& path, std::string& text) {
    return readUtf8Bounded(path, 256U * 1024U, text);
}

enum class OptionalConfigRead {
    present,
    missing,
    error,
};

OptionalConfigRead readOptionalConfig(const fs::path& path, std::string& text) {
    const std::wstring pathText = path.wstring();
    char* bytes = nullptr;
    std::size_t length = 0;
    const int status = fcitx5_control_read_optional_config_utf16(
        {pathText.data(), pathText.size()}, &bytes, &length);
    if (status == kControlFileReadMissing)
        return OptionalConfigRead::missing;
    if (status != 0)
        return OptionalConfigRead::error;
    text = takeRustUtf8(bytes, length);
    return OptionalConfigRead::present;
}

std::vector<std::byte> readBinary(const fs::path& path, std::size_t maximum) {
    const std::wstring pathText = path.wstring();
    char* bytes = nullptr;
    std::size_t length = 0;
    const int status = fcitx5_control_read_file_utf16(
        {pathText.data(), pathText.size()}, maximum, &bytes, &length);
    if (status == kControlFileReadInvalidFile)
        throw fcitx::package::PackageError("invalid_file", "file is missing or too large");
    if (status != 0)
        throw fcitx::package::PackageError("io_error", "file read failed");
    std::vector<std::byte> result(length);
    if (bytes && length > 0)
        std::memcpy(result.data(), bytes, length);
    fcitx5_control_utf8_free(bytes, length);
    return result;
}

fs::path executableDirectory() {
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity) ||
        identity.executablePath.empty()) {
        return {};
    }
    return fs::path(identity.executablePath).parent_path();
}

fs::path installationRoot() {
    const auto directory = executableDirectory();
    return directory.filename() == L"bin" ? directory.parent_path() : directory;
}

bool runProcess(const fs::path& executable, const std::vector<std::wstring>& arguments,
                 DWORD timeout = 120000U) {
    std::wstring ignoredOutput;
    return fcitx::windows::config::runExecutable(executable, arguments, ignoredOutput, timeout);
}

struct RepositoryFiles {
    fs::path index;
    fs::path signature;
    fs::path keyring;
};

RepositoryFiles repositoryFiles(const fs::path& dataRoot) {
    return {dataRoot / L"repository/index.json", dataRoot / L"repository/index.sig",
            installationRoot() / L"security/trusted-keys.json"};
}

Fcitx5ControlUtf16 pathView(const fs::path& path, std::wstring& storage) {
    storage = path.wstring();
    return nativeView(storage);
}

fs::path repositoryIncomingPath(const fs::path& path) {
    const std::wstring pathText = path.wstring();
    const std::wstring incoming = takeRustWide([&](wchar_t* output, std::size_t capacity) {
        return fcitx5_control_repository_cache_incoming_path_utf16(
            nativeView(pathText), output, capacity);
    });
    return incoming.empty() ? fs::path{} : fs::path(incoming);
}

bool prepareRepositoryCache(const RepositoryFiles& files) {
    std::wstring indexText;
    std::wstring signatureText;
    return fcitx5_control_repository_cache_prepare_utf16(
               pathView(files.index, indexText), pathView(files.signature, signatureText)) == 0;
}

void cleanupRepositoryCache(const RepositoryFiles& files) {
    std::wstring indexText;
    std::wstring signatureText;
    (void)fcitx5_control_repository_cache_cleanup_utf16(
        pathView(files.index, indexText), pathView(files.signature, signatureText));
}

bool publishRepositoryCache(const RepositoryFiles& files) {
    std::wstring indexText;
    std::wstring signatureText;
    return fcitx5_control_repository_cache_publish_utf16(
               pathView(files.index, indexText), pathView(files.signature, signatureText)) == 0;
}

std::string repositoryErrorCode(const fs::path& dataRoot, std::string_view errorCode) {
    const auto files = repositoryFiles(dataRoot);
    std::wstring keyringText;
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_repository_error_utf8(
            utf8View(errorCode), pathView(files.keyring, keyringText), &bytes, &length) != 0)
        return std::string(errorCode);
    return takeRustUtf8(bytes, length);
}

fs::path packageArchiveCachePath(const fs::path& dataRoot,
                                 const fcitx::package::RepositoryEntry& entry) {
    const std::wstring dataRootText = dataRoot.wstring();
    const std::wstring archive = takeRustWide([&](wchar_t* output, std::size_t capacity) {
        return fcitx5_control_package_archive_cache_path_utf16(
            nativeView(dataRootText), utf8View(entry.id), utf8View(entry.version), output,
            capacity);
    });
    return archive.empty() ? fs::path{} : fs::path(archive);
}

fs::path preparePackageArchiveCache(const fs::path& dataRoot,
                                    const fcitx::package::RepositoryEntry& entry,
                                    bool existingHashMatches) {
    const std::wstring dataRootText = dataRoot.wstring();
    std::wstring archive(32768, L'\0');
    const auto result = fcitx5_control_package_archive_cache_prepare_utf16(
        nativeView(dataRootText), utf8View(entry.id), utf8View(entry.version),
        existingHashMatches ? 1 : 0, archive.data(), archive.size());
    if (result.status == kControlArchiveCacheInvalid || result.pathLen == 0 ||
        result.pathLen > archive.size())
        throw fcitx::package::PackageError("io_error", "package download cache preparation failed");
    archive.resize(result.pathLen);
    return archive;
}

struct SequenceState {
    bool present{};
    bool valid{};
    std::uint64_t maximum{};
};

SequenceState readSequenceState(const fs::path& dataRoot, std::string_view channel) {
    RepositorySequenceStateNative native{};
    const auto channelWide = widen(channel);
    if (channelWide.empty() && !channel.empty())
        return {.present = true, .valid = false, .maximum = 0};
    if (fcitx5_repository_sequence_state_read_utf16(
            dataRoot.c_str(), dataRoot.native().size(), channelWide.c_str(), channelWide.size(),
            &native) != 0) {
        return {.present = true, .valid = false, .maximum = 0};
    }
    return {.present = native.present != 0, .valid = native.valid != 0, .maximum = native.maximum};
}

std::uint64_t readMaxSequence(const fs::path& dataRoot, std::string_view channel,
                              bool sequenceStateExpected) {
    const auto state = readSequenceState(dataRoot, channel);
    if (state.valid)
        return state.maximum;
    if (state.present)
        throw fcitx::package::PackageError(
            "sequence_state_corrupt",
            "repository anti-rollback sequence state is corrupt; run explicit repair/reset");
    if (sequenceStateExpected)
        throw fcitx::package::PackageError(
            "sequence_state_missing",
            "repository anti-rollback sequence state is missing; run explicit repair/reset");
    return 0;
}

void writeMaxSequence(const fs::path& dataRoot, std::string_view channel,
                      std::uint64_t maximum) {
    const auto channelWide = widen(channel);
    if (channelWide.empty() && !channel.empty()) {
        throw fcitx::package::PackageError(
            "io_error", "repository sequence state publication failed");
    }
    if (fcitx5_repository_sequence_state_write_utf16(
            dataRoot.c_str(), dataRoot.native().size(), channelWide.c_str(), channelWide.size(),
            maximum) != 0) {
        throw fcitx::package::PackageError(
            "io_error", "repository sequence state publication failed");
    }
}

std::uint64_t repositoryMaxReleaseSequence(const fcitx::package::RepositoryIndex& repository) {
    std::vector<std::uint64_t> sequences;
    sequences.reserve(repository.packages.size());
    for (const auto& entry : repository.packages)
        sequences.push_back(entry.release_sequence);
    const auto* data = sequences.empty() ? nullptr : sequences.data();
    return fcitx5_control_repository_max_release_sequence(data, sequences.size());
}

std::wstring repositoryMetadataUrl(std::wstring_view baseUrl, std::string_view metadataName) {
    const std::size_t required = fcitx5_control_repository_metadata_url_utf16(
        nativeView(baseUrl), utf8View(metadataName), nullptr, 0);
    if (required == 0)
        return {};
    std::wstring result(required, L'\0');
    const std::size_t written = fcitx5_control_repository_metadata_url_utf16(
        nativeView(baseUrl), utf8View(metadataName), result.data(), result.size());
    return written == result.size() ? result : std::wstring{};
}

std::wstring repositoryDefaultBaseUrl(std::string_view channel) {
    const std::size_t required =
        fcitx5_control_repository_default_base_url_utf16(utf8View(channel), nullptr, 0);
    if (required == 0)
        return {};
    std::wstring result(required, L'\0');
    const std::size_t written = fcitx5_control_repository_default_base_url_utf16(
        utf8View(channel), result.data(), result.size());
    return written == result.size() ? result : std::wstring{};
}

std::string packageTransactionId(std::string_view sha256) {
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_package_transaction_id_utf8(utf8View(sha256), &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

fcitx::package::RepositoryIndex loadRepository(const fs::path& dataRoot) {
    const auto files = repositoryFiles(dataRoot);
    std::string index;
    if (!readUtf8(files.index, index))
        throw fcitx::package::PackageError("repository_unavailable",
                                           "repository cache is unavailable");
    const auto signature = readBinary(files.signature, 16U * 1024U);
    const auto repository = fcitx::package::verify_repository_index(
        index, signature, fcitx::package::read_trusted_keys(files.keyring),
        fcitx::windows::kReleaseIdentity.channel_name);
    // Defense in depth: the cached index itself must not be an older
    // sequence than what was previously accepted, even if the cache file was
    // replaced outside the refresh path.
    if (repositoryMaxReleaseSequence(repository) <
        readMaxSequence(dataRoot, repository.channel, true))
        throw fcitx::package::PackageError(
            "rollback_rejected",
            "cached repository index is older than the accepted release sequence");
    return repository;
}

void refreshRepository(const fs::path& dataRoot, std::wstring baseUrl) {
    const auto indexUrl = repositoryMetadataUrl(baseUrl, "index.json");
    const auto signatureUrl = repositoryMetadataUrl(baseUrl, "index.sig");
    if (indexUrl.empty() || signatureUrl.empty())
        throw fcitx::package::PackageError("network_error", "repository metadata URL is invalid");
    const auto files = repositoryFiles(dataRoot);
    if (!prepareRepositoryCache(files))
        throw fcitx::package::PackageError("io_error", "repository cache staging failed");
    const auto incomingIndex = repositoryIncomingPath(files.index);
    const auto incomingSignature = repositoryIncomingPath(files.signature);
    if (incomingIndex.empty() || incomingSignature.empty())
        throw fcitx::package::PackageError("io_error", "repository cache staging failed");
    const auto downloader = executableDirectory() / L"fcitx5-downloader.exe";
    if (!runProcess(downloader,
                    {L"--download-signed-metadata", indexUrl, incomingIndex.wstring()}) ||
        !runProcess(downloader,
                    {L"--download-signed-metadata", signatureUrl, incomingSignature.wstring()})) {
        cleanupRepositoryCache(files);
        throw fcitx::package::PackageError("network_error", "repository download failed");
    }
    std::string index;
    if (!readUtf8(incomingIndex, index))
        throw fcitx::package::PackageError("invalid_repository", "repository index is unreadable");
    const auto signature = readBinary(incomingSignature, 16U * 1024U);
    const auto repository = fcitx::package::verify_repository_index(
        index, signature, fcitx::package::read_trusted_keys(files.keyring),
        fcitx::windows::kReleaseIdentity.channel_name);
    // Anti-rollback: reject an index whose release sequences are older than
    // the highest previously accepted for this channel. The signature proves
    // who published it, not that it is the latest; the sequence check keeps
    // a stale-but-valid index from being treated as an update.
    const auto maximum = repositoryMaxReleaseSequence(repository);
    const auto accepted = readMaxSequence(
        dataRoot, repository.channel,
        fs::exists(files.index) ||
            readSequenceState(dataRoot, repository.channel).present);
    if (maximum < accepted)
        throw fcitx::package::PackageError("rollback_rejected",
                                           "repository index is older than the accepted "
                                           "release sequence");
    if (!publishRepositoryCache(files)) {
        throw fcitx::package::PackageError("io_error", "repository cache publication failed");
    }
    writeMaxSequence(dataRoot, repository.channel, maximum);
}

std::string repairRepositorySequenceState(
    const fs::path& dataRoot, std::span<const fcitx::package::TrustedKey> trustedKeys) {
    const auto files = repositoryFiles(dataRoot);
    const auto channel = fcitx::windows::kReleaseIdentity.channel_name;
    return fcitx::package::repair_repository_sequence_state(dataRoot, files.index, files.signature,
                                                            trustedKeys, channel);
}

std::uint32_t packageTypeValue(fcitx::package::PackageType type) {
    using fcitx::package::PackageType;
    switch (type) {
    case PackageType::core:
        return 0;
    case PackageType::addon:
        return 1;
    case PackageType::input_method_data:
        return 2;
    case PackageType::theme:
        return 3;
    case PackageType::translation:
        return 4;
    }
    return 0;
}

std::string packageTypeName(fcitx::package::PackageType type) {
    return copyUtf8(fcitx5_control_package_type_name_utf8(packageTypeValue(type)));
}

bool packageUpdateAvailable(bool installedPresent, std::string_view installedVersion,
                            std::string_view availableVersion) {
    return fcitx5_control_package_update_available_utf8(
               installedPresent ? std::uint8_t{1} : std::uint8_t{0},
               utf8View(installedVersion), utf8View(availableVersion)) != 0;
}

bool packageStateSatisfiesDependency(std::string_view state) {
    return fcitx5_control_package_state_satisfies_dependency_utf8(utf8View(state)) != 0;
}

bool packageStateKeepsInstalledVersion(std::string_view state) {
    return fcitx5_control_package_state_keeps_installed_version_utf8(utf8View(state)) != 0;
}

std::string jsonDependencies(std::span<const fcitx::package::Dependency> dependencies) {
    std::vector<Fcitx5ControlPackageDependency> views;
    views.reserve(dependencies.size());
    for (const auto& dependency : dependencies) {
        views.push_back({utf8View(dependency.id), utf8View(dependency.version)});
    }
    char* bytes = nullptr;
    std::size_t length = 0;
    const auto* data = views.empty() ? nullptr : views.data();
    if (fcitx5_control_package_dependencies_json_utf8(data, views.size(), &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::string jsonStringArray(std::span<const std::string> values) {
    std::vector<Fcitx5ControlUtf8> views;
    views.reserve(values.size());
    for (const auto& value : values) {
        views.push_back(utf8View(value));
    }
    char* bytes = nullptr;
    std::size_t length = 0;
    const auto* data = views.empty() ? nullptr : views.data();
    if (fcitx5_control_string_array_json_utf8(data, views.size(), &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::string installedManifestBytes(const fs::path& packageRoot,
                                   const fcitx::package::LockEntry& entry) {
    const std::wstring packageRootText = packageRoot.wstring();
    char* bytes = nullptr;
    std::size_t length = 0;
    const auto status = fcitx5_control_installed_manifest_bytes_utf16(
        nativeView(packageRootText), utf8View(entry.id), utf8View(entry.version), &bytes, &length);
    if (status != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::string configSurfaceJson(const fcitx::package::Manifest* manifest,
                              fcitx::package::PackageType type,
                              std::string_view packageId) {
    std::vector<Fcitx5ControlUtf8> permissions;
    std::vector<Fcitx5ControlUtf8> filePaths;
    if (manifest) {
        permissions.reserve(manifest->permissions.size());
        for (const auto& permission : manifest->permissions)
            permissions.push_back(utf8View(permission));
        filePaths.reserve(manifest->files.size());
        for (const auto& file : manifest->files)
            filePaths.push_back(utf8View(file.path));
    }
    char* bytes = nullptr;
    std::size_t length = 0;
    const auto* permissionsData = permissions.empty() ? nullptr : permissions.data();
    const auto* filePathsData = filePaths.empty() ? nullptr : filePaths.data();
    if (fcitx5_control_package_config_surface_json_utf8(
            utf8View(packageId), packageTypeValue(type), permissionsData, permissions.size(),
            filePathsData, filePaths.size(), &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

struct ThemeRecord {
    std::string id;
    std::string source;
    std::string name;
    std::string version;
    std::string license;
    std::string description;
    fcitx::windows::config::Theme theme;
};

struct AddonDescriptor {
    std::string id;
    std::string name;
    std::string category;
    std::string library;
    std::string type;
    std::string version;
    bool configurable{};
    bool onDemand{};
    bool libraryPresent{};
};

std::string_view trimAscii(std::string_view value) {
    while (!value.empty() &&
           (value.front() == ' ' || value.front() == '\t' || value.front() == '\r'))
        value.remove_prefix(1);
    while (!value.empty() &&
           (value.back() == ' ' || value.back() == '\t' || value.back() == '\r'))
        value.remove_suffix(1);
    return value;
}

bool addonMetadataBool(std::string_view value) {
    return fcitx5_control_addon_metadata_bool_utf8(utf8View(value)) != 0;
}

bool addonMetadataSectionIsAddon(std::string_view section) {
    return fcitx5_control_addon_metadata_section_is_addon_utf8(utf8View(section)) != 0;
}

enum class AddonMetadataKey : std::uint32_t {
    unknown = 0,
    name = 1,
    category = 2,
    library = 3,
    type = 4,
    version = 5,
    configurable = 6,
    onDemand = 7,
};

AddonMetadataKey addonMetadataKey(std::string_view key) {
    return static_cast<AddonMetadataKey>(fcitx5_control_addon_metadata_key_utf8(utf8View(key)));
}

std::optional<AddonDescriptor> parseAddonDescriptor(const fs::path& path,
                                                    const fs::path& libraryRoot) {
    std::string text;
    if (!readUtf8(path, text))
        return std::nullopt;
    AddonDescriptor descriptor;
    descriptor.id = narrow(path.stem().wstring());
    bool inAddon = false;
    std::size_t offset = 0;
    while (offset <= text.size()) {
        const std::size_t end = text.find('\n', offset);
        std::string_view line(text.data() + offset,
                              (end == std::string::npos ? text.size() : end) - offset);
        offset = end == std::string::npos ? text.size() + 1 : end + 1;
        line = trimAscii(line);
        if (line.empty() || line.front() == '#')
            continue;
        if (line.front() == '[' && line.back() == ']') {
            inAddon = addonMetadataSectionIsAddon(line);
            continue;
        }
        if (!inAddon)
            continue;
        const std::size_t separator = line.find('=');
        if (separator == std::string_view::npos)
            continue;
        const auto key = trimAscii(line.substr(0, separator));
        const auto value = trimAscii(line.substr(separator + 1));
        switch (addonMetadataKey(key)) {
        case AddonMetadataKey::name:
            descriptor.name = std::string(value);
            break;
        case AddonMetadataKey::category:
            descriptor.category = std::string(value);
            break;
        case AddonMetadataKey::library:
            descriptor.library = std::string(value);
            break;
        case AddonMetadataKey::type:
            descriptor.type = std::string(value);
            break;
        case AddonMetadataKey::version:
            descriptor.version = std::string(value);
            break;
        case AddonMetadataKey::configurable:
            descriptor.configurable = addonMetadataBool(value);
            break;
        case AddonMetadataKey::onDemand:
            descriptor.onDemand = addonMetadataBool(value);
            break;
        case AddonMetadataKey::unknown:
            break;
        }
    }
    if (descriptor.id.empty() || !fcitx::package::is_lower_package_id(descriptor.id) ||
        descriptor.name.empty())
        return std::nullopt;
    if (!descriptor.library.empty()) {
        const auto dllName = widen(descriptor.library + ".dll");
        descriptor.libraryPresent = !dllName.empty() && fs::is_regular_file(libraryRoot / dllName);
    }
    return descriptor;
}

std::vector<AddonDescriptor> listAddonDescriptors() {
    std::vector<AddonDescriptor> result;
    const fs::path installRoot = installationRoot();
    const fs::path addonRoot = installRoot / L"share/fcitx5/addon";
    const fs::path libraryRoot = installRoot / L"lib/fcitx5";
    std::error_code error;
    if (!fs::is_directory(addonRoot, error))
        return result;
    for (const auto& entry : fs::directory_iterator(addonRoot, error)) {
        if (error)
            break;
        if (!entry.is_regular_file(error) || entry.path().extension() != L".conf")
            continue;
        if (auto descriptor = parseAddonDescriptor(entry.path(), libraryRoot))
            result.push_back(std::move(*descriptor));
    }
    std::ranges::sort(result, {}, &AddonDescriptor::id);
    return result;
}

std::string addonsJson(const std::vector<AddonDescriptor>& addons) {
    std::vector<Fcitx5ControlAddonDescriptor> views;
    views.reserve(addons.size());
    for (const auto& addon : addons) {
        views.push_back(Fcitx5ControlAddonDescriptor{
            utf8View(addon.id),
            utf8View(addon.name),
            utf8View(addon.category),
            utf8View(addon.library),
            utf8View(addon.type),
            utf8View(addon.version),
            addon.configurable ? std::uint8_t{1} : std::uint8_t{0},
            addon.onDemand ? std::uint8_t{1} : std::uint8_t{0},
            addon.libraryPresent ? std::uint8_t{1} : std::uint8_t{0}});
    }
    char* bytes = nullptr;
    std::size_t length = 0;
    const auto* data = views.empty() ? nullptr : views.data();
    if (fcitx5_control_addons_json_utf8(data, views.size(), &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

void printAddons() {
    const auto addons = listAddonDescriptors();
    std::cout << addonsJson(addons) << '\n';
}

std::optional<ThemeRecord> loadThemeRecord(const fs::path& path, std::string id,
                                           std::string source) {
    std::string text;
    if (!readUtf8(path, text))
        return std::nullopt;
    fcitx::windows::config::Theme theme;
    ParseError error;
    if (!fcitx::windows::config::parseTheme(text, theme, error))
        return std::nullopt;
    if (source == "user" && theme.id != id)
        return std::nullopt;
    return ThemeRecord{std::move(id), std::move(source), theme.name, theme.version,
                       theme.license, theme.description, std::move(theme)};
}

std::vector<ThemeRecord> listThemes(const fs::path& dataRoot) {
    std::vector<ThemeRecord> result;
    if (auto builtin = loadThemeRecord(installationRoot() / L"resources/themes/default/theme.toml",
                                       "builtin:default", "builtin")) {
        result.push_back(std::move(*builtin));
    }
    const auto userThemes = dataRoot / L"themes";
    std::error_code error;
    if (fs::is_directory(userThemes, error)) {
        for (const auto& entry : fs::directory_iterator(userThemes, error)) {
            if (error)
                break;
            if (!entry.is_directory(error))
                continue;
            const auto id = narrow(entry.path().filename().wstring());
            if (!fcitx::package::is_lower_package_id(id))
                continue;
            if (auto theme = loadThemeRecord(entry.path() / L"theme.toml", id, "user"))
                result.push_back(std::move(*theme));
        }
    }
    std::ranges::sort(result, {}, &ThemeRecord::id);
    return result;
}

const ThemeRecord* findTheme(std::span<const ThemeRecord> themes, std::string_view id) {
    const auto found = std::ranges::find_if(themes, [&](const ThemeRecord& theme) {
        return theme.id == id;
    });
    return found == themes.end() ? nullptr : &*found;
}

Fcitx5ControlThemeRecord themeView(const ThemeRecord& theme) {
    return Fcitx5ControlThemeRecord{utf8View(theme.id),      utf8View(theme.source),
                                    utf8View(theme.name),    utf8View(theme.version),
                                    utf8View(theme.license), utf8View(theme.description)};
}

std::string themesJson(const std::vector<ThemeRecord>& themes) {
    std::vector<Fcitx5ControlThemeRecord> views;
    views.reserve(themes.size());
    for (const auto& theme : themes)
        views.push_back(themeView(theme));
    char* bytes = nullptr;
    std::size_t length = 0;
    const auto* data = views.empty() ? nullptr : views.data();
    if (fcitx5_control_themes_json_utf8(data, views.size(), &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::string themeDetailJson(const ThemeRecord& theme) {
    const Fcitx5ControlThemeDetail detail{
        themeView(theme), !theme.theme.light.colors.empty() ? std::uint8_t{1} : std::uint8_t{0},
        !theme.theme.dark.colors.empty() ? std::uint8_t{1} : std::uint8_t{0}};
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_theme_detail_json_utf8(&detail, &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

void printThemes(const fs::path& dataRoot) {
    const auto themes = listThemes(dataRoot);
    std::cout << themesJson(themes) << '\n';
}

void printThemeDetail(const fs::path& dataRoot, std::string_view id) {
    if (id != "builtin:default" && !fcitx::package::is_lower_package_id(id))
        throw fcitx::package::PackageError("invalid_theme", "theme id is invalid");
    const auto themes = listThemes(dataRoot);
    const auto* theme = findTheme(themes, id);
    if (!theme)
        throw fcitx::package::PackageError("theme_not_found", "theme is unknown");
    std::cout << themeDetailJson(*theme) << '\n';
}

fs::path defaultDataRoot() {
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity) ||
        identity.executablePath.empty())
        return {};
    return fcitx::windows::platform::defaultDataRootForModule(
        identity.executablePath, fcitx::windows::kReleaseIdentity.data_directory);
}

bool validateConfig(const fs::path& source, std::string& text, ParseError& parseError) {
    if (!readUtf8(source, text))
        return false;
    Config config;
    return fcitx::windows::config::parseConfig(text, config, parseError);
}

bool atomicWrite(const fs::path& destination, std::string_view text) {
    const std::wstring destinationText = destination.wstring();
    return fcitx5_control_atomic_write_utf8_file_utf16(
               {destinationText.data(), destinationText.size()}, utf8View(text)) == 0;
}

bool writeVisualConfig(const fs::path& destination, std::string_view text) {
    if (!atomicWrite(destination, text))
        return false;
    const UINT message = RegisterWindowMessageW(kVisualConfigChangedMessage);
    if (message != 0)
        (void)PostMessageW(HWND_BROADCAST, message, 0, 0);
    return true;
}

bool launcherCommand(fcitx::windows::protocol::LauncherCommand command,
                     fcitx::windows::protocol::LauncherResponse& response) {
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity))
        return false;
    const auto policy = fcitx::windows::ipc::PeerPolicy::exact(
        (executableDirectory() / L"fcitx5-launcher.exe").wstring());
    return fcitx::windows::ipc::sendLauncherCommand(
        identity, fcitx5_windows_common_deadline_after_milliseconds(1000), policy, command,
        response);
}

std::optional<fcitx::windows::protocol::LauncherCommand>
launcherCommandFromRust(std::uint32_t command) {
    using fcitx::windows::protocol::LauncherCommand;
    switch (command) {
    case static_cast<std::uint32_t>(LauncherCommand::startDemand):
        return LauncherCommand::startDemand;
    case static_cast<std::uint32_t>(LauncherCommand::userStop):
        return LauncherCommand::userStop;
    case static_cast<std::uint32_t>(LauncherCommand::resume):
        return LauncherCommand::resume;
    case static_cast<std::uint32_t>(LauncherCommand::shutdown):
        return LauncherCommand::shutdown;
    default:
        return std::nullopt;
    }
}

bool runLauncherAction(std::uint32_t action) {
    const std::uint32_t* commands = nullptr;
    std::size_t commandCount = 0;
    if (fcitx5_control_launcher_action_sequence(action, &commands, &commandCount) != 0 ||
        commands == nullptr || commandCount == 0) {
        return false;
    }
    fcitx::windows::protocol::LauncherResponse response;
    for (std::size_t index = 0; index < commandCount; ++index) {
        const auto command = launcherCommandFromRust(commands[index]);
        if (!command || !launcherCommand(*command, response))
            return false;
    }
    return true;
}

bool runEngineManagement(const std::vector<std::wstring>& arguments, std::string& output) {
    fcitx::windows::protocol::LauncherResponse response;
    const bool launcherReachable =
        launcherCommand(fcitx::windows::protocol::LauncherCommand::status, response);
    if (launcherReachable &&
        !launcherCommand(fcitx::windows::protocol::LauncherCommand::userStop, response))
        return false;
    std::wstring wideOutput;
    const bool commandOk = fcitx::windows::config::runExecutable(
        executableDirectory() / L"fcitx5-engine.exe", arguments, wideOutput);
    output = narrow(wideOutput);
    bool restoreOk = true;
    if (launcherReachable) {
        restoreOk = launcherCommand(fcitx::windows::protocol::LauncherCommand::resume, response) &&
                    launcherCommand(fcitx::windows::protocol::LauncherCommand::startDemand,
                                    response);
    }
    return commandOk && restoreOk;
}

std::string nativeArchitecture() {
    return copyUtf8(fcitx5_control_native_package_architecture_utf8());
}

bool packageArchitectureMatchesNative(std::string_view architecture) {
    return fcitx5_control_package_architecture_matches_native_utf8(utf8View(architecture)) != 0;
}

void installRepositoryPackage(const fs::path& dataRoot,
                              const fcitx::package::RepositoryIndex& repository,
                              std::string_view packageId, std::set<std::string>& visiting) {
    const auto* entry =
        fcitx::package::find_repository_package(repository, packageId, nativeArchitecture());
    if (entry == nullptr)
        throw fcitx::package::PackageError("package_not_found",
                                           "package is not present in this repository");
    if (!visiting.emplace(entry->id).second)
        throw fcitx::package::PackageError("resolution_failed",
                                           "repository dependency cycle detected");
    const auto packageRoot = dataRoot / L"packages";
    const auto lock = fcitx::package::read_lockfile(packageRoot);
    for (const auto& dependency : entry->dependencies) {
        const auto installed =
            std::ranges::find_if(lock, [&](const fcitx::package::LockEntry& item) {
                return item.id == dependency.id && item.version == dependency.version &&
                       packageStateSatisfiesDependency(item.state);
            });
        if (installed == lock.end()) {
            const auto* dependencyEntry = fcitx::package::find_repository_package(
                repository, dependency.id, nativeArchitecture());
            if (dependencyEntry == nullptr || dependencyEntry->version != dependency.version) {
                throw fcitx::package::PackageError(
                    "resolution_failed", "exact dependency is absent from the repository");
            }
            installRepositoryPackage(dataRoot, repository, dependency.id, visiting);
        }
    }
    visiting.erase(entry->id);
    const auto current = fcitx::package::read_lockfile(packageRoot);
    const auto same = std::ranges::find_if(current, [&](const fcitx::package::LockEntry& item) {
        return item.id == entry->id && item.version == entry->version &&
               packageStateKeepsInstalledVersion(item.state);
    });
    if (same != current.end())
        return;

    const auto archive = packageArchiveCachePath(dataRoot, *entry);
    if (archive.empty())
        throw fcitx::package::PackageError("io_error", "package download cache preparation failed");
    bool validCache = false;
    if (fs::exists(archive)) {
        validCache =
            fcitx::package::hex_sha256(fcitx::package::sha256_file(archive)) == entry->sha256;
    }
    const auto preparedArchive = preparePackageArchiveCache(dataRoot, *entry, validCache);
    if (preparedArchive != archive)
        throw fcitx::package::PackageError("io_error", "package download cache preparation failed");
    if (!validCache && !runProcess(executableDirectory() / L"fcitx5-downloader.exe",
                                   {L"--download", widen(entry->download_url), widen(entry->sha256),
                                    archive.wstring()})) {
        throw fcitx::package::PackageError("network_error", "package download failed");
    }
    const auto keys = fcitx::package::read_trusted_keys(repositoryFiles(dataRoot).keyring);
    const std::string transaction = packageTransactionId(entry->sha256);
    const auto staged =
        fcitx::package::stage_verified_archive(archive, packageRoot, transaction, keys);
    fcitx::package::activate_staged_package(staged, packageRoot, keys);
}

void requestEngineReload() {
    fcitx::windows::protocol::LauncherResponse response;
    if (!launcherCommand(fcitx::windows::protocol::LauncherCommand::status, response))
        return;
    if (!launcherCommand(fcitx::windows::protocol::LauncherCommand::userStop, response) ||
        !launcherCommand(fcitx::windows::protocol::LauncherCommand::resume, response) ||
        !launcherCommand(fcitx::windows::protocol::LauncherCommand::startDemand, response)) {
        // The package transaction is already durable at this point. Never report that install,
        // update, or state persistence failed merely because a concurrent launcher transition
        // could not hot-reload it; the tray's restart action remains available to the user.
        std::cerr << "warning: package change is saved; restart the input service to activate it\n";
    }
}

struct PackageSummaryRow {
    std::string id;
    std::string title;
    std::string summary;
    std::string type;
    std::string availableVersion;
    std::string installedVersion;
    std::string state;
    bool updateAvailable{};
};

struct BundledComponent {
    std::string id;
    std::string title;
};

bool bundledPackagePresent(const fs::path& installRoot, std::string_view id) {
    const std::wstring installRootText = installRoot.wstring();
    return fcitx5_control_bundled_package_present_utf16(
               nativeView(installRootText), utf8View(id)) != 0;
}

std::vector<BundledComponent> presentBundledPackages(const fs::path& installRoot) {
    std::vector<BundledComponent> result;
    const std::size_t count = fcitx5_control_bundled_package_count();
    result.reserve(count);
    for (std::size_t index = 0; index < count; ++index) {
        Fcitx5ControlBundledPackageDescriptor descriptor{};
        if (fcitx5_control_bundled_package_descriptor(index, &descriptor) == 0)
            continue;
        const std::string id = copyUtf8(descriptor.id);
        if (id.empty() || !bundledPackagePresent(installRoot, id))
            continue;
        result.push_back(BundledComponent{id, copyUtf8(descriptor.title)});
    }
    return result;
}

std::string packagesListJson(bool repositoryAvailable, std::string_view repositoryError,
                             const std::vector<PackageSummaryRow>& packages) {
    std::vector<Fcitx5ControlPackageSummary> views;
    views.reserve(packages.size());
    for (const auto& package : packages) {
        views.push_back(Fcitx5ControlPackageSummary{
            utf8View(package.id),
            utf8View(package.title),
            utf8View(package.summary),
            utf8View(package.type),
            utf8View(package.availableVersion),
            utf8View(package.installedVersion),
            utf8View(package.state),
            package.updateAvailable ? std::uint8_t{1} : std::uint8_t{0}});
    }
    const Fcitx5ControlPackagesList list{
        repositoryAvailable ? std::uint8_t{1} : std::uint8_t{0}, utf8View(repositoryError),
        views.empty() ? nullptr : views.data(), views.size()};
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_packages_list_json_utf8(&list, &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

std::string packageDetailJson(const Fcitx5ControlPackageDetail& detail) {
    char* bytes = nullptr;
    std::size_t length = 0;
    if (fcitx5_control_package_detail_json_utf8(&detail, &bytes, &length) != 0)
        return {};
    return takeRustUtf8(bytes, length);
}

void printPackages(const fs::path& dataRoot) {
    const auto root = dataRoot / L"packages";
    const auto installed = fcitx::package::read_lockfile(root);
    std::map<std::string, fcitx::package::LockEntry, std::less<>> active;
    for (const auto& entry : installed)
        active.emplace(entry.id, entry);
    const fs::path installRoot = installationRoot();
    std::map<std::string, BundledComponent, std::less<>> bundled;
    for (auto component : presentBundledPackages(installRoot))
        bundled.emplace(component.id, std::move(component));
    fcitx::package::RepositoryIndex repository;
    bool repositoryAvailable = false;
    std::string repositoryError;
    try {
        repository = loadRepository(dataRoot);
        repositoryAvailable = true;
    } catch (const fcitx::package::PackageError& error) {
        repositoryError = repositoryErrorCode(dataRoot, error.code());
    }
    std::vector<PackageSummaryRow> packageRows;
    std::set<std::string> emitted;
    if (repositoryAvailable) {
        for (const auto& entry : repository.packages) {
            if (!packageArchitectureMatchesNative(entry.architecture))
                continue;
            const auto found = active.find(entry.id);
            const bool bundledNow = bundled.contains(entry.id);
            const bool installedNow = found != active.end();
            const std::string_view installedVersion =
                installedNow ? std::string_view(found->second.version) : std::string_view{};
            const bool update =
                packageUpdateAvailable(installedNow, installedVersion, entry.version);
            packageRows.push_back(PackageSummaryRow{
                entry.id,
                entry.title,
                entry.summary,
                packageTypeName(entry.type),
                entry.version,
                installedNow
                    ? found->second.version
                    : (bundledNow ? std::string(fcitx::windows::version()) : std::string{}),
                installedNow ? found->second.state
                             : (bundledNow ? std::string("bundled") : std::string{}),
                update});
            emitted.emplace(entry.id);
        }
    }
    for (const auto& entry : installed) {
        if (emitted.contains(entry.id))
            continue;
        packageRows.push_back(PackageSummaryRow{entry.id,
                                                entry.id,
                                                "",
                                                "unknown",
                                                "",
                                                entry.version,
                                                entry.state,
                                                false});
    }
    for (const auto& [id, component] : bundled) {
        if (emitted.contains(id) || active.contains(id))
            continue;
        packageRows.push_back(PackageSummaryRow{id,
                                                component.title,
                                                "Bundled with Fcitx5 for Windows Next",
                                                "addon",
                                                "",
                                                std::string(fcitx::windows::version()),
                                                "bundled",
                                                false});
    }
    std::cout << packagesListJson(repositoryAvailable, repositoryError, packageRows) << '\n';
}

void printPackageDetail(const fs::path& dataRoot, std::string_view packageId) {
    if (!fcitx::package::is_lower_package_id(packageId))
        throw fcitx::package::PackageError("invalid_package", "package id is invalid");
    const auto packageRoot = dataRoot / L"packages";
    const auto installed = fcitx::package::read_lockfile(packageRoot);
    const auto active = std::ranges::find_if(installed, [&](const fcitx::package::LockEntry& item) {
        return item.id == packageId;
    });
    std::optional<fcitx::package::Manifest> manifest;
    std::string manifestBytes;
    if (active != installed.end()) {
        manifestBytes = installedManifestBytes(packageRoot, *active);
        if (!manifestBytes.empty())
            manifest = fcitx::package::parse_manifest(manifestBytes);
    }
    fcitx::package::RepositoryIndex repository;
    const fcitx::package::RepositoryEntry* repositoryEntry = nullptr;
    bool repositoryAvailable = false;
    std::string repositoryError;
    try {
        repository = loadRepository(dataRoot);
        repositoryAvailable = true;
        repositoryEntry = fcitx::package::find_repository_package(repository, packageId,
                                                                  nativeArchitecture());
    } catch (const fcitx::package::PackageError& error) {
        repositoryError = repositoryErrorCode(dataRoot, error.code());
    }

    const bool bundledNow = bundledPackagePresent(installationRoot(), packageId);
    if (active == installed.end() && !repositoryEntry && !bundledNow)
        throw fcitx::package::PackageError("package_not_found", "package is unknown");

    const std::string type = manifest ? packageTypeName(manifest->type)
                             : repositoryEntry ? packageTypeName(repositoryEntry->type)
                                               : "addon";
    const std::string title = repositoryEntry ? repositoryEntry->title : std::string(packageId);
    const std::string summary = repositoryEntry ? repositoryEntry->summary
                                : bundledNow ? "Bundled with Fcitx5 for Windows Next"
                                             : "";
    const std::string available = repositoryEntry ? repositoryEntry->version : "";
    const std::string installedVersion = active != installed.end()
                                             ? active->version
                                             : (bundledNow ? std::string(fcitx::windows::version())
                                                          : "");
    const std::string state = active != installed.end()
                                  ? active->state
                                  : (bundledNow ? "bundled" : "");
    const bool activeInstalled = active != installed.end();
    const bool update =
        packageUpdateAvailable(activeInstalled, activeInstalled ? std::string_view(active->version)
                                                                : std::string_view{},
                               repositoryEntry ? std::string_view(repositoryEntry->version)
                                               : std::string_view{});
    const auto typeValue = manifest ? manifest->type
                                    : (repositoryEntry ? repositoryEntry->type
                                                       : fcitx::package::PackageType::addon);
    const std::string dependencies =
        manifest ? jsonDependencies(manifest->dependencies)
                 : (repositoryEntry ? jsonDependencies(repositoryEntry->dependencies) : "[]");
    const std::string permissions =
        manifest ? jsonStringArray(manifest->permissions) : "[]";
    const std::string configSurface =
        configSurfaceJson(manifest ? &*manifest : nullptr, typeValue, packageId);
    const std::string packageIdText{packageId};
    const std::string manifestSha256 =
        active != installed.end() ? active->manifest_sha256 : std::string{};
    const std::string sourceCommit = manifest ? manifest->source_commit : std::string{};
    const Fcitx5ControlPackageDetail detail{
        repositoryAvailable ? std::uint8_t{1} : std::uint8_t{0},
        utf8View(repositoryError),
        utf8View(packageIdText),
        utf8View(title),
        utf8View(summary),
        utf8View(type),
        utf8View(available),
        utf8View(installedVersion),
        utf8View(state),
        bundledNow ? std::uint8_t{1} : std::uint8_t{0},
        update ? std::uint8_t{1} : std::uint8_t{0},
        utf8View(manifestSha256),
        utf8View(sourceCommit),
        utf8View(dependencies),
        utf8View(permissions),
        utf8View(configSurface)};
    std::cout << packageDetailJson(detail) << '\n';
}

bool printControlSchema() {
    const char* schema = nullptr;
    std::size_t schemaLength = 0;
    if (fcitx5_control_schema_json_utf8(&schema, &schemaLength) != 0 || schema == nullptr)
        return false;
    std::cout.write(schema, static_cast<std::streamsize>(schemaLength));
    std::cout << '\n';
    return std::cout.good();
}

bool printUsage() {
    const char* usage = nullptr;
    std::size_t usageLength = 0;
    if (fcitx5_control_usage_text_utf8(&usage, &usageLength) != 0 || usage == nullptr)
        return false;
    std::cerr.write(usage, static_cast<std::streamsize>(usageLength));
    return std::cerr.good();
}

bool validInputMethodId(std::wstring_view value) noexcept {
    return fcitx5_control_input_method_id_valid_utf16(nativeView(value)) != 0;
}

bool queryStartup(bool& enabled) {
    const std::wstring directory = executableDirectory().wstring();
    const std::wstring registryValue = fcitx::windows::kReleaseIdentity.registry_value;
    std::uint8_t rustEnabled = 0;
    if (fcitx5_control_startup_query_utf16(nativeView(directory), nativeView(registryValue),
                                           &rustEnabled) != 0) {
        enabled = false;
        return false;
    }
    enabled = rustEnabled != 0;
    return true;
}

bool setStartup(bool enabled) {
    const std::wstring directory = executableDirectory().wstring();
    const std::wstring registryValue = fcitx::windows::kReleaseIdentity.registry_value;
    return fcitx5_control_startup_set_utf16(nativeView(directory), nativeView(registryValue),
                                           enabled ? 1 : 0) == 0;
}

void usage() {
    if (!printUsage())
        std::cerr << "Usage: fcitx5-control --reset-presentation|--schema|--version\n";
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    fs::path dataRoot = defaultDataRoot();
    std::vector<std::wstring_view> arguments;
    for (int index = 1; index < argc; ++index) {
        const std::wstring_view argument(argv[index]);
        if (argument == L"--data-root" && index + 1 < argc)
            dataRoot = argv[++index];
        else
            arguments.push_back(argument);
    }
    const std::uint32_t rootCommand = rootAction(arguments);
    if (rootCommand == kRootActionVersion) {
        std::cout << fcitx::windows::version() << '\n';
        return 0;
    }
    if (arguments.empty()) {
        usage();
        return 2;
    }
    if (rootCommand == kRootActionSchema) {
        return printControlSchema() ? 0 : 5;
    }
    if (rootCommand == kRootActionGetStartup) {
        bool enabled = false;
        if (!queryStartup(enabled))
            return 5;
        std::cout << startupJson(enabled) << '\n';
        return 0;
    }
    if (rootCommand == kRootActionSetStartupEnabled ||
        rootCommand == kRootActionSetStartupDisabled) {
        return setStartup(rootCommand == kRootActionSetStartupEnabled) ? 0 : 5;
    }
    if (dataRoot.empty()) {
        std::cerr << "unable to resolve the user data directory\n";
        return 5;
    }
    if (rootCommand == kRootActionGetTsfGuard) {
        const auto status = fcitx::windows::tsf::activationGuardStatus(dataRoot);
        const std::string markerPath = narrow(status.markerPath.wstring());
        const Fcitx5ControlTsfGuard guardStatus{
            status.disabled ? std::uint8_t{1} : std::uint8_t{0}, utf8View(status.reason),
            utf8View(markerPath)};
        std::cout << tsfGuardJson(guardStatus) << '\n';
        return 0;
    }
    if (rootCommand == kRootActionResetTsfGuard) {
        if (!fcitx::windows::tsf::clearActivationGuard(dataRoot)) {
            std::cerr << "unable to clear TSF activation guard\n";
            return 5;
        }
        std::cout << tsfGuardResetJson() << '\n';
        return 0;
    }
    try {
        const std::uint32_t engineAction =
            arguments.empty() ? 0 : engineManagementAction(arguments[0], arguments.size());
        if (engineAction == kEngineActionGetInputMethods) {
            std::string output;
            if (!runEngineManagement({L"--list-input-methods"}, output))
                return 4;
            std::cout << output;
            return 0;
        }
        if (engineAction == kEngineActionSetInputMethod) {
            const std::string id = narrow(arguments[1]);
            if (id.empty() || !validInputMethodId(arguments[1]))
                return 2;
            std::string ignored;
            return runEngineManagement({L"--set-input-method", std::wstring(arguments[1])},
                                       ignored)
                       ? 0
                       : 4;
        }
        const std::uint32_t packageCommand = packageAction(arguments);
        if (packageCommand == kPackageActionPackagesList) {
            printPackages(dataRoot);
            return 0;
        }
        if (packageCommand == kPackageActionThemesList) {
            printThemes(dataRoot);
            return 0;
        }
        if (packageCommand == kPackageActionThemesDetail) {
            printThemeDetail(dataRoot, narrow(arguments[1]));
            return 0;
        }
        if (packageCommand == kPackageActionAddonsList) {
            printAddons();
            return 0;
        }
        if (packageCommand == kPackageActionPackagesDetail) {
            printPackageDetail(dataRoot, narrow(arguments[1]));
            return 0;
        }
        if (packageCommand == kPackageActionPackagesRefresh) {
            const auto defaultBase =
                repositoryDefaultBaseUrl(fcitx::windows::kReleaseIdentity.channel_name);
            refreshRepository(dataRoot,
                              arguments.size() == 2 ? std::wstring(arguments[1]) : defaultBase);
            printPackages(dataRoot);
            return 0;
        }
        if (packageCommand == kPackageActionPackagesInstall ||
            packageCommand == kPackageActionPackagesUpdate) {
            const auto repository = loadRepository(dataRoot);
            std::set<std::string> visiting;
            installRepositoryPackage(dataRoot, repository, narrow(arguments[1]), visiting);
            requestEngineReload();
            printPackages(dataRoot);
            return 0;
        }
        if (packageCommand == kPackageActionPackagesState) {
            fcitx::package::set_package_state(dataRoot / L"packages", narrow(arguments[1]),
                                              narrow(arguments[2]));
            requestEngineReload();
            return 0;
        }
        if (packageCommand == kPackageActionPackagesRemove) {
            const auto id = narrow(arguments[1]);
            fcitx::package::mark_package_for_removal(dataRoot / L"packages", id);
            requestEngineReload();
            fcitx::package::finalize_package_removal(dataRoot / L"packages", id);
            printPackages(dataRoot);
            return 0;
        }
        if (packageCommand == kPackageActionPackagesRepair) {
            const auto trustedKeys = fcitx::package::read_trusted_keys(
                repositoryFiles(dataRoot).keyring);
            fcitx::package::verify_installed_packages(dataRoot / L"packages", trustedKeys);
            const auto sequenceState = repairRepositorySequenceState(dataRoot, trustedKeys);
            const Fcitx5ControlPackageRepair repair{utf8View(sequenceState)};
            std::cout << packageRepairJson(repair) << '\n';
            return 0;
        }
    } catch (const fcitx::package::PackageError& error) {
        std::cerr << error.code() << ": " << error.what() << '\n';
        return 6;
    }
    const std::uint32_t controlConfigAction =
        arguments.empty() ? 0 : configAction(arguments[0], arguments.size());
    if (controlConfigAction == kConfigActionGetPresentation) {
        const fs::path configPath = dataRoot / L"config.toml";
        std::string text;
        if (readOptionalConfig(configPath, text) == OptionalConfigRead::error)
            return 5;
        ParseError error;
        Config defaults;
        if (!fcitx::windows::config::parseConfig(
                fcitx::windows::config::defaultConfigToml(), defaults, error))
            return 3;
        Config user;
        if (!text.empty() && !fcitx::windows::config::parseConfig(text, user, error))
            return 3;
        const Config config = fcitx::windows::config::mergeConfig(defaults, user);
        const char* mode =
            !config.appearanceMode ||
                    *config.appearanceMode == fcitx::windows::config::AppearanceMode::system
                ? "system"
                : (*config.appearanceMode == fcitx::windows::config::AppearanceMode::light
                       ? "light"
                       : "dark");
        const char* orientation =
            !config.orientation ||
                    *config.orientation == fcitx::windows::config::Orientation::automatic
                ? "automatic"
            : *config.orientation == fcitx::windows::config::Orientation::vertical ? "vertical"
                                                                                   : "horizontal";
        const std::string theme = config.theme.value_or("builtin:default");
        const bool scrollMode = config.scrollMode.value_or(false);
        const int pageSize = config.candidatePageSize.value_or(5);
        const double maxWidth = config.maxWidth.value_or(860.0);
        const double scrollCellWidth = config.scrollCellWidth.value_or(96.0);
        const double opacity = config.opacity.value_or(1.0);
        const double fontSize = config.candidateFont.size.value_or(18.0);
        const double cornerRadius = config.geometry.cornerRadius.value_or(12.0);
        const bool shadow = config.geometry.shadow.value_or(true);
        const char* preeditMode =
            config.preeditMode && *config.preeditMode == fcitx::windows::config::PreeditMode::panel
                ? "panel"
                : "inline";
        const std::string font =
            config.candidateFont.families && !config.candidateFont.families->empty()
                ? config.candidateFont.families->front()
                : "Microsoft YaHei";
        const std::string pageSizeText = std::to_string(pageSize);
        const std::string maxWidthText = std::to_string(static_cast<int>(maxWidth));
        const std::string scrollCellWidthText =
            std::to_string(static_cast<int>(scrollCellWidth));
        const std::string fontSizeText = std::to_string(static_cast<int>(fontSize));
        const std::string cornerRadiusText = std::to_string(static_cast<int>(cornerRadius));
        const std::string opacityText = std::to_string(opacity);
        const Fcitx5ControlPresentation presentation{
            utf8View(mode),
            utf8View(theme),
            utf8View(orientation),
            utf8View(font),
            utf8View(pageSizeText),
            utf8View(maxWidthText),
            utf8View(scrollCellWidthText),
            utf8View(fontSizeText),
            utf8View(cornerRadiusText),
            utf8View(opacityText),
            utf8View(preeditMode),
            static_cast<std::uint8_t>(shadow ? 1 : 0),
            static_cast<std::uint8_t>(scrollMode ? 1 : 0)};
        const std::string output = presentationJson(presentation);
        if (output.empty())
            return 5;
        std::cout << output << '\n';
        return 0;
    }
    if (controlConfigAction == kConfigActionSetPresentation) {
        const fs::path configPath = dataRoot / L"config.toml";
        std::string source = fcitx::windows::config::defaultConfigToml();
        if (readOptionalConfig(configPath, source) == OptionalConfigRead::error)
            return 5;
        std::string updated;
        ParseError error;
        if (!fcitx::windows::config::updatePresentationToml(
                source, narrow(arguments[1]), narrow(arguments[2]), narrow(arguments[3]),
                narrow(arguments[4]), narrow(arguments[5]), narrow(arguments[6]), updated,
                error, arguments.size() >= 9 ? narrow(arguments[7]) : std::string{},
                arguments.size() >= 9 ? narrow(arguments[8]) : std::string{},
                arguments.size() >= 12 ? narrow(arguments[9]) : std::string{},
                arguments.size() >= 12 ? narrow(arguments[10]) : std::string{},
                arguments.size() >= 12 ? narrow(arguments[11]) : std::string{},
                arguments.size() == 14 ? narrow(arguments[12]) : std::string{},
                arguments.size() == 14 ? narrow(arguments[13]) : std::string{})) {
            std::cerr << "invalid presentation at " << error.line << ':' << error.column << ": "
                      << error.message << '\n';
            return 3;
        }
        return writeVisualConfig(configPath, updated) ? 0 : 5;
    }
    if (rootCommand == kRootActionStatus || rootCommand == kRootActionDiagnosticsPlan) {
        fcitx::windows::protocol::LauncherResponse response;
        const bool reachable =
            launcherCommand(fcitx::windows::protocol::LauncherCommand::status, response);
        const fs::path configPath = dataRoot / L"config.toml";
        bool configValid = true;
        std::string text;
        ParseError error;
        const auto configRead = readOptionalConfig(configPath, text);
        if (configRead == OptionalConfigRead::error)
            configValid = false;
        else if (configRead == OptionalConfigRead::present) {
            Config parsed;
            configValid = fcitx::windows::config::parseConfig(text, parsed, error);
        }
        const auto tsfGuard = fcitx::windows::tsf::activationGuardStatus(dataRoot);
        const std::string dataRootUtf8 = narrow(dataRoot.generic_wstring());
        const std::string updateOwner{updateOwnerName(readUpdateOwner(installationRoot()))};
        const Fcitx5ControlStatus status{
            reachable ? std::uint8_t{1} : std::uint8_t{0},
            static_cast<std::int32_t>(response.launcherState),
            static_cast<std::int32_t>(response.engineState),
            utf8View(response.currentInputMethodId),
            utf8View(response.currentInputMethodName),
            utf8View(response.currentInputMethodNativeName),
            utf8View(response.currentInputMethodShortLabel),
            configValid ? std::uint8_t{1} : std::uint8_t{0},
            tsfGuard.disabled ? std::uint8_t{1} : std::uint8_t{0},
            utf8View(tsfGuard.reason),
            utf8View(dataRootUtf8),
            utf8View(updateOwner)};
        std::cout << (rootCommand == kRootActionDiagnosticsPlan ? diagnosticsPlanJson(status)
                                                                 : statusJson(status))
                  << '\n';
        if (rootCommand == kRootActionDiagnosticsPlan)
            return 0;
        return configValid ? 0 : 3;
    }
    if (rootCommand == kRootActionRestartEngine) {
        if (!runLauncherAction(kLauncherActionRestartEngine)) {
            std::cerr << "launcher unavailable or restart rejected\n";
            return 4;
        }
        return 0;
    }
    if (rootCommand == kRootActionShutdown) {
        return runLauncherAction(kLauncherActionShutdown) ? 0 : 4;
    }
    if (controlConfigAction == kConfigActionResetConfig) {
        return writeVisualConfig(dataRoot / L"config.toml",
                                 fcitx::windows::config::defaultConfigToml())
                   ? 0
                   : 5;
    }
    if (controlConfigAction == kConfigActionResetPresentation) {
        const fs::path configPath = dataRoot / L"config.toml";
        std::string source = "format_version = 1\n";
        if (readOptionalConfig(configPath, source) == OptionalConfigRead::error)
            return 5;
        std::string updated;
        ParseError error;
        if (!fcitx::windows::config::resetPresentationToml(source, updated, error)) {
            std::cerr << "invalid presentation reset at " << error.line << ':' << error.column
                      << ": " << error.message << '\n';
            return 3;
        }
        return writeVisualConfig(configPath, updated) ? 0 : 5;
    }
    if (controlConfigAction == kConfigActionValidate ||
        controlConfigAction == kConfigActionApply) {
        std::string text;
        ParseError error;
        if (!validateConfig(fs::path(arguments[1]), text, error)) {
            std::cerr << "invalid config at " << error.line << ':' << error.column << ": "
                      << error.message << '\n';
            return 3;
        }
        if (controlConfigAction == kConfigActionValidate)
            return 0;
        return writeVisualConfig(dataRoot / L"config.toml", text) ? 0 : 5;
    }
    usage();
    return 2;
}
