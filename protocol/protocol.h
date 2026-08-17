#pragma once

#include <cstddef>
#include <cstdint>
#include <span>
#include <string>
#include <vector>

namespace fcitx::windows::protocol {

inline constexpr std::uint32_t kMagic = 0x32574346U; // "FCW2"
inline constexpr std::uint16_t kVersion = 2;
inline constexpr std::size_t kHeaderSize = 64;
inline constexpr std::size_t kMaxHotFrameSize = 256U * 1024U;
inline constexpr std::size_t kMaxControlFrameSize = 1024U * 1024U;
inline constexpr std::size_t kMaxFrameSize = kMaxHotFrameSize;
inline constexpr std::size_t kMaxCommitUtf8 = 32;

enum class MessageType : std::uint16_t {
    helloRequest = 1,
    helloResponse = 2,
    keyRequest = 3,
    keyResponse = 4,
    launcherRequest = 5,
    launcherResponse = 6,
};

enum class Status : std::uint32_t {
    ok = 0,
    malformed = 1,
    versionMismatch = 2,
    unsupported = 3,
    staleIdentity = 4,
    accessDenied = 5,
};

struct Metadata {
    std::uint64_t requestId{};
    std::uint64_t responseTo{};
    std::uint64_t engineEpoch{};
    std::uint32_t sessionId{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};

    bool operator==(const Metadata&) const = default;
};

struct FrameView {
    MessageType type{};
    Metadata metadata;
    std::span<const std::uint8_t> body;
};

struct HelloRequest {
    Metadata metadata;
    std::uint32_t clientArchitectureBits{};
    std::uint32_t clientProcessId{};
};

struct HelloResponse {
    Metadata metadata;
    Status status{Status::malformed};
    std::uint32_t serverArchitectureBits{};
};

struct KeyRequest {
    Metadata metadata;
    std::uint32_t virtualKey{};
    std::uint32_t keyFlags{};
};

struct KeyResponse {
    Metadata metadata;
    Status status{Status::malformed};
    bool handled{};
    std::string commitUtf8;
};

enum class LauncherCommand : std::uint32_t {
    startDemand = 1,
    userStop = 2,
    resume = 3,
    beginUpdate = 4,
    endUpdate = 5,
    beginUninstall = 6,
    resetSafeMode = 7,
    status = 8,
    shutdown = 9,
};

struct LauncherRequest {
    Metadata metadata;
    LauncherCommand command{LauncherCommand::status};
};

struct LauncherResponse {
    Metadata metadata;
    Status status{Status::malformed};
    std::uint32_t launcherState{};
    std::uint32_t engineState{};
    std::uint32_t startDisposition{};
    bool safeMode{};
    std::uint64_t retryAfterMilliseconds{};
};

[[nodiscard]] bool isRequest(MessageType type) noexcept;
[[nodiscard]] bool isResponse(MessageType type) noexcept;
[[nodiscard]] bool decodeFrame(std::span<const std::uint8_t> bytes,
                               FrameView& output) noexcept;
[[nodiscard]] bool decodeHeader(std::span<const std::uint8_t> bytes,
                                MessageType& type,
                                std::uint32_t& bodySize,
                                Metadata& metadata) noexcept;

[[nodiscard]] std::vector<std::uint8_t> encode(const HelloRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const HelloResponse& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const KeyRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const KeyResponse& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const LauncherRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const LauncherResponse& message);

[[nodiscard]] bool decode(const FrameView& frame, HelloRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, HelloResponse& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, KeyRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, KeyResponse& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, LauncherRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, LauncherResponse& message) noexcept;

} // namespace fcitx::windows::protocol
