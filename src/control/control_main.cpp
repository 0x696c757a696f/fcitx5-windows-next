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

namespace {

namespace fs = std::filesystem;
using fcitx::windows::config::Config;
using fcitx::windows::config::ParseError;

constexpr wchar_t kVisualConfigChangedMessage[] =
    L"Fcitx5WindowsNext.VisualConfigChanged.v1";

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
    std::string result = "\"";
    for (const unsigned char character : value) {
        switch (character) {
        case '\\':
            result += "\\\\";
            break;
        case '"':
            result += "\\\"";
            break;
        case '\b':
            result += "\\b";
            break;
        case '\f':
            result += "\\f";
            break;
        case '\n':
            result += "\\n";
            break;
        case '\r':
            result += "\\r";
            break;
        case '\t':
            result += "\\t";
            break;
        default:
            if (character < 0x20U)
                return {};
            result.push_back(static_cast<char>(character));
        }
    }
    result.push_back('"');
    return result;
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

std::wstring quoteArgument(std::wstring_view value) {
    std::wstring result = L"\"";
    unsigned slashes = 0;
    for (const auto character : value) {
        if (character == L'\\') {
            ++slashes;
            continue;
        }
        if (character == L'\"')
            result.append(slashes + 1U, L'\\');
        else
            result.append(slashes, L'\\');
        slashes = 0;
        result.push_back(character);
    }
    result.append(slashes * 2U, L'\\');
    result.push_back(L'\"');
    return result;
}

bool runProcess(const fs::path& executable, const std::vector<std::wstring>& arguments,
                DWORD timeout = 120000U) {
    std::wstring command = quoteArgument(executable.wstring());
    for (const auto& argument : arguments)
        command += L" " + quoteArgument(argument);
    STARTUPINFOW startup{sizeof(startup)};
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, executable.parent_path().c_str(), &startup,
                        &process))
        return false;
    const DWORD wait = WaitForSingleObject(process.hProcess, timeout);
    if (wait == WAIT_TIMEOUT)
        TerminateProcess(process.hProcess, ERROR_TIMEOUT);
    DWORD code = 1;
    GetExitCodeProcess(process.hProcess, &code);
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    return wait == WAIT_OBJECT_0 && code == 0;
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

std::optional<std::string_view> lineValue(std::string_view text, std::string_view key) {
    const std::string marker = std::string(key) + "=";
    std::size_t position = 0;
    while (position <= text.size()) {
        const std::size_t lineEnd = text.find('\n', position);
        std::string_view line =
            lineEnd == std::string_view::npos
                ? text.substr(position)
                : text.substr(position, lineEnd - position);
        if (!line.empty() && line.back() == '\r')
            line.remove_suffix(1);
        if (line.starts_with(marker))
            return line.substr(marker.size());
        if (lineEnd == std::string_view::npos)
            break;
        position = lineEnd + 1;
    }
    return std::nullopt;
}

bool parseUnsigned(std::string_view value, std::uint64_t& output) {
    if (value.empty() ||
        !std::all_of(value.begin(), value.end(),
                     [](char character) { return character >= '0' && character <= '9'; }))
        return false;
    try {
        output = std::stoull(std::string(value));
        return true;
    } catch (...) {
        return false;
    }
}

