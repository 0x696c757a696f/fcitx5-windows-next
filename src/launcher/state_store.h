#pragma once

#include "launcher_rust_abi.h"
#include "state_machine.h"

#include <cstdint>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

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

    [[nodiscard]] LoadStateResult load(LauncherState& state) const noexcept {
        LauncherSnapshot snapshot;
        const auto result = load(snapshot);
        if (result == LoadStateResult::loaded)
            state = snapshot.state;
        return result;
    }
    [[nodiscard]] LoadStateResult load(LauncherSnapshot& snapshot) const noexcept {
        rust_abi::Fcitx5LauncherSnapshot rustSnapshot{};
        const auto result =
            fcitx5_launcher_state_store_load_utf16(utf16Data(path_), path_.size(), &rustSnapshot);
        if (result == static_cast<std::uint32_t>(LoadStateResult::loaded)) {
            snapshot = fromRust(rustSnapshot);
        }
        switch (result) {
        case 0:
            return LoadStateResult::missing;
        case 1:
            return LoadStateResult::loaded;
        case 2:
            return LoadStateResult::invalid;
        default:
            return LoadStateResult::ioError;
        }
    }
    [[nodiscard]] bool save(LauncherState state) const noexcept {
        return save(LauncherSnapshot{state, 0, 0});
    }
    [[nodiscard]] bool save(LauncherSnapshot snapshot) const noexcept {
        return fcitx5_launcher_state_store_save_utf16(utf16Data(path_), path_.size(),
                                                      toRust(snapshot)) != 0;
    }
    [[nodiscard]] const std::wstring& path() const noexcept { return path_; }

private:
    [[nodiscard]] static const std::uint16_t* utf16Data(const std::wstring& value) noexcept {
        static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
        return reinterpret_cast<const std::uint16_t*>(value.data());
    }
    [[nodiscard]] static rust_abi::Fcitx5LauncherSnapshot toRust(
        LauncherSnapshot snapshot) noexcept {
        return {static_cast<std::uint32_t>(snapshot.state), snapshot.consecutiveStartupCrashes,
                snapshot.nextStartAllowedMilliseconds};
    }
    [[nodiscard]] static LauncherSnapshot fromRust(
        rust_abi::Fcitx5LauncherSnapshot snapshot) noexcept {
        return {static_cast<LauncherState>(snapshot.state), snapshot.consecutiveStartupCrashes,
                snapshot.nextStartAllowedMilliseconds};
    }

    std::wstring path_;
};

[[nodiscard]] inline std::wstring defaultStateStorePath() noexcept {
    const std::size_t required = fcitx5_launcher_default_state_store_path_utf16(nullptr, 0);
    if (required == 0)
        return {};
    std::vector<wchar_t> buffer(required);
    const std::size_t written = fcitx5_launcher_default_state_store_path_utf16(
        reinterpret_cast<std::uint16_t*>(buffer.data()), buffer.size());
    if (written == 0 || written > buffer.size())
        return {};
    return std::wstring(buffer.data(), buffer.data() + written);
}

[[nodiscard]] inline bool isPersistentState(LauncherState state) noexcept {
    return fcitx5_launcher_state_is_persistent(static_cast<std::uint32_t>(state)) != 0;
}

} // namespace fcitx::windows::launcher
