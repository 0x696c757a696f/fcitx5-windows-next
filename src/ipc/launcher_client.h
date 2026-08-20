#pragma once

#include "peer_verification.h"
#include "protocol.h"
#include "runtime_identity.h"

#include <cstdint>
#include <string_view>

namespace fcitx::windows::ipc {

[[nodiscard]] bool sendLauncherCommand(const platform::RuntimeIdentity& identity,
                                       std::uint64_t absoluteDeadlineMilliseconds,
                                       const PeerPolicy& peerPolicy,
                                       protocol::LauncherCommand command,
                                       protocol::LauncherResponse& response) noexcept;
[[nodiscard]] bool sendLauncherCommand(const platform::RuntimeIdentity& identity,
                                       std::wstring_view generation,
                                       std::uint64_t absoluteDeadlineMilliseconds,
                                       const PeerPolicy& peerPolicy,
                                       protocol::LauncherCommand command,
                                       protocol::LauncherResponse& response) noexcept;

[[nodiscard]] bool requestLauncherStart(const platform::RuntimeIdentity& identity,
                                        std::uint64_t absoluteDeadlineMilliseconds,
                                        const PeerPolicy& peerPolicy) noexcept;
[[nodiscard]] bool requestLauncherStart(const platform::RuntimeIdentity& identity,
                                        std::wstring_view generation,
                                        std::uint64_t absoluteDeadlineMilliseconds,
                                        const PeerPolicy& peerPolicy) noexcept;

} // namespace fcitx::windows::ipc
