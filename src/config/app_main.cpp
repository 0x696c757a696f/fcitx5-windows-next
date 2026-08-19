#include "fcitx5_windows/version.h"
#include "process_execution.h"

#include <Windows.h>
#include <CommCtrl.h>
#include <shellapi.h>
#include <d2d1.h>

#include <atlbase.h>
#include <atlapp.h>
extern CAppModule _Module;
#include <atlctrls.h>
#include <atlframe.h>
#include <atlwin.h>

#include <filesystem>
#include <fstream>
#include <array>
#include <string>
#include <string_view>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <vector>

#include <nlohmann/json.hpp>

namespace {

namespace fs = std::filesystem;
using Strings = std::unordered_map<std::string, std::wstring>;

void enableDpiAwareness() noexcept {
    const HMODULE user32 = GetModuleHandleW(L"user32.dll");
    using SetContext = BOOL(WINAPI*)(HANDLE);
    const auto setContext = user32 ? reinterpret_cast<SetContext>(
                                        GetProcAddress(user32, "SetProcessDpiAwarenessContext"))
                                   : nullptr;
    if (!setContext || !setContext(reinterpret_cast<HANDLE>(-4)))
        (void)SetProcessDPIAware();
}

void enableNativeWindowEffects(HWND window) noexcept {
    const HMODULE dwm = LoadLibraryW(L"dwmapi.dll");
    if (!dwm)
        return;
    using SetAttribute = HRESULT(WINAPI*)(HWND, DWORD, const void*, DWORD);
    const auto setAttribute =
        reinterpret_cast<SetAttribute>(GetProcAddress(dwm, "DwmSetWindowAttribute"));
    if (setAttribute) {
        constexpr DWORD kCornerPreference = 33;
        constexpr DWORD kCaptionColor = 35;
        constexpr DWORD kTextColor = 36;
        constexpr DWORD kRound = 2;
        constexpr COLORREF kWhite = RGB(255, 255, 255);
        constexpr COLORREF kDarkText = RGB(32, 34, 37);
        (void)setAttribute(window, kCornerPreference, &kRound, sizeof(kRound));
        (void)setAttribute(window, kCaptionColor, &kWhite, sizeof(kWhite));
        (void)setAttribute(window, kTextColor, &kDarkText, sizeof(kDarkText));
    }
    FreeLibrary(dwm);
}

std::wstring widen(std::string_view value) {
    if (value.empty())
        return {};
    const int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
                                          static_cast<int>(value.size()), nullptr, 0);
    if (count <= 0)
        return {};
    std::wstring result(static_cast<std::size_t>(count), L'\0');
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
                               static_cast<int>(value.size()), result.data(), count) == count
               ? result
               : std::wstring{};
}

std::string narrow(std::wstring_view value) {
    if (value.empty())
        return {};
    const int count =
        WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                            static_cast<int>(value.size()), nullptr, 0, nullptr, nullptr);
    if (count <= 0)
        return {};
    std::string result(static_cast<std::size_t>(count), '\0');
    return WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                               static_cast<int>(value.size()), result.data(), count, nullptr,
                               nullptr) == count
               ? result
               : std::string{};
}

bool parseString(std::string_view text, std::size_t& offset, std::string& value) {
    if (offset >= text.size() || text[offset++] != '"')
        return false;
    value.clear();
    while (offset < text.size()) {
        const unsigned char character = text[offset++];
        if (character == '"')
            return true;
        if (character < 0x20U)
            return false;
        if (character != '\\') {
            value.push_back(static_cast<char>(character));
            continue;
        }
        if (offset >= text.size())
            return false;
        switch (text[offset++]) {
        case '"':
            value.push_back('"');
            break;
        case '\\':
            value.push_back('\\');
            break;
        case '/':
            value.push_back('/');
            break;
        case 'b':
            value.push_back('\b');
            break;
        case 'f':
            value.push_back('\f');
            break;
        case 'n':
            value.push_back('\n');
            break;
        case 'r':
            value.push_back('\r');
            break;
        case 't':
            value.push_back('\t');
            break;
        default:
            return false;
        }
    }
    return false;
}

bool parseFlatJson(std::string_view text, Strings& strings) {
    if (text.size() > 2U * 1024U * 1024U)
        return false;
    strings.clear();
    std::size_t offset = 0;
    const auto whitespace = [&] {
        while (offset < text.size() && (text[offset] == ' ' || text[offset] == '\t' ||
                                        text[offset] == '\r' || text[offset] == '\n'))
            ++offset;
    };
    whitespace();
    if (offset >= text.size() || text[offset++] != '{')
        return false;
    bool sawVersion = false;
    for (;;) {
        whitespace();
        if (offset < text.size() && text[offset] == '}') {
            ++offset;
            whitespace();
            return offset == text.size() && sawVersion;
        }
        std::string key;
        if (!parseString(text, offset, key))
            return false;
        whitespace();
        if (offset >= text.size() || text[offset++] != ':')
            return false;
        whitespace();
        if (key == "format_version") {
            if (sawVersion || offset >= text.size() || text[offset++] != '1')
                return false;
            sawVersion = true;
        } else {
            std::string value;
            if (offset + 4U <= text.size() && text.substr(offset, 4U) == "true") {
                value = "true";
                offset += 4U;
            } else if (offset + 5U <= text.size() && text.substr(offset, 5U) == "false") {
                value = "false";
                offset += 5U;
            } else if (!parseString(text, offset, value))
                return false;
            if (strings.contains(key))
                return false;
            std::wstring wide = widen(value);
            if (!value.empty() && wide.empty())
                return false;
            strings.emplace(std::move(key), std::move(wide));
        }
        whitespace();
        if (offset >= text.size())
            return false;
        if (text[offset] == '}')
            continue;
        if (text[offset++] != ',')
            return false;
    }
}

bool loadLocale(const fs::path& path, Strings& strings) {
    std::ifstream stream(path, std::ios::binary);
    if (!stream)
        return false;
    const std::string text(std::istreambuf_iterator<char>(stream), {});
    return parseFlatJson(text, strings);
}

