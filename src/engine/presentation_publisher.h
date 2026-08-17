#pragma once

#include "protocol.h"

#include <Windows.h>

#include <condition_variable>
#include <mutex>
#include <optional>
#include <string>
#include <thread>

namespace fcitx::windows::engine {

class PresentationPublisher final {
public:
    PresentationPublisher(std::wstring pipeName, std::wstring uiExecutable);
    ~PresentationPublisher();
    PresentationPublisher(const PresentationPublisher&) = delete;
    PresentationPublisher& operator=(const PresentationPublisher&) = delete;

    void publish(protocol::KeyResponse response);

private:
    void run();
    bool connect();
    bool send(const protocol::KeyResponse& response);
    void disconnect() noexcept;

    std::wstring pipeName_;
    std::wstring uiExecutable_;
    std::mutex mutex_;
    std::condition_variable condition_;
    std::optional<protocol::KeyResponse> latest_;
    bool stopping_{};
    std::thread thread_;
    HANDLE pipe_{INVALID_HANDLE_VALUE};
};

} // namespace fcitx::windows::engine
