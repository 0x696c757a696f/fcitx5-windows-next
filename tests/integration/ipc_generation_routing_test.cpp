#include "pipe_client.h"
#include "runtime_identity.h"

#include <Windows.h>

#include <cstdint>
#include <cwctype>
#include <iostream>
#include <string>
#include <string_view>
#include <vector>

namespace {

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

struct EngineProcess {
    HANDLE process{};
    HANDLE ready{};
};

EngineProcess start_engine(const wchar_t* executable, std::wstring_view generation,
                           std::wstring_view suffix) {
    const std::wstring readyName = L"Local\\Fcitx5WindowsNext.GenerationRouting." +
                                   std::wstring(generation) + L"." + std::wstring(suffix);
    HANDLE ready = CreateEventW(nullptr, TRUE, FALSE, readyName.c_str());
    if (!ready) return {};
    std::wstring command = quote(executable) + L" --test-clients 1 --generation " +
                           quote(generation) + L" --ready-event " + quote(readyName);
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{sizeof(startup)};
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(executable, mutableCommand.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process)) {
        CloseHandle(ready);
        return {};
    }
    CloseHandle(process.hThread);
    if (WaitForSingleObject(ready, 2000) != WAIT_OBJECT_0) {
        TerminateProcess(process.hProcess, 2);
        WaitForSingleObject(process.hProcess, 1000);
        CloseHandle(process.hProcess);
        CloseHandle(ready);
        return {};
    }
    return {process.hProcess, ready};
}

bool stop_engine(EngineProcess& engine) {
    if (!engine.process) return false;
    const DWORD wait = WaitForSingleObject(engine.process, 2000);
    if (wait != WAIT_OBJECT_0) {
        TerminateProcess(engine.process, 2);
        WaitForSingleObject(engine.process, 1000);
    }
    DWORD exitCode = 1;
    GetExitCodeProcess(engine.process, &exitCode);
    CloseHandle(engine.process);
    CloseHandle(engine.ready);
    engine = {};
    return wait == WAIT_OBJECT_0 && exitCode == 0;
}

bool send_key(const fcitx::windows::platform::RuntimeIdentity& identity,
              std::wstring_view generation, const wchar_t* enginePath,
              std::uint64_t contextId, WPARAM key) {
    fcitx::windows::ipc::PipeClient client(
        fcitx::windows::platform::makeLocalEndpointName(identity, generation, L"engine"),
        fcitx::windows::ipc::PeerPolicy::exact(enginePath), std::wstring(generation));
    fcitx::windows::ipc::KeyResult result;
    return client.processKey(contextId, static_cast<std::uint32_t>(key), 0, result) &&
           result.handled && result.commit.size() == 1U &&
           result.commit[0] == static_cast<wchar_t>(std::towlower(static_cast<wint_t>(key)));
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) {
        std::cerr << "mock engine argument required\n";
        return 1;
    }
    const std::wstring suffix = std::to_wstring(GetCurrentProcessId());
    if (!SetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE",
                                 (L"generation-" + suffix).c_str()))
        return 1;
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity)) return 1;

    EngineProcess gen41 = start_engine(argv[1], L"00000041", suffix);
    EngineProcess gen42 = start_engine(argv[1], L"00000042", suffix);
    bool ok = gen41.process && gen42.process &&
              send_key(identity, L"00000041", argv[1], 4101, 'A') &&
              send_key(identity, L"00000042", argv[1], 4201, 'B') &&
              stop_engine(gen41) && stop_engine(gen42);
    if (gen41.process) ok = stop_engine(gen41) && ok;
    if (gen42.process) ok = stop_engine(gen42) && ok;
    SetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", nullptr);
    if (!ok) {
        std::cerr << "generation-specific IPC routing failed\n";
        return 1;
    }
    return 0;
}