fs::path executableDirectory() {
    std::wstring path(32768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    if (size == 0 || size >= path.size())
        return {};
    path.resize(size);
    return fs::path(path).parent_path();
}

std::wstring quote(std::wstring_view value) {
    std::wstring result = L"\"";
    unsigned backslashes = 0;
    for (const wchar_t character : value) {
        if (character == L'\\') {
            ++backslashes;
        } else {
            if (character == L'"')
                result.append(backslashes + 1, L'\\');
            backslashes = 0;
            result.push_back(character);
        }
    }
    result.append(backslashes, L'\\');
    result.push_back(L'"');
    return result;
}

bool runControl(const std::vector<std::wstring>& arguments, std::wstring& output) {
    return fcitx::windows::config::runExecutable(
        executableDirectory() / L"fcitx5-control.exe", arguments, output);
}

bool checkI18n() {
    Strings english;
    Strings chinese;
    const fs::path localeDirectory = executableDirectory() / L"locales";
    if (!loadLocale(localeDirectory / L"en-US.json", english) ||
        !loadLocale(localeDirectory / L"zh-CN.json", chinese) || english.size() != chinese.size())
        return false;
    for (const auto& [key, value] : english) {
        (void)value;
        if (!chinese.contains(key))
            return false;
    }
    return true;
}

bool checkResources() {
    std::wstring output;
    return fcitx::windows::config::runExecutable(
        executableDirectory() / L"fcitx5-ui.exe", {L"--self-test", L"--safe-mode"}, output);
}

constexpr int kStartup = 100;
constexpr int kAppearance = 101;
constexpr int kTheme = 102;
constexpr int kFont = 103;
constexpr int kVertical = 104;
constexpr int kHorizontal = 105;
constexpr int kApply = 106;
constexpr int kRestart = 107;
constexpr int kDiagnostics = 108;
constexpr int kRepair = 109;
constexpr int kStatus = 110;
constexpr int kInputMethod = 111;
constexpr int kPreview = 112;
constexpr int kPackages = 113;
constexpr int kPackageRefresh = 114;
constexpr int kPackageInstall = 115;
constexpr int kPackageRemove = 116;
constexpr int kPackageToggle = 117;
constexpr int kScrollMode = 118;
constexpr int kPageSize = 119;
constexpr int kNavGeneral = 130;
constexpr int kNavAppearance = 131;
constexpr int kNavTheme = 132;
constexpr int kNavDiagnostics = 133;
constexpr int kNavRepair = 134;
constexpr int kNavPackages = 135;
constexpr int kPageTitle = 140;
constexpr int kInputMethodLabel = 200;
constexpr int kAppearanceLabel = 201;
constexpr int kThemeLabel = 202;
constexpr int kFontLabel = 203;
constexpr int kLayoutLabel = 204;
constexpr int kPackagesTitle = 205;
constexpr int kSaveStatus = 206;
constexpr int kPageSizeLabel = 207;

// Transient notices ("保存成功" / 命令错误 / 重启完成 / 修复已开始) are
// cleared automatically a few seconds after they appear.
constexpr UINT_PTR kStatusTimerId = 0x4A44U;
constexpr UINT kStatusTimeoutMs = 3000;

struct PackageRow {
    std::wstring id;
    std::wstring title;
    std::wstring summary;
    std::wstring type;
    std::wstring available;
    std::wstring installed;
    std::wstring state;
    bool update{};
};

struct InputMethodRow {
    std::wstring id;
    std::wstring name;
    std::wstring nativeName;
    bool selected{};
};

bool parseInputMethods(std::wstring_view output, std::vector<InputMethodRow>& rows) {
    try {
        const auto document = nlohmann::json::parse(narrow(output));
        if (!document.is_object() || document.size() != 2U ||
            document.at("format_version") != 1 || !document.at("input_methods").is_array() ||
            document.at("input_methods").size() > 128U)
            return false;
        rows.clear();
        unsigned selectedCount = 0;
        for (const auto& item : document.at("input_methods")) {
            if (!item.is_object() || item.size() != 4U || !item.at("id").is_string() ||
                !item.at("name").is_string() || !item.at("native_name").is_string() ||
                !item.at("selected").is_boolean())
                return false;
            InputMethodRow row{widen(item.at("id").get<std::string>()),
                               widen(item.at("name").get<std::string>()),
                               widen(item.at("native_name").get<std::string>()),
                               item.at("selected").get<bool>()};
            if (row.id.empty() || row.name.empty())
                return false;
            selectedCount += row.selected ? 1U : 0U;
            rows.push_back(std::move(row));
        }
        return !rows.empty() && selectedCount == 1U;
    } catch (const nlohmann::json::exception&) {
        return false;
    }
}

bool parsePackages(std::wstring_view output, std::vector<PackageRow>& rows,
                   bool& repositoryAvailable) {
    try {
        const auto document = nlohmann::json::parse(narrow(output));
        if (!document.is_object() || document.size() != 3U || document.at("format_version") != 1 ||
            !document.at("repository_available").is_boolean() ||
            !document.at("packages").is_array() || document.at("packages").size() > 4096U)
            return false;
        repositoryAvailable = document.at("repository_available").get<bool>();
        rows.clear();
        for (const auto& item : document.at("packages")) {
            if (!item.is_object() || item.size() != 8U)
                return false;
            PackageRow row;
            row.id = widen(item.at("id").get<std::string>());
            row.title = widen(item.at("title").get<std::string>());
            row.summary = widen(item.at("summary").get<std::string>());
            row.type = widen(item.at("type").get<std::string>());
            if (!item.at("available_version").is_null())
                row.available = widen(item.at("available_version").get<std::string>());
            if (!item.at("installed_version").is_null())
                row.installed = widen(item.at("installed_version").get<std::string>());
            if (!item.at("state").is_null())
                row.state = widen(item.at("state").get<std::string>());
            row.update = item.at("update_available").get<bool>();
            if (row.id.empty() || row.title.empty() || row.type.empty())
                return false;
            rows.push_back(std::move(row));
        }
        return true;
    } catch (const nlohmann::json::exception&) {
        return false;
    }
}

std::wstring localeValue(const Strings& strings, const char* key,
                         std::wstring_view fallback = {}) {
    const auto iterator = strings.find(key);
    if (iterator != strings.end())
        return iterator->second;
    return std::wstring(fallback);
}

std::wstring packageTypeLabel(const PackageRow& package, const Strings& strings) {
    if (package.type == L"addon")
        return localeValue(strings, "packages.type.addon", L"Addon");
    if (package.type == L"inputmethod-data")
        return localeValue(strings, "packages.type.input_method_data", L"Input data");
    if (package.type == L"theme")
        return localeValue(strings, "packages.type.theme", L"Theme");
    if (package.type == L"translation")
        return localeValue(strings, "packages.type.translation", L"Translation");
    if (package.type == L"core")
        return localeValue(strings, "packages.type.core", L"Core");
    return localeValue(strings, "packages.type.component", L"Component");
}

std::wstring packageListLabel(const PackageRow& package, const Strings& strings) {
    std::wstring label = L"[" + packageTypeLabel(package, strings) + L"] " + package.title;
    if (!package.summary.empty())
        label += L" — " + package.summary;
    const std::wstring version = !package.installed.empty() ? package.installed : package.available;
    if (!version.empty())
        label += L"  " + version;
    if (package.update)
        label += L"  ↑";
    if (package.state == L"disabled")
        label += L"  " + localeValue(strings, "packages.disabled", L"(disabled)");
    if (package.state == L"bundled")
        label += L"  " + localeValue(strings, "packages.bundled", L"(bundled)");
    return label;
}

class ConfigWindow final : public CWindowImpl<ConfigWindow> {
  public:
    DECLARE_WND_CLASS_EX(L"Fcitx5ConfigWindow", CS_HREDRAW | CS_VREDRAW, COLOR_WINDOW)

    explicit ConfigWindow(Strings strings) : strings_(std::move(strings)) {}
    const wchar_t* title() const { return get("app.title"); }
    void selectPage(int page) { showPage(page); }
    void resizeToDefaultClient() {
        RECT windowRect{};
        GetWindowRect(&windowRect);
        const HMONITOR monitor = MonitorFromRect(&windowRect, MONITOR_DEFAULTTONEAREST);
        MONITORINFO monitorInfo{};
        monitorInfo.cbSize = sizeof(monitorInfo);
        GetMonitorInfoW(monitor, &monitorInfo);
        const int workWidth = monitorInfo.rcWork.right - monitorInfo.rcWork.left;
        const int workHeight = monitorInfo.rcWork.bottom - monitorInfo.rcWork.top;
        const int desiredWidth = (std::min)(scale(1010), (std::max)(scale(820), workWidth - 80));
        const int desiredHeight = (std::min)(scale(650), (std::max)(scale(520), workHeight - 80));
        ResizeClient(desiredWidth, desiredHeight);
        layoutControls();
    }

    [[nodiscard]] bool verifyUiContract() {
        const auto hasVisibleStyle = [&](int id) {
            const HWND child = control(id);
            return child &&
                   (::GetWindowLongPtrW(child, GWL_STYLE) &
                    static_cast<LONG_PTR>(WS_VISIBLE)) != 0;
        };
        const auto pageMatches = [&](int page, bool apply, bool details) {
            showPage(page);
            return hasVisibleStyle(kApply) == apply &&
                   hasVisibleStyle(kSaveStatus) == apply &&
                   hasVisibleStyle(kStatus) == details;
        };
        if (!pageMatches(kNavGeneral, true, false) ||
            !pageMatches(kNavAppearance, true, false) ||
            !pageMatches(kNavTheme, true, false) ||
            !pageMatches(kNavDiagnostics, false, true) ||
            !pageMatches(kNavRepair, false, true) ||
            !pageMatches(kNavPackages, false, true)) {
            return false;
        }
        const fs::path directory = executableDirectory();
        const fs::path root = directory.filename() == L"bin" ? directory.parent_path() : directory;
        if (fs::is_regular_file(root / L"lib/fcitx5/librime.dll")) {
            showPage(kNavPackages);
            refreshPackages(false);
            if (SendMessageW(control(kPackages), LB_GETCOUNT, 0, 0) < 5 ||
                ::IsWindowEnabled(control(kPackageInstall)) ||
                ::IsWindowEnabled(control(kPackageToggle)) ||
                ::IsWindowEnabled(control(kPackageRemove))) {
                return false;
            }
        }
        showPage(kNavAppearance);
        BOOL handled = FALSE;
        (void)onDirty(0, kHorizontal, control(kHorizontal), handled);
        std::array<wchar_t, 128> status{};
        ::GetWindowTextW(control(kSaveStatus), status.data(), static_cast<int>(status.size()));
        return status[0] != L'\0' && std::wstring_view(status.data()) == get("status.unsaved");
    }

    [[nodiscard]] bool verifyInteractionCoverage() {
        interactionTest_ = true;
        actionCoverage_ = 0;
        std::unordered_set<int> clickedButtons;
        const auto click = [&](int id) {
            const HWND child = control(id);
            if (!child ||
                (::GetWindowLongPtrW(child, GWL_STYLE) &
                 static_cast<LONG_PTR>(WS_VISIBLE)) == 0 ||
                !::IsWindowEnabled(child))
                return false;
            if (SendMessageW(child, BM_CLICK, 0, 0) != 0)
                return false;
            clickedButtons.insert(id);
            return true;
        };
        const auto notify = [&](int id, int notification) {
            const HWND child = control(id);
            if (!child ||
                (::GetWindowLongPtrW(child, GWL_STYLE) &
                 static_cast<LONG_PTR>(WS_VISIBLE)) == 0 ||
                !::IsWindowEnabled(child))
                return false;
            SendMessageW(WM_COMMAND, MAKEWPARAM(id, notification),
                         reinterpret_cast<LPARAM>(child));
            return true;
        };
        const auto statusIs = [&](const char* key) {
            std::array<wchar_t, 128> status{};
            ::GetWindowTextW(control(kSaveStatus), status.data(),
                             static_cast<int>(status.size()));
            return std::wstring_view(status.data()) == get(key);
        };

        // Navigate through every page using the actual owner-drawn buttons.
        for (const int page : {kNavGeneral, kNavAppearance, kNavTheme, kNavDiagnostics,
                               kNavRepair, kNavPackages}) {
            if (!click(page) || selectedPage_ != page)
                return false;
        }

        if (!click(kNavGeneral) || !click(kStartup) || !statusIs("status.unsaved") ||
            !click(kStartup) ||
            !([&] {
                if (!::IsWindowEnabled(control(kInputMethod))) {
                    inputMethods_ = {{L"test", L"Test", L"Test", true}};
                    SendMessageW(control(kInputMethod), CB_RESETCONTENT, 0, 0);
                    SendMessageW(control(kInputMethod), CB_ADDSTRING, 0,
                                 reinterpret_cast<LPARAM>(L"Test"));
                    SendMessageW(control(kInputMethod), CB_SETCURSEL, 0, 0);
                    ::EnableWindow(control(kInputMethod), TRUE);
                }
                const LRESULT count = SendMessageW(control(kInputMethod), CB_GETCOUNT, 0, 0);
                if (count <= 0)
                    return false;
                for (LRESULT index = 0; index < count; ++index) {
                    SendMessageW(control(kInputMethod), CB_SETCURSEL, index, 0);
                    if (!notify(kInputMethod, CBN_SELCHANGE))
                        return false;
                }
                return true;
            }()) || !click(kApply) ||
            !statusIs("status.saved"))
            return false;

        if (!click(kNavAppearance))
            return false;
        const LRESULT appearanceCount = SendMessageW(control(kAppearance), CB_GETCOUNT, 0, 0);
        for (LRESULT index = 0; index < appearanceCount; ++index) {
            SendMessageW(control(kAppearance), CB_SETCURSEL, index, 0);
            if (!notify(kAppearance, CBN_SELCHANGE))
                return false;
        }
        if (appearanceCount != 3 || !click(kVertical) || !click(kHorizontal))
            return false;
        if (!click(kVertical))
            return false;
        if (!click(kScrollMode))
            return false;
        if (!click(kScrollMode))
            return false;
        if (!statusIs("status.unsaved") || !click(kApply) || !statusIs("status.saved"))
            return false;

        if (!click(kNavTheme))
            return false;
        const LRESULT themeCount = SendMessageW(control(kTheme), CB_GETCOUNT, 0, 0);
        for (LRESULT index = 0; index < themeCount; ++index) {
            SendMessageW(control(kTheme), CB_SETCURSEL, index, 0);
            if (!notify(kTheme, CBN_SELCHANGE))
                return false;
        }
        for (const wchar_t* font : {L"", L"Microsoft YaHei", L"思源黑体"}) {
            ::SetWindowTextW(control(kFont), font);
            if (!notify(kFont, EN_KILLFOCUS))
                return false;
        }
        if (themeCount <= 0 || !click(kPreview) || !click(kApply) ||
            !statusIs("status.saved"))
            return false;

        if (!click(kNavDiagnostics) || !click(kRestart) || !click(kDiagnostics) ||
            !click(kNavRepair) || !click(kRepair) || !click(kNavPackages) ||
            !click(kPackageRefresh))
            return false;

        // Exercise each package action through a synthetic managed row. Production package
        // transaction semantics are covered separately with a signed fixture repository.
        packages_ = {{L"test-addon", L"Test addon", L"Test summary", L"addon",
                      L"1.1.0", L"", L"", false}};
        SendMessageW(control(kPackages), LB_RESETCONTENT, 0, 0);
        const std::wstring testPackageLabel = packageListLabel(packages_[0], strings_);
        SendMessageW(control(kPackages), LB_ADDSTRING, 0,
                     reinterpret_cast<LPARAM>(testPackageLabel.c_str()));
        SendMessageW(control(kPackages), LB_SETCURSEL, 0, 0);
        std::array<wchar_t, 128> packageText{};
        SendMessageW(control(kPackages), LB_GETTEXT, 0,
                     reinterpret_cast<LPARAM>(packageText.data()));
        if (std::wstring_view(packageText.data()).find(
                L"[" + localeValue(strings_, "packages.type.addon", L"Addon") +
                L"] Test addon") ==
            std::wstring_view::npos)
            return false;
        packages_[0].type = L"inputmethod-data";
        if (packageListLabel(packages_[0], strings_)
                .find(localeValue(strings_, "packages.type.input_method_data", L"Input data")) ==
            std::wstring::npos)
            return false;
        packages_[0].type = L"addon";
        updatePackageActions();
        if (!notify(kPackages, LBN_SELCHANGE) || !click(kPackageInstall))
            return false;
        packages_[0].installed = L"1.0.0";
        packages_[0].update = true;
        packages_[0].state = L"enabled";
        updatePackageActions();
        if (!click(kPackageInstall) || !click(kPackageToggle))
            return false;
        packages_[0].state = L"disabled";
        if (!click(kPackageToggle) || !click(kPackageRemove))
            return false;

        constexpr unsigned long long expected =
            kCoveredGeneralApply | kCoveredAppearanceApply | kCoveredThemeApply |
            kCoveredRestart | kCoveredDiagnostics | kCoveredRepair | kCoveredPreview |
            kCoveredPackageRefresh | kCoveredPackageInstall | kCoveredPackageUpdate |
            kCoveredPackageDisable | kCoveredPackageEnable | kCoveredPackageRemove;
        if (actionCoverage_ != expected)
            return false;

        // Fail closed when a future visible command is added without extending this sweep.
        // This inventories the real HWND tree rather than maintaining a second hand-written list.
        for (HWND child = ::GetWindow(m_hWnd, GW_CHILD); child;
             child = ::GetWindow(child, GW_HWNDNEXT)) {
            std::array<wchar_t, 16> className{};
            if (::GetClassNameW(child, className.data(), static_cast<int>(className.size())) <= 0 ||
                _wcsicmp(className.data(), L"Button") != 0)
                continue;
            const int id = ::GetDlgCtrlID(child);
            if (id != 0 && !clickedButtons.contains(id))
                return false;
        }
        return true;
    }

    BEGIN_MSG_MAP(ConfigWindow)
    MESSAGE_HANDLER(WM_CREATE, onCreate)
    MESSAGE_HANDLER(WM_SIZE, onSize)
    MESSAGE_HANDLER(WM_GETMINMAXINFO, onGetMinMaxInfo)
    MESSAGE_HANDLER(WM_DPICHANGED, onDpiChanged)
    MESSAGE_HANDLER(WM_PAINT, onPaint)
    MESSAGE_HANDLER(WM_DRAWITEM, onDrawItem)
    MESSAGE_HANDLER(WM_CTLCOLORSTATIC, onColorStatic)
    MESSAGE_HANDLER(WM_TIMER, onTimer)
    MESSAGE_HANDLER(WM_DESTROY, onDestroy)
    COMMAND_RANGE_HANDLER(kNavGeneral, kNavPackages, onNavigate)
    COMMAND_ID_HANDLER(kApply, onApply)
    COMMAND_ID_HANDLER(kRestart, onRestart)
    COMMAND_ID_HANDLER(kDiagnostics, onDiagnostics)
    COMMAND_ID_HANDLER(kRepair, onRepair)
    COMMAND_ID_HANDLER(kPreview, onPreview)
    COMMAND_ID_HANDLER(kPackageRefresh, onPackageRefresh)
    COMMAND_ID_HANDLER(kPackageInstall, onPackageInstall)
    COMMAND_ID_HANDLER(kPackageRemove, onPackageRemove)
    COMMAND_ID_HANDLER(kPackageToggle, onPackageToggle)
    COMMAND_HANDLER(kPackages, LBN_SELCHANGE, onPackageSelection)
    COMMAND_HANDLER(kInputMethod, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kStartup, BN_CLICKED, onDirty)
    COMMAND_HANDLER(kAppearance, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kTheme, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kPageSize, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kVertical, BN_CLICKED, onDirty)
    COMMAND_HANDLER(kHorizontal, BN_CLICKED, onDirty)
    COMMAND_HANDLER(kScrollMode, BN_CLICKED, onDirty)
    COMMAND_HANDLER(kFont, EN_KILLFOCUS, onDirty)
    END_MSG_MAP()

  private:
    static constexpr unsigned long long kCoveredGeneralApply = 1ULL << 0U;
    static constexpr unsigned long long kCoveredAppearanceApply = 1ULL << 1U;
    static constexpr unsigned long long kCoveredThemeApply = 1ULL << 2U;
    static constexpr unsigned long long kCoveredRestart = 1ULL << 3U;
    static constexpr unsigned long long kCoveredDiagnostics = 1ULL << 4U;
    static constexpr unsigned long long kCoveredRepair = 1ULL << 5U;
    static constexpr unsigned long long kCoveredPreview = 1ULL << 6U;
    static constexpr unsigned long long kCoveredPackageRefresh = 1ULL << 7U;
    static constexpr unsigned long long kCoveredPackageInstall = 1ULL << 8U;
    static constexpr unsigned long long kCoveredPackageUpdate = 1ULL << 9U;
    static constexpr unsigned long long kCoveredPackageDisable = 1ULL << 10U;
    static constexpr unsigned long long kCoveredPackageEnable = 1ULL << 11U;
    static constexpr unsigned long long kCoveredPackageRemove = 1ULL << 12U;

    void cover(unsigned long long action) noexcept { actionCoverage_ |= action; }
    const wchar_t* get(const char* key) const {
        const auto iterator = strings_.find(key);
        return iterator == strings_.end() ? L"" : iterator->second.c_str();
    }
    HWND control(int id) const { return GetDlgItem(id); }
    [[nodiscard]] int scale(int value) const noexcept {
        return MulDiv(value, static_cast<int>(dpi_), 96);
    }
    [[nodiscard]] int unscale(int value) const noexcept {
        return MulDiv(value, 96, static_cast<int>(dpi_ == 0 ? 96 : dpi_));
    }
    [[nodiscard]] UINT windowDpi() const noexcept {
        const HMODULE user32 = GetModuleHandleW(L"user32.dll");
        using GetDpiForWindowProc = UINT(WINAPI*)(HWND);
        const auto getDpiForWindow =
            user32 ? reinterpret_cast<GetDpiForWindowProc>(
                         GetProcAddress(user32, "GetDpiForWindow"))
                   : nullptr;
        if (getDpiForWindow) {
            const UINT dpi = getDpiForWindow(m_hWnd);
            if (dpi != 0)
                return dpi;
        }
        HDC screen = ::GetDC(nullptr);
        const UINT dpi = screen ? static_cast<UINT>(GetDeviceCaps(screen, LOGPIXELSX)) : 96U;
        if (screen)
            ::ReleaseDC(nullptr, screen);
        return dpi == 0 ? 96U : dpi;
    }
    void createFonts() {
        if (font_) {
            DeleteObject(font_);
            font_ = nullptr;
        }
        if (titleFont_) {
            DeleteObject(titleFont_);
            titleFont_ = nullptr;
        }
        NONCLIENTMETRICSW metrics{};
        metrics.cbSize = sizeof(metrics);
        const HMODULE user32 = GetModuleHandleW(L"user32.dll");
        using SystemParametersInfoForDpiProc = BOOL(WINAPI*)(UINT, UINT, PVOID, UINT, UINT);
        const auto systemParametersInfoForDpi =
            user32 ? reinterpret_cast<SystemParametersInfoForDpiProc>(
                         GetProcAddress(user32, "SystemParametersInfoForDpi"))
                   : nullptr;
        const bool dpiMetrics =
            systemParametersInfoForDpi &&
            systemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, sizeof(metrics), &metrics, 0,
                                       dpi_);
        if (!dpiMetrics &&
            !SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, sizeof(metrics), &metrics, 0)) {
            GetObjectW(GetStockObject(DEFAULT_GUI_FONT), sizeof(metrics.lfMessageFont),
                       &metrics.lfMessageFont);
        } else if (!dpiMetrics && dpi_ != 96U && metrics.lfMessageFont.lfHeight != 0) {
            metrics.lfMessageFont.lfHeight =
                MulDiv(metrics.lfMessageFont.lfHeight, static_cast<int>(dpi_), 96);
        }
        font_ = CreateFontIndirectW(&metrics.lfMessageFont);
        LOGFONTW titleMetrics = metrics.lfMessageFont;
        titleMetrics.lfHeight = -scale(24);
        titleMetrics.lfWeight = FW_SEMIBOLD;
        titleFont_ = CreateFontIndirectW(&titleMetrics);
    }
    void applyFonts() {
        for (HWND child = ::GetWindow(m_hWnd, GW_CHILD); child;
             child = ::GetWindow(child, GW_HWNDNEXT)) {
            SendMessageW(child, WM_SETFONT,
                         reinterpret_cast<WPARAM>(child == control(kPageTitle) ? titleFont_
                                                                               : font_),
                         TRUE);
        }
    }
    void moveControl(int id, int x, int y, int width, int height) const {
        if (const HWND child = control(id)) {
            ::SetWindowPos(child, nullptr, scale(x), scale(y), scale(width), scale(height),
                           SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
    void layoutControls() {
        if (!m_hWnd)
            return;
        RECT client{};
        GetClientRect(&client);
        const int logicalWidth = (std::max)(820, unscale(client.right - client.left));
        const int logicalHeight = (std::max)(520, unscale(client.bottom - client.top));
        const int contentWidth = (std::max)(430, logicalWidth - 310);
        const int tallStatusHeight = (std::max)(250, logicalHeight - 400);
        const int packageListHeight = (std::max)(220, logicalHeight - 300);
        const int packageButtonY = (std::max)(488, logicalHeight - 162);
        const int packageStatusY = (std::max)(542, logicalHeight - 108);

        moveControl(kPageTitle, 238, 26, contentWidth, 38);
        moveControl(kStartup, 250, 104, (std::min)(contentWidth, 520), 32);
        moveControl(kInputMethodLabel, 250, 158, 150, 24);
        moveControl(kInputMethod, 420, 152, (std::min)(contentWidth - 170, 380), 140);
        moveControl(kAppearanceLabel, 250, 106, 150, 24);
        moveControl(kAppearance, 420, 100, (std::min)(contentWidth - 170, 380), 150);
        moveControl(kThemeLabel, 250, 106, 150, 24);
        moveControl(kTheme, 420, 100, (std::min)(contentWidth - 170, 380), 150);
        moveControl(kFontLabel, 250, 162, 150, 24);
        moveControl(kFont, 420, 156, (std::min)(contentWidth - 170, 380), 30);
        moveControl(kLayoutLabel, 250, 162, 150, 24);
        moveControl(kVertical, 420, 156, 120, 28);
        moveControl(kHorizontal, 550, 156, 140, 28);
        moveControl(kScrollMode, 420, 204, (std::min)(contentWidth - 170, 420), 28);
        moveControl(kPageSizeLabel, 250, 236, 150, 24);
        moveControl(kPageSize, 420, 230, 120, 180);
        moveControl(kApply, 650, 264, 120, 36);
        moveControl(kPreview, 250, 264, 160, 36);
        moveControl(kSaveStatus, 420, 272, (std::min)(contentWidth - 170, 320), 24);
        moveControl(kRestart, 250, 106, 170, 36);
        moveControl(kDiagnostics, 436, 106, 170, 36);
        moveControl(kRepair, 250, 106, 170, 36);
        moveControl(kPackagesTitle, 250, 88, 300, 28);
        moveControl(kPackages, 250, 122, contentWidth, packageListHeight);
        moveControl(kPackageRefresh, 250, packageButtonY, 150, 34);
        moveControl(kPackageInstall, 414, packageButtonY, 160, 34);
        moveControl(kPackageToggle, 588, packageButtonY, 160, 34);
        moveControl(kPackageRemove, 762, packageButtonY, 150, 34);
        if (selectedPage_ == kNavPackages) {
            moveControl(kStatus, 250, packageStatusY, contentWidth, 50);
        } else {
            moveControl(kStatus, 250, 168, contentWidth, tallStatusHeight);
        }
    }
    void add(const wchar_t* type, const wchar_t* label, DWORD style, int x, int y, int width,
             int height, int id = 0) {
        HWND child = CreateWindowExW(
            0, type, label, WS_CHILD | WS_VISIBLE | style, scale(x), scale(y), scale(width),
            scale(height), m_hWnd, reinterpret_cast<HMENU>(static_cast<std::intptr_t>(id)),
            _Module.GetModuleInstance(), nullptr);
        SendMessageW(child, WM_SETFONT, reinterpret_cast<WPARAM>(font_), TRUE);
    }
    LRESULT onCreate(UINT, WPARAM, LPARAM, BOOL&) {
        dpi_ = windowDpi();
        createFonts();
        add(L"STATIC", L"Fcitx5", 0, 24, 28, 160, 34);
        add(L"BUTTON", get("nav.general"), BS_OWNERDRAW | WS_TABSTOP, 12, 82, 180, 34, kNavGeneral);
        add(L"BUTTON", get("nav.appearance"), BS_OWNERDRAW | WS_TABSTOP, 12, 122, 180, 34,
            kNavAppearance);
        add(L"BUTTON", get("nav.theme"), BS_OWNERDRAW | WS_TABSTOP, 12, 162, 180, 34, kNavTheme);
        add(L"BUTTON", get("nav.diagnostics"), BS_OWNERDRAW | WS_TABSTOP, 12, 202, 180, 34,
            kNavDiagnostics);
        add(L"BUTTON", get("nav.repair"), BS_OWNERDRAW | WS_TABSTOP, 12, 242, 180, 34, kNavRepair);
        add(L"BUTTON", get("nav.packages"), BS_OWNERDRAW | WS_TABSTOP, 12, 282, 180, 34,
            kNavPackages);
        add(L"STATIC", get("nav.general"), 0, 238, 26, 430, 38, kPageTitle);
        SendMessageW(control(kPageTitle), WM_SETFONT, reinterpret_cast<WPARAM>(titleFont_), TRUE);
        add(L"BUTTON", get("general.startup"), BS_AUTOCHECKBOX | WS_TABSTOP, 250, 104, 390, 32,
            kStartup);
        add(L"STATIC", get("general.input_method"), 0, 250, 158, 150, 24, kInputMethodLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST, 420, 152, 330, 140, kInputMethod);
        ::EnableWindow(control(kInputMethod), FALSE);
        add(L"STATIC", get("appearance.mode"), 0, 250, 106, 150, 24, kAppearanceLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 420, 100, 330, 150, kAppearance);
        for (const wchar_t* item : {get("mode.system"), get("mode.light"), get("mode.dark")})
            SendMessageW(control(kAppearance), CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(item));
        SendMessageW(control(kAppearance), CB_SETCURSEL, 0, 0);
        add(L"STATIC", get("theme.label"), 0, 250, 106, 150, 24, kThemeLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 420, 100, 330, 150, kTheme);
        SendMessageW(control(kTheme), CB_ADDSTRING, 0,
                     reinterpret_cast<LPARAM>(get("theme.default")));
        SendMessageW(control(kTheme), CB_SETCURSEL, 0, 0);
        add(L"STATIC", get("font.label"), 0, 250, 162, 150, 24, kFontLabel);
        add(L"EDIT", L"Microsoft YaHei", WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL, 420, 156, 330, 30,
            kFont);
        add(L"STATIC", get("candidate.layout"), 0, 250, 162, 150, 24, kLayoutLabel);
        add(L"BUTTON", get("candidate.vertical"), BS_AUTORADIOBUTTON | WS_GROUP | WS_TABSTOP, 420,
            156, 120, 28, kVertical);
        add(L"BUTTON", get("candidate.horizontal"), BS_AUTORADIOBUTTON | WS_TABSTOP, 550, 156, 140,
            28, kHorizontal);
        add(L"BUTTON", get("candidate.scroll"), BS_AUTOCHECKBOX | WS_TABSTOP, 420, 204, 300, 28,
            kScrollMode);
        add(L"STATIC", get("candidate.page_size"), 0, 250, 236, 150, 24, kPageSizeLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 420, 230, 120, 180, kPageSize);
        for (int value = 1; value <= 9; ++value) {
            const std::wstring text = std::to_wstring(value);
            SendMessageW(control(kPageSize), CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(text.c_str()));
        }
        SendMessageW(control(kPageSize), CB_SETCURSEL, 4, 0);
        SendMessageW(control(kVertical), BM_SETCHECK, BST_CHECKED, 0);
        add(L"BUTTON", get("action.apply"), BS_DEFPUSHBUTTON | WS_TABSTOP, 650, 264, 120, 36,
            kApply);
        add(L"BUTTON", get("action.preview"), WS_TABSTOP, 250, 264, 160, 36, kPreview);
        add(L"STATIC", L"", SS_LEFT, 420, 272, 210, 24, kSaveStatus);
        add(L"BUTTON", get("action.restart"), WS_TABSTOP, 250, 106, 170, 36, kRestart);
        add(L"BUTTON", get("action.diagnostics"), WS_TABSTOP, 436, 106, 170, 36, kDiagnostics);
        add(L"BUTTON", get("action.repair"), WS_TABSTOP, 250, 106, 170, 36, kRepair);
        add(L"EDIT", L"", WS_BORDER | ES_MULTILINE | ES_READONLY | WS_VSCROLL, 250, 168, 700, 250,
            kStatus);
        add(L"STATIC", get("packages.title"), 0, 250, 88, 300, 28, kPackagesTitle);
        add(L"LISTBOX", L"", WS_BORDER | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY, 250, 122, 700, 350,
            kPackages);
        add(L"BUTTON", get("packages.refresh"), WS_TABSTOP, 250, 488, 150, 34, kPackageRefresh);
        add(L"BUTTON", get("packages.install_update"), WS_TABSTOP, 414, 488, 160, 34,
            kPackageInstall);
        add(L"BUTTON", get("packages.enable_disable"), WS_TABSTOP, 588, 488, 160, 34,
            kPackageToggle);
        add(L"BUTTON", get("packages.uninstall"), WS_TABSTOP, 762, 488, 150, 34, kPackageRemove);
        loadState();
        layoutControls();
        showPage(kNavGeneral);
        return 0;
    }
    LRESULT onSize(UINT, WPARAM, LPARAM, BOOL&) {
        layoutControls();
        return 0;
    }
    LRESULT onGetMinMaxInfo(UINT, WPARAM, LPARAM lparam, BOOL&) {
        auto* limits = reinterpret_cast<MINMAXINFO*>(lparam);
        limits->ptMinTrackSize.x = scale(820);
        limits->ptMinTrackSize.y = scale(520);
        return 0;
    }
    LRESULT onDpiChanged(UINT, WPARAM wparam, LPARAM lparam, BOOL&) {
        dpi_ = LOWORD(wparam);
        if (dpi_ == 0)
            dpi_ = 96;
        createFonts();
        applyFonts();
        const auto* suggested = reinterpret_cast<const RECT*>(lparam);
        if (suggested) {
            ::SetWindowPos(m_hWnd, nullptr, suggested->left, suggested->top,
                           suggested->right - suggested->left,
                           suggested->bottom - suggested->top,
                           SWP_NOZORDER | SWP_NOACTIVATE);
        }
        layoutControls();
        InvalidateRect(nullptr, TRUE);
        return 0;
    }
    void showPage(int page) {
        selectedPage_ = page;
        const auto show = [&](int id, bool visible) {
            if (const HWND child = control(id))
                ::ShowWindow(child, visible ? SW_SHOW : SW_HIDE);
        };
        for (const int id : {kStartup,         kInputMethod,    kInputMethodLabel, kAppearance,
                             kAppearanceLabel, kTheme,          kThemeLabel,       kFont,
                             kFontLabel,       kVertical,       kHorizontal,       kLayoutLabel,
                             kScrollMode,      kPageSize,       kPageSizeLabel,    kApply,
                             kPreview,         kRestart,
                             kDiagnostics,     kRepair,         kStatus,           kPackages,
                             kPackageRefresh,  kPackageInstall, kPackageToggle,    kPackageRemove,
                             kPackagesTitle,   kSaveStatus})
            show(id, false);
        const bool general = page == kNavGeneral;
        const bool appearance = page == kNavAppearance;
        const bool theme = page == kNavTheme;
        const bool diagnostics = page == kNavDiagnostics;
        const bool repair = page == kNavRepair;
        const bool packages = page == kNavPackages;
        for (const int id : {kStartup, kInputMethod, kInputMethodLabel})
            show(id, general);
        for (const int id : {kAppearance, kAppearanceLabel, kVertical, kHorizontal, kLayoutLabel,
                             kScrollMode, kPageSize, kPageSizeLabel})
            show(id, appearance);
        for (const int id : {kTheme, kThemeLabel, kFont, kFontLabel, kPreview})
            show(id, theme);
        show(kApply, general || appearance || theme);
        show(kSaveStatus, general || appearance || theme);
        const bool dirty = general ? generalDirty_ : presentationDirty_;
        for (const int id : {kRestart, kDiagnostics})
            show(id, diagnostics);
        for (const int id : {kRepair})
            show(id, repair);
        for (const int id : {kPackages, kPackageRefresh, kPackageInstall, kPackageToggle,
                             kPackageRemove, kPackagesTitle})
            show(id, packages);
        show(kStatus, diagnostics || repair || packages);
        setSaveStatus(dirty ? get("status.unsaved") : L"");
        ::SetWindowTextW(control(kPageTitle), page == kNavGeneral       ? get("nav.general")
                                              : page == kNavAppearance  ? get("nav.appearance")
                                              : page == kNavTheme       ? get("nav.theme")
                                              : page == kNavDiagnostics ? get("nav.diagnostics")
                                              : page == kNavRepair      ? get("nav.repair")
                                                                        : get("nav.packages"));
        layoutControls();
        for (int id = kNavGeneral; id <= kNavPackages; ++id)
            ::InvalidateRect(control(id), nullptr, TRUE);
        InvalidateRect(nullptr, TRUE);
    }
    void loadState() {
        std::wstring output;
        Strings presentation;
        if (runControl({L"--get-presentation"}, output) &&
            parseFlatJson(narrow(output), presentation)) {
            const auto mode = presentation.find("appearance_mode");
            if (mode != presentation.end())
                SendMessageW(control(kAppearance), CB_SETCURSEL,
                             mode->second == L"light" ? 1 : (mode->second == L"dark" ? 2 : 0), 0);
            const auto orientation = presentation.find("orientation");
            if (orientation != presentation.end() && orientation->second == L"horizontal") {
                SendMessageW(control(kVertical), BM_SETCHECK, BST_UNCHECKED, 0);
                SendMessageW(control(kHorizontal), BM_SETCHECK, BST_CHECKED, 0);
            }
            const auto candidateFont = presentation.find("candidate_font");
            if (candidateFont != presentation.end())
                ::SetWindowTextW(control(kFont), candidateFont->second.c_str());
            const auto scrollMode = presentation.find("scroll_mode");
            if (scrollMode != presentation.end())
                SendMessageW(control(kScrollMode), BM_SETCHECK,
                             scrollMode->second == L"true" ? BST_CHECKED : BST_UNCHECKED, 0);
            const auto pageSize = presentation.find("candidate_page_size");
            if (pageSize != presentation.end() && pageSize->second.size() == 1U &&
                pageSize->second[0] >= L'1' && pageSize->second[0] <= L'9') {
                SendMessageW(control(kPageSize), CB_SETCURSEL, pageSize->second[0] - L'1', 0);
            }
        }
        if (runControl({L"--get-startup"}, output))
            SendMessageW(control(kStartup), BM_SETCHECK,
                         output.find(L"\"enabled\":true") != std::wstring::npos ? BST_CHECKED
                                                                                : BST_UNCHECKED,
                         0);
        loadInputMethods();
        refresh();
        refreshPackages(false);
    }
    void loadInputMethods() {
        std::wstring output;
        std::vector<InputMethodRow> rows;
        if (!runControl({L"--get-input-methods"}, output) ||
            !parseInputMethods(output, rows)) {
            ::EnableWindow(control(kInputMethod), FALSE);
            return;
        }
        SendMessageW(control(kInputMethod), CB_RESETCONTENT, 0, 0);
        inputMethods_ = std::move(rows);
        int selected = 0;
        for (std::size_t index = 0; index < inputMethods_.size(); ++index) {
            const auto& method = inputMethods_[index];
            std::wstring label = method.nativeName.empty() || method.nativeName == method.name
                                     ? method.name
                                     : method.nativeName + L" (" + method.name + L")";
            SendMessageW(control(kInputMethod), CB_ADDSTRING, 0,
                         reinterpret_cast<LPARAM>(label.c_str()));
            if (method.selected)
                selected = static_cast<int>(index);
        }
        SendMessageW(control(kInputMethod), CB_SETCURSEL, selected, 0);
        ::EnableWindow(control(kInputMethod), TRUE);
    }
    bool applyInputMethod() {
        const auto selected = SendMessageW(control(kInputMethod), CB_GETCURSEL, 0, 0);
        if (selected == CB_ERR || static_cast<std::size_t>(selected) >= inputMethods_.size())
            return false;
        std::wstring output;
        const bool ok = runControl(
            {L"--set-input-method", inputMethods_[static_cast<std::size_t>(selected)].id}, output);
        if (ok)
            loadInputMethods();
        return ok;
    }
    bool applyStartup() {
        std::wstring output;
        return runControl(
            {L"--set-startup",
             SendMessageW(control(kStartup), BM_GETCHECK, 0, 0) == BST_CHECKED ? L"enabled"
                                                                               : L"disabled"},
            output);
    }
    void refresh() {
        std::wstring output;
        ::SetWindowTextW(control(kStatus),
                         runControl({L"--status"}, output) ? output.c_str() : get("error.command"));
    }
    void restart() {
        std::wstring output;
        setStatus(runControl({L"--restart-engine"}, output) ? get("restart.done")
                                                            : get("error.command"));
    }
    bool applyPresentation() {
        const auto modeIndex = SendMessageW(control(kAppearance), CB_GETCURSEL, 0, 0);
        const wchar_t* const mode =
            modeIndex == 1 ? L"light" : (modeIndex == 2 ? L"dark" : L"system");
        const wchar_t* const orientation =
            SendMessageW(control(kHorizontal), BM_GETCHECK, 0, 0) == BST_CHECKED ? L"horizontal"
                                                                                 : L"vertical";
        const auto pageSizeIndex = SendMessageW(control(kPageSize), CB_GETCURSEL, 0, 0);
        if (pageSizeIndex == CB_ERR || pageSizeIndex < 0 || pageSizeIndex > 8)
            return false;
        const std::wstring pageSize = std::to_wstring(pageSizeIndex + 1);
        wchar_t fontBuffer[129]{};
        ::GetWindowTextW(control(kFont), fontBuffer, static_cast<int>(std::size(fontBuffer)));
        const std::wstring font(fontBuffer);
        std::wstring output;
        const bool ok =
            !font.empty() &&
            runControl({L"--set-presentation", mode, L"builtin:default", orientation,
                        SendMessageW(control(kScrollMode), BM_GETCHECK, 0, 0) == BST_CHECKED
                            ? L"enabled"
                            : L"disabled",
                        pageSize, font},
                       output);
        return ok;
    }
    void repair() {
        const fs::path directory = executableDirectory();
        const fs::path root = directory.filename() == L"bin" ? directory.parent_path() : directory;
        const fs::path bootstrap = root / L"Start Fcitx5.exe";
        if (!fs::is_regular_file(bootstrap)) {
            setStatus(get("error.command"));
            return;
        }
        const auto result = reinterpret_cast<std::intptr_t>(ShellExecuteW(
            m_hWnd, nullptr, bootstrap.c_str(), L"--repair-only", root.c_str(), SW_SHOWNORMAL));
        setStatus(result > 32 ? get("repair.started") : get("error.command"));
    }

    int selectedPackage() const {
        const auto selected = SendMessageW(control(kPackages), LB_GETCURSEL, 0, 0);
        return selected == LB_ERR || static_cast<std::size_t>(selected) >= packages_.size()
                   ? -1
                   : static_cast<int>(selected);
    }

    void refreshPackages(bool online) {
        std::wstring output;
        const bool refreshed = !online || runControl({L"--packages-refresh"}, output);
        if (!runControl({L"--packages-list"}, output) ||
            !parsePackages(output, packages_, repositoryAvailable_)) {
            setStatus(get("error.command"));
            return;
        }
        SendMessageW(control(kPackages), LB_RESETCONTENT, 0, 0);
        for (const auto& package : packages_) {
            const std::wstring label = packageListLabel(package, strings_);
            SendMessageW(control(kPackages), LB_ADDSTRING, 0,
                         reinterpret_cast<LPARAM>(label.c_str()));
        }
        if (!packages_.empty())
            SendMessageW(control(kPackages), LB_SETCURSEL, 0, 0);
        updatePackageActions();
        if (!refreshed)
            setStatus(get("packages.online_error"));
        else if (!repositoryAvailable_)
            setStatus(get("packages.online_unavailable"));
    }

    void updatePackageActions() {
        const int selected = selectedPackage();
        const PackageRow* package =
            selected < 0 ? nullptr : &packages_[static_cast<std::size_t>(selected)];
        const bool bundled = package && package->state == L"bundled";
        ::EnableWindow(control(kPackageInstall),
                       package && !bundled && !package->available.empty());
        ::EnableWindow(control(kPackageToggle),
                       package && !bundled && !package->installed.empty());
        ::EnableWindow(control(kPackageRemove),
                       package && !bundled && !package->installed.empty());
    }

    void installOrUpdatePackage() {
        const int selected = selectedPackage();
        if (selected < 0)
            return;
        auto& package = packages_[static_cast<std::size_t>(selected)];
        if (package.state == L"bundled") {
            setStatus(get("packages.bundled_readonly"));
            return;
        }
        if (package.available.empty())
            return;
        std::wstring output;
        const bool ok = runControl(
            {package.installed.empty() ? L"--packages-install" : L"--packages-update", package.id},
            output);
        setStatus(ok ? get("packages.changed") : get("error.command"));
        refreshPackages(false);
    }

    void removePackage() {
        const int selected = selectedPackage();
        if (selected < 0 || packages_[static_cast<std::size_t>(selected)].installed.empty())
            return;
        if (packages_[static_cast<std::size_t>(selected)].state == L"bundled") {
            setStatus(get("packages.bundled_readonly"));
            return;
        }
        std::wstring output;
        const bool ok = runControl(
            {L"--packages-remove", packages_[static_cast<std::size_t>(selected)].id}, output);
        setStatus(ok ? get("packages.changed") : get("error.command"));
        refreshPackages(false);
    }

    void togglePackage() {
        const int selected = selectedPackage();
        if (selected < 0)
            return;
        const auto& package = packages_[static_cast<std::size_t>(selected)];
        if (package.installed.empty())
            return;
        if (package.state == L"bundled") {
            setStatus(get("packages.bundled_readonly"));
            return;
        }
        std::wstring output;
        const bool ok = runControl({L"--packages-state", package.id,
                                    package.state == L"disabled" ? L"enabled" : L"disabled"},
                                   output);
        setStatus(ok ? get("packages.changed") : get("error.command"));
        refreshPackages(false);
    }

    void preview() {
        if (previewProcess_) {
            if (WaitForSingleObject(previewProcess_, 0) == WAIT_TIMEOUT)
                TerminateProcess(previewProcess_, 0);
            CloseHandle(previewProcess_);
            previewProcess_ = nullptr;
        }
        const fs::path executable = executableDirectory() / L"fcitx5-ui.exe";
        if (!fs::exists(executable)) {
            setStatus(get("error.command"));
            return;
        }
        std::wstring command = quote(executable.wstring()) + L" --demo --parent-pid " +
                               std::to_wstring(GetCurrentProcessId());
        STARTUPINFOW startup{};
        startup.cb = sizeof(startup);
        PROCESS_INFORMATION process{};
        if (!CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, FALSE, 0, nullptr,
                            executable.parent_path().c_str(), &startup, &process)) {
            setStatus(get("error.command"));
            return;
        }
        CloseHandle(process.hThread);
        previewProcess_ = process.hProcess;
    }

    LRESULT onPaint(UINT, WPARAM, LPARAM, BOOL&) {
        PAINTSTRUCT paint{};
        BeginPaint(&paint);
        if (!factory_)
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, &factory_);
        if (factory_ && !target_) {
            RECT rectangle{};
            GetClientRect(&rectangle);
            factory_->CreateHwndRenderTarget(
                D2D1::RenderTargetProperties(),
                D2D1::HwndRenderTargetProperties(m_hWnd,
                                                 D2D1::SizeU(rectangle.right, rectangle.bottom)),
                &target_);
            // Controls are positioned in physical client pixels. Keep the decorative D2D layer
            // in that same coordinate space so cards and controls stay aligned at 150/200% DPI.
            if (target_)
                target_->SetDpi(96.0f, 96.0f);
        }
        if (target_) {
            target_->BeginDraw();
            target_->Clear(D2D1::ColorF(0xF7F8FA));
            ID2D1SolidColorBrush* brush = nullptr;
            target_->CreateSolidColorBrush(D2D1::ColorF(0xE9EBEF), &brush);
            if (brush) {
                target_->FillRectangle(D2D1::RectF(0, 0, 204, 650), brush);
                brush->SetColor(D2D1::ColorF(0xFFFFFF));
                target_->FillRoundedRectangle(
                    D2D1::RoundedRect(D2D1::RectF(220, 74, 986, 610), 14, 14), brush);
                brush->Release();
            }
            if (target_->EndDraw() == D2DERR_RECREATE_TARGET) {
                target_->Release();
                target_ = nullptr;
            }
        }
        EndPaint(&paint);
        // A Direct2D parent surface can be presented after native child controls have painted.
        // Refresh visible children once so the decorative card never obscures real controls.
        for (HWND child = ::GetWindow(m_hWnd, GW_CHILD); child;
             child = ::GetWindow(child, GW_HWNDNEXT)) {
            if (::IsWindowVisible(child))
                ::RedrawWindow(child, nullptr, nullptr,
                               RDW_INVALIDATE | RDW_ERASE | RDW_UPDATENOW);
        }
        return 0;
    }
    LRESULT onDrawItem(UINT, WPARAM, LPARAM lparam, BOOL&) {
        const auto* item = reinterpret_cast<const DRAWITEMSTRUCT*>(lparam);
        if (!item || item->CtlID < kNavGeneral || item->CtlID > kNavPackages)
            return FALSE;
        const bool selected = static_cast<int>(item->CtlID) == selectedPage_;
        HBRUSH background = CreateSolidBrush(selected ? RGB(255, 255, 255) : RGB(233, 235, 239));
        HPEN pen = CreatePen(PS_NULL, 0, 0);
        const HGDIOBJ oldBrush = SelectObject(item->hDC, background);
        const HGDIOBJ oldPen = SelectObject(item->hDC, pen);
        RoundRect(item->hDC, item->rcItem.left, item->rcItem.top, item->rcItem.right,
                  item->rcItem.bottom, 14, 14);
        SelectObject(item->hDC, oldBrush);
        SelectObject(item->hDC, oldPen);
        DeleteObject(background);
        DeleteObject(pen);
        SetBkMode(item->hDC, TRANSPARENT);
        SetTextColor(item->hDC, selected ? RGB(0, 122, 82) : RGB(63, 66, 71));
        RECT text = item->rcItem;
        text.left += 16;
        wchar_t label[128]{};
        ::GetWindowTextW(item->hwndItem, label, static_cast<int>(std::size(label)));
        DrawTextW(item->hDC, label, -1, &text, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        if ((item->itemState & ODS_FOCUS) != 0)
            DrawFocusRect(item->hDC, &item->rcItem);
        return TRUE;
    }
    LRESULT onColorStatic(UINT, WPARAM wparam, LPARAM, BOOL&) {
        const HDC dc = reinterpret_cast<HDC>(wparam);
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, RGB(48, 50, 54));
        return reinterpret_cast<LRESULT>(GetStockObject(HOLLOW_BRUSH));
    }
    LRESULT onDestroy(UINT, WPARAM, LPARAM, BOOL&) {
        if (statusTimer_)
            ::KillTimer(m_hWnd, statusTimer_);
        if (previewProcess_) {
            if (WaitForSingleObject(previewProcess_, 0) == WAIT_TIMEOUT)
                TerminateProcess(previewProcess_, 0);
            CloseHandle(previewProcess_);
        }
        if (font_)
            DeleteObject(font_);
        if (titleFont_)
            DeleteObject(titleFont_);
        if (target_)
            target_->Release();
        if (factory_)
            factory_->Release();
        PostQuitMessage(0);
        return 0;
    }
    LRESULT onApply(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_) {
            if (selectedPage_ == kNavGeneral) {
                cover(kCoveredGeneralApply);
                generalDirty_ = false;
            } else if (selectedPage_ == kNavAppearance) {
                cover(kCoveredAppearanceApply);
                presentationDirty_ = false;
            } else if (selectedPage_ == kNavTheme) {
                cover(kCoveredThemeApply);
                presentationDirty_ = false;
            }
            setSaveStatus(get("status.saved"));
            return 0;
        }
        bool ok = false;
        if (selectedPage_ == kNavGeneral) {
            const bool startupSaved = applyStartup();
            const bool inputMethodSaved = applyInputMethod();
            ok = startupSaved && inputMethodSaved;
            if (ok)
                generalDirty_ = false;
        } else if (selectedPage_ == kNavAppearance || selectedPage_ == kNavTheme) {
            ok = applyPresentation();
            if (ok)
                presentationDirty_ = false;
        }
        setSaveStatus(ok ? get("status.saved") : get("error.command"));
        return 0;
    }
    LRESULT onNavigate(WORD, WORD id, HWND, BOOL&) {
        showPage(id);
        return 0;
    }
    LRESULT onRestart(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_) {
            cover(kCoveredRestart);
            ::SetWindowTextW(control(kStatus), L"restart-covered");
            return 0;
        }
        restart();
        return 0;
    }
    LRESULT onDiagnostics(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_) {
            cover(kCoveredDiagnostics);
            ::SetWindowTextW(control(kStatus), L"diagnostics-covered");
            return 0;
        }
        refresh();
        return 0;
    }
    LRESULT onRepair(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_) {
            cover(kCoveredRepair);
            ::SetWindowTextW(control(kStatus), L"repair-covered");
            return 0;
        }
        repair();
        return 0;
    }
    LRESULT onPreview(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_) {
            cover(kCoveredPreview);
            return 0;
        }
        preview();
        return 0;
    }
    LRESULT onPackageRefresh(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_) {
            cover(kCoveredPackageRefresh);
            return 0;
        }
        refreshPackages(true);
        return 0;
    }
    LRESULT onPackageInstall(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_) {
            const int selected = selectedPackage();
            if (selected >= 0 &&
                packages_[static_cast<std::size_t>(selected)].installed.empty())
                cover(kCoveredPackageInstall);
            else
                cover(kCoveredPackageUpdate);
            return 0;
        }
        installOrUpdatePackage();
        return 0;
    }
    LRESULT onPackageRemove(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_) {
            cover(kCoveredPackageRemove);
            return 0;
        }
        removePackage();
        return 0;
    }
    LRESULT onPackageToggle(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_) {
            const int selected = selectedPackage();
            if (selected < 0)
                return 0;
            cover(packages_[static_cast<std::size_t>(selected)].state == L"disabled"
                      ? kCoveredPackageEnable
                      : kCoveredPackageDisable);
            return 0;
        }
        togglePackage();
        return 0;
    }
    LRESULT onPackageSelection(WORD, WORD, HWND, BOOL&) {
        updatePackageActions();
        return 0;
    }
    // Updating a STATIC control with SetWindowTextW only repaints the new
    // text's bounding box. When the new text is shorter than the previous one
    // (for example "设置已安全保存" -> "有未应用的更改"), the tail of the old
    // text stays on screen and the two messages overlap. Invalidate the whole
    // control so the background is erased before drawing the new text.
    void armStatusTimer() {
        if (statusTimer_)
            ::KillTimer(m_hWnd, statusTimer_);
        statusTimer_ = ::SetTimer(m_hWnd, kStatusTimerId, kStatusTimeoutMs, nullptr);
    }
    // Transient notices in the diagnostics status line (package operations,
    // restart/repair/preview results, command errors) auto-dismiss after a few
    // seconds. The diagnostics --status snapshot (refresh) is persistent and
    // does not go through this helper.
    void setStatus(const wchar_t* text) {
        ::SetWindowTextW(control(kStatus), text);
        if (text && *text)
            armStatusTimer();
    }
    // The status STATICs use a transparent background (WM_CTLCOLORSTATIC ->
    // HOLLOW_BRUSH), so invalidating only the control leaves the previous
    // text on screen and the notices overlap ("保存成功" + "有未应用的更改").
    // Redraw the whole window background (D2D layer) as well as the controls.
    void refreshStatusControls() {
        if (m_hWnd)
            ::InvalidateRect(m_hWnd, nullptr, TRUE);
        if (const HWND status = control(kSaveStatus))
            ::InvalidateRect(status, nullptr, TRUE);
        if (const HWND status = control(kStatus))
            ::InvalidateRect(status, nullptr, TRUE);
    }
    void setSaveStatus(const wchar_t* text) {
        ::SetWindowTextW(control(kSaveStatus), text);
        refreshStatusControls();
        // Every notice ("已保存" / "有未应用的更改" / 命令错误) clears the
        // status control a few seconds after it appears.
        if (text && *text)
            armStatusTimer();
    }
    LRESULT onTimer(UINT, WPARAM, LPARAM, BOOL&) {
        if (statusTimer_) {
            ::KillTimer(m_hWnd, statusTimer_);
            statusTimer_ = 0;
        }
        ::SetWindowTextW(control(kSaveStatus), L"");
        ::SetWindowTextW(control(kStatus), L"");
        refreshStatusControls();
        return 0;
    }
    LRESULT onDirty(WORD, WORD id, HWND, BOOL&) {
        if (id == kStartup || id == kInputMethod)
            generalDirty_ = true;
        else
            presentationDirty_ = true;
        setSaveStatus(get("status.unsaved"));
        return 0;
    }

    Strings strings_;
    HFONT font_{};
    HFONT titleFont_{};
    ID2D1Factory* factory_{};
    ID2D1HwndRenderTarget* target_{};
    UINT_PTR statusTimer_{};
    HANDLE previewProcess_{};
    std::vector<PackageRow> packages_;
    std::vector<InputMethodRow> inputMethods_;
    int selectedPage_{kNavGeneral};
    bool generalDirty_{};
    bool presentationDirty_{};
    bool repositoryAvailable_{};
    bool interactionTest_{};
    unsigned long long actionCoverage_{};
    UINT dpi_{96};
};

} // namespace

