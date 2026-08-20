#include "peer_verification.h"
#include "pipe_client.h"
#include "pipe_security.h"
#include "protocol.h"
#include "runtime_identity.h"
#include "state_machine.h"
#include "state_store.h"
#include "tray_icon.h"

#include <fcitx5_windows/release_identity.h>
#include <fcitx5_windows/version.h>

#include <Windows.h>

#include <algorithm>
#include <array>
#include <atomic>
#include <cstdint>
#include <filesystem>
#include <iostream>
#include <span>
#include <string>
#include <vector>

namespace {

using namespace fcitx::windows;

class SystemClock final : public launcher::Clock {
  public:
    [[nodiscard]] std::uint64_t nowMilliseconds() const noexcept override {
        return GetTickCount64();
    }
};

bool absoluteWindowsPath(std::wstring_view path) {
    return path.size() >= 3 &&
           (((path[0] >= L'A' && path[0] <= L'Z') || (path[0] >= L'a' && path[0] <= L'z')) &&
            path[1] == L':' && (path[2] == L'\\' || path[2] == L'/'));
}

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

std::filesystem::path executableDirectory() {
    std::wstring path(32'768, L'\0');
    const DWORD size =
        GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    if (size == 0 || size >= path.size())
        return {};
    path.resize(size);
    return std::filesystem::path(path).parent_path();
}

bool transfer(HANDLE pipe, bool write, void* data, std::size_t size, DWORD timeout) {
    auto* cursor = static_cast<std::uint8_t*>(data);
    std::size_t completed = 0;
    while (completed < size) {
        HANDLE event = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        if (!event)
            return false;
        OVERLAPPED operation{};
        operation.hEvent = event;
        DWORD transferred = 0;
        const DWORD requested = static_cast<DWORD>(size - completed);
        const BOOL immediate =
            write ? WriteFile(pipe, cursor + completed, requested, &transferred, &operation)
                  : ReadFile(pipe, cursor + completed, requested, &transferred, &operation);
        bool success = immediate != FALSE;
        if (!success && GetLastError() == ERROR_IO_PENDING) {
            if (WaitForSingleObject(event, timeout) == WAIT_OBJECT_0) {
                success = GetOverlappedResult(pipe, &operation, &transferred, FALSE) != FALSE;
            } else {
                CancelIoEx(pipe, &operation);
                GetOverlappedResult(pipe, &operation, &transferred, TRUE);
                success = false;
            }
        }
        CloseHandle(event);
        if (!success || transferred == 0)
            return false;
        completed += transferred;
    }
    return true;
}

bool readFrame(HANDLE pipe, std::vector<std::uint8_t>& bytes) {
    std::array<std::uint8_t, protocol::kHeaderSize> header{};
    if (!transfer(pipe, false, header.data(), header.size(), 500))
        return false;
    protocol::MessageType type{};
    protocol::Metadata metadata;
    std::uint32_t bodySize = 0;
    if (!protocol::decodeHeader(header, type, bodySize, metadata))
        return false;
    bytes.assign(header.begin(), header.end());
    bytes.resize(protocol::kHeaderSize + bodySize);
    return bodySize == 0 ||
           transfer(pipe, false, bytes.data() + protocol::kHeaderSize, bodySize, 100);
}

bool writeFrame(HANDLE pipe, const std::vector<std::uint8_t>& bytes) {
    return !bytes.empty() &&
           transfer(pipe, true, const_cast<std::uint8_t*>(bytes.data()), bytes.size(), 100);
}

struct EngineProcess {
    HANDLE process{};
    HANDLE stopEvent{};
    std::uint64_t startedAt{};
};

struct UiProcess {
    HANDLE process{};
    bool safeMode{};
};

bool launchEngine(const std::wstring& enginePath, const std::wstring& readyEventName,
                  const std::wstring& stopEventName, bool safeMode, HANDLE job,
                  SECURITY_ATTRIBUTES* security, EngineProcess& output) {
    HANDLE stopEvent = CreateEventW(security, TRUE, FALSE, stopEventName.c_str());
    if (!stopEvent)
        return false;
    std::wstring command = quote(enginePath) + L" --ready-event " + quote(readyEventName) +
                           L" --stop-event " + quote(stopEventName) + L" --generation " +
                           quote(platform::currentRuntimeGeneration());
    if (safeMode)
        command += L" --safe-mode";
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(enginePath.c_str(), mutableCommand.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process)) {
        CloseHandle(stopEvent);
        return false;
    }
    CloseHandle(process.hThread);
    if (!AssignProcessToJobObject(job, process.hProcess)) {
        TerminateProcess(process.hProcess, 2);
        WaitForSingleObject(process.hProcess, 1000);
        CloseHandle(process.hProcess);
        CloseHandle(stopEvent);
        return false;
    }
    output = {process.hProcess, stopEvent, GetTickCount64()};
    return true;
}

bool launchUi(const std::wstring& uiPath, bool safeMode, HANDLE job, UiProcess& output) {
    std::wstring command =
        quote(uiPath) + L" --parent-pid " + std::to_wstring(GetCurrentProcessId()) +
        L" --generation " + quote(platform::currentRuntimeGeneration());
    if (safeMode)
        command += L" --safe-mode";
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(uiPath.c_str(), mutableCommand.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process))
        return false;
    CloseHandle(process.hThread);
    if (!AssignProcessToJobObject(job, process.hProcess)) {
        TerminateProcess(process.hProcess, 2);
        WaitForSingleObject(process.hProcess, 1000);
        CloseHandle(process.hProcess);
        return false;
    }
    output = {process.hProcess, safeMode};
    return true;
}

