#include "pipe_client.h"

#include <Windows.h>

#include <atomic>
#include <barrier>
#include <cstdint>
#include <cwchar>
#include <iostream>
#include <memory>
#include <string>
#include <thread>
#include <vector>

namespace {

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

} // namespace

int wmain(int argc, wchar_t** argv) {
    const std::size_t clientCount =
        argc == 3 ? static_cast<std::size_t>(std::wcstoul(argv[2], nullptr, 10)) : 4U;
    if (clientCount == 0 || clientCount > 64U) {
        std::cerr << "client count must be in [1, 64]\n";
        return 1;
    }
    if (argc < 2) {
        std::cerr << "engine executable argument required\n";
        return 1;
    }
    const std::wstring suffix = std::to_wstring(GetCurrentProcessId());
    const std::wstring pipeName = L"\\\\.\\pipe\\Fcitx5WindowsNext.Multi." + suffix;
    const std::wstring readyEventName = L"Local\\Fcitx5WindowsNext.Multi.Ready." + suffix;
    HANDLE readyEvent = CreateEventW(nullptr, TRUE, FALSE, readyEventName.c_str());
    if (!readyEvent) return 1;
    std::wstring command = quote(argv[1]) + L" --test-clients " +
                           std::to_wstring(clientCount) + L" --pipe " + quote(pipeName) +
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
    const bool ready = WaitForSingleObject(readyEvent, 15'000) == WAIT_OBJECT_0;
    CloseHandle(readyEvent);

    // Establish all connections sequentially first (each client completes one
    // key to bring its pipe up), then fire the second key from every client
    // concurrently. This stresses N established clients issuing requests at
    // once without every client racing its first connection inside the
    // 100 ms input deadline.
    std::vector<std::unique_ptr<fcitx::windows::ipc::PipeClient>> clients;
    clients.reserve(clientCount);
    bool connected = ready;
    for (std::size_t index = 0; index < clientCount; ++index) {
        auto client = std::make_unique<fcitx::windows::ipc::PipeClient>(
            pipeName, fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
        fcitx::windows::ipc::KeyResult result;
        const auto key = static_cast<std::uint32_t>('A' + (index % 26U));
        if (!client->processKey(index + 1, key, 0, result) || !result.handled) {
            connected = false;
            break;
        }
        clients.push_back(std::move(client));
    }
    if (!connected || clients.size() != clientCount) {
        clients.clear();
        std::cerr << "concurrent IPC stress failed for " << clientCount
                  << " clients (connection setup)\n";
        TerminateProcess(process.hProcess, 2);
        WaitForSingleObject(process.hProcess, 1000);
        CloseHandle(process.hProcess);
        return 1;
    }

    std::barrier startBarrier(static_cast<std::ptrdiff_t>(clientCount));
    std::atomic<unsigned> succeeded{};
    std::vector<std::thread> active;
    active.reserve(clientCount);
    for (std::size_t index = 0; index < clientCount; ++index) {
        active.emplace_back([&, index] {
            startBarrier.arrive_and_wait();
            fcitx::windows::ipc::KeyResult result;
            const auto key = static_cast<std::uint32_t>('A' + (index % 26U));
            if (clients[index]->processKey(0x10000000ULL + index, key, 0, result) &&
                result.handled &&
                result.commit == std::wstring(1, static_cast<wchar_t>(L'a' + (index % 26U)))) {
                succeeded.fetch_add(1, std::memory_order_relaxed);
            }
        });
    }
    for (auto& thread : active) thread.join();
    // Release the connections so the engine's completed-client count reaches
    // the configured total and it can exit normally.
    clients.clear();

    int exitResult = connected && succeeded.load() == clientCount ? 0 : 1;
    if (WaitForSingleObject(process.hProcess, 2000) != WAIT_OBJECT_0) {
        TerminateProcess(process.hProcess, 2);
        WaitForSingleObject(process.hProcess, 1000);
        exitResult = 1;
    }
    DWORD engineExitCode = 1;
    GetExitCodeProcess(process.hProcess, &engineExitCode);
    CloseHandle(process.hProcess);
    if (engineExitCode != 0) exitResult = 1;
    if (exitResult != 0)
        std::cerr << "concurrent IPC stress failed for " << clientCount << " clients\n";
    return exitResult;
}