CAppModule _Module;

int WINAPI wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE, _In_ PWSTR commandLine,
                    _In_ int showCommand) {
    enableDpiAwareness();
    const std::wstring_view command(commandLine);
    if (command == L"--check-i18n")
        return checkI18n() ? 0 : 2;
    if (command == L"--check-resources")
        return checkResources() ? 0 : 2;
    if (command == L"--self-test")
        return checkI18n() && checkResources() ? 0 : 2;
    const bool uiContractTest = command == L"--ui-contract-test";
    const bool uiInteractionTest = command == L"--ui-interaction-test";
    const bool showDiagnostics = command == L"--diagnostics";
    if (!command.empty() && !uiContractTest && !uiInteractionTest && !showDiagnostics)
        return 1;
    const LANGID language = GetUserDefaultUILanguage();
    const wchar_t* locale = PRIMARYLANGID(language) == LANG_CHINESE ? L"zh-CN.json" : L"en-US.json";
    Strings strings;
    if (!loadLocale(executableDirectory() / L"locales" / locale, strings))
        return 2;
    if (FAILED(_Module.Init(nullptr, instance)))
        return 3;
    CMessageLoop loop;
    _Module.AddMessageLoop(&loop);
    ConfigWindow window(std::move(strings));
    std::wstring title = window.title();
    title += L"  v";
    title += widen(fcitx::windows::version());
    if (!window.Create(nullptr, CWindow::rcDefault, title.c_str(),
                       WS_OVERLAPPEDWINDOW & ~WS_MAXIMIZEBOX)) {
        _Module.RemoveMessageLoop();
        _Module.Term();
        return 4;
    }
    enableNativeWindowEffects(window);
    window.resizeToDefaultClient();
    window.CenterWindow();
    if (uiContractTest || uiInteractionTest) {
        const bool passed = uiContractTest ? window.verifyUiContract()
                                           : window.verifyInteractionCoverage();
        window.DestroyWindow();
        _Module.RemoveMessageLoop();
        _Module.Term();
        return passed ? 0 : 5;
    }
    if (showDiagnostics)
        window.selectPage(kNavDiagnostics);
    window.ShowWindow(showCommand);
    const int result = loop.Run();
    _Module.RemoveMessageLoop();
    _Module.Term();
    return result;
}
