#pragma once

#include <cstddef>
#include <cstdint>
#include <span>
#include <string>
#include <vector>

namespace fcitx::windows::protocol {

inline constexpr std::uint32_t kMagic = 0x32574346U; // "FCW2"
inline constexpr std::uint16_t kVersion = 2;
inline constexpr std::size_t kHeaderSize = 12;
inline constexpr std::size_t kMaxFrameSize = 4096;

enum class MessageType : std::uint16_t {
    helloRequest = 1,
    helloResponse = 2,
    keyRequest = 3,
    keyResponse = 4,
};

enum class Status : std::uint32_t {
    ok = 0,
    malformed = 1,
    versionMismatch = 2,
    unsupported = 3,
};

struct FrameView {
    MessageType type{};
    std::span<const std::uint8_t> body;
};

struct HelloRequest {
    std::uint64_t requestId{};
    std::uint32_t clientArchitectureBits{};
};

struct HelloResponse {
    std::uint64_t responseTo{};
    Status status{Status::malformed};
    std::uint64_t engineEpoch{};
};

struct KeyRequest {
    std::uint64_t requestId{};
    std::uint64_t contextId{};
    std::uint32_t virtualKey{};
    std::uint32_t keyFlags{};
};

struct KeyResponse {
    std::uint64_t responseTo{};
    Status status{Status::malformed};
    bool handled{};
    std::string commitUtf8;
};

[[nodiscard]] bool decodeFrame(std::span<const std::uint8_t> bytes,
                               FrameView& output) noexcept;
[[nodiscard]] bool decodeHeader(std::span<const std::uint8_t> bytes,
                                MessageType& type,
                                std::uint32_t& bodySize) noexcept;

[[nodiscard]] std::vector<std::uint8_t> encode(const HelloRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const HelloResponse& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const KeyRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const KeyResponse& message);

[[nodiscard]] bool decode(const FrameView& frame, HelloRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, HelloResponse& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, KeyRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, KeyResponse& message) noexcept;

} // namespace fcitx::windows::protocol
