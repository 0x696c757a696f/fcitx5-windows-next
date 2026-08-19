#include "protocol.h"

#include <algorithm>
#include <utility>

namespace fcitx::windows::protocol {
namespace {

bool validMetadata(MessageType type, const Metadata& metadata) noexcept {
    if (metadata.requestId == 0) {
        return false;
    }
    return isRequest(type) ? metadata.responseTo == 0 : metadata.responseTo != 0;
}

class Writer {
  public:
    Writer(MessageType type, const Metadata& metadata) {
        bytes_.reserve(kHeaderSize);
        appendU32(kMagic);
        appendU16(kVersion);
        appendU16(static_cast<std::uint16_t>(type));
        appendU32(0);
        appendU64(metadata.requestId);
        appendU64(metadata.responseTo);
        appendU64(metadata.engineEpoch);
        appendU32(metadata.sessionId);
        appendU64(metadata.contextId);
        appendU64(metadata.compositionId);
        appendU64(metadata.revision);
    }

    void appendU8(std::uint8_t value) { bytes_.push_back(value); }

    void appendU32(std::uint32_t value) {
        for (unsigned shift = 0; shift < 32; shift += 8) {
            bytes_.push_back(static_cast<std::uint8_t>((value >> shift) & 0xffU));
        }
    }

    void appendI32(std::int32_t value) { appendU32(static_cast<std::uint32_t>(value)); }

    void appendU16(std::uint16_t value) {
        bytes_.push_back(static_cast<std::uint8_t>(value & 0xffU));
        bytes_.push_back(static_cast<std::uint8_t>((value >> 8U) & 0xffU));
    }

    void appendU64(std::uint64_t value) {
        for (unsigned shift = 0; shift < 64; shift += 8) {
            bytes_.push_back(static_cast<std::uint8_t>((value >> shift) & 0xffU));
        }
    }

    void appendString(std::string_view value) {
        appendU32(static_cast<std::uint32_t>(value.size()));
        bytes_.insert(bytes_.end(), value.begin(), value.end());
    }

    std::vector<std::uint8_t> finish() {
        const auto bodySize = static_cast<std::uint32_t>(bytes_.size() - kHeaderSize);
        for (unsigned index = 0; index < 4; ++index) {
            bytes_[8 + index] = static_cast<std::uint8_t>((bodySize >> (index * 8U)) & 0xffU);
        }
        return std::move(bytes_);
    }

  private:
    std::vector<std::uint8_t> bytes_;
};

class Reader {
  public:
    explicit Reader(std::span<const std::uint8_t> bytes) : bytes_(bytes) {}

    bool readU8(std::uint8_t& value) noexcept {
        if (remaining() < 1)
            return false;
        value = bytes_[offset_++];
        return true;
    }

    bool readU16(std::uint16_t& value) noexcept {
        if (remaining() < 2)
            return false;
        value = static_cast<std::uint16_t>(bytes_[offset_]) |
                static_cast<std::uint16_t>(bytes_[offset_ + 1] << 8U);
        offset_ += 2;
        return true;
    }

    bool readU32(std::uint32_t& value) noexcept {
        if (remaining() < 4)
            return false;
        value = 0;
        for (unsigned index = 0; index < 4; ++index) {
            value |= static_cast<std::uint32_t>(bytes_[offset_ + index]) << (index * 8U);
        }
        offset_ += 4;
        return true;
    }

    bool readI32(std::int32_t& value) noexcept {
        std::uint32_t raw = 0;
        if (!readU32(raw))
            return false;
        value = static_cast<std::int32_t>(raw);
        return true;
    }

    bool readU64(std::uint64_t& value) noexcept {
        if (remaining() < 8)
            return false;
        value = 0;
        for (unsigned index = 0; index < 8; ++index) {
            value |= static_cast<std::uint64_t>(bytes_[offset_ + index]) << (index * 8U);
        }
        offset_ += 8;
        return true;
    }

    bool readString(std::string& value) {
        std::uint32_t size = 0;
        if (!readU32(size) || size > remaining())
            return false;
        value.assign(reinterpret_cast<const char*>(bytes_.data() + offset_), size);
        offset_ += size;
        return true;
    }

    [[nodiscard]] bool done() const noexcept { return offset_ == bytes_.size(); }

  private:
    [[nodiscard]] std::size_t remaining() const noexcept { return bytes_.size() - offset_; }

