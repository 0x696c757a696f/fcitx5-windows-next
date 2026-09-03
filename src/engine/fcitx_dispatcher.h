#pragma once

#include "fcitx_runtime.h"

#include <atomic>
#include <chrono>
#include <memory>
#include <thread>

namespace fcitx {
class EventDispatcher;
}

namespace fcitx::windows::engine {

class FcitxDispatcher final {
  public:
    FcitxDispatcher();
    ~FcitxDispatcher();

    FcitxDispatcher(const FcitxDispatcher&) = delete;
    FcitxDispatcher& operator=(const FcitxDispatcher&) = delete;

    [[nodiscard]] bool start(bool safeMode = false);
    // Test hook: make the FIRST queued dispatcher task sleep before execution
    // so integration tests can drive the timeout-then-late-execution path
    // (the client times out while the task is stalled, then the stalled task
    // is dropped at its deadline check). Subsequent tasks run normally.
    // Honored when FCITX5_TEST_DISPATCH_DELAY_MS is set at startup.
    void setTestDispatchDelay(std::uint32_t milliseconds) { testDispatchDelayMs_ = milliseconds; }
    [[nodiscard]] bool processKey(const ClientContextKey& context, const FcitxKeyRequestC& request,
                                  RuntimeResult& result, std::chrono::milliseconds timeout);
    [[nodiscard]] bool selectCandidate(std::uint32_t targetProcessId,
                                       const FcitxCandidateSelectRequestC& request,
                                       RuntimeResult& result, std::chrono::milliseconds timeout);
    [[nodiscard]] bool takePendingState(const ClientContextKey& context,
                                        const FcitxStateRequestC& request, RuntimeResult& result,
                                        std::chrono::milliseconds timeout);
    [[nodiscard]] bool queryInputMethodStatus(InputMethodStatus& result,
                                              std::chrono::milliseconds timeout);
    void forgetConnection(std::uint64_t connectionId);
    void stop();

    // Number of queued tasks dropped at their deadline check because the
    // caller had already timed out. Exposed for the REG-DISPATCH-LATE
    // integration test, which asserts the stalled key never touched Fcitx.
    [[nodiscard]] std::uint64_t droppedCount() const noexcept {
        return droppedCount_.load(std::memory_order_acquire);
    }

  private:
    std::thread thread_;
    std::unique_ptr<FcitxRuntime> runtime_;
    std::unique_ptr<::fcitx::EventDispatcher> dispatcher_;
    std::atomic<bool> accepting_{};
    std::uint32_t testDispatchDelayMs_{};
    std::atomic<bool> testDispatchDelayConsumed_{};
    std::atomic<std::uint64_t> droppedCount_{};
};

} // namespace fcitx::windows::engine
