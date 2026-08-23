#pragma once

#include "launcher_rust_abi.h"

#include <cstdint>

namespace fcitx::windows::launcher {

enum class LauncherState {
    normal,
    userStopped,
    updating,
    uninstalling,
    crashBackoff,
    safeMode,
};

enum class EngineState {
    stopped,
    starting,
    ready,
};

enum class Command {
    userStop,
    resume,
    beginUpdate,
    endUpdate,
    beginUninstall,
    resetSafeMode,
};

enum class StartDisposition {
    start,
    alreadyActive,
    suppressed,
    backoff,
};

struct StartDecision {
    StartDisposition disposition{StartDisposition::suppressed};
    bool safeMode{};
    std::uint64_t retryAfterMilliseconds{};
};

struct LauncherSnapshot {
    LauncherState state{LauncherState::normal};
    unsigned consecutiveStartupCrashes{};
    std::uint64_t nextStartAllowedMilliseconds{};
};

class Clock {
public:
    virtual ~Clock() = default;
    [[nodiscard]] virtual std::uint64_t nowMilliseconds() const noexcept = 0;
};

class LauncherStateMachine final {
public:
    explicit LauncherStateMachine(const Clock& clock,
                                  LauncherState initialState = LauncherState::normal) noexcept
        : LauncherStateMachine(clock, LauncherSnapshot{initialState, 0, 0}) {}
    explicit LauncherStateMachine(const Clock& clock, LauncherSnapshot snapshot) noexcept
        : clock_(clock) {
        rust_abi::Fcitx5LauncherMachine machine{};
        if (fcitx5_launcher_state_init(clock_.nowMilliseconds(), toRust(snapshot), &machine) ==
            0) {
            state_ = static_cast<LauncherState>(machine.snapshot.state);
            engineState_ = static_cast<EngineState>(machine.engineState);
            consecutiveStartupCrashes_ = machine.snapshot.consecutiveStartupCrashes;
            nextStartAllowedMilliseconds_ = machine.snapshot.nextStartAllowedMilliseconds;
        } else {
            state_ = LauncherState::userStopped;
            engineState_ = EngineState::stopped;
            consecutiveStartupCrashes_ = 0;
            nextStartAllowedMilliseconds_ = 0;
        }
    }

    [[nodiscard]] LauncherState state() const noexcept { return state_; }
    [[nodiscard]] EngineState engineState() const noexcept { return engineState_; }
    [[nodiscard]] unsigned consecutiveStartupCrashes() const noexcept {
        return consecutiveStartupCrashes_;
    }
    [[nodiscard]] std::uint64_t nextStartAllowedMilliseconds() const noexcept {
        return nextStartAllowedMilliseconds_;
    }
    [[nodiscard]] LauncherSnapshot snapshot() const noexcept {
        return {state_, consecutiveStartupCrashes_, nextStartAllowedMilliseconds_};
    }

    [[nodiscard]] bool apply(Command command) noexcept {
        rust_abi::Fcitx5LauncherMachine machine{toRust(snapshot()),
                                                static_cast<std::uint32_t>(engineState_)};
        if (!fcitx5_launcher_state_apply(&machine, static_cast<std::uint32_t>(command))) {
            return false;
        }
        applyRustMachine(machine);
        return true;
    }
    [[nodiscard]] bool canApply(Command command) const noexcept {
        return fcitx5_launcher_state_can_apply(static_cast<std::uint32_t>(state_),
                                               static_cast<std::uint32_t>(command)) != 0;
    }
    [[nodiscard]] LauncherState stateAfter(Command command) const noexcept {
        return static_cast<LauncherState>(fcitx5_launcher_state_after(
            static_cast<std::uint32_t>(state_), static_cast<std::uint32_t>(command)));
    }
    [[nodiscard]] StartDecision requestStart() noexcept {
        rust_abi::Fcitx5LauncherMachine machine{toRust(snapshot()),
                                                static_cast<std::uint32_t>(engineState_)};
        rust_abi::Fcitx5LauncherStartDecision rustDecision{};
        if (fcitx5_launcher_state_request_start(&machine, clock_.nowMilliseconds(),
                                                &rustDecision) != 0) {
            return {};
        }
        applyRustMachine(machine);
        return {static_cast<StartDisposition>(rustDecision.disposition),
                rustDecision.safeMode != 0, rustDecision.retryAfterMilliseconds};
    }
    void engineReady() noexcept {
        rust_abi::Fcitx5LauncherMachine machine{toRust(snapshot()),
                                                static_cast<std::uint32_t>(engineState_)};
        fcitx5_launcher_state_engine_ready(&machine);
        engineState_ = static_cast<EngineState>(machine.engineState);
    }
    void engineExited(std::uint64_t runtimeMilliseconds) noexcept {
        rust_abi::Fcitx5LauncherMachine machine{toRust(snapshot()),
                                                static_cast<std::uint32_t>(engineState_)};
        fcitx5_launcher_state_engine_exited(&machine, runtimeMilliseconds,
                                            clock_.nowMilliseconds());
        applyRustMachine(machine);
    }
    void engineStoppedIntentionally() noexcept {
        rust_abi::Fcitx5LauncherMachine machine{toRust(snapshot()),
                                                static_cast<std::uint32_t>(engineState_)};
        fcitx5_launcher_state_engine_stopped_intentionally(&machine);
        engineState_ = static_cast<EngineState>(machine.engineState);
    }

    static constexpr std::uint64_t kStartupCrashWindowMilliseconds = 10'000;
    static constexpr std::uint64_t kStableRuntimeMilliseconds = 60'000;
    static constexpr std::uint64_t kInitialBackoffMilliseconds = 250;
    static constexpr std::uint64_t kMaximumBackoffMilliseconds = 30'000;
    static constexpr unsigned kSafeModeCrashThreshold = 3;

private:
    [[nodiscard]] static rust_abi::Fcitx5LauncherSnapshot toRust(
        LauncherSnapshot snapshot) noexcept {
        return {static_cast<std::uint32_t>(snapshot.state), snapshot.consecutiveStartupCrashes,
                snapshot.nextStartAllowedMilliseconds};
    }
    void applyRustMachine(const rust_abi::Fcitx5LauncherMachine& machine) noexcept {
        state_ = static_cast<LauncherState>(machine.snapshot.state);
        engineState_ = static_cast<EngineState>(machine.engineState);
        consecutiveStartupCrashes_ = machine.snapshot.consecutiveStartupCrashes;
        nextStartAllowedMilliseconds_ = machine.snapshot.nextStartAllowedMilliseconds;
    }

    const Clock& clock_;
    LauncherState state_{LauncherState::normal};
    EngineState engineState_{EngineState::stopped};
    unsigned consecutiveStartupCrashes_{};
    std::uint64_t nextStartAllowedMilliseconds_{};
};

} // namespace fcitx::windows::launcher
