#include "fcitx5_windows/version.h"

#include <Windows.h>
#include <shellapi.h>

#include <filesystem>
#include <string>
#include <string_view>
#include <vector>

namespace {

namespace fs = std::filesystem;

fs::path executablePath() {
    std::wstring value(32'768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, value.data(), static_cast<DWORD>(value.size()));
    if (size == 0 || size >= value.size())
        return {};
    value.resize(size);
    return value;
}

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

bool launchAndWait(const fs::path& executable, const std::wstring& arguments, DWORD& exitCode) {
    std::wstring command = quote(executable.wstring()) + L" " + arguments;
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(executable.c_str(), mutableCommand.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process))
        return false;
    CloseHandle(process.hThread);
    const DWORD wait = WaitForSingleObject(process.hProcess, 60'000);
    const bool success = wait == WAIT_OBJECT_0 &&
                         GetExitCodeProcess(process.hProcess, &exitCode) != FALSE;
    if (wait == WAIT_TIMEOUT) {
        TerminateProcess(process.hProcess, ERROR_TIMEOUT);
        WaitForSingleObject(process.hProcess, 5000);
    }
    CloseHandle(process.hProcess);
    return success;
}

bool launchDetached(const fs::path& executable, const std::wstring& arguments) {
    std::wstring command = quote(executable.wstring()) + L" " + arguments;
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(executable.c_str(), mutableCommand.data(), nullptr, nullptr, FALSE,
                        CREATE_NEW_PROCESS_GROUP, nullptr, nullptr, &startup, &process))
        return false;
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    return true;
}

int runRegistration(const fs::path& root, bool unregister) {
    const fs::path register64 = root / L"bin" / L"fcitx5-register.exe";
    const fs::path register32 = root / L"bin" / L"fcitx5-register-x86.exe";
    const fs::path dll64 = root / L"tsf" / L"x64" / L"fcitx5-tsf.dll";
    const fs::path dll32 = root / L"tsf" / L"x86" / L"fcitx5-tsf.dll";
    const std::wstring operation = unregister ? L"--unregister" : L"--repair";
    for (const auto& pair : {std::pair{register64, dll64}, std::pair{register32, dll32}}) {
        if (!fs::is_regular_file(pair.first) || !fs::is_regular_file(pair.second))
            return 10;
        DWORD exitCode = 0;
        if (!launchAndWait(pair.first, operation + L" --dll " + quote(pair.second.wstring()),
                           exitCode) ||
            exitCode != 0)
            return 11;
    }
    return 0;
}

bool registrationHealthy(const fs::path& root) {
    const fs::path register64 = root / L"bin" / L"fcitx5-register.exe";
    const fs::path register32 = root / L"bin" / L"fcitx5-register-x86.exe";
    const fs::path dll64 = root / L"tsf" / L"x64" / L"fcitx5-tsf.dll";
    const fs::path dll32 = root / L"tsf" / L"x86" / L"fcitx5-tsf.dll";
    for (const auto& pair : {std::pair{register64, dll64}, std::pair{register32, dll32}}) {
        if (!fs::is_regular_file(pair.first) || !fs::is_regular_file(pair.second))
            return false;
        DWORD exitCode = 0;
        if (!launchAndWait(pair.first, L"--status --dll " + quote(pair.second.wstring()),
                           exitCode) ||
            exitCode != 0)
            return false;
    }
    return true;
}

int elevateRegistration(const fs::path& executable, bool unregister) {
    const wchar_t* argument = unregister ? L"--elevated-unregister" : L"--elevated-register";
    SHELLEXECUTEINFOW info{};
    info.cbSize = sizeof(info);
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    info.lpVerb = L"runas";
    info.lpFile = executable.c_str();
    info.lpParameters = argument;
    info.nShow = SW_HIDE;
    if (!ShellExecuteExW(&info) || !info.hProcess)
        return 12;
    const DWORD wait = WaitForSingleObject(info.hProcess, 120'000);
    DWORD exitCode = 13;
    if (wait == WAIT_OBJECT_0)
        GetExitCodeProcess(info.hProcess, &exitCode);
    else if (wait == WAIT_TIMEOUT) {
        TerminateProcess(info.hProcess, ERROR_TIMEOUT);
        WaitForSingleObject(info.hProcess, 5000);
        exitCode = ERROR_TIMEOUT;
    }
    CloseHandle(info.hProcess);
    return static_cast<int>(exitCode);
}

void errorBox(std::wstring_view detail) {
    const std::wstring message = L"Fcitx5 for Windows Next could not complete the operation.\n\n" +
                                 std::wstring(detail);
    MessageBoxW(nullptr, message.c_str(), L"Fcitx5 for Windows Next", MB_OK | MB_ICONERROR);
}

enum class Action { start, settings, unregister };

Action inferredAction(const fs::path& executable) {
    const std::wstring name = executable.stem().wstring();
    if (name.find(L"Settings") != std::wstring::npos)
        return Action::settings;
    if (name.find(L"Unregister") != std::wstring::npos)
        return Action::unregister;
    return Action::start;
}

} // namespace

int WINAPI wWinMain(_In_ HINSTANCE, _In_opt_ HINSTANCE, _In_ PWSTR, _In_ int) {
    const fs::path executable = executablePath();
    if (executable.empty())
        return 2;
    const fs::path root = executable.parent_path();
    const int argc = __argc;
    wchar_t** argv = __wargv;
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
        MessageBoxA(nullptr, fcitx::windows::version().data(), "Fcitx5 for Windows Next",
                    MB_OK | MB_ICONINFORMATION);
        return 0;
    }
    if (argc == 2 && std::wstring_view(argv[1]) == L"--self-test") {
        return fs::is_regular_file(root / L"bin" / L"fcitx5-launcher.exe") &&
                       fs::is_regular_file(root / L"bin" / L"fcitx5-config.exe") &&
                       fs::is_regular_file(root / L"tsf" / L"x64" / L"fcitx5-tsf.dll") &&
                       fs::is_regular_file(root / L"tsf" / L"x86" / L"fcitx5-tsf.dll")
                   ? 0
                   : 3;
    }
    if (argc == 2 && std::wstring_view(argv[1]) == L"--elevated-register")
        return runRegistration(root, false);
    if (argc == 2 && std::wstring_view(argv[1]) == L"--elevated-unregister")
        return runRegistration(root, true);
    if (argc == 2 && std::wstring_view(argv[1]) == L"--repair-only") {
        const int result = elevateRegistration(executable, false);
        if (result != 0) {
            errorBox(L"Administrator approval is required to repair both TSF architectures.");
            return result;
        }
        if (!launchDetached(root / L"bin" / L"fcitx5-launcher.exe", L"--background")) {
            errorBox(L"Registration was repaired, but the input method service could not start.");
            return 5;
        }
        MessageBoxW(nullptr,
                    L"Fcitx5 registration and background service were repaired successfully.",
                    L"Fcitx5 for Windows Next", MB_OK | MB_ICONINFORMATION);
        return 0;
    }

