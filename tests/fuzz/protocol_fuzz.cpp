#include "protocol.h"

#include <cstddef>
#include <cstdint>
#include <fstream>
#include <iterator>
#include <random>
#include <span>
#include <string>
#include <vector>

namespace {

void consume(std::span<const std::uint8_t> bytes) {
    using namespace fcitx::windows::protocol;
    FrameView frame;
    if (!decodeFrame(bytes, frame)) return;
    switch (frame.type) {
    case MessageType::helloRequest: {
        HelloRequest message;
        (void)decode(frame, message);
        break;
    }
    case MessageType::helloResponse: {
        HelloResponse message;
        (void)decode(frame, message);
        break;
    }
    case MessageType::keyRequest: {
        KeyRequest message;
        (void)decode(frame, message);
        break;
    }
    case MessageType::keyResponse: {
        KeyResponse message;
        (void)decode(frame, message);
        break;
    }
    case MessageType::launcherRequest: {
        LauncherRequest message;
        (void)decode(frame, message);
        break;
    }
    case MessageType::launcherResponse: {
        LauncherResponse message;
        (void)decode(frame, message);
        break;
    }
    case MessageType::candidateSelectRequest: {
        CandidateSelectRequest message;
        (void)decode(frame, message);
        break;
    }
    case MessageType::candidateSelectResponse: {
        CandidateSelectResponse message;
        (void)decode(frame, message);
        break;
    }
    case MessageType::stateRequest: {
        StateRequest message;
        (void)decode(frame, message);
        break;
    }
    }
}

} // namespace

extern "C" int LLVMFuzzerTestOneInput(const std::uint8_t* data, std::size_t size) {
    consume(std::span(data, size));
    return 0;
}

int main(int argc, char** argv) {
    if (argc > 1) {
        for (int index = 1; index < argc; ++index) {
            std::ifstream input(argv[index], std::ios::binary);
            std::vector<std::uint8_t> bytes((std::istreambuf_iterator<char>(input)),
                                            std::istreambuf_iterator<char>());
            if (bytes.size() <= fcitx::windows::protocol::kMaxControlFrameSize) consume(bytes);
        }
        return 0;
    }

    std::mt19937_64 random(0x34574346U);
    std::vector<std::uint8_t> bytes;
    for (std::size_t iteration = 0; iteration < 20'000; ++iteration) {
        const std::size_t size = static_cast<std::size_t>(random() % 1024U);
        bytes.resize(size);
        for (auto& byte : bytes) byte = static_cast<std::uint8_t>(random());
        consume(bytes);
    }
    return 0;
}
