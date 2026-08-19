#pragma once

#include "protocol.h"
#include "state_machine.h"

#include <Windows.h>
#include <shellapi.h>

#include <filesystem>

namespace fcitx::windows::launcher {

enum class TrayCommand { none, restart, pause, resume, exit };

class TrayIcon final {
public:
    TrayIcon() = default;
    ~TrayIcon();

    TrayIcon(const TrayIcon&) = delete;
    TrayIcon& operator=(const TrayIcon&) = delete;

    [[nodiscard]] bool create(HINSTANCE instance,
                              const std::filesystem::path& executableDirectory);
    void update(LauncherState launcherState, EngineState engineState,
                const protocol::EngineStatusResponse& inputMethodStatus = {});
    void dispatchMessages();
    [[nodiscard]] TrayCommand takeCommand() noexcept;
    [[nodiscard]] bool valid() const noexcept { return window_ != nullptr; }
    [[nodiscard]] bool iconAdded() const noexcept { return iconAdded_; }
    [[nodiscard]] bool shellVisible() const noexcept;
    [[nodiscard]] bool usesGuidIdentity() const noexcept { return usesGuidIdentity_; }

private:
    static LRESULT CALLBACK windowProcedure(HWND window, UINT message, WPARAM wparam,
                                            LPARAM lparam) noexcept;
    LRESULT handleMessage(HWND window, UINT message, WPARAM wparam, LPARAM lparam) noexcept;
    void addIcon() noexcept;
    void removeIcon() noexcept;
    void showMenu() noexcept;
    void launch(const wchar_t* arguments) noexcept;

    HINSTANCE instance_{};
    HWND window_{};
    NOTIFYICONDATAW icon_{};
    UINT taskbarCreated_{};
    std::filesystem::path configPath_;
    LauncherState launcherState_{LauncherState::normal};
    EngineState engineState_{EngineState::stopped};
    protocol::EngineStatusResponse inputMethodStatus_;
    TrayCommand pendingCommand_{TrayCommand::none};
    bool iconAdded_{};
    bool usesGuidIdentity_{true};
};

} // namespace fcitx::windows::launcher
