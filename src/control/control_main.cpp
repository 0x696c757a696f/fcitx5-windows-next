#include "config_model.h"
#include "activation_guard.h"
#include "deployment_core.h"
#include "fcitx5_windows/release_identity.h"
#include "fcitx5_windows/version.h"
#include "launcher_client.h"
#include "package_core.h"
#include "peer_verification.h"
#include "process_execution.h"
#include "protocol.h"
#include "runtime_identity.h"

#include <ShlObj.h>
#include <Windows.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <map>
#include <optional>
#include <set>
#include <span>
#include <string>
#include <string_view>
#include <utility>
#include <vector>
#include <filesystem>
#include <fstream>
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

struct Fcitx5ControlUtf16 {
    const wchar_t* ptr;
    std::size_t len;
};
struct Fcitx5ControlUtf8 {
    const char* ptr;
    std::size_t len;
};
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
int fcitx5_control_startup_query_utf16(Fcitx5ControlUtf16 executable_directory,
                                       Fcitx5ControlUtf16 registry_value,
                                       std::uint8_t* out_enabled);
int fcitx5_control_startup_set_utf16(Fcitx5ControlUtf16 executable_directory,
                                     Fcitx5ControlUtf16 registry_value,
                                     std::uint8_t enabled);
int fcitx5_control_schema_json_utf8(const char** out_ptr, std::size_t* out_len);
std::uint8_t fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 id);
int fcitx5_control_json_string_utf8(Fcitx5ControlUtf8 value, char** out_ptr,
                                    std::size_t* out_len);
int fcitx5_control_presentation_json_utf8(const Fcitx5ControlPresentation* presentation,
                                          char** out_ptr, std::size_t* out_len);
int fcitx5_control_status_json_utf8(const Fcitx5ControlStatus* status, char** out_ptr,
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
std::uint32_t fcitx5_control_config_file_action_utf16(Fcitx5ControlUtf16 command);
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
void fcitx5_control_utf8_free(char* ptr, std::size_t len);
}

namespace {

namespace fs = std::filesystem;
using fcitx::windows::config::Config;
using fcitx::windows::config::ParseError;

constexpr wchar_t kVisualConfigChangedMessage[] =
    L"Fcitx5WindowsNext.VisualConfigChanged.v1";
constexpr std::uint32_t kLauncherActionRestartEngine = 1;
constexpr std::uint32_t kLauncherActionShutdown = 2;
constexpr std::uint32_t kConfigFileActionValidate = 1;
constexpr std::uint32_t kConfigFileActionApply = 2;
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
    if (value.empty())
        return {};
    const int count =
        WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                            static_cast<int>(value.size()), nullptr, 0, nullptr, nullptr);
    if (count <= 0)
        return {};
    std::string result(static_cast<std::size_t>(count), '\0');
    if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                            static_cast<int>(value.size()), result.data(), count, nullptr,
                            nullptr) != count)
        return {};
    return result;
}

std::wstring widen(std::string_view value) {
    if (value.empty())
        return {};
    const int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
                                          static_cast<int>(value.size()), nullptr, 0);
    if (count <= 0)
        return {};
    std::wstring result(static_cast<std::size_t>(count), L'\0');
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
                               static_cast<int>(value.size()), result.data(), count) == count
               ? result
               : std::wstring{};
}

std::string jsonString(std::string_view value) {
    char* escaped = nullptr;
    std::size_t escapedLength = 0;
    if (fcitx5_control_json_string_utf8({value.data(), value.size()}, &escaped,
                                        &escapedLength) != 0) {
        return {};
    }
    std::string result;
    if (escaped && escapedLength > 0) {
        result.assign(escaped, escapedLength);
    }
    fcitx5_control_utf8_free(escaped, escapedLength);
    return result;
}