void stopUi(UiProcess& ui) {
    if (!ui.process)
        return;
    if (WaitForSingleObject(ui.process, 0) != WAIT_OBJECT_0) {
        TerminateProcess(ui.process, 0);
        WaitForSingleObject(ui.process, 1000);
    }
    CloseHandle(ui.process);
    ui = {};
}

void stopEngine(EngineProcess& engine) {
    if (!engine.process)
        return;
    if (engine.stopEvent)
        SetEvent(engine.stopEvent);
    if (WaitForSingleObject(engine.process, 2000) != WAIT_OBJECT_0) {
        TerminateProcess(engine.process, 2);
        WaitForSingleObject(engine.process, 1000);
    }
    CloseHandle(engine.process);
    if (engine.stopEvent)
        CloseHandle(engine.stopEvent);
    engine = {};
}

protocol::Status applyCommand(protocol::LauncherCommand command,
                              launcher::LauncherStateMachine& state,
                              const launcher::StateStore& store, EngineProcess& engine) {
    bool accepted = true;
    launcher::Command stateCommand{};
    bool changesState = true;
    switch (command) {
    case protocol::LauncherCommand::userStop:
        stateCommand = launcher::Command::userStop;
        break;
    case protocol::LauncherCommand::resume:
        stateCommand = launcher::Command::resume;
        break;
    case protocol::LauncherCommand::beginUpdate:
        stateCommand = launcher::Command::beginUpdate;
        break;
    case protocol::LauncherCommand::endUpdate:
        stateCommand = launcher::Command::endUpdate;
        break;
    case protocol::LauncherCommand::beginUninstall:
        stateCommand = launcher::Command::beginUninstall;
        break;
    case protocol::LauncherCommand::resetSafeMode:
        stateCommand = launcher::Command::resetSafeMode;
        break;
    case protocol::LauncherCommand::startDemand:
    case protocol::LauncherCommand::status:
    case protocol::LauncherCommand::shutdown:
        changesState = false;
        break;
    }
    if (changesState) {
        accepted = state.canApply(stateCommand) && state.apply(stateCommand) &&
                   store.save(state.snapshot());
    }
    if (accepted && engine.process &&
        (command == protocol::LauncherCommand::userStop ||
         command == protocol::LauncherCommand::beginUpdate ||
         command == protocol::LauncherCommand::beginUninstall)) {
        stopEngine(engine);
        state.engineStoppedIntentionally();
        (void)store.save(state.snapshot());
    }
    return accepted ? protocol::Status::ok : protocol::Status::unsupported;
}

} // namespace

