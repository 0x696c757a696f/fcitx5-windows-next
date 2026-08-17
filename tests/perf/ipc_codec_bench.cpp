#include "protocol.h"

#include <Windows.h>

#include <cstdint>
#include <iostream>

int main() {
    constexpr std::uint64_t iterations = 200'000;
    LARGE_INTEGER frequency{};
    LARGE_INTEGER started{};
    LARGE_INTEGER finished{};
    if (!QueryPerformanceFrequency(&frequency) || !QueryPerformanceCounter(&started)) {
        std::cerr << "high-resolution clock unavailable\n";
        return 1;
    }

    std::uint64_t checksum = 0;
    for (std::uint64_t index = 0; index < iterations; ++index) {
        const auto bytes = fcitx::windows::protocol::encode(
            fcitx::windows::protocol::KeyRequest{index + 1, 7, 'A', 0});
        fcitx::windows::protocol::FrameView frame;
        fcitx::windows::protocol::KeyRequest decoded;
        if (!fcitx::windows::protocol::decodeFrame(bytes, frame) ||
            !fcitx::windows::protocol::decode(frame, decoded)) {
            std::cerr << "codec failed during benchmark\n";
            return 1;
        }
        checksum += decoded.requestId;
    }
    QueryPerformanceCounter(&finished);
    const double elapsedSeconds =
        static_cast<double>(finished.QuadPart - started.QuadPart) /
        static_cast<double>(frequency.QuadPart);
    const double nanosecondsPerOperation = elapsedSeconds * 1'000'000'000.0 / iterations;
    const double operationsPerSecond = static_cast<double>(iterations) / elapsedSeconds;
    std::cout << "{\"benchmark\":\"ipc_codec\",\"architecture_bits\":"
              << sizeof(void*) * 8 << ",\"iterations\":" << iterations
              << ",\"ns_per_operation\":" << nanosecondsPerOperation
              << ",\"operations_per_second\":" << operationsPerSecond
              << ",\"checksum\":" << checksum << "}\n";
    return 0;
}
