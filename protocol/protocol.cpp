// FCW4 protocol adapter.
//
// E1 cutover: the codec (validation, byte layout, decode rejection) is now
// authoritative in the Rust `fcitx5-protocol-core` crate and reached through
// the narrow C ABI in `protocol_ffi.h`. This file only marshals between the
// `protocol.h` DTOs and the flat C structures; `protocol.h`'s API and all call
// sites are unchanged.

#include "protocol.h"
#include "protocol_ffi.h"

#include <array>
#include <span>
#include <string>
#include <vector>

namespace fcitx::windows::protocol {
namespace {

FcitxMetadataC toC(const Metadata& metadata) {
    return FcitxMetadataC{metadata.requestId, metadata.responseTo, metadata.engineEpoch,
                          metadata.sessionId, metadata.contextId, metadata.compositionId,
                          metadata.revision};
}

Metadata fromC(const FcitxMetadataC& metadata) {
    return Metadata{metadata.requestId, metadata.responseTo, metadata.engineEpoch,
                    metadata.sessionId, metadata.contextId, metadata.compositionId,
                    metadata.revision};
}

std::uint8_t toFlag(bool value) { return value ? 1U : 0U; }

FcitxCaretRectC toC(const CaretRect& caret) {
    return FcitxCaretRectC{toFlag(caret.valid), caret.left, caret.top,
                           caret.right, caret.bottom, caret.dpi};
}

CaretRect fromC(const FcitxCaretRectC& caret) {
    return CaretRect{caret.valid != 0, caret.left, caret.top,
                     caret.right, caret.bottom, caret.dpi};
}

FcitxBytesC toC(const std::string& value) {
    return FcitxBytesC{reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

std::string fromC(const FcitxBytesC& value) {
    if (value.len == 0) {
        return {};
    }
    return std::string(reinterpret_cast<const char*>(value.data), value.len);
}

// Encodes through the Rust FFI. Tries a stack buffer first so typical frames
// cost a single call; larger frames fall back to an exact heap allocation.
template <typename C, typename FFI>
std::vector<std::uint8_t> encodeVia(FFI ffi, const C& message) {
    std::array<std::uint8_t, 512> stack{};
    std::size_t written = 0;
    if (ffi(&message, stack.data(), stack.size(), &written)) {
        return std::vector<std::uint8_t>(stack.begin(), stack.begin() + written);
    }
    if (written == 0) {
        return {}; // rejected by validation
    }
    std::vector<std::uint8_t> out(written);
    std::size_t finalWritten = 0;
    if (!ffi(&message, out.data(), out.size(), &finalWritten)) {
        return {};
    }
    out.resize(finalWritten);
    return out;
}

// Decodes through the Rust FFI into `out`, then runs `fill` with the decoded
// C structure. Tries a stack string arena first so typical frames cost a
// single call.
template <typename C, typename FFI, typename Fill>
bool decodeVia(FFI ffi, const FcitxMetadataC& metadata, std::span<const std::uint8_t> body, C& out,
               Fill&& fill) {
    std::array<std::uint8_t, 4096> stack{};
    std::size_t stringsNeeded = 0;
    if (ffi(&metadata, body.data(), body.size(), &out, stack.data(), stack.size(),
            &stringsNeeded)) {
        fill(out);
        return true;
    }
    if (stringsNeeded == 0) {
        return false; // rejected by validation
    }
    std::vector<std::uint8_t> heap(stringsNeeded);
    std::size_t finalNeeded = 0;
    if (!ffi(&metadata, body.data(), body.size(), &out, heap.data(), heap.size(),
             &finalNeeded)) {
        return false;
    }
    fill(out);
    return true;
}

} // namespace

bool isRequest(MessageType type) noexcept {
    return type == MessageType::helloRequest || type == MessageType::keyRequest ||
           type == MessageType::launcherRequest ||
           type == MessageType::candidateSelectRequest || type == MessageType::stateRequest ||
           type == MessageType::engineStatusRequest;
}

bool isResponse(MessageType type) noexcept {
    return type == MessageType::helloResponse || type == MessageType::keyResponse ||
           type == MessageType::launcherResponse ||
           type == MessageType::candidateSelectResponse ||
           type == MessageType::engineStatusResponse;
}

bool decodeHeader(std::span<const std::uint8_t> bytes, MessageType& type, std::uint32_t& bodySize,
                  Metadata& metadata) noexcept {
    std::uint16_t rawType = 0;
    FcitxMetadataC outMetadata{};
    if (fcitx5_protocol_core_decode_header(bytes.data(), bytes.size(), &rawType, &bodySize,
                                           &outMetadata) == 0) {
        return false;
    }
    type = static_cast<MessageType>(rawType);
    metadata = fromC(outMetadata);
    return true;
}

bool decodeFrame(std::span<const std::uint8_t> bytes, FrameView& output) noexcept {
    if (bytes.size() < kHeaderSize || bytes.size() > kMaxControlFrameSize)
        return false;
    std::uint32_t bodySize = 0;
    Metadata metadata;
    MessageType type{};
    if (!decodeHeader(bytes.first(kHeaderSize), type, bodySize, metadata) ||
        bodySize != bytes.size() - kHeaderSize) {
        return false;
    }
    output = FrameView{type, metadata, bytes.subspan(kHeaderSize)};
    return true;
}

std::vector<std::uint8_t> encode(const HelloRequest& message) {
    const FcitxHelloRequestC marshalled{toC(message.metadata),
                                        message.clientArchitectureBits,
                                        message.clientProcessId};
    return encodeVia(fcitx5_protocol_core_encode_hello_request, marshalled);
}

std::vector<std::uint8_t> encode(const HelloResponse& message) {
    const FcitxHelloResponseC marshalled{
        toC(message.metadata), static_cast<std::uint32_t>(message.status),
        message.serverArchitectureBits};
    return encodeVia(fcitx5_protocol_core_encode_hello_response, marshalled);
}

std::vector<std::uint8_t> encode(const KeyRequest& message) {
    const FcitxKeyRequestC marshalled{
        toC(message.metadata), message.virtualKey, message.keyFlags, message.scanCode,
        toFlag(message.extendedKey), toFlag(message.popupAllowed), message.keyboardLayout,
        toC(message.logicalTextUtf8), toC(message.inputMethodUtf8),
        toFlag(message.surroundingTextValid), toC(message.surroundingTextUtf8),
        message.surroundingCursor, message.surroundingAnchor, toC(message.caret)};
    return encodeVia(fcitx5_protocol_core_encode_key_request, marshalled);
}

std::vector<std::uint8_t> encode(const KeyResponse& message) {
    std::vector<FcitxCandidateRecordC> candidates;
    candidates.reserve(message.candidates.size());
    for (const auto& candidate : message.candidates) {
        candidates.push_back(FcitxCandidateRecordC{candidate.id, toC(candidate.labelUtf8),
                                                   toC(candidate.textUtf8),
                                                   toC(candidate.commentUtf8)});
    }
    const FcitxKeyResponseC marshalled{
        toC(message.metadata), static_cast<std::uint32_t>(message.status),
        toFlag(message.handled), toC(message.commitUtf8), toC(message.preeditUtf8),
        message.preeditCaretUtf8, message.selectedCandidate, message.candidatePage,
        message.candidateTotal, message.candidateVisibility, message.candidatePageSize,
        toFlag(message.candidateBulk), toFlag(message.candidateEnd),
        toFlag(message.deleteSurroundingText), message.deleteSurroundingOffset,
        message.deleteSurroundingSize, toFlag(message.forwardKey), message.forwardKeySym,
        message.forwardKeyStates, message.forwardKeyCode,
        toFlag(message.forwardKeyRelease), toC(message.caret),
        toFlag(message.popupAllowed), toC(message.contentLocaleUtf8), candidates.data(),
        candidates.size()};
    return encodeVia(fcitx5_protocol_core_encode_key_response, marshalled);
}

std::vector<std::uint8_t> encode(const CandidateSelectRequest& message) {
    const FcitxCandidateSelectRequestC marshalled{toC(message.metadata),
                                                  message.targetProcessId, message.candidateId};
    return encodeVia(fcitx5_protocol_core_encode_candidate_select_request, marshalled);
}

std::vector<std::uint8_t> encode(const CandidateSelectResponse& message) {
    const FcitxCandidateSelectResponseC marshalled{toC(message.metadata),
                                                   static_cast<std::uint32_t>(message.status)};
    return encodeVia(fcitx5_protocol_core_encode_candidate_select_response, marshalled);
}

std::vector<std::uint8_t> encode(const StateRequest& message) {
    const FcitxStateRequestC marshalled{toC(message.metadata)};
    return encodeVia(fcitx5_protocol_core_encode_state_request, marshalled);
}

std::vector<std::uint8_t> encode(const EngineStatusRequest& message) {
    const FcitxEngineStatusRequestC marshalled{toC(message.metadata)};
    return encodeVia(fcitx5_protocol_core_encode_engine_status_request, marshalled);
}

std::vector<std::uint8_t> encode(const EngineStatusResponse& message) {
    const FcitxEngineStatusResponseC marshalled{
        toC(message.metadata), static_cast<std::uint32_t>(message.status),
        toC(message.currentInputMethodId), toC(message.currentInputMethodName),
        toC(message.currentInputMethodNativeName), toC(message.currentInputMethodShortLabel)};
    return encodeVia(fcitx5_protocol_core_encode_engine_status_response, marshalled);
}

std::vector<std::uint8_t> encode(const LauncherRequest& message) {
    const FcitxLauncherRequestC marshalled{toC(message.metadata),
                                           static_cast<std::uint32_t>(message.command)};
    return encodeVia(fcitx5_protocol_core_encode_launcher_request, marshalled);
}

std::vector<std::uint8_t> encode(const LauncherResponse& message) {
    const FcitxLauncherResponseC marshalled{
        toC(message.metadata), static_cast<std::uint32_t>(message.status),
        message.launcherState, message.engineState, message.startDisposition,
        toFlag(message.safeMode), message.retryAfterMilliseconds,
        toC(message.currentInputMethodId), toC(message.currentInputMethodName),
        toC(message.currentInputMethodNativeName), toC(message.currentInputMethodShortLabel)};
    return encodeVia(fcitx5_protocol_core_encode_launcher_response, marshalled);
}

bool decode(const FrameView& frame, HelloRequest& message) noexcept {
    FcitxHelloRequestC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    return decodeVia(fcitx5_protocol_core_decode_hello_request, metadata, frame.body, out,
                     [&](const FcitxHelloRequestC& decoded) {
                         message.metadata = frame.metadata;
                         message.clientArchitectureBits = decoded.clientArchitectureBits;
                         message.clientProcessId = decoded.clientProcessId;
                     });
}

bool decode(const FrameView& frame, HelloResponse& message) noexcept {
    FcitxHelloResponseC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    return decodeVia(fcitx5_protocol_core_decode_hello_response, metadata, frame.body, out,
                     [&](const FcitxHelloResponseC& decoded) {
                         message.metadata = frame.metadata;
                         message.status = static_cast<Status>(decoded.status);
                         message.serverArchitectureBits = decoded.serverArchitectureBits;
                     });
}

bool decode(const FrameView& frame, KeyRequest& message) noexcept {
    FcitxKeyRequestC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    return decodeVia(fcitx5_protocol_core_decode_key_request, metadata, frame.body, out,
                     [&](const FcitxKeyRequestC& decoded) {
                         message.metadata = frame.metadata;
                         message.virtualKey = decoded.virtualKey;
                         message.keyFlags = decoded.keyFlags;
                         message.scanCode = decoded.scanCode;
                         message.extendedKey = decoded.extendedKey != 0;
                         message.popupAllowed = decoded.popupAllowed != 0;
                         message.keyboardLayout = decoded.keyboardLayout;
                         message.logicalTextUtf8 = fromC(decoded.logicalText);
                         message.inputMethodUtf8 = fromC(decoded.inputMethod);
                         message.surroundingTextValid = decoded.surroundingTextValid != 0;
                         message.surroundingTextUtf8 = fromC(decoded.surroundingText);
                         message.surroundingCursor = decoded.surroundingCursor;
                         message.surroundingAnchor = decoded.surroundingAnchor;
                         message.caret = fromC(decoded.caret);
                     });
}

bool decode(const FrameView& frame, KeyResponse& message) noexcept {
    FcitxKeyResponseC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    std::array<std::uint8_t, 4096> stackStrings{};
    std::array<FcitxCandidateRecordC, 16> stackCandidates{};
    std::size_t stringsNeeded = 0;
    std::size_t candidatesNeeded = 0;
    const auto fill = [&](const FcitxKeyResponseC& decoded) {
        message.metadata = frame.metadata;
        message.status = static_cast<Status>(decoded.status);
        message.handled = decoded.handled != 0;
        message.commitUtf8 = fromC(decoded.commit);
        message.preeditUtf8 = fromC(decoded.preedit);
        message.preeditCaretUtf8 = decoded.preeditCaretUtf8;
        message.selectedCandidate = decoded.selectedCandidate;
        message.candidatePage = decoded.candidatePage;
        message.candidateTotal = decoded.candidateTotal;
        message.candidateVisibility = decoded.candidateVisibility;
        message.candidatePageSize = decoded.candidatePageSize;
        message.candidateBulk = decoded.candidateBulk != 0;
        message.candidateEnd = decoded.candidateEnd != 0;
        message.deleteSurroundingText = decoded.deleteSurroundingText != 0;
        message.deleteSurroundingOffset = decoded.deleteSurroundingOffset;
        message.deleteSurroundingSize = decoded.deleteSurroundingSize;
        message.forwardKey = decoded.forwardKey != 0;
        message.forwardKeySym = decoded.forwardKeySym;
        message.forwardKeyStates = decoded.forwardKeyStates;
        message.forwardKeyCode = decoded.forwardKeyCode;
        message.forwardKeyRelease = decoded.forwardKeyRelease != 0;
        message.caret = fromC(decoded.caret);
        message.popupAllowed = decoded.popupAllowed != 0;
        message.contentLocaleUtf8 = fromC(decoded.contentLocale);
        message.candidates.clear();
        message.candidates.reserve(decoded.candidateCount);
        for (std::size_t index = 0; index < decoded.candidateCount; ++index) {
            const FcitxCandidateRecordC& record = decoded.candidates[index];
            message.candidates.push_back(
                CandidateRecord{record.id, fromC(record.label), fromC(record.text),
                                fromC(record.comment)});
        }
    };
    if (fcitx5_protocol_core_decode_key_response(
            &metadata, frame.body.data(), frame.body.size(), &out, stackStrings.data(),
            stackStrings.size(), &stringsNeeded, stackCandidates.data(), stackCandidates.size(),
            &candidatesNeeded)) {
        fill(out);
        return true;
    }
    if (stringsNeeded == 0 && candidatesNeeded == 0) {
        return false; // rejected by validation
    }
    std::vector<std::uint8_t> strings(stringsNeeded);
    std::vector<FcitxCandidateRecordC> candidates(candidatesNeeded);
    if (!fcitx5_protocol_core_decode_key_response(
            &metadata, frame.body.data(), frame.body.size(), &out, strings.data(), strings.size(),
            &stringsNeeded, candidates.data(), candidates.size(), &candidatesNeeded)) {
        return false;
    }
    fill(out);
    return true;
}

bool decode(const FrameView& frame, CandidateSelectRequest& message) noexcept {
    FcitxCandidateSelectRequestC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    return decodeVia(fcitx5_protocol_core_decode_candidate_select_request, metadata, frame.body,
                     out,
                     [&](const FcitxCandidateSelectRequestC& decoded) {
                         message.metadata = frame.metadata;
                         message.targetProcessId = decoded.targetProcessId;
                         message.candidateId = decoded.candidateId;
                     });
}

bool decode(const FrameView& frame, CandidateSelectResponse& message) noexcept {
    FcitxCandidateSelectResponseC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    return decodeVia(fcitx5_protocol_core_decode_candidate_select_response, metadata, frame.body,
                     out,
                     [&](const FcitxCandidateSelectResponseC& decoded) {
                         message.metadata = frame.metadata;
                         message.status = static_cast<Status>(decoded.status);
                     });
}

bool decode(const FrameView& frame, StateRequest& message) noexcept {
    FcitxStateRequestC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    return decodeVia(fcitx5_protocol_core_decode_state_request, metadata, frame.body, out,
                     [&](const FcitxStateRequestC&) { message.metadata = frame.metadata; });
}

bool decode(const FrameView& frame, EngineStatusRequest& message) noexcept {
    FcitxEngineStatusRequestC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    return decodeVia(fcitx5_protocol_core_decode_engine_status_request, metadata, frame.body, out,
                     [&](const FcitxEngineStatusRequestC&) { message.metadata = frame.metadata; });
}

bool decode(const FrameView& frame, EngineStatusResponse& message) noexcept {
    FcitxEngineStatusResponseC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    return decodeVia(fcitx5_protocol_core_decode_engine_status_response, metadata, frame.body, out,
                     [&](const FcitxEngineStatusResponseC& decoded) {
                         message.metadata = frame.metadata;
                         message.status = static_cast<Status>(decoded.status);
                         message.currentInputMethodId = fromC(decoded.currentInputMethodId);
                         message.currentInputMethodName = fromC(decoded.currentInputMethodName);
                         message.currentInputMethodNativeName =
                             fromC(decoded.currentInputMethodNativeName);
                         message.currentInputMethodShortLabel =
                             fromC(decoded.currentInputMethodShortLabel);
                     });
}

bool decode(const FrameView& frame, LauncherRequest& message) noexcept {
    FcitxLauncherRequestC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    return decodeVia(fcitx5_protocol_core_decode_launcher_request, metadata, frame.body, out,
                     [&](const FcitxLauncherRequestC& decoded) {
                         message.metadata = frame.metadata;
                         message.command = static_cast<LauncherCommand>(decoded.command);
                     });
}

bool decode(const FrameView& frame, LauncherResponse& message) noexcept {
    FcitxLauncherResponseC out{};
    const FcitxMetadataC metadata = toC(frame.metadata);
    return decodeVia(fcitx5_protocol_core_decode_launcher_response, metadata, frame.body, out,
                     [&](const FcitxLauncherResponseC& decoded) {
                         message.metadata = frame.metadata;
                         message.status = static_cast<Status>(decoded.status);
                         message.launcherState = decoded.launcherState;
                         message.engineState = decoded.engineState;
                         message.startDisposition = decoded.startDisposition;
                         message.safeMode = decoded.safeMode != 0;
                         message.retryAfterMilliseconds = decoded.retryAfterMilliseconds;
                         message.currentInputMethodId = fromC(decoded.currentInputMethodId);
                         message.currentInputMethodName = fromC(decoded.currentInputMethodName);
                         message.currentInputMethodNativeName =
                             fromC(decoded.currentInputMethodNativeName);
                         message.currentInputMethodShortLabel =
                             fromC(decoded.currentInputMethodShortLabel);
                     });
}

} // namespace fcitx::windows::protocol
