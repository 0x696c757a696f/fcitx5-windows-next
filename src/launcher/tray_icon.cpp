#include "tray_icon.h"

#include "fcitx5_windows/release_identity.h"
#include "launcher_rust_abi.h"
#include "resource.h"

#include <shellapi.h>

#include <array>
#include <algorithm>
#include <cwchar>
#include <iterator>
#include <string>
#include <vector>

namespace fcitx::windows::launcher {
namespace {

constexpr wchar_t kWindowClass[] = L"Fcitx5WindowsNext.LauncherTray";
constexpr UINT kTrayMessage = WM_APP + 42U;
constexpr UINT_PTR kRetryTimer = 1U;
constexpr UINT kStatus = 100U;
constexpr UINT kRestart = 101U;
constexpr UINT kToggle = 102U;
constexpr UINT kSettings = 103U;
constexpr UINT kDiagnostics = 104U;
constexpr UINT kExit = 105U;

bool chineseUi() noexcept {
    return PRIMARYLANGID(GetUserDefaultUILanguage()) == LANG_CHINESE;
}

HICON statusIcon(HINSTANCE instance, LauncherState launcherState,
                 EngineState engineState) noexcept {
    int resource = IDI_FCITX5_APP;
    if (launcherState == LauncherState::safeMode ||
        launcherState == LauncherState::userStopped)
        resource = IDI_FCITX5_PAUSED;
    else if (launcherState == LauncherState::crashBackoff ||
             launcherState == LauncherState::uninstalling ||
             (launcherState == LauncherState::normal && engineState == EngineState::stopped))
        resource = IDI_FCITX5_ERROR;
    return LoadIconW(instance, MAKEINTRESOURCEW(resource));
}

void copyText(wchar_t* destination, std::size_t capacity, const std::wstring& value) noexcept {
    if (capacity == 0)
        return;
    wcsncpy_s(destination, capacity, value.c_str(), _TRUNCATE);
}

template <typename Producer>
std::wstring callRustText(Producer producer) {
    const std::size_t required = producer(nullptr, 0);
    if (required == 0)
        return {};
    std::vector<wchar_t> buffer(required);
    const std::size_t written =
        producer(reinterpret_cast<std::uint16_t*>(buffer.data()), buffer.size());
    if (written == 0 || written > buffer.size())
        return {};
    return std::wstring(buffer.data(), buffer.data() + written);
}

std::wstring statusText(LauncherState launcherState, EngineState engineState, bool chinese) {
    return callRustText([&](std::uint16_t* output, std::size_t capacity) {
        return fcitx5_launcher_tray_status_text_utf16(
            static_cast<std::uint32_t>(launcherState), static_cast<std::uint32_t>(engineState),
            chinese ? 1 : 0, output, capacity);
    });
}

std::wstring inputMethodDisplay(const protocol::EngineStatusResponse& status) {
    return callRustText([&](std::uint16_t* output, std::size_t capacity) {
        return fcitx5_launcher_tray_input_method_display_utf16(
            reinterpret_cast<const std::uint8_t*>(status.currentInputMethodNativeName.data()),
            status.currentInputMethodNativeName.size(),
            reinterpret_cast<const std::uint8_t*>(status.currentInputMethodName.data()),
            status.currentInputMethodName.size(),
            reinterpret_cast<const std::uint8_t*>(status.currentInputMethodId.data()),
            status.currentInputMethodId.size(), output, capacity);
    });
}

std::wstring tooltipText(LauncherState launcherState, EngineState engineState,
                         const protocol::EngineStatusResponse& inputMethodStatus) {
    const std::wstring productName = fcitx::windows::kReleaseIdentity.service_description;
    const bool chinese = chineseUi();
    return callRustText([&](std::uint16_t* output, std::size_t capacity) {
        return fcitx5_launcher_tray_tooltip_utf16(
            reinterpret_cast<const std::uint16_t*>(productName.data()), productName.size(),
            static_cast<std::uint32_t>(launcherState), static_cast<std::uint32_t>(engineState),
            chinese ? 1 : 0,
            reinterpret_cast<const std::uint8_t*>(
                inputMethodStatus.currentInputMethodNativeName.data()),
            inputMethodStatus.currentInputMethodNativeName.size(),
            reinterpret_cast<const std::uint8_t*>(inputMethodStatus.currentInputMethodName.data()),
            inputMethodStatus.currentInputMethodName.size(),
            reinterpret_cast<const std::uint8_t*>(inputMethodStatus.currentInputMethodId.data()),
            inputMethodStatus.currentInputMethodId.size(), output, capacity);
    });
}

std::wstring joinExecutable(std::wstring_view directory, const wchar_t* executable) {
    std::wstring result(directory);
    if (!result.empty() && result.back() != L'\\' && result.back() != L'/')
        result.push_back(L'\\');
    result += executable;
    return result;
}

} // namespace

TrayIcon::~TrayIcon() {
    removeIcon();
    if (window_)
        DestroyWindow(window_);
}

bool TrayIcon::create(HINSTANCE instance, std::wstring_view executableDirectory) {
    instance_ = instance;
    configDirectory_ = std::wstring(executableDirectory);
    configPath_ = joinExecutable(configDirectory_, L"fcitx5-config.exe");
    taskbarCreated_ = RegisterWindowMessageW(L"TaskbarCreated");
    WNDCLASSEXW windowClass{sizeof(windowClass)};
    windowClass.lpfnWndProc = windowProcedure;
    windowClass.hInstance = instance_;
    windowClass.lpszClassName = kWindowClass;
    const ATOM registered = RegisterClassExW(&windowClass);
    if (!registered && GetLastError() != ERROR_CLASS_ALREADY_EXISTS)
        return false;
    window_ = CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, kWindowClass,
                              fcitx::windows::kReleaseIdentity.service_description, WS_POPUP, 0,
                              0, 0, 0, nullptr, nullptr, instance_, this);
    if (!window_)
        return false;
    icon_.cbSize = sizeof(icon_);
    icon_.hWnd = window_;
    icon_.uID = 1;
    icon_.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP | NIF_GUID;
    icon_.uCallbackMessage = kTrayMessage;
    icon_.guidItem = fcitx::windows::kReleaseIdentity.notification_icon_guid;
    // GUID identity can survive briefly in Explorer after a crashed or just-exited process.
    // Remove that stale record before the new owner adds the channel's icon.
    (void)Shell_NotifyIconW(NIM_DELETE, &icon_);
    addIcon();
    return true;
}

