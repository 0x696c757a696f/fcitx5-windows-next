// C ABI surface of the Rust `fcitx5-protocol-core` crate.
//
// This header declares the flat structures and functions exported by
// `rust/protocol-core/src/capi.rs`. Rust owns the codec: all validation and
// byte layout live in Rust; every C++ consumer (engine, IPC clients, and the
// candidate UI presentation host) calls these functions directly.
//
// ABI conventions (see capi.rs for the authoritative documentation):
//
// * Encode returns 1 on success; 0 on rejection (no `*out_length` write) or
//   on insufficient space (`*out_length` = required size).
// * Decode returns 1 on success; 0 on rejection (needed counters untouched)
//   or on insufficient space (needed counters = required sizes). String bytes
//   are written into the caller-provided `strings` arena and each output
//   string field points into it.

#pragma once

#include <cstddef>
#include <cstdint>

extern "C" {

struct FcitxMetadataC {
    std::uint64_t requestId;
    std::uint64_t responseTo;
    std::uint64_t engineEpoch;
    std::uint32_t sessionId;
    std::uint64_t contextId;
    std::uint64_t compositionId;
    std::uint64_t revision;
};

struct FcitxCaretRectC {
    std::uint8_t valid;
    std::int32_t left;
    std::int32_t top;
    std::int32_t right;
    std::int32_t bottom;
    std::uint32_t dpi;
};

struct FcitxBytesC {
    const std::uint8_t* data;
    std::size_t len;
};

enum FcitxProtocolKeyFlagC : std::uint32_t {
    FCITX5_PROTOCOL_KEY_FLAG_SHIFT = 1U << 0U,
    FCITX5_PROTOCOL_KEY_FLAG_CONTROL = 1U << 1U,
    FCITX5_PROTOCOL_KEY_FLAG_ALT = 1U << 2U,
    FCITX5_PROTOCOL_KEY_FLAG_SUPER = 1U << 3U,
    FCITX5_PROTOCOL_KEY_FLAG_RELEASE = 1U << 4U,
    FCITX5_PROTOCOL_KEY_FLAG_ALTGR = 1U << 5U,
    FCITX5_PROTOCOL_KEY_FLAG_DEAD_KEY = 1U << 6U,
};

struct FcitxHelloRequestC {
    FcitxMetadataC metadata;
    std::uint32_t clientArchitectureBits;
    std::uint32_t clientProcessId;
};

struct FcitxHelloResponseC {
    FcitxMetadataC metadata;
    std::uint32_t status;
    std::uint32_t serverArchitectureBits;
};

struct FcitxKeyRequestC {
    FcitxMetadataC metadata;
    std::uint32_t virtualKey;
    std::uint32_t keyFlags;
    std::uint32_t scanCode;
    std::uint8_t extendedKey;
    std::uint8_t popupAllowed;
    std::uint64_t keyboardLayout;
    FcitxBytesC logicalText;
    FcitxBytesC inputMethod;
    std::uint8_t surroundingTextValid;
    FcitxBytesC surroundingText;
    std::uint32_t surroundingCursor;
    std::uint32_t surroundingAnchor;
    FcitxCaretRectC caret;
};

struct FcitxCandidateRecordC {
    std::uint64_t id;
    FcitxBytesC label;
    FcitxBytesC text;
    FcitxBytesC comment;
};

struct FcitxKeyResponseC {
    FcitxMetadataC metadata;
    std::uint32_t status;
    std::uint8_t handled;
    FcitxBytesC commit;
    FcitxBytesC preedit;
    std::uint32_t preeditCaretUtf8;
    std::uint32_t selectedCandidate;
    std::uint32_t candidatePage;
    std::uint32_t candidateTotal;
    std::uint8_t candidateVisibility;
    std::uint32_t candidatePageSize;
    std::uint8_t candidateBulk;
    std::uint8_t candidateEnd;
    std::uint8_t deleteSurroundingText;
    std::int32_t deleteSurroundingOffset;
    std::uint32_t deleteSurroundingSize;
    std::uint8_t forwardKey;
    std::uint32_t forwardKeySym;
    std::uint32_t forwardKeyStates;
    std::int32_t forwardKeyCode;
    std::uint8_t forwardKeyRelease;
    FcitxCaretRectC caret;
    std::uint8_t popupAllowed;
    FcitxBytesC contentLocale;
    const FcitxCandidateRecordC* candidates;
    std::size_t candidateCount;
};

struct FcitxCandidateSelectRequestC {
    FcitxMetadataC metadata;
    std::uint32_t targetProcessId;
    std::uint64_t candidateId;
};

struct FcitxCandidateSelectResponseC {
    FcitxMetadataC metadata;
    std::uint32_t status;
};

struct FcitxStateRequestC {
    FcitxMetadataC metadata;
};

struct FcitxEngineStatusRequestC {
    FcitxMetadataC metadata;
};

struct FcitxEngineStatusResponseC {
    FcitxMetadataC metadata;
    std::uint32_t status;
    FcitxBytesC currentInputMethodId;
    FcitxBytesC currentInputMethodName;
    FcitxBytesC currentInputMethodNativeName;
    FcitxBytesC currentInputMethodShortLabel;
};

struct FcitxLauncherRequestC {
    FcitxMetadataC metadata;
    std::uint32_t command;
};

struct FcitxLauncherResponseC {
    FcitxMetadataC metadata;
    std::uint32_t status;
    std::uint32_t launcherState;
    std::uint32_t engineState;
    std::uint32_t startDisposition;
    std::uint8_t safeMode;
    std::uint64_t retryAfterMilliseconds;
    FcitxBytesC currentInputMethodId;
    FcitxBytesC currentInputMethodName;
    FcitxBytesC currentInputMethodNativeName;
    FcitxBytesC currentInputMethodShortLabel;
};

std::uint8_t fcitx5_protocol_core_decode_header(
    const std::uint8_t* bytes, std::size_t length, std::uint16_t* outType,
    std::uint32_t* outBodySize, FcitxMetadataC* outMetadata);

std::uint8_t fcitx5_protocol_core_encode_hello_request(
    const FcitxHelloRequestC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);
std::uint8_t fcitx5_protocol_core_encode_hello_response(
    const FcitxHelloResponseC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);
std::uint8_t fcitx5_protocol_core_encode_key_request(
    const FcitxKeyRequestC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);
std::uint8_t fcitx5_protocol_core_encode_key_response(
    const FcitxKeyResponseC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);
std::uint8_t fcitx5_protocol_core_encode_candidate_select_request(
    const FcitxCandidateSelectRequestC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);
std::uint8_t fcitx5_protocol_core_encode_candidate_select_response(
    const FcitxCandidateSelectResponseC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);
std::uint8_t fcitx5_protocol_core_encode_state_request(
    const FcitxStateRequestC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);
std::uint8_t fcitx5_protocol_core_encode_engine_status_request(
    const FcitxEngineStatusRequestC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);
std::uint8_t fcitx5_protocol_core_encode_engine_status_response(
    const FcitxEngineStatusResponseC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);
std::uint8_t fcitx5_protocol_core_encode_launcher_request(
    const FcitxLauncherRequestC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);
std::uint8_t fcitx5_protocol_core_encode_launcher_response(
    const FcitxLauncherResponseC* message, std::uint8_t* out, std::size_t outCapacity,
    std::size_t* outLength);

std::uint8_t fcitx5_protocol_core_decode_hello_request(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxHelloRequestC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded);
std::uint8_t fcitx5_protocol_core_decode_hello_response(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxHelloResponseC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded);
std::uint8_t fcitx5_protocol_core_decode_key_request(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxKeyRequestC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded);
std::uint8_t fcitx5_protocol_core_decode_key_response(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxKeyResponseC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded, FcitxCandidateRecordC* candidates,
    std::size_t candidatesCapacity, std::size_t* candidatesNeeded);
std::uint8_t fcitx5_protocol_core_decode_candidate_select_request(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxCandidateSelectRequestC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded);
std::uint8_t fcitx5_protocol_core_decode_candidate_select_response(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxCandidateSelectResponseC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded);
std::uint8_t fcitx5_protocol_core_decode_state_request(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxStateRequestC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded);
std::uint8_t fcitx5_protocol_core_decode_engine_status_request(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxEngineStatusRequestC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded);
std::uint8_t fcitx5_protocol_core_decode_engine_status_response(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxEngineStatusResponseC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded);
std::uint8_t fcitx5_protocol_core_decode_launcher_request(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxLauncherRequestC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded);
std::uint8_t fcitx5_protocol_core_decode_launcher_response(
    const FcitxMetadataC* metadata, const std::uint8_t* body, std::size_t bodyLength,
    FcitxLauncherResponseC* out, std::uint8_t* strings, std::size_t stringsCapacity,
    std::size_t* stringsNeeded);

} // extern "C"
