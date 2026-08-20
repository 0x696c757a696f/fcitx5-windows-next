#pragma once

#include <cstddef>
#include <cstdint>
#include <span>
#include <string>
#include <vector>

namespace fcitx::windows::protocol {

inline constexpr std::uint32_t kMagic = 0x34574346U; // "FCW4"
inline constexpr std::uint16_t kVersion = 14;
inline constexpr std::size_t kHeaderSize = 64;
inline constexpr std::size_t kMaxHotFrameSize = 256U * 1024U;
inline constexpr std::size_t kMaxControlFrameSize = 1024U * 1024U;
inline constexpr std::size_t kMaxFrameSize = kMaxHotFrameSize;
inline constexpr std::size_t kMaxCommitUtf8 = 16U * 1024U;
inline constexpr std::size_t kMaxPreeditUtf8 = 16U * 1024U;
inline constexpr std::size_t kMaxCandidates = 128;
inline constexpr std::size_t kMaxCandidateFieldUtf8 = 4096;
inline constexpr std::size_t kMaxLogicalKeyUtf8 = 64;
inline constexpr std::size_t kMaxInputMethodIdUtf8 = 64;
inline constexpr std::size_t kMaxInputMethodNameUtf8 = 128;
inline constexpr std::size_t kMaxLocaleUtf8 = 35;
inline constexpr std::size_t kMaxSurroundingTextUtf8 = 16U * 1024U;

enum class MessageType : std::uint16_t {
    helloRequest = 1,
    helloResponse = 2,
    keyRequest = 3,
    keyResponse = 4,
    launcherRequest = 5,
    launcherResponse = 6,
    candidateSelectRequest = 7,
    candidateSelectResponse = 8,
    stateRequest = 9,
    engineStatusRequest = 10,
    engineStatusResponse = 11,
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

struct CaretRect {
    bool valid{};
    std::int32_t left{};
    std::int32_t top{};
    std::int32_t right{};
    std::int32_t bottom{};
    std::uint32_t dpi{96};

    bool operator==(const CaretRect&) const = default;
};

enum KeyFlag : std::uint32_t {
    kKeyFlagShift = 1U << 0U,
    kKeyFlagControl = 1U << 1U,
    kKeyFlagAlt = 1U << 2U,
    kKeyFlagSuper = 1U << 3U,
    // Distinguishes a key-up event from a key-down. Key-up events are routed
    // to the engine so Fcitx can track modifier release (e.g. Ctrl+Shift IME
    // switching) without the TSF whitelist deciding which keys matter.
    kKeyFlagRelease = 1U << 4U,
    // AltGr appears to many Windows clients as Ctrl+RightAlt. Keeping it as a
    // first-class bit prevents non-US layouts from looking like generic
    // Ctrl+Alt shortcuts at the protocol boundary.
    kKeyFlagAltGr = 1U << 5U,
    // ToUnicodeEx reported a dead key. The logical text, when present, is the
    // dead key's display character and must not be treated as host fallback
    // commit text.
    kKeyFlagDeadKey = 1U << 6U,
};

inline constexpr std::uint32_t kKnownKeyFlags =
    kKeyFlagShift | kKeyFlagControl | kKeyFlagAlt | kKeyFlagSuper |
    kKeyFlagRelease | kKeyFlagAltGr | kKeyFlagDeadKey;

struct KeyRequest {
    Metadata metadata;
    std::uint32_t virtualKey{};
    std::uint32_t keyFlags{};
    std::uint32_t scanCode{};
    bool extendedKey{};
    bool popupAllowed{true};
    std::uint64_t keyboardLayout{};
    std::string logicalTextUtf8;
    std::string inputMethodUtf8;
    bool surroundingTextValid{};
    std::string surroundingTextUtf8;
    std::uint32_t surroundingCursor{};
    std::uint32_t surroundingAnchor{};
    CaretRect caret;
};

struct CandidateRecord {
    std::uint64_t id{};
    std::string labelUtf8;
    std::string textUtf8;
    std::string commentUtf8;

    bool operator==(const CandidateRecord&) const = default;
};

struct KeyResponse {
    Metadata metadata;
    Status status{Status::malformed};
    bool handled{};
    std::string commitUtf8;
    std::string preeditUtf8;
    std::uint32_t preeditCaretUtf8{};
    std::vector<CandidateRecord> candidates;
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
    CaretRect caret;
    bool popupAllowed{true};
    std::string contentLocaleUtf8;
};

struct CandidateSelectRequest {
    Metadata metadata;
    std::uint32_t targetProcessId{};
    std::uint64_t candidateId{};
};

struct CandidateSelectResponse {
    Metadata metadata;
    Status status{Status::malformed};
};

struct StateRequest {
    Metadata metadata;
};

struct EngineStatusRequest {
    Metadata metadata;
};

struct EngineStatusResponse {
    Metadata metadata;
    Status status{Status::malformed};
    std::string currentInputMethodId;
    std::string currentInputMethodName;
    std::string currentInputMethodNativeName;
    std::string currentInputMethodShortLabel;
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
    std::string currentInputMethodId;
    std::string currentInputMethodName;
    std::string currentInputMethodNativeName;
    std::string currentInputMethodShortLabel;
};

[[nodiscard]] bool isRequest(MessageType type) noexcept;
[[nodiscard]] bool isResponse(MessageType type) noexcept;
[[nodiscard]] bool decodeFrame(std::span<const std::uint8_t> bytes, FrameView& output) noexcept;
[[nodiscard]] bool decodeHeader(std::span<const std::uint8_t> bytes, MessageType& type,
                                std::uint32_t& bodySize, Metadata& metadata) noexcept;

[[nodiscard]] std::vector<std::uint8_t> encode(const HelloRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const HelloResponse& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const KeyRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const KeyResponse& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const CandidateSelectRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const CandidateSelectResponse& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const StateRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const EngineStatusRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const EngineStatusResponse& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const LauncherRequest& message);
[[nodiscard]] std::vector<std::uint8_t> encode(const LauncherResponse& message);

[[nodiscard]] bool decode(const FrameView& frame, HelloRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, HelloResponse& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, KeyRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, KeyResponse& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, CandidateSelectRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, CandidateSelectResponse& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, StateRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, EngineStatusRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, EngineStatusResponse& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, LauncherRequest& message) noexcept;
[[nodiscard]] bool decode(const FrameView& frame, LauncherResponse& message) noexcept;

} // namespace fcitx::windows::protocol
