#include "state_store.h"

#include <fcitx5_windows/release_identity.h>

#include <ShlObj.h>
#include <Windows.h>

#include <array>
#include <cstdint>
#include <limits>
#include <optional>
#include <string>

namespace fcitx::windows::launcher {
namespace {

std::string_view legacyStateName(LauncherState state) noexcept {
    switch (state) {
    case LauncherState::normal:
        return "normal";
    case LauncherState::userStopped:
        return "user-stopped";
    case LauncherState::updating:
        return "updating";
    case LauncherState::uninstalling:
        return "uninstalling";
    case LauncherState::crashBackoff:
        return "crash-backoff";
    case LauncherState::safeMode:
        return "safe-mode";
    }
    return {};
}

bool parseStateName(std::string_view text, LauncherState& state) noexcept {
    if (text == "normal")
        state = LauncherState::normal;
    else if (text == "user-stopped")
        state = LauncherState::userStopped;
    else if (text == "updating")
        state = LauncherState::updating;
    else if (text == "uninstalling")
        state = LauncherState::uninstalling;
    else if (text == "crash-backoff")
        state = LauncherState::crashBackoff;
    else if (text == "safe-mode")
        state = LauncherState::safeMode;
    else
        return false;
    return true;
}

std::optional<std::string_view> lineValue(std::string_view text, std::string_view key) {
    const std::string marker = std::string(key) + "=";
    std::size_t position = 0;
    while (position <= text.size()) {
        const std::size_t lineEnd = text.find('\n', position);
        std::string_view line =
            lineEnd == std::string_view::npos
                ? text.substr(position)
                : text.substr(position, lineEnd - position);
        if (!line.empty() && line.back() == '\r')
            line.remove_suffix(1);
        if (line.starts_with(marker))
            return line.substr(marker.size());
        if (lineEnd == std::string_view::npos)
            break;
        position = lineEnd + 1;
    }
    return std::nullopt;
}

bool parseUnsigned(std::string_view value, std::uint64_t& output) noexcept {
    if (value.empty())
        return false;
    std::uint64_t result = 0;
    for (const char character : value) {
        if (character < '0' || character > '9')
            return false;
        const auto digit = static_cast<std::uint64_t>(character - '0');
        if (result > (std::numeric_limits<std::uint64_t>::max() - digit) / 10U)
            return false;
        result = result * 10U + digit;
    }
    output = result;
    return true;
}

bool parseSnapshot(std::string_view text, LauncherSnapshot& snapshot) noexcept {
    if (!text.empty() && text.back() == '\n') {
        std::string_view legacy = text;
        legacy.remove_suffix(1);
        LauncherState state{};
        if (parseStateName(legacy, state)) {
            snapshot = {.state = state};
            return true;
        }
    }
    const auto format = lineValue(text, "format_version");
    const auto stateName = lineValue(text, "state");
    const auto crashes = lineValue(text, "consecutive_startup_crashes");
    const auto nextStart = lineValue(text, "next_start_allowed_ms");
    LauncherState state{};
    std::uint64_t parsedCrashes = 0;
    std::uint64_t parsedNextStart = 0;
    if (!format || *format != "2" || !stateName || !parseStateName(*stateName, state) ||
        !crashes || !parseUnsigned(*crashes, parsedCrashes) ||
        parsedCrashes > static_cast<std::uint64_t>(std::numeric_limits<unsigned>::max()) ||
        !nextStart ||
        !parseUnsigned(*nextStart, parsedNextStart)) {
        return false;
    }
    snapshot = {state, static_cast<unsigned>(parsedCrashes), parsedNextStart};
    return true;
}

} // namespace

bool isPersistentState(LauncherState state) noexcept { return !legacyStateName(state).empty(); }

std::wstring defaultStateStorePath() noexcept {
    PWSTR rawPath = nullptr;
    if (FAILED(SHGetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_CREATE, nullptr, &rawPath))) {
        return {};
    }
    std::wstring result;
    try {
        result = rawPath;
        result += L"\\";
        result += kReleaseIdentity.data_directory;
        const int createResult = SHCreateDirectoryExW(nullptr, result.c_str(), nullptr);
        if (createResult != ERROR_SUCCESS && createResult != ERROR_ALREADY_EXISTS &&
            createResult != ERROR_FILE_EXISTS) {
            result.clear();
        } else {
            result += L"\\launcher-state.v1";
        }
    } catch (...) {
        result.clear();
    }
    CoTaskMemFree(rawPath);
    return result;
}

LoadStateResult StateStore::load(LauncherState& state) const noexcept {
    LauncherSnapshot snapshot;
    const auto result = load(snapshot);
    if (result == LoadStateResult::loaded)
        state = snapshot.state;
    return result;
}

LoadStateResult StateStore::load(LauncherSnapshot& snapshot) const noexcept {
    if (path_.empty())
        return LoadStateResult::ioError;
    HANDLE file = CreateFileW(path_.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
                              FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) {
        return GetLastError() == ERROR_FILE_NOT_FOUND ? LoadStateResult::missing
                                                      : LoadStateResult::ioError;
    }
    std::array<char, 256> bytes{};
    DWORD read = 0;
    const BOOL success =
        ReadFile(file, bytes.data(), static_cast<DWORD>(bytes.size()), &read, nullptr);
    CloseHandle(file);
    if (!success)
        return LoadStateResult::ioError;
    LauncherSnapshot parsed{};
    if (!parseSnapshot(std::string_view(bytes.data(), read), parsed))
        return LoadStateResult::invalid;
    snapshot = parsed;
    return LoadStateResult::loaded;
}

bool StateStore::save(LauncherState state) const noexcept {
    return save(LauncherSnapshot{state, 0, 0});
}

bool StateStore::save(LauncherSnapshot snapshot) const noexcept {
    const auto stateName = legacyStateName(snapshot.state);
    if (path_.empty() || stateName.empty())
        return false;
    const std::string text =
        "format_version=2\nstate=" + std::string(stateName) +
        "\nconsecutive_startup_crashes=" +
        std::to_string(snapshot.consecutiveStartupCrashes) +
        "\nnext_start_allowed_ms=" +
        std::to_string(snapshot.nextStartAllowedMilliseconds) + "\n";
    const std::wstring temporary = path_ + L".tmp." + std::to_wstring(GetCurrentProcessId()) +
                                   L"." + std::to_wstring(GetTickCount64());
    HANDLE file = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                              FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH, nullptr);
    if (file == INVALID_HANDLE_VALUE)
        return false;
    DWORD written = 0;
    const bool success =
        WriteFile(file, text.data(), static_cast<DWORD>(text.size()), &written, nullptr) != FALSE &&
        written == static_cast<DWORD>(text.size()) && FlushFileBuffers(file) != FALSE;
    CloseHandle(file);
    if (!success || !MoveFileExW(temporary.c_str(), path_.c_str(),
                                 MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
        DeleteFileW(temporary.c_str());
        return false;
    }
    return true;
}

} // namespace fcitx::windows::launcher
