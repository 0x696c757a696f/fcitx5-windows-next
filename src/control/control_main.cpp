#include "config_model.h"
#include "launcher_client.h"
#include "peer_verification.h"
#include "protocol.h"
#include "runtime_identity.h"
#include "fcitx5_windows/version.h"

#include <Windows.h>
#include <ShlObj.h>

#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <string_view>
#include <vector>

namespace {

namespace fs = std::filesystem;
using fcitx::windows::config::Config;
using fcitx::windows::config::ParseError;

std::string narrow(std::wstring_view value) {
    if (value.empty()) return {};
    const int count = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                                          static_cast<int>(value.size()), nullptr, 0, nullptr,
                                          nullptr);
    if (count <= 0) return {};
    std::string result(static_cast<std::size_t>(count), '\0');
    if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                            static_cast<int>(value.size()), result.data(), count, nullptr,
                            nullptr) != count) return {};
    return result;
}

std::string jsonString(std::string_view value) {
    std::string result = "\"";
    for (const unsigned char character : value) {
        switch (character) {
        case '\\': result += "\\\\"; break;
        case '"': result += "\\\""; break;
        case '\b': result += "\\b"; break;
        case '\f': result += "\\f"; break;
        case '\n': result += "\\n"; break;
        case '\r': result += "\\r"; break;
        case '\t': result += "\\t"; break;
        default:
            if (character < 0x20U) return {};
            result.push_back(static_cast<char>(character));
        }
    }
    result.push_back('"');
    return result;
}

bool readUtf8(const fs::path& path, std::string& text) {
    std::error_code error;
    const auto size = fs::file_size(path, error);
    if (error || size > 256U * 1024U) return false;
    std::ifstream stream(path, std::ios::binary);
    if (!stream) return false;
    text.assign(std::istreambuf_iterator<char>(stream), {});
    return stream.good() || stream.eof();
}

fs::path executableDirectory() {
    std::wstring path(32768, L'\0');
    const DWORD length = GetModuleFileNameW(nullptr, path.data(),
                                            static_cast<DWORD>(path.size()));
    if (length == 0 || length >= path.size()) return {};
    path.resize(length);
    return fs::path(path).parent_path();
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
    if (FAILED(SHGetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_CREATE, nullptr,
                                    &localAppData))) return {};
    fs::path result(localAppData);
    CoTaskMemFree(localAppData);
    return result / L"Fcitx5";
}

bool validateConfig(const fs::path& source, std::string& text, ParseError& parseError) {
    if (!readUtf8(source, text)) return false;
    Config config;
    return fcitx::windows::config::parseConfig(text, config, parseError);
}

bool atomicWrite(const fs::path& destination, std::string_view text) {
    std::error_code error;
    fs::create_directories(destination.parent_path(), error);
    if (error) return false;
    GUID identifier{};
    std::array<wchar_t, 40> identifierText{};
    if (FAILED(CoCreateGuid(&identifier)) ||
        StringFromGUID2(identifier, identifierText.data(),
                        static_cast<int>(identifierText.size())) == 0) return false;
    const fs::path temporary = destination.wstring() + L"." + identifierText.data() + L".tmp";
    HANDLE file = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                              FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    DWORD written = 0;
    const bool writeOk = text.size() <= MAXDWORD &&
                         WriteFile(file, text.data(), static_cast<DWORD>(text.size()), &written,
                                   nullptr) && written == text.size() && FlushFileBuffers(file);
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

bool launcherCommand(fcitx::windows::protocol::LauncherCommand command,
                     fcitx::windows::protocol::LauncherResponse& response) {
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity)) return false;
    const auto policy = fcitx::windows::ipc::PeerPolicy::exact(
        (executableDirectory() / L"fcitx5-launcher.exe").wstring());
    return fcitx::windows::ipc::sendLauncherCommand(identity, GetTickCount64() + 1000, policy,
                                                     command, response);
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
    const LSTATUS sizeResult =
        RegQueryValueExW(key, L"Fcitx5", nullptr, &type, nullptr, &bytes);
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
    const LSTATUS readResult = RegQueryValueExW(
        key, L"Fcitx5", nullptr, &type, reinterpret_cast<BYTE*>(value.data()), &bytes);
    RegCloseKey(key);
    while (!value.empty() && value.back() == L'\0') value.pop_back();
    enabled = readResult == ERROR_SUCCESS && value == startupCommand();
    return readResult == ERROR_SUCCESS;
}

bool setStartup(bool enabled) {
    constexpr wchar_t path[] = L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    HKEY key = nullptr;
    if (RegCreateKeyExW(HKEY_CURRENT_USER, path, 0, nullptr, 0, KEY_SET_VALUE, nullptr, &key,
                        nullptr) != ERROR_SUCCESS) return false;
    LSTATUS result = ERROR_SUCCESS;
    if (enabled) {
        const std::wstring command = startupCommand();
        result = RegSetValueExW(key, L"Fcitx5", 0, REG_SZ,
                                reinterpret_cast<const BYTE*>(command.c_str()),
                                static_cast<DWORD>((command.size() + 1) * sizeof(wchar_t)));
    } else {
        result = RegDeleteValueW(key, L"Fcitx5");
        if (result == ERROR_FILE_NOT_FOUND) result = ERROR_SUCCESS;
    }
    RegCloseKey(key);
    return result == ERROR_SUCCESS;
}