void TrayIcon::addIcon() noexcept {
    if (!window_)
        return;
    icon_.hIcon = statusIcon(instance_, launcherState_, engineState_);
    if (!icon_.hIcon)
        icon_.hIcon = LoadIconW(nullptr, IDI_APPLICATION);
    copyText(icon_.szTip, std::size(icon_.szTip),
             tooltipText(launcherState_, engineState_, inputMethodStatus_));
    icon_.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP |
                   (usesGuidIdentity_ ? NIF_GUID : 0U);
    iconAdded_ = Shell_NotifyIconW(NIM_ADD, &icon_) != FALSE;
    // GUID identity is preferred and keeps release channels stable across restarts. Some
    // Explorer builds/policies reject a first-time GUID notification icon, while the documented
    // hWnd/uID identity remains available. Fall back within the same attempt so the user never
    // loses service status or recovery controls merely because Explorer rejected the GUID form.
    if (!iconAdded_ && usesGuidIdentity_) {
        usesGuidIdentity_ = false;
        icon_.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
        iconAdded_ = Shell_NotifyIconW(NIM_ADD, &icon_) != FALSE;
    }
    if (iconAdded_) {
        icon_.uVersion = NOTIFYICON_VERSION_4;
        (void)Shell_NotifyIconW(NIM_SETVERSION, &icon_);
        // Some Explorer versions acknowledge a first-time GUID icon but never expose it through
        // the notification area. Give Explorer one message-loop interval, then fall back to the
        // documented hWnd/uID identity if the GUID still has no shell rectangle.
        if (usesGuidIdentity_)
            (void)SetTimer(window_, kRetryTimer, 500U, nullptr);
        else
            KillTimer(window_, kRetryTimer);
    } else
        (void)SetTimer(window_, kRetryTimer, 1000U, nullptr);
}

