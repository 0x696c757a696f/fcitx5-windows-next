#include "process_execution.h"

#include <Windows.h>

#include <array>
#include <cstddef>
#include <memory>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace fcitx::windows::config {
namespace {

struct HandleCloser {
    void operator()(void* handle) const noexcept {
        if (handle && handle != INVALID_HANDLE_VALUE)
            CloseHandle(static_cast<HANDLE>(handle));
    }
};

using UniqueHandle = std::unique_ptr<void, HandleCloser>;

UniqueHandle uniqueHandle(HANDLE handle) noexcept {
    return UniqueHandle(handle == INVALID_HANDLE_VALUE ? nullptr : handle);
}

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
                   std::wstring& output, unsigned timeoutMilliseconds,
                   std::size_t maxOutputBytes) {
    output.clear();
    if (!std::filesystem::exists(executable))
        return false;
    std::wstring command = quote(executable.wstring());
    for (const auto& argument : arguments)
        command += L" " + quote(argument);
    UniqueHandle job(CreateJobObjectW(nullptr, nullptr));
    if (!job)
        return false;
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION jobLimits{};
    jobLimits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if (!SetInformationJobObject(job.get(), JobObjectExtendedLimitInformation, &jobLimits,
                                 sizeof(jobLimits))) {
        return false;
    }

    SECURITY_ATTRIBUTES attributes{sizeof(attributes), nullptr, TRUE};
    HANDLE readPipe = nullptr;
    HANDLE writePipe = nullptr;
    if (!CreatePipe(&readPipe, &writePipe, &attributes, 0))
        return false;
    UniqueHandle readHandle = uniqueHandle(readPipe);
    UniqueHandle writeHandle = uniqueHandle(writePipe);
    SetHandleInformation(readPipe, HANDLE_FLAG_INHERIT, 0);

    SIZE_T attributeListSize = 0;
    (void)InitializeProcThreadAttributeList(nullptr, 1, 0, &attributeListSize);
    std::vector<std::byte> attributeListStorage(attributeListSize);
    auto* attributeList =
        reinterpret_cast<PPROC_THREAD_ATTRIBUTE_LIST>(attributeListStorage.data());
    if (!InitializeProcThreadAttributeList(attributeList, 1, 0, &attributeListSize))
        return false;
    const auto deleteAttributeList = [&] {
        DeleteProcThreadAttributeList(attributeList);
    };
    HANDLE inheritedHandles[]{writePipe};
    if (!UpdateProcThreadAttribute(attributeList, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                                   inheritedHandles, sizeof(inheritedHandles), nullptr,
                                   nullptr)) {
        deleteAttributeList();
        return false;
    }

    STARTUPINFOEXW startup{};
    startup.StartupInfo.cb = sizeof(startup);
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdOutput = writePipe;
    startup.StartupInfo.hStdError = writePipe;
    startup.lpAttributeList = attributeList;
    PROCESS_INFORMATION process{};
    const BOOL created =
        CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, TRUE,
                       CREATE_NO_WINDOW | CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT,
                       nullptr, executable.parent_path().c_str(), &startup.StartupInfo,
                       &process);
    deleteAttributeList();
    writeHandle.reset();
    if (!created) {
        return false;
    }
    UniqueHandle processHandle = uniqueHandle(process.hProcess);
    UniqueHandle threadHandle = uniqueHandle(process.hThread);
    if (!AssignProcessToJobObject(job.get(), process.hProcess)) {
        TerminateProcess(process.hProcess, ERROR_ACCESS_DENIED);
        return false;
    }
    if (ResumeThread(process.hThread) == static_cast<DWORD>(-1)) {
        TerminateJobObject(job.get(), ERROR_ACCESS_DENIED);
        WaitForSingleObject(process.hProcess, 5000);
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
               count != 0) {
            if (bytes.size() < maxOutputBytes) {
                const auto remaining = maxOutputBytes - bytes.size();
                bytes.append(buffer.data(), (std::min)(remaining,
                                                       static_cast<std::size_t>(count)));
            }
        }
    });
    const DWORD wait = WaitForSingleObject(process.hProcess, timeoutMilliseconds);
    DWORD finalWait = wait;
    if (wait == WAIT_TIMEOUT) {
        TerminateJobObject(job.get(), ERROR_TIMEOUT);
        finalWait = WaitForSingleObject(process.hProcess, 5000);
    }
    reader.join();
    DWORD exitCode = 1;
    GetExitCodeProcess(process.hProcess, &exitCode);
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
    return wait == WAIT_OBJECT_0 && finalWait == WAIT_OBJECT_0 && exitCode == 0;
}

} // namespace fcitx::windows::config
