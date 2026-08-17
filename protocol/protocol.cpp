#include "protocol.h"

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
            bytes_[8 + index] =
                static_cast<std::uint8_t>((bodySize >> (index * 8U)) & 0xffU);
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
        if (remaining() < 1) return false;
        value = bytes_[offset_++];
        return true;
    }

    bool readU16(std::uint16_t& value) noexcept {
        if (remaining() < 2) return false;
        value = static_cast<std::uint16_t>(bytes_[offset_]) |
                static_cast<std::uint16_t>(bytes_[offset_ + 1] << 8U);
        offset_ += 2;
        return true;
    }

    bool readU32(std::uint32_t& value) noexcept {
        if (remaining() < 4) return false;
        value = 0;
        for (unsigned index = 0; index < 4; ++index) {
            value |= static_cast<std::uint32_t>(bytes_[offset_ + index]) << (index * 8U);
        }
        offset_ += 4;
        return true;
    }

    bool readU64(std::uint64_t& value) noexcept {
        if (remaining() < 8) return false;
        value = 0;
        for (unsigned index = 0; index < 8; ++index) {
            value |= static_cast<std::uint64_t>(bytes_[offset_ + index]) << (index * 8U);
        }
        offset_ += 8;
        return true;
    }

    bool readString(std::string& value) {
        std::uint32_t size = 0;
        if (!readU32(size) || size > remaining()) return false;
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
    return metadata.engineEpoch == 0 && metadata.contextId == 0 &&
           metadata.compositionId == 0 && metadata.revision == 0;
}

bool validHelloResponseMetadata(const Metadata& metadata) noexcept {
    return metadata.engineEpoch != 0 && metadata.contextId == 0 &&
           metadata.compositionId == 0 && metadata.revision == 0;
}

bool validKeyMetadata(const Metadata& metadata) noexcept {
    return metadata.engineEpoch != 0 && metadata.contextId != 0;
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
           type == MessageType::launcherRequest;
}

bool isResponse(MessageType type) noexcept {
    return type == MessageType::helloResponse || type == MessageType::keyResponse ||
           type == MessageType::launcherResponse;
}

bool decodeHeader(std::span<const std::uint8_t> bytes, MessageType& type,
                  std::uint32_t& bodySize, Metadata& metadata) noexcept {
    if (bytes.size() != kHeaderSize) return false;
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
        rawType > static_cast<std::uint16_t>(MessageType::launcherResponse)) {
        return false;
    }
    type = static_cast<MessageType>(rawType);
    return bodySize <= maximumFrameSize(type) - kHeaderSize &&
           validMetadata(type, metadata);
}

bool decodeFrame(std::span<const std::uint8_t> bytes, FrameView& output) noexcept {
    if (bytes.size() < kHeaderSize || bytes.size() > kMaxControlFrameSize) return false;
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
        !validKeyMetadata(message.metadata)) {
        return {};
    }
    Writer writer(MessageType::keyRequest, message.metadata);
    writer.appendU32(message.virtualKey);
    writer.appendU32(message.keyFlags);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const KeyResponse& message) {
    if (!validMetadata(MessageType::keyResponse, message.metadata) ||
        !validKeyMetadata(message.metadata) || message.commitUtf8.size() > kMaxCommitUtf8 ||
        static_cast<std::uint32_t>(message.status) >
            static_cast<std::uint32_t>(Status::accessDenied)) {
        return {};
    }
    Writer writer(MessageType::keyResponse, message.metadata);
    writer.appendU32(static_cast<std::uint32_t>(message.status));
    writer.appendU8(message.handled ? 1U : 0U);
    writer.appendString(message.commitUtf8);
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
    return writer.finish();
}

bool decode(const FrameView& frame, HelloRequest& message) noexcept {
    if (frame.type != MessageType::helloRequest ||
        !validHelloRequestMetadata(frame.metadata)) return false;
    Reader reader(frame.body);
    message.metadata = frame.metadata;
    return reader.readU32(message.clientArchitectureBits) &&
           reader.readU32(message.clientProcessId) &&
           (message.clientArchitectureBits == 32 || message.clientArchitectureBits == 64) &&
           message.clientProcessId != 0 && reader.done();
}

bool decode(const FrameView& frame, HelloResponse& message) noexcept {
    if (frame.type != MessageType::helloResponse ||
        !validHelloResponseMetadata(frame.metadata)) return false;
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
    if (frame.type != MessageType::keyRequest || !validKeyMetadata(frame.metadata)) return false;
    Reader reader(frame.body);
    message.metadata = frame.metadata;
    return reader.readU32(message.virtualKey) && reader.readU32(message.keyFlags) && reader.done();
}

bool decode(const FrameView& frame, KeyResponse& message) noexcept {
    if (frame.type != MessageType::keyResponse || !validKeyMetadata(frame.metadata)) return false;
    Reader reader(frame.body);
    std::uint32_t status = 0;
    std::uint8_t handled = 0;
    message.metadata = frame.metadata;
    try {
        if (!reader.readU32(status) || !reader.readU8(handled) || handled > 1 ||
            !reader.readString(message.commitUtf8) || !reader.done() ||
            message.commitUtf8.size() > kMaxCommitUtf8 ||
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

bool decode(const FrameView& frame, LauncherRequest& message) noexcept {
    if (frame.type != MessageType::launcherRequest ||
        !validLauncherMetadata(frame.metadata)) return false;
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
    if (frame.type != MessageType::launcherResponse ||
        !validLauncherMetadata(frame.metadata)) return false;
    Reader reader(frame.body);
    std::uint32_t status = 0;
    std::uint8_t safeMode = 0;
    if (!reader.readU32(status) || !reader.readU32(message.launcherState) ||
        !reader.readU32(message.engineState) || !reader.readU32(message.startDisposition) ||
        !reader.readU8(safeMode) || safeMode > 1 ||
        !reader.readU64(message.retryAfterMilliseconds) || !reader.done() ||
        status > static_cast<std::uint32_t>(Status::accessDenied)) {
        return false;
    }
    message.metadata = frame.metadata;
    message.status = static_cast<Status>(status);
    message.safeMode = safeMode != 0;
    return true;
}

} // namespace fcitx::windows::protocol
