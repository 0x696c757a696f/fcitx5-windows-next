#include "fcitx5_windows/version.h"

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
#include <string>
#include <string_view>
#include <unordered_map>
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

bool runExecutable(const fs::path& executable, const std::vector<std::wstring>& arguments,
                   std::wstring& output) {
    if (!fs::exists(executable))
        return false;
    std::wstring command = quote(executable.wstring());
    for (const auto& argument : arguments)
        command += L" " + quote(argument);
    SECURITY_ATTRIBUTES attributes{sizeof(attributes), nullptr, TRUE};
    HANDLE readPipe = nullptr;
    HANDLE writePipe = nullptr;
    if (!CreatePipe(&readPipe, &writePipe, &attributes, 0))
        return false;
    SetHandleInformation(readPipe, HANDLE_FLAG_INHERIT, 0);
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    startup.dwFlags = STARTF_USESTDHANDLES;
    startup.hStdOutput = writePipe;
    startup.hStdError = writePipe;
    PROCESS_INFORMATION process{};
    const BOOL created =
        CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, TRUE, CREATE_NO_WINDOW,
                       nullptr, executable.parent_path().c_str(), &startup, &process);
    CloseHandle(writePipe);
    if (!created) {
        CloseHandle(readPipe);
        return false;
    }
    const DWORD wait = WaitForSingleObject(process.hProcess, 120000);
    if (wait == WAIT_TIMEOUT)
        TerminateProcess(process.hProcess, ERROR_TIMEOUT);
    std::string bytes;
    char buffer[2048];
    DWORD count = 0;
    while (ReadFile(readPipe, buffer, sizeof(buffer), &count, nullptr) && count != 0)
        bytes.append(buffer, count);
    DWORD exitCode = 1;
    GetExitCodeProcess(process.hProcess, &exitCode);
    CloseHandle(readPipe);
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    output = widen(bytes);
    return wait == WAIT_OBJECT_0 && exitCode == 0;
}

