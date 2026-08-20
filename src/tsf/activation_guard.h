#pragma once

#include <chrono>
#include <filesystem>
#include <string>
#include <string_view>

namespace fcitx::windows::tsf {

struct ActivationGuardStatus {
    bool disabled{};
    std::filesystem::path markerPath;
    std::string reason;
};

[[nodiscard]] std::filesystem::path defaultActivationGuardDataRoot() noexcept;
[[nodiscard]] std::filesystem::path activationGuardMarkerPath(
    const std::filesystem::path& dataRoot);
[[nodiscard]] ActivationGuardStatus activationGuardStatus(
    const std::filesystem::path& dataRoot) noexcept;
[[nodiscard]] bool disableActivationGuard(const std::filesystem::path& dataRoot,
                                          std::string_view reason) noexcept;
[[nodiscard]] bool clearActivationGuard(const std::filesystem::path& dataRoot) noexcept;

class ActivationAttempt final {
public:
    ActivationAttempt() = default;
    ~ActivationAttempt();

    ActivationAttempt(const ActivationAttempt&) = delete;
    ActivationAttempt& operator=(const ActivationAttempt&) = delete;
    ActivationAttempt(ActivationAttempt&& other) noexcept;
    ActivationAttempt& operator=(ActivationAttempt&& other) noexcept;

    [[nodiscard]] static ActivationAttempt begin(
        const std::filesystem::path& dataRoot,
        std::chrono::seconds staleThreshold = std::chrono::seconds(15)) noexcept;

    [[nodiscard]] bool failOpen() const noexcept { return failOpen_; }
    [[nodiscard]] const std::string& reason() const noexcept { return reason_; }
    void finish() noexcept;
    void disableAndFinish(std::string_view reason) noexcept;

private:
    std::filesystem::path dataRoot_;
    std::filesystem::path attemptPath_;
    std::string reason_;
    bool failOpen_{};
    bool active_{};
};

} // namespace fcitx::windows::tsf
