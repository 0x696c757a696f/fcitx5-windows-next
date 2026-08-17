#include "pipe_client.h"
#include "runtime_identity.h"

#include <Windows.h>
#include <Psapi.h>

#include <cstdint>
#include <chrono>
#include <iostream>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace {

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

struct Process {
    HANDLE handle{};
    Process() = default;
    explicit Process(HANDLE value) : handle(value) {}
    Process(const Process&) = delete;
    Process& operator=(const Process&) = delete;
    Process(Process&& other) noexcept : handle(std::exchange(other.handle, nullptr)) {}
    Process& operator=(Process&& other) noexcept {
        if (this != &other) {
            if (handle) CloseHandle(handle);
            handle = std::exchange(other.handle, nullptr);
        }
        return *this;
    }
    ~Process() {
        if (!handle) return;
        if (WaitForSingleObject(handle, 5000) != WAIT_OBJECT_0) {
            TerminateProcess(handle, 9);
            WaitForSingleObject(handle, 1000);
        }
        CloseHandle(handle);
    }
};

bool startEngine(const wchar_t* executable, unsigned sequence, bool safeMode,
                 Process& process) {
    const std::wstring eventName =
        L"Local\\Fcitx5WindowsNext.RealEngine.Ready." +
        std::to_wstring(GetCurrentProcessId()) + L"." + std::to_wstring(sequence);
    HANDLE ready = CreateEventW(nullptr, TRUE, FALSE, eventName.c_str());
    if (!ready) return false;
    std::wstring command = quote(executable) + L" --test-once --ready-event " +
                           quote(eventName);
    if (safeMode) command += L" --safe-mode";
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION information{};
    const bool created = CreateProcessW(executable, mutableCommand.data(), nullptr, nullptr,
                                        FALSE, CREATE_NO_WINDOW, nullptr, nullptr,
                                        &startup, &information) != FALSE;
    if (!created) {
        std::cerr << "real engine creation failed: " << GetLastError() << '\n';
        CloseHandle(ready);
        return false;
    }
    CloseHandle(information.hThread);
    process = Process(information.hProcess);
    const bool signaled = WaitForSingleObject(ready, 15'000) == WAIT_OBJECT_0;
    CloseHandle(ready);
    if (!signaled) std::cerr << "real engine readiness timed out\n";
    return signaled;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2 && argc != 3) return 1;
    const bool safeMode = argc == 3 && std::wstring_view(argv[2]) == L"--safe-mode";
    if (argc == 3 && !safeMode) return 1;
    Process process;
    const auto startupBegin = std::chrono::steady_clock::now();
    if (!startEngine(argv[1], 1, safeMode, process)) return 1;
    const auto startupDuration = std::chrono::steady_clock::now() - startupBegin;

    FILETIME creationBefore{}, exitBefore{}, kernelBefore{}, userBefore{};
    FILETIME creationAfter{}, exitAfter{}, kernelAfter{}, userAfter{};
    if (!GetProcessTimes(process.handle, &creationBefore, &exitBefore, &kernelBefore,
                         &userBefore)) return 1;
    const auto settleBegin = std::chrono::steady_clock::now();
    unsigned quietWindows = 0;
    for (unsigned sample = 0; sample < 50 && quietWindows < 3; ++sample) {
        if (WaitForSingleObject(process.handle, 100) != WAIT_TIMEOUT) return 1;
        if (!GetProcessTimes(process.handle, &creationAfter, &exitAfter, &kernelAfter,
                             &userAfter)) return 1;
        ULARGE_INTEGER previousKernel{}, currentKernel{}, previousUser{}, currentUser{};
        previousKernel.LowPart = kernelBefore.dwLowDateTime;
        previousKernel.HighPart = kernelBefore.dwHighDateTime;
        currentKernel.LowPart = kernelAfter.dwLowDateTime;
        currentKernel.HighPart = kernelAfter.dwHighDateTime;
        previousUser.LowPart = userBefore.dwLowDateTime;
        previousUser.HighPart = userBefore.dwHighDateTime;
        currentUser.LowPart = userAfter.dwLowDateTime;
        currentUser.HighPart = userAfter.dwHighDateTime;
        const auto cpu100ns = (currentKernel.QuadPart - previousKernel.QuadPart) +
                              (currentUser.QuadPart - previousUser.QuadPart);
        quietWindows = cpu100ns <= 50'000U ? quietWindows + 1 : 0;
        kernelBefore = kernelAfter;
        userBefore = userAfter;
    }
    if (quietWindows < 3) {
        std::cerr << "engine did not reach a steady idle state within 5 seconds\n";
        return 1;
    }
    const auto settleDuration = std::chrono::steady_clock::now() - settleBegin;
    PROCESS_MEMORY_COUNTERS_EX memory{};
    memory.cb = sizeof(memory);
    if (!GetProcessMemoryInfo(process.handle,
                              reinterpret_cast<PROCESS_MEMORY_COUNTERS*>(&memory),
                              sizeof(memory))) return 1;
    if (WaitForSingleObject(process.handle, 1000) != WAIT_TIMEOUT) return 1;
    if (!GetProcessTimes(process.handle, &creationAfter, &exitAfter, &kernelAfter,
                         &userAfter)) return 1;
    ULARGE_INTEGER kernelStart{}, kernelEnd{}, userStart{}, userEnd{};
    kernelStart.LowPart = kernelBefore.dwLowDateTime;
    kernelStart.HighPart = kernelBefore.dwHighDateTime;
    kernelEnd.LowPart = kernelAfter.dwLowDateTime;
    kernelEnd.HighPart = kernelAfter.dwHighDateTime;
    userStart.LowPart = userBefore.dwLowDateTime;
    userStart.HighPart = userBefore.dwHighDateTime;
    userEnd.LowPart = userAfter.dwLowDateTime;
    userEnd.HighPart = userAfter.dwHighDateTime;
    const auto idleCpu100ns = (kernelEnd.QuadPart - kernelStart.QuadPart) +
                              (userEnd.QuadPart - userStart.QuadPart);
    if (idleCpu100ns > 500'000U) {
        std::cerr << "engine idle CPU exceeded 50 ms per second: "
                  << idleCpu100ns / 10U << " us\n";
        return 1;
    }

    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity)) return 1;
    fcitx::windows::ipc::PipeClient client(
        fcitx::windows::platform::makeLocalEndpointName(identity, L"engine"),
        fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
    fcitx::windows::ipc::KeyResult result;
    constexpr std::uint64_t contextId = 0x31415926U;
    const auto firstStart = std::chrono::steady_clock::now();
    const bool firstOk = client.processKey(contextId, 'N', 0, result);
    const auto firstDuration = std::chrono::steady_clock::now() - firstStart;
    if (!firstOk || !result.handled ||
        result.preedit != L"n" || !result.commit.empty()) {
        std::wcerr << L"first pinyin key failed: ipc=" << firstOk
                   << L" handled=" << result.handled << L" preedit="
                   << result.preedit << L" commit=" << result.commit << L'\n';
        if (WaitForSingleObject(process.handle, 0) == WAIT_OBJECT_0) {
            DWORD code = 0;
            GetExitCodeProcess(process.handle, &code);
            std::cerr << "engine exited early with code " << code << '\n';
        }
        return 1;
    }
    const auto secondStart = std::chrono::steady_clock::now();
    if (!client.processKey(contextId, 'I', 0, result) || !result.handled ||
        result.preedit != L"ni" || !result.commit.empty() ||
        result.candidates.empty() || result.candidateVisibility != 1) {
        std::wcerr << L"second pinyin key failed: preedit=" << result.preedit << L'\n';
        return 1;
    }
    const auto secondDuration = std::chrono::steady_clock::now() - secondStart;
    const auto commitStart = std::chrono::steady_clock::now();
    if (!client.processKey(contextId, VK_SPACE, 0, result) || !result.handled ||
        result.commit.empty() || !result.preedit.empty()) {
        std::wcerr << L"pinyin commit failed: commit=" << result.commit
                   << L" preedit=" << result.preedit << L'\n';
        return 1;
    }
    const auto commitDuration = std::chrono::steady_clock::now() - commitStart;
    std::cout << "engine-startup-ms="
              << std::chrono::duration_cast<std::chrono::milliseconds>(startupDuration).count()
              << " idle-cpu-us=" << idleCpu100ns / 10U
              << " settle-ms="
              << std::chrono::duration_cast<std::chrono::milliseconds>(settleDuration).count()
              << " private-kib=" << memory.PrivateUsage / 1024U
              << " context-start-us="
              << std::chrono::duration_cast<std::chrono::microseconds>(firstDuration).count()
              << " hot-key-us="
              << std::chrono::duration_cast<std::chrono::microseconds>(secondDuration).count()
              << " commit-us="
              << std::chrono::duration_cast<std::chrono::microseconds>(commitDuration).count()
              << '\n';

    constexpr std::uint64_t secondContextId = 0x27182818U;
    if (!client.processKey(secondContextId, 'H', 0, result) ||
        result.preedit != L"h") {
        std::wcerr << L"second context did not start independently\n";
        return 1;
    }
    if (!client.processKey(contextId, 'H', 0, result) || result.preedit != L"h") {
        std::wcerr << L"first context retained state from second context: preedit="
                   << result.preedit << L" commit=" << result.commit << L'\n';
        return 1;
    }
    if (!client.processKey(secondContextId, 'A', 0, result) ||
        result.preedit != L"a" || result.commit != L"h") {
        std::wcerr << L"second context state was not preserved: "
                   << result.preedit << L" commit=" << result.commit << L'\n';
        return 1;
    }
    if (!client.processKey(contextId, 'A', 0, result) ||
        result.preedit != L"a" || result.commit != L"h") {
        std::wcerr << L"first context received state from second context\n";
        return 1;
    }

    constexpr std::uint64_t repeatContextId = 0x16180339U;
    constexpr unsigned repeatCount = 120;
    constexpr auto repeatPeriod = std::chrono::microseconds(16'667);
    const auto repeatStart = std::chrono::steady_clock::now();
    auto deadline = repeatStart;
    for (unsigned index = 0; index < repeatCount; ++index) {
        deadline += repeatPeriod;
        const auto virtualKey = (index & 1U) == 0 ? 'N' : VK_BACK;
        const bool repeatOk = client.processKey(repeatContextId, virtualKey, 0, result);
        if (!repeatOk || !result.handled) {
            std::cerr << "60 Hz key-repeat request failed at " << index
                      << " ipc=" << repeatOk << " handled=" << result.handled << '\n';
            return 1;
        }
        std::this_thread::sleep_until(deadline);
    }
    const auto repeatElapsed = std::chrono::steady_clock::now() - repeatStart;
    if (repeatElapsed > repeatPeriod * (repeatCount + 6U)) {
        std::cerr << "60 Hz key-repeat accumulated backlog\n";
        return 1;
    }
    std::cout << "key-repeat-count=" << repeatCount << " elapsed-ms="
              << std::chrono::duration_cast<std::chrono::milliseconds>(repeatElapsed).count()
              << '\n';
    const std::uint64_t firstEpoch = result.engineEpoch;
    client.disconnect();
    if (WaitForSingleObject(process.handle, 5000) != WAIT_OBJECT_0) {
        std::cerr << "real engine did not stop after test client disconnected\n";
        return 1;
    }
    DWORD exitCode = 1;
    GetExitCodeProcess(process.handle, &exitCode);
    if (exitCode != 0 || firstEpoch == 0) return 1;

    Process restarted;
    if (!startEngine(argv[1], 2, safeMode, restarted)) return 1;
    fcitx::windows::ipc::PipeClient restartedClient(
        fcitx::windows::platform::makeLocalEndpointName(identity, L"engine"),
        fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
    if (!restartedClient.processKey(contextId, 'N', 0, result) ||
        result.engineEpoch <= firstEpoch) {
        std::cerr << "engine restart did not advance epoch\n";
        return 1;
    }
    restartedClient.disconnect();
    if (WaitForSingleObject(restarted.handle, 5000) != WAIT_OBJECT_0) return 1;
    GetExitCodeProcess(restarted.handle, &exitCode);
    return exitCode == 0 ? 0 : 1;
}
