#pragma once

#include "peer_verification.h"
#include "runtime_identity.h"

#include <cstdint>
#include <string>
#include <string_view>

namespace fcitx::windows::ipc {

struct LauncherResponse {
    std::uint64_t requestId{};
    std::uint64_t responseTo{};
    std::uint64_t engineEpoch{};
    std::uint32_t sessionId{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    std::uint32_t status{};
    std::uint32_t launcherState{};
    std::uint32_t engineState{};
    std::uint32_t startDisposition{};
    bool safeMode{};
    std::uint64_t retryAfterMilliseconds{};
    std::string currentInputMethodId;
    std::string currentInputMethodName;
    std::string currentInputMethodNativeName;
    std::string currentInputMethodShortLabel;
};

[[nodiscard]] bool sendLauncherCommand(const platform::RuntimeIdentity& identity,
                                       std::uint64_t absoluteDeadlineMilliseconds,
                                       const PeerPolicy& peerPolicy,
                                       std::uint32_t command,
                                       LauncherResponse& response) noexcept;
[[nodiscard]] bool sendLauncherCommand(const platform::RuntimeIdentity& identity,
                                       std::wstring_view generation,
                                       std::uint64_t absoluteDeadlineMilliseconds,
                                       const PeerPolicy& peerPolicy,
                                       std::uint32_t command,
                                       LauncherResponse& response) noexcept;

[[nodiscard]] bool requestLauncherStart(const platform::RuntimeIdentity& identity,
                                        std::uint64_t absoluteDeadlineMilliseconds,
                                        const PeerPolicy& peerPolicy) noexcept;
[[nodiscard]] bool requestLauncherStart(const platform::RuntimeIdentity& identity,
                                        std::wstring_view generation,
                                        std::uint64_t absoluteDeadlineMilliseconds,
                                        const PeerPolicy& peerPolicy) noexcept;

} // namespace fcitx::windows::ipc
