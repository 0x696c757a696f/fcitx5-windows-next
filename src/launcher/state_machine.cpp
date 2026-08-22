#include "state_machine.h"

#include <cstdint>

namespace {

struct Fcitx5LauncherSnapshot {
    std::uint32_t state;
    std::uint32_t consecutiveStartupCrashes;
    std::uint64_t nextStartAllowedMilliseconds;
};

struct Fcitx5LauncherMachine {
    Fcitx5LauncherSnapshot snapshot;
    std::uint32_t engineState;
};

struct Fcitx5LauncherStartDecision {
    std::uint32_t disposition;
    std::uint8_t safeMode;
    std::uint8_t reserved[7];
    std::uint64_t retryAfterMilliseconds;
};

extern "C" {
int fcitx5_launcher_state_init(std::uint64_t now, Fcitx5LauncherSnapshot snapshot,
                               Fcitx5LauncherMachine* output);
std::uint8_t fcitx5_launcher_state_can_apply(std::uint32_t state, std::uint32_t command);
std::uint32_t fcitx5_launcher_state_after(std::uint32_t state, std::uint32_t command);
std::uint8_t fcitx5_launcher_state_apply(Fcitx5LauncherMachine* machine, std::uint32_t command);
int fcitx5_launcher_state_request_start(Fcitx5LauncherMachine* machine, std::uint64_t now,
                                        Fcitx5LauncherStartDecision* output);
void fcitx5_launcher_state_engine_ready(Fcitx5LauncherMachine* machine);
void fcitx5_launcher_state_engine_exited(Fcitx5LauncherMachine* machine,
                                         std::uint64_t runtimeMilliseconds, std::uint64_t now);
void fcitx5_launcher_state_engine_stopped_intentionally(Fcitx5LauncherMachine* machine);
}

Fcitx5LauncherSnapshot toRust(fcitx::windows::launcher::LauncherSnapshot snapshot) noexcept {
    return {static_cast<std::uint32_t>(snapshot.state), snapshot.consecutiveStartupCrashes,
            snapshot.nextStartAllowedMilliseconds};
}

} // namespace

namespace fcitx::windows::launcher {

LauncherStateMachine::LauncherStateMachine(const Clock& clock,
                                           LauncherState initialState) noexcept
    : LauncherStateMachine(clock, LauncherSnapshot{initialState, 0, 0}) {}

LauncherStateMachine::LauncherStateMachine(const Clock& clock, LauncherSnapshot snapshot) noexcept
    : clock_(clock) {
    Fcitx5LauncherMachine machine{};
    if (fcitx5_launcher_state_init(clock_.nowMilliseconds(), toRust(snapshot), &machine) == 0) {
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

LauncherSnapshot LauncherStateMachine::snapshot() const noexcept {
    return {state_, consecutiveStartupCrashes_, nextStartAllowedMilliseconds_};
}

bool LauncherStateMachine::apply(Command command) noexcept {
    Fcitx5LauncherMachine machine{toRust(snapshot()), static_cast<std::uint32_t>(engineState_)};
    if (!fcitx5_launcher_state_apply(&machine, static_cast<std::uint32_t>(command))) {
        return false;
    }
    state_ = static_cast<LauncherState>(machine.snapshot.state);
    engineState_ = static_cast<EngineState>(machine.engineState);
    consecutiveStartupCrashes_ = machine.snapshot.consecutiveStartupCrashes;
    nextStartAllowedMilliseconds_ = machine.snapshot.nextStartAllowedMilliseconds;
    return true;
}

bool LauncherStateMachine::canApply(Command command) const noexcept {
    return fcitx5_launcher_state_can_apply(static_cast<std::uint32_t>(state_),
                                           static_cast<std::uint32_t>(command)) != 0;
}

LauncherState LauncherStateMachine::stateAfter(Command command) const noexcept {
    return static_cast<LauncherState>(fcitx5_launcher_state_after(
        static_cast<std::uint32_t>(state_), static_cast<std::uint32_t>(command)));
}

StartDecision LauncherStateMachine::requestStart() noexcept {
    Fcitx5LauncherMachine machine{toRust(snapshot()), static_cast<std::uint32_t>(engineState_)};
    Fcitx5LauncherStartDecision rustDecision{};
    if (fcitx5_launcher_state_request_start(&machine, clock_.nowMilliseconds(), &rustDecision) !=
        0) {
        return {};
    }
    state_ = static_cast<LauncherState>(machine.snapshot.state);
    engineState_ = static_cast<EngineState>(machine.engineState);
    consecutiveStartupCrashes_ = machine.snapshot.consecutiveStartupCrashes;
    nextStartAllowedMilliseconds_ = machine.snapshot.nextStartAllowedMilliseconds;
    return {static_cast<StartDisposition>(rustDecision.disposition), rustDecision.safeMode != 0,
            rustDecision.retryAfterMilliseconds};
}

void LauncherStateMachine::engineReady() noexcept {
    Fcitx5LauncherMachine machine{toRust(snapshot()), static_cast<std::uint32_t>(engineState_)};
    fcitx5_launcher_state_engine_ready(&machine);
    engineState_ = static_cast<EngineState>(machine.engineState);
}

void LauncherStateMachine::engineExited(std::uint64_t runtimeMilliseconds) noexcept {
    Fcitx5LauncherMachine machine{toRust(snapshot()), static_cast<std::uint32_t>(engineState_)};
    fcitx5_launcher_state_engine_exited(&machine, runtimeMilliseconds, clock_.nowMilliseconds());
    state_ = static_cast<LauncherState>(machine.snapshot.state);
    engineState_ = static_cast<EngineState>(machine.engineState);
    consecutiveStartupCrashes_ = machine.snapshot.consecutiveStartupCrashes;
    nextStartAllowedMilliseconds_ = machine.snapshot.nextStartAllowedMilliseconds;
}

void LauncherStateMachine::engineStoppedIntentionally() noexcept {
    Fcitx5LauncherMachine machine{toRust(snapshot()), static_cast<std::uint32_t>(engineState_)};
    fcitx5_launcher_state_engine_stopped_intentionally(&machine);
    engineState_ = static_cast<EngineState>(machine.engineState);
}

} // namespace fcitx::windows::launcher
