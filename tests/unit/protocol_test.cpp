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
                           static_cast<std::uint32_t>('A'), 0,
                           0x1e, false, true, 0x04090409ULL, "a",
                           "pinyin",
                           true, "你a", 2, 2,
                           CaretRect{true, -100, 200, -98, 222, 144}};
    const auto bytes = encode(input);
    FrameView frame;
    KeyRequest output;
    if (!expect(decodeFrame(bytes, frame), "valid frame rejected") ||
        !expect(decode(frame, output), "valid key request rejected") ||
        !expect(output.metadata == input.metadata &&
                    output.virtualKey == input.virtualKey && output.keyFlags == input.keyFlags &&
                    output.scanCode == input.scanCode &&
                    output.extendedKey == input.extendedKey &&
                    output.popupAllowed == input.popupAllowed &&
                    output.keyboardLayout == input.keyboardLayout &&
                    output.logicalTextUtf8 == input.logicalTextUtf8 &&
                    output.inputMethodUtf8 == input.inputMethodUtf8 &&
                    output.surroundingTextValid == input.surroundingTextValid &&
                    output.surroundingTextUtf8 == input.surroundingTextUtf8 &&
                    output.surroundingCursor == input.surroundingCursor &&
                    output.surroundingAnchor == input.surroundingAnchor &&
                    output.caret == input.caret,
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

    KeyResponse responseInput{Metadata{43, 42, 99, 3, 7, 11, 14},
                              Status::ok, true, "a", "ni", 2};
    responseInput.candidates = {{101, "1", "\xe4\xbd\xa0", "n\xc7\x90"}};
    responseInput.selectedCandidate = 0;
    responseInput.candidateTotal = 1;
    responseInput.candidateVisibility = 1;
    responseInput.deleteSurroundingText = true;
    responseInput.deleteSurroundingOffset = -1;
    responseInput.deleteSurroundingSize = 1;
    responseInput.forwardKey = true;
    responseInput.forwardKeySym = 0xff0d;
    responseInput.forwardKeyStates = 4;
    responseInput.forwardKeyCode = 28;
    responseInput.forwardKeyRelease = true;
    responseInput.caret = input.caret;
    const auto responseBytes = encode(responseInput);
    KeyResponse responseOutput;
    if (!expect(decodeFrame(responseBytes, frame) && decode(frame, responseOutput),
                "valid response rejected") ||
        !expect(responseOutput.metadata == responseInput.metadata && responseOutput.handled &&
                    responseOutput.commitUtf8 == "a" &&
                    responseOutput.preeditUtf8 == "ni" &&
                    responseOutput.preeditCaretUtf8 == 2 &&
                    responseOutput.candidates == responseInput.candidates &&
                    responseOutput.selectedCandidate == 0 &&
                    responseOutput.candidateVisibility == 1 &&
                    responseOutput.deleteSurroundingText &&
                    responseOutput.deleteSurroundingOffset == -1 &&
                    responseOutput.deleteSurroundingSize == 1 &&
                    responseOutput.forwardKey &&
                    responseOutput.forwardKeySym == 0xff0d &&
                    responseOutput.forwardKeyStates == 4 &&
                    responseOutput.forwardKeyCode == 28 &&
                    responseOutput.forwardKeyRelease &&
                    responseOutput.caret == responseInput.caret,
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

    const CandidateSelectRequest selectInput{
        Metadata{3, 0, 99, 3, 7, 11, 14}, 1234, 0x0b02};
    CandidateSelectRequest selectOutput;
    if (!expect(decodeFrame(encode(selectInput), frame) && decode(frame, selectOutput) &&
                    selectOutput.metadata == selectInput.metadata &&
                    selectOutput.targetProcessId == selectInput.targetProcessId &&
                    selectOutput.candidateId == selectInput.candidateId,
                "candidate selection request roundtrip failed")) return 1;

    const CandidateSelectResponse selectResponseInput{
        Metadata{4, 3, 99, 3, 7, 0, 15}, Status::ok};
    CandidateSelectResponse selectResponseOutput;
    if (!expect(decodeFrame(encode(selectResponseInput), frame) &&
                    decode(frame, selectResponseOutput) &&
                    selectResponseOutput.metadata == selectResponseInput.metadata &&
                    selectResponseOutput.status == Status::ok,
                "candidate selection response roundtrip failed")) return 1;

    const StateRequest stateInput{Metadata{5, 0, 99, 3, 7, 11, 14}};
    StateRequest stateOutput;
    if (!expect(decodeFrame(encode(stateInput), frame) && decode(frame, stateOutput) &&
                    stateOutput.metadata == stateInput.metadata,
                "state request roundtrip failed")) return 1;

    const EngineStatusRequest engineStatusInput{Metadata{6, 0, 99, 3, 0, 0, 0}};
    EngineStatusRequest engineStatusOutput;
    if (!expect(decodeFrame(encode(engineStatusInput), frame) &&
                    decode(frame, engineStatusOutput) &&
                    engineStatusOutput.metadata == engineStatusInput.metadata,
                "engine status request roundtrip failed")) return 1;

    const EngineStatusResponse engineStatusResponseInput{
        Metadata{7, 6, 99, 3, 0, 0, 0}, Status::ok,
        "pinyin", "Pinyin", "\xe6\x8b\xbc\xe9\x9f\xb3", "\xe6\x8b\xbc"};
    EngineStatusResponse engineStatusResponseOutput;
    if (!expect(decodeFrame(encode(engineStatusResponseInput), frame) &&
                    decode(frame, engineStatusResponseOutput) &&
                    engineStatusResponseOutput.metadata ==
                        engineStatusResponseInput.metadata &&
                    engineStatusResponseOutput.status ==
                        engineStatusResponseInput.status &&
                    engineStatusResponseOutput.currentInputMethodId ==
                        engineStatusResponseInput.currentInputMethodId &&
                    engineStatusResponseOutput.currentInputMethodName ==
                        engineStatusResponseInput.currentInputMethodName &&
                    engineStatusResponseOutput.currentInputMethodNativeName ==
                        engineStatusResponseInput.currentInputMethodNativeName &&
                    engineStatusResponseOutput.currentInputMethodShortLabel ==
                        engineStatusResponseInput.currentInputMethodShortLabel,
                "engine status response roundtrip failed")) return 1;

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
        Metadata{9, 8, 0, 3, 0, 0, 0}, Status::ok, 1, 2, 3, true, 250,
        "rime", "Rime", "\xe4\xb8\xad\xe6\xb4\xb2\xe9\x9f\xb5", "\xe4\xb8\xad"};
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
                        launcherResponseInput.retryAfterMilliseconds &&
                    launcherResponseOutput.currentInputMethodId ==
                        launcherResponseInput.currentInputMethodId &&
                    launcherResponseOutput.currentInputMethodName ==
                        launcherResponseInput.currentInputMethodName &&
                    launcherResponseOutput.currentInputMethodNativeName ==
                        launcherResponseInput.currentInputMethodNativeName &&
                    launcherResponseOutput.currentInputMethodShortLabel ==
                        launcherResponseInput.currentInputMethodShortLabel,
                "launcher response roundtrip failed")) return 1;

    std::mt19937_64 random(0x32574346U);
    for (std::uint64_t iteration = 1; iteration <= 10'000; ++iteration) {
        const Metadata requestMetadata{iteration, 0, random() | 1U, 1U, random() | 1U,
                                       random(), random()};
        const KeyRequest propertyInput{requestMetadata, static_cast<std::uint32_t>(random()),
                                       static_cast<std::uint32_t>(random()),
                                       static_cast<std::uint32_t>(random() & 0xffU),
                                       (random() & 1U) != 0,
                                       (random() & 1U) != 0,
                                       random(), "x", "pinyin",
                                       true, "\xe4\xbd\xa0", 1, 1};
        KeyRequest propertyOutput;
        if (!expect(decodeFrame(encode(propertyInput), frame) &&
                        decode(frame, propertyOutput) &&
                        propertyOutput.metadata == propertyInput.metadata &&
                        propertyOutput.virtualKey == propertyInput.virtualKey &&
                        propertyOutput.keyFlags == propertyInput.keyFlags &&
                        propertyOutput.scanCode == propertyInput.scanCode &&
                        propertyOutput.extendedKey == propertyInput.extendedKey &&
                        propertyOutput.popupAllowed == propertyInput.popupAllowed &&
                        propertyOutput.keyboardLayout == propertyInput.keyboardLayout &&
                        propertyOutput.logicalTextUtf8 == propertyInput.logicalTextUtf8 &&
                        propertyOutput.inputMethodUtf8 == propertyInput.inputMethodUtf8 &&
                        propertyOutput.surroundingTextValid ==
                            propertyInput.surroundingTextValid &&
                        propertyOutput.surroundingTextUtf8 ==
                            propertyInput.surroundingTextUtf8 &&
                        propertyOutput.surroundingCursor ==
                            propertyInput.surroundingCursor &&
                        propertyOutput.surroundingAnchor ==
                            propertyInput.surroundingAnchor,
                    "key request property roundtrip failed")) return 1;

        std::string commit(static_cast<std::size_t>(random() % (kMaxCommitUtf8 + 1)), '\0');
        for (char& value : commit) value = static_cast<char>(random());
        const KeyResponse propertyResponse{
            Metadata{iteration + 20'000, iteration, requestMetadata.engineEpoch,
                     requestMetadata.sessionId, requestMetadata.contextId,
                     requestMetadata.compositionId, requestMetadata.revision},
            static_cast<Status>(random() %
                                (static_cast<std::uint32_t>(Status::accessDenied) + 1U)),
            (random() & 1U) != 0, commit, "preedit", 3};
        KeyResponse propertyResponseOutput;
        if (!expect(decodeFrame(encode(propertyResponse), frame) &&
                        decode(frame, propertyResponseOutput) &&
                        propertyResponseOutput.metadata == propertyResponse.metadata &&
                        propertyResponseOutput.status == propertyResponse.status &&
                        propertyResponseOutput.handled == propertyResponse.handled &&
                        propertyResponseOutput.commitUtf8 == propertyResponse.commitUtf8 &&
                        propertyResponseOutput.preeditUtf8 == propertyResponse.preeditUtf8 &&
                        propertyResponseOutput.preeditCaretUtf8 ==
                            propertyResponse.preeditCaretUtf8,
                    "key response property roundtrip failed")) return 1;
    }

    if (!expect(encode(KeyRequest{}).empty(), "invalid request metadata encoded") ||
        !expect(encode(KeyResponse{Metadata{1, 1, 1, 1, 1, 0, 0}, Status::ok, true,
                                   std::string(kMaxCommitUtf8 + 1, 'x')})
                    .empty(),
                "oversize commit encoded")) {
        return 1;
    }
    KeyRequest invalidCaret{Metadata{88, 0, 1, 1, 1, 0, 0}, 'A', 0,
                            0, false, true, 0, {}, {},
                            false, {}, 0, 0,
                            CaretRect{true, 100, 100, 90, 110, 96}};
    if (!expect(encode(invalidCaret).empty(), "inverted caret rectangle encoded")) return 1;
    KeyRequest oversizedLogical{Metadata{89, 0, 1, 1, 1, 0, 0}, 'A', 0,
                                0, false, true, 0,
                                std::string(kMaxLogicalKeyUtf8 + 1, 'x'), {},
                                false, {}, 0, 0};
    if (!expect(encode(oversizedLogical).empty(),
                "oversize logical key text encoded")) return 1;
    KeyRequest oversizedInputMethod{Metadata{90, 0, 1, 1, 1, 0, 0}, 'A', 0,
                                    0, false, true, 0, {},
                                    std::string(kMaxInputMethodIdUtf8 + 1, 'x'),
                                    false, {}, 0, 0};
    if (!expect(encode(oversizedInputMethod).empty(),
                "oversize input method id encoded")) return 1;
    KeyRequest invalidSurroundingCursor{Metadata{91, 0, 1, 1, 1, 0, 0}, 'A', 0,
                                        0, false, true, 0, {}, {},
                                        true, "\xe4\xbd\xa0", 2, 1};
    if (!expect(encode(invalidSurroundingCursor).empty(),
                "out-of-range surrounding cursor encoded")) return 1;
    KeyRequest invalidSurroundingState{Metadata{92, 0, 1, 1, 1, 0, 0}, 'A', 0,
                                       0, false, true, 0, {}, {},
                                       false, "stale", 0, 0};
    if (!expect(encode(invalidSurroundingState).empty(),
                "invalid surrounding snapshot carried text")) return 1;
    if (!expect(encode(KeyResponse{Metadata{1, 1, 1, 1, 1, 0, 0},
                                   Status::ok, true, {}, {}, 0, {}, UINT32_MAX,
                                   0, 0, 0, 0, false, false, false, -1, 1})
                    .empty(),
                "disabled delete-surrounding carried payload")) {
        return 1;
    }
    if (!expect(encode(KeyResponse{Metadata{1, 1, 1, 1, 1, 0, 0}, Status::ok, true,
                                   {}, std::string(kMaxPreeditUtf8 + 1, 'x'), 0})
                    .empty(),
                "oversize preedit encoded") ||
        !expect(encode(KeyResponse{Metadata{1, 1, 1, 1, 1, 0, 0}, Status::ok, true,
                                   {}, "x", 2})
                    .empty(),
                "out-of-range preedit caret encoded")) {
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
