#pragma once

#include "protocol.h"

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

namespace fcitx {
class EventLoop;
}

namespace fcitx::windows::engine {

struct ClientContextKey {
    std::uint32_t processId{};
    std::uint64_t connectionId{};
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
    std::vector<protocol::CandidateRecord> candidates;
    std::uint32_t selectedCandidate{UINT32_MAX};
    std::uint32_t candidatePage{};
    std::uint32_t candidateTotal{};
    std::uint8_t candidateVisibility{};
    std::uint32_t candidatePageSize{};
    bool candidateBulk{};
    bool candidateEnd{};
    bool deleteSurroundingText{};
    std::int32_t deleteSurroundingOffset{};
    std::uint32_t deleteSurroundingSize{};
    bool forwardKey{};
    std::uint32_t forwardKeySym{};
    std::uint32_t forwardKeyStates{};
    std::int32_t forwardKeyCode{};
    bool forwardKeyRelease{};
    protocol::CaretRect caret;
};

struct InputMethodInfo {
    std::string id;
    std::string name;
    std::string nativeName;
    bool selected{};
};

struct InputMethodStatus {
    std::string id;
    std::string name;
    std::string nativeName;
    std::string shortLabel;
};

class FcitxRuntime final {
  public:
    FcitxRuntime();
    ~FcitxRuntime();

    FcitxRuntime(const FcitxRuntime&) = delete;
    FcitxRuntime& operator=(const FcitxRuntime&) = delete;

    [[nodiscard]] bool initialize(bool safeMode = false);
    [[nodiscard]] ::fcitx::EventLoop& eventLoop();
    [[nodiscard]] RuntimeResult processKey(const ClientContextKey& key,
                                           const protocol::KeyRequest& request);
    [[nodiscard]] RuntimeResult selectCandidate(
        std::uint32_t targetProcessId,
        const protocol::CandidateSelectRequest& request);
    [[nodiscard]] RuntimeResult takePendingState(
        const ClientContextKey& key, const protocol::StateRequest& request);
    [[nodiscard]] std::vector<InputMethodInfo> inputMethods() const;
    [[nodiscard]] InputMethodStatus currentInputMethod() const;
    [[nodiscard]] bool setDefaultInputMethod(std::string_view id);
    void forgetConnection(std::uint64_t connectionId);

    class Impl;

  private:
    std::unique_ptr<Impl> impl_;
};

} // namespace fcitx::windows::engine
