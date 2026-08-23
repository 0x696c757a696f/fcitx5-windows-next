#include "state_store.h"

#include <cstdint>
#include <string>
#include <vector>

namespace {

static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));

struct Fcitx5LauncherSnapshot {
    std::uint32_t state;
    std::uint32_t consecutiveStartupCrashes;
    std::uint64_t nextStartAllowedMilliseconds;
};

extern "C" {
std::uint8_t fcitx5_launcher_state_is_persistent(std::uint32_t state);
std::uint32_t fcitx5_launcher_state_store_load_utf16(const std::uint16_t* path, std::size_t len,
                                                     Fcitx5LauncherSnapshot* snapshot);
std::uint8_t fcitx5_launcher_state_store_save_utf16(const std::uint16_t* path, std::size_t len,
                                                    Fcitx5LauncherSnapshot snapshot);
std::size_t fcitx5_launcher_default_state_store_path_utf16(std::uint16_t* output,
                                                           std::size_t capacity);
}

const std::uint16_t* utf16Data(const std::wstring& value) noexcept {
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

Fcitx5LauncherSnapshot toRust(fcitx::windows::launcher::LauncherSnapshot snapshot) noexcept {
    return {static_cast<std::uint32_t>(snapshot.state), snapshot.consecutiveStartupCrashes,
            snapshot.nextStartAllowedMilliseconds};
}

fcitx::windows::launcher::LauncherSnapshot fromRust(Fcitx5LauncherSnapshot snapshot) noexcept {
    return {static_cast<fcitx::windows::launcher::LauncherState>(snapshot.state),
            snapshot.consecutiveStartupCrashes, snapshot.nextStartAllowedMilliseconds};
}

} // namespace

namespace fcitx::windows::launcher {

bool isPersistentState(LauncherState state) noexcept {
    return fcitx5_launcher_state_is_persistent(static_cast<std::uint32_t>(state)) != 0;
}

std::wstring defaultStateStorePath() noexcept {
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

LoadStateResult StateStore::load(LauncherState& state) const noexcept {
    LauncherSnapshot snapshot;
    const auto result = load(snapshot);
    if (result == LoadStateResult::loaded)
        state = snapshot.state;
    return result;
}

LoadStateResult StateStore::load(LauncherSnapshot& snapshot) const noexcept {
    Fcitx5LauncherSnapshot rustSnapshot{};
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

bool StateStore::save(LauncherState state) const noexcept {
    return save(LauncherSnapshot{state, 0, 0});
}

bool StateStore::save(LauncherSnapshot snapshot) const noexcept {
    return fcitx5_launcher_state_store_save_utf16(utf16Data(path_), path_.size(),
                                                  toRust(snapshot)) != 0;
}

} // namespace fcitx::windows::launcher
