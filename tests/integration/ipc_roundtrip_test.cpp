#include "pipe_client.h"

#include <Windows.h>

#include <cstdint>
#include <iostream>
#include <iterator>
#include <string>
#include <thread>
#include <vector>

namespace {

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) {
        std::cerr << "engine executable argument required\n";
        return 1;
    }

    const std::wstring uniqueSuffix = std::to_wstring(GetCurrentProcessId());
    std::wstring executablePath(32768, L'\0');
    const DWORD executableLength = GetModuleFileNameW(
        nullptr, executablePath.data(), static_cast<DWORD>(executablePath.size()));
    if (executableLength == 0 || executableLength == executablePath.size()) {
        std::cerr << "failed to resolve test executable path\n";
        return 1;
    }
    executablePath.resize(executableLength);

    {
        fcitx::windows::ipc::PipeClient missingClient(
            L"\\\\.\\pipe\\Fcitx5WindowsNext.Missing." + uniqueSuffix);
        fcitx::windows::ipc::KeyResult missingResult;
        const auto started = GetTickCount64();
        if (missingClient.processKey(1, 'A', 0, missingResult) ||
            GetTickCount64() - started > 100 || missingResult.handled) {
            std::cerr << "missing engine did not fail open within the bound\n";
            return 1;
        }
    }

    {
        const std::wstring stalledPipeName =
            L"\\\\.\\pipe\\Fcitx5WindowsNext.Stalled." + uniqueSuffix;
        HANDLE stalledPipe =
            CreateNamedPipeW(stalledPipeName.c_str(), PIPE_ACCESS_DUPLEX,
                             PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT, 1, 4096, 4096, 0,
                             nullptr);
        HANDLE releaseEvent = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        if (!stalledPipe || stalledPipe == INVALID_HANDLE_VALUE || !releaseEvent) {
            if (stalledPipe && stalledPipe != INVALID_HANDLE_VALUE) {
                CloseHandle(stalledPipe);
            }
            if (releaseEvent) {
                CloseHandle(releaseEvent);
            }
            std::cerr << "failed to create stalled-engine fixture\n";
            return 1;
        }

        bool serverAcceptedRequest = false;
        std::thread stalledServer([&] {
            const bool connected =
                ConnectNamedPipe(stalledPipe, nullptr) != FALSE ||
                GetLastError() == ERROR_PIPE_CONNECTED;
            std::uint8_t request[128]{};
            DWORD bytesRead = 0;
            serverAcceptedRequest =
                connected && ReadFile(stalledPipe, request, sizeof(request), &bytesRead, nullptr) !=
                                 FALSE &&
                bytesRead > 0;
            WaitForSingleObject(releaseEvent, 1000);
        });

        fcitx::windows::ipc::PipeClient stalledClient(
            stalledPipeName, fcitx::windows::ipc::PeerPolicy::exact(executablePath));
        fcitx::windows::ipc::KeyResult stalledResult;
        const auto started = GetTickCount64();
        const bool processed = stalledClient.processKey(1, 'A', 0, stalledResult);
        const auto elapsed = GetTickCount64() - started;
        SetEvent(releaseEvent);
        stalledServer.join();
        DisconnectNamedPipe(stalledPipe);
        CloseHandle(stalledPipe);
        CloseHandle(releaseEvent);

        if (processed ||
            elapsed > fcitx::windows::ipc::kContextStartDeadlineMilliseconds + 500 ||
            stalledResult.handled || !serverAcceptedRequest) {
            std::cerr << "stalled engine did not fail open within the bound\n";
            return 1;
        }
    }

    const std::wstring pipeName = L"\\\\.\\pipe\\Fcitx5WindowsNext.Test." + uniqueSuffix;
    const std::wstring readyEventName = L"Local\\Fcitx5WindowsNext.Test.Ready." + uniqueSuffix;
    HANDLE readyEvent = CreateEventW(nullptr, TRUE, FALSE, readyEventName.c_str());
    if (!readyEvent) {
        std::cerr << "failed to create readiness event\n";
        return 1;
    }
    std::wstring commandLine = quote(argv[1]) + L" --test-once --pipe " + quote(pipeName) +
                               L" --ready-event " + quote(readyEventName);
    std::vector<wchar_t> mutableCommand(commandLine.begin(), commandLine.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(argv[1], mutableCommand.data(), nullptr, nullptr, FALSE, CREATE_NO_WINDOW,
                        nullptr, nullptr, &startup, &process)) {
        std::cerr << "failed to start mock engine: " << GetLastError() << '\n';
        CloseHandle(readyEvent);
        return 1;
    }
    CloseHandle(process.hThread);

    const bool available = WaitForSingleObject(readyEvent, 2000) == WAIT_OBJECT_0;
    CloseHandle(readyEvent);

    int resultCode = 0;
    {
        fcitx::windows::ipc::PipeClient client(
            pipeName, fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
        fcitx::windows::ipc::KeyResult result;
        fcitx::windows::protocol::EngineStatusResponse status;
        if (!available || !client.processKey(7, 'A', 0, result) || !result.handled ||
            result.commit != L"a") {
            std::cerr << "IPC key-to-commit roundtrip failed\n";
            resultCode = 1;
        } else if (!client.queryEngineStatus(status) ||
                   status.currentInputMethodId != "mock-pinyin" ||
                   status.currentInputMethodShortLabel != "\xe5\xb0\x8f") {
            std::cerr << "IPC engine-status roundtrip failed\n";
            resultCode = 1;
        }
    }

    if (WaitForSingleObject(process.hProcess, 2000) != WAIT_OBJECT_0) {
        TerminateProcess(process.hProcess, 2);
        WaitForSingleObject(process.hProcess, 1000);
        std::cerr << "mock engine did not exit after client disconnect\n";
        resultCode = 1;
    }
    DWORD engineExitCode = 1;
    GetExitCodeProcess(process.hProcess, &engineExitCode);
    CloseHandle(process.hProcess);
    if (engineExitCode != 0) {
        std::cerr << "mock engine returned " << engineExitCode << '\n';
        resultCode = 1;
    }
    return resultCode;
}
