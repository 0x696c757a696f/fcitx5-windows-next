#pragma once

#include "state_machine.h"

#include <string>
#include <string_view>
#include <utility>

namespace fcitx::windows::launcher {

enum class LoadStateResult {
    missing,
    loaded,
    invalid,
    ioError,
};

class StateStore final {
public:
    explicit StateStore(std::wstring path) : path_(std::move(path)) {}

    [[nodiscard]] LoadStateResult load(LauncherState& state) const noexcept;
    [[nodiscard]] bool save(LauncherState state) const noexcept;
    [[nodiscard]] const std::wstring& path() const noexcept { return path_; }

private:
    std::wstring path_;
};

[[nodiscard]] std::wstring defaultStateStorePath() noexcept;
[[nodiscard]] bool isPersistentState(LauncherState state) noexcept;

} // namespace fcitx::windows::launcher
