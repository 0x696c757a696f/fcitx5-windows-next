#include "protocol.h"

#include <cstdint>
#include <iostream>
#include <random>
#include <string>
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

    const KeyRequest input{Metadata{42, 0, 99, 3, 7, 11, 13},
                           static_cast<std::uint32_t>('A'), 0};
    const auto bytes = encode(input);
    FrameView frame;
    KeyRequest output;
    if (!expect(decodeFrame(bytes, frame), "valid frame rejected") ||
        !expect(decode(frame, output), "valid key request rejected") ||
        !expect(output.metadata == input.metadata &&
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

    const KeyResponse responseInput{Metadata{43, 42, 99, 3, 7, 11, 14},
                                    Status::ok, true, "a"};
    const auto responseBytes = encode(responseInput);
    KeyResponse responseOutput;
    if (!expect(decodeFrame(responseBytes, frame) && decode(frame, responseOutput),
                "valid response rejected") ||
        !expect(responseOutput.metadata == responseInput.metadata && responseOutput.handled &&
                    responseOutput.commitUtf8 == "a",
                "roundtrip changed response")) {
        return 1;
    }

    const HelloRequest helloInput{Metadata{1, 0, 0, 3, 0, 0, 0}, 64, 100};
    HelloRequest helloOutput;
    if (!expect(decodeFrame(encode(helloInput), frame) && decode(frame, helloOutput) &&
                    helloOutput.metadata == helloInput.metadata &&
                    helloOutput.clientArchitectureBits == helloInput.clientArchitectureBits &&
                    helloOutput.clientProcessId == helloInput.clientProcessId,
                "hello request roundtrip failed")) return 1;

    const HelloResponse helloResponseInput{Metadata{2, 1, 7, 3, 0, 0, 0}, Status::ok, 64};
    HelloResponse helloResponseOutput;
    if (!expect(decodeFrame(encode(helloResponseInput), frame) &&
                    decode(frame, helloResponseOutput) &&
                    helloResponseOutput.metadata == helloResponseInput.metadata &&
                    helloResponseOutput.status == helloResponseInput.status &&
                    helloResponseOutput.serverArchitectureBits ==
                        helloResponseInput.serverArchitectureBits,
                "hello response roundtrip failed")) return 1;

    for (std::uint32_t rawCommand =
             static_cast<std::uint32_t>(LauncherCommand::startDemand);
         rawCommand <= static_cast<std::uint32_t>(LauncherCommand::shutdown); ++rawCommand) {
        const LauncherRequest launcherInput{
            Metadata{rawCommand, 0, 0, 3, 0, 0, 0},
            static_cast<LauncherCommand>(rawCommand)};
        LauncherRequest launcherOutput;
        if (!expect(decodeFrame(encode(launcherInput), frame) &&
                        decode(frame, launcherOutput) &&
                        launcherOutput.metadata == launcherInput.metadata &&
                        launcherOutput.command == launcherInput.command,
                    "launcher request roundtrip failed")) return 1;
    }

    const LauncherResponse launcherResponseInput{
        Metadata{9, 8, 0, 3, 0, 0, 0}, Status::ok, 1, 2, 3, true, 250};
    LauncherResponse launcherResponseOutput;
    if (!expect(decodeFrame(encode(launcherResponseInput), frame) &&
                    decode(frame, launcherResponseOutput) &&
                    launcherResponseOutput.metadata == launcherResponseInput.metadata &&
                    launcherResponseOutput.status == launcherResponseInput.status &&
                    launcherResponseOutput.launcherState ==
                        launcherResponseInput.launcherState &&
                    launcherResponseOutput.engineState == launcherResponseInput.engineState &&
                    launcherResponseOutput.startDisposition ==
                        launcherResponseInput.startDisposition &&
                    launcherResponseOutput.safeMode == launcherResponseInput.safeMode &&
                    launcherResponseOutput.retryAfterMilliseconds ==
                        launcherResponseInput.retryAfterMilliseconds,
                "launcher response roundtrip failed")) return 1;

    std::mt19937_64 random(0x32574346U);
    for (std::uint64_t iteration = 1; iteration <= 10'000; ++iteration) {
        const Metadata requestMetadata{iteration, 0, random() | 1U, 1U, random() | 1U,
                                       random(), random()};
        const KeyRequest propertyInput{requestMetadata, static_cast<std::uint32_t>(random()),
                                       static_cast<std::uint32_t>(random())};
        KeyRequest propertyOutput;
        if (!expect(decodeFrame(encode(propertyInput), frame) &&
                        decode(frame, propertyOutput) &&
                        propertyOutput.metadata == propertyInput.metadata &&
                        propertyOutput.virtualKey == propertyInput.virtualKey &&
                        propertyOutput.keyFlags == propertyInput.keyFlags,
                    "key request property roundtrip failed")) return 1;

        std::string commit(static_cast<std::size_t>(random() % (kMaxCommitUtf8 + 1)), '\0');
        for (char& value : commit) value = static_cast<char>(random());
        const KeyResponse propertyResponse{
            Metadata{iteration + 20'000, iteration, requestMetadata.engineEpoch,
                     requestMetadata.sessionId, requestMetadata.contextId,
                     requestMetadata.compositionId, requestMetadata.revision},
            static_cast<Status>(random() %
                                (static_cast<std::uint32_t>(Status::accessDenied) + 1U)),
            (random() & 1U) != 0, commit};
        KeyResponse propertyResponseOutput;
        if (!expect(decodeFrame(encode(propertyResponse), frame) &&
                        decode(frame, propertyResponseOutput) &&
                        propertyResponseOutput.metadata == propertyResponse.metadata &&
                        propertyResponseOutput.status == propertyResponse.status &&
                        propertyResponseOutput.handled == propertyResponse.handled &&
                        propertyResponseOutput.commitUtf8 == propertyResponse.commitUtf8,
                    "key response property roundtrip failed")) return 1;
    }

    if (!expect(encode(KeyRequest{}).empty(), "invalid request metadata encoded") ||
        !expect(encode(KeyResponse{Metadata{1, 1, 1, 1, 1, 0, 0}, Status::ok, true,
                                   std::string(kMaxCommitUtf8 + 1, 'x')})
                    .empty(),
                "oversize commit encoded")) {
        return 1;
    }

    auto invalidType = bytes;
    invalidType[6] = 0xffU;
    invalidType[7] = 0xffU;
    if (!expect(!decodeFrame(invalidType, frame), "unknown message type accepted")) return 1;

    auto invalidLength = bytes;
    invalidLength[8] = 0xffU;
    invalidLength[9] = 0xffU;
    invalidLength[10] = 0xffU;
    invalidLength[11] = 0x7fU;
    if (!expect(!decodeFrame(invalidLength, frame), "oversize body length accepted")) return 1;

    auto largeControlFrame = encode(LauncherRequest{
        Metadata{99, 0, 0, 3, 0, 0, 0}, LauncherCommand::status});
    largeControlFrame.resize(kMaxHotFrameSize + 1, 0);
    const auto controlBodySize =
        static_cast<std::uint32_t>(largeControlFrame.size() - kHeaderSize);
    for (unsigned index = 0; index < 4; ++index) {
        largeControlFrame[8 + index] =
            static_cast<std::uint8_t>((controlBodySize >> (index * 8U)) & 0xffU);
    }
    LauncherRequest oversizedTypedControl;
    if (!expect(decodeFrame(largeControlFrame, frame),
                "valid control-frame size was rejected") ||
        !expect(!decode(frame, oversizedTypedControl),
                "typed launcher decoder accepted trailing payload")) {
        return 1;
    }

    return 0;
}
