#include "launcher_client.h"
#include "runtime_identity.h"

#include <Windows.h>

#include <array>
#include <iostream>
#include <string>
#include <vector>

namespace {

constexpr std::uint32_t kLauncherSafeMode = 5;
constexpr std::uint32_t kEngineReady = 2;
constexpr std::uint32_t kCommandStartDemand = 1;
constexpr std::uint32_t kCommandStatus = 8;
constexpr std::uint32_t kCommandShutdown = 9;

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 3) return 1;
    using namespace fcitx::windows;
    platform::RuntimeIdentity identity;
    if (!platform::queryCurrentIdentity(identity)) return 1;
    const std::wstring suffix = std::to_wstring(GetCurrentProcessId());
    if (!SetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", (L"crash-" + suffix).c_str()))
        return 1;
    const std::wstring launcherReadyName =
        L"Local\\Fcitx5WindowsNext.CrashTest.Launcher." + suffix;
    const std::wstring stopName = L"Local\\Fcitx5WindowsNext.CrashTest.Stop." + suffix;
    const std::wstring safeName = L"Local\\Fcitx5WindowsNext.CrashTest.Safe." + suffix;
    std::array<wchar_t, MAX_PATH> temporaryDirectory{};
    if (GetTempPathW(static_cast<DWORD>(temporaryDirectory.size()),
                     temporaryDirectory.data()) == 0) return 1;
    const std::wstring stateFile = std::wstring(temporaryDirectory.data()) +
                                   L"fcitx5-crash-loop-test-" + suffix + L".state";
    HANDLE launcherReady = CreateEventW(nullptr, TRUE, FALSE, launcherReadyName.c_str());
    HANDLE stopEvent = CreateEventW(nullptr, TRUE, FALSE, stopName.c_str());
    HANDLE safeEvent = CreateEventW(nullptr, TRUE, FALSE, safeName.c_str());
    if (!launcherReady || !stopEvent || !safeEvent) return 1;

    SetEnvironmentVariableW(L"FCITX_TEST_SAFE_MODE_EVENT", safeName.c_str());
    std::wstring command = quote(argv[1]) + L" --engine " + quote(argv[2]) +
                           L" --no-warmup --ready-event " + quote(launcherReadyName) +
                           L" --stop-event " + quote(stopName) + L" --state-file " +
                           quote(stateFile);
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    const bool created = CreateProcessW(argv[1], mutableCommand.data(), nullptr, nullptr,
                                        FALSE, CREATE_NO_WINDOW, nullptr, nullptr,
                                        &startup, &process) != FALSE;
    SetEnvironmentVariableW(L"FCITX_TEST_SAFE_MODE_EVENT", nullptr);
    if (!created) return 1;
    CloseHandle(process.hThread);

    int stage = 0;
    int result = WaitForSingleObject(launcherReady, 2000) == WAIT_OBJECT_0 ? 0 : 1;
    if (result != 0) stage = 1;
    ipc::LauncherResponse response;
    if (result == 0 && !ipc::sendLauncherCommand(
                           identity, GetTickCount64() + 1000,
                           ipc::PeerPolicy::exact(argv[1]),
                           kCommandStartDemand, response)) {
        result = 1;
        stage = 2;
    }
    if (result == 0 && WaitForSingleObject(safeEvent, 5000) != WAIT_OBJECT_0) {
        result = 1;
        stage = 3;
    }
    if (result == 0 &&
        (!ipc::sendLauncherCommand(identity, GetTickCount64() + 1000,
                                   ipc::PeerPolicy::exact(argv[1]),
                                   kCommandStatus, response) ||
         response.launcherState != kLauncherSafeMode || response.engineState != kEngineReady)) {
        result = 1;
        stage = 4;
    }
    if (!ipc::sendLauncherCommand(identity, GetTickCount64() + 1000,
                                  ipc::PeerPolicy::exact(argv[1]),
                                  kCommandShutdown, response)) {
        SetEvent(stopEvent);
        result = 1;
        stage = 5;
    }
    if (WaitForSingleObject(process.hProcess, 3000) != WAIT_OBJECT_0) {
        SetEvent(stopEvent);
        TerminateProcess(process.hProcess, 9);
        WaitForSingleObject(process.hProcess, 1000);
        result = 1;
        stage = 6;
    }
    DWORD exitCode = 1;
    GetExitCodeProcess(process.hProcess, &exitCode);
    CloseHandle(process.hProcess);
    CloseHandle(safeEvent);
    CloseHandle(stopEvent);
    CloseHandle(launcherReady);
    DeleteFileW(stateFile.c_str());
    if (exitCode != 0) {
        result = 1;
        stage = 7;
    }
    if (result != 0) {
        std::cerr << "launcher crash-loop did not converge to Safe Mode at stage " << stage
                  << ", exit " << exitCode << ", status "
                  << static_cast<std::uint32_t>(response.status) << ", launcher state "
                  << response.launcherState << ", engine state " << response.engineState
                  << '\n';
    }
    return result;
}
