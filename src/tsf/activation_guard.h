#pragma once

#include <algorithm>
#include <array>
#include <chrono>
#include <cstdint>
#include <filesystem>
#include <string>
#include <string_view>
#include <vector>

extern "C" {
struct Fcitx5TsfActivationAttemptAbi {
    std::uint8_t fail_open;
    std::uint8_t active;
    std::size_t reason_len;
    std::size_t attempt_path_len;
};

std::size_t fcitx5_tsf_activation_guard_default_data_root(wchar_t* out,
                                                          std::size_t capacity);
std::size_t fcitx5_tsf_activation_guard_marker_path(const wchar_t* data_root,
                                                    std::size_t data_root_len,
                                                    wchar_t* out,
                                                    std::size_t capacity);
std::uint8_t fcitx5_tsf_activation_guard_status(const wchar_t* data_root,
                                                std::size_t data_root_len,
                                                std::uint8_t* reason_out,
                                                std::size_t reason_capacity,
                                                std::size_t* reason_len);
std::uint8_t fcitx5_tsf_activation_guard_disable(const wchar_t* data_root,
                                                 std::size_t data_root_len,
                                                 const std::uint8_t* reason,
                                                 std::size_t reason_len);
std::uint8_t fcitx5_tsf_activation_guard_clear(const wchar_t* data_root,
                                               std::size_t data_root_len);
Fcitx5TsfActivationAttemptAbi fcitx5_tsf_activation_attempt_begin(
    const wchar_t* data_root, std::size_t data_root_len,
    std::uint64_t stale_threshold_seconds, std::uint8_t* reason_out,
    std::size_t reason_capacity, wchar_t* attempt_path_out,
    std::size_t attempt_path_capacity);
void fcitx5_tsf_activation_attempt_finish(const wchar_t* attempt_path,
                                          std::size_t attempt_path_len);
}

namespace fcitx::windows::tsf {

struct ActivationGuardStatus {
    bool disabled{};
    std::filesystem::path markerPath;
    std::string reason;
};

namespace detail {

[[nodiscard]] inline std::filesystem::path pathFromRustBuffer(
    std::size_t (*writer)(wchar_t*, std::size_t)) {
    const std::size_t required = writer(nullptr, 0);
    if (required == 0) return {};
    std::wstring buffer(required, L'\0');
    const std::size_t written = writer(buffer.data(), buffer.size());
    buffer.resize((std::min)(written, buffer.size()));
    return std::filesystem::path(buffer);
}

[[nodiscard]] inline std::filesystem::path pathFromRustBuffer(
    const std::filesystem::path& dataRoot,
    std::size_t (*writer)(const wchar_t*, std::size_t, wchar_t*, std::size_t)) {
    const std::wstring root = dataRoot.wstring();
    const std::size_t required = writer(root.data(), root.size(), nullptr, 0);
    if (required == 0) return {};
    std::wstring buffer(required, L'\0');
    const std::size_t written =
        writer(root.data(), root.size(), buffer.data(), buffer.size());
    buffer.resize((std::min)(written, buffer.size()));
    return std::filesystem::path(buffer);
}

} // namespace detail

[[nodiscard]] inline std::filesystem::path defaultActivationGuardDataRoot() noexcept {
    try {
        return detail::pathFromRustBuffer(fcitx5_tsf_activation_guard_default_data_root);
    } catch (...) {
        return {};
    }
}

[[nodiscard]] inline std::filesystem::path activationGuardMarkerPath(
    const std::filesystem::path& dataRoot) {
    return detail::pathFromRustBuffer(dataRoot, fcitx5_tsf_activation_guard_marker_path);
}

[[nodiscard]] inline ActivationGuardStatus activationGuardStatus(
    const std::filesystem::path& dataRoot) noexcept {
    ActivationGuardStatus status;
    try {
        status.markerPath = activationGuardMarkerPath(dataRoot);
        const std::wstring root = dataRoot.wstring();
        std::array<std::uint8_t, 512> reason{};
        std::size_t reasonLength = 0;
        status.disabled = fcitx5_tsf_activation_guard_status(
                              root.data(), root.size(), reason.data(), reason.size(),
                              &reasonLength) != 0;
        status.reason.assign(reinterpret_cast<const char*>(reason.data()),
                             (std::min)(reasonLength, reason.size()));
    } catch (...) {
        status = {};
    }
    return status;
}

[[nodiscard]] inline bool disableActivationGuard(const std::filesystem::path& dataRoot,
                                                 std::string_view reason) noexcept {
    try {
        const std::wstring root = dataRoot.wstring();
        return fcitx5_tsf_activation_guard_disable(
                   root.data(), root.size(),
                   reinterpret_cast<const std::uint8_t*>(reason.data()), reason.size()) != 0;
    } catch (...) {
        return false;
    }
}

[[nodiscard]] inline bool clearActivationGuard(const std::filesystem::path& dataRoot) noexcept {
    try {
        const std::wstring root = dataRoot.wstring();
        return fcitx5_tsf_activation_guard_clear(root.data(), root.size()) != 0;
    } catch (...) {
        return false;
    }
}

class ActivationAttempt final {
public:
    ActivationAttempt() = default;
    ~ActivationAttempt() { finish(); }