SequenceState readSequenceState(const fs::path& dataRoot, std::string_view channel) {
    std::string text;
    if (!readUtf8(repositorySequencePath(dataRoot, channel), text))
        return {};
    std::uint64_t maximum = 0;
    const auto format = lineValue(text, "format_version");
    const auto storedChannel = lineValue(text, "channel");
    const auto storedMaximum = lineValue(text, "max_release_sequence");
    if (!format || *format != "1" || !storedChannel || *storedChannel != channel ||
        !storedMaximum || !parseUnsigned(*storedMaximum, maximum)) {
        return {.present = true, .valid = false, .maximum = 0};
    }
    return {.present = true, .valid = true, .maximum = maximum};
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
    const auto path = repositorySequencePath(dataRoot, channel);
    const auto incoming = fs::path(path.wstring() + L".new");
    std::error_code ignored;
    fs::create_directories(path.parent_path(), ignored);
    fs::remove(incoming, ignored);
    std::ofstream output(incoming, std::ios::binary | std::ios::trunc);
    output << "format_version=1\n"
           << "channel=" << channel << '\n'
           << "max_release_sequence=" << maximum << '\n';
    output.close();
    if (!output || !MoveFileExW(incoming.c_str(), path.c_str(),
                                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
        fs::remove(incoming, ignored);
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

void printAddons() {
    const auto addons = listAddonDescriptors();
    std::cout << "{\"format_version\":1,\"surface\":\"descriptor-inventory\","
                 "\"typed_config\":\"not_available\",\"addons\":[";
    bool first = true;
    for (const auto& addon : addons) {
        if (!first)
            std::cout << ',';
        first = false;
        std::cout << "{\"id\":" << jsonString(addon.id)
                  << ",\"name\":" << jsonString(addon.name)
                  << ",\"category\":" << jsonString(addon.category)
                  << ",\"library\":" << jsonString(addon.library)
                  << ",\"type\":" << jsonString(addon.type)
                  << ",\"version\":"
                  << (addon.version.empty() ? "null" : jsonString(addon.version))
                  << ",\"configurable\":" << (addon.configurable ? "true" : "false")
                  << ",\"on_demand\":" << (addon.onDemand ? "true" : "false")
                  << ",\"library_present\":" << (addon.libraryPresent ? "true" : "false")
                  << '}';
    }
    std::cout << "]}\n";
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

std::string themeRecordJson(const ThemeRecord& theme) {
    return "{\"id\":" + jsonString(theme.id) + ",\"source\":" + jsonString(theme.source) +
           ",\"name\":" + jsonString(theme.name) + ",\"version\":" +
           jsonString(theme.version) + ",\"license\":" + jsonString(theme.license) +
           ",\"description\":" + jsonString(theme.description) + '}';
}

std::string themeEditableFieldsJson() {
    static constexpr std::array fields{
        "appearance.mode",
        "candidate.orientation",
        "candidate.page_size",
        "candidate.scroll_mode",
        "candidate.max_width_dip",
        "candidate.scroll_cell_width_dip",
        "candidate.opacity",
        "candidate.geometry.padding_x_dip",
        "candidate.geometry.padding_y_dip",
        "candidate.geometry.item_padding_x_dip",
        "candidate.geometry.item_padding_y_dip",
        "candidate.geometry.row_gap_dip",
        "candidate.geometry.column_gap_dip",
        "candidate.geometry.border_width_dip",
        "candidate.geometry.corner_radius_dip",
        "candidate.geometry.shadow",
        "candidate.label.visible",
        "candidate.label.style",
        "candidate.label.font_scale",
        "candidate.label.gap_dip",
        "fonts.candidate.families",
        "fonts.candidate.size_dip",
        "fonts.candidate.weight",
        "fonts.annotation.scale",
        "candidate.colors.background",
        "candidate.colors.border",
        "candidate.colors.candidate_text",
        "candidate.colors.label_text",
        "candidate.colors.comment_text",
        "candidate.colors.selected_background",
        "candidate.colors.selected_candidate_text",
        "candidate.colors.selected_label_text",
        "candidate.colors.selected_comment_text",
    };
    std::string output = "[";
    bool first = true;
    for (const auto field : fields) {
        if (!first)
            output += ',';
        first = false;
        output += jsonString(field);
    }
    output += ']';
    return output;
}

void printThemes(const fs::path& dataRoot) {
    const auto themes = listThemes(dataRoot);
    std::cout << "{\"format_version\":1,\"themes\":[";
    bool first = true;
    for (const auto& theme : themes) {
        if (!first)
            std::cout << ',';
        first = false;
        std::cout << themeRecordJson(theme);
    }
    std::cout << "]}\n";
}

void printThemeDetail(const fs::path& dataRoot, std::string_view id) {
    if (id != "builtin:default" && !fcitx::package::is_lower_package_id(id))
        throw fcitx::package::PackageError("invalid_theme", "theme id is invalid");
    const auto themes = listThemes(dataRoot);
    const auto* theme = findTheme(themes, id);
    if (!theme)
        throw fcitx::package::PackageError("theme_not_found", "theme is unknown");
    std::cout << "{\"format_version\":1,\"theme\":" << themeRecordJson(*theme)
              << ",\"has_light_branch\":"
              << (!theme->theme.light.colors.empty() ? "true" : "false")
              << ",\"has_dark_branch\":"
              << (!theme->theme.dark.colors.empty() ? "true" : "false")
              << ",\"editable_fields\":" << themeEditableFieldsJson()
              << ",\"security\":{\"script_allowed\":false,\"network_allowed\":false,"
                 "\"unknown_fields\":\"reject\",\"path_scope\":\"theme-directory\"}}\n";
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
    try {
        repository = loadRepository(dataRoot);
        repositoryAvailable = true;
    } catch (const fcitx::package::PackageError&) {
    }
    std::cout << "{\"format_version\":1,\"repository_available\":"
              << (repositoryAvailable ? "true" : "false") << ",\"packages\":[";
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
                  << ",\"summary\":\"Bundled with Fcitx5 for Windows\","
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
    try {
        repository = loadRepository(dataRoot);
        repositoryAvailable = true;
        repositoryEntry = fcitx::package::find_repository_package(repository, packageId,
                                                                  nativeArchitecture());
    } catch (const fcitx::package::PackageError&) {
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
                                : bundledNow ? "Bundled with Fcitx5 for Windows"
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

std::wstring startupCommand() {
    const fs::path launcher = executableDirectory() / L"fcitx5-launcher.exe";
    return L"\"" + launcher.wstring() + L"\" --background";
}

bool queryStartup(bool& enabled) {
    enabled = false;
    HKEY key = nullptr;
    constexpr wchar_t path[] = L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    if (RegOpenKeyExW(HKEY_CURRENT_USER, path, 0, KEY_QUERY_VALUE, &key) != ERROR_SUCCESS)
        return true;
    DWORD type = 0;
    DWORD bytes = 0;
    const LSTATUS sizeResult = RegQueryValueExW(
        key, fcitx::windows::kReleaseIdentity.registry_value, nullptr, &type, nullptr, &bytes);
    if (sizeResult == ERROR_FILE_NOT_FOUND) {
        RegCloseKey(key);
        return true;
    }
    if (sizeResult != ERROR_SUCCESS || type != REG_SZ || bytes < sizeof(wchar_t) ||
        bytes > 64U * 1024U) {
        RegCloseKey(key);
        return false;
    }
    std::wstring value(bytes / sizeof(wchar_t), L'\0');
    const LSTATUS readResult =
        RegQueryValueExW(key, fcitx::windows::kReleaseIdentity.registry_value, nullptr, &type,
                         reinterpret_cast<BYTE*>(value.data()), &bytes);
    RegCloseKey(key);
    while (!value.empty() && value.back() == L'\0')
        value.pop_back();
    enabled = readResult == ERROR_SUCCESS && value == startupCommand();
    return readResult == ERROR_SUCCESS;
}

bool setStartup(bool enabled) {
    constexpr wchar_t path[] = L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    HKEY key = nullptr;
    if (RegCreateKeyExW(HKEY_CURRENT_USER, path, 0, nullptr, 0, KEY_SET_VALUE, nullptr, &key,
                        nullptr) != ERROR_SUCCESS)
        return false;
    LSTATUS result = ERROR_SUCCESS;
    if (enabled) {
        const std::wstring command = startupCommand();
        result = RegSetValueExW(key, fcitx::windows::kReleaseIdentity.registry_value, 0, REG_SZ,
                                reinterpret_cast<const BYTE*>(command.c_str()),
                                static_cast<DWORD>((command.size() + 1) * sizeof(wchar_t)));
    } else {
        result = RegDeleteValueW(key, fcitx::windows::kReleaseIdentity.registry_value);
        if (result == ERROR_FILE_NOT_FOUND)
            result = ERROR_SUCCESS;
    }
    RegCloseKey(key);
    return result == ERROR_SUCCESS;
}

void usage() {
    std::wcerr << L"Usage: fcitx5-control [--data-root PATH] "
                  L"--status|--restart-engine|--validate-config FILE|--apply-config FILE|"
                  L"--reset-config|--get-startup|--set-startup enabled|disabled|"
                  L"--get-presentation|"
                  L"--get-input-methods|--set-input-method ID|--shutdown|"
                  L"--set-presentation MODE THEME ORIENTATION SCROLL PAGE_SIZE FONT "
                  L"[MAX_WIDTH_DIP SCROLL_CELL_WIDTH_DIP "
                  L"FONT_SIZE_DIP CORNER_RADIUS_DIP SHADOW]|"
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
        std::cout
            << R"({"format_version":1,"commands":["status","restart_engine","shutdown","validate_config","apply_config","reset_config","get_startup","set_startup","get_presentation","set_presentation","get_input_methods","set_input_method","themes_list","themes_detail","addons_list","packages_list","packages_detail","packages_refresh","packages_install","packages_update","packages_state","packages_remove","packages_repair","get_tsf_guard","reset_tsf_guard"],"sensitive_input":false,"package_network_owner":"fcitx5-downloader.exe"})"
            << '\n';
        return 0;
    }
    if (arguments.size() == 1 && arguments[0] == L"--get-startup") {
        bool enabled = false;
        if (!queryStartup(enabled))
            return 5;
        std::cout << "{\"format_version\":1,\"enabled\":" << (enabled ? "true" : "false") << "}\n";
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
        std::cout << "{\"format_version\":1,\"disabled\":"
                  << (status.disabled ? "true" : "false")
                  << ",\"reason\":" << jsonString(status.reason)
                  << ",\"marker_path\":" << jsonString(narrow(status.markerPath.wstring()))
                  << "}\n";
        return 0;
    }
    if (arguments.size() == 1 && arguments[0] == L"--reset-tsf-guard") {
        if (!fcitx::windows::tsf::clearActivationGuard(dataRoot)) {
            std::cerr << "unable to clear TSF activation guard\n";
            return 5;
        }
        std::cout << "{\"format_version\":1,\"tsf_guard\":\"enabled\"}\n";
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
            if (id.empty() || id.size() > 64U ||
                !std::ranges::all_of(id, [](unsigned char value) {
                    return (value >= 'a' && value <= 'z') ||
                           (value >= '0' && value <= '9') || value == '-' || value == '_';
                }))
                return 2;
            std::string ignored;
            return runEngineManagement({L"--set-input-method", std::wstring(arguments[1])},
                                       ignored)
                       ? 0
                       : 4;
        }
        if (arguments.size() == 1 && arguments[0] == L"--packages-list") {
            printPackages(dataRoot);
            return 0;
        }
        if (arguments.size() == 1 && arguments[0] == L"--themes-list") {
            printThemes(dataRoot);
            return 0;
        }
        if (arguments.size() == 2 && arguments[0] == L"--themes-detail") {
            printThemeDetail(dataRoot, narrow(arguments[1]));
            return 0;
        }
        if (arguments.size() == 1 && arguments[0] == L"--addons-list") {
            printAddons();
            return 0;
        }
        if (arguments.size() == 2 && arguments[0] == L"--packages-detail") {
            printPackageDetail(dataRoot, narrow(arguments[1]));
            return 0;
        }
        if ((arguments.size() == 1 || arguments.size() == 2) &&
            arguments[0] == L"--packages-refresh") {
            const auto defaultBase =
                widen(std::string("https://packages.fcitx5-windows.org/v1/") +
                      std::string(fcitx::windows::kReleaseIdentity.channel_name));
            refreshRepository(dataRoot,
                              arguments.size() == 2 ? std::wstring(arguments[1]) : defaultBase);
            printPackages(dataRoot);
            return 0;
        }
        if (arguments.size() == 2 &&
            (arguments[0] == L"--packages-install" || arguments[0] == L"--packages-update")) {
            const auto repository = loadRepository(dataRoot);
            std::set<std::string> visiting;
            installRepositoryPackage(dataRoot, repository, narrow(arguments[1]), visiting);
            requestEngineReload();
            printPackages(dataRoot);
            return 0;
        }
        if (arguments.size() == 3 && arguments[0] == L"--packages-state" &&
            (arguments[2] == L"enabled" || arguments[2] == L"disabled")) {
            fcitx::package::set_package_state(dataRoot / L"packages", narrow(arguments[1]),
                                              narrow(arguments[2]));
            requestEngineReload();
            return 0;
        }
        if (arguments.size() == 2 && arguments[0] == L"--packages-remove") {
            const auto id = narrow(arguments[1]);
            fcitx::package::mark_package_for_removal(dataRoot / L"packages", id);
            requestEngineReload();
            fcitx::package::finalize_package_removal(dataRoot / L"packages", id);
            printPackages(dataRoot);
            return 0;
        }
        if (arguments.size() == 1 && arguments[0] == L"--packages-repair") {
            fcitx::package::verify_installed_packages(
                dataRoot / L"packages",
                fcitx::package::read_trusted_keys(repositoryFiles(dataRoot).keyring));
            std::cout << "{\"format_version\":1,\"repair\":\"verified\"}\n";
            return 0;
        }
    } catch (const fcitx::package::PackageError& error) {
        std::cerr << error.code() << ": " << error.what() << '\n';
        return 6;
    }
    if (arguments.size() == 1 && arguments[0] == L"--get-presentation") {
        const fs::path configPath = dataRoot / L"config.toml";
        std::string text = fcitx::windows::config::defaultConfigToml();
        if (fs::exists(configPath) && !readUtf8(configPath, text))
            return 5;
        Config config;
        ParseError error;
        if (!fcitx::windows::config::parseConfig(text, config, error))
            return 3;
        const char* mode =
            !config.appearanceMode ||
                    *config.appearanceMode == fcitx::windows::config::AppearanceMode::system
                ? "system"
                : (*config.appearanceMode == fcitx::windows::config::AppearanceMode::light
                       ? "light"
                       : "dark");
        const char* orientation =
            !config.orientation ||
                    *config.orientation == fcitx::windows::config::Orientation::vertical
                ? "vertical"
                : "horizontal";
        const std::string theme = config.theme.value_or("builtin:default");
        const bool scrollMode = config.scrollMode.value_or(false);
        const int pageSize = config.candidatePageSize.value_or(5);
        const double maxWidth = config.maxWidth.value_or(860.0);
        const double scrollCellWidth = config.scrollCellWidth.value_or(96.0);
        const double fontSize = config.candidateFont.size.value_or(18.0);
        const double cornerRadius = config.geometry.cornerRadius.value_or(12.0);
        const bool shadow = config.geometry.shadow.value_or(true);
        const std::string font =
            config.candidateFont.families && !config.candidateFont.families->empty()
                ? config.candidateFont.families->front()
                : "Microsoft YaHei";
        std::cout << "{\"format_version\":1,\"appearance_mode\":" << jsonString(mode)
                  << ",\"theme\":" << jsonString(theme)
                  << ",\"orientation\":" << jsonString(orientation)
                  << ",\"candidate_font\":" << jsonString(font)
                  << ",\"candidate_page_size\":" << jsonString(std::to_string(pageSize))
                  << ",\"candidate_max_width_dip\":"
                  << jsonString(std::to_string(static_cast<int>(maxWidth)))
                  << ",\"candidate_scroll_cell_width_dip\":"
                  << jsonString(std::to_string(static_cast<int>(scrollCellWidth)))
                  << ",\"candidate_font_size_dip\":"
                  << jsonString(std::to_string(static_cast<int>(fontSize)))
                  << ",\"candidate_corner_radius_dip\":"
                  << jsonString(std::to_string(static_cast<int>(cornerRadius)))
                  << ",\"candidate_shadow\":" << (shadow ? "true" : "false")
                  << ",\"scroll_mode\":" << (scrollMode ? "true" : "false") << "}\n";
        return 0;
    }
    if ((arguments.size() == 7 || arguments.size() == 9 || arguments.size() == 12) &&
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
                arguments.size() == 12 ? narrow(arguments[9]) : std::string{},
                arguments.size() == 12 ? narrow(arguments[10]) : std::string{},
                arguments.size() == 12 ? narrow(arguments[11]) : std::string{})) {
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
        std::cout << "{\"format_version\":1,\"launcher_reachable\":"
                  << (reachable ? "true" : "false") << ",\"launcher_state\":"
                  << (reachable ? std::to_string(response.launcherState) : "null")
                  << ",\"engine_state\":"
                  << (reachable ? std::to_string(response.engineState) : "null")
                  << ",\"current_input_method_id\":"
                  << (reachable ? jsonString(response.currentInputMethodId) : "null")
                  << ",\"current_input_method_name\":"
                  << (reachable ? jsonString(response.currentInputMethodName) : "null")
                  << ",\"current_input_method_native_name\":"
                  << (reachable ? jsonString(response.currentInputMethodNativeName) : "null")
                  << ",\"current_input_method_short_label\":"
                  << (reachable ? jsonString(response.currentInputMethodShortLabel) : "null")
                  << ",\"config_valid\":" << (configValid ? "true" : "false")
                  << ",\"tsf_guard_disabled\":"
                  << (tsfGuard.disabled ? "true" : "false")
                  << ",\"tsf_guard_reason\":" << jsonString(tsfGuard.reason)
                  << ",\"data_root\":\""
                  << narrow(dataRoot.generic_wstring()) << "\",\"update_owner\":\""
                  << fcitx::update::owner_name(fcitx::update::read_update_owner(installationRoot()))
                  << "\"}\n";
        return configValid ? 0 : 3;
    }
    if (arguments.size() == 1 && arguments[0] == L"--restart-engine") {
        fcitx::windows::protocol::LauncherResponse response;
        if (!launcherCommand(fcitx::windows::protocol::LauncherCommand::userStop, response) ||
            !launcherCommand(fcitx::windows::protocol::LauncherCommand::resume, response) ||
            !launcherCommand(fcitx::windows::protocol::LauncherCommand::startDemand, response)) {
            std::cerr << "launcher unavailable or restart rejected\n";
            return 4;
        }
        return 0;
    }
    if (arguments.size() == 1 && arguments[0] == L"--shutdown") {
        fcitx::windows::protocol::LauncherResponse response;
        return launcherCommand(fcitx::windows::protocol::LauncherCommand::shutdown, response)
                   ? 0
                   : 4;
    }
    if (arguments.size() == 1 && arguments[0] == L"--reset-config") {
        return writeVisualConfig(dataRoot / L"config.toml",
                                 fcitx::windows::config::defaultConfigToml())
                   ? 0
                   : 5;
    }
    if (arguments.size() == 2 &&
        (arguments[0] == L"--validate-config" || arguments[0] == L"--apply-config")) {
        std::string text;
        ParseError error;
        if (!validateConfig(fs::path(arguments[1]), text, error)) {
            std::cerr << "invalid config at " << error.line << ':' << error.column << ": "
                      << error.message << '\n';
            return 3;
        }
        if (arguments[0] == L"--validate-config")
            return 0;
        return writeVisualConfig(dataRoot / L"config.toml", text) ? 0 : 5;
    }
    usage();
    return 2;
}
