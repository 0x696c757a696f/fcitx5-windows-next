// Pipe-deadlock regression test for config::runExecutable: a child that emits
// more than the pipe buffer must be drained concurrently while the parent
// waits, otherwise the child blocks in WriteFile and the parent blocks in
// WaitForSingleObject until the timeout kills both.
#include "process_execution.h"

#include <Windows.h>

#include <filesystem>
#include <iostream>
#include <string>
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

    std::cout << "process-execution-pipe ok\n";
    return 0;
}
