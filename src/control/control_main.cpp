#include "config_model.h"
#include "deployment_core.h"
#include "fcitx5_windows/release_identity.h"
#include "fcitx5_windows/version.h"
#include "launcher_client.h"
#include "package_core.h"
#include "peer_verification.h"
#include "protocol.h"
#include "runtime_identity.h"

#include <ShlObj.h>
#include <Windows.h>

#include <algorithm>
#include <array>
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

bool runProcessCapture(const fs::path& executable,
                       const std::vector<std::wstring>& arguments,
                       std::string& output, DWORD timeout = 120000U) {
    output.clear();
    SECURITY_ATTRIBUTES attributes{sizeof(attributes), nullptr, TRUE};
    HANDLE readPipe = nullptr;
    HANDLE writePipe = nullptr;
    if (!CreatePipe(&readPipe, &writePipe, &attributes, 0))
        return false;
    SetHandleInformation(readPipe, HANDLE_FLAG_INHERIT, 0);
    HANDLE nullError = CreateFileW(L"NUL", GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE,
                                   &attributes, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (nullError == INVALID_HANDLE_VALUE) {
        CloseHandle(readPipe);
        CloseHandle(writePipe);
        return false;
    }
    std::wstring command = quoteArgument(executable.wstring());
    for (const auto& argument : arguments)
        command += L" " + quoteArgument(argument);
    STARTUPINFOW startup{sizeof(startup)};
    startup.dwFlags = STARTF_USESTDHANDLES;
    startup.hStdInput = GetStdHandle(STD_INPUT_HANDLE);
    startup.hStdOutput = writePipe;
    startup.hStdError = nullError;
    PROCESS_INFORMATION process{};
    const BOOL created = CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, TRUE,
                                        CREATE_NO_WINDOW, nullptr,
                                        executable.parent_path().c_str(), &startup, &process);
    CloseHandle(writePipe);
    CloseHandle(nullError);
    if (!created) {
        CloseHandle(readPipe);
        return false;
    }
    const DWORD wait = WaitForSingleObject(process.hProcess, timeout);
    if (wait == WAIT_TIMEOUT)
        TerminateProcess(process.hProcess, ERROR_TIMEOUT);
    char buffer[4096];
    DWORD count = 0;
    while (output.size() <= 1024U * 1024U &&
           ReadFile(readPipe, buffer, sizeof(buffer), &count, nullptr) && count != 0)
        output.append(buffer, count);
    DWORD code = 1;
    GetExitCodeProcess(process.hProcess, &code);
    CloseHandle(readPipe);
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    return wait == WAIT_OBJECT_0 && code == 0 && output.size() <= 1024U * 1024U;
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

fcitx::package::RepositoryIndex loadRepository(const fs::path& dataRoot) {
    const auto files = repositoryFiles(dataRoot);
    std::string index;
    if (!readUtf8(files.index, index))
        throw fcitx::package::PackageError("repository_unavailable",
                                           "repository cache is unavailable");
    const auto signature = readBinary(files.signature, 16U * 1024U);
    return fcitx::package::verify_repository_index(
        index, signature, fcitx::package::read_trusted_keys(files.keyring));
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
    static_cast<void>(fcitx::package::verify_repository_index(
        index, signature, fcitx::package::read_trusted_keys(files.keyring)));
    if (!MoveFileExW(incomingSignature.c_str(), files.signature.c_str(),
                     MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) ||
        !MoveFileExW(incomingIndex.c_str(), files.index.c_str(),
                     MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
        throw fcitx::package::PackageError("io_error", "repository cache publication failed");
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

fs::path defaultDataRoot() {
    const fs::path executable = executableDirectory();
    if (!executable.empty() && fs::exists(executable / L"portable.flag")) {
        return executable / L"data";
    }
    if (!executable.empty() && fs::exists(executable.parent_path() / L"portable.flag")) {
        return executable.parent_path() / L"data";
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
    const bool commandOk =
        runProcessCapture(executableDirectory() / L"fcitx5-engine.exe", arguments, output);
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
                  L"--set-presentation MODE THEME ORIENTATION SCROLL FONT|"
                  L"--packages-list|--packages-refresh [HTTPS_BASE]|"
                  L"--packages-install ID|--packages-update ID|"
                  L"--packages-state ID enabled|disabled|--packages-remove ID|"
                  L"--packages-repair|--schema|--version\n";
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
            << R"({"format_version":1,"commands":["status","restart_engine","shutdown","validate_config","apply_config","reset_config","get_startup","set_startup","get_presentation","set_presentation","get_input_methods","set_input_method","packages_list","packages_refresh","packages_install","packages_update","packages_state","packages_remove","packages_repair"],"sensitive_input":false,"package_network_owner":"fcitx5-downloader.exe"})"
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
        const std::string font =
            config.candidateFont.families && !config.candidateFont.families->empty()
                ? config.candidateFont.families->front()
                : "Microsoft YaHei";
        std::cout << "{\"format_version\":1,\"appearance_mode\":" << jsonString(mode)
                  << ",\"theme\":" << jsonString(theme)
                  << ",\"orientation\":" << jsonString(orientation)
                  << ",\"candidate_font\":" << jsonString(font)
                  << ",\"scroll_mode\":" << (scrollMode ? "true" : "false") << "}\n";
        return 0;
    }
    if (arguments.size() == 6 && arguments[0] == L"--set-presentation") {
        const fs::path configPath = dataRoot / L"config.toml";
        std::string source = fcitx::windows::config::defaultConfigToml();
        if (fs::exists(configPath) && !readUtf8(configPath, source))
            return 5;
        std::string updated;
        ParseError error;
        if (!fcitx::windows::config::updatePresentationToml(
                source, narrow(arguments[1]), narrow(arguments[2]), narrow(arguments[3]),
                narrow(arguments[4]), narrow(arguments[5]), updated, error)) {
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
        std::cout << "{\"format_version\":1,\"launcher_reachable\":"
                  << (reachable ? "true" : "false") << ",\"launcher_state\":"
                  << (reachable ? std::to_string(response.launcherState) : "null")
                  << ",\"engine_state\":"
                  << (reachable ? std::to_string(response.engineState) : "null")
                  << ",\"config_valid\":" << (configValid ? "true" : "false") << ",\"data_root\":\""
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
