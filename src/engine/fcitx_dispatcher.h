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
    [[nodiscard]] bool processKey(const ClientContextKey& context,
                                  const protocol::KeyRequest& request,
                                  RuntimeResult& result,
                                  std::chrono::milliseconds timeout);
    [[nodiscard]] bool selectCandidate(
        std::uint32_t targetProcessId,
        const protocol::CandidateSelectRequest& request,
        RuntimeResult& result, std::chrono::milliseconds timeout);
    [[nodiscard]] bool takePendingState(
        const ClientContextKey& context, const protocol::StateRequest& request,
        RuntimeResult& result, std::chrono::milliseconds timeout);
    void forgetConnection(std::uint64_t connectionId);
    void stop();

private:
    std::thread thread_;
    std::unique_ptr<FcitxRuntime> runtime_;
    std::unique_ptr<::fcitx::EventDispatcher> dispatcher_;
    std::atomic<bool> accepting_{};
};

} // namespace fcitx::windows::engine
