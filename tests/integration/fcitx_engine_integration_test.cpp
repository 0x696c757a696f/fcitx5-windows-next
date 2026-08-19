#include "pipe_client.h"
#include "runtime_identity.h"

#include <Windows.h>
#include <Psapi.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <chrono>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <random>
#include <sstream>
#include <string>
#include <thread>
#include <unordered_map>
#include <utility>
#include <vector>

namespace {

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

struct Process {
    HANDLE handle{};
    HANDLE stopEvent{};
    Process() = default;
    explicit Process(HANDLE value, HANDLE stop = nullptr) : handle(value), stopEvent(stop) {}
    Process(const Process&) = delete;
    Process& operator=(const Process&) = delete;
    Process(Process&& other) noexcept
        : handle(std::exchange(other.handle, nullptr)),
          stopEvent(std::exchange(other.stopEvent, nullptr)) {}
    Process& operator=(Process&& other) noexcept {
        if (this != &other) {
            if (handle) CloseHandle(handle);
            if (stopEvent) CloseHandle(stopEvent);
            handle = std::exchange(other.handle, nullptr);
            stopEvent = std::exchange(other.stopEvent, nullptr);
        }
        return *this;
    }
    ~Process() {
        if (stopEvent) SetEvent(stopEvent);
        if (!handle) {
            if (stopEvent) CloseHandle(stopEvent);
            return;
        }
        if (WaitForSingleObject(handle, 5000) != WAIT_OBJECT_0) {
            TerminateProcess(handle, 9);
            WaitForSingleObject(handle, 1000);
        }
        CloseHandle(handle);
        if (stopEvent) CloseHandle(stopEvent);
    }

