#include "state_store.h"

#include <Windows.h>
#include <ShlObj.h>

#include <array>
#include <cstdint>
#include <string>

namespace fcitx::windows::launcher {
namespace {

std::string_view stateName(LauncherState state) noexcept {
    switch (state) {
    case LauncherState::normal:
        return "normal\n";
    case LauncherState::userStopped:
        return "user-stopped\n";
    case LauncherState::updating:
        return "updating\n";
    case LauncherState::uninstalling:
        return "uninstalling\n";
    case LauncherState::crashBackoff:
    case LauncherState::safeMode:
        return {};
    }
    return {};
}

bool parseState(std::string_view text, LauncherState& state) noexcept {
    if (text == "normal\n") state = LauncherState::normal;
    else if (text == "user-stopped\n") state = LauncherState::userStopped;
    else if (text == "updating\n") state = LauncherState::updating;
    else if (text == "uninstalling\n") state = LauncherState::uninstalling;
    else return false;
    return true;
}

} // namespace

bool isPersistentState(LauncherState state) noexcept { return !stateName(state).empty(); }

std::wstring defaultStateStorePath() noexcept {
    PWSTR rawPath = nullptr;
    if (FAILED(SHGetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_CREATE, nullptr, &rawPath))) {
        return {};
    }
    std::wstring result;
    try {
        result = rawPath;
        result += L"\\Fcitx5WindowsNext";
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
    if (path_.empty()) return LoadStateResult::ioError;
    HANDLE file = CreateFileW(path_.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr,
                              OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) {
        return GetLastError() == ERROR_FILE_NOT_FOUND ? LoadStateResult::missing
                                                      : LoadStateResult::ioError;
    }
    std::array<char, 32> bytes{};
    DWORD read = 0;
    const BOOL success = ReadFile(file, bytes.data(), static_cast<DWORD>(bytes.size()), &read,
                                  nullptr);
    CloseHandle(file);
    if (!success) return LoadStateResult::ioError;
    LauncherState parsed{};
    if (!parseState(std::string_view(bytes.data(), read), parsed)) return LoadStateResult::invalid;
    state = parsed;
    return LoadStateResult::loaded;
}

bool StateStore::save(LauncherState state) const noexcept {
    const auto text = stateName(state);
    if (path_.empty() || text.empty()) return false;
    const std::wstring temporary = path_ + L".tmp." + std::to_wstring(GetCurrentProcessId()) +
                                   L"." + std::to_wstring(GetTickCount64());
    HANDLE file = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                              FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    DWORD written = 0;
    const bool success = WriteFile(file, text.data(), static_cast<DWORD>(text.size()), &written,
                                   nullptr) != FALSE &&
                         written == static_cast<DWORD>(text.size()) &&
                         FlushFileBuffers(file) != FALSE;
    CloseHandle(file);
    if (!success || !MoveFileExW(temporary.c_str(), path_.c_str(),
                                 MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
        DeleteFileW(temporary.c_str());
        return false;
    }
    return true;
}

} // namespace fcitx::windows::launcher
