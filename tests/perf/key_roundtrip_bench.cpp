#include "pipe_client.h"

#include <Windows.h>

#include <algorithm>
#include <cstdint>
#include <iostream>
#include <string>
#include <vector>

namespace {

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

double percentile(const std::vector<double>& sorted, std::size_t numerator) {
    const std::size_t index = ((sorted.size() - 1) * numerator) / 100;
    return sorted[index];
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    constexpr std::size_t warmupCount = 100;
    constexpr std::size_t sampleCount = 2'000;
    if (argc != 2) {
        std::cerr << "engine executable argument required\n";
        return 1;
    }

    const std::wstring suffix = std::to_wstring(GetCurrentProcessId());
    const std::wstring pipeName = L"\\\\.\\pipe\\Fcitx5WindowsNext.Bench." + suffix;
    const std::wstring readyEventName = L"Local\\Fcitx5WindowsNext.Bench.Ready." + suffix;
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
        CloseHandle(readyEvent);
        std::cerr << "failed to start mock engine\n";
        return 1;
    }
    CloseHandle(process.hThread);
    const bool ready = WaitForSingleObject(readyEvent, 2000) == WAIT_OBJECT_0;
    CloseHandle(readyEvent);

    LARGE_INTEGER frequency{};
    QueryPerformanceFrequency(&frequency);
    std::vector<double> microseconds;
    microseconds.reserve(sampleCount);
    bool succeeded = ready;
    {
        fcitx::windows::ipc::PipeClient client(
            pipeName, fcitx::windows::ipc::PeerPolicy::exact(argv[1]));
        fcitx::windows::ipc::KeyResult result;
        for (std::size_t index = 0; succeeded && index < warmupCount + sampleCount; ++index) {
            LARGE_INTEGER started{};
            LARGE_INTEGER finished{};
            QueryPerformanceCounter(&started);
            succeeded = client.processKey(7, 'A', 0, result) && result.handled &&
                        result.commit == L"a";
            QueryPerformanceCounter(&finished);
            if (index >= warmupCount) {
                microseconds.push_back(
                    static_cast<double>(finished.QuadPart - started.QuadPart) * 1'000'000.0 /
                    static_cast<double>(frequency.QuadPart));
            }
        }
    }

    if (WaitForSingleObject(process.hProcess, 2000) != WAIT_OBJECT_0) {
        TerminateProcess(process.hProcess, 2);
        WaitForSingleObject(process.hProcess, 1000);
        succeeded = false;
    }
    DWORD engineExitCode = 1;
    GetExitCodeProcess(process.hProcess, &engineExitCode);
    CloseHandle(process.hProcess);
    if (!succeeded || engineExitCode != 0 || microseconds.size() != sampleCount) {
        std::cerr << "roundtrip benchmark failed\n";
        return 1;
    }

    std::sort(microseconds.begin(), microseconds.end());
    std::cout << "{\"benchmark\":\"key_roundtrip\",\"architecture_bits\":"
              << sizeof(void*) * 8 << ",\"samples\":" << sampleCount
              << ",\"p50_us\":" << percentile(microseconds, 50)
              << ",\"p95_us\":" << percentile(microseconds, 95)
              << ",\"p99_us\":" << percentile(microseconds, 99)
              << ",\"max_us\":" << microseconds.back() << "}\n";
    return 0;
}
