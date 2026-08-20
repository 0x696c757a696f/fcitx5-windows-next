#pragma once

#include "peer_verification.h"
#include "protocol.h"
#include "runtime_identity.h"

#include <Windows.h>

#include <atomic>
#include <cstdint>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

namespace fcitx::windows::ipc {

inline constexpr DWORD kInputDeadlineMilliseconds = 100;
// The hot input path stays at 100 ms. A brand-new TSF context is allowed a
// wider, still bounded startup window because some Fcitx addons lazily create
// per-user cache/config state on first focus in an empty FCITX_USER_DATA_ROOT.
// This replaces the old synthetic "n" warmup: correctness first, no fake user
// key, but the first real context is not dropped solely because initialization
// crossed the hot-path deadline.
inline constexpr DWORD kContextStartDeadlineMilliseconds = 3000;

struct KeyResult {
    struct Candidate {
        std::uint64_t id{};
        std::wstring label;
        std::wstring text;
        std::wstring comment;
    };
    bool handled{};
    std::uint64_t engineEpoch{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    std::wstring commit;
    std::wstring preedit;
    std::uint32_t preeditCaretUtf16{};
    std::vector<Candidate> candidates;
    std::uint32_t selectedCandidate{UINT32_MAX};
    std::uint32_t candidatePage{};
    std::uint32_t candidateTotal{};
    std::uint8_t candidateVisibility{};
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

class PipeClient final {
public:
    PipeClient();
    explicit PipeClient(std::wstring pipeName,
                        PeerPolicy peerPolicy = PeerPolicy::development(),
                        std::wstring launcherGeneration = {});
    ~PipeClient();

    PipeClient(const PipeClient&) = delete;
    PipeClient& operator=(const PipeClient&) = delete;

    [[nodiscard]] bool processKey(std::uint64_t contextId, std::uint32_t virtualKey,
                                  std::uint32_t keyFlags, KeyResult& result,
                                  const protocol::CaretRect& caret = {},
                                  bool popupAllowed = true,
                                  std::uint32_t scanCode = 0,
                                  bool extendedKey = false,
                                  std::uint64_t keyboardLayout = 0,
                                  std::string_view logicalText = {},
                                  std::string_view inputMethod = {},
                                  bool surroundingTextValid = false,
                                  std::string_view surroundingText = {},
                                  std::uint32_t surroundingCursor = 0,
                                  std::uint32_t surroundingAnchor = 0) noexcept;
    [[nodiscard]] bool selectCandidate(std::uint32_t targetProcessId,
                                       std::uint64_t expectedEngineEpoch,
                                       std::uint64_t contextId,
                                       std::uint64_t compositionId,
                                       std::uint64_t revision,
                                       std::uint64_t candidateId) noexcept;
    [[nodiscard]] bool pollState(std::uint64_t contextId, KeyResult& result) noexcept;
    [[nodiscard]] bool queryEngineStatus(protocol::EngineStatusResponse& result,
                                         DWORD timeoutMilliseconds =
                                             kInputDeadlineMilliseconds) noexcept;
    void disconnect() noexcept;

private:
    struct ContextState {
        std::uint64_t compositionId{};
        std::uint64_t revision{};
    };

    [[nodiscard]] bool connect(std::uint64_t deadline) noexcept;
    [[nodiscard]] bool handshake(std::uint64_t deadline) noexcept;
    [[nodiscard]] bool transact(const std::vector<std::uint8_t>& request,
                                std::vector<std::uint8_t>& response,
                                std::uint64_t deadline) noexcept;
    [[nodiscard]] bool transfer(bool write, void* data, std::size_t size,
                                std::uint64_t deadline) noexcept;
    [[nodiscard]] bool acceptKeyResponse(const protocol::KeyResponse& response,
                                         std::uint64_t requestId,
                                         std::uint64_t contextId,
                                         ContextState& contextState,
                                         KeyResult& result) noexcept;

    std::wstring pipeName_;
    std::wstring launcherGeneration_;
    PeerPolicy peerPolicy_;
    platform::RuntimeIdentity identity_;
    HANDLE pipe_{INVALID_HANDLE_VALUE};
    bool handshakeComplete_{};
    std::uint64_t engineEpoch_{};
    std::uint32_t sessionId_{};
    std::atomic<std::uint64_t> nextRequestId_{1};
    std::unordered_map<std::uint64_t, ContextState> contexts_;
};

} // namespace fcitx::windows::ipc