    Action action = inferredAction(executable);
    if (argc == 2 && std::wstring_view(argv[1]) == L"--settings")
        action = Action::settings;
    else if (argc == 2 && std::wstring_view(argv[1]) == L"--unregister")
        action = Action::unregister;
    else if (argc > 1 && !(argc == 2 && std::wstring_view(argv[1]) == L"--start"))
        return 2;

    if (action == Action::settings) {
        if (!launchDetached(root / L"bin" / L"fcitx5-config.exe", L"")) {
            errorBox(L"The settings program is missing or could not be started.");
            return 4;
        }
        return 0;
    }

    if (action == Action::unregister) {
        DWORD ignored = 0;
        launchAndWait(root / L"bin" / L"fcitx5-control.exe", L"--shutdown", ignored);
        const int result = elevateRegistration(executable, true);
        if (result != 0) {
            errorBox(L"Administrator approval is required to unregister the TSF components.");
            return result;
        }
        MessageBoxW(nullptr, L"Fcitx5 has been unregistered. The portable files can now be removed.",
                    L"Fcitx5 for Windows Next", MB_OK | MB_ICONINFORMATION);
        return 0;
    }

    if (!registrationHealthy(root)) {
        const int registration = elevateRegistration(executable, false);
        if (registration != 0) {
            errorBox(L"Administrator approval is required once to register the TSF components.");
            return registration;
        }
    }
    if (!launchDetached(root / L"bin" / L"fcitx5-launcher.exe", L"--background")) {
        errorBox(L"The input method service could not be started.");
        return 5;
    }
    MessageBoxW(nullptr,
                L"Fcitx5 is running. Select it from the Windows input indicator (Win+Space).\n\n"
                L"Use 'Fcitx5 Settings.exe' in this folder to configure it.",
                L"Fcitx5 for Windows Next", MB_OK | MB_ICONINFORMATION);
    return 0;
}
