#include "fcitx_dispatcher.h"

#include <fcitx-utils/event.h>
#include <fcitx-utils/eventdispatcher.h>
#include <fcitx/instance.h>

#include <windows.h>

#include <future>
#include <iostream>
#include <utility>

namespace fcitx::windows::engine {

FcitxDispatcher::FcitxDispatcher() = default;
FcitxDispatcher::~FcitxDispatcher() { stop(); }

bool FcitxDispatcher::start(bool safeMode) {
    if (thread_.joinable()) return false;
    auto ready = std::make_shared<std::promise<bool>>();
    auto future = ready->get_future();
    thread_ = std::thread([this, ready, safeMode] {
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
            dispatcher_->schedule([ready] { ready->set_value(true); });
            runtime_->eventLoop().exec();
            accepting_.store(false, std::memory_order_release);
            dispatcher_->detach();
            dispatcher_.reset();
            runtime_.reset();
        } catch (...) {
            accepting_.store(false, std::memory_order_release);
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
    return true;
}

bool FcitxDispatcher::processKey(const ClientContextKey& context,
                                 const protocol::KeyRequest& request,
                                 RuntimeResult& result,
                                 std::chrono::milliseconds timeout) {
    if (!accepting_.load(std::memory_order_acquire) || !dispatcher_) return false;
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

bool FcitxDispatcher::selectCandidate(
    std::uint32_t targetProcessId,
    const protocol::CandidateSelectRequest& request,
    RuntimeResult& result, std::chrono::milliseconds timeout) {
    if (!accepting_.load(std::memory_order_acquire) || !dispatcher_) return false;
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
    if (future.wait_for(timeout) != std::future_status::ready) return false;
    try {
        result = future.get();
        return true;
    } catch (...) {
        return false;
    }
}

bool FcitxDispatcher::takePendingState(
    const ClientContextKey& context, const protocol::StateRequest& request,
    RuntimeResult& result, std::chrono::milliseconds timeout) {
    if (!accepting_.load(std::memory_order_acquire) || !dispatcher_) return false;
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
    if (future.wait_for(timeout) != std::future_status::ready) return false;
    try {
        result = future.get();
        return true;
    } catch (...) {
        return false;
    }
}

void FcitxDispatcher::forgetConnection(std::uint64_t connectionId) {
    if (!accepting_.load(std::memory_order_acquire) || !dispatcher_ || connectionId == 0)
        return;
    dispatcher_->schedule(
        [this, connectionId] { runtime_->forgetConnection(connectionId); });
}

void FcitxDispatcher::stop() {
    accepting_.store(false, std::memory_order_release);
    if (dispatcher_ && runtime_) {
        dispatcher_->schedule([this] { runtime_->eventLoop().exit(); });
    }
    if (thread_.joinable()) thread_.join();
}

} // namespace fcitx::windows::engine