    ActivationAttempt(const ActivationAttempt&) = delete;
    ActivationAttempt& operator=(const ActivationAttempt&) = delete;
    ActivationAttempt(ActivationAttempt&& other) noexcept
        : dataRoot_(std::move(other.dataRoot_)),
          attemptPath_(std::move(other.attemptPath_)),
          reason_(std::move(other.reason_)),
          failOpen_(other.failOpen_),
          active_(other.active_) {
        other.active_ = false;
    }
    ActivationAttempt& operator=(ActivationAttempt&& other) noexcept {
        if (this != &other) {
            finish();
            dataRoot_ = std::move(other.dataRoot_);
            attemptPath_ = std::move(other.attemptPath_);
            reason_ = std::move(other.reason_);
            failOpen_ = other.failOpen_;
            active_ = other.active_;
            other.active_ = false;
        }
        return *this;
    }

    [[nodiscard]] static ActivationAttempt begin(
        const std::filesystem::path& dataRoot,
        std::chrono::seconds staleThreshold = std::chrono::seconds(15)) noexcept {
        ActivationAttempt attempt;
        try {
            attempt.dataRoot_ = dataRoot;
            const std::wstring root = dataRoot.wstring();
            std::array<std::uint8_t, 512> reason{};
            std::wstring attemptPath(32768, L'\0');
            const auto result = fcitx5_tsf_activation_attempt_begin(
                root.data(), root.size(),
                static_cast<std::uint64_t>(staleThreshold.count()), reason.data(),
                reason.size(), attemptPath.data(), attemptPath.size());
            attempt.failOpen_ = result.fail_open != 0;
            attempt.active_ = result.active != 0;
            attempt.reason_.assign(reinterpret_cast<const char*>(reason.data()),
                                   (std::min)(result.reason_len, reason.size()));
            attemptPath.resize((std::min)(result.attempt_path_len, attemptPath.size()));
            attempt.attemptPath_ = std::filesystem::path(attemptPath);
        } catch (...) {
            attempt = {};
        }
        return attempt;
    }

    [[nodiscard]] bool failOpen() const noexcept { return failOpen_; }
    [[nodiscard]] const std::string& reason() const noexcept { return reason_; }
    void finish() noexcept {
        if (!active_ || attemptPath_.empty()) return;
        try {
            const std::wstring attemptPath = attemptPath_.wstring();
            fcitx5_tsf_activation_attempt_finish(attemptPath.data(), attemptPath.size());
        } catch (...) {
        }
        active_ = false;
    }
    void disableAndFinish(std::string_view reason) noexcept {
        reason_ = reason.empty() ? "unspecified" : std::string(reason.substr(0, 160));
        failOpen_ = true;
        if (!dataRoot_.empty()) (void)disableActivationGuard(dataRoot_, reason_);
        finish();
    }

private:
    std::filesystem::path dataRoot_;
    std::filesystem::path attemptPath_;
    std::string reason_;
    bool failOpen_{};
    bool active_{};
};

} // namespace fcitx::windows::tsf
