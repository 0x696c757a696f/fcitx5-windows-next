#include "process_execution.h"

#include <Windows.h>

#include <array>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace fcitx::windows::config {
namespace {

std::wstring quote(std::wstring_view value) {
    std::wstring result;
    result.reserve(value.size() + 2);
    result.push_back(L'"');
    std::size_t backslashes = 0;
    for (const wchar_t character : value) {
        if (character == L'\\') {
            ++backslashes;
        } else if (character == L'"') {
            result.append(backslashes + 1, L'\\');
            backslashes = 0;
            result.push_back(character);
        } else {
            result.append(backslashes, L'\\');
            backslashes = 0;
            result.push_back(character);
        }
    }
    result.append(backslashes * 2, L'\\');
    result.push_back(L'"');
    return result;
}

} // namespace

bool runExecutable(const std::filesystem::path& executable,
                   const std::vector<std::wstring>& arguments,
                   std::wstring& output, unsigned timeoutMilliseconds) {
    output.clear();
    if (!std::filesystem::exists(executable))
        return false;
    std::wstring command = quote(executable.wstring());
    for (const auto& argument : arguments)
        command += L" " + quote(argument);
    SECURITY_ATTRIBUTES attributes{sizeof(attributes), nullptr, TRUE};
    HANDLE readPipe = nullptr;
    HANDLE writePipe = nullptr;
    if (!CreatePipe(&readPipe, &writePipe, &attributes, 0))
        return false;
    SetHandleInformation(readPipe, HANDLE_FLAG_INHERIT, 0);
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    startup.dwFlags = STARTF_USESTDHANDLES;
    startup.hStdOutput = writePipe;
    startup.hStdError = writePipe;
    PROCESS_INFORMATION process{};
    const BOOL created =
        CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, TRUE,
                       CREATE_NO_WINDOW, nullptr, executable.parent_path().c_str(),
                       &startup, &process);
    CloseHandle(writePipe);
    if (!created) {
        CloseHandle(readPipe);
        return false;
    }
    // Drain the pipes on a reader thread while the process runs. Waiting for
    // the child before reading would deadlock as soon as the child fills the
    // pipe buffer: the child blocks in WriteFile, the parent blocks in
    // WaitForSingleObject, and only the timeout terminates them.
    std::string bytes;
    std::thread reader([&] {
        std::array<char, 2048> buffer{};
        DWORD count = 0;
        while (ReadFile(readPipe, buffer.data(), static_cast<DWORD>(buffer.size()),
                        &count, nullptr) &&
               count != 0)
            bytes.append(buffer.data(), count);
    });
    const DWORD wait = WaitForSingleObject(process.hProcess, timeoutMilliseconds);
    if (wait == WAIT_TIMEOUT)
        TerminateProcess(process.hProcess, ERROR_TIMEOUT);
    reader.join();
    DWORD exitCode = 1;
    GetExitCodeProcess(process.hProcess, &exitCode);
    CloseHandle(readPipe);
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    if (!bytes.empty()) {
        const int wideSize = MultiByteToWideChar(
            CP_UTF8, 0, bytes.data(), static_cast<int>(bytes.size()), nullptr, 0);
        output.resize(static_cast<std::size_t>(wideSize));
        if (wideSize > 0) {
            (void)MultiByteToWideChar(CP_UTF8, 0, bytes.data(),
                                      static_cast<int>(bytes.size()), output.data(),
                                      wideSize);
        }
    }
    return wait == WAIT_OBJECT_0 && exitCode == 0;
}

} // namespace fcitx::windows::config