    std::span<const std::uint8_t> bytes_;
    std::size_t offset_{};
};

bool validHelloRequestMetadata(const Metadata& metadata) noexcept {
    return metadata.engineEpoch == 0 && metadata.contextId == 0 && metadata.compositionId == 0 &&
           metadata.revision == 0;
}

bool validHelloResponseMetadata(const Metadata& metadata) noexcept {
    return metadata.engineEpoch != 0 && metadata.contextId == 0 && metadata.compositionId == 0 &&
           metadata.revision == 0;
}

bool validKeyMetadata(const Metadata& metadata) noexcept {
    return metadata.engineEpoch != 0 && metadata.contextId != 0;
}

bool validEngineStatusMetadata(const Metadata& metadata) noexcept {
    return metadata.engineEpoch != 0 && metadata.contextId == 0 &&
           metadata.compositionId == 0 && metadata.revision == 0;
}

bool validCaret(const CaretRect& caret) noexcept {
    if (!caret.valid) {
        return caret.left == 0 && caret.top == 0 && caret.right == 0 && caret.bottom == 0 &&
               caret.dpi == 96;
    }
    constexpr std::int32_t kCoordinateLimit = 1'000'000;
    return caret.left >= -kCoordinateLimit && caret.top >= -kCoordinateLimit &&
           caret.right <= kCoordinateLimit && caret.bottom <= kCoordinateLimit &&
           caret.right >= caret.left && caret.bottom >= caret.top && caret.dpi >= 48 &&
           caret.dpi <= 960;
}

bool utf8CodePointCount(std::string_view text, std::size_t& count) noexcept {
    count = 0;
    for (std::size_t index = 0; index < text.size();) {
        const auto byte = static_cast<unsigned char>(text[index]);
        std::size_t length = 0;
        if ((byte & 0x80U) == 0) {
            length = 1;
        } else if ((byte & 0xe0U) == 0xc0U) {
            length = 2;
            if (byte < 0xc2U)
                return false;
        } else if ((byte & 0xf0U) == 0xe0U) {
            length = 3;
        } else if ((byte & 0xf8U) == 0xf0U) {
            length = 4;
            if (byte > 0xf4U)
                return false;
        } else {
            return false;
        }
        if (index + length > text.size())
            return false;
        for (std::size_t offset = 1; offset < length; ++offset) {
            if ((static_cast<unsigned char>(text[index + offset]) & 0xc0U) != 0x80U)
                return false;
        }
        index += length;
        ++count;
    }
    return true;
}

bool validSurroundingText(const KeyRequest& message) noexcept {
    if (!message.surroundingTextValid) {
        return message.surroundingTextUtf8.empty() &&
               message.surroundingCursor == 0 &&
               message.surroundingAnchor == 0;
    }
    if (message.surroundingTextUtf8.size() > kMaxSurroundingTextUtf8)
        return false;
    std::size_t length = 0;
    return utf8CodePointCount(message.surroundingTextUtf8, length) &&
           message.surroundingCursor <= length &&
           message.surroundingAnchor <= length;
}

bool validInputMethodText(std::string_view text, std::size_t maximumBytes) noexcept {
    std::size_t codePoints = 0;
    return text.size() <= maximumBytes && utf8CodePointCount(text, codePoints);
}

bool validInputMethodStatus(std::string_view id, std::string_view name,
                            std::string_view nativeName,
                            std::string_view shortLabel) noexcept {
    return validInputMethodText(id, kMaxInputMethodIdUtf8) &&
           validInputMethodText(name, kMaxInputMethodNameUtf8) &&
           validInputMethodText(nativeName, kMaxInputMethodNameUtf8) &&
           validInputMethodText(shortLabel, kMaxInputMethodNameUtf8);
}

bool validLauncherResponsePayload(const LauncherResponse& message) noexcept {
    return validInputMethodStatus(message.currentInputMethodId,
                                  message.currentInputMethodName,
                                  message.currentInputMethodNativeName,
                                  message.currentInputMethodShortLabel);
}

bool validEngineStatusResponsePayload(const EngineStatusResponse& message) noexcept {
    return validInputMethodStatus(message.currentInputMethodId,
                                  message.currentInputMethodName,
                                  message.currentInputMethodNativeName,
                                  message.currentInputMethodShortLabel);
}

bool validKeyResponsePayload(const KeyResponse& message) noexcept {
    if (message.commitUtf8.size() > kMaxCommitUtf8 ||
        message.preeditUtf8.size() > kMaxPreeditUtf8 ||
        message.preeditCaretUtf8 > message.preeditUtf8.size() ||
        message.candidates.size() > kMaxCandidates || message.candidateVisibility > 2 ||
        message.candidatePageSize > kMaxCandidates ||
        (message.selectedCandidate != UINT32_MAX &&
         message.selectedCandidate >= message.candidates.size()) ||
        message.candidateTotal < message.candidates.size() || !validCaret(message.caret))
        return false;
    if (message.candidateVisibility == 0 && !message.candidates.empty())
        return false;
    if (!message.deleteSurroundingText &&
        (message.deleteSurroundingOffset != 0 || message.deleteSurroundingSize != 0))
        return false;
    if (!message.forwardKey &&
        (message.forwardKeySym != 0 || message.forwardKeyStates != 0 ||
         message.forwardKeyCode != 0 || message.forwardKeyRelease))
        return false;
    return std::all_of(message.candidates.begin(), message.candidates.end(),
                       [](const CandidateRecord& candidate) {
                           return candidate.id != 0 &&
                                  candidate.labelUtf8.size() <= kMaxCandidateFieldUtf8 &&
                                  candidate.textUtf8.size() <= kMaxCandidateFieldUtf8 &&
                                  candidate.commentUtf8.size() <= kMaxCandidateFieldUtf8;
                       }) &&
           message.commitUtf8.size() <= kMaxCommitUtf8 &&
           message.preeditUtf8.size() <= kMaxPreeditUtf8 &&
           message.preeditCaretUtf8 <= message.preeditUtf8.size();
}

bool validKeyRequestPayload(const KeyRequest& message) noexcept {
    return message.scanCode <= 0xffU &&
           message.logicalTextUtf8.size() <= kMaxLogicalKeyUtf8 &&
           message.inputMethodUtf8.size() <= kMaxInputMethodIdUtf8 &&
           validSurroundingText(message) &&
           validCaret(message.caret);
}

bool validLauncherMetadata(const Metadata& metadata) noexcept {
    return metadata.contextId == 0 && metadata.compositionId == 0 && metadata.revision == 0;
}

std::size_t maximumFrameSize(MessageType type) noexcept {
    return type == MessageType::launcherRequest || type == MessageType::launcherResponse
               ? kMaxControlFrameSize
               : kMaxHotFrameSize;
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
    if (bytes.size() != kHeaderSize)
        return false;
    Reader reader(bytes);
    std::uint32_t magic = 0;
    std::uint16_t version = 0;
    std::uint16_t rawType = 0;
    if (!reader.readU32(magic) || !reader.readU16(version) || !reader.readU16(rawType) ||
        !reader.readU32(bodySize) || !reader.readU64(metadata.requestId) ||
        !reader.readU64(metadata.responseTo) || !reader.readU64(metadata.engineEpoch) ||
        !reader.readU32(metadata.sessionId) || !reader.readU64(metadata.contextId) ||
        !reader.readU64(metadata.compositionId) || !reader.readU64(metadata.revision) ||
        !reader.done()) {
        return false;
    }
    if (magic != kMagic || version != kVersion ||
        rawType < static_cast<std::uint16_t>(MessageType::helloRequest) ||
        rawType > static_cast<std::uint16_t>(MessageType::engineStatusResponse)) {
        return false;
    }
    type = static_cast<MessageType>(rawType);
    return bodySize <= maximumFrameSize(type) - kHeaderSize && validMetadata(type, metadata);
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
    if (!validMetadata(MessageType::helloRequest, message.metadata) ||
        !validHelloRequestMetadata(message.metadata) ||
        (message.clientArchitectureBits != 32 && message.clientArchitectureBits != 64) ||
        message.clientProcessId == 0) {
        return {};
    }
    Writer writer(MessageType::helloRequest, message.metadata);
    writer.appendU32(message.clientArchitectureBits);
    writer.appendU32(message.clientProcessId);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const HelloResponse& message) {
    if (!validMetadata(MessageType::helloResponse, message.metadata) ||
        !validHelloResponseMetadata(message.metadata) ||
        (message.serverArchitectureBits != 32 && message.serverArchitectureBits != 64) ||
        static_cast<std::uint32_t>(message.status) >
            static_cast<std::uint32_t>(Status::accessDenied)) {
        return {};
    }
    Writer writer(MessageType::helloResponse, message.metadata);
    writer.appendU32(static_cast<std::uint32_t>(message.status));
    writer.appendU32(message.serverArchitectureBits);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const KeyRequest& message) {
    if (!validMetadata(MessageType::keyRequest, message.metadata) ||
        !validKeyMetadata(message.metadata) || !validKeyRequestPayload(message)) {
        return {};
    }
    Writer writer(MessageType::keyRequest, message.metadata);
    writer.appendU32(message.virtualKey);
    writer.appendU32(message.keyFlags);
    writer.appendU32(message.scanCode);
    writer.appendU8(message.extendedKey ? 1U : 0U);
    writer.appendU8(message.popupAllowed ? 1U : 0U);
    writer.appendU64(message.keyboardLayout);
    writer.appendString(message.logicalTextUtf8);
    writer.appendString(message.inputMethodUtf8);
    writer.appendU8(message.surroundingTextValid ? 1U : 0U);
    writer.appendString(message.surroundingTextUtf8);
    writer.appendU32(message.surroundingCursor);
    writer.appendU32(message.surroundingAnchor);
    writer.appendU8(message.caret.valid ? 1U : 0U);
    writer.appendI32(message.caret.left);
    writer.appendI32(message.caret.top);
    writer.appendI32(message.caret.right);
    writer.appendI32(message.caret.bottom);
    writer.appendU32(message.caret.dpi);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const KeyResponse& message) {
    if (!validMetadata(MessageType::keyResponse, message.metadata) ||
        !validKeyMetadata(message.metadata) || !validKeyResponsePayload(message) ||
        static_cast<std::uint32_t>(message.status) >
            static_cast<std::uint32_t>(Status::accessDenied)) {
        return {};
    }
    Writer writer(MessageType::keyResponse, message.metadata);
    writer.appendU32(static_cast<std::uint32_t>(message.status));
    writer.appendU8(message.handled ? 1U : 0U);
    writer.appendString(message.commitUtf8);
    writer.appendString(message.preeditUtf8);
    writer.appendU32(message.preeditCaretUtf8);
    writer.appendU32(static_cast<std::uint32_t>(message.candidates.size()));
    writer.appendU32(message.selectedCandidate);
    writer.appendU32(message.candidatePage);
    writer.appendU32(message.candidateTotal);
    writer.appendU8(message.candidateVisibility);
    writer.appendU32(message.candidatePageSize);
    writer.appendU8(message.candidateBulk ? 1U : 0U);
    writer.appendU8(message.candidateEnd ? 1U : 0U);
    writer.appendU8(message.deleteSurroundingText ? 1U : 0U);
    writer.appendI32(message.deleteSurroundingOffset);
    writer.appendU32(message.deleteSurroundingSize);
    writer.appendU8(message.forwardKey ? 1U : 0U);
    writer.appendU32(message.forwardKeySym);
    writer.appendU32(message.forwardKeyStates);
    writer.appendI32(message.forwardKeyCode);
    writer.appendU8(message.forwardKeyRelease ? 1U : 0U);
    writer.appendU8(message.caret.valid ? 1U : 0U);
    writer.appendI32(message.caret.left);
    writer.appendI32(message.caret.top);
    writer.appendI32(message.caret.right);
    writer.appendI32(message.caret.bottom);
    writer.appendU32(message.caret.dpi);
    for (const auto& candidate : message.candidates) {
        writer.appendU64(candidate.id);
        writer.appendString(candidate.labelUtf8);
        writer.appendString(candidate.textUtf8);
        writer.appendString(candidate.commentUtf8);
    }
    return writer.finish();
}

std::vector<std::uint8_t> encode(const CandidateSelectRequest& message) {
    if (!validMetadata(MessageType::candidateSelectRequest, message.metadata) ||
        !validKeyMetadata(message.metadata) || message.targetProcessId == 0 ||
        message.candidateId == 0) {
        return {};
    }
    Writer writer(MessageType::candidateSelectRequest, message.metadata);
    writer.appendU32(message.targetProcessId);
    writer.appendU64(message.candidateId);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const CandidateSelectResponse& message) {
    if (!validMetadata(MessageType::candidateSelectResponse, message.metadata) ||
        !validKeyMetadata(message.metadata) ||
        static_cast<std::uint32_t>(message.status) >
            static_cast<std::uint32_t>(Status::accessDenied)) {
        return {};
    }
    Writer writer(MessageType::candidateSelectResponse, message.metadata);
    writer.appendU32(static_cast<std::uint32_t>(message.status));
    return writer.finish();
}

std::vector<std::uint8_t> encode(const StateRequest& message) {
    if (!validMetadata(MessageType::stateRequest, message.metadata) ||
        !validKeyMetadata(message.metadata)) {
        return {};
    }
    Writer writer(MessageType::stateRequest, message.metadata);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const EngineStatusRequest& message) {
    if (!validMetadata(MessageType::engineStatusRequest, message.metadata) ||
        !validEngineStatusMetadata(message.metadata)) {
        return {};
    }
    Writer writer(MessageType::engineStatusRequest, message.metadata);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const EngineStatusResponse& message) {
    if (!validMetadata(MessageType::engineStatusResponse, message.metadata) ||
        !validEngineStatusMetadata(message.metadata) ||
        !validEngineStatusResponsePayload(message) ||
        static_cast<std::uint32_t>(message.status) >
            static_cast<std::uint32_t>(Status::accessDenied)) {
        return {};
    }
    Writer writer(MessageType::engineStatusResponse, message.metadata);
    writer.appendU32(static_cast<std::uint32_t>(message.status));
    writer.appendString(message.currentInputMethodId);
    writer.appendString(message.currentInputMethodName);
    writer.appendString(message.currentInputMethodNativeName);
    writer.appendString(message.currentInputMethodShortLabel);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const LauncherRequest& message) {
    if (!validMetadata(MessageType::launcherRequest, message.metadata) ||
        !validLauncherMetadata(message.metadata) ||
        message.command < LauncherCommand::startDemand ||
        message.command > LauncherCommand::shutdown) {
        return {};
    }
    Writer writer(MessageType::launcherRequest, message.metadata);
    writer.appendU32(static_cast<std::uint32_t>(message.command));
    return writer.finish();
}

std::vector<std::uint8_t> encode(const LauncherResponse& message) {
    if (!validMetadata(MessageType::launcherResponse, message.metadata) ||
        !validLauncherMetadata(message.metadata) ||
        !validLauncherResponsePayload(message) ||
        static_cast<std::uint32_t>(message.status) >
            static_cast<std::uint32_t>(Status::accessDenied)) {
        return {};
    }
    Writer writer(MessageType::launcherResponse, message.metadata);
    writer.appendU32(static_cast<std::uint32_t>(message.status));
    writer.appendU32(message.launcherState);
    writer.appendU32(message.engineState);
    writer.appendU32(message.startDisposition);
    writer.appendU8(message.safeMode ? 1U : 0U);
    writer.appendU64(message.retryAfterMilliseconds);
    writer.appendString(message.currentInputMethodId);
    writer.appendString(message.currentInputMethodName);
    writer.appendString(message.currentInputMethodNativeName);
    writer.appendString(message.currentInputMethodShortLabel);
    return writer.finish();
}

bool decode(const FrameView& frame, HelloRequest& message) noexcept {
    if (frame.type != MessageType::helloRequest || !validHelloRequestMetadata(frame.metadata))
        return false;
    Reader reader(frame.body);
    message.metadata = frame.metadata;
    return reader.readU32(message.clientArchitectureBits) &&
           reader.readU32(message.clientProcessId) &&
           (message.clientArchitectureBits == 32 || message.clientArchitectureBits == 64) &&
           message.clientProcessId != 0 && reader.done();
}

bool decode(const FrameView& frame, HelloResponse& message) noexcept {
    if (frame.type != MessageType::helloResponse || !validHelloResponseMetadata(frame.metadata))
        return false;
    Reader reader(frame.body);
    std::uint32_t status = 0;
    message.metadata = frame.metadata;
    if (!reader.readU32(status) || !reader.readU32(message.serverArchitectureBits) ||
        !reader.done() || status > static_cast<std::uint32_t>(Status::accessDenied) ||
        (message.serverArchitectureBits != 32 && message.serverArchitectureBits != 64)) {
        return false;
    }
    message.status = static_cast<Status>(status);
    return true;
}

bool decode(const FrameView& frame, KeyRequest& message) noexcept {
    if (frame.type != MessageType::keyRequest || !validKeyMetadata(frame.metadata))
        return false;
    Reader reader(frame.body);
    message.metadata = frame.metadata;
    std::uint8_t valid = 0;
    std::uint8_t extended = 0;
    std::uint8_t popupAllowed = 0;
    std::uint8_t surroundingTextValid = 0;
    try {
        return reader.readU32(message.virtualKey) && reader.readU32(message.keyFlags) &&
               reader.readU32(message.scanCode) && reader.readU8(extended) && extended <= 1 &&
               reader.readU8(popupAllowed) && popupAllowed <= 1 &&
               reader.readU64(message.keyboardLayout) &&
               reader.readString(message.logicalTextUtf8) &&
               reader.readString(message.inputMethodUtf8) &&
               reader.readU8(surroundingTextValid) && surroundingTextValid <= 1 &&
               reader.readString(message.surroundingTextUtf8) &&
               reader.readU32(message.surroundingCursor) &&
               reader.readU32(message.surroundingAnchor) &&
               reader.readU8(valid) && valid <= 1 && reader.readI32(message.caret.left) &&
               reader.readI32(message.caret.top) && reader.readI32(message.caret.right) &&
               reader.readI32(message.caret.bottom) && reader.readU32(message.caret.dpi) &&
               reader.done() &&
               ((message.extendedKey = extended != 0),
                (message.popupAllowed = popupAllowed != 0),
                (message.surroundingTextValid = surroundingTextValid != 0),
                (message.caret.valid = valid != 0),
                validKeyRequestPayload(message));
    } catch (...) {
        return false;
    }
}

bool decode(const FrameView& frame, KeyResponse& message) noexcept {
    if (frame.type != MessageType::keyResponse || !validKeyMetadata(frame.metadata))
        return false;
    Reader reader(frame.body);
    std::uint32_t status = 0;
    std::uint8_t handled = 0;
    std::uint8_t caretValid = 0;
    std::uint8_t candidateBulk = 0;
    std::uint8_t candidateEnd = 0;
    std::uint8_t deleteSurroundingText = 0;
    std::uint8_t forwardKey = 0;
    std::uint8_t forwardKeyRelease = 0;
    std::uint32_t candidateCount = 0;
    message.metadata = frame.metadata;
    try {
        if (!reader.readU32(status) || !reader.readU8(handled) || handled > 1 ||
            !reader.readString(message.commitUtf8) || !reader.readString(message.preeditUtf8) ||
            !reader.readU32(message.preeditCaretUtf8) || !reader.readU32(candidateCount) ||
            candidateCount > kMaxCandidates || !reader.readU32(message.selectedCandidate) ||
            !reader.readU32(message.candidatePage) || !reader.readU32(message.candidateTotal) ||
            !reader.readU8(message.candidateVisibility) ||
            !reader.readU32(message.candidatePageSize) || !reader.readU8(candidateBulk) ||
            candidateBulk > 1 || !reader.readU8(candidateEnd) || candidateEnd > 1 ||
            !reader.readU8(deleteSurroundingText) || deleteSurroundingText > 1 ||
            !reader.readI32(message.deleteSurroundingOffset) ||
            !reader.readU32(message.deleteSurroundingSize) ||
            !reader.readU8(forwardKey) || forwardKey > 1 ||
            !reader.readU32(message.forwardKeySym) ||
            !reader.readU32(message.forwardKeyStates) ||
            !reader.readI32(message.forwardKeyCode) ||
            !reader.readU8(forwardKeyRelease) || forwardKeyRelease > 1 ||
            !reader.readU8(caretValid) || caretValid > 1 || !reader.readI32(message.caret.left) ||
            !reader.readI32(message.caret.top) || !reader.readI32(message.caret.right) ||
            !reader.readI32(message.caret.bottom) || !reader.readU32(message.caret.dpi)) {
            return false;
        }
        message.caret.valid = caretValid != 0;
        message.candidateBulk = candidateBulk != 0;
        message.candidateEnd = candidateEnd != 0;
        message.deleteSurroundingText = deleteSurroundingText != 0;
        message.forwardKey = forwardKey != 0;
        message.forwardKeyRelease = forwardKeyRelease != 0;
        message.candidates.clear();
        message.candidates.reserve(candidateCount);
        for (std::uint32_t index = 0; index < candidateCount; ++index) {
            CandidateRecord candidate;
            if (!reader.readU64(candidate.id) || !reader.readString(candidate.labelUtf8) ||
                !reader.readString(candidate.textUtf8) || !reader.readString(candidate.commentUtf8))
                return false;
            message.candidates.emplace_back(std::move(candidate));
        }
        if (!reader.done() || !validKeyResponsePayload(message) ||
            status > static_cast<std::uint32_t>(Status::accessDenied)) {
            return false;
        }
    } catch (...) {
        return false;
    }
    message.status = static_cast<Status>(status);
    message.handled = handled != 0;
    return true;
}

bool decode(const FrameView& frame, CandidateSelectRequest& message) noexcept {
    if (frame.type != MessageType::candidateSelectRequest ||
        !validKeyMetadata(frame.metadata)) {
        return false;
    }
    Reader reader(frame.body);
    message.metadata = frame.metadata;
    return reader.readU32(message.targetProcessId) &&
           reader.readU64(message.candidateId) && message.targetProcessId != 0 &&
           message.candidateId != 0 && reader.done();
}

bool decode(const FrameView& frame, CandidateSelectResponse& message) noexcept {
    if (frame.type != MessageType::candidateSelectResponse ||
        !validKeyMetadata(frame.metadata)) {
        return false;
    }
    Reader reader(frame.body);
    std::uint32_t status = 0;
    if (!reader.readU32(status) || !reader.done() ||
        status > static_cast<std::uint32_t>(Status::accessDenied)) {
        return false;
    }
    message.metadata = frame.metadata;
    message.status = static_cast<Status>(status);
    return true;
}

bool decode(const FrameView& frame, StateRequest& message) noexcept {
    if (frame.type != MessageType::stateRequest || !validKeyMetadata(frame.metadata) ||
        !frame.body.empty()) {
        return false;
    }
    message.metadata = frame.metadata;
    return true;
}

bool decode(const FrameView& frame, EngineStatusRequest& message) noexcept {
    if (frame.type != MessageType::engineStatusRequest ||
        !validEngineStatusMetadata(frame.metadata) || !frame.body.empty()) {
        return false;
    }
    message.metadata = frame.metadata;
    return true;
}

bool decode(const FrameView& frame, EngineStatusResponse& message) noexcept {
    if (frame.type != MessageType::engineStatusResponse ||
        !validEngineStatusMetadata(frame.metadata)) {
        return false;
    }
    Reader reader(frame.body);
    std::uint32_t status = 0;
    if (!reader.readU32(status) ||
        !reader.readString(message.currentInputMethodId) ||
        !reader.readString(message.currentInputMethodName) ||
        !reader.readString(message.currentInputMethodNativeName) ||
        !reader.readString(message.currentInputMethodShortLabel) || !reader.done() ||
        !validEngineStatusResponsePayload(message) ||
        status > static_cast<std::uint32_t>(Status::accessDenied)) {
        return false;
    }
    message.metadata = frame.metadata;
    message.status = static_cast<Status>(status);
    return true;
}

bool decode(const FrameView& frame, LauncherRequest& message) noexcept {
    if (frame.type != MessageType::launcherRequest || !validLauncherMetadata(frame.metadata))
        return false;
    Reader reader(frame.body);
    std::uint32_t command = 0;
    if (!reader.readU32(command) || !reader.done() ||
        command < static_cast<std::uint32_t>(LauncherCommand::startDemand) ||
        command > static_cast<std::uint32_t>(LauncherCommand::shutdown)) {
        return false;
    }
    message.metadata = frame.metadata;
    message.command = static_cast<LauncherCommand>(command);
    return true;
}

bool decode(const FrameView& frame, LauncherResponse& message) noexcept {
    if (frame.type != MessageType::launcherResponse || !validLauncherMetadata(frame.metadata))
        return false;
    Reader reader(frame.body);
    std::uint32_t status = 0;
    std::uint8_t safeMode = 0;
    if (!reader.readU32(status) || !reader.readU32(message.launcherState) ||
        !reader.readU32(message.engineState) || !reader.readU32(message.startDisposition) ||
        !reader.readU8(safeMode) || safeMode > 1 ||
        !reader.readU64(message.retryAfterMilliseconds) ||
        !reader.readString(message.currentInputMethodId) ||
        !reader.readString(message.currentInputMethodName) ||
        !reader.readString(message.currentInputMethodNativeName) ||
        !reader.readString(message.currentInputMethodShortLabel) || !reader.done() ||
        !validLauncherResponsePayload(message) ||
        status > static_cast<std::uint32_t>(Status::accessDenied)) {
        return false;
    }
    message.metadata = frame.metadata;
    message.status = static_cast<Status>(status);
    message.safeMode = safeMode != 0;
    return true;
}

} // namespace fcitx::windows::protocol