void TrayIcon::removeIcon() noexcept {
    if (window_)
        KillTimer(window_, kRetryTimer);
    if (iconAdded_)
        (void)Shell_NotifyIconW(NIM_DELETE, &icon_);
    iconAdded_ = false;
}

void TrayIcon::update(LauncherState launcherState, EngineState engineState,
                      const protocol::EngineStatusResponse& inputMethodStatus) {
    if (!window_)
        return;
    if (launcherState_ == launcherState && engineState_ == engineState &&
        inputMethodStatus_.currentInputMethodId == inputMethodStatus.currentInputMethodId &&
        inputMethodStatus_.currentInputMethodName == inputMethodStatus.currentInputMethodName &&
        inputMethodStatus_.currentInputMethodNativeName ==
            inputMethodStatus.currentInputMethodNativeName &&
        inputMethodStatus_.currentInputMethodShortLabel ==
            inputMethodStatus.currentInputMethodShortLabel)
        return;
    launcherState_ = launcherState;
    engineState_ = engineState;
    inputMethodStatus_ = inputMethodStatus;
    if (!iconAdded_) {
        addIcon();
        return;
    }
    icon_.uFlags = NIF_ICON | NIF_TIP | NIF_SHOWTIP |
                   (usesGuidIdentity_ ? NIF_GUID : 0U);
    icon_.hIcon = statusIcon(instance_, launcherState_, engineState_);
    copyText(icon_.szTip, std::size(icon_.szTip),
             tooltipText(launcherState_, engineState_, inputMethodStatus_));
    (void)Shell_NotifyIconW(NIM_MODIFY, &icon_);
    icon_.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP |
                   (usesGuidIdentity_ ? NIF_GUID : 0U);
}

bool TrayIcon::shellVisible() const noexcept {
    if (!iconAdded_ || !window_)
        return false;
    NOTIFYICONIDENTIFIER identifier{sizeof(identifier)};
    if (usesGuidIdentity_)
        identifier.guidItem = icon_.guidItem;
    else {
        identifier.hWnd = window_;
        identifier.uID = icon_.uID;
    }
    RECT rectangle{};
    return Shell_NotifyIconGetRect(&identifier, &rectangle) == S_OK;
}

void TrayIcon::dispatchMessages() {
    MSG message{};
    while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}

TrayCommand TrayIcon::takeCommand() noexcept {
    const TrayCommand result = pendingCommand_;
    pendingCommand_ = TrayCommand::none;
    return result;
}

void TrayIcon::launch(const wchar_t* arguments) noexcept {
    if (configPath_.empty() ||
        GetFileAttributesW(configPath_.c_str()) == INVALID_FILE_ATTRIBUTES)
        return;
    std::wstring command = L"\"" + configPath_ + L"\"";
    if (arguments && *arguments) {
        command.push_back(L' ');
        command.append(arguments);
    }
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{sizeof(startup)};
    PROCESS_INFORMATION process{};
    if (CreateProcessW(configPath_.c_str(), mutableCommand.data(), nullptr, nullptr, FALSE,
                       CREATE_UNICODE_ENVIRONMENT, nullptr, configDirectory_.c_str(),
                       &startup, &process)) {
        CloseHandle(process.hThread);
        CloseHandle(process.hProcess);
    }
}