int WINAPI wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE, _In_ PWSTR, _In_ int) {
    const int argc = __argc;
    wchar_t** argv = __wargv;
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
        std::cout << "fcitx5-launcher " << fcitx::windows::version() << " protocol "
                  << protocol::kVersion << '\n';
        return 0;
    }
    if (argc == 2 && std::wstring_view(argv[1]) == L"--tray-self-test") {
        launcher::TrayIcon tray;
        if (!tray.create(instance, executableDirectory()))
            return 10;
        const std::uint64_t deadline = GetTickCount64() + 5000;
        do {
            tray.dispatchMessages();
            if (tray.iconAdded() && tray.shellVisible())
                return 0;
            Sleep(25);
        } while (GetTickCount64() < deadline);
        if (!tray.iconAdded())
            return 11;
        return tray.usesGuidIdentity() ? 13 : 12;
    }
    std::wstring enginePath;
    std::wstring uiPath;
    std::wstring externalReadyEvent;
    std::wstring launcherReadyEventName;
    std::wstring stopEventName;
    std::wstring stateFilePath;
    bool warmup = true;
    bool installedDefaults = argc == 1;
    for (int index = 1; index < argc; ++index) {
        const std::wstring_view argument(argv[index]);
        if (argument == L"--engine" && index + 1 < argc) {
            enginePath = argv[++index];
        } else if (argument == L"--ui" && index + 1 < argc) {
            uiPath = argv[++index];
        } else if (argument == L"--no-warmup") {
            warmup = false;
        } else if (argument == L"--background") {
            installedDefaults = true;
        } else if (argument == L"--engine-ready-event" && index + 1 < argc) {
            externalReadyEvent = argv[++index];
        } else if (argument == L"--ready-event" && index + 1 < argc) {
            launcherReadyEventName = argv[++index];
        } else if (argument == L"--stop-event" && index + 1 < argc) {
            stopEventName = argv[++index];
        } else if (argument == L"--state-file" && index + 1 < argc) {
            stateFilePath = argv[++index];
        } else if (argument == L"--generation" && index + 1 < argc) {
            const std::wstring generation = argv[++index];
            if (!SetEnvironmentVariableW(L"FCITX5_RELEASE_GENERATION", generation.c_str()) ||
                platform::currentRuntimeGeneration() != generation)
                return 1;
        } else {
            std::wcerr << L"Usage: fcitx5-launcher [--background] | "
                          L"--tray-self-test | "
                          L"--engine ABSOLUTE_PATH [--ui ABSOLUTE_PATH] "
                          L"[--no-warmup] "
                          L"[--engine-ready-event NAME] [--ready-event NAME] "
                          L"[--stop-event NAME] [--state-file ABSOLUTE_PATH] "
                          L"[--generation GENERATION]\n";
            return 1;
        }
    }
    if (enginePath.empty() && installedDefaults) {
        const auto directory = executableDirectory();
        const auto generation = platform::currentRuntimeGeneration();
        const auto installedRoot = directory.filename() == L"bin" ? directory.parent_path()
                                                                  : directory;
        const auto generationBin = installedRoot / L"runtime" / generation / L"bin";
        const auto generationEngine = generationBin / L"fcitx5-engine.exe";
        const auto generationUi = generationBin / L"fcitx5-ui.exe";
        if (std::filesystem::exists(generationEngine) && std::filesystem::exists(generationUi)) {
            enginePath = generationEngine.wstring();
            uiPath = generationUi.wstring();
        } else {
            enginePath = (directory / L"fcitx5-engine.exe").wstring();
            uiPath = (directory / L"fcitx5-ui.exe").wstring();
        }
    }
    if (!absoluteWindowsPath(enginePath) ||
        GetFileAttributesW(enginePath.c_str()) == INVALID_FILE_ATTRIBUTES)
        return 1;
    if (!uiPath.empty() && (!absoluteWindowsPath(uiPath) ||
                            GetFileAttributesW(uiPath.c_str()) == INVALID_FILE_ATTRIBUTES))
        return 1;

    platform::RuntimeIdentity identity;
    platform::PipeSecurity security;
    if (!platform::queryCurrentIdentity(identity) || !identity.mayUseUserEngine() ||
        !platform::PipeSecurity::create(identity, security))
        return 4;
    const std::wstring endpoint = platform::makeLocalEndpointName(identity, L"launcher");
    const std::wstring mutexName = platform::makeLocalObjectName(identity, L"launcher");
    HANDLE mutex = CreateMutexW(security.attributes(), FALSE, mutexName.c_str());
    if (!mutex)
        return 2;
    if (GetLastError() == ERROR_ALREADY_EXISTS) {
        const BOOL available = WaitNamedPipeW(endpoint.c_str(), 100);
        CloseHandle(mutex);
        return available ? 0 : 3;
    }
    HANDLE job = CreateJobObjectW(security.attributes(), nullptr);
    if (!job) {
        CloseHandle(mutex);
        return 2;
    }
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION jobLimits{};
    jobLimits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if (!SetInformationJobObject(job, JobObjectExtendedLimitInformation, &jobLimits,
                                 sizeof(jobLimits))) {
        CloseHandle(job);
        CloseHandle(mutex);
        return 2;
    }
    UiProcess ui;
    if (!uiPath.empty() && !launchUi(uiPath, false, job, ui)) {
        CloseHandle(job);
        CloseHandle(mutex);
        return 5;
    }
    const std::wstring readyEventName = externalReadyEvent.empty()
                                            ? platform::makeLocalObjectName(identity, L"engine-ready")
                                            : externalReadyEvent;
    HANDLE engineReady = CreateEventW(security.attributes(), TRUE, FALSE, readyEventName.c_str());
    HANDLE launcherReady =
        launcherReadyEventName.empty()
            ? nullptr
            : CreateEventW(security.attributes(), TRUE, FALSE, launcherReadyEventName.c_str());
    HANDLE stopEvent =
        stopEventName.empty()
            ? CreateEventW(security.attributes(), TRUE, FALSE, nullptr)
            : CreateEventW(security.attributes(), TRUE, FALSE, stopEventName.c_str());
    if (!engineReady || !stopEvent || (!launcherReadyEventName.empty() && !launcherReady)) {
        if (engineReady)
            CloseHandle(engineReady);
        if (launcherReady)
            CloseHandle(launcherReady);
        if (stopEvent)
            CloseHandle(stopEvent);
        CloseHandle(job);
        CloseHandle(mutex);
        return 2;
    }

    if (stateFilePath.empty())
        stateFilePath = launcher::defaultStateStorePath();
    if (!absoluteWindowsPath(stateFilePath)) {
        CloseHandle(stopEvent);
        if (launcherReady)
            CloseHandle(launcherReady);
        CloseHandle(engineReady);
        CloseHandle(job);
        CloseHandle(mutex);
        return 2;
    }
    launcher::StateStore stateStore(stateFilePath);
    launcher::LauncherSnapshot initialSnapshot;
    const auto loadResult = stateStore.load(initialSnapshot);
    if ((loadResult == launcher::LoadStateResult::missing &&
         !stateStore.save(launcher::LauncherSnapshot{})) ||
        loadResult == launcher::LoadStateResult::ioError) {
        CloseHandle(stopEvent);
        if (launcherReady)
            CloseHandle(launcherReady);
        CloseHandle(engineReady);
        CloseHandle(job);
        CloseHandle(mutex);
        return 2;
    }
    if (loadResult == launcher::LoadStateResult::invalid) {
        initialSnapshot = {.state = launcher::LauncherState::userStopped};
    }

    SystemClock clock;
    launcher::LauncherStateMachine state(clock, initialSnapshot);
    launcher::TrayIcon tray;
    EngineProcess engine;
    bool restartDesired = warmup;
    bool running = true;
    HANDLE pendingPipe = nullptr;
    std::uint64_t engineGeneration = 0;
    std::uint64_t nextInputMethodStatusPoll = 0;
    protocol::EngineStatusResponse inputMethodStatus;
    ipc::PipeClient engineStatusClient(platform::makeLocalEndpointName(identity, L"engine"),
                                       ipc::PeerPolicy::exact(enginePath));
    std::atomic<std::uint64_t> nextResponseId{1};
    const auto clearInputMethodStatus = [&] {
        engineStatusClient.disconnect();
        inputMethodStatus = {};
        nextInputMethodStatusPoll = 0;
    };
    const auto refreshInputMethodStatus = [&](bool force) {
        if (state.engineState() != launcher::EngineState::ready || !engine.process) {
            clearInputMethodStatus();
            return;
        }
        const auto now = clock.nowMilliseconds();
        if (!force && now < nextInputMethodStatusPoll)
            return;
        protocol::EngineStatusResponse status;
        if (engineStatusClient.queryEngineStatus(status, 75)) {
            inputMethodStatus = std::move(status);
        }
        nextInputMethodStatusPoll = now + 1000;
    };
    while (running) {
        switch (tray.takeCommand()) {
        case launcher::TrayCommand::restart:
            stopEngine(engine);
            state.engineStoppedIntentionally();
            clearInputMethodStatus();
            if (state.state() == launcher::LauncherState::userStopped &&
                state.canApply(launcher::Command::resume) &&
                state.apply(launcher::Command::resume) && stateStore.save(state.snapshot())) {
                // Resumed by an explicit restart command.
            }
            restartDesired = true;
            break;
        case launcher::TrayCommand::pause:
            if (state.canApply(launcher::Command::userStop) &&
                state.apply(launcher::Command::userStop) && stateStore.save(state.snapshot())) {
                stopEngine(engine);
                state.engineStoppedIntentionally();
                (void)stateStore.save(state.snapshot());
                clearInputMethodStatus();
                restartDesired = false;
            }
            break;
        case launcher::TrayCommand::resume:
            if (state.canApply(launcher::Command::resume) &&
                state.apply(launcher::Command::resume) && stateStore.save(state.snapshot())) {
                restartDesired = true;
            }
            break;
        case launcher::TrayCommand::exit:
            running = false;
            break;
        case launcher::TrayCommand::none:
            break;
        }
        if (!running)
            break;
        const bool uiSafeMode = state.state() == launcher::LauncherState::safeMode;
        if (!uiPath.empty() && ui.process && ui.safeMode != uiSafeMode) {
            stopUi(ui);
            (void)launchUi(uiPath, uiSafeMode, job, ui);
        }
        if (restartDesired && !engine.process) {
            const auto decision = state.requestStart();
            if (decision.disposition == launcher::StartDisposition::start) {
                ResetEvent(engineReady);
                const std::wstring engineStopEventName =
                    platform::makeLocalObjectName(
                        identity, L"engine-stop-" + std::to_wstring(++engineGeneration));
                if (!launchEngine(enginePath, readyEventName, engineStopEventName,
                                  decision.safeMode, job, security.attributes(), engine)) {
                    state.engineExited(0);
                    (void)stateStore.save(state.snapshot());
                }
            }
        }
        if (engine.process && state.engineState() == launcher::EngineState::starting &&
            WaitForSingleObject(engineReady, 0) == WAIT_OBJECT_0) {
            state.engineReady();
            refreshInputMethodStatus(true);
        }
        refreshInputMethodStatus(false);
        tray.update(state.state(), state.engineState(), inputMethodStatus);

        HANDLE pipe = pendingPipe;
        pendingPipe = nullptr;
        if (!pipe) {
            pipe = CreateNamedPipeW(
                endpoint.c_str(), PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS, 2,
                static_cast<DWORD>(protocol::kMaxControlFrameSize),
                static_cast<DWORD>(protocol::kMaxControlFrameSize), 100, security.attributes());
        }
        if (pipe == INVALID_HANDLE_VALUE)
            break;
        if (launcherReady)
            SetEvent(launcherReady);
        HANDLE connectEvent = CreateEventW(nullptr, TRUE, FALSE, nullptr);
        OVERLAPPED connection{};
        connection.hEvent = connectEvent;
        BOOL connected = ConnectNamedPipe(pipe, &connection);
        const DWORD connectError = connected ? ERROR_SUCCESS : GetLastError();
        bool connectionIssued = false;
        bool connectionSucceeded = connected != FALSE || connectError == ERROR_PIPE_CONNECTED;
        if (connected)
            SetEvent(connectEvent);
        else if (connectError == ERROR_PIPE_CONNECTED)
            SetEvent(connectEvent);
        else if (connectError == ERROR_IO_PENDING)
            connectionIssued = true;
        else
            SetEvent(connectEvent);

        std::array<HANDLE, 3> waits{connectEvent, stopEvent, engine.process};
        const DWORD waitCount = engine.process ? 3U : 2U;
        DWORD timeout = INFINITE;
        if (restartDesired && !engine.process &&
            state.state() == launcher::LauncherState::crashBackoff) {
            const auto now = clock.nowMilliseconds();
            const auto allowed = state.nextStartAllowedMilliseconds();
            timeout =
                now >= allowed
                    ? 0
                    : static_cast<DWORD>((std::min)(allowed - now, std::uint64_t{MAXDWORD - 1}));
        }
        const DWORD waitResult =
            tray.valid()
                ? MsgWaitForMultipleObjects(waitCount, waits.data(), FALSE, timeout, QS_ALLINPUT)
                : WaitForMultipleObjects(waitCount, waits.data(), FALSE, timeout);
        HANDLE standbyPipe = nullptr;
        if (tray.valid() && waitResult == WAIT_OBJECT_0 + waitCount) {
            tray.dispatchMessages();
        } else if (waitResult == WAIT_OBJECT_0 + 1) {
            running = false;
        } else if (engine.process && waitResult == WAIT_OBJECT_0 + 2) {
            const auto runtime = clock.nowMilliseconds() - engine.startedAt;
            CloseHandle(engine.process);
            if (engine.stopEvent)
                CloseHandle(engine.stopEvent);
            engine = {};
            state.engineExited(runtime);
            (void)stateStore.save(state.snapshot());
            clearInputMethodStatus();
            restartDesired = true;
        } else if (waitResult == WAIT_TIMEOUT) {
            restartDesired = true;
        } else if (waitResult == WAIT_OBJECT_0) {
            if (connectionIssued) {
                DWORD ignored = 0;
                connectionSucceeded =
                    GetOverlappedResult(pipe, &connection, &ignored, FALSE) != FALSE;
                connectionIssued = false;
            }
            if (connectionSucceeded) {
                standbyPipe = CreateNamedPipeW(
                    endpoint.c_str(), PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS, 2,
                    static_cast<DWORD>(protocol::kMaxControlFrameSize),
                    static_cast<DWORD>(protocol::kMaxControlFrameSize), 100, security.attributes());
                if (standbyPipe != INVALID_HANDLE_VALUE && launcherReady) {
                    SetEvent(launcherReady);
                }
                if (engine.process && state.engineState() == launcher::EngineState::starting &&
                    WaitForSingleObject(engineReady, 0) == WAIT_OBJECT_0) {
                    state.engineReady();
                    refreshInputMethodStatus(true);
                }
                platform::ProcessIdentity client;
                if (ipc::verifyPipeClient(pipe, identity, &client)) {
                    std::vector<std::uint8_t> requestBytes;
                    protocol::FrameView frame;
                    protocol::LauncherRequest request;
                    if (readFrame(pipe, requestBytes) &&
                        protocol::decodeFrame(requestBytes, frame) &&
                        protocol::decode(frame, request) &&
                        request.metadata.sessionId == identity.sessionId) {
                        protocol::Status status =
                            applyCommand(request.command, state, stateStore, engine);
                        launcher::StartDecision decision;
                        if (status == protocol::Status::ok &&
                            request.command == protocol::LauncherCommand::startDemand) {
                            restartDesired = true;
                            decision = state.requestStart();
                            if (decision.disposition == launcher::StartDisposition::start &&
                                !launchEngine(enginePath, readyEventName,
                                              platform::makeLocalObjectName(
                                                  identity,
                                                  L"engine-stop-" +
                                                      std::to_wstring(++engineGeneration)),
                                              decision.safeMode, job, security.attributes(),
                                              engine)) {
                                state.engineExited(0);
                                (void)stateStore.save(state.snapshot());
                                status = protocol::Status::unsupported;
                            }
                        }
                        if (request.command == protocol::LauncherCommand::shutdown) {
                            running = false;
                        }
                        if (!engine.process ||
                            state.engineState() != launcher::EngineState::ready) {
                            clearInputMethodStatus();
                        } else if (request.command == protocol::LauncherCommand::status) {
                            refreshInputMethodStatus(true);
                        }
                        const protocol::LauncherResponse response{
                            protocol::Metadata{nextResponseId.fetch_add(1),
                                               request.metadata.requestId, 0, identity.sessionId, 0,
                                               0, 0},
                            status,
                            static_cast<std::uint32_t>(state.state()),
                            static_cast<std::uint32_t>(state.engineState()),
                            static_cast<std::uint32_t>(decision.disposition),
                            decision.safeMode,
                            decision.retryAfterMilliseconds,
                            inputMethodStatus.currentInputMethodId,
                            inputMethodStatus.currentInputMethodName,
                            inputMethodStatus.currentInputMethodNativeName,
                            inputMethodStatus.currentInputMethodShortLabel};
                        if (writeFrame(pipe, protocol::encode(response))) {
                            std::uint8_t unexpected = 0;
                            (void)transfer(pipe, false, &unexpected, 1, 100);
                        }
                    }
                }
            }
        }
        if (connectionIssued) {
            CancelIoEx(pipe, &connection);
            DWORD ignored = 0;
            GetOverlappedResult(pipe, &connection, &ignored, TRUE);
        }
        DisconnectNamedPipe(pipe);
        CloseHandle(connectEvent);
        CloseHandle(pipe);
        if (standbyPipe && standbyPipe != INVALID_HANDLE_VALUE) {
            if (running)
                pendingPipe = standbyPipe;
            else
                CloseHandle(standbyPipe);
        }
    }

    if (pendingPipe)
        CloseHandle(pendingPipe);
    stopEngine(engine);
    stopUi(ui);
    CloseHandle(stopEvent);
    if (launcherReady)
        CloseHandle(launcherReady);
    CloseHandle(engineReady);
    CloseHandle(job);
    CloseHandle(mutex);
    return 0;
}
