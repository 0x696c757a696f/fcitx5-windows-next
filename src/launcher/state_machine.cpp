#include "state_machine.h"

#include <algorithm>
#include <limits>

namespace fcitx::windows::launcher {

LauncherStateMachine::LauncherStateMachine(const Clock& clock,
                                           LauncherState initialState) noexcept
    : LauncherStateMachine(clock, LauncherSnapshot{initialState, 0, 0}) {}

LauncherStateMachine::LauncherStateMachine(const Clock& clock, LauncherSnapshot snapshot) noexcept
    : clock_(clock),
      state_(snapshot.state),
      consecutiveStartupCrashes_(snapshot.consecutiveStartupCrashes),
      nextStartAllowedMilliseconds_(snapshot.nextStartAllowedMilliseconds) {
    if (state_ == LauncherState::crashBackoff) {
        if (consecutiveStartupCrashes_ == 0)
            consecutiveStartupCrashes_ = 1;
        const auto now = clock_.nowMilliseconds();
        if (nextStartAllowedMilliseconds_ == 0 ||
            nextStartAllowedMilliseconds_ > now + kMaximumBackoffMilliseconds) {
            nextStartAllowedMilliseconds_ = now + kInitialBackoffMilliseconds;
        }
    } else if (state_ == LauncherState::safeMode) {
        if (consecutiveStartupCrashes_ < kSafeModeCrashThreshold)
            consecutiveStartupCrashes_ = kSafeModeCrashThreshold;
        nextStartAllowedMilliseconds_ = 0;
    } else {
        resetCrashAccounting();
    }
}

bool LauncherStateMachine::startSuppressed() const noexcept {
    return state_ == LauncherState::userStopped || state_ == LauncherState::updating ||
           state_ == LauncherState::uninstalling;
}

void LauncherStateMachine::resetCrashAccounting() noexcept {
    consecutiveStartupCrashes_ = 0;
    nextStartAllowedMilliseconds_ = 0;
}

LauncherSnapshot LauncherStateMachine::snapshot() const noexcept {
    return {state_, consecutiveStartupCrashes_, nextStartAllowedMilliseconds_};
}

bool LauncherStateMachine::apply(Command command) noexcept {
    if (!canApply(command)) return false;
    state_ = stateAfter(command);
    switch (command) {
    case Command::userStop:
        engineState_ = EngineState::stopped;
        return true;
    case Command::resume:
        resetCrashAccounting();
        return true;
    case Command::beginUpdate:
        engineState_ = EngineState::stopped;
        return true;
    case Command::endUpdate:
        resetCrashAccounting();
        return true;
    case Command::beginUninstall:
        engineState_ = EngineState::stopped;
        return true;
    case Command::resetSafeMode:
        resetCrashAccounting();
        return true;
    }
    return false;
}

bool LauncherStateMachine::canApply(Command command) const noexcept {
    switch (command) {
    case Command::userStop:
    case Command::beginUpdate:
        return state_ != LauncherState::uninstalling && state_ != LauncherState::updating;
    case Command::resume:
        return state_ == LauncherState::userStopped;
    case Command::endUpdate:
        return state_ == LauncherState::updating;
    case Command::beginUninstall:
        return state_ != LauncherState::uninstalling;
    case Command::resetSafeMode:
        return state_ == LauncherState::safeMode;
    }
    return false;
}

LauncherState LauncherStateMachine::stateAfter(Command command) const noexcept {
    switch (command) {
    case Command::userStop:
        return LauncherState::userStopped;
    case Command::beginUpdate:
        return LauncherState::updating;
    case Command::beginUninstall:
        return LauncherState::uninstalling;
    case Command::resume:
    case Command::endUpdate:
    case Command::resetSafeMode:
        return LauncherState::normal;
    }
    return state_;
}

StartDecision LauncherStateMachine::requestStart() noexcept {
    if (startSuppressed()) return {StartDisposition::suppressed, false, 0};
    if (engineState_ != EngineState::stopped) {
        return {StartDisposition::alreadyActive, state_ == LauncherState::safeMode, 0};
    }
    const auto now = clock_.nowMilliseconds();
    if (state_ == LauncherState::crashBackoff && now < nextStartAllowedMilliseconds_) {
        return {StartDisposition::backoff, false, nextStartAllowedMilliseconds_ - now};
    }
    if (state_ == LauncherState::crashBackoff) state_ = LauncherState::normal;
    engineState_ = EngineState::starting;
    return {StartDisposition::start, state_ == LauncherState::safeMode, 0};
}

void LauncherStateMachine::engineReady() noexcept {
    if (engineState_ == EngineState::starting) engineState_ = EngineState::ready;
}

void LauncherStateMachine::engineExited(std::uint64_t runtimeMilliseconds) noexcept {
    engineState_ = EngineState::stopped;
    if (startSuppressed()) return;
    if (runtimeMilliseconds >= kStableRuntimeMilliseconds) {
        state_ = LauncherState::normal;
        resetCrashAccounting();
        return;
    }
    if (runtimeMilliseconds >= kStartupCrashWindowMilliseconds) {
        state_ = LauncherState::normal;
        consecutiveStartupCrashes_ = 0;
        nextStartAllowedMilliseconds_ = 0;
        return;
    }
    if (consecutiveStartupCrashes_ < std::numeric_limits<unsigned>::max()) {
        ++consecutiveStartupCrashes_;
    }
    if (consecutiveStartupCrashes_ >= kSafeModeCrashThreshold) {
        state_ = LauncherState::safeMode;
        nextStartAllowedMilliseconds_ = 0;
        return;
    }
    const unsigned shift = (std::min)(consecutiveStartupCrashes_ - 1, 16U);
    const std::uint64_t delay =
        (std::min)(kInitialBackoffMilliseconds << shift, kMaximumBackoffMilliseconds);
    state_ = LauncherState::crashBackoff;
    const auto now = clock_.nowMilliseconds();
    nextStartAllowedMilliseconds_ =
        now > std::numeric_limits<std::uint64_t>::max() - delay
            ? std::numeric_limits<std::uint64_t>::max()
            : now + delay;
}

void LauncherStateMachine::engineStoppedIntentionally() noexcept {
    engineState_ = EngineState::stopped;
}

} // namespace fcitx::windows::launcher
