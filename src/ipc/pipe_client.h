#pragma once

#include <Windows.h>

#include <atomic>
#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

namespace fcitx::windows::ipc {

inline constexpr wchar_t kDefaultPipeName[] = L"\\\\.\\pipe\\Fcitx5WindowsNext.v2";
inline constexpr DWORD kInputDeadlineMilliseconds = 25;

struct KeyResult {
    bool handled{};
    std::wstring commit;
};

class PipeClient final {
public:
    PipeClient();
    explicit PipeClient(std::wstring pipeName);
    ~PipeClient();

    PipeClient(const PipeClient&) = delete;
    PipeClient& operator=(const PipeClient&) = delete;

    [[nodiscard]] bool processKey(std::uint64_t contextId, std::uint32_t virtualKey,
                                  std::uint32_t keyFlags, KeyResult& result) noexcept;
    void disconnect() noexcept;

private:
    [[nodiscard]] bool connect(std::uint64_t deadline) noexcept;
    [[nodiscard]] bool handshake(std::uint64_t deadline) noexcept;
    [[nodiscard]] bool transact(const std::vector<std::uint8_t>& request,
                                std::vector<std::uint8_t>& response,
                                std::uint64_t deadline) noexcept;
    [[nodiscard]] bool transfer(bool write, void* data, std::size_t size,
                                std::uint64_t deadline) noexcept;

    std::wstring pipeName_;
    HANDLE pipe_{INVALID_HANDLE_VALUE};
    bool handshakeComplete_{};
    std::uint64_t engineEpoch_{};
    std::atomic<std::uint64_t> nextRequestId_{1};
};

} // namespace fcitx::windows::ipc
