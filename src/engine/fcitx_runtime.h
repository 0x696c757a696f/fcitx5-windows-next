#pragma once

#include "protocol.h"

#include <cstdint>
#include <memory>

namespace fcitx {
class EventLoop;
}

namespace fcitx::windows::engine {

struct ClientContextKey {
    std::uint32_t processId{};
    std::uint64_t contextId{};

    bool operator==(const ClientContextKey&) const = default;
};

struct RuntimeResult {
    bool handled{};
    std::string commitUtf8;
    std::string preeditUtf8;
    std::uint32_t preeditCaretUtf8{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
};

class FcitxRuntime final {
public:
    FcitxRuntime();
    ~FcitxRuntime();

    FcitxRuntime(const FcitxRuntime&) = delete;
    FcitxRuntime& operator=(const FcitxRuntime&) = delete;

    [[nodiscard]] bool initialize();
    [[nodiscard]] ::fcitx::EventLoop& eventLoop();
    [[nodiscard]] RuntimeResult processKey(const ClientContextKey& key,
                                           const protocol::KeyRequest& request);
    void forgetProcess(std::uint32_t processId);

    class Impl;

private:
    std::unique_ptr<Impl> impl_;
};

} // namespace fcitx::windows::engine