bool runControl(const std::vector<std::wstring>& arguments, std::wstring& output) {
    return runExecutable(executableDirectory() / L"fcitx5-control.exe", arguments, output);
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
    return runExecutable(executableDirectory() / L"fcitx5-ui.exe", {L"--self-test", L"--safe-mode"},
                         output);
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

struct PackageRow {
    std::wstring id;
    std::wstring title;
    std::wstring available;
    std::wstring installed;
    std::wstring state;
    bool update{};
};

bool parsePackages(std::wstring_view output, std::vector<PackageRow>& rows) {
    try {
        const auto document = nlohmann::json::parse(narrow(output));
        if (!document.is_object() || document.size() != 3U || document.at("format_version") != 1 ||
            !document.at("repository_available").is_boolean() ||
            !document.at("packages").is_array() || document.at("packages").size() > 4096U)
            return false;
        rows.clear();
        for (const auto& item : document.at("packages")) {
            if (!item.is_object() || item.size() != 8U)
                return false;
            PackageRow row;
            row.id = widen(item.at("id").get<std::string>());
            row.title = widen(item.at("title").get<std::string>());
            if (!item.at("available_version").is_null())
                row.available = widen(item.at("available_version").get<std::string>());
            if (!item.at("installed_version").is_null())
                row.installed = widen(item.at("installed_version").get<std::string>());
            if (!item.at("state").is_null())
                row.state = widen(item.at("state").get<std::string>());
            row.update = item.at("update_available").get<bool>();
            if (row.id.empty() || row.title.empty())
                return false;
            rows.push_back(std::move(row));
        }
        return true;
    } catch (const nlohmann::json::exception&) {
        return false;
    }
}

class ConfigWindow final : public CWindowImpl<ConfigWindow> {
  public:
    DECLARE_WND_CLASS_EX(L"Fcitx5ConfigWindow", CS_HREDRAW | CS_VREDRAW, COLOR_WINDOW)

    explicit ConfigWindow(Strings strings) : strings_(std::move(strings)) {}
    const wchar_t* title() const { return get("app.title"); }

    BEGIN_MSG_MAP(ConfigWindow)
    MESSAGE_HANDLER(WM_CREATE, onCreate)
    MESSAGE_HANDLER(WM_PAINT, onPaint)
    MESSAGE_HANDLER(WM_DRAWITEM, onDrawItem)
    MESSAGE_HANDLER(WM_CTLCOLORSTATIC, onColorStatic)
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
    END_MSG_MAP()

  private:
    const wchar_t* get(const char* key) const {
        const auto iterator = strings_.find(key);
        return iterator == strings_.end() ? L"" : iterator->second.c_str();
    }
    HWND control(int id) const { return GetDlgItem(id); }
    void add(const wchar_t* type, const wchar_t* label, DWORD style, int x, int y, int width,
             int height, int id = 0) {
        HWND child =
            CreateWindowExW(0, type, label, WS_CHILD | WS_VISIBLE | style, x, y, width, height,
                            m_hWnd, reinterpret_cast<HMENU>(static_cast<std::intptr_t>(id)),
                            _Module.GetModuleInstance(), nullptr);
        SendMessageW(child, WM_SETFONT, reinterpret_cast<WPARAM>(font_), TRUE);
    }
    LRESULT onCreate(UINT, WPARAM, LPARAM, BOOL&) {
        NONCLIENTMETRICSW metrics{};
        metrics.cbSize = sizeof(metrics);
        if (!SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, sizeof(metrics), &metrics, 0)) {
            GetObjectW(GetStockObject(DEFAULT_GUI_FONT), sizeof(metrics.lfMessageFont),
                       &metrics.lfMessageFont);
        }
        font_ = CreateFontIndirectW(&metrics.lfMessageFont);
        LOGFONTW titleMetrics = metrics.lfMessageFont;
        titleMetrics.lfHeight = -24;
        titleMetrics.lfWeight = FW_SEMIBOLD;
        titleFont_ = CreateFontIndirectW(&titleMetrics);
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
        SendMessageW(control(kInputMethod), CB_ADDSTRING, 0,
                     reinterpret_cast<LPARAM>(get("input.pinyin")));
        SendMessageW(control(kInputMethod), CB_SETCURSEL, 0, 0);
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
        SendMessageW(control(kVertical), BM_SETCHECK, BST_CHECKED, 0);
        add(L"BUTTON", get("action.apply"), BS_DEFPUSHBUTTON | WS_TABSTOP, 650, 264, 120, 36,
            kApply);
        add(L"BUTTON", get("action.preview"), WS_TABSTOP, 250, 264, 160, 36, kPreview);
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
        showPage(kNavGeneral);
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
                             kScrollMode,      kApply,          kPreview,          kRestart,
                             kDiagnostics,     kRepair,         kStatus,           kPackages,
                             kPackageRefresh,  kPackageInstall, kPackageToggle,    kPackageRemove,
                             kPackagesTitle})
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
                             kScrollMode, kApply})
            show(id, appearance);
        for (const int id : {kTheme, kThemeLabel, kFont, kFontLabel, kPreview, kApply})
            show(id, theme);
        for (const int id : {kRestart, kDiagnostics, kStatus})
            show(id, diagnostics);
        for (const int id : {kRepair, kStatus})
            show(id, repair);
        for (const int id : {kPackages, kPackageRefresh, kPackageInstall, kPackageToggle,
                             kPackageRemove, kStatus, kPackagesTitle})
            show(id, packages);
        ::SetWindowTextW(control(kPageTitle), page == kNavGeneral       ? get("nav.general")
                                              : page == kNavAppearance  ? get("nav.appearance")
                                              : page == kNavTheme       ? get("nav.theme")
                                              : page == kNavDiagnostics ? get("nav.diagnostics")
                                              : page == kNavRepair      ? get("nav.repair")
                                                                        : get("nav.packages"));
        if (packages)
            ::SetWindowPos(control(kStatus), nullptr, 250, 542, 700, 50, SWP_NOZORDER);
        else
            ::SetWindowPos(control(kStatus), nullptr, 250, 168, 700, 250, SWP_NOZORDER);
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
        }
        if (runControl({L"--get-startup"}, output))
            SendMessageW(control(kStartup), BM_SETCHECK,
                         output.find(L"\"enabled\":true") != std::wstring::npos ? BST_CHECKED
                                                                                : BST_UNCHECKED,
                         0);
        refresh();
        refreshPackages(false);
    }
    void refresh() {
        std::wstring output;
        ::SetWindowTextW(control(kStatus),
                         runControl({L"--status"}, output) ? output.c_str() : get("error.command"));
    }
    void restart() {
        std::wstring output;
        ::SetWindowTextW(control(kStatus), runControl({L"--restart-engine"}, output)
                                               ? get("restart.done")
                                               : get("error.command"));
    }
    void apply() {
        const auto modeIndex = SendMessageW(control(kAppearance), CB_GETCURSEL, 0, 0);
        const wchar_t* const mode =
            modeIndex == 1 ? L"light" : (modeIndex == 2 ? L"dark" : L"system");
        const wchar_t* const orientation =
            SendMessageW(control(kHorizontal), BM_GETCHECK, 0, 0) == BST_CHECKED ? L"horizontal"
                                                                                 : L"vertical";
        wchar_t fontBuffer[129]{};
        ::GetWindowTextW(control(kFont), fontBuffer, static_cast<int>(std::size(fontBuffer)));
        const std::wstring font(fontBuffer);
        std::wstring output;
        std::wstring startupOutput;
        const bool ok =
            !font.empty() &&
            runControl(
                {L"--set-startup", SendMessageW(control(kStartup), BM_GETCHECK, 0, 0) == BST_CHECKED
                                       ? L"enabled"
                                       : L"disabled"},
                startupOutput) &&
            runControl({L"--set-presentation", mode, L"builtin:default", orientation,
                        SendMessageW(control(kScrollMode), BM_GETCHECK, 0, 0) == BST_CHECKED
                            ? L"enabled"
                            : L"disabled",
                        font},
                       output);
        ::SetWindowTextW(control(kStatus), ok ? get("status.saved") : get("error.command"));
    }
    void repair() {
        const fs::path directory = executableDirectory();
        const fs::path root = directory.filename() == L"bin" ? directory.parent_path() : directory;
        fs::path dll = root / L"tsf/x64/fcitx5-tsf.dll";
        if (!fs::exists(dll))
            dll = directory / L"fcitx5-tsf.dll";
        const fs::path registration = directory / L"fcitx5-register.exe";
        const std::wstring arguments = L"--repair --dll " + quote(dll.wstring());
        const auto result = reinterpret_cast<std::intptr_t>(ShellExecuteW(
            m_hWnd, L"runas", registration.c_str(), arguments.c_str(), directory.c_str(), SW_HIDE));
        ::SetWindowTextW(control(kStatus),
                         result > 32 ? get("repair.started") : get("error.command"));
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
        if (!refreshed || !runControl({L"--packages-list"}, output) ||
            !parsePackages(output, packages_)) {
            ::SetWindowTextW(control(kStatus), get("error.command"));
            return;
        }
        SendMessageW(control(kPackages), LB_RESETCONTENT, 0, 0);
        for (const auto& package : packages_) {
            std::wstring label =
                package.title + L"  " +
                (!package.installed.empty() ? package.installed : package.available);
            if (package.update)
                label += L"  ↑";
            if (package.state == L"disabled")
                label += L"  (disabled)";
            SendMessageW(control(kPackages), LB_ADDSTRING, 0,
                         reinterpret_cast<LPARAM>(label.c_str()));
        }
        if (!packages_.empty())
            SendMessageW(control(kPackages), LB_SETCURSEL, 0, 0);
    }

    void installOrUpdatePackage() {
        const int selected = selectedPackage();
        if (selected < 0)
            return;
        auto& package = packages_[static_cast<std::size_t>(selected)];
        if (package.available.empty())
            return;
        std::wstring output;
        const bool ok = runControl(
            {package.installed.empty() ? L"--packages-install" : L"--packages-update", package.id},
            output);
        ::SetWindowTextW(control(kStatus), ok ? get("packages.changed") : get("error.command"));
        refreshPackages(false);
    }

    void removePackage() {
        const int selected = selectedPackage();
        if (selected < 0 || packages_[static_cast<std::size_t>(selected)].installed.empty())
            return;
        std::wstring output;
        const bool ok = runControl(
            {L"--packages-remove", packages_[static_cast<std::size_t>(selected)].id}, output);
        ::SetWindowTextW(control(kStatus), ok ? get("packages.changed") : get("error.command"));
        refreshPackages(false);
    }

    void togglePackage() {
        const int selected = selectedPackage();
        if (selected < 0)
            return;
        const auto& package = packages_[static_cast<std::size_t>(selected)];
        if (package.installed.empty())
            return;
        std::wstring output;
        const bool ok = runControl({L"--packages-state", package.id,
                                    package.state == L"disabled" ? L"enabled" : L"disabled"},
                                   output);
        ::SetWindowTextW(control(kStatus), ok ? get("packages.changed") : get("error.command"));
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
            ::SetWindowTextW(control(kStatus), get("error.command"));
            return;
        }
        std::wstring command = quote(executable.wstring()) + L" --demo --parent-pid " +
                               std::to_wstring(GetCurrentProcessId());
        STARTUPINFOW startup{};
        startup.cb = sizeof(startup);
        PROCESS_INFORMATION process{};
        if (!CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, FALSE, 0, nullptr,
                            executable.parent_path().c_str(), &startup, &process)) {
            ::SetWindowTextW(control(kStatus), get("error.command"));
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
        apply();
        return 0;
    }
    LRESULT onNavigate(WORD, WORD id, HWND, BOOL&) {
        showPage(id);
        return 0;
    }
    LRESULT onRestart(WORD, WORD, HWND, BOOL&) {
        restart();
        return 0;
    }
    LRESULT onDiagnostics(WORD, WORD, HWND, BOOL&) {
        refresh();
        return 0;
    }
    LRESULT onRepair(WORD, WORD, HWND, BOOL&) {
        repair();
        return 0;
    }
    LRESULT onPreview(WORD, WORD, HWND, BOOL&) {
        preview();
        return 0;
    }
    LRESULT onPackageRefresh(WORD, WORD, HWND, BOOL&) {
        refreshPackages(true);
        return 0;
    }
    LRESULT onPackageInstall(WORD, WORD, HWND, BOOL&) {
        installOrUpdatePackage();
        return 0;
    }
    LRESULT onPackageRemove(WORD, WORD, HWND, BOOL&) {
        removePackage();
        return 0;
    }
    LRESULT onPackageToggle(WORD, WORD, HWND, BOOL&) {
        togglePackage();
        return 0;
    }

    Strings strings_;
    HFONT font_{};
    HFONT titleFont_{};
    ID2D1Factory* factory_{};
    ID2D1HwndRenderTarget* target_{};
    HANDLE previewProcess_{};
    std::vector<PackageRow> packages_;
    int selectedPage_{kNavGeneral};
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
    if (!window.Create(nullptr, CWindow::rcDefault, window.title(),
                       WS_OVERLAPPEDWINDOW & ~WS_MAXIMIZEBOX)) {
        _Module.RemoveMessageLoop();
        _Module.Term();
        return 4;
    }
    enableNativeWindowEffects(window);
    window.ResizeClient(1010, 650);
    window.CenterWindow();
    window.ShowWindow(showCommand);
    const int result = loop.Run();
    _Module.RemoveMessageLoop();
    _Module.Term();
    return result;
}
