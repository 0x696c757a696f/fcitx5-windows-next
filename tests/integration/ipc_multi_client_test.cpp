#include "pipe_client.h"

#include <Windows.h>

#include <array>
#include <atomic>
#include <barrier>
#include <cstdint>
#include <iostream>
#include <string>
#include <thread>
#include <vector>

namespace {

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

} // namespace

int wmain(int argc, wchar_t** argv) {
    constexpr std::size_t clientCount = 4;
    if (argc != 2) {
        std::cerr << "engine executable argument required\n";
        return 1;
    }
    const std::wstring suffix = std::to_wstring(GetCurrentProcessId());
    const std::wstring pipeName = L"\\\\.\\pipe\\Fcitx5WindowsNext.Multi." + suffix;
    const std::wstring readyEventName = L"Local\\Fcitx5WindowsNext.Multi.Ready." + suffix;
    HANDLE readyEvent = CreateEventW(nullptr, TRUE, FALSE, readyEventName.c_str());
    if (!readyEvent) return 1;
    std::wstring command = quote(argv[1]) + L" --test-clients 4 --pipe " + quote(pipeName) +
                           L" --ready-event " + quote(readyEventName);
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(argv[1], mutableCommand.data(), nullptr, nullptr, FALSE, CREATE_NO_WINDOW,
                        nullptr, nullptr, &startup, &process)) {
        CloseHandle(readyEvent);
        return 1;
    }
    CloseHandle(process.hThread);
    const bool ready = WaitForSingleObject(readyEvent, 2000) == WAIT_OBJECT_0;
    CloseHandle(readyEvent);

    std::barrier startBarrier(static_cast<std::ptrdiff_t>(clientCount));
    std::atomic<unsigned> succeeded{};
    std::array<std::thread, clientCount> clients;
    for (std::size_t index = 0; index < clientCount; ++index) {
        clients[index] = std::thread([&, index] {
            fcitx::windows::ipc::PipeClient client(
                pipeName, fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
            startBarrier.arrive_and_wait();
            fcitx::windows::ipc::KeyResult result;
            const auto key = static_cast<std::uint32_t>('A' + index);
            if (client.processKey(index + 1, key, 0, result) && result.handled &&
                result.commit == std::wstring(1, static_cast<wchar_t>(L'a' + index))) {
                succeeded.fetch_add(1, std::memory_order_relaxed);
            }
        });
    }
    for (auto& client : clients) client.join();

    int exitResult = ready && succeeded.load() == clientCount ? 0 : 1;
    if (WaitForSingleObject(process.hProcess, 2000) != WAIT_OBJECT_0) {
        TerminateProcess(process.hProcess, 2);
        WaitForSingleObject(process.hProcess, 1000);
        exitResult = 1;
    }
    DWORD engineExitCode = 1;
    GetExitCodeProcess(process.hProcess, &engineExitCode);
    CloseHandle(process.hProcess);
    if (engineExitCode != 0) exitResult = 1;
    if (exitResult != 0) std::cerr << "four-client concurrent IPC test failed\n";
    return exitResult;
}
