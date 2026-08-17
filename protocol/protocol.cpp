#include "protocol.h"

#include <limits>

namespace fcitx::windows::protocol {
namespace {

class Writer {
public:
    explicit Writer(MessageType type) {
        appendU32(kMagic);
        appendU16(kVersion);
        appendU16(static_cast<std::uint16_t>(type));
        appendU32(0);
    }

    void appendU8(std::uint8_t value) { bytes_.push_back(value); }

    void appendU16(std::uint16_t value) {
        bytes_.push_back(static_cast<std::uint8_t>(value & 0xffU));
        bytes_.push_back(static_cast<std::uint8_t>((value >> 8U) & 0xffU));
    }

    void appendU32(std::uint32_t value) {
        for (unsigned shift = 0; shift < 32; shift += 8) {
            bytes_.push_back(static_cast<std::uint8_t>((value >> shift) & 0xffU));
        }
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
        if (offset_ >= bytes_.size()) {
            return false;
        }
        value = bytes_[offset_++];
        return true;
    }

    bool readU16(std::uint16_t& value) noexcept {
        if (remaining() < 2) {
            return false;
        }
        value = static_cast<std::uint16_t>(bytes_[offset_]) |
                static_cast<std::uint16_t>(bytes_[offset_ + 1] << 8U);
        offset_ += 2;
        return true;
    }

    bool readU32(std::uint32_t& value) noexcept {
        if (remaining() < 4) {
            return false;
        }
        value = 0;
        for (unsigned index = 0; index < 4; ++index) {
            value |= static_cast<std::uint32_t>(bytes_[offset_ + index]) << (index * 8U);
        }
        offset_ += 4;
        return true;
    }

    bool readU64(std::uint64_t& value) noexcept {
        if (remaining() < 8) {
            return false;
        }
        value = 0;
        for (unsigned index = 0; index < 8; ++index) {
            value |= static_cast<std::uint64_t>(bytes_[offset_ + index]) << (index * 8U);
        }
        offset_ += 8;
        return true;
    }

    bool readString(std::string& value) {
        std::uint32_t size = 0;
        if (!readU32(size) || size > remaining()) {
            return false;
        }
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

Writer begin(MessageType type) { return Writer(type); }

} // namespace

bool decodeHeader(std::span<const std::uint8_t> bytes, MessageType& type,
                  std::uint32_t& bodySize) noexcept {
    if (bytes.size() != kHeaderSize) {
        return false;
    }
    Reader reader(bytes);
    std::uint32_t magic = 0;
    std::uint16_t version = 0;
    std::uint16_t rawType = 0;
    if (!reader.readU32(magic) || !reader.readU16(version) || !reader.readU16(rawType) ||
        !reader.readU32(bodySize) || !reader.done()) {
        return false;
    }
    if (magic != kMagic || version != kVersion || bodySize > kMaxFrameSize - kHeaderSize) {
        return false;
    }
    if (rawType < static_cast<std::uint16_t>(MessageType::helloRequest) ||
        rawType > static_cast<std::uint16_t>(MessageType::keyResponse)) {
        return false;
    }
    type = static_cast<MessageType>(rawType);
    return true;
}

bool decodeFrame(std::span<const std::uint8_t> bytes, FrameView& output) noexcept {
    if (bytes.size() < kHeaderSize || bytes.size() > kMaxFrameSize) {
        return false;
    }
    std::uint32_t bodySize = 0;
    MessageType type{};
    if (!decodeHeader(bytes.first(kHeaderSize), type, bodySize) ||
        bodySize != bytes.size() - kHeaderSize) {
        return false;
    }
    output = FrameView{type, bytes.subspan(kHeaderSize)};
    return true;
}

std::vector<std::uint8_t> encode(const HelloRequest& message) {
    auto writer = begin(MessageType::helloRequest);
    writer.appendU64(message.requestId);
    writer.appendU32(message.clientArchitectureBits);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const HelloResponse& message) {
    auto writer = begin(MessageType::helloResponse);
    writer.appendU64(message.responseTo);
    writer.appendU32(static_cast<std::uint32_t>(message.status));
    writer.appendU64(message.engineEpoch);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const KeyRequest& message) {
    auto writer = begin(MessageType::keyRequest);
    writer.appendU64(message.requestId);
    writer.appendU64(message.contextId);
    writer.appendU32(message.virtualKey);
    writer.appendU32(message.keyFlags);
    return writer.finish();
}

std::vector<std::uint8_t> encode(const KeyResponse& message) {
    if (message.commitUtf8.size() > 32) {
        return {};
    }
    auto writer = begin(MessageType::keyResponse);
    writer.appendU64(message.responseTo);
    writer.appendU32(static_cast<std::uint32_t>(message.status));
    writer.appendU8(message.handled ? 1U : 0U);
    writer.appendString(message.commitUtf8);
    return writer.finish();
}

bool decode(const FrameView& frame, HelloRequest& message) noexcept {
    if (frame.type != MessageType::helloRequest) {
        return false;
    }
    Reader reader(frame.body);
    return reader.readU64(message.requestId) && reader.readU32(message.clientArchitectureBits) &&
           reader.done();
}

bool decode(const FrameView& frame, HelloResponse& message) noexcept {
    if (frame.type != MessageType::helloResponse) {
        return false;
    }
    Reader reader(frame.body);
    std::uint32_t status = 0;
    if (!reader.readU64(message.responseTo) || !reader.readU32(status) ||
        !reader.readU64(message.engineEpoch) || !reader.done()) {
        return false;
    }
    message.status = static_cast<Status>(status);
    return status <= static_cast<std::uint32_t>(Status::unsupported);
}

bool decode(const FrameView& frame, KeyRequest& message) noexcept {
    if (frame.type != MessageType::keyRequest) {
        return false;
    }
    Reader reader(frame.body);
    return reader.readU64(message.requestId) && reader.readU64(message.contextId) &&
           reader.readU32(message.virtualKey) && reader.readU32(message.keyFlags) && reader.done();
}

bool decode(const FrameView& frame, KeyResponse& message) noexcept {
    if (frame.type != MessageType::keyResponse) {
        return false;
    }
    Reader reader(frame.body);
    std::uint32_t status = 0;
    std::uint8_t handled = 0;
    try {
        if (!reader.readU64(message.responseTo) || !reader.readU32(status) ||
            !reader.readU8(handled) || handled > 1 || !reader.readString(message.commitUtf8) ||
            !reader.done() || message.commitUtf8.size() > 32) {
            return false;
        }
    } catch (...) {
        return false;
    }
    message.status = static_cast<Status>(status);
    message.handled = handled != 0;
    return status <= static_cast<std::uint32_t>(Status::unsupported);
}

} // namespace fcitx::windows::protocol
