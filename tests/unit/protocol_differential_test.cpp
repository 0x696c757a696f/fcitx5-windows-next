// Wire freeze + bridge regression gate for the FCW4 protocol.
//
// Phase 1 (pre-cutover, completed): the C++ `fcitx5::protocol` codec and the
// Rust `fcitx5-protocol-core` codec were proven byte- and rejection-equivalent
// (fixed corpus + randomized corpora + mutated frames).
//
// Phase 2 (this file, post-cutover): `protocol/protocol.cpp` is a thin marshalling
// bridge over the Rust codec, so the independent-implementation differential is
// no longer possible. Instead this test pins the wire:
//
//   * `--dump-golden` prints a generated include file (`protocol_wire_golden.inc`)
//     with the exact bytes produced by the pre-cutover C++ codec for a fixed
//     corpus covering all 11 message types.
//   * the default run re-encodes the same corpus through the current
//     `protocol.h` API and requires byte-identical output (wire freeze), then
//     checks randomized roundtrips and rejection of mutated/truncated frames.

#include "protocol.h"
#include "protocol_ffi.h"
#include "protocol_wire_golden.inc"

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <random>
#include <span>
#include <string>
#include <vector>

namespace {

using namespace fcitx::windows::protocol;

bool expect(bool condition, const char* message) {
    if (!condition) {
        std::cerr << message << '\n';
    }
    return condition;
}

KeyRequest makeKeyRequest(std::uint64_t requestId, std::uint64_t contextId) {
    return KeyRequest{Metadata{requestId, 0, 99, 3, contextId, 11, 13},
                      static_cast<std::uint32_t>('A'), kKeyFlagDeadKey,
                      0x1e, false, true, 0x04090409ULL, "a",
                      "pinyin",
                      true, "\xe4\xbd\xa0""a", 2, 2,
                      CaretRect{true, -100, 200, -98, 222, 144}};
}

KeyResponse makeKeyResponse(std::uint64_t requestId, std::uint64_t responseTo,
                            std::uint64_t contextId) {
    KeyResponse response{Metadata{requestId, responseTo, 99, 3, contextId, 11, 14},
                         Status::ok, true, "a", "ni", 2};
    response.candidates = {{101, "1", "\xe4\xbd\xa0", "n\xc7\x90"}};
    response.selectedCandidate = 0;
    response.candidateTotal = 1;
    response.candidateVisibility = 1;
    response.deleteSurroundingText = true;
    response.deleteSurroundingOffset = -1;
    response.deleteSurroundingSize = 1;
    response.forwardKey = true;
    response.forwardKeySym = 0xff0d;
    response.forwardKeyStates = 4;
    response.forwardKeyCode = 28;
    response.forwardKeyRelease = true;
    response.caret = CaretRect{true, -100, 200, -98, 222, 144};
    response.popupAllowed = false;
    response.contentLocaleUtf8 = "ja-JP";
    return response;
}

// Fixed corpus covering every message type. Order is part of the golden wire:
// the dump mode and the verify mode must construct exactly the same samples.
void appendFixedCorpus(std::vector<std::vector<std::uint8_t>>& samples) {
    samples.push_back(encode(makeKeyRequest(42, 7)));
    samples.push_back(encode(makeKeyResponse(43, 42, 7)));

    const HelloRequest helloInput{Metadata{1, 0, 0, 3, 0, 0, 0}, 64, 100};
    samples.push_back(encode(helloInput));

    const HelloResponse helloResponse{Metadata{2, 1, 7, 3, 0, 0, 0}, Status::ok, 64};
    samples.push_back(encode(helloResponse));

    const CandidateSelectRequest selectInput{
        Metadata{3, 0, 99, 3, 7, 11, 14}, 1234, 0x0b02};
    samples.push_back(encode(selectInput));

    const CandidateSelectResponse selectResponse{
        Metadata{4, 3, 99, 3, 7, 0, 15}, Status::ok};
    samples.push_back(encode(selectResponse));

    const StateRequest stateInput{Metadata{5, 0, 99, 3, 7, 11, 14}};
    samples.push_back(encode(stateInput));

    const EngineStatusRequest engineStatusInput{Metadata{6, 0, 99, 3, 0, 0, 0}};
    samples.push_back(encode(engineStatusInput));

    const EngineStatusResponse engineStatusResponse{
        Metadata{7, 6, 99, 3, 0, 0, 0}, Status::ok,
        "pinyin", "Pinyin", "\xe6\x8b\xbc\xe9\x9f\xb3", "\xe6\x8b\xbc"};
    samples.push_back(encode(engineStatusResponse));

    for (std::uint32_t rawCommand =
             static_cast<std::uint32_t>(LauncherCommand::startDemand);
         rawCommand <= static_cast<std::uint32_t>(LauncherCommand::shutdown); ++rawCommand) {
        const LauncherRequest launcherInput{
            Metadata{rawCommand, 0, 0, 3, 0, 0, 0},
            static_cast<LauncherCommand>(rawCommand)};
        samples.push_back(encode(launcherInput));
    }

    const LauncherResponse launcherResponse{
        Metadata{9, 8, 0, 3, 0, 0, 0}, Status::ok, 1, 2, 3, true, 250,
        "rime", "Rime", "\xe4\xb8\xad\xe6\xb4\xb2\xe9\x9f\xb5", "\xe4\xb8\xad"};
    samples.push_back(encode(launcherResponse));
}

int dumpGolden() {
    std::vector<std::vector<std::uint8_t>> samples;
    appendFixedCorpus(samples);

    std::size_t total = 0;
    for (const auto& sample : samples) {
        total += sample.size();
    }

    std::cout
        << "// Generated by tests/unit/protocol_differential_test.cpp --dump-golden\n"
        << "// Wire freeze for the E1 protocol-core cutover (pre-cutover C++ codec\n"
        << "// output). Do not edit by hand; regenerate only from the pre-cutover\n"
        << "// codec before intentionally changing the FCW4 wire.\n"
        << "#pragma once\n\n"
        << "#include <cstddef>\n"
        << "#include <cstdint>\n\n"
        << "static constexpr std::uint32_t kGoldenSampleCount = " << samples.size() << ";\n\n"
        << "static constexpr std::uint32_t kGoldenSampleSizes[kGoldenSampleCount] = {";
    for (std::size_t index = 0; index < samples.size(); ++index) {
        std::cout << (index == 0 ? "" : ", ") << samples[index].size();
    }
    std::cout << "};\n\n";
    std::cout << "static constexpr std::size_t kGoldenSampleBytesTotal = " << total << ";\n\n";
    std::cout << "static constexpr std::uint8_t kGoldenSampleBytes[kGoldenSampleBytesTotal] = {\n";
    std::size_t column = 0;
    for (const auto& sample : samples) {
        for (std::uint8_t byte : sample) {
            if (column != 0) {
                std::cout << ' ';
            }
            std::cout << "0x" << (byte < 0x10 ? "0" : "") << std::hex << static_cast<int>(byte)
                      << std::dec << ',';
            if (++column == 16) {
                std::cout << '\n';
                column = 0;
            }
        }
    }
    if (column != 0) {
        std::cout << '\n';
    }
    std::cout << "};\n";
    return 0;
}

int verifyGolden() {
    std::vector<std::vector<std::uint8_t>> samples;
    appendFixedCorpus(samples);

    if (!expect(samples.size() == kGoldenSampleCount,
                "golden sample count mismatch")) {
        return 1;
    }
    const std::uint8_t* cursor = kGoldenSampleBytes;
    for (std::size_t index = 0; index < samples.size(); ++index) {
        if (!expect(samples[index].size() == kGoldenSampleSizes[index],
                    "golden sample size mismatch") ||
            !expect(std::memcmp(samples[index].data(), cursor, samples[index].size()) == 0,
                    "golden sample bytes mismatch (wire changed!)")) {
            std::cerr << "  sample " << index << '\n';
            return 1;
        }
        cursor += samples[index].size();
    }

    // Randomized roundtrip: every encoded key request/response must decode
    // back to the same DTO through the current API.
    std::mt19937_64 random(0x32574346U);
    for (std::uint64_t iteration = 1; iteration <= 1'000; ++iteration) {
        const Metadata requestMetadata{iteration, 0, random() | 1U, 1U, random() | 1U,
                                       random(), random()};
        const KeyRequest propertyInput{requestMetadata, static_cast<std::uint32_t>(random()),
                                       static_cast<std::uint32_t>(random()) & kKnownKeyFlags,
                                       static_cast<std::uint32_t>(random() & 0xffU),
                                       (random() & 1U) != 0,
                                       (random() & 1U) != 0,
                                       random(), "x", "pinyin",
                                       true, "\xe4\xbd\xa0", 1, 1};
        const auto requestBytes = encode(propertyInput);
        FrameView frame;
        KeyRequest decodedRequest;
        if (!expect(!requestBytes.empty(), "random key request encoded") ||
            !expect(decodeFrame(requestBytes, frame), "random key request frame accepted") ||
            !expect(decode(frame, decodedRequest), "random key request decoded") ||
            !expect(decodedRequest.metadata == propertyInput.metadata &&
                        decodedRequest.virtualKey == propertyInput.virtualKey &&
                        decodedRequest.keyFlags == propertyInput.keyFlags &&
                        decodedRequest.scanCode == propertyInput.scanCode &&
                        decodedRequest.extendedKey == propertyInput.extendedKey &&
                        decodedRequest.popupAllowed == propertyInput.popupAllowed &&
                        decodedRequest.keyboardLayout == propertyInput.keyboardLayout &&
                        decodedRequest.logicalTextUtf8 == propertyInput.logicalTextUtf8 &&
                        decodedRequest.surroundingTextUtf8 == propertyInput.surroundingTextUtf8 &&
                        decodedRequest.caret == propertyInput.caret,
                    "random key request roundtrip mismatch")) {
            return 1;
        }

        std::string commit(static_cast<std::size_t>(random() % 64U), '\0');
        for (char& value : commit) value = static_cast<char>(random());
        KeyResponse propertyResponse{
            Metadata{iteration + 20'000, iteration, requestMetadata.engineEpoch,
                     requestMetadata.sessionId, requestMetadata.contextId,
                     requestMetadata.compositionId, requestMetadata.revision},
            static_cast<Status>(random() %
                                (static_cast<std::uint32_t>(Status::accessDenied) + 1U)),
            (random() & 1U) != 0, commit, "preedit", 3};
        propertyResponse.contentLocaleUtf8 = (iteration & 1U) != 0 ? "ja-JP" : "en-US";
        const auto responseBytes = encode(propertyResponse);
        KeyResponse decodedResponse;
        if (!expect(!responseBytes.empty(), "random key response encoded") ||
            !expect(decodeFrame(responseBytes, frame), "random key response frame accepted") ||
            !expect(decode(frame, decodedResponse), "random key response decoded") ||
            !expect(decodedResponse.commitUtf8 == propertyResponse.commitUtf8 &&
                        decodedResponse.preeditUtf8 == propertyResponse.preeditUtf8 &&
                        decodedResponse.status == propertyResponse.status &&
                        decodedResponse.handled == propertyResponse.handled &&
                        decodedResponse.contentLocaleUtf8 == propertyResponse.contentLocaleUtf8,
                    "random key response roundtrip mismatch")) {
            return 1;
        }
    }

    // Rejection of mutated and truncated frames (wire behavior freeze).
    FrameView frame;
    auto mutated = encode(makeKeyRequest(42, 7));
    mutated[4] = static_cast<std::uint8_t>(kVersion + 1);
    if (!expect(!decodeFrame(mutated, frame), "wrong version still rejected")) return 1;

    mutated = encode(makeKeyRequest(42, 7));
    mutated[6] = 0xffU;
    mutated[7] = 0xffU;
    if (!expect(!decodeFrame(mutated, frame), "unknown type still rejected")) return 1;

    mutated = encode(makeKeyRequest(42, 7));
    mutated[8] = 0xffU;
    mutated[9] = 0xffU;
    mutated[10] = 0xffU;
    mutated[11] = 0x7fU;
    if (!expect(!decodeFrame(mutated, frame), "oversize body still rejected")) return 1;

    auto truncated = encode(makeKeyRequest(42, 7));
    truncated.resize(truncated.size() / 2);
    if (!expect(!decodeFrame(truncated, frame), "truncated frame still rejected")) return 1;

    std::cout << "protocol wire freeze: " << samples.size() << " golden samples match, "
              << "randomized roundtrips and rejection behavior verified\n";
    return 0;
}

} // namespace

int main(int argc, char** argv) {
    if (argc == 2 && std::strcmp(argv[1], "--dump-golden") == 0) {
        return dumpGolden();
    }
    return verifyGolden();
}
