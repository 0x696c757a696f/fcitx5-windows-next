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

    [[nodiscard]] bool start();
    [[nodiscard]] bool processKey(const ClientContextKey& context,
                                  const protocol::KeyRequest& request,
                                  RuntimeResult& result,
                                  std::chrono::milliseconds timeout);
    void stop();

private:
    std::thread thread_;
    std::unique_ptr<FcitxRuntime> runtime_;
    std::unique_ptr<::fcitx::EventDispatcher> dispatcher_;
    std::atomic<bool> accepting_{};
};

} // namespace fcitx::windows::engine