void usage() {
    std::wcerr << L"Usage: fcitx5-control [--data-root PATH] "
                  L"--status|--restart-engine|--validate-config FILE|--apply-config FILE|"
                  L"--reset-config|--get-startup|--set-startup enabled|disabled|"
                  L"--get-presentation|"
                  L"--set-presentation MODE THEME ORIENTATION FONT|--schema|--version\n";
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    fs::path dataRoot = defaultDataRoot();
    std::vector<std::wstring_view> arguments;
    for (int index = 1; index < argc; ++index) {
        const std::wstring_view argument(argv[index]);
        if (argument == L"--data-root" && index + 1 < argc) dataRoot = argv[++index];
        else arguments.push_back(argument);
    }
    if (arguments.size() == 1 && arguments[0] == L"--version") {
        std::cout << fcitx::windows::version() << '\n';
        return 0;
    }
    if (dataRoot.empty() || arguments.empty()) {
        usage();
        return 2;
    }
    if (arguments.size() == 1 && arguments[0] == L"--schema") {
        std::cout << R"({"format_version":1,"commands":["status","restart_engine","validate_config","apply_config","reset_config","get_startup","set_startup","get_presentation","set_presentation"],"sensitive_input":false})" << '\n';
        return 0;
    }
    if (arguments.size() == 1 && arguments[0] == L"--get-startup") {
        bool enabled = false;
        if (!queryStartup(enabled)) return 5;
        std::cout << "{\"format_version\":1,\"enabled\":"
                  << (enabled ? "true" : "false") << "}\n";
        return 0;
    }
    if (arguments.size() == 2 && arguments[0] == L"--set-startup" &&
        (arguments[1] == L"enabled" || arguments[1] == L"disabled")) {
        return setStartup(arguments[1] == L"enabled") ? 0 : 5;
    }
    if (arguments.size() == 1 && arguments[0] == L"--get-presentation") {
        const fs::path configPath = dataRoot / L"config.toml";
        std::string text = fcitx::windows::config::defaultConfigToml();
        if (fs::exists(configPath) && !readUtf8(configPath, text)) return 5;
        Config config;
        ParseError error;
        if (!fcitx::windows::config::parseConfig(text, config, error)) return 3;
        const char* mode = !config.appearanceMode ||
                                   *config.appearanceMode == fcitx::windows::config::AppearanceMode::system
                               ? "system"
                               : (*config.appearanceMode == fcitx::windows::config::AppearanceMode::light
                                      ? "light"
                                      : "dark");
        const char* orientation = !config.orientation ||
                                          *config.orientation == fcitx::windows::config::Orientation::vertical
                                      ? "vertical"
                                      : "horizontal";
        const std::string theme = config.theme.value_or("builtin:default");
        const std::string font = config.candidateFont.families &&
                                         !config.candidateFont.families->empty()
                                     ? config.candidateFont.families->front()
                                     : "Microsoft YaHei";
        std::cout << "{\"format_version\":1,\"appearance_mode\":" << jsonString(mode)
                  << ",\"theme\":" << jsonString(theme) << ",\"orientation\":"
                  << jsonString(orientation) << ",\"candidate_font\":" << jsonString(font)
                  << "}\n";
        return 0;
    }
    if (arguments.size() == 5 && arguments[0] == L"--set-presentation") {
        const fs::path configPath = dataRoot / L"config.toml";
        std::string source = fcitx::windows::config::defaultConfigToml();
        if (fs::exists(configPath) && !readUtf8(configPath, source)) return 5;
        std::string updated;
        ParseError error;
        if (!fcitx::windows::config::updatePresentationToml(
                source, narrow(arguments[1]), narrow(arguments[2]), narrow(arguments[3]),
                narrow(arguments[4]), updated, error)) {
            std::cerr << "invalid presentation at " << error.line << ':' << error.column
                      << ": " << error.message << '\n';
            return 3;
        }
        return atomicWrite(configPath, updated) ? 0 : 5;
    }
    if (arguments.size() == 1 && arguments[0] == L"--status") {
        fcitx::windows::protocol::LauncherResponse response;
        const bool reachable = launcherCommand(
            fcitx::windows::protocol::LauncherCommand::status, response);
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
                  << ",\"config_valid\":" << (configValid ? "true" : "false")
                  << ",\"data_root\":\"" << narrow(dataRoot.generic_wstring()) << "\"}\n";
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
    if (arguments.size() == 1 && arguments[0] == L"--reset-config") {
        return atomicWrite(dataRoot / L"config.toml",
                           fcitx::windows::config::defaultConfigToml()) ? 0 : 5;
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
        if (arguments[0] == L"--validate-config") return 0;
        return atomicWrite(dataRoot / L"config.toml", text) ? 0 : 5;
    }
    usage();
    return 2;
}