Fcitx5ControlUtf8 utf8View(std::string_view value) noexcept {
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

std::uint32_t configFileAction(std::wstring_view command) {
    return fcitx5_control_config_file_action_utf16({command.data(), command.size()});
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

bool readUtf8(const fs::path& path, std::string& text) {
    std::error_code error;
    const auto size = fs::file_size(path, error);
    if (error || size > 256U * 1024U)
        return false;
    std::ifstream stream(path, std::ios::binary);
    if (!stream)
        return false;
    text.assign(std::istreambuf_iterator<char>(stream), {});
    return stream.good() || stream.eof();
}

std::vector<std::byte> readBinary(const fs::path& path, std::size_t maximum) {
    std::error_code error;
    const auto size = fs::file_size(path, error);
    if (error || size > maximum)
        throw fcitx::package::PackageError("invalid_file", "file is missing or too large");
    std::ifstream stream(path, std::ios::binary);
    std::vector<std::byte> bytes(static_cast<std::size_t>(size));
    if (!bytes.empty())
        stream.read(reinterpret_cast<char*>(bytes.data()),
                    static_cast<std::streamsize>(bytes.size()));
    if (!stream)
        throw fcitx::package::PackageError("io_error", "file read failed");
    return bytes;
}

fs::path executableDirectory() {
    std::wstring path(32768, L'\0');
    const DWORD length = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    if (length == 0 || length >= path.size())
        return {};
    path.resize(length);
    return fs::path(path).parent_path();
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

// Anti-rollback state: the highest release_sequence ever accepted for this
// channel. A signed-but-stale repository index (lower sequences) is rejected
// so a compromised/old CDN cannot silently roll packages back. Only an
// explicit manual rollback path is allowed to move versions downwards.
fs::path repositorySequencePath(const fs::path& dataRoot, std::string_view channel) {
    return dataRoot / L"repository" /
           (L"sequence-" + widen(std::string(channel)) + L".json");
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

std::uint64_t indexMaxSequence(const fcitx::package::RepositoryIndex& repository) {
    std::uint64_t maximum = 0;
    for (const auto& entry : repository.packages)
        maximum = std::max(maximum, entry.release_sequence);
    return maximum;
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
    if (indexMaxSequence(repository) < readMaxSequence(dataRoot, repository.channel, true))
        throw fcitx::package::PackageError(
            "rollback_rejected",
            "cached repository index is older than the accepted release sequence");
    return repository;
}

void refreshRepository(const fs::path& dataRoot, std::wstring baseUrl) {
    while (!baseUrl.empty() && baseUrl.back() == L'/')
        baseUrl.pop_back();
    const auto files = repositoryFiles(dataRoot);
    fs::create_directories(files.index.parent_path());
    const auto incomingIndex = fs::path(files.index.wstring() + L".new");
    const auto incomingSignature = fs::path(files.signature.wstring() + L".new");
    std::error_code ignored;
    fs::remove(incomingIndex, ignored);
    fs::remove(incomingSignature, ignored);
    const auto downloader = executableDirectory() / L"fcitx5-downloader.exe";
    if (!runProcess(downloader, {L"--download-signed-metadata", baseUrl + L"/index.json",
                                 incomingIndex.wstring()}) ||
        !runProcess(downloader, {L"--download-signed-metadata", baseUrl + L"/index.sig",
                                 incomingSignature.wstring()})) {
        fs::remove(incomingIndex, ignored);
        fs::remove(incomingSignature, ignored);
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
    const auto maximum = indexMaxSequence(repository);
    const auto accepted = readMaxSequence(
        dataRoot, repository.channel,
        fs::exists(files.index) ||
            readSequenceState(dataRoot, repository.channel).present);
    if (maximum < accepted)
        throw fcitx::package::PackageError("rollback_rejected",
                                           "repository index is older than the accepted "
                                           "release sequence");
    if (!MoveFileExW(incomingSignature.c_str(), files.signature.c_str(),
                     MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) ||
        !MoveFileExW(incomingIndex.c_str(), files.index.c_str(),
                     MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
        throw fcitx::package::PackageError("io_error", "repository cache publication failed");
    }
    writeMaxSequence(dataRoot, repository.channel, maximum);
}

std::string repairRepositorySequenceState(
    const fs::path& dataRoot, std::span<const fcitx::package::TrustedKey> trustedKeys) {
    const auto files = repositoryFiles(dataRoot);
    const auto channel = fcitx::windows::kReleaseIdentity.channel_name;
    std::error_code ignored;
    if (!fs::exists(files.index) && !fs::exists(files.signature)) {
        fs::remove(repositorySequencePath(dataRoot, channel), ignored);
        return "reset";
    }
    try {
        std::string index;
        if (!readUtf8(files.index, index))
            throw fcitx::package::PackageError("repository_unavailable",
                                               "repository cache is unavailable");
        const auto signature = readBinary(files.signature, 16U * 1024U);
        const auto repository =
            fcitx::package::verify_repository_index(index, signature, trustedKeys, channel);
        writeMaxSequence(dataRoot, repository.channel, indexMaxSequence(repository));
        return "repaired";
    } catch (const fcitx::package::PackageError&) {
        fs::remove(files.index, ignored);
        fs::remove(files.signature, ignored);
        fs::remove(repositorySequencePath(dataRoot, channel), ignored);
        return "reset";
    }
}

std::string typeName(fcitx::package::PackageType type) {
    using fcitx::package::PackageType;
    switch (type) {
    case PackageType::core:
        return "core";
    case PackageType::addon:
        return "addon";
    case PackageType::input_method_data:
        return "inputmethod-data";
    case PackageType::theme:
        return "theme";
    case PackageType::translation:
        return "translation";
    }
    return "unknown";
}

std::string jsonDependencies(std::span<const fcitx::package::Dependency> dependencies) {
    std::string output = "[";
    bool first = true;
    for (const auto& dependency : dependencies) {
        if (!first)
            output += ',';
        first = false;
        output += "{\"id\":" + jsonString(dependency.id) +
                  ",\"version\":" + jsonString(dependency.version) + '}';
    }
    output += ']';
    return output;
}

std::string jsonStringArray(std::span<const std::string> values) {
    std::string output = "[";
    bool first = true;
    for (const auto& value : values) {
        if (!first)
            output += ',';
        first = false;
        output += jsonString(value);
    }
    output += ']';
    return output;
}

std::string installedManifestBytes(const fs::path& packageRoot,
                                   const fcitx::package::LockEntry& entry) {
    const auto path = packageRoot / L"manifests" / widen(entry.id) /
                      widen(entry.version + ".json");
    std::error_code error;
    const auto size = fs::file_size(path, error);
    if (error || size > fcitx::package::kMaximumManifestBytes)
        return {};
    std::ifstream input(path, std::ios::binary);
    if (!input)
        return {};
    return {std::istreambuf_iterator<char>(input), {}};
}

std::string configSurfaceJson(const fcitx::package::Manifest* manifest,
                              fcitx::package::PackageType type,
                              std::string_view packageId) {
    std::set<std::string> surfaces;
    if (type == fcitx::package::PackageType::theme)
        surfaces.emplace("theme");
    if (type == fcitx::package::PackageType::input_method_data)
        surfaces.emplace("input-method-data");
    if (type == fcitx::package::PackageType::addon)
        surfaces.emplace("fcitx-addon");
    if (manifest) {
        if (std::ranges::find(manifest->permissions, "input-data") !=
            manifest->permissions.end())
            surfaces.emplace("input-method-data");
        for (const auto& file : manifest->files) {
            if (file.path.starts_with("share/fcitx5/addon/") && file.path.ends_with(".conf"))
                surfaces.emplace("fcitx-addon-config");
            if (file.path.starts_with("lib/fcitx5/") && file.path.ends_with(".dll"))
                surfaces.emplace("fcitx-addon");
            if (file.path.starts_with("share/rime-data/"))
                surfaces.emplace("rime-data");
            if (file.path.starts_with("themes/") || file.path.starts_with("share/themes/"))
                surfaces.emplace("theme");
        }
    }
    std::string output = "[";
    bool first = true;
    for (const auto& surface : surfaces) {
        if (!first)
            output += ',';
        first = false;
        output += "{\"kind\":" + jsonString(surface) +
                  ",\"owner\":" + jsonString(packageId) +
                  ",\"schema\":\"generic-fcitx-config-v1\"}";
    }
    output += ']';
    return output;
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

bool parseBool(std::string_view value) {
    return value == "True" || value == "true" || value == "1";
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
            inAddon = line == "[Addon]";
            continue;
        }
        if (!inAddon)
            continue;
        const std::size_t separator = line.find('=');
        if (separator == std::string_view::npos)
            continue;
        const auto key = trimAscii(line.substr(0, separator));
        const auto value = trimAscii(line.substr(separator + 1));
        if (key == "Name")
            descriptor.name = std::string(value);
        else if (key == "Category")
            descriptor.category = std::string(value);
        else if (key == "Library")
            descriptor.library = std::string(value);
        else if (key == "Type")
            descriptor.type = std::string(value);
        else if (key == "Version")
            descriptor.version = std::string(value);
        else if (key == "Configurable")
            descriptor.configurable = parseBool(value);
        else if (key == "OnDemand")
            descriptor.onDemand = parseBool(value);
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
    std::wstring modulePath(32768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, modulePath.data(), static_cast<DWORD>(modulePath.size()));
    if (size > 0 && size < modulePath.size()) {
        modulePath.resize(size);
        if (const auto portableData =
                fcitx::windows::platform::portableDataRootForModule(modulePath);
            !portableData.empty()) {
            return portableData;
        }
    }
    PWSTR localAppData = nullptr;
    if (FAILED(SHGetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_CREATE, nullptr, &localAppData)))
        return {};
    fs::path result(localAppData);
    CoTaskMemFree(localAppData);
    return result / fcitx::windows::kReleaseIdentity.data_directory;
}

bool validateConfig(const fs::path& source, std::string& text, ParseError& parseError) {
    if (!readUtf8(source, text))
        return false;
    Config config;
    return fcitx::windows::config::parseConfig(text, config, parseError);
}

bool atomicWrite(const fs::path& destination, std::string_view text) {
    std::error_code error;
    fs::create_directories(destination.parent_path(), error);
    if (error)
        return false;
    GUID identifier{};
    std::array<wchar_t, 40> identifierText{};
    if (FAILED(CoCreateGuid(&identifier)) ||
        StringFromGUID2(identifier, identifierText.data(),
                        static_cast<int>(identifierText.size())) == 0)
        return false;
    const fs::path temporary = destination.wstring() + L"." + identifierText.data() + L".tmp";
    HANDLE file = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                              FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH, nullptr);
    if (file == INVALID_HANDLE_VALUE)
        return false;
    DWORD written = 0;
    const bool writeOk =
        text.size() <= MAXDWORD &&
        WriteFile(file, text.data(), static_cast<DWORD>(text.size()), &written, nullptr) &&
        written == text.size() && FlushFileBuffers(file);
    CloseHandle(file);
    if (!writeOk) {
        DeleteFileW(temporary.c_str());
        return false;
    }
    if (!MoveFileExW(temporary.c_str(), destination.c_str(),
                     MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
        DeleteFileW(temporary.c_str());
        return false;
    }
    return true;
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
    return fcitx::windows::ipc::sendLauncherCommand(identity, GetTickCount64() + 1000, policy,
                                                    command, response);
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
#if defined(_WIN64)
    return "x64";
#else
    return "x86";
#endif
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
                       item.state != "disabled" && item.state != "pending_remove" &&
                       item.state != "broken" && item.state != "quarantined";
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
               item.state != "pending_remove";
    });
    if (same != current.end())
        return;

    const auto downloads = dataRoot / L"downloads";
    fs::create_directories(downloads);
    const auto archive = downloads / widen(entry->id + "-" + entry->version + ".fcpkg");
    bool validCache = false;
    if (fs::exists(archive)) {
        validCache =
            fcitx::package::hex_sha256(fcitx::package::sha256_file(archive)) == entry->sha256;
        if (!validCache) {
            std::error_code ignored;
            fs::remove(archive, ignored);
        }
    }
    if (!validCache && !runProcess(executableDirectory() / L"fcitx5-downloader.exe",
                                   {L"--download", widen(entry->download_url), widen(entry->sha256),
                                    archive.wstring()})) {
        throw fcitx::package::PackageError("network_error", "package download failed");
    }
    const auto keys = fcitx::package::read_trusted_keys(repositoryFiles(dataRoot).keyring);
    const std::string transaction = "pkg-" + entry->sha256.substr(0U, 24U);
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

void printPackages(const fs::path& dataRoot) {
    const auto root = dataRoot / L"packages";
    const auto installed = fcitx::package::read_lockfile(root);
    std::map<std::string, fcitx::package::LockEntry, std::less<>> active;
    for (const auto& entry : installed)
        active.emplace(entry.id, entry);
    struct BundledComponent {
        const char* id;
        const char* title;
        fs::path probe;
    };
    const fs::path installRoot = installationRoot();
    const std::array bundledCandidates{
        BundledComponent{"fcitx5-chinese-addons", "Fcitx5 Chinese Addons",
                         installRoot / L"lib/fcitx5/libpinyin.dll"},
        BundledComponent{"fcitx5-rime", "Rime",
                         installRoot / L"lib/fcitx5/librime.dll"},
        BundledComponent{"fcitx5-lua", "Fcitx5 Lua",
                         installRoot / L"lib/fcitx5/libluaaddonloader.dll"},
        BundledComponent{"fcitx5-chttrans", "Simplified / Traditional Conversion",
                         installRoot / L"lib/fcitx5/libchttrans.dll"},
        BundledComponent{"librime-lua", "Rime Lua", installRoot / L"bin/lua54.dll"},
    };
    std::map<std::string, BundledComponent, std::less<>> bundled;
    for (const auto& component : bundledCandidates) {
        if (fs::is_regular_file(component.probe))
            bundled.emplace(component.id, component);
    }
    fcitx::package::RepositoryIndex repository;
    bool repositoryAvailable = false;
    std::string repositoryError;
    try {
        repository = loadRepository(dataRoot);
        repositoryAvailable = true;
    } catch (const fcitx::package::PackageError& error) {
        repositoryError = error.code();
        if (repositoryError == "invalid_file" && !fs::exists(repositoryFiles(dataRoot).keyring))
            repositoryError = "missing_key";
    }
    std::cout << "{\"format_version\":1,\"repository_available\":"
              << (repositoryAvailable ? "true" : "false")
              << ",\"repository_error\":"
              << (repositoryError.empty() ? "null" : jsonString(repositoryError))
              << ",\"packages\":[";
    bool first = true;
    std::set<std::string> emitted;
    if (repositoryAvailable) {
        for (const auto& entry : repository.packages) {
            if (entry.architecture != "any" && entry.architecture != nativeArchitecture())
                continue;
            if (!first)
                std::cout << ',';
            first = false;
            const auto found = active.find(entry.id);
            const bool bundledNow = bundled.contains(entry.id);
            const bool update = found != active.end() && found->second.version != entry.version;
            std::cout << "{\"id\":" << jsonString(entry.id)
                      << ",\"title\":" << jsonString(entry.title)
                      << ",\"summary\":" << jsonString(entry.summary)
                      << ",\"type\":" << jsonString(typeName(entry.type))
                      << ",\"available_version\":" << jsonString(entry.version)
                      << ",\"installed_version\":"
                      << (found != active.end()
                              ? jsonString(found->second.version)
                              : (bundledNow ? jsonString(std::string(fcitx::windows::version()))
                                            : "null"))
                      << ",\"state\":"
                      << (found != active.end()
                              ? jsonString(found->second.state)
                              : (bundledNow ? "\"bundled\"" : "null"))
                      << ",\"update_available\":" << (update ? "true" : "false") << '}';
            emitted.emplace(entry.id);
        }
    }
    for (const auto& entry : installed) {
        if (emitted.contains(entry.id))
            continue;
        if (!first)
            std::cout << ',';
        first = false;
        std::cout << "{\"id\":" << jsonString(entry.id) << ",\"title\":" << jsonString(entry.id)
                  << ",\"summary\":\"\",\"type\":\"unknown\","
                     "\"available_version\":null,\"installed_version\":"
                  << jsonString(entry.version) << ",\"state\":" << jsonString(entry.state)
                  << ",\"update_available\":false}";
    }
    for (const auto& [id, component] : bundled) {
        if (emitted.contains(id) || active.contains(id))
            continue;
        if (!first)
            std::cout << ',';
        first = false;
        std::cout << "{\"id\":" << jsonString(id)
                  << ",\"title\":" << jsonString(component.title)
                  << ",\"summary\":\"Bundled with Fcitx5 for Windows Next\","
                     "\"type\":\"addon\",\"available_version\":null,"
                     "\"installed_version\":"
                  << jsonString(std::string(fcitx::windows::version()))
                  << ",\"state\":\"bundled\",\"update_available\":false}";
    }
    std::cout << "]}\n";
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
        repositoryError = error.code();
        if (repositoryError == "invalid_file" && !fs::exists(repositoryFiles(dataRoot).keyring))
            repositoryError = "missing_key";
    }

    const fs::path installRoot = installationRoot();
    const std::map<std::string, fs::path, std::less<>> bundledProbes{
        {"fcitx5-chinese-addons", installRoot / L"lib/fcitx5/libpinyin.dll"},
        {"fcitx5-rime", installRoot / L"lib/fcitx5/librime.dll"},
        {"fcitx5-lua", installRoot / L"lib/fcitx5/libluaaddonloader.dll"},
        {"fcitx5-chttrans", installRoot / L"lib/fcitx5/libchttrans.dll"},
        {"librime-lua", installRoot / L"bin/lua54.dll"},
    };
    const auto bundled = bundledProbes.find(packageId);
    const bool bundledNow = bundled != bundledProbes.end() && fs::is_regular_file(bundled->second);
    if (active == installed.end() && !repositoryEntry && !bundledNow)
        throw fcitx::package::PackageError("package_not_found", "package is unknown");

    const std::string type = manifest ? typeName(manifest->type)
                             : repositoryEntry ? typeName(repositoryEntry->type)
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
    const bool update = active != installed.end() && repositoryEntry &&
                        active->version != repositoryEntry->version;
    const auto typeValue = manifest ? manifest->type
                                    : (repositoryEntry ? repositoryEntry->type
                                                       : fcitx::package::PackageType::addon);
    std::cout << "{\"format_version\":1,\"repository_available\":"
              << (repositoryAvailable ? "true" : "false")
              << ",\"repository_error\":"
              << (repositoryError.empty() ? "null" : jsonString(repositoryError))
              << ",\"id\":" << jsonString(packageId)
              << ",\"title\":" << jsonString(title)
              << ",\"summary\":" << jsonString(summary)
              << ",\"type\":" << jsonString(type)
              << ",\"available_version\":"
              << (available.empty() ? "null" : jsonString(available))
              << ",\"installed_version\":"
              << (installedVersion.empty() ? "null" : jsonString(installedVersion))
              << ",\"state\":" << (state.empty() ? "null" : jsonString(state))
              << ",\"bundled\":" << (bundledNow ? "true" : "false")
              << ",\"update_available\":" << (update ? "true" : "false")
              << ",\"manifest_sha256\":"
              << (active != installed.end() ? jsonString(active->manifest_sha256) : "null")
              << ",\"source_commit\":"
              << (manifest ? jsonString(manifest->source_commit) : "null")
              << ",\"dependencies\":"
              << (manifest ? jsonDependencies(manifest->dependencies)
                           : (repositoryEntry ? jsonDependencies(repositoryEntry->dependencies)
                                              : "[]"))
              << ",\"permissions\":"
              << (manifest ? jsonStringArray(manifest->permissions) : "[]")
              << ",\"config_surface\":"
              << configSurfaceJson(manifest ? &*manifest : nullptr, typeValue, packageId)
              << "}\n";
}

Fcitx5ControlUtf16 nativeView(std::wstring_view value) noexcept {
    return {value.data(), value.size()};
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
    std::wcerr << L"Usage: fcitx5-control [--data-root PATH] "
                  L"--status|--restart-engine|--validate-config FILE|--apply-config FILE|"
                  L"--reset-config|--reset-presentation|--get-startup|--set-startup enabled|disabled|"
                  L"--get-presentation|"
                  L"--get-input-methods|--set-input-method ID|--shutdown|"
                  L"--set-presentation MODE THEME ORIENTATION SCROLL PAGE_SIZE FONT "
                  L"[MAX_WIDTH_DIP SCROLL_CELL_WIDTH_DIP "
                  L"FONT_SIZE_DIP CORNER_RADIUS_DIP SHADOW OPACITY PREEDIT_MODE]|"
                  L"--themes-list|--themes-detail ID|"
                  L"--addons-list|"
                  L"--packages-list|--packages-detail ID|--packages-refresh [HTTPS_BASE]|"
                  L"--packages-install ID|--packages-update ID|"
                  L"--packages-state ID enabled|disabled|--packages-remove ID|"
                  L"--packages-repair|--get-tsf-guard|--reset-tsf-guard|--schema|--version\n";
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
    if (arguments.size() == 1 && arguments[0] == L"--version") {
        std::cout << fcitx::windows::version() << '\n';
        return 0;
    }
    if (arguments.empty()) {
        usage();
        return 2;
    }
    if (arguments.size() == 1 && arguments[0] == L"--schema") {
        return printControlSchema() ? 0 : 5;
    }
    if (arguments.size() == 1 && arguments[0] == L"--get-startup") {
        bool enabled = false;
        if (!queryStartup(enabled))
            return 5;
        std::cout << startupJson(enabled) << '\n';
        return 0;
    }
    if (arguments.size() == 2 && arguments[0] == L"--set-startup" &&
        (arguments[1] == L"enabled" || arguments[1] == L"disabled")) {
        return setStartup(arguments[1] == L"enabled") ? 0 : 5;
    }
    if (dataRoot.empty()) {
        std::cerr << "unable to resolve the user data directory\n";
        return 5;
    }
    if (arguments.size() == 1 && arguments[0] == L"--get-tsf-guard") {
        const auto status = fcitx::windows::tsf::activationGuardStatus(dataRoot);
        const std::string markerPath = narrow(status.markerPath.wstring());
        const Fcitx5ControlTsfGuard guardStatus{
            status.disabled ? std::uint8_t{1} : std::uint8_t{0}, utf8View(status.reason),
            utf8View(markerPath)};
        std::cout << tsfGuardJson(guardStatus) << '\n';
        return 0;
    }
    if (arguments.size() == 1 && arguments[0] == L"--reset-tsf-guard") {
        if (!fcitx::windows::tsf::clearActivationGuard(dataRoot)) {
            std::cerr << "unable to clear TSF activation guard\n";
            return 5;
        }
        std::cout << tsfGuardResetJson() << '\n';
        return 0;
    }
    try {
        if (arguments.size() == 1 && arguments[0] == L"--get-input-methods") {
            std::string output;
            if (!runEngineManagement({L"--list-input-methods"}, output))
                return 4;
            std::cout << output;
            return 0;
        }
        if (arguments.size() == 2 && arguments[0] == L"--set-input-method") {
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
                widen(std::string("https://packages.fcitx5-windows.org/v1/") +
                      std::string(fcitx::windows::kReleaseIdentity.channel_name));
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
    if (arguments.size() == 1 && arguments[0] == L"--get-presentation") {
        const fs::path configPath = dataRoot / L"config.toml";
        std::string text;
        if (fs::exists(configPath) && !readUtf8(configPath, text))
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
    if ((arguments.size() == 7 || arguments.size() == 9 || arguments.size() == 12 ||
         arguments.size() == 14) &&
        arguments[0] == L"--set-presentation") {
        const fs::path configPath = dataRoot / L"config.toml";
        std::string source = fcitx::windows::config::defaultConfigToml();
        if (fs::exists(configPath) && !readUtf8(configPath, source))
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
    if (arguments.size() == 1 && arguments[0] == L"--status") {
        fcitx::windows::protocol::LauncherResponse response;
        const bool reachable =
            launcherCommand(fcitx::windows::protocol::LauncherCommand::status, response);
        const fs::path configPath = dataRoot / L"config.toml";
        bool configValid = true;
        if (fs::exists(configPath)) {
            std::string text;
            ParseError error;
            configValid = validateConfig(configPath, text, error);
        }
        const auto tsfGuard = fcitx::windows::tsf::activationGuardStatus(dataRoot);
        const std::string dataRootUtf8 = narrow(dataRoot.generic_wstring());
        const std::string updateOwner{
            fcitx::update::owner_name(fcitx::update::read_update_owner(installationRoot()))};
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
        std::cout << statusJson(status) << '\n';
        return configValid ? 0 : 3;
    }
    if (arguments.size() == 1 && arguments[0] == L"--restart-engine") {
        if (!runLauncherAction(kLauncherActionRestartEngine)) {
            std::cerr << "launcher unavailable or restart rejected\n";
            return 4;
        }
        return 0;
    }
    if (arguments.size() == 1 && arguments[0] == L"--shutdown") {
        return runLauncherAction(kLauncherActionShutdown) ? 0 : 4;
    }
    if (arguments.size() == 1 && arguments[0] == L"--reset-config") {
        return writeVisualConfig(dataRoot / L"config.toml",
                                 fcitx::windows::config::defaultConfigToml())
                   ? 0
                   : 5;
    }
    if (arguments.size() == 1 && arguments[0] == L"--reset-presentation") {
        const fs::path configPath = dataRoot / L"config.toml";
        std::string source = "format_version = 1\n";
        if (fs::exists(configPath) && !readUtf8(configPath, source))
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
    const std::uint32_t configAction =
        arguments.empty() ? 0 : configFileAction(arguments[0]);
    if (arguments.size() == 2 &&
        (configAction == kConfigFileActionValidate || configAction == kConfigFileActionApply)) {
        std::string text;
        ParseError error;
        if (!validateConfig(fs::path(arguments[1]), text, error)) {
            std::cerr << "invalid config at " << error.line << ':' << error.column << ": "
                      << error.message << '\n';
            return 3;
        }
        if (configAction == kConfigFileActionValidate)
            return 0;
        return writeVisualConfig(dataRoot / L"config.toml", text) ? 0 : 5;
    }
    usage();
    return 2;
}
