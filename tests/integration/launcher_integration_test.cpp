#include "launcher_client.h"
#include "pipe_client.h"
#include "runtime_identity.h"

#include <Windows.h>

#include <array>
#include <cstdint>
#include <iostream>
#include <string>
#include <vector>

namespace {

constexpr std::uint32_t kLauncherUserStopped = 1;
constexpr std::uint32_t kStartSuppressed = 2;

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

bool send(const fcitx::windows::platform::RuntimeIdentity& identity,
          const wchar_t* launcherPath, fcitx::windows::protocol::LauncherCommand command,
          fcitx::windows::protocol::LauncherResponse& response) {
    return fcitx::windows::ipc::sendLauncherCommand(
        identity, GetTickCount64() + 1000,
        fcitx::windows::ipc::PeerPolicy::exact(launcherPath), command, response);
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 3) {
        std::cerr << "launcher and engine arguments required\n";
        return 1;
    }
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity)) return 1;
    const std::wstring suffix = std::to_wstring(GetCurrentProcessId());
    if (!SetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", (L"launcher-" + suffix).c_str()))
        return 1;
    const std::wstring engineReadyName =
        L"Local\\Fcitx5WindowsNext.LauncherTest.EngineReady." + suffix;
    const std::wstring launcherReadyName =
        L"Local\\Fcitx5WindowsNext.LauncherTest.LauncherReady." + suffix;
    const std::wstring stopName = L"Local\\Fcitx5WindowsNext.LauncherTest.Stop." + suffix;
    std::array<wchar_t, MAX_PATH> temporaryDirectory{};
    if (GetTempPathW(static_cast<DWORD>(temporaryDirectory.size()),
                     temporaryDirectory.data()) == 0) return 1;
    const std::wstring stateFile = std::wstring(temporaryDirectory.data()) +
                                   L"fcitx5-launcher-test-" + suffix + L".state";
    HANDLE engineReady = CreateEventW(nullptr, TRUE, FALSE, engineReadyName.c_str());
    HANDLE launcherReady = CreateEventW(nullptr, TRUE, FALSE, launcherReadyName.c_str());
    HANDLE stopEvent = CreateEventW(nullptr, TRUE, FALSE, stopName.c_str());
    if (!engineReady || !launcherReady || !stopEvent) return 1;

    std::wstring command = quote(argv[1]) + L" --engine " + quote(argv[2]) +
                           L" --no-warmup --engine-ready-event " + quote(engineReadyName) +
                           L" --ready-event " + quote(launcherReadyName) +
                           L" --stop-event " + quote(stopName) + L" --state-file " +
                           quote(stateFile);
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(argv[1], mutableCommand.data(), nullptr, nullptr, FALSE, CREATE_NO_WINDOW,
                        nullptr, nullptr, &startup, &process)) {
        CloseHandle(stopEvent);
        CloseHandle(launcherReady);
        CloseHandle(engineReady);
        return 1;
    }
    CloseHandle(process.hThread);
    int stage = 0;
    int result = WaitForSingleObject(launcherReady, 2000) == WAIT_OBJECT_0 ? 0 : 1;
    std::uint32_t failureStatus = 0;
    if (result != 0) stage = 1;
    fcitx::windows::protocol::LauncherResponse response;
    int controlFailure = 0;
    DWORD controlError = ERROR_SUCCESS;
    const auto sendAndAwaitNext = [&](fcitx::windows::protocol::LauncherCommand command) {
        controlFailure = 0;
        ResetEvent(launcherReady);
        if (!send(identity, argv[1], command, response)) {
            controlFailure = 1;
            controlError = GetLastError();
            return false;
        }
        if (WaitForSingleObject(launcherReady, 2000) != WAIT_OBJECT_0) {
            controlFailure = 2;
            return false;
        }
        return true;
    };
    if (result == 0) {
        if (!sendAndAwaitNext(fcitx::windows::protocol::LauncherCommand::startDemand)) {
            result = 1;
            stage = 20 + controlFailure;
        } else if (response.status != fcitx::windows::protocol::Status::ok) {
            result = 1;
            stage = 21;
            failureStatus = static_cast<std::uint32_t>(response.status);
        } else if (WaitForSingleObject(engineReady, 2000) != WAIT_OBJECT_0) {
            result = 1;
            stage = 22;
        }
    }
    if (result == 0) {
        const std::wstring engineEndpoint =
            fcitx::windows::platform::makeLocalEndpointName(identity, L"engine");
        fcitx::windows::ipc::PipeClient client(
            engineEndpoint, fcitx::windows::ipc::PeerPolicy::exact(argv[2]));
        fcitx::windows::ipc::KeyResult keyResult;
        if (!client.processKey(1, 'A', 0, keyResult) || !keyResult.handled ||
            keyResult.commit != L"a") {
            result = 1;
            stage = 3;
        }
    }
    if (result == 0 &&
        (!sendAndAwaitNext(fcitx::windows::protocol::LauncherCommand::status) ||
         response.status != fcitx::windows::protocol::Status::ok ||
         response.currentInputMethodId != "mock-pinyin" ||
         response.currentInputMethodName != "Mock Pinyin" ||
         response.currentInputMethodNativeName != "\xe5\xb0\x8f\xe4\xbc\x81\xe9\xb9\x85" ||
         response.currentInputMethodShortLabel != "\xe5\xb0\x8f")) {
        result = 1;
        stage = 35;
    }
    if (result == 0 &&
        (!sendAndAwaitNext(fcitx::windows::protocol::LauncherCommand::userStop) ||
         response.status != fcitx::windows::protocol::Status::ok ||
         response.launcherState != kLauncherUserStopped ||
         !sendAndAwaitNext(fcitx::windows::protocol::LauncherCommand::startDemand) ||
         response.startDisposition != kStartSuppressed)) {
        result = 1;
        stage = 4;
    }
    if (result == 0 &&
        (!sendAndAwaitNext(fcitx::windows::protocol::LauncherCommand::resume) ||
         response.status != fcitx::windows::protocol::Status::ok ||
         !sendAndAwaitNext(fcitx::windows::protocol::LauncherCommand::startDemand) ||
         response.status != fcitx::windows::protocol::Status::ok ||
         WaitForSingleObject(engineReady, 2000) != WAIT_OBJECT_0)) {
        result = 1;
        stage = 5;
    }
    if (!send(identity, argv[1], fcitx::windows::protocol::LauncherCommand::shutdown, response)) {
        SetEvent(stopEvent);
        result = 1;
        stage = 6;
    }
    if (WaitForSingleObject(process.hProcess, 3000) != WAIT_OBJECT_0) {
        SetEvent(stopEvent);
        if (WaitForSingleObject(process.hProcess, 1000) != WAIT_OBJECT_0) {
            TerminateProcess(process.hProcess, 2);
            WaitForSingleObject(process.hProcess, 1000);
        }
        result = 1;
        stage = 7;
    }
    DWORD exitCode = 1;
    GetExitCodeProcess(process.hProcess, &exitCode);
    CloseHandle(process.hProcess);
    CloseHandle(stopEvent);
    CloseHandle(launcherReady);
    CloseHandle(engineReady);
    DeleteFileW(stateFile.c_str());
    if (exitCode != 0) {
        result = 1;
        stage = 8;
    }
    if (result != 0) {
        std::cerr << "launcher lifecycle integration failed at stage " << stage
                  << ", launcher exit " << exitCode << ", Win32 error " << controlError
                  << ", control failure " << controlFailure
                  << ", status " << failureStatus
                  << ", launcher state " << response.launcherState
                  << ", engine state " << response.engineState
                  << ", start disposition " << response.startDisposition
                  << ", input id '" << response.currentInputMethodId
                  << "', name '" << response.currentInputMethodName
                  << "', native '" << response.currentInputMethodNativeName
                  << "', short '" << response.currentInputMethodShortLabel << "'"
                  << '\n';
    }
    return result;
}
