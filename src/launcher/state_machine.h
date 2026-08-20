#pragma once

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
                                  LauncherState initialState = LauncherState::normal) noexcept;
    explicit LauncherStateMachine(const Clock& clock, LauncherSnapshot snapshot) noexcept;

    [[nodiscard]] LauncherState state() const noexcept { return state_; }
    [[nodiscard]] EngineState engineState() const noexcept { return engineState_; }
    [[nodiscard]] unsigned consecutiveStartupCrashes() const noexcept {
        return consecutiveStartupCrashes_;
    }
    [[nodiscard]] std::uint64_t nextStartAllowedMilliseconds() const noexcept {
        return nextStartAllowedMilliseconds_;
    }
    [[nodiscard]] LauncherSnapshot snapshot() const noexcept;

    [[nodiscard]] bool apply(Command command) noexcept;
    [[nodiscard]] bool canApply(Command command) const noexcept;
    [[nodiscard]] LauncherState stateAfter(Command command) const noexcept;
    [[nodiscard]] StartDecision requestStart() noexcept;
    void engineReady() noexcept;
    void engineExited(std::uint64_t runtimeMilliseconds) noexcept;
    void engineStoppedIntentionally() noexcept;

    static constexpr std::uint64_t kStartupCrashWindowMilliseconds = 10'000;
    static constexpr std::uint64_t kStableRuntimeMilliseconds = 60'000;
    static constexpr std::uint64_t kInitialBackoffMilliseconds = 250;
    static constexpr std::uint64_t kMaximumBackoffMilliseconds = 30'000;
    static constexpr unsigned kSafeModeCrashThreshold = 3;

private:
    [[nodiscard]] bool startSuppressed() const noexcept;
    void resetCrashAccounting() noexcept;

    const Clock& clock_;
    LauncherState state_{LauncherState::normal};
    EngineState engineState_{EngineState::stopped};
    unsigned consecutiveStartupCrashes_{};
    std::uint64_t nextStartAllowedMilliseconds_{};
};

} // namespace fcitx::windows::launcher