void TrayIcon::showMenu() noexcept {
    HMENU menu = CreatePopupMenu();
    if (!menu)
        return;
    const bool chinese = chineseUi();
    const std::wstring status = std::wstring(chinese ? L"状态：" : L"Status: ") +
                                statusText(launcherState_, engineState_, chinese);
    AppendMenuW(menu, MF_STRING | MF_DISABLED, kStatus, status.c_str());
    const std::wstring inputMethod = inputMethodDisplay(inputMethodStatus_);
    if (!inputMethod.empty()) {
        const std::wstring method = std::wstring(chinese ? L"当前方案：" : L"Input method: ") +
                                    inputMethod;
        AppendMenuW(menu, MF_STRING | MF_DISABLED, kStatus + 1U, method.c_str());
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
    AppendMenuW(menu, MF_STRING, kRestart,
                chinese ? L"重新启动输入服务" : L"Restart input service");
    const bool paused = launcherState_ == LauncherState::userStopped;
    AppendMenuW(menu, MF_STRING, kToggle,
                paused ? (chinese ? L"恢复输入服务" : L"Resume input service")
                       : (chinese ? L"暂停输入服务" : L"Pause input service"));
    AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
    AppendMenuW(menu, MF_STRING, kSettings, chinese ? L"设置" : L"Settings");
    AppendMenuW(menu, MF_STRING, kDiagnostics,
                chinese ? L"诊断与修复" : L"Diagnostics and repair");
    AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
    AppendMenuW(menu, MF_STRING, kExit,
                chinese ? L"退出 Fcitx5" : L"Exit Fcitx5");
    POINT point{};
    GetCursorPos(&point);
    SetForegroundWindow(window_);
    const UINT selected = TrackPopupMenuEx(
        menu, TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY, point.x, point.y,
        window_, nullptr);
    DestroyMenu(menu);
    switch (selected) {
    case kRestart:
        pendingCommand_ = TrayCommand::restart;
        break;
    case kToggle:
        pendingCommand_ = paused ? TrayCommand::resume : TrayCommand::pause;
        break;
    case kSettings:
        launch(L"");
        break;
    case kDiagnostics:
        launch(L"--diagnostics");
        break;
    case kExit:
        pendingCommand_ = TrayCommand::exit;
        break;
    default:
        break;
    }
    PostMessageW(window_, WM_NULL, 0, 0);
}

LRESULT CALLBACK TrayIcon::windowProcedure(HWND window, UINT message, WPARAM wparam,
                                           LPARAM lparam) noexcept {
    TrayIcon* self = reinterpret_cast<TrayIcon*>(GetWindowLongPtrW(window, GWLP_USERDATA));
    if (message == WM_NCCREATE) {
        const auto* create = reinterpret_cast<const CREATESTRUCTW*>(lparam);
        self = static_cast<TrayIcon*>(create->lpCreateParams);
        SetWindowLongPtrW(window, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(self));
    }
    return self ? self->handleMessage(window, message, wparam, lparam)
                : DefWindowProcW(window, message, wparam, lparam);
}

LRESULT TrayIcon::handleMessage(HWND window, UINT message, WPARAM wparam,
                                LPARAM lparam) noexcept {
    if (taskbarCreated_ != 0 && message == taskbarCreated_) {
        iconAdded_ = false;
        addIcon();
        return 0;
    }
    if (message == WM_TIMER && wparam == kRetryTimer) {
        if (!iconAdded_) {
            addIcon();
        } else if (usesGuidIdentity_ && !shellVisible()) {
            (void)Shell_NotifyIconW(NIM_DELETE, &icon_);
            iconAdded_ = false;
            usesGuidIdentity_ = false;
            addIcon();
        } else {
            KillTimer(window_, kRetryTimer);
        }
        return 0;
    }
    if (message == kTrayMessage) {
        const UINT event = LOWORD(lparam);
        if (event == WM_CONTEXTMENU || event == WM_RBUTTONUP) {
            showMenu();
            return 0;
        }
        if (event == NIN_SELECT || event == NIN_KEYSELECT || event == WM_LBUTTONDBLCLK) {
            launch(L"");
            return 0;
        }
    }
    return DefWindowProcW(window, message, wparam, lparam);
}

} // namespace fcitx::windows::launcher
