#include "protocol.h"

#include <cstdint>
#include <iostream>
#include <vector>

namespace {

bool expect(bool condition, const char* message) {
    if (!condition) {
        std::cerr << message << '\n';
    }
    return condition;
}

} // namespace

int main() {
    using namespace fcitx::windows::protocol;

    const KeyRequest input{42, 7, static_cast<std::uint32_t>('A'), 0};
    const auto bytes = encode(input);
    FrameView frame;
    KeyRequest output;
    if (!expect(decodeFrame(bytes, frame), "valid frame rejected") ||
        !expect(decode(frame, output), "valid key request rejected") ||
        !expect(output.requestId == input.requestId && output.contextId == input.contextId &&
                    output.virtualKey == input.virtualKey && output.keyFlags == input.keyFlags,
                "roundtrip changed key request")) {
        return 1;
    }

    for (std::size_t size = 0; size < bytes.size(); ++size) {
        if (!expect(!decodeFrame(std::span(bytes).first(size), frame),
                    "truncated frame accepted")) {
            return 1;
        }
    }

    auto wrongVersion = bytes;
    wrongVersion[4] = static_cast<std::uint8_t>(kVersion + 1);
    if (!expect(!decodeFrame(wrongVersion, frame), "wrong protocol version accepted")) {
        return 1;
    }

    const KeyResponse responseInput{42, Status::ok, true, "a"};
    const auto responseBytes = encode(responseInput);
    KeyResponse responseOutput;
    if (!expect(decodeFrame(responseBytes, frame) && decode(frame, responseOutput),
                "valid response rejected") ||
        !expect(responseOutput.responseTo == 42 && responseOutput.handled &&
                    responseOutput.commitUtf8 == "a",
                "roundtrip changed response")) {
        return 1;
    }

    return 0;
}
