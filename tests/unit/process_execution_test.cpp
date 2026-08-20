// Pipe-deadlock regression test for config::runExecutable: a child that emits
// more than the pipe buffer must be drained concurrently while the parent
// waits, otherwise the child blocks in WriteFile and the parent blocks in
// WaitForSingleObject until the timeout kills both.
#include "process_execution.h"

#include <Windows.h>

#include <filesystem>
#include <iostream>
#include <string>
#include <thread>
#include <vector>

namespace {

int fail(const std::string& message) {
    std::cerr << message << '\n';
    return 1;
}

} // namespace

int wmain() {
    wchar_t systemDirectory[MAX_PATH]{};
    if (!GetSystemDirectoryW(systemDirectory, MAX_PATH))
        return fail("GetSystemDirectoryW failed");
    const std::filesystem::path powershell =
        std::filesystem::path(systemDirectory) / L"WindowsPowerShell" / L"v1.0" /
        L"powershell.exe";
    if (!std::filesystem::exists(powershell))
        return fail("powershell.exe not found");

    // Small output still works and is decoded correctly.
    {
        std::wstring output;
        const bool ok = fcitx::windows::config::runExecutable(
            powershell, {L"-NoProfile", L"-Command", L"Write-Output hello"}, output,
            30'000);
        if (!ok || output.find(L"hello") == std::wstring::npos) {
            std::wcerr << L"small output failed: ok=" << ok << L" output=" << output
                       << L'\n';
            return 1;
        }
    }

    // 64 KiB+ of child output: the reader thread must drain the pipe while the
    // parent waits, so the child never blocks on a full pipe buffer.
    {
        std::wstring output;
        const bool ok = fcitx::windows::config::runExecutable(
            powershell, {L"-NoProfile", L"-Command",
                         L"1..2000 | ForEach-Object { 'x' * 60 }"},
            output, 30'000);
        if (!ok || output.size() < 64 * 1024) {
            std::wcerr << L"64 KiB output failed: ok=" << ok
                       << L" bytes=" << output.size() << L'\n';
            return 1;
        }
    }

    // 1 MiB of child output must complete within the timeout (the pre-fix
    // implementation deadlocked here for the full 120 s timeout).
    {
        std::wstring output;
        const bool ok = fcitx::windows::config::runExecutable(
            powershell, {L"-NoProfile", L"-Command",
                         L"1..20000 | ForEach-Object { 'x' * 60 }"},
            output, 60'000);
        if (!ok || output.size() < 1024 * 1024) {
            std::wcerr << L"1 MiB output failed: ok=" << ok
                       << L" bytes=" << output.size() << L'\n';
            return 1;
        }
    }

    // Output above the configured retention limit is drained but not retained
    // unboundedly.
    {
        std::wstring output;
        const bool ok = fcitx::windows::config::runExecutable(
            powershell, {L"-NoProfile", L"-Command",
                         L"1..4000 | ForEach-Object { 'y' * 60 }"},
            output, 30'000, 4096);
        if (!ok || output.size() > 4096) {
            std::wcerr << L"bounded output failed: ok=" << ok
                       << L" bytes=" << output.size() << L'\n';
            return 1;
        }
    }

    // Non-zero exit is reported as failure while still returning captured
    // output for diagnostics.
    {
        std::wstring output;
        const bool ok = fcitx::windows::config::runExecutable(
            powershell, {L"-NoProfile", L"-Command", L"Write-Output nope; exit 7"},
            output, 30'000);
        if (ok || output.find(L"nope") == std::wstring::npos) {
            std::wcerr << L"non-zero exit contract failed: ok=" << ok
                       << L" output=" << output << L'\n';
            return 1;
        }
    }

    // Invalid UTF-8/binary-ish output must not crash or make a successful child
    // look like a process failure.
    {
        std::wstring output;
        const bool ok = fcitx::windows::config::runExecutable(
            powershell, {L"-NoProfile", L"-Command",
                         L"$s=[Console]::OpenStandardOutput();"
                         L"$b=[byte[]](0xff,0xfe,0x61);$s.Write($b,0,$b.Length)"},
            output, 30'000);
        if (!ok || output.empty()) {
            std::wcerr << L"binary-ish output contract failed: ok=" << ok
                       << L" length=" << output.size() << L'\n';
            return 1;
        }
    }

    // Timeout must contain the process tree: a grandchild that would write a
    // marker after the timeout should be killed with the job.
    {
        const auto marker = std::filesystem::temp_directory_path() /
                            (L"fcitx5-process-tree-" +
                             std::to_wstring(GetCurrentProcessId()) + L".txt");
        DeleteFileW(marker.c_str());
        std::wstring command =
            L"$m='" + marker.wstring() + L"';"
            L"Start-Process -WindowStyle Hidden -FilePath '" + powershell.wstring() +
            L"' -ArgumentList '-NoProfile','-Command',"
            L"'Start-Sleep -Milliseconds 1500; Set-Content -LiteralPath \"' + $m + '\" -Value survived';"
            L"Start-Sleep -Seconds 10";
        std::wstring output;
        const auto begin = GetTickCount64();
        const bool ok = fcitx::windows::config::runExecutable(
            powershell, {L"-NoProfile", L"-Command", command}, output, 500);
        const auto elapsed = GetTickCount64() - begin;
        std::this_thread::sleep_for(std::chrono::milliseconds(2200));
        const bool markerExists = std::filesystem::exists(marker);
        DeleteFileW(marker.c_str());
        if (ok || elapsed > 7000 || markerExists) {
            std::wcerr << L"timeout/process-tree containment failed: ok=" << ok
                       << L" elapsed=" << elapsed << L" marker=" << markerExists
                       << L'\n';
            return 1;
        }
    }

    std::cout << "process-execution-pipe ok\n";
    return 0;
}
