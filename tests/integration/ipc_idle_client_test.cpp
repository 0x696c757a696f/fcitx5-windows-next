// Idle-client IPC test: connections that are established but idle (no key
// traffic) must not starve other clients' requests. The engine runs a pool of
// pipe workers; this test holds several idle connections open while a second
// batch of clients sends keys concurrently and verifies all of them succeed.
#include "pipe_client.h"

#include <Windows.h>

#include <atomic>
#include <barrier>
#include <cstdint>
#include <iostream>
#include <memory>
#include <string>
#include <thread>
#include <vector>

namespace {

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

} // namespace

int wmain(int argc, wchar_t** argv) {
    constexpr std::size_t idleCount = 8U;
    constexpr std::size_t activeCount = 8U;
    constexpr std::size_t totalCount = idleCount + activeCount;
    if (argc != 2) {
        std::cerr << "engine executable argument required\n";
        return 1;
    }
    const std::wstring suffix = std::to_wstring(GetCurrentProcessId());
    const std::wstring pipeName = L"\\\\.\\pipe\\Fcitx5WindowsNext.Idle." + suffix;
    const std::wstring readyEventName = L"Local\\Fcitx5WindowsNext.Idle.Ready." + suffix;
    HANDLE readyEvent = CreateEventW(nullptr, TRUE, FALSE, readyEventName.c_str());
    if (!readyEvent) return 1;
    std::wstring command = quote(argv[1]) + L" --test-clients " +
                           std::to_wstring(totalCount) + L" --pipe " + quote(pipeName) +
                           L" --ready-event " + quote(readyEventName);
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(argv[1], mutableCommand.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process)) {
        CloseHandle(readyEvent);
        return 1;
    }
    CloseHandle(process.hThread);
    const bool ready = WaitForSingleObject(readyEvent, 2000) == WAIT_OBJECT_0;
    CloseHandle(readyEvent);

    // Open the idle connections first and leave them established while the
    // active clients run. Each idle client completes one key transaction
    // (which establishes the pipe connection) and then stays alive without
    // further traffic, holding its worker while the active batch runs.
    std::vector<std::unique_ptr<fcitx::windows::ipc::PipeClient>> idleClients;
    idleClients.reserve(idleCount);
    bool idleOk = ready;
    for (std::size_t index = 0; index < idleCount; ++index) {
        fcitx::windows::ipc::KeyResult result;
        const auto key = static_cast<std::uint32_t>('Z' - (index % 26U));
        auto client = std::make_unique<fcitx::windows::ipc::PipeClient>(
            pipeName, fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
        if (!client->processKey(0x40000000ULL + index, key, 0, result) || !result.handled) {
            idleOk = false;
            break;
        }
        idleClients.push_back(std::move(client));
    }

    std::barrier startBarrier(static_cast<std::ptrdiff_t>(activeCount));
    std::atomic<unsigned> succeeded{};
    std::vector<std::thread> active;
    active.reserve(activeCount);
    for (std::size_t index = 0; index < activeCount; ++index) {
        active.emplace_back([&, index] {
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
    for (auto& thread : active) thread.join();
    // Release the idle connections so the engine's completed-client count
    // reaches the configured total and it can exit normally.
    idleClients.clear();

    int exitResult = idleOk && succeeded.load() == activeCount ? 0 : 1;
    if (WaitForSingleObject(process.hProcess, 3000) != WAIT_OBJECT_0) {
        TerminateProcess(process.hProcess, 2);
        WaitForSingleObject(process.hProcess, 1000);
        exitResult = 1;
    }
    DWORD engineExitCode = 1;
    GetExitCodeProcess(process.hProcess, &engineExitCode);
    CloseHandle(process.hProcess);
    if (engineExitCode != 0) exitResult = 1;
    if (exitResult != 0) std::cerr << "idle-client IPC stress failed\n";
    return exitResult;
}