    void requestStop() const {
        if (stopEvent) SetEvent(stopEvent);
    }
};

bool startEngine(const wchar_t* executable, unsigned sequence, bool safeMode,
                 unsigned testClientCount,
                 Process& process, const wchar_t* stderrPath = nullptr) {
    const std::wstring eventName =
        L"Local\\Fcitx5WindowsNext.RealEngine.Ready." +
        std::to_wstring(GetCurrentProcessId()) + L"." + std::to_wstring(sequence);
    const std::wstring stopEventName =
        L"Local\\Fcitx5WindowsNext.RealEngine.Stop." +
        std::to_wstring(GetCurrentProcessId()) + L"." + std::to_wstring(sequence);
    HANDLE ready = CreateEventW(nullptr, TRUE, FALSE, eventName.c_str());
    if (!ready) return false;
    HANDLE stop = testClientCount == 0
                      ? CreateEventW(nullptr, TRUE, FALSE, stopEventName.c_str())
                      : nullptr;
    if (testClientCount == 0 && !stop) {
        CloseHandle(ready);
        return false;
    }
    std::wstring command = quote(executable);
    if (testClientCount == 0)
        command += L" --stop-event " + quote(stopEventName);
    else
        command += L" --test-clients " + std::to_wstring(testClientCount);
    command += L" --ready-event " + quote(eventName);
    if (safeMode) command += L" --safe-mode";
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    SECURITY_ATTRIBUTES inheritAttributes{sizeof(inheritAttributes), nullptr, TRUE};
    HANDLE stderrFile = INVALID_HANDLE_VALUE;
    if (stderrPath) {
        startup.dwFlags = STARTF_USESTDHANDLES;
        startup.hStdOutput = GetStdHandle(STD_OUTPUT_HANDLE);
        stderrFile =
            CreateFileW(stderrPath, GENERIC_WRITE, FILE_SHARE_READ, &inheritAttributes,
                        OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
        if (stderrFile == INVALID_HANDLE_VALUE) {
            CloseHandle(ready);
            if (stop) CloseHandle(stop);
            return false;
        }
        SetFilePointer(stderrFile, 0, nullptr, FILE_BEGIN);
        SetEndOfFile(stderrFile);
        startup.hStdError = stderrFile;
    }
    PROCESS_INFORMATION information{};
    const bool created = CreateProcessW(executable, mutableCommand.data(), nullptr, nullptr,
                                        TRUE, CREATE_NO_WINDOW, nullptr, nullptr,
                                        &startup, &information) != FALSE;
    if (stderrFile != INVALID_HANDLE_VALUE) CloseHandle(stderrFile);
    if (!created) {
        std::cerr << "real engine creation failed: " << GetLastError() << '\n';
        CloseHandle(ready);
        if (stop) CloseHandle(stop);
        return false;
    }
    CloseHandle(information.hThread);
    process = Process(information.hProcess, stop);
    const bool signaled = WaitForSingleObject(ready, 15'000) == WAIT_OBJECT_0;
    CloseHandle(ready);
    if (!signaled) std::cerr << "real engine readiness timed out\n";
    return signaled;
}

bool runCandidateSelection(const std::filesystem::path& uiExecutable,
                           const wchar_t* engineExecutable,
                           std::uint64_t engineEpoch, std::uint64_t contextId,
                           std::uint64_t compositionId, std::uint64_t revision,
                           std::uint64_t candidateId) {
    std::wstring command = quote(uiExecutable.wstring()) + L" --candidate-select-test " +
                           quote(engineExecutable) + L" " +
                           std::to_wstring(GetCurrentProcessId()) + L" " +
                           std::to_wstring(engineEpoch) + L" " +
                           std::to_wstring(contextId) + L" " +
                           std::to_wstring(compositionId) + L" " +
                           std::to_wstring(revision) + L" " +
                           std::to_wstring(candidateId);
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION information{};
    if (!CreateProcessW(uiExecutable.c_str(), mutableCommand.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, nullptr, &startup, &information)) return false;
    CloseHandle(information.hThread);
    const bool stopped = WaitForSingleObject(information.hProcess, 5000) == WAIT_OBJECT_0;
    DWORD exitCode = UINT32_MAX;
    if (stopped) GetExitCodeProcess(information.hProcess, &exitCode);
    CloseHandle(information.hProcess);
    return stopped && exitCode == 0;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc < 2 || argc > 6) return 1;
    // An external runner (for example tools/test-candidate-ui.ps1) may already
    // have injected a shared FCITX5_TEST_NAMESPACE so that the engine and the
    // independent UI process listen on the same pipe namespace. Respect it;
    // otherwise default to a process-unique namespace as before.
    wchar_t injected[34]{};
    const DWORD injectedLength = GetEnvironmentVariableW(
        L"FCITX5_TEST_NAMESPACE", injected, static_cast<DWORD>(std::size(injected)));
    const std::wstring testNamespace =
        injectedLength > 0 && injectedLength < std::size(injected)
            ? std::wstring(injected, injectedLength)
            : L"engine-" + std::to_wstring(GetCurrentProcessId());
    if (!SetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", testNamespace.c_str())) return 1;
    bool safeMode = false;
    bool firstRunRime = false;
    bool rimeLua = false;
    bool typingFuzz = false;
    bool chttrans = false;
    for (int index = 2; index < argc; ++index) {
        const std::wstring_view argument(argv[index]);
        if (argument == L"--safe-mode") {
            safeMode = true;
        } else if (argument == L"--first-run-rime") {
            firstRunRime = true;
        } else if (argument == L"--rime-lua") {
            firstRunRime = true;
            rimeLua = true;
        } else if (argument == L"--typing-fuzz") {
            typingFuzz = true;
        } else if (argument == L"--chttrans") {
            chttrans = true;
        } else {
            return 1;
        }
    }
    if (safeMode && firstRunRime) return 1;
    Process process;
    const auto startupBegin = std::chrono::steady_clock::now();
    if (!startEngine(argv[1], 1, safeMode, typingFuzz ? 0U : 2U, process)) return 1;
    const auto startupDuration = std::chrono::steady_clock::now() - startupBegin;

    FILETIME creationBefore{}, exitBefore{}, kernelBefore{}, userBefore{};
    FILETIME creationAfter{}, exitAfter{}, kernelAfter{}, userAfter{};
    if (!GetProcessTimes(process.handle, &creationBefore, &exitBefore, &kernelBefore,
                         &userBefore)) return 1;
    const auto settleBegin = std::chrono::steady_clock::now();
    // Startup preloads every enabled input method addon (so Ctrl+Space /
    // Ctrl+Shift switching never pays a first-activation cost inside the
    // input deadline); allow the resulting one-time initialization CPU time
    // to settle before declaring the engine idle.
    const unsigned requiredQuietWindows = firstRunRime ? 20U : 3U;
    const unsigned maximumSamples = firstRunRime ? 1'200U : 150U;
    unsigned quietWindows = 0;
    for (unsigned sample = 0;
         sample < maximumSamples && quietWindows < requiredQuietWindows; ++sample) {
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
    if (quietWindows < requiredQuietWindows) {
        std::cerr << "engine did not reach a steady idle state within "
                  << maximumSamples / 10U << " seconds\n";
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

    constexpr std::uint64_t candidateContextId = 0x43414E4449444154ULL;
    if (!client.processKey(candidateContextId, 'N', 0, result) ||
        !client.processKey(candidateContextId, 'I', 0, result) ||
        result.candidates.empty() || result.compositionId == 0 || result.revision == 0) {
        std::cerr << "candidate selection setup failed\n";
        return 1;
    }
    const auto selectionSnapshot = result;
    const std::filesystem::path uiExecutable =
        std::filesystem::path(argv[1]).parent_path() / L"fcitx5-ui.exe";
    const std::wstring notificationName =
        fcitx::windows::platform::makeLocalObjectName(
            identity, L"candidate-" + std::to_wstring(GetCurrentProcessId()));
    HANDLE notification = CreateEventW(nullptr, FALSE, FALSE, notificationName.c_str());
    if (!notification ||
        !runCandidateSelection(uiExecutable, argv[1], selectionSnapshot.engineEpoch,
                               candidateContextId, selectionSnapshot.compositionId,
                               selectionSnapshot.revision,
                               selectionSnapshot.candidates.front().id) ||
        WaitForSingleObject(notification, 2000) != WAIT_OBJECT_0 ||
        !client.pollState(candidateContextId, result) || result.commit.empty() ||
        !result.preedit.empty()) {
        if (notification) CloseHandle(notification);
        std::cerr << "semantic candidate selection did not commit through Engine and TSF state\n";
        return 1;
    }
    CloseHandle(notification);
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

    if (chttrans) {
        constexpr std::uint64_t conversionContext = 0x4348545452414E53ULL;
        constexpr std::uint32_t conversionFlags =
            fcitx::windows::protocol::kKeyFlagControl |
            fcitx::windows::protocol::kKeyFlagShift;
        if (!client.processKey(conversionContext, 'F', conversionFlags, result) ||
            !result.handled) {
            std::cerr << "chttrans toggle hotkey was not handled\n";
            return 1;
        }
        for (const std::uint32_t key : {'S', 'H', 'U'}) {
            if (!client.processKey(conversionContext, key, 0, result) || !result.handled) {
                std::cerr << "chttrans pinyin input failed\n";
                return 1;
            }
        }
        if (!client.processKey(conversionContext, VK_SPACE, 0, result) ||
            result.commit != L"書") {
            std::wcerr << L"chttrans did not convert the committed word: "
                       << result.commit << L'\n';
            return 1;
        }
        if (!client.processKey(conversionContext, 'F', conversionFlags, result) ||
            !result.handled) {
            std::cerr << "chttrans restore hotkey was not handled\n";
            return 1;
        }
    }

    // Engine-side switch hotkeys must be reachable through the routing layer:
    // TSF now forwards modifier chords and modifier keys themselves, and the
    // engine decides. Ctrl+Space toggles active input method / keyboard
    // passthrough, Ctrl+Shift cycles the enabled list, and a key-up (release)
    // of the toggle chord must not switch again.
    {
        constexpr std::uint64_t hotkeyContext = 0x484F544B455953ULL;
        constexpr std::uint32_t ctrl = fcitx::windows::protocol::kKeyFlagControl;
        constexpr std::uint32_t ctrlRelease =
            ctrl | fcitx::windows::protocol::kKeyFlagRelease;
        // Toggle to keyboard passthrough: the chord is handled, a following
        // letter is not claimed by Fcitx.
        if (!client.processKey(hotkeyContext, VK_SPACE, ctrl, result) ||
            !result.handled) {
            std::cerr << "Ctrl+Space toggle hotkey was not handled\n";
            return 1;
        }
        if (!client.processKey(hotkeyContext, 'N', 0, result) || result.handled ||
            !result.preedit.empty()) {
            std::wcerr << L"toggle did not reach keyboard passthrough: preedit="
                       << result.preedit << L'\n';
            return 1;
        }
        // Releasing Ctrl+Space must not toggle back.
        (void)client.processKey(hotkeyContext, VK_SPACE, ctrlRelease, result);
        if (!client.processKey(hotkeyContext, 'N', 0, result) || result.handled) {
            std::wcerr << L"key-up of Ctrl+Space changed the input method\n";
            return 1;
        }
        // Toggle back to pinyin.
        if (!client.processKey(hotkeyContext, VK_SPACE, ctrl, result) ||
            !result.handled) {
            std::cerr << "Ctrl+Space restore hotkey was not handled\n";
            return 1;
        }
        // Ctrl+Shift cycles the enabled input method list. The Shift key-down
        // carries the modifier state of the chord (as TSF reports it: by the
        // time OnKeyDown(VK_SHIFT) runs, GetKeyState(VK_SHIFT) is already
        // down), so the request must include both modifiers.
        constexpr std::uint32_t ctrlShift =
            fcitx::windows::protocol::kKeyFlagControl |
            fcitx::windows::protocol::kKeyFlagShift;
        if (!client.processKey(hotkeyContext, VK_SHIFT, ctrlShift, result) ||
            !result.handled) {
            std::cerr << "Ctrl+Shift next hotkey was not handled\n";
            return 1;
        }
    }

    constexpr std::uint64_t secondContextId = 0x27182818U;
    if (firstRunRime) {
        if (!client.processKey(contextId, 'H', 0, result) || result.preedit != L"h") {
            std::wcerr << L"Rime first context start failed: " << result.preedit << L'\n';
            return 1;
        }
        if (!client.processKey(secondContextId, 'N', 0, result) || result.preedit != L"n") {
            std::wcerr << L"Rime second context start failed: " << result.preedit << L'\n';
            return 1;
        }
        if (!client.processKey(contextId, VK_BACK, 0, result) || !result.preedit.empty()) {
            std::wcerr << L"Rime first context clear failed: " << result.preedit << L'\n';
            return 1;
        }
        // Rime deliberately clears the inactive session when focus moves between
        // contexts. The new composition must start clean rather than leak "h" or "n".
        if (!client.processKey(secondContextId, 'I', 0, result) || result.preedit != L"i") {
            std::wcerr << L"Rime second context resume failed: " << result.preedit << L'\n';
            return 1;
        }
    } else {
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
    }

    if (rimeLua) {
        constexpr std::uint64_t luaContextId = 0x14142135U;
        for (const std::uint32_t key : {'Z', 'Z', 'L', 'U', 'A'}) {
            if (!client.processKey(luaContextId, key, 0, result) || !result.handled) {
                std::cerr << "Rime Lua probe input failed\n";
                return 1;
            }
        }
        const auto luaCandidate = std::ranges::find_if(
            result.candidates,
            [](const auto& candidate) { return candidate.text == L"Lua Works"; });
        if (luaCandidate == result.candidates.end()) {
            std::wcerr << L"Rime Lua translator did not produce its probe candidate\n";
            return 1;
        }
        const auto luaIndex = static_cast<std::size_t>(
            std::distance(result.candidates.begin(), luaCandidate));
        if (luaIndex >= 9U ||
            !client.processKey(luaContextId, static_cast<std::uint32_t>('1' + luaIndex), 0,
                               result) ||
            !result.handled ||
            result.commit != L"Lua Works") {
            std::wcerr << L"Rime Lua candidate did not commit: " << result.commit << L'\n';
            return 1;
        }
    }

    if (typingFuzz) {
        constexpr std::uint64_t reconnectContextId = 0x42424242U;
        {
            fcitx::windows::ipc::PipeClient abandoned(
                fcitx::windows::platform::makeLocalEndpointName(identity, L"engine"),
                fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
            if (!abandoned.processKey(reconnectContextId, 'H', 0, result) ||
                result.preedit != L"h") {
                std::cerr << "disconnect recovery setup failed\n";
                return 1;
            }
        }
        fcitx::windows::ipc::PipeClient recovered(
            fcitx::windows::platform::makeLocalEndpointName(identity, L"engine"),
            fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
        if (!recovered.processKey(reconnectContextId, 'N', 0, result) ||
            result.preedit != L"n") {
            std::wcerr << L"same-epoch reconnect retained stale composition: "
                       << result.preedit << L'\n';
            return 1;
        }
        recovered.disconnect();
    }

    if (typingFuzz) {
        constexpr std::array<std::uint32_t, 48> commonKeys{
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
            'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X',
            'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
            VK_SPACE, VK_BACK, VK_RETURN, VK_ESCAPE, VK_LEFT, VK_RIGHT,
            VK_UP, VK_DOWN, VK_PRIOR, VK_NEXT, 0U, UINT32_MAX};
        constexpr unsigned longStringRounds = 300;
        constexpr std::uint64_t longStringContext = 0x48414841U;
        constexpr std::array<std::uint32_t, 3> longStringKeys{
            static_cast<std::uint32_t>('H'), static_cast<std::uint32_t>('A'),
            static_cast<std::uint32_t>(VK_SPACE)};
        std::vector<std::int64_t> longStringLatency;
        longStringLatency.reserve(longStringRounds * longStringKeys.size());
        for (unsigned round = 0; round < longStringRounds; ++round) {
            for (const std::uint32_t key : longStringKeys) {
                const auto keyStart = std::chrono::steady_clock::now();
                if (!client.processKey(longStringContext, key, 0, result)) {
                    std::cerr << "continuous ha typing smoke failed at round " << round
                              << " key=0x" << std::hex << key << std::dec << '\n';
                    return 1;
                }
                longStringLatency.push_back(
                    std::chrono::duration_cast<std::chrono::microseconds>(
                        std::chrono::steady_clock::now() - keyStart)
                        .count());
            }
            if (result.commit.empty() || !result.preedit.empty()) {
                std::cerr << "continuous ha typing did not commit cleanly at round "
                          << round << '\n';
                return 1;
            }
        }
        std::ranges::sort(longStringLatency);
        const auto percentile = [&](std::size_t value) {
            return longStringLatency[(longStringLatency.size() - 1U) * value / 100U];
        };
        std::cout << "continuous-typing-rounds=" << longStringRounds
                  << " keys=" << longStringRounds * 3U
                  << " p50-us=" << percentile(50U)
                  << " p95-us=" << percentile(95U)
                  << " p99-us=" << percentile(99U)
                  << " max-us=" << longStringLatency.back() << '\n';

        constexpr unsigned iterations = 4'000;
        constexpr std::uint64_t contextBase = 0xF0220000U;
        std::mt19937_64 random(0x46575F545950494EULL);
        std::unordered_map<std::uint64_t, bool> activeContexts;
        unsigned recoveredTransportFailures = 0;
        const auto fuzzStart = std::chrono::steady_clock::now();
        for (unsigned index = 0; index < iterations; ++index) {
            const std::uint64_t context = contextBase + (random() % 16U);
            const bool active = activeContexts[context];
            const std::size_t reachableKeyCount = active ? commonKeys.size() - 2U : 36U;
            const std::uint32_t key = commonKeys[random() % reachableKeyCount];
            // This is a stateful typing fuzz, so generate only modifier states that
            // ordinary printable typing can reach. Control hotkeys have dedicated
            // functional tests and can trigger intentionally expensive on-demand addons.
            const std::uint32_t flags =
                (random() & 1U) == 0 ? 0U : fcitx::windows::protocol::kKeyFlagShift;
            if (!client.processKey(context, key, flags, result)) {
                ++recoveredTransportFailures;
                activeContexts.clear();
                constexpr std::uint64_t recoveryContext = contextBase + 100U;
                if (!client.processKey(recoveryContext, 'N', 0, result) ||
                    result.preedit != L"n") {
                    std::cerr << "typing fuzz did not recover after transport failure at "
                              << index << " key=0x" << std::hex << key << std::dec << '\n';
                    return 1;
                }
                activeContexts[recoveryContext] = true;
                continue;
            }
            if (result.preedit.size() > fcitx::windows::protocol::kMaxPreeditUtf8 ||
                result.commit.size() > fcitx::windows::protocol::kMaxCommitUtf8 ||
                result.candidates.size() > fcitx::windows::protocol::kMaxCandidates ||
                result.candidateVisibility > 2U ||
                (result.selectedCandidate != UINT32_MAX &&
                 result.selectedCandidate >= result.candidates.size())) {
                std::cerr << "typing fuzz response invariant failed at iteration "
                          << index << '\n';
                return 1;
            }
            activeContexts[context] = !result.preedit.empty() || !result.candidates.empty();
            if ((index % 64U) == 63U) {
                for (unsigned cleanup = 0; cleanup < 16U; ++cleanup) {
                    if (!client.processKey(contextBase + cleanup, VK_ESCAPE, 0, result)) {
                        std::cerr << "typing fuzz context cleanup failed\n";
                        return 1;
                    }
                    activeContexts[contextBase + cleanup] = false;
                }
            }
        }
        if (recoveredTransportFailures > iterations / 100U) {
            std::cerr << "typing fuzz transport failure rate exceeded 1%: "
                      << recoveredTransportFailures << '/' << iterations << '\n';
            return 1;
        }
        const auto fuzzElapsed = std::chrono::steady_clock::now() - fuzzStart;
        std::cout << "typing-fuzz-seed=0x46575f545950494e iterations="
                  << iterations << " recovered-failures=" << recoveredTransportFailures
                  << " elapsed-ms="
                  << std::chrono::duration_cast<std::chrono::milliseconds>(fuzzElapsed).count()
                  << '\n';
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
    if (typingFuzz) process.requestStop();
    if (WaitForSingleObject(process.handle, 5000) != WAIT_OBJECT_0) {
        std::cerr << "real engine did not stop after test client disconnected\n";
        return 1;
    }
    DWORD exitCode = 1;
    GetExitCodeProcess(process.handle, &exitCode);
    if (exitCode != 0 || firstEpoch == 0) return 1;

    Process restarted;
    if (!startEngine(argv[1], 2, safeMode, 1U, restarted)) return 1;
    fcitx::windows::ipc::PipeClient restartedClient(
        fcitx::windows::platform::makeLocalEndpointName(identity, L"engine"),
        fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
    const bool restartKeyOk = restartedClient.processKey(contextId, 'N', 0, result);
    if (!restartKeyOk || result.engineEpoch <= firstEpoch) {
        std::wcerr << L"engine restart did not advance epoch: ipc=" << restartKeyOk
                   << L" handled=" << result.handled << L" preedit=" << result.preedit
                   << L" first-epoch=" << firstEpoch
                   << L" restart-epoch=" << result.engineEpoch << L'\n';
        return 1;
    }
    restartedClient.disconnect();
    if (WaitForSingleObject(restarted.handle, 5000) != WAIT_OBJECT_0) return 1;
    GetExitCodeProcess(restarted.handle, &exitCode);
    if (exitCode != 0) return 1;

    // REG-DISPATCH-LATE: a key request that times out must never execute late.
    // Start an engine whose FIRST dispatcher task is stalled past the client
    // deadline; the client gives up, then the stalled task must be dropped at
    // its deadline check instead of touching Fcitx state. The engine reports
    // how many tasks it dropped on exit; the assertion below requires at
    // least one drop, and a subsequent key on the same engine still works.
    {
        const std::filesystem::path stderrPath =
            std::filesystem::temp_directory_path() /
            (L"fcitx5-engine-late-" + std::to_wstring(GetCurrentProcessId()) + L".log");
        SetEnvironmentVariableW(L"FCITX5_TEST_DISPATCH_DELAY_MS", L"800");
        Process late;
        const bool lateStarted =
            startEngine(argv[1], 3, safeMode, 0U, late, stderrPath.c_str());
        SetEnvironmentVariableW(L"FCITX5_TEST_DISPATCH_DELAY_MS", nullptr);
        if (!lateStarted) {
            std::cerr << "late-engine start failed\n";
            return 1;
        }
        fcitx::windows::ipc::PipeClient lateClient(
            fcitx::windows::platform::makeLocalEndpointName(identity, L"engine"),
            fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
        // The first dispatcher task sleeps 800 ms while the client deadline is
        // 100 ms, so this request must time out at the client.
        const std::uint64_t lateContextId = 0x4C4154454B4559ULL;
        const bool lateFirst = lateClient.processKey(lateContextId, 'N', 0, result);
        if (lateFirst) {
            std::cerr << "stalled dispatcher task did not time out the client\n";
            return 1;
        }
        // Give the stalled task time to reach its deadline check and be
        // dropped, then verify the engine still serves keys normally.
        std::this_thread::sleep_for(std::chrono::milliseconds(1500));
        const bool lateSecond = lateClient.processKey(lateContextId, 'N', 0, result);
        if (!lateSecond || !result.handled || result.preedit != L"n") {
            std::wcerr << L"engine unhealthy after dropped late key: ipc=" << lateSecond
                       << L" handled=" << result.handled << L" preedit=" << result.preedit
                       << L'\n';
            return 1;
        }
        lateClient.disconnect();
        late.requestStop();
        if (WaitForSingleObject(late.handle, 5000) != WAIT_OBJECT_0) {
            TerminateProcess(late.handle, 9);
            WaitForSingleObject(late.handle, 1000);
        }
        DWORD lateExit = 1;
        GetExitCodeProcess(late.handle, &lateExit);
        std::string lateStderr;
        {
            std::ifstream log(stderrPath, std::ios::binary);
            std::ostringstream buffer;
            buffer << log.rdbuf();
            lateStderr = buffer.str();
        }
        (void)DeleteFileW(stderrPath.c_str());
        if (lateExit != 0 || lateStderr.find("dispatcher-dropped=") == std::string::npos) {
            std::cerr << "late engine did not report dropped count (exit=" << lateExit
                      << ")\n";
            return 1;
        }
        const auto marker = lateStderr.find("dispatcher-dropped=");
        const auto dropped =
            std::strtoull(lateStderr.c_str() + marker + std::strlen("dispatcher-dropped="),
                          nullptr, 10);
        if (dropped == 0) {
            std::cerr << "stalled key was executed instead of dropped\n";
            return 1;
        }
        std::cout << "dispatcher-dropped=" << dropped << '\n';
    }
    return 0;
}
