#include "fcitx_dispatcher.h"

#include <fcitx-utils/event.h>
#include <fcitx-utils/eventdispatcher.h>
#include <fcitx/instance.h>

#include <windows.h>

#include <cstdlib>
#include <future>
#include <iostream>
#include <utility>

namespace fcitx::windows::engine {

namespace {

// The Fcitx event loop must answer a heartbeat within this window. A loop that
// is stuck this long, or that has exited, can no longer serve keys; ending the
// process lets the launcher start a fresh engine instead of leaving every
// client failing open.
constexpr auto kHeartbeatInterval = std::chrono::seconds(5);
constexpr auto kHeartbeatTimeout = std::chrono::seconds(30);
constexpr UINT kRestartExitCode = 70;

std::int64_t steadyMilliseconds() {
    return std::chrono::duration_cast<std::chrono::milliseconds>(
               std::chrono::steady_clock::now().time_since_epoch())
        .count();
}

} // namespace

FcitxDispatcher::FcitxDispatcher() = default;
FcitxDispatcher::~FcitxDispatcher() { stop(); }

bool FcitxDispatcher::start(bool safeMode) {
    if (thread_.joinable())
        return false;
    auto ready = std::make_shared<std::promise<bool>>();
    auto future = ready->get_future();
    thread_ = std::thread([this, ready, safeMode] {
        bool acceptingStarted = false;
        try {
            runtime_ = std::make_unique<FcitxRuntime>();
            if (!runtime_->initialize(safeMode)) {
                runtime_.reset();
                ready->set_value(false);
                return;
            }
            dispatcher_ = std::make_unique<::fcitx::EventDispatcher>();
            dispatcher_->attach(&runtime_->eventLoop());
            accepting_.store(true, std::memory_order_release);
            acceptingStarted = true;
            dispatcher_->schedule([ready] { ready->set_value(true); });
            runtime_->eventLoop().exec();
            accepting_.store(false, std::memory_order_release);
            if (!stopping_.load(std::memory_order_acquire))
                terminateForRestart("Fcitx event loop exited unexpectedly");
            dispatcher_->detach();
            dispatcher_.reset();
            runtime_.reset();
        } catch (...) {
            accepting_.store(false, std::memory_order_release);
            if (acceptingStarted && !stopping_.load(std::memory_order_acquire))
                terminateForRestart("Fcitx event loop failed");
            dispatcher_.reset();
            runtime_.reset();
            try {
                ready->set_value(false);
            } catch (...) {
            }
        }
    });
    if (!future.get()) {
        thread_.join();
        return false;
    }
    lastHeartbeatMs_.store(steadyMilliseconds(), std::memory_order_release);
    watchdog_ = std::thread([this] { watch(); });
    return true;
}

void FcitxDispatcher::watch() {
    std::unique_lock lock(watchdogMutex_);
    while (!watchdogWake_.wait_for(lock, kHeartbeatInterval,
                                   [this] { return stopping_.load(std::memory_order_acquire); })) {
        const auto silentMs =
            steadyMilliseconds() - lastHeartbeatMs_.load(std::memory_order_acquire);
        if (silentMs >=
            std::chrono::duration_cast<std::chrono::milliseconds>(kHeartbeatTimeout).count()) {
            terminateForRestart("Fcitx event loop stopped responding");
        }
        if (accepting_.load(std::memory_order_acquire) && dispatcher_) {
            dispatcher_->schedule([this] {
                lastHeartbeatMs_.store(steadyMilliseconds(), std::memory_order_release);
            });
        }
    }
}

void FcitxDispatcher::terminateForRestart(const char* reason) {
    std::cerr << reason << "; exiting so the launcher restarts the engine\n";
    std::cerr.flush();
    TerminateProcess(GetCurrentProcess(), kRestartExitCode);
    std::abort();
}

bool FcitxDispatcher::processKey(const ClientContextKey& context, const FcitxKeyRequestC& request,
                                 RuntimeResult& result, std::chrono::milliseconds timeout) {
    if (!accepting_.load(std::memory_order_acquire) || !dispatcher_)
        return false;
    auto completed = std::make_shared<std::promise<RuntimeResult>>();
    auto future = completed->get_future();
    // Absolute deadline: the queued work re-checks it right before touching
    // Fcitx state. If the caller already gave up (timeout), the task must not
    // execute at all - the client has fail-opened the key and a late
    // processKey would silently mutate composition/history after the caller
    // stopped listening. Already-running work cannot be aborted mid-flight,
    // but work that only got scheduled and then stalled is dropped here.
    const auto deadline = std::chrono::steady_clock::now() + timeout;
    dispatcher_->schedule([this, context, request, completed, deadline] {
        if (testDispatchDelayMs_ > 0 &&
            !testDispatchDelayConsumed_.exchange(true, std::memory_order_acq_rel)) {
            Sleep(testDispatchDelayMs_);
        }
        if (std::chrono::steady_clock::now() >= deadline) {
            droppedCount_.fetch_add(1, std::memory_order_acq_rel);
            try {
                completed->set_value(RuntimeResult{});
            } catch (...) {
            }
            return;
        }
        try {
            completed->set_value(runtime_->processKey(context, request));
        } catch (const std::exception& error) {
            std::cerr << "Fcitx dispatcher request failed: " << error.what() << '\n';
            completed->set_exception(std::current_exception());
        } catch (...) {
            std::cerr << "Fcitx dispatcher request failed with unknown exception\n";
            completed->set_exception(std::current_exception());
        }
    });
    if (future.wait_for(timeout) != std::future_status::ready) {
        std::cerr << "Fcitx dispatcher request exceeded " << timeout.count() << " ms\n";
        return false;
    }
    try {
        result = future.get();
        return true;
    } catch (...) {
        return false;
    }
}

bool FcitxDispatcher::selectCandidate(std::uint32_t targetProcessId,
                                      const FcitxCandidateSelectRequestC& request,
                                      RuntimeResult& result, std::chrono::milliseconds timeout) {
    if (!accepting_.load(std::memory_order_acquire) || !dispatcher_)
        return false;
    auto completed = std::make_shared<std::promise<RuntimeResult>>();
    auto future = completed->get_future();
    const auto deadline = std::chrono::steady_clock::now() + timeout;
    dispatcher_->schedule([this, targetProcessId, request, completed, deadline] {
        if (testDispatchDelayMs_ > 0 &&
            !testDispatchDelayConsumed_.exchange(true, std::memory_order_acq_rel)) {
            Sleep(testDispatchDelayMs_);
        }
        if (std::chrono::steady_clock::now() >= deadline) {
            droppedCount_.fetch_add(1, std::memory_order_acq_rel);
            try {
                completed->set_value(RuntimeResult{});
            } catch (...) {
            }
            return;
        }
        try {
            completed->set_value(runtime_->selectCandidate(targetProcessId, request));
        } catch (...) {
            completed->set_exception(std::current_exception());
        }
    });
    if (future.wait_for(timeout) != std::future_status::ready)
        return false;
    try {
        result = future.get();
        return true;
    } catch (...) {
        return false;
    }
}

bool FcitxDispatcher::takePendingState(const ClientContextKey& context,
                                       const FcitxStateRequestC& request, RuntimeResult& result,
                                       std::chrono::milliseconds timeout) {
    if (!accepting_.load(std::memory_order_acquire) || !dispatcher_)
        return false;
    auto completed = std::make_shared<std::promise<RuntimeResult>>();
    auto future = completed->get_future();
    const auto deadline = std::chrono::steady_clock::now() + timeout;
    dispatcher_->schedule([this, context, request, completed, deadline] {
        if (testDispatchDelayMs_ > 0 &&
            !testDispatchDelayConsumed_.exchange(true, std::memory_order_acq_rel)) {
            Sleep(testDispatchDelayMs_);
        }
        if (std::chrono::steady_clock::now() >= deadline) {
            droppedCount_.fetch_add(1, std::memory_order_acq_rel);
            try {
                completed->set_value(RuntimeResult{});
            } catch (...) {
            }
            return;
        }
        try {
            completed->set_value(runtime_->takePendingState(context, request));
        } catch (...) {
            completed->set_exception(std::current_exception());
        }
    });
    if (future.wait_for(timeout) != std::future_status::ready)
        return false;
    try {
        result = future.get();
        return true;
    } catch (...) {
        return false;
    }
}

bool FcitxDispatcher::queryInputMethodStatus(InputMethodStatus& result,
                                             std::chrono::milliseconds timeout) {
    result = {};
    if (!accepting_.load(std::memory_order_acquire) || !dispatcher_)
        return false;
    auto completed = std::make_shared<std::promise<InputMethodStatus>>();
    auto future = completed->get_future();
    const auto deadline = std::chrono::steady_clock::now() + timeout;
    dispatcher_->schedule([this, completed, deadline] {
        if (std::chrono::steady_clock::now() >= deadline) {
            droppedCount_.fetch_add(1, std::memory_order_acq_rel);
            try {
                completed->set_value(InputMethodStatus{});
            } catch (...) {
            }
            return;
        }
        try {
            completed->set_value(runtime_->currentInputMethod());
        } catch (...) {
            completed->set_exception(std::current_exception());
        }
    });
    if (future.wait_for(timeout) != std::future_status::ready)
        return false;
    try {
        result = future.get();
        return true;
    } catch (...) {
        result = {};
        return false;
    }
}

bool FcitxDispatcher::inputMethodAction(const ClientContextKey& key, bool toggle,
                                        InputMethodStatus& result,
                                        std::chrono::milliseconds timeout) {
    result = {};
    if (!accepting_.load(std::memory_order_acquire) || !dispatcher_)
        return false;
    auto completed = std::make_shared<std::promise<InputMethodStatus>>();
    auto future = completed->get_future();
    const auto deadline = std::chrono::steady_clock::now() + timeout;
    dispatcher_->schedule([this, completed, deadline, key, toggle] {
        // A late toggle must not flip the mode after the client gave up.
        if (std::chrono::steady_clock::now() >= deadline) {
            droppedCount_.fetch_add(1, std::memory_order_acq_rel);
            try {
                completed->set_value(InputMethodStatus{});
            } catch (...) {
            }
            return;
        }
        try {
            completed->set_value(runtime_->inputMethodAction(key, toggle));
        } catch (...) {
            completed->set_exception(std::current_exception());
        }
    });
    if (future.wait_for(timeout) != std::future_status::ready)
        return false;
    try {
        result = future.get();
        return !result.id.empty();
    } catch (...) {
        result = {};
        return false;
    }
}

void FcitxDispatcher::forgetConnection(std::uint64_t connectionId) {
    if (!accepting_.load(std::memory_order_acquire) || !dispatcher_ || connectionId == 0)
        return;
    dispatcher_->schedule([this, connectionId] { runtime_->forgetConnection(connectionId); });
}

void FcitxDispatcher::stop() {
    {
        std::lock_guard lock(watchdogMutex_);
        stopping_.store(true, std::memory_order_release);
    }
    watchdogWake_.notify_all();
    if (watchdog_.joinable())
        watchdog_.join();
    accepting_.store(false, std::memory_order_release);
    if (dispatcher_ && runtime_) {
        dispatcher_->schedule([this] { runtime_->eventLoop().exit(); });
    }
    if (thread_.joinable())
        thread_.join();
}

} // namespace fcitx::windows::engine
