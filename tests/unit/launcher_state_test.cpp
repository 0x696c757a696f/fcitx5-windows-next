#include "state_machine.h"
#include "state_store.h"

#include <Windows.h>

#include <cstdint>
#include <iostream>

namespace {

class FakeClock final : public fcitx::windows::launcher::Clock {
public:
    [[nodiscard]] std::uint64_t nowMilliseconds() const noexcept override { return now_; }
    void advance(std::uint64_t amount) noexcept { now_ += amount; }

private:
    std::uint64_t now_{};
};

bool require(bool condition, const char* message) {
    if (!condition) std::cerr << message << '\n';
    return condition;
}

} // namespace

int main() {
    using namespace fcitx::windows::launcher;
    FakeClock clock;
    LauncherStateMachine machine(clock);

    if (!require(machine.requestStart().disposition == StartDisposition::start,
                 "normal state did not start") ||
        !require(machine.requestStart().disposition == StartDisposition::alreadyActive,
                 "duplicate start was not coalesced")) return 1;
    machine.engineReady();
    machine.engineExited(1);
    if (!require(machine.state() == LauncherState::crashBackoff &&
                     machine.consecutiveStartupCrashes() == 1,
                 "first startup crash did not enter backoff")) return 1;
    const auto first = machine.requestStart();
    if (!require(first.disposition == StartDisposition::backoff &&
                     first.retryAfterMilliseconds == 250,
                 "first backoff duration was incorrect")) return 1;
    clock.advance(250);
    if (!require(machine.requestStart().disposition == StartDisposition::start,
                 "virtual-clock expiry did not permit restart")) return 1;
    machine.engineExited(2);
    if (!require(machine.requestStart().retryAfterMilliseconds == 500,
                 "second backoff was not exponential")) return 1;
    clock.advance(500);
    (void)machine.requestStart();
    machine.engineExited(3);
    if (!require(machine.state() == LauncherState::safeMode &&
                     machine.requestStart().safeMode,
                 "third startup crash did not enter SafeMode")) return 1;
    const auto safeModeSnapshot = machine.snapshot();
    LauncherStateMachine restartedDuringSafeMode(clock, safeModeSnapshot);
    if (!require(restartedDuringSafeMode.state() == LauncherState::safeMode &&
                     restartedDuringSafeMode.consecutiveStartupCrashes() >=
                         LauncherStateMachine::kSafeModeCrashThreshold &&
                     restartedDuringSafeMode.requestStart().safeMode,
                 "REG-LAUNCHER-LEDGER-001 SafeMode did not survive launcher restart")) return 1;
    machine.engineExited(LauncherStateMachine::kStableRuntimeMilliseconds);
    if (!require(machine.state() == LauncherState::normal &&
                     machine.consecutiveStartupCrashes() == 0,
                 "stable runtime did not reset crash accounting")) return 1;

    if (!require(machine.apply(Command::userStop), "UserStopped transition rejected") ||
        !require(machine.requestStart().disposition == StartDisposition::suppressed,
                 "UserStopped allowed TSF demand start") ||
        !require(!machine.apply(Command::endUpdate), "illegal EndUpdate was accepted") ||
        !require(machine.apply(Command::resume), "resume transition rejected") ||
        !require(machine.apply(Command::beginUpdate), "Updating transition rejected") ||
        !require(machine.requestStart().disposition == StartDisposition::suppressed,
                 "Updating allowed TSF demand start") ||
        !require(machine.apply(Command::endUpdate), "EndUpdate transition rejected") ||
        !require(machine.apply(Command::beginUninstall), "Uninstalling transition rejected") ||
        !require(machine.requestStart().disposition == StartDisposition::suppressed,
                 "Uninstalling allowed TSF demand start") ||
        !require(!machine.apply(Command::resume), "Uninstalling illegally resumed")) return 1;

    wchar_t temporaryDirectory[MAX_PATH]{};
    if (!require(GetTempPathW(MAX_PATH, temporaryDirectory) != 0,
                 "temporary directory query failed")) return 1;
    const std::wstring statePath = std::wstring(temporaryDirectory) +
                                   L"fcitx5-launcher-state-test-" +
                                   std::to_wstring(GetCurrentProcessId());
    DeleteFileW(statePath.c_str());
    StateStore store(statePath);
    LauncherState loaded = LauncherState::safeMode;
    if (!require(store.load(loaded) == LoadStateResult::missing,
                 "missing state was not reported") ||
        !require(store.save(LauncherState::userStopped), "UserStopped state save failed") ||
        !require(store.load(loaded) == LoadStateResult::loaded &&
                     loaded == LauncherState::userStopped,
                 "UserStopped state did not survive reload")) {
        DeleteFileW(statePath.c_str());
        return 1;
    }
    LauncherStateMachine restored(clock, loaded);
    if (!require(restored.requestStart().disposition == StartDisposition::suppressed,
                 "restored UserStopped state allowed demand restart") ||
        !require(store.save(LauncherState::updating), "Updating state save failed") ||
        !require(store.load(loaded) == LoadStateResult::loaded &&
                     loaded == LauncherState::updating,
                 "Updating state did not survive reload") ||
        !require(store.save(LauncherState::uninstalling), "Uninstalling state save failed") ||
        !require(store.load(loaded) == LoadStateResult::loaded &&
                     loaded == LauncherState::uninstalling,
                 "Uninstalling state did not survive reload")) {
        DeleteFileW(statePath.c_str());
        return 1;
    }
    LauncherSnapshot persistedCrash{LauncherState::crashBackoff, 2, clock.nowMilliseconds() + 500};
    LauncherSnapshot loadedSnapshot;
    if (!require(store.save(persistedCrash), "crash ledger save failed") ||
        !require(store.load(loadedSnapshot) == LoadStateResult::loaded &&
                     loadedSnapshot.state == LauncherState::crashBackoff &&
                     loadedSnapshot.consecutiveStartupCrashes == 2,
                 "crash ledger did not survive reload")) {
        DeleteFileW(statePath.c_str());
        return 1;
    }
    LauncherStateMachine restoredCrash(clock, loadedSnapshot);
    if (!require(restoredCrash.state() == LauncherState::crashBackoff &&
                     restoredCrash.consecutiveStartupCrashes() == 2 &&
                     restoredCrash.requestStart().disposition == StartDisposition::backoff,
                 "restored crash ledger did not enforce backoff")) {
        DeleteFileW(statePath.c_str());
        return 1;
    }
    DeleteFileW(statePath.c_str());

    return 0;
}
