#include "fcitx5_windows/version.h"
#include "candidate_layout.h"
#include "process_execution.h"
#include "resource.h"

#include <fcitx5_windows/release_identity.h>

#include <Windows.h>
#include <CommCtrl.h>
#include <shellapi.h>
#include <d2d1.h>
#include <dwrite.h>

#include <atlbase.h>
#include <atlapp.h>
extern CAppModule _Module;
#include <atlctrls.h>
#include <atlframe.h>
#include <atlwin.h>

#include <algorithm>
#include <filesystem>
#include <fstream>
#include <iostream>
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

void setCurrentProcessAppUserModelId(const wchar_t* appId) noexcept {
    const HMODULE shell = LoadLibraryW(L"shell32.dll");
    if (!shell)
        return;
    using SetAppId = HRESULT(WINAPI*)(PCWSTR);
    const auto setAppId =
        reinterpret_cast<SetAppId>(GetProcAddress(shell, "SetCurrentProcessExplicitAppUserModelID"));
    if (setAppId && appId && *appId)
        (void)setAppId(appId);
    FreeLibrary(shell);
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

struct ParsedCommandLine {
    std::wstring command;
    std::wstring localeOverride;
    bool valid{true};
};

ParsedCommandLine parseCommandLine(std::wstring_view commandLine) {
    ParsedCommandLine parsed;
    if (commandLine.empty())
        return parsed;
    std::wstring mutableCommandLine(commandLine);
    int count = 0;
    PWSTR* arguments = CommandLineToArgvW(mutableCommandLine.c_str(), &count);
    if (!arguments)
        return {{}, {}, false};
    for (int index = 0; index < count; ++index) {
        const std::wstring_view argument(arguments[index]);
        if (argument.starts_with(L"--lang=")) {
            if (!parsed.localeOverride.empty()) {
                parsed.valid = false;
                break;
            }
            parsed.localeOverride = std::wstring(argument.substr(7));
        } else if (!argument.empty()) {
            if (!parsed.command.empty()) {
                parsed.valid = false;
                break;
            }
            parsed.command = std::wstring(argument);
        }
    }
    LocalFree(arguments);
    return parsed;
}

const wchar_t* localeFileForOverride(std::wstring_view overrideLocale) {
    if (overrideLocale.empty() || overrideLocale == L"system") {
        const LANGID language = GetUserDefaultUILanguage();
        return PRIMARYLANGID(language) == LANG_CHINESE ? L"zh-CN.json" : L"en-US.json";
    }
    if (overrideLocale == L"zh-CN")
        return L"zh-CN.json";
    if (overrideLocale == L"en-US")
        return L"en-US.json";
    return nullptr;
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
        if (value.empty() || !chinese.contains(key) || chinese.at(key).empty())
            return false;
    }
    static constexpr std::array requiredKeys{
        "language.hint",
        "language.selector",
        "language.option.system",
        "language.option.en-US",
        "language.option.zh-CN",
        "language.restart_required",
        "operation.status.idle",
        "operation.status.running",
        "operation.status.success",
        "operation.status.warning",
        "operation.status.failure",
        "operation.settings.operation_inventory",
        "operation.input_methods.refresh",
        "operation.input_methods.set_default",
        "operation.appearance.apply",
        "operation.appearance.reset",
        "operation.packages.refresh_local",
        "operation.packages.refresh_online",
        "operation.packages.install_update",
        "operation.packages.enable_disable",
        "operation.packages.remove",
        "operation.diagnostics.recheck",
        "operation.diagnostics.repair",
        "packages.official_unconfigured",
        "packages.missing_key",
        "packages.trust_failed",
        "packages.revoked_key",
        "packages.rollback_blocked",
        "packages.state.bundled",
        "packages.state.disabled",
        "packages.state.enabled",
        "packages.state.update_available",
        "packages.state.available_online",
        "packages.state.trust_failed",
        "packages.state.incompatible",
        "packages.state.pending_restart",
        "packages.state.unavailable",
        "dialog.reset_appearance.title",
        "dialog.reset_appearance.body",
        "dialog.remove_package.title",
        "dialog.remove_package.body",
        "dialog.repair.title",
        "dialog.repair.body",
        "dialog.language_restart.title",
        "dialog.language_restart.body",
        "dialog.trust_failure.title",
        "dialog.trust_failure.body",
        "dialog.button.ok",
        "dialog.button.cancel",
        "dialog.button.continue"};
    for (const auto* key : requiredKeys) {
        if (!english.contains(key) || !chinese.contains(key) || english.at(key).empty() ||
            chinese.at(key).empty())
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
constexpr int kMaxWidth = 120;
constexpr int kScrollCellWidth = 121;
constexpr int kFontSize = 122;
constexpr int kCornerRadius = 123;
constexpr int kShadow = 124;
constexpr int kThemeLibrary = 125;
constexpr int kThemeDetail = 126;
constexpr int kPackageDetail = 127;
constexpr int kOpacity = 128;
constexpr int kPreeditMode = 129;
constexpr int kNavGeneral = 130;
constexpr int kNavAppearance = 131;
constexpr int kNavTheme = 132;
constexpr int kNavDiagnostics = 133;
constexpr int kNavRepair = 134;
constexpr int kNavPackages = 135;
constexpr int kAppearanceAdvanced = 136;
constexpr int kResetAppearance = 137;
constexpr int kAutomatic = 138;
constexpr int kPageTitle = 140;
constexpr int kInputMethodLabel = 200;
constexpr int kAppearanceLabel = 201;
constexpr int kThemeLabel = 202;
constexpr int kFontLabel = 203;
constexpr int kLayoutLabel = 204;
constexpr int kPackagesTitle = 205;
constexpr int kSaveStatus = 206;
constexpr int kPageSizeLabel = 207;
constexpr int kMaxWidthLabel = 208;
constexpr int kScrollCellWidthLabel = 209;
constexpr int kFontSizeLabel = 210;
constexpr int kCornerRadiusLabel = 211;
constexpr int kThemeLibraryLabel = 212;
constexpr int kOpacityLabel = 213;
constexpr int kPreeditModeLabel = 214;
constexpr int kBrandIcon = 215;
constexpr int kBrandText = 216;

// Transient notices ("保存成功" / 命令错误 / 重启完成 / 修复已开始) are
// cleared automatically a few seconds after they appear.
constexpr UINT_PTR kStatusTimerId = 0x4A44U;
constexpr UINT kStatusTimeoutMs = 3000;

struct DesignTokens {
    int navigationWidth{204};
    int contentLeft{238};
    int rowHeight{34};
    int controlHeight{30};
    int hitTarget{34};
    float cornerRadius{14.0F};
    float focusStroke{1.0F};
    float animationBudgetMs{120.0F};
    COLORREF appBackground{RGB(247, 248, 250)};
    COLORREF navigationBackground{RGB(233, 235, 239)};
    COLORREF surface{RGB(255, 255, 255)};
    COLORREF text{RGB(48, 50, 54)};
    COLORREF subtleText{RGB(63, 66, 71)};
    COLORREF accent{RGB(0, 122, 82)};
    COLORREF focus{RGB(0, 95, 184)};
};

enum class ModernAction {
    none,
    navGeneral,
    navAppearance,
    navShortcuts,
    navUpdates,
    navRepair,
    navPackages,
    toggleStartup,
    inputMethodRefresh,
    inputMethodCard,
    selectModeSystem,
    selectModeLight,
    selectModeDark,
    selectLayoutAutomatic,
    selectLayoutVertical,
    selectLayoutHorizontal,
    cyclePageSize,
    cycleMaxWidth,
    cycleScrollCellWidth,
    cycleFontSize,
    cycleCornerRadius,
    cycleOpacity,
    cyclePreeditMode,
    toggleShadow,
    toggleScrollMode,
    editFont,
    preview,
    resetAppearance,
    packageRefresh,
    packageInstallOrUpdate,
    packageToggle,
    packageRemove,
    diagnostics,
    repair,
    toggleTechnicalDetails,
    packageCard
};

struct ModernHitTarget {
    RECT rect{};
    ModernAction action{ModernAction::none};
    int index{-1};
};

bool highContrastEnabled() noexcept {
    HIGHCONTRASTW contrast{};
    contrast.cbSize = sizeof(contrast);
    return SystemParametersInfoW(SPI_GETHIGHCONTRAST, sizeof(contrast), &contrast, 0) &&
           (contrast.dwFlags & HCF_HIGHCONTRASTON) != 0;
}

DesignTokens designTokens() noexcept {
    DesignTokens tokens;
    if (highContrastEnabled()) {
        tokens.appBackground = GetSysColor(COLOR_WINDOW);
        tokens.navigationBackground = GetSysColor(COLOR_WINDOW);
        tokens.surface = GetSysColor(COLOR_WINDOW);
        tokens.text = GetSysColor(COLOR_WINDOWTEXT);
        tokens.subtleText = GetSysColor(COLOR_WINDOWTEXT);
        tokens.accent = GetSysColor(COLOR_HIGHLIGHT);
        tokens.focus = GetSysColor(COLOR_HIGHLIGHT);
    }
    return tokens;
}

D2D1::ColorF d2dColor(COLORREF color) noexcept {
    return D2D1::ColorF(GetRValue(color) / 255.0F, GetGValue(color) / 255.0F,
                        GetBValue(color) / 255.0F);
}

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

struct ThemeRow {
    std::wstring id;
    std::wstring source;
    std::wstring name;
    std::wstring version;
    std::wstring license;
    std::wstring description;
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
                   bool& repositoryAvailable, std::wstring& repositoryError) {
    try {
        const auto document = nlohmann::json::parse(narrow(output));
        if (!document.is_object() || (document.size() != 3U && document.size() != 4U) ||
            document.at("format_version") != 1 ||
            !document.at("repository_available").is_boolean() ||
            !document.at("packages").is_array() || document.at("packages").size() > 4096U)
            return false;
        repositoryAvailable = document.at("repository_available").get<bool>();
        repositoryError.clear();
        if (document.contains("repository_error") && !document.at("repository_error").is_null()) {
            if (!document.at("repository_error").is_string())
                return false;
            repositoryError = widen(document.at("repository_error").get<std::string>());
            if (repositoryError.size() > 64U)
                return false;
        }
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
            if (!repositoryAvailable && row.installed.empty() && !row.available.empty())
                continue;
            if (!repositoryAvailable) {
                row.available.clear();
                row.update = false;
            }
            rows.push_back(std::move(row));
        }
        return true;
    } catch (const nlohmann::json::exception&) {
        return false;
    }
}

bool parseThemes(std::wstring_view output, std::vector<ThemeRow>& rows) {
    try {
        const auto document = nlohmann::json::parse(narrow(output));
        if (!document.is_object() || document.size() != 2U || document.at("format_version") != 1 ||
            !document.at("themes").is_array() || document.at("themes").size() > 1024U)
            return false;
        rows.clear();
        for (const auto& item : document.at("themes")) {
            if (!item.is_object() || item.size() != 6U)
                return false;
            ThemeRow row;
            row.id = widen(item.at("id").get<std::string>());
            row.source = widen(item.at("source").get<std::string>());
            row.name = widen(item.at("name").get<std::string>());
            row.version = widen(item.at("version").get<std::string>());
            row.license = widen(item.at("license").get<std::string>());
            row.description = widen(item.at("description").get<std::string>());
            if (row.id.empty() || row.source.empty() || row.name.empty())
                return false;
            rows.push_back(std::move(row));
        }
        return true;
    } catch (const nlohmann::json::exception&) {
        return false;
    }
}

void addUniqueFontFamily(std::vector<std::wstring>& fonts, std::wstring_view family) {
    if (family.empty() || family.front() == L'@')
        return;
    const std::wstring candidate(family);
    for (const auto& font : fonts) {
        if (_wcsicmp(font.c_str(), candidate.c_str()) == 0)
            return;
    }
    fonts.push_back(candidate);
}

int CALLBACK collectFontFamily(const LOGFONTW* logFont, const TEXTMETRICW*, DWORD, LPARAM data) {
    auto* fonts = reinterpret_cast<std::vector<std::wstring>*>(data);
    if (!fonts || !logFont)
        return 0;
    addUniqueFontFamily(*fonts, logFont->lfFaceName);
    return fonts->size() < 512U ? 1 : 0;
}

std::vector<std::wstring> enumerateFontFamilies(HWND owner) {
    std::vector<std::wstring> discovered;
    HDC dc = owner ? ::GetDC(owner) : nullptr;
    if (dc) {
        LOGFONTW query{};
        query.lfCharSet = DEFAULT_CHARSET;
        EnumFontFamiliesExW(dc, &query, collectFontFamily,
                            reinterpret_cast<LPARAM>(&discovered), 0);
        ::ReleaseDC(owner, dc);
    }
    std::sort(discovered.begin(), discovered.end(),
              [](const std::wstring& left, const std::wstring& right) {
                  return _wcsicmp(left.c_str(), right.c_str()) < 0;
              });

    std::vector<std::wstring> ordered;
    for (const wchar_t* preset : {L"Microsoft YaHei", L"Segoe UI", L"Segoe UI Emoji",
                                  L"Noto Sans CJK SC", L"Cascadia Mono", L"Consolas"}) {
        for (const auto& font : discovered) {
            if (_wcsicmp(font.c_str(), preset) == 0) {
                addUniqueFontFamily(ordered, font);
                break;
            }
        }
    }
    for (const auto& font : discovered)
        addUniqueFontFamily(ordered, font);
    if (ordered.empty())
        addUniqueFontFamily(ordered, L"Segoe UI");
    return ordered;
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

bool repositoryErrorIsMissingKey(std::wstring_view error) {
    return error == L"missing_key" || error == L"invalid_keyring" || error == L"untrusted_key";
}

bool repositoryErrorIsRollback(std::wstring_view error) {
    return error == L"rollback_rejected" || error == L"sequence_state_corrupt" ||
           error == L"sequence_state_missing";
}

bool repositoryErrorIsSignatureFailure(std::wstring_view error) {
    return error == L"invalid_signature" || error == L"invalid_repository";
}

std::wstring repositoryTrustMessage(bool repositoryAvailable, std::wstring_view repositoryError,
                                    const Strings& strings) {
    if (repositoryAvailable)
        return L"";
    if (repositoryErrorIsMissingKey(repositoryError))
        return localeValue(strings, "packages.missing_key",
                           L"Official repository signing key is missing or not trusted.");
    if (repositoryError == L"revoked_key")
        return localeValue(strings, "packages.revoked_key",
                           L"The repository or package was signed by a revoked key.");
    if (repositoryErrorIsRollback(repositoryError))
        return localeValue(strings, "packages.rollback_blocked",
                           L"Repository rollback protection blocked this metadata.");
    if (repositoryErrorIsSignatureFailure(repositoryError))
        return localeValue(strings, "packages.trust_failed",
                           L"Repository or package signature verification failed.");
    return localeValue(strings, "packages.official_unconfigured",
                       L"Official add-on repository is not configured yet.");
}

std::wstring packageStateLabel(const PackageRow& package, const Strings& strings,
                               bool repositoryAvailable = true) {
    if (package.state == L"bundled")
        return localeValue(strings, "packages.state.bundled", L"Bundled");
    if (package.state == L"disabled")
        return localeValue(strings, "packages.state.disabled", L"Disabled");
    if (package.state == L"incompatible")
        return localeValue(strings, "packages.state.incompatible", L"Incompatible");
    if (package.state == L"pending-restart")
        return localeValue(strings, "packages.state.pending_restart", L"Restart required");
    if (package.state == L"trust-failed")
        return localeValue(strings, "packages.state.trust_failed", L"Trust failed");
    if (repositoryAvailable && package.update)
        return localeValue(strings, "packages.state.update_available", L"Update available");
    if (repositoryAvailable && package.installed.empty() && !package.available.empty())
        return localeValue(strings, "packages.state.available_online", L"Available online");
    if (!package.installed.empty())
        return localeValue(strings, "packages.state.enabled", L"Enabled");
    return localeValue(strings, "packages.state.unavailable", L"Unavailable");
}

bool packageUnsafeForInstall(const PackageRow& package, bool repositoryAvailable) {
    return package.state == L"incompatible" || package.state == L"trust-failed" ||
           package.state == L"pending-restart" ||
           (!repositoryAvailable && !package.available.empty());
}

bool packageUnsafeForInstalledAction(const PackageRow& package) {
    return package.state == L"bundled" || package.state == L"trust-failed" ||
           package.state == L"incompatible" || package.state == L"pending-restart";
}

std::wstring packageBlockedActionMessage(const PackageRow& package, const Strings& strings,
                                         bool repositoryAvailable,
                                         std::wstring_view repositoryError) {
    if (package.state == L"bundled")
        return localeValue(strings, "packages.bundled_readonly",
                           L"This component is bundled and tested with the whole product.");
    if (package.state == L"trust-failed")
        return localeValue(strings, "packages.trust_failed",
                           L"Repository or package signature verification failed.");
    if (package.state == L"incompatible" || package.state == L"pending-restart")
        return packageStateLabel(package, strings, repositoryAvailable);
    if (!repositoryAvailable)
        return repositoryTrustMessage(repositoryAvailable, repositoryError, strings);
    return packageStateLabel(package, strings, repositoryAvailable);
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

std::wstring themeSourceLabel(const ThemeRow& theme, const Strings& strings) {
    if (theme.source == L"builtin")
        return localeValue(strings, "theme.source.builtin", L"Built-in");
    if (theme.source == L"user")
        return localeValue(strings, "theme.source.user", L"User");
    if (theme.source == L"package")
        return localeValue(strings, "theme.source.package", L"Package");
    return theme.source;
}

std::wstring themeListLabel(const ThemeRow& theme, const Strings& strings) {
    std::wstring label = L"[" + themeSourceLabel(theme, strings) + L"] " + theme.name;
    if (!theme.version.empty())
        label += L"  " + theme.version;
    return label;
}

std::wstring jsonStringOrNull(const nlohmann::json& object, const char* key) {
    return object.contains(key) && !object.at(key).is_null()
               ? widen(object.at(key).get<std::string>())
               : std::wstring{};
}

std::wstring jsonArraySummary(const nlohmann::json& array, std::wstring_view empty) {
    if (!array.is_array() || array.empty())
        return std::wstring(empty);
    std::wstring output;
    for (const auto& item : array) {
        if (!output.empty())
            output += L", ";
        if (item.is_string()) {
            output += widen(item.get<std::string>());
        } else if (item.is_object() && item.contains("id") && item.at("id").is_string()) {
            output += widen(item.at("id").get<std::string>());
            if (item.contains("version") && item.at("version").is_string())
                output += L" " + widen(item.at("version").get<std::string>());
        }
    }
    return output.empty() ? std::wstring(empty) : output;
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
        const int desiredWidth = (std::min)(scale(1100), (std::max)(scale(860), workWidth - 80));
        const int desiredHeight = (std::min)(scale(720), (std::max)(scale(600), workHeight - 80));
        ResizeClient(desiredWidth, desiredHeight);
        layoutControls();
    }

    [[nodiscard]] bool verifyUiContract() {
        uiContractTest_ = true;
        livePreviewContractTest_ = true;
        const auto hasVisibleStyle = [&](int id) {
            const HWND child = control(id);
            return child &&
                   (::GetWindowLongPtrW(child, GWL_STYLE) &
                    static_cast<LONG_PTR>(WS_VISIBLE)) != 0;
        };
        const auto pageMatches = [&](int page, bool apply, bool saveStatus, bool details) {
            showPage(page);
            return hasVisibleStyle(kApply) == apply &&
                   hasVisibleStyle(kSaveStatus) == saveStatus &&
                   hasVisibleStyle(kStatus) == details;
        };
        if (!pageMatches(kNavGeneral, true, true, false) ||
            !pageMatches(kNavAppearance, false, true, false) ||
            !pageMatches(kNavTheme, false, false, true) ||
            !pageMatches(kNavDiagnostics, false, false, true) ||
            !pageMatches(kNavRepair, false, false, true) ||
            !pageMatches(kNavPackages, false, false, true)) {
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
        return status[0] != L'\0' && std::wstring_view(status.data()) == get("status.saved");
    }

    [[nodiscard]] bool verifyVisualContract() {
        legacyVisualContractTest_ = true;
        const auto tokens = designTokens();
        if (tokens.navigationWidth <= 0 || tokens.rowHeight < 32 ||
            tokens.hitTarget < 32 || tokens.cornerRadius <= 0.0F ||
            tokens.animationBudgetMs > 150.0F) {
            return false;
        }
        const auto textOf = [&](int id) {
            std::array<wchar_t, 128> text{};
            ::GetWindowTextW(control(id), text.data(), static_cast<int>(text.size()));
            return std::wstring(text.data());
        };
        if (textOf(kNavGeneral) != get("nav.general") ||
            textOf(kNavAppearance) != get("nav.appearance") ||
            textOf(kNavTheme) != get("nav.theme") ||
            textOf(kNavDiagnostics) != get("nav.diagnostics") ||
            textOf(kNavRepair) != get("nav.repair") ||
            textOf(kNavPackages) != get("nav.packages") ||
            textOf(kNavTheme) == L"Theme" || textOf(kNavRepair) == L"Repair") {
            return false;
        }
        for (int id = kNavGeneral; id <= kNavPackages; ++id) {
            const auto style = ::GetWindowLongPtrW(control(id), GWL_STYLE);
            if ((style & WS_TABSTOP) == 0 || (style & BS_OWNERDRAW) == 0 ||
                textOf(id).empty()) {
                return false;
            }
        }
        const auto visible = [&](int id) {
            const HWND child = control(id);
            return child &&
                   (::GetWindowLongPtrW(child, GWL_STYLE) & WS_VISIBLE) != 0;
        };
        const auto visibleRect = [&](int id, RECT& rectangle) {
            if (!visible(id) || !::GetWindowRect(control(id), &rectangle))
                return false;
            return rectangle.right > rectangle.left && rectangle.bottom > rectangle.top;
        };
        const auto intersects = [](const RECT& left, const RECT& right) {
            RECT intersection{};
            return ::IntersectRect(&intersection, &left, &right) &&
                   intersection.right > intersection.left &&
                   intersection.bottom > intersection.top;
        };
        const auto pageHasNoOverlaps = [&](int page, std::initializer_list<int> ids) {
            showPage(page);
            std::vector<std::pair<int, RECT>> rectangles;
            for (const int id : ids) {
                RECT rectangle{};
                if (!visibleRect(id, rectangle))
                    continue;
                rectangles.emplace_back(id, rectangle);
            }
            for (std::size_t outer = 0; outer < rectangles.size(); ++outer) {
                for (std::size_t inner = outer + 1; inner < rectangles.size(); ++inner) {
                    if (intersects(rectangles[outer].second, rectangles[inner].second)) {
                        std::cerr << "Config visual overlap: " << rectangles[outer].first
                                  << " intersects " << rectangles[inner].first << '\n';
                        return false;
                    }
                }
            }
            return true;
        };
        if (!visible(kBrandIcon)) {
            return false;
        }
        showPage(kNavAppearance);
        if (!visible(kAppearance) || !visible(kTheme) || !visible(kPreview) ||
            !visible(kResetAppearance) || visible(kApply) || visible(kMaxWidth) ||
            visible(kScrollCellWidth) || visible(kOpacity) || visible(kPreeditMode)) {
            return false;
        }
        if (!pageHasNoOverlaps(kNavGeneral,
                               {kPageTitle, kStartup, kInputMethodLabel, kInputMethod,
                                kApply, kSaveStatus}) ||
            !pageHasNoOverlaps(kNavAppearance,
                               {kPageTitle, kAppearanceLabel, kAppearance, kThemeLabel,
                                kTheme, kFontLabel, kFont, kLayoutLabel, kAutomatic,
                                kVertical, kHorizontal, kScrollMode, kPageSizeLabel,
                                kPageSize, kFontSizeLabel, kFontSize, kAppearanceAdvanced,
                                kPreview, kResetAppearance, kSaveStatus, kThemeLibraryLabel,
                                kThemeLibrary, kThemeDetail}) ||
            !pageHasNoOverlaps(kNavTheme, {kPageTitle, kStatus}) ||
            !pageHasNoOverlaps(kNavRepair,
                               {kPageTitle, kRestart, kDiagnostics, kRepair, kStatus}) ||
            !pageHasNoOverlaps(kNavDiagnostics,
                               {kPageTitle, kPackagesTitle, kPackages, kPackageDetail,
                                kPackageRefresh, kPackageInstall, kPackageToggle,
                                kPackageRemove, kStatus}) ||
            !pageHasNoOverlaps(kNavPackages,
                               {kPageTitle, kPackagesTitle, kPackages, kPackageDetail,
                                kPackageRefresh, kPackageInstall, kPackageToggle,
                                kPackageRemove, kStatus})) {
            return false;
        }
        showPage(kNavTheme);
        if (!visible(kStatus) || visible(kApply) || visible(kTheme)) {
            return false;
        }
        showPage(kNavRepair);
        if (!visible(kRestart) || !visible(kDiagnostics) || !visible(kRepair) ||
            !visible(kStatus)) {
            return false;
        }
        showPage(kNavDiagnostics);
        if (!visible(kPackages) || !visible(kPackageRefresh) || !visible(kPackageInstall)) {
            return false;
        }
        for (const UINT dpi : {96U, 120U, 144U, 192U}) {
            dpi_ = dpi;
            layoutControls();
            RECT rectangle{};
            if (!::GetWindowRect(control(kNavGeneral), &rectangle) ||
                rectangle.right <= rectangle.left ||
                rectangle.bottom <= rectangle.top) {
                return false;
            }
        }
        dpi_ = windowDpi();
        layoutControls();
        legacyVisualContractTest_ = false;
        const auto modernPageHidesRawControls = [&](int page) {
            showPage(page);
            for (const int id : {kNavGeneral, kNavAppearance, kNavTheme, kNavDiagnostics,
                                 kNavRepair, kNavPackages, kPageTitle, kBrandText, kStartup,
                                 kInputMethod, kInputMethodLabel, kAppearance, kAppearanceLabel,
                                 kTheme, kThemeLabel, kFont, kFontLabel, kThemeLibrary,
                                 kThemeLibraryLabel, kThemeDetail, kAutomatic, kVertical,
                                 kHorizontal, kLayoutLabel, kScrollMode, kPageSize, kPageSizeLabel,
                                 kApply, kMaxWidth, kMaxWidthLabel, kScrollCellWidth,
                                 kScrollCellWidthLabel, kFontSize, kFontSizeLabel, kCornerRadius,
                                 kCornerRadiusLabel, kShadow, kOpacity, kOpacityLabel,
                                 kPreeditMode, kPreeditModeLabel, kAppearanceAdvanced, kPreview,
                                 kResetAppearance, kRestart, kDiagnostics, kRepair, kStatus,
                                 kPackages, kPackageDetail, kPackageRefresh, kPackageInstall,
                                 kPackageToggle, kPackageRemove, kPackagesTitle, kSaveStatus}) {
                if (visible(id)) {
                    std::cerr << "Modern Config surface leaked raw HWND control " << id
                              << " on page " << page << '\n';
                    return false;
                }
            }
            return visible(kBrandIcon);
        };
        const auto textAreasDoNotOverlap = [&](std::initializer_list<RECT> areas) {
            std::vector<RECT> rectangles(areas);
            for (std::size_t outer = 0; outer < rectangles.size(); ++outer) {
                for (std::size_t inner = outer + 1; inner < rectangles.size(); ++inner) {
                    if (intersects(rectangles[outer], rectangles[inner])) {
                        std::cerr << "Modern Config text/display areas overlap: area "
                                  << outer << " [" << rectangles[outer].left << ','
                                  << rectangles[outer].top << ','
                                  << rectangles[outer].right << ','
                                  << rectangles[outer].bottom << "] intersects area "
                                  << inner << " [" << rectangles[inner].left << ','
                                  << rectangles[inner].top << ','
                                  << rectangles[inner].right << ','
                                  << rectangles[inner].bottom << "]\n";
                        return false;
                    }
                }
            }
            return true;
        };
        const auto rowText = [&](int y) {
            const int rowRight = static_cast<int>(modernRowRight());
            const int valueLeft = (std::max)(560, rowRight - 230);
            const int textRight = valueLeft - 20;
            return std::array<RECT, 3>{logicalHitRect(310, y + 10, textRight, y + 32),
                                       logicalHitRect(310, y + 34, textRight, y + 56),
                                       logicalHitRect(valueLeft, y + 18, rowRight - 22,
                                                      y + 44)};
        };
        const auto candidatePreviewAreasDoNotOverlap = [&](bool verticalLayout) {
            const int rowRight = static_cast<int>(modernRowRight());
            const auto labelBox = [&](int left, int top) {
                return logicalHitRect(left, top + 3, left + 22, top + 23);
            };
            const auto candidateBox = [&](int left, int top, int right) {
                return logicalHitRect(left + 28, top, right, top + 28);
            };
            if (verticalLayout) {
                return textAreasDoNotOverlap(
                    {logicalHitRect(316, 162, rowRight - 232, 188),
                     logicalHitRect(rowRight - 216, 160, rowRight - 132, 182),
                     logicalHitRect(rowRight - 124, 160, rowRight - 18, 182),
                     labelBox(316, 192), candidateBox(316, 192, 438),
                     labelBox(316, 224), candidateBox(316, 224, 438),
                     labelBox(464, 192), candidateBox(464, 192, 610),
                     labelBox(464, 224), candidateBox(464, 224, 630)});
            }
            return textAreasDoNotOverlap(
                {logicalHitRect(316, 162, rowRight - 232, 188),
                 logicalHitRect(rowRight - 216, 160, rowRight - 132, 182),
                 logicalHitRect(rowRight - 124, 160, rowRight - 18, 182),
                 labelBox(316, 192), candidateBox(316, 192, 388),
                 labelBox(400, 192), candidateBox(400, 192, 500),
                 labelBox(514, 192), candidateBox(514, 192, 596),
                 labelBox(608, 192), candidateBox(608, 192, 720),
                 labelBox(732, 192), candidateBox(732, 192, rowRight - 18)});
        };
        const auto advancedCompactAreasDoNotOverlap = [&]() {
            const int rowRight = static_cast<int>(modernRowRight());
            const int cardWidth = (rowRight - 288 - 24) / 3;
            const int x0 = 288;
            const int x1 = x0 + cardWidth + 12;
            const int x2 = x1 + cardWidth + 12;
            const auto cardTitle = [&](int left, int top, int right) {
                return logicalHitRect(left + 14, top + 8, right - 14, top + 25);
            };
            const auto cardValue = [&](int left, int top, int right) {
                return logicalHitRect(left + 14, top + 26, right - 14, top + 46);
            };
            return textAreasDoNotOverlap(
                {logicalHitRect(288, 270, rowRight, 298),
                 cardTitle(x0, 304, x0 + cardWidth), cardValue(x0, 304, x0 + cardWidth),
                 cardTitle(x1, 304, x1 + cardWidth), cardValue(x1, 304, x1 + cardWidth),
                 cardTitle(x2, 304, rowRight), cardValue(x2, 304, rowRight),
                 cardTitle(x0, 366, x0 + cardWidth), cardValue(x0, 366, x0 + cardWidth),
                 cardTitle(x1, 366, x1 + cardWidth), cardValue(x1, 366, x1 + cardWidth),
                 cardTitle(x2, 366, rowRight), cardValue(x2, 366, rowRight),
                 cardTitle(x0, 428, x0 + cardWidth), cardValue(x0, 428, x0 + cardWidth),
                 cardTitle(x1, 428, x1 + cardWidth), cardValue(x1, 428, x1 + cardWidth),
                 cardTitle(x2, 428, rowRight), cardValue(x2, 428, rowRight),
                 cardTitle(x0, 490, x1 + cardWidth), cardValue(x0, 490, x1 + cardWidth),
                 logicalHitRect(x0, 552, rowRight, 584)});
        };
        ResizeClient(scale(860), scale(600));
        layoutControls();
        for (const int page : {kNavGeneral, kNavAppearance, kNavTheme, kNavDiagnostics,
                               kNavRepair, kNavPackages}) {
            if (!modernPageHidesRawControls(page))
                return false;
        }
        const auto general148 = rowText(148);
        const auto general282 = rowText(282);
        if (!textAreasDoNotOverlap({logicalHitRect(288, 64,
                                                   static_cast<int>(modernRowRight()), 102),
                                    logicalHitRect(288, 114,
                                                   static_cast<int>(modernRowRight()), 142),
                                    general148[0], general148[1], general148[2],
                                    logicalHitRect(288, 248,
                                                   static_cast<int>(modernRowRight()), 276),
                                    general282[0], general282[1], general282[2]}))
            return false;
        if (!candidatePreviewSampleCoversRequiredContent() ||
            !candidatePreviewAreasDoNotOverlap(false) ||
            !candidatePreviewAreasDoNotOverlap(true) ||
            !advancedCompactAreasDoNotOverlap()) {
            return false;
        }
        const auto appearance476 = rowText(476);
        const auto appearance552 = rowText(552);
        const auto appearance628 = rowText(628);
        if (!textAreasDoNotOverlap({logicalHitRect(288, 64,
                                                   static_cast<int>(modernRowRight()), 102),
                                    logicalHitRect(288, 114,
                                                   static_cast<int>(modernRowRight()), 142),
                                    logicalHitRect(288, 270,
                                                   static_cast<int>(modernRowRight()), 298),
                                    logicalHitRect(288, 346,
                                                   static_cast<int>(modernRowRight()), 374),
                                    logicalHitRect(288, 374, 920, 396),
                                    appearance476[0], appearance476[1], appearance476[2],
                                    appearance552[0], appearance552[1], appearance552[2],
                                    appearance628[0], appearance628[1], appearance628[2]}))
            return false;
        const auto update148 = rowText(148);
        const auto update224 = rowText(224);
        const auto update300 = rowText(300);
        const auto update376 = rowText(376);
        const auto update452 = rowText(452);
        const auto update528 = rowText(528);
        if (!textAreasDoNotOverlap({logicalHitRect(288, 64,
                                                   static_cast<int>(modernRowRight()), 102),
                                    logicalHitRect(288, 114,
                                                   static_cast<int>(modernRowRight()), 142),
                                    update148[0], update148[1], update148[2],
                                    update224[0], update224[1], update224[2],
                                    update300[0], update300[1], update300[2],
                                    update376[0], update376[1], update376[2],
                                    update452[0], update452[1], update452[2],
                                    update528[0], update528[1], update528[2]}))
            return false;
        const auto package148 = rowText(148);
        const auto package224 = rowText(224);
        const auto package300 = rowText(300);
        const auto package396 = rowText(396);
        const auto package472 = rowText(472);
        const auto package548 = rowText(548);
        const auto package624 = rowText(624);
        if (!textAreasDoNotOverlap({logicalHitRect(288, 64,
                                                   static_cast<int>(modernRowRight()), 102),
                                    logicalHitRect(288, 114,
                                                   static_cast<int>(modernRowRight()), 142),
                                    package148[0], package148[1], package148[2],
                                    package224[0], package224[1], package224[2],
                                    package300[0], package300[1], package300[2],
                                    logicalHitRect(288, 372,
                                                   static_cast<int>(modernRowRight()), 400),
                                    package396[0], package396[1], package396[2],
                                    package472[0], package472[1], package472[2],
                                    package548[0], package548[1], package548[2],
                                    package624[0], package624[1], package624[2]}))
            return false;
        legacyVisualContractTest_ = true;
        return true;
    }

    [[nodiscard]] bool verifyLivePreviewContract() {
        livePreviewContractTest_ = true;
        previewLaunchCount_ = 0;
        liveApplyCount_ = 0;
        resetApplyCount_ = 0;
        forceLiveApplyFailure_ = false;
        appearanceAdvanced_ = false;
        SendMessageW(control(kAppearanceAdvanced), BM_SETCHECK, BST_UNCHECKED, 0);
        showPage(kNavAppearance);
        if (previewLaunchCount_ != 1 || !previewActiveForContract_ || visible(kApply) ||
            visible(kMaxWidth) || visible(kScrollCellWidth) || visible(kOpacity) ||
            visible(kPreeditMode)) {
            std::cerr << "live preview default surface failed: launches="
                      << previewLaunchCount_ << " active=" << previewActiveForContract_
                      << " apply=" << visible(kApply) << " max=" << visible(kMaxWidth)
                      << " scrollCell=" << visible(kScrollCellWidth) << '\n';
            return false;
        }
        for (const int id : {kAppearance, kTheme, kFont, kAutomatic, kPageSize, kFontSize,
                             kAppearanceAdvanced, kPreview, kResetAppearance}) {
            const HWND child = control(id);
            if (!child || (::GetWindowLongPtrW(child, GWL_STYLE) & WS_TABSTOP) == 0) {
                std::cerr << "live preview tabstop missing for " << id << '\n';
                return false;
            }
        }
        const auto notify = [&](int id, int notification) {
            SendMessageW(WM_COMMAND, MAKEWPARAM(id, notification),
                         reinterpret_cast<LPARAM>(control(id)));
        };
        const auto statusIs = [&](const char* key) {
            std::array<wchar_t, 128> status{};
            ::GetWindowTextW(control(kSaveStatus), status.data(),
                             static_cast<int>(status.size()));
            return std::wstring_view(status.data()) == get(key);
        };
        SendMessageW(control(kAppearance), CB_SETCURSEL, 1, 0);
        notify(kAppearance, CBN_SELCHANGE);
        SendMessageW(control(kFontSize), CB_SETCURSEL, 2, 0);
        notify(kFontSize, CBN_SELCHANGE);
        SendMessageW(control(kHorizontal), BM_CLICK, 0, 0);
        if (liveApplyCount_ != 3 || previewLaunchCount_ != 1 ||
            presentationDirty_ || !statusIs("status.saved")) {
            std::cerr << "live apply did not save without relaunch: applies="
                      << liveApplyCount_ << " launches=" << previewLaunchCount_
                      << " dirty=" << presentationDirty_ << '\n';
            return false;
        }
        SendMessageW(control(kAppearanceAdvanced), BM_CLICK, 0, 0);
        if (!appearanceAdvanced_ || !visible(kMaxWidth) || !visible(kScrollCellWidth) ||
            !visible(kOpacity) || !visible(kPreeditMode)) {
            std::cerr << "advanced appearance controls did not expand\n";
            return false;
        }
        forceLiveApplyFailure_ = true;
        SendMessageW(control(kFont), CB_SETCURSEL, 0, 0);
        notify(kFont, CBN_SELCHANGE);
        if (!presentationDirty_ || !statusIs("error.command") || previewLaunchCount_ != 1) {
            std::cerr << "invalid live apply did not fail safe\n";
            return false;
        }
        forceLiveApplyFailure_ = false;
        SendMessageW(control(kResetAppearance), BM_CLICK, 0, 0);
        if (resetApplyCount_ != 1 || presentationDirty_ || !statusIs("status.saved") ||
            previewLaunchCount_ != 1) {
            std::cerr << "appearance reset did not save without relaunch: resets="
                      << resetApplyCount_ << " dirty=" << presentationDirty_
                      << " launches=" << previewLaunchCount_ << '\n';
            return false;
        }
        BOOL handled = FALSE;
        (void)onVisualSystemChanged(WM_SYSCOLORCHANGE, 0, 0, handled);
        (void)onVisualSystemChanged(WM_THEMECHANGED, 0, 0, handled);
        return designTokens().hitTarget >= 32;
    }

    [[nodiscard]] bool verifyInteractionCoverage() {
        interactionTest_ = true;
        showPage(kNavGeneral);
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
                    inputMethods_ = {{L"test", L"Test", L"Test", true},
                                     {L"rime", L"Rime", L"中州韵", false}};
                    SendMessageW(control(kInputMethod), CB_RESETCONTENT, 0, 0);
                    for (const auto& method : inputMethods_) {
                        const std::wstring label =
                            method.nativeName.empty() || method.nativeName == method.name
                                ? method.name
                                : method.nativeName + L" (" + method.name + L")";
                        SendMessageW(control(kInputMethod), CB_ADDSTRING, 0,
                                     reinterpret_cast<LPARAM>(label.c_str()));
                    }
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

        inputMethods_ = {{L"test", L"Test", L"Test", true},
                         {L"rime", L"Rime", L"中州韵", false}};
        SendMessageW(control(kInputMethod), CB_RESETCONTENT, 0, 0);
        SendMessageW(control(kInputMethod), CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"Test"));
        SendMessageW(control(kInputMethod), CB_ADDSTRING, 0,
                     reinterpret_cast<LPARAM>(L"中州韵 (Rime)"));
        SendMessageW(control(kInputMethod), CB_SETCURSEL, 0, 0);
        invokeModernAction(ModernHitTarget{{}, ModernAction::inputMethodRefresh});
        invokeModernAction(ModernHitTarget{{}, ModernAction::inputMethodCard, 1});
        if (SendMessageW(control(kInputMethod), CB_GETCURSEL, 0, 0) != 1 ||
            !statusIs("status.saved") || !inputMethods_[1].selected || inputMethods_[0].selected)
            return false;

        if (!click(kNavAppearance))
            return false;
        const LRESULT appearanceCount = SendMessageW(control(kAppearance), CB_GETCOUNT, 0, 0);
        for (LRESULT index = 0; index < appearanceCount; ++index) {
            SendMessageW(control(kAppearance), CB_SETCURSEL, index, 0);
            if (!notify(kAppearance, CBN_SELCHANGE))
                return false;
        }
        if (appearanceCount != 3 || !click(kAutomatic) || !click(kVertical) ||
            !click(kHorizontal))
            return false;
        if (!click(kVertical))
            return false;
        if (!click(kScrollMode))
            return false;
        if (!click(kScrollMode))
            return false;
        if (!click(kAppearanceAdvanced))
            return false;
        for (const int combo : {kPageSize, kMaxWidth, kScrollCellWidth, kFontSize,
                                kCornerRadius}) {
            const LRESULT count = SendMessageW(control(combo), CB_GETCOUNT, 0, 0);
            if (count <= 0)
                return false;
            for (LRESULT index = 0; index < count; ++index) {
                SendMessageW(control(combo), CB_SETCURSEL, index, 0);
                if (!notify(combo, CBN_SELCHANGE))
                    return false;
            }
        }
        if (!click(kShadow))
            return false;
        if (!click(kShadow))
            return false;
        if (!statusIs("status.saved") || !click(kResetAppearance) ||
            !statusIs("status.saved"))
            return false;

        const LRESULT fontSizeBefore = SendMessageW(control(kFontSize), CB_GETCURSEL, 0, 0);
        invokeModernAction(ModernHitTarget{{}, ModernAction::cycleFontSize});
        const LRESULT fontSizeAfter = SendMessageW(control(kFontSize), CB_GETCURSEL, 0, 0);
        if (fontSizeBefore == CB_ERR || fontSizeAfter == CB_ERR ||
            fontSizeAfter == fontSizeBefore)
            return false;
        invokeModernAction(ModernHitTarget{{}, ModernAction::editFont});
        if (!visible(kFont))
            return false;
        const LRESULT fontCount = SendMessageW(control(kFont), CB_GETCOUNT, 0, 0);
        if (fontCount <= 0)
            return false;
        SendMessageW(control(kFont), CB_SETCURSEL, fontCount > 1 ? 1 : 0, 0);
        if (!notify(kFont, CBN_SELCHANGE) || visible(kFont))
            return false;

        if (!click(kNavTheme))
            return false;
        std::array<wchar_t, 256> shortcutsText{};
        ::GetWindowTextW(control(kStatus), shortcutsText.data(),
                         static_cast<int>(shortcutsText.size()));
        if (std::wstring_view(shortcutsText.data()).find(get("nav.theme")) ==
            std::wstring_view::npos) {
            return false;
        }
        if (!click(kNavAppearance))
            return false;
        const LRESULT themeCount = SendMessageW(control(kTheme), CB_GETCOUNT, 0, 0);
        for (LRESULT index = 0; index < themeCount; ++index) {
            SendMessageW(control(kTheme), CB_SETCURSEL, index, 0);
            if (!notify(kTheme, CBN_SELCHANGE))
                return false;
        }
        const LRESULT themeLibraryCount = SendMessageW(control(kThemeLibrary), LB_GETCOUNT, 0, 0);
        if (themeLibraryCount <= 0)
            return false;
        for (LRESULT index = 0; index < themeLibraryCount; ++index) {
            SendMessageW(control(kThemeLibrary), LB_SETCURSEL, index, 0);
            if (!notify(kThemeLibrary, LBN_SELCHANGE))
                return false;
        }
        std::array<wchar_t, 512> themeDetail{};
        ::GetWindowTextW(control(kThemeDetail), themeDetail.data(),
                         static_cast<int>(themeDetail.size()));
        if (std::wstring_view(themeDetail.data()).find(L"ID:") == std::wstring::npos)
            return false;
        if (fontCount <= 0)
            return false;
        SendMessageW(control(kFont), CB_SETCURSEL, 0, 0);
        if (!notify(kFont, CBN_SELCHANGE))
            return false;
        if (themeCount <= 0 || !click(kPreview) || !statusIs("status.saved"))
            return false;

        if (!click(kNavDiagnostics) || !click(kPackageRefresh) ||
            !click(kNavRepair) || !click(kRestart) || !click(kDiagnostics) ||
            !click(kRepair) || !click(kNavPackages) || !click(kPackageRefresh))
            return false;

        // Exercise each package action through a synthetic managed row. Production package
        // transaction semantics are covered separately with a signed fixture repository.
        packages_ = {{L"test-addon", L"Test addon", L"Test summary", L"addon",
                      L"1.1.0", L"", L"", false}};
        repositoryAvailable_ = true;
        repositoryError_.clear();
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
        invokeModernAction(ModernHitTarget{{}, ModernAction::packageInstallOrUpdate});
        packages_[0].installed = L"1.0.0";
        packages_[0].update = true;
        packages_[0].state = L"enabled";
        updatePackageActions();
        if (!click(kPackageInstall) || !click(kPackageToggle))
            return false;
        invokeModernAction(ModernHitTarget{{}, ModernAction::packageInstallOrUpdate});
        invokeModernAction(ModernHitTarget{{}, ModernAction::packageToggle});
        packages_[0].state = L"disabled";
        if (!click(kPackageToggle) || !click(kPackageRemove))
            return false;
        invokeModernAction(ModernHitTarget{{}, ModernAction::packageToggle});
        invokeModernAction(ModernHitTarget{{}, ModernAction::packageRemove});
        PackageRow stateProbe{L"probe", L"Probe", L"", L"addon", L"1.1.0", L"", L"", false};
        repositoryAvailable_ = false;
        if (!packageUnsafeForInstall(stateProbe, repositoryAvailable_) ||
            packageStateLabel(stateProbe, strings_, repositoryAvailable_) !=
                localeValue(strings_, "packages.state.unavailable", L"Unavailable") ||
            !repositoryTrustMessage(false, L"invalid_keyring", strings_).starts_with(
                localeValue(strings_, "packages.missing_key",
                            L"Official repository signing key is missing or not trusted.")) ||
            repositoryTrustMessage(false, L"invalid_signature", strings_) !=
                localeValue(strings_, "packages.trust_failed",
                            L"Repository or package signature verification failed.") ||
            repositoryTrustMessage(false, L"revoked_key", strings_) !=
                localeValue(strings_, "packages.revoked_key",
                            L"The repository or package was signed by a revoked key.") ||
            repositoryTrustMessage(false, L"rollback_rejected", strings_) !=
                localeValue(strings_, "packages.rollback_blocked",
                            L"Repository rollback protection blocked this metadata."))
            return false;
        repositoryAvailable_ = true;
        if (packageStateLabel(stateProbe, strings_, repositoryAvailable_) !=
                localeValue(strings_, "packages.state.available_online", L"Available online"))
            return false;
        stateProbe.installed = L"1.0.0";
        stateProbe.update = true;
        if (packageStateLabel(stateProbe, strings_, repositoryAvailable_) !=
            localeValue(strings_, "packages.state.update_available", L"Update available"))
            return false;
        for (const auto& [state, key] :
             std::initializer_list<std::pair<std::wstring_view, const char*>>{
                 {L"bundled", "packages.state.bundled"},
                 {L"disabled", "packages.state.disabled"},
                 {L"trust-failed", "packages.state.trust_failed"},
                 {L"incompatible", "packages.state.incompatible"},
                 {L"pending-restart", "packages.state.pending_restart"}}) {
            stateProbe.state = std::wstring(state);
            if (packageStateLabel(stateProbe, strings_, repositoryAvailable_) !=
                localeValue(strings_, key))
                return false;
        }

        constexpr unsigned long long expected =
            kCoveredGeneralApply |
            kCoveredRestart | kCoveredDiagnostics | kCoveredRepair | kCoveredPreview |
            kCoveredPackageRefresh | kCoveredPackageInstall | kCoveredPackageUpdate |
            kCoveredPackageDisable | kCoveredPackageEnable | kCoveredPackageRemove |
            kCoveredAppearanceReset | kCoveredAppearanceAdvanced | kCoveredFontSize |
            kCoveredFontEdit | kCoveredInputMethodRefresh | kCoveredInputMethodSetDefault;
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
    MESSAGE_HANDLER(WM_SETTINGCHANGE, onVisualSystemChanged)
    MESSAGE_HANDLER(WM_THEMECHANGED, onVisualSystemChanged)
    MESSAGE_HANDLER(WM_SYSCOLORCHANGE, onVisualSystemChanged)
    MESSAGE_HANDLER(WM_PAINT, onPaint)
    MESSAGE_HANDLER(WM_LBUTTONUP, onModernClick)
    MESSAGE_HANDLER(WM_KEYDOWN, onModernKeyDown)
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
    COMMAND_ID_HANDLER(kResetAppearance, onResetAppearance)
    COMMAND_HANDLER(kAppearanceAdvanced, BN_CLICKED, onAppearanceAdvanced)
    COMMAND_ID_HANDLER(kPackageRefresh, onPackageRefresh)
    COMMAND_ID_HANDLER(kPackageInstall, onPackageInstall)
    COMMAND_ID_HANDLER(kPackageRemove, onPackageRemove)
    COMMAND_ID_HANDLER(kPackageToggle, onPackageToggle)
    COMMAND_HANDLER(kPackages, LBN_SELCHANGE, onPackageSelection)
    COMMAND_HANDLER(kThemeLibrary, LBN_SELCHANGE, onThemeLibrarySelection)
    COMMAND_HANDLER(kInputMethod, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kStartup, BN_CLICKED, onDirty)
    COMMAND_HANDLER(kAppearance, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kTheme, CBN_SELCHANGE, onThemeSelection)
    COMMAND_HANDLER(kPageSize, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kMaxWidth, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kScrollCellWidth, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kFontSize, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kCornerRadius, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kOpacity, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kPreeditMode, CBN_SELCHANGE, onDirty)
    COMMAND_HANDLER(kAutomatic, BN_CLICKED, onDirty)
    COMMAND_HANDLER(kVertical, BN_CLICKED, onDirty)
    COMMAND_HANDLER(kHorizontal, BN_CLICKED, onDirty)
    COMMAND_HANDLER(kScrollMode, BN_CLICKED, onDirty)
    COMMAND_HANDLER(kShadow, BN_CLICKED, onDirty)
    COMMAND_HANDLER(kFont, CBN_SELCHANGE, onDirty)
    END_MSG_MAP()

  private:
    static constexpr unsigned long long kCoveredGeneralApply = 1ULL << 0U;
    static constexpr unsigned long long kCoveredAppearanceApply = 1ULL << 1U;
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
    static constexpr unsigned long long kCoveredAppearanceReset = 1ULL << 13U;
    static constexpr unsigned long long kCoveredAppearanceAdvanced = 1ULL << 14U;
    static constexpr unsigned long long kCoveredFontSize = 1ULL << 15U;
    static constexpr unsigned long long kCoveredFontEdit = 1ULL << 16U;
    static constexpr unsigned long long kCoveredInputMethodRefresh = 1ULL << 17U;
    static constexpr unsigned long long kCoveredInputMethodSetDefault = 1ULL << 18U;

    void cover(unsigned long long action) noexcept { actionCoverage_ |= action; }
    const wchar_t* get(const char* key) const {
        const auto iterator = strings_.find(key);
        return iterator == strings_.end() ? L"" : iterator->second.c_str();
    }
    HWND control(int id) const { return GetDlgItem(id); }
    [[nodiscard]] bool chineseUi() const {
        return std::wstring_view(get("nav.general")) == L"输入法";
    }
    [[nodiscard]] const wchar_t* modernText(const wchar_t* english,
                                            const wchar_t* chinese) const {
        return chineseUi() ? chinese : english;
    }
    [[nodiscard]] float px(float logical) const noexcept {
        return logical * static_cast<float>(dpi_) / 96.0F;
    }
    [[nodiscard]] D2D1_RECT_F logicalRect(float left, float top, float right,
                                          float bottom) const noexcept {
        return D2D1::RectF(px(left), px(top), px(right), px(bottom));
    }
    [[nodiscard]] RECT logicalHitRect(int left, int top, int right, int bottom) const noexcept {
        return RECT{scale(left), scale(top), scale(right), scale(bottom)};
    }
    [[nodiscard]] int modernLogicalWidth() const noexcept {
        RECT client{};
        if (!m_hWnd || !::GetClientRect(m_hWnd, &client))
            return 1100;
        return (std::max)(860, unscale(client.right - client.left));
    }
    [[nodiscard]] float modernSurfaceRight() const noexcept {
        return static_cast<float>((std::min)(1076, (std::max)(828, modernLogicalWidth() - 24)));
    }
    [[nodiscard]] float modernRowRight() const noexcept {
        return modernSurfaceRight() - 36.0F;
    }
    void addModernHit(int left, int top, int right, int bottom, ModernAction action,
                      int index = -1) {
        modernHits_.push_back(ModernHitTarget{logicalHitRect(left, top, right, bottom), action,
                                              index});
    }
    void showModernFontEditor() {
        const HWND picker = control(kFont);
        if (!picker)
            return;
        const int rowRight = static_cast<int>(modernRowRight());
        const int editorLeft = (std::max)(560, rowRight - 320);
        const int editorWidth = (std::max)(180, rowRight - editorLeft - 22);
        moveControl(kFont, editorLeft, appearanceAdvanced_ ? 552 : 552, editorWidth, 30);
        fontEditActive_ = true;
        ::ShowWindow(picker, SW_SHOW);
        ::EnableWindow(picker, TRUE);
        ::SetFocus(picker);
        SendMessageW(picker, CB_SHOWDROPDOWN, TRUE, 0);
    }
    void hideModernFontEditor() {
        if (fontEditActive_) {
            fontEditActive_ = false;
            ::ShowWindow(control(kFont), SW_HIDE);
        }
    }
    [[nodiscard]] bool legacyControlsVisible() const noexcept {
        return uiContractTest_ || interactionTest_ || livePreviewContractTest_ ||
               legacyVisualContractTest_;
    }
    [[nodiscard]] std::wstring comboText(int id) const {
        const HWND combo = control(id);
        const LRESULT selected = SendMessageW(combo, CB_GETCURSEL, 0, 0);
        if (!combo || selected == CB_ERR)
            return {};
        std::array<wchar_t, 128> text{};
        SendMessageW(combo, CB_GETLBTEXT, static_cast<WPARAM>(selected),
                     reinterpret_cast<LPARAM>(text.data()));
        return std::wstring(text.data());
    }
    [[nodiscard]] std::wstring windowText(int id) const {
        const HWND child = control(id);
        if (!child)
            return {};
        const int length = ::GetWindowTextLengthW(child);
        if (length <= 0)
            return {};
        std::wstring text(static_cast<std::size_t>(length + 1), L'\0');
        ::GetWindowTextW(child, text.data(), length + 1);
        text.resize(static_cast<std::size_t>(length));
        return text;
    }
    [[nodiscard]] std::wstring selectedThemeId() const {
        const LRESULT selected = SendMessageW(control(kTheme), CB_GETCURSEL, 0, 0);
        return selected == CB_ERR || static_cast<std::size_t>(selected) >= themes_.size()
                   ? L"builtin:default"
                   : themes_[static_cast<std::size_t>(selected)].id;
    }
    [[nodiscard]] bool selectComboText(int id, std::wstring_view value) const {
        const HWND combo = control(id);
        if (!combo || value.empty())
            return false;
        const LRESULT count = SendMessageW(combo, CB_GETCOUNT, 0, 0);
        for (LRESULT index = 0; index < count; ++index) {
            std::array<wchar_t, 128> text{};
            SendMessageW(combo, CB_GETLBTEXT, static_cast<WPARAM>(index),
                         reinterpret_cast<LPARAM>(text.data()));
            if (value == std::wstring_view(text.data())) {
                SendMessageW(combo, CB_SETCURSEL, index, 0);
                return true;
            }
        }
        return false;
    }
    [[nodiscard]] bool visible(int id) const {
        const HWND child = control(id);
        return child &&
               (::GetWindowLongPtrW(child, GWL_STYLE) & static_cast<LONG_PTR>(WS_VISIBLE)) != 0;
    }
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
        const int contentLeft = 250;
        const int labelWidth = 150;
        const int fieldLeft = contentLeft + labelWidth + 20;
        const int fieldWidth = (std::max)(220, (std::min)(contentWidth - labelWidth - 20, 460));
        const int contentRight = contentLeft + contentWidth;
        const int tallStatusHeight = (std::max)(250, logicalHeight - 400);
        const int packageListHeight = (std::max)(220, logicalHeight - 300);
        const int packageButtonY = (std::max)(488, logicalHeight - 162);
        const int packageStatusY = (std::max)(542, logicalHeight - 108);
        const int packageGap = 24;
        const int packageListWidth = (std::max)(300, (contentWidth - packageGap) / 2);
        const int packageDetailX = contentLeft + packageListWidth + packageGap;
        const int packageDetailWidth =
            (std::max)(260, contentRight - packageDetailX);
        const int buttonGap = 16;
        const int packageButtonWidth =
            (std::max)(128, (contentWidth - (buttonGap * 3)) / 4);
        const int packageButton2 = contentLeft + packageButtonWidth + buttonGap;
        const int packageButton3 = packageButton2 + packageButtonWidth + buttonGap;
        const int packageButton4 = packageButton3 + packageButtonWidth + buttonGap;
        const int themeLibraryTop = 480;
        const int themeLibraryHeight =
            (std::max)(120, (std::min)(180, logicalHeight - themeLibraryTop - 56));
        const int themeLibraryWidth = (std::max)(260, (contentWidth - 24) / 2);
        const int themeDetailX = contentLeft + themeLibraryWidth + 24;
        const int themeDetailWidth = (std::max)(260, contentRight - themeDetailX);
        const int saveStatusWidth =
            (std::max)(120, (std::min)(fieldWidth, contentRight - fieldLeft - 312));

        moveControl(kPageTitle, 238, 26, contentWidth, 38);
        moveControl(kBrandIcon, 22, 28, 24, 24);
        moveControl(kStartup, contentLeft, 104, (std::min)(contentWidth, 520), 32);
        moveControl(kInputMethodLabel, contentLeft, 158, labelWidth, 24);
        moveControl(kInputMethod, fieldLeft, 152, fieldWidth, 140);
        moveControl(kAppearanceLabel, contentLeft, 106, labelWidth, 24);
        moveControl(kAppearance, fieldLeft, 100, fieldWidth, 150);
        moveControl(kThemeLabel, contentLeft, 150, labelWidth, 24);
        moveControl(kTheme, fieldLeft, 144, fieldWidth, 150);
        moveControl(kFontLabel, contentLeft, 194, labelWidth, 24);
        moveControl(kFont, fieldLeft, 188, fieldWidth, 30);
        moveControl(kLayoutLabel, contentLeft, 242, labelWidth, 24);
        moveControl(kAutomatic, fieldLeft, 236, 90, 28);
        moveControl(kVertical, fieldLeft + 110, 236, 120, 28);
        moveControl(kHorizontal, fieldLeft + 250, 236, 140, 28);
        moveControl(kScrollMode, fieldLeft, 282, (std::min)(fieldWidth, 420), 28);
        moveControl(kPageSizeLabel, contentLeft, 326, labelWidth, 24);
        moveControl(kPageSize, fieldLeft, 320, 120, 180);
        moveControl(kFontSizeLabel, contentLeft + 330, 326, 130, 24);
        moveControl(kFontSize, contentLeft + 480, 320, 120, 180);
        moveControl(kMaxWidthLabel, contentLeft, 366, labelWidth, 24);
        moveControl(kMaxWidth, fieldLeft, 360, 120, 180);
        moveControl(kScrollCellWidthLabel, contentLeft + 330, 366, 130, 24);
        moveControl(kScrollCellWidth, contentLeft + 480, 360, 120, 180);
        moveControl(kCornerRadiusLabel, contentLeft, 406, labelWidth, 24);
        moveControl(kCornerRadius, fieldLeft, 400, 120, 180);
        moveControl(kOpacityLabel, contentLeft + 330, 406, 130, 24);
        moveControl(kOpacity, contentLeft + 480, 400, 120, 180);
        moveControl(kPreeditModeLabel, contentLeft, 446, labelWidth, 24);
        moveControl(kPreeditMode, fieldLeft, 440, (std::min)(fieldWidth, 260), 180);
        moveControl(kShadow, fieldLeft, 480, (std::min)(fieldWidth, 280), 28);
        const int appearanceAdvancedY = appearanceAdvanced_ ? 520 : 366;
        const int appearanceActionY = appearanceAdvanced_ ? 560 : 414;
        moveControl(kAppearanceAdvanced, contentLeft, appearanceAdvancedY, 240, 28);
        moveControl(kThemeLibraryLabel, contentLeft, themeLibraryTop - 28, 220, 24);
        moveControl(kThemeLibrary, contentLeft, themeLibraryTop, themeLibraryWidth,
                    themeLibraryHeight);
        moveControl(kThemeDetail, themeDetailX, themeLibraryTop, themeDetailWidth,
                    themeLibraryHeight);
        moveControl(kApply, contentRight - 300, appearanceActionY, 120, 36);
        moveControl(kPreview, contentLeft, appearanceActionY, 160, 36);
        moveControl(kResetAppearance, contentRight - 160, appearanceActionY, 150, 36);
        moveControl(kSaveStatus, fieldLeft, appearanceActionY + 8, saveStatusWidth, 24);
        moveControl(kRestart, contentLeft, 106, 170, 36);
        moveControl(kDiagnostics, contentLeft + 186, 106, 170, 36);
        moveControl(kRepair, contentLeft + 372, 106, 170, 36);
        moveControl(kPackagesTitle, contentLeft, 88, 300, 28);
        moveControl(kPackages, contentLeft, 122, packageListWidth, packageListHeight);
        moveControl(kPackageDetail, packageDetailX, 122, packageDetailWidth,
                    packageListHeight);
        moveControl(kPackageRefresh, contentLeft, packageButtonY, packageButtonWidth, 34);
        moveControl(kPackageInstall, packageButton2, packageButtonY, packageButtonWidth, 34);
        moveControl(kPackageToggle, packageButton3, packageButtonY, packageButtonWidth, 34);
        moveControl(kPackageRemove, packageButton4, packageButtonY, packageButtonWidth, 34);
        if (selectedPage_ == kNavPackages || selectedPage_ == kNavDiagnostics) {
            moveControl(kStatus, contentLeft, packageStatusY, contentWidth, 50);
        } else {
            moveControl(kStatus, contentLeft, 168, contentWidth, tallStatusHeight);
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
        add(L"STATIC", L"", SS_ICON, 22, 28, 24, 24, kBrandIcon);
        brandIcon_ = static_cast<HICON>(LoadImageW(
            _Module.GetResourceInstance(), MAKEINTRESOURCEW(IDI_FCITX5_APP), IMAGE_ICON,
            scale(24), scale(24), LR_DEFAULTCOLOR));
        if (brandIcon_)
            SendMessageW(control(kBrandIcon), STM_SETICON, reinterpret_cast<WPARAM>(brandIcon_), 0);
        add(L"STATIC", L"Fcitx5", 0, 54, 28, 130, 34, kBrandText);
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
        add(L"STATIC", get("font.label"), 0, 250, 148, 150, 24, kFontLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP | WS_VSCROLL, 420, 142, 330,
            220, kFont);
        for (const auto& fontFamily : enumerateFontFamilies(m_hWnd)) {
            SendMessageW(control(kFont), CB_ADDSTRING, 0,
                         reinterpret_cast<LPARAM>(fontFamily.c_str()));
        }
        if (!selectComboText(kFont, L"Microsoft YaHei"))
            SendMessageW(control(kFont), CB_SETCURSEL, 0, 0);
        add(L"STATIC", get("theme.library"), 0, 250, 196, 220, 24, kThemeLibraryLabel);
        add(L"LISTBOX", L"", WS_BORDER | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY, 250, 224, 300,
            210, kThemeLibrary);
        add(L"EDIT", L"", WS_BORDER | ES_MULTILINE | ES_READONLY | WS_VSCROLL, 570, 224, 380,
            210, kThemeDetail);
        add(L"STATIC", get("candidate.layout"), 0, 250, 162, 150, 24, kLayoutLabel);
        add(L"BUTTON", get("candidate.automatic"), BS_AUTORADIOBUTTON | WS_GROUP | WS_TABSTOP,
            420, 156, 90, 28, kAutomatic);
        add(L"BUTTON", get("candidate.vertical"), BS_AUTORADIOBUTTON | WS_TABSTOP, 520,
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
        add(L"STATIC", get("candidate.max_width"), 0, 250, 272, 150, 24, kMaxWidthLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 420, 266, 120, 180, kMaxWidth);
        for (const wchar_t* value : {L"520", L"720", L"860", L"1080"}) {
            SendMessageW(control(kMaxWidth), CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(value));
        }
        SendMessageW(control(kMaxWidth), CB_SETCURSEL, 2, 0);
        add(L"STATIC", get("candidate.scroll_cell_width"), 0, 250, 308, 150, 24,
            kScrollCellWidthLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 420, 302, 120, 180,
            kScrollCellWidth);
        for (const wchar_t* value : {L"72", L"88", L"96", L"120"}) {
            SendMessageW(control(kScrollCellWidth), CB_ADDSTRING, 0,
                         reinterpret_cast<LPARAM>(value));
        }
        SendMessageW(control(kScrollCellWidth), CB_SETCURSEL, 2, 0);
        add(L"STATIC", get("candidate.font_size"), 0, 250, 344, 150, 24, kFontSizeLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 420, 338, 120, 180,
            kFontSize);
        for (const wchar_t* value : {L"16", L"18", L"20", L"22", L"24"}) {
            SendMessageW(control(kFontSize), CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(value));
        }
        SendMessageW(control(kFontSize), CB_SETCURSEL, 1, 0);
        add(L"STATIC", get("candidate.corner_radius"), 0, 250, 380, 150, 24,
            kCornerRadiusLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 420, 374, 120, 180,
            kCornerRadius);
        for (const wchar_t* value : {L"0", L"8", L"12", L"16", L"24"}) {
            SendMessageW(control(kCornerRadius), CB_ADDSTRING, 0,
                         reinterpret_cast<LPARAM>(value));
        }
        SendMessageW(control(kCornerRadius), CB_SETCURSEL, 2, 0);
        add(L"STATIC", get("candidate.opacity"), 0, 560, 380, 150, 24, kOpacityLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 710, 374, 120, 180, kOpacity);
        for (const wchar_t* value : {L"1.00", L"0.95", L"0.90", L"0.85", L"0.75"}) {
            SendMessageW(control(kOpacity), CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(value));
        }
        SendMessageW(control(kOpacity), CB_SETCURSEL, 0, 0);
        add(L"STATIC", get("candidate.preedit_mode"), 0, 250, 416, 150, 24,
            kPreeditModeLabel);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 420, 410, 260, 180,
            kPreeditMode);
        for (const wchar_t* item : {get("preedit.inline"), get("preedit.panel")}) {
            SendMessageW(control(kPreeditMode), CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(item));
        }
        SendMessageW(control(kPreeditMode), CB_SETCURSEL, 0, 0);
        add(L"BUTTON", get("candidate.shadow"), BS_AUTOCHECKBOX | WS_TABSTOP, 420, 448, 260,
            28, kShadow);
        SendMessageW(control(kShadow), BM_SETCHECK, BST_CHECKED, 0);
        SendMessageW(control(kAutomatic), BM_SETCHECK, BST_CHECKED, 0);
        add(L"BUTTON", get("appearance.more"), BS_AUTOCHECKBOX | WS_TABSTOP, 250, 416, 240, 28,
            kAppearanceAdvanced);
        add(L"BUTTON", get("action.apply"), BS_DEFPUSHBUTTON | WS_TABSTOP, 650, 264, 120, 36,
            kApply);
        add(L"BUTTON", get("action.preview"), WS_TABSTOP, 250, 264, 160, 36, kPreview);
        add(L"BUTTON", get("action.reset_appearance"), WS_TABSTOP, 780, 264, 150, 36,
            kResetAppearance);
        add(L"STATIC", L"", SS_LEFT, 420, 272, 210, 24, kSaveStatus);
        add(L"BUTTON", get("action.restart"), WS_TABSTOP, 250, 106, 170, 36, kRestart);
        add(L"BUTTON", get("action.diagnostics"), WS_TABSTOP, 436, 106, 170, 36, kDiagnostics);
        add(L"BUTTON", get("action.repair"), WS_TABSTOP, 250, 106, 170, 36, kRepair);
        add(L"EDIT", L"", WS_BORDER | ES_MULTILINE | ES_READONLY | WS_VSCROLL, 250, 168, 700, 250,
            kStatus);
        add(L"STATIC", get("packages.title"), 0, 250, 88, 300, 28, kPackagesTitle);
        add(L"LISTBOX", L"", WS_BORDER | WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY, 250, 122, 700, 350,
            kPackages);
        add(L"EDIT", L"", WS_BORDER | ES_MULTILINE | ES_READONLY | WS_VSCROLL, 570, 122, 380,
            350, kPackageDetail);
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
        if (target_) {
            RECT rectangle{};
            GetClientRect(&rectangle);
            target_->Resize(D2D1::SizeU(rectangle.right, rectangle.bottom));
        }
        layoutControls();
        return 0;
    }
    LRESULT onGetMinMaxInfo(UINT, WPARAM, LPARAM lparam, BOOL&) {
        auto* limits = reinterpret_cast<MINMAXINFO*>(lparam);
        limits->ptMinTrackSize.x = scale(860);
        limits->ptMinTrackSize.y = scale(600);
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
    LRESULT onVisualSystemChanged(UINT, WPARAM, LPARAM, BOOL&) {
        if (target_) {
            target_->Release();
            target_ = nullptr;
        }
        layoutControls();
        refreshStatusControls();
        InvalidateRect(nullptr, TRUE);
        return 0;
    }
    void showPage(int page) {
        if (page != kNavAppearance)
            hideModernFontEditor();
        selectedPage_ = page;
        const auto show = [&](int id, bool visible) {
            if (const HWND child = control(id))
                ::ShowWindow(child, visible ? SW_SHOW : SW_HIDE);
        };
        const bool showLegacyControls = legacyControlsVisible();
        for (const int id : {kStartup,         kInputMethod,    kInputMethodLabel, kAppearance,
                             kAppearanceLabel, kTheme,          kThemeLabel,       kFont,
                             kFontLabel,       kThemeLibrary,   kThemeLibraryLabel, kThemeDetail,
                             kAutomatic,      kVertical,       kHorizontal,       kLayoutLabel,
                             kScrollMode,      kPageSize,       kPageSizeLabel,    kApply,
                             kMaxWidth,        kMaxWidthLabel,  kScrollCellWidth,
                             kScrollCellWidthLabel, kFontSize,  kFontSizeLabel,
                             kCornerRadius,    kCornerRadiusLabel, kShadow,
                             kOpacity,         kOpacityLabel,   kPreeditMode,      kPreeditModeLabel,
                             kAppearanceAdvanced, kPreview,     kResetAppearance,  kRestart,
                             kDiagnostics,     kRepair,         kStatus,           kPackages,
                             kPackageDetail,   kPackageRefresh, kPackageInstall, kPackageToggle,    kPackageRemove,
                             kPackagesTitle,   kSaveStatus})
            show(id, false);
        for (const int id : {kNavGeneral, kNavAppearance, kNavTheme, kNavDiagnostics,
                             kNavRepair, kNavPackages, kPageTitle, kBrandText})
            show(id, showLegacyControls);
        const bool general = page == kNavGeneral;
        const bool appearance = page == kNavAppearance;
        const bool shortcuts = page == kNavTheme;
        const bool updates = page == kNavDiagnostics;
        const bool diagnosticsRepair = page == kNavRepair;
        const bool packages = page == kNavPackages;
        for (const int id : {kStartup, kInputMethod, kInputMethodLabel})
            show(id, showLegacyControls && general);
        for (const int id : {kAppearance, kAppearanceLabel, kAutomatic, kVertical, kHorizontal,
                             kLayoutLabel,
                             kScrollMode, kPageSize, kPageSizeLabel, kFontSize, kFontSizeLabel,
                             kTheme, kThemeLabel, kFont, kFontLabel, kAppearanceAdvanced, kPreview,
                             kResetAppearance})
            show(id, showLegacyControls && appearance);
        for (const int id : {kThemeLibrary, kThemeLibraryLabel, kThemeDetail})
            show(id, showLegacyControls && appearance && !appearanceAdvanced_);
        for (const int id : {kMaxWidth, kMaxWidthLabel, kScrollCellWidth, kScrollCellWidthLabel,
                             kCornerRadius, kCornerRadiusLabel, kOpacity, kOpacityLabel,
                             kPreeditMode, kPreeditModeLabel, kShadow})
            show(id, showLegacyControls && appearance && appearanceAdvanced_);
        show(kApply, showLegacyControls && general);
        show(kSaveStatus, showLegacyControls && (general || appearance));
        const bool dirty = general ? generalDirty_ : presentationDirty_;
        for (const int id : {kRestart, kDiagnostics})
            show(id, showLegacyControls && diagnosticsRepair);
        for (const int id : {kRepair})
            show(id, showLegacyControls && diagnosticsRepair);
        for (const int id : {kPackages, kPackageDetail, kPackageRefresh, kPackageInstall,
                             kPackageToggle, kPackageRemove, kPackagesTitle})
            show(id, showLegacyControls && (packages || updates));
        show(kStatus, showLegacyControls && (shortcuts || diagnosticsRepair || packages || updates));
        if (shortcuts)
            ::SetWindowTextW(control(kStatus), get("shortcuts.placeholder"));
        else if (updates || packages)
            ::SetWindowTextW(control(kStatus), L"");
        if (diagnosticsRepair)
            refreshStatusControls();
        ::SetWindowTextW(control(kPackagesTitle),
                         updates ? get("updates.title") : get("packages.title"));
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
        if (appearance) {
            ensureProductionPreview();
        } else {
            stopProductionPreview();
        }
    }
    void loadState() {
        std::wstring output;
        Strings presentation;
        std::wstring selectedTheme = L"builtin:default";
        if (runControl({L"--get-presentation"}, output) &&
            parseFlatJson(narrow(output), presentation)) {
            const auto mode = presentation.find("appearance_mode");
            if (mode != presentation.end())
                SendMessageW(control(kAppearance), CB_SETCURSEL,
                             mode->second == L"light" ? 1 : (mode->second == L"dark" ? 2 : 0), 0);
            const auto orientation = presentation.find("orientation");
            if (orientation != presentation.end()) {
                SendMessageW(control(kAutomatic), BM_SETCHECK,
                             orientation->second == L"automatic" ? BST_CHECKED : BST_UNCHECKED,
                             0);
                SendMessageW(control(kVertical), BM_SETCHECK,
                             orientation->second == L"vertical" ? BST_CHECKED : BST_UNCHECKED,
                             0);
                SendMessageW(control(kHorizontal), BM_SETCHECK,
                             orientation->second == L"horizontal" ? BST_CHECKED : BST_UNCHECKED,
                             0);
            }
            const auto candidateFont = presentation.find("candidate_font");
            if (candidateFont != presentation.end() && !candidateFont->second.empty()) {
                if (!selectComboText(kFont, candidateFont->second)) {
                    SendMessageW(control(kFont), CB_ADDSTRING, 0,
                                 reinterpret_cast<LPARAM>(candidateFont->second.c_str()));
                    (void)selectComboText(kFont, candidateFont->second);
                }
            }
            const auto theme = presentation.find("theme");
            if (theme != presentation.end() && !theme->second.empty())
                selectedTheme = theme->second;
            const auto scrollMode = presentation.find("scroll_mode");
            if (scrollMode != presentation.end())
                SendMessageW(control(kScrollMode), BM_SETCHECK,
                             scrollMode->second == L"true" ? BST_CHECKED : BST_UNCHECKED, 0);
            const auto pageSize = presentation.find("candidate_page_size");
            if (pageSize != presentation.end() && pageSize->second.size() == 1U &&
                pageSize->second[0] >= L'1' && pageSize->second[0] <= L'9') {
                SendMessageW(control(kPageSize), CB_SETCURSEL, pageSize->second[0] - L'1', 0);
            }
            const auto maxWidth = presentation.find("candidate_max_width_dip");
            if (maxWidth != presentation.end())
                (void)selectComboText(kMaxWidth, maxWidth->second);
            const auto scrollCellWidth = presentation.find("candidate_scroll_cell_width_dip");
            if (scrollCellWidth != presentation.end())
                (void)selectComboText(kScrollCellWidth, scrollCellWidth->second);
            const auto fontSize = presentation.find("candidate_font_size_dip");
            if (fontSize != presentation.end())
                (void)selectComboText(kFontSize, fontSize->second);
            const auto cornerRadius = presentation.find("candidate_corner_radius_dip");
            if (cornerRadius != presentation.end())
                (void)selectComboText(kCornerRadius, cornerRadius->second);
            const auto opacity = presentation.find("candidate_opacity");
            if (opacity != presentation.end()) {
                std::wstring value = opacity->second.substr(0, 4);
                if (value == L"1.0")
                    value = L"1.00";
                (void)selectComboText(kOpacity, value);
            }
            const auto preeditMode = presentation.find("candidate_preedit_mode");
            if (preeditMode != presentation.end()) {
                SendMessageW(control(kPreeditMode), CB_SETCURSEL,
                             preeditMode->second == L"panel" ? 1 : 0, 0);
            }
            const auto shadow = presentation.find("candidate_shadow");
            if (shadow != presentation.end())
                SendMessageW(control(kShadow), BM_SETCHECK,
                             shadow->second == L"true" ? BST_CHECKED : BST_UNCHECKED, 0);
        }
        refreshThemes(selectedTheme);
        if (runControl({L"--get-startup"}, output))
            SendMessageW(control(kStartup), BM_SETCHECK,
                         output.find(L"\"enabled\":true") != std::wstring::npos ? BST_CHECKED
                                                                                : BST_UNCHECKED,
                         0);
        loadInputMethods();
        refresh();
        refreshPackages(false);
    }
    bool loadInputMethods() {
        std::wstring output;
        std::vector<InputMethodRow> rows;
        if (!runControl({L"--get-input-methods"}, output) ||
            !parseInputMethods(output, rows)) {
            inputMethods_.clear();
            SendMessageW(control(kInputMethod), CB_RESETCONTENT, 0, 0);
            ::EnableWindow(control(kInputMethod), FALSE);
            return false;
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
        return true;
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
                         runControl({L"--diagnostics-plan"}, output) ? output.c_str()
                                                                      : get("error.command"));
    }
    void selectThemeById(std::wstring_view id) {
        if (themes_.empty())
            return;
        int selected = 0;
        for (std::size_t index = 0; index < themes_.size(); ++index) {
            if (themes_[index].id == id) {
                selected = static_cast<int>(index);
                break;
            }
        }
        SendMessageW(control(kTheme), CB_SETCURSEL, selected, 0);
        SendMessageW(control(kThemeLibrary), LB_SETCURSEL, selected, 0);
        updateThemeDetail();
    }
    std::wstring basicThemeDetail(const ThemeRow& theme) const {
        std::wstring detail = L"ID: " + theme.id + L"\r\n";
        detail += localeValue(strings_, "theme.detail.source", L"Source") + L": " +
                  themeSourceLabel(theme, strings_) + L"\r\n";
        if (!theme.version.empty())
            detail += localeValue(strings_, "theme.detail.version", L"Version") + L": " +
                      theme.version + L"\r\n";
        if (!theme.license.empty())
            detail += localeValue(strings_, "theme.detail.license", L"License") + L": " +
                      theme.license + L"\r\n";
        if (!theme.description.empty())
            detail += L"\r\n" + theme.description + L"\r\n";
        return detail;
    }
    void updateThemeDetail() {
        const LRESULT selected = SendMessageW(control(kThemeLibrary), LB_GETCURSEL, 0, 0);
        if (selected == LB_ERR || static_cast<std::size_t>(selected) >= themes_.size()) {
            ::SetWindowTextW(control(kThemeDetail), L"");
            return;
        }
        const auto& theme = themes_[static_cast<std::size_t>(selected)];
        std::wstring detail = basicThemeDetail(theme);
        std::wstring output;
        if (runControl({L"--themes-detail", theme.id}, output)) {
            try {
                const auto document = nlohmann::json::parse(narrow(output));
                const bool light = document.at("has_light_branch").get<bool>();
                const bool dark = document.at("has_dark_branch").get<bool>();
                const auto editableFields = document.at("editable_fields").size();
                const auto& security = document.at("security");
                detail += L"\r\n" + localeValue(strings_, "theme.detail.branches", L"Branches") +
                          L": ";
                detail += light ? localeValue(strings_, "theme.detail.light", L"light")
                                : localeValue(strings_, "theme.detail.no_light", L"no light");
                detail += L", ";
                detail += dark ? localeValue(strings_, "theme.detail.dark", L"dark")
                               : localeValue(strings_, "theme.detail.no_dark", L"no dark");
                detail += L"\r\n" +
                          localeValue(strings_, "theme.detail.editable_fields",
                                      L"Editable fields") +
                          L": " + std::to_wstring(editableFields);
                detail += L"\r\n" + localeValue(strings_, "theme.detail.security", L"Security") +
                          L": ";
                detail += security.at("script_allowed").get<bool>() ? L"script" : L"no script";
                detail += L", ";
                detail += security.at("network_allowed").get<bool>() ? L"network" : L"no network";
                detail += L", ";
                detail += widen(security.at("path_scope").get<std::string>());
            } catch (const nlohmann::json::exception&) {
                detail += L"\r\n" + localeValue(strings_, "theme.detail_limited",
                                                 L"Detailed theme metadata is unavailable.");
            }
        } else {
            detail += L"\r\n" + localeValue(strings_, "theme.detail_limited",
                                             L"Detailed theme metadata is unavailable.");
        }
        ::SetWindowTextW(control(kThemeDetail), detail.c_str());
    }
    void refreshThemes(std::wstring_view selectedTheme) {
        std::wstring output;
        std::vector<ThemeRow> rows;
        if (!runControl({L"--themes-list"}, output) || !parseThemes(output, rows) ||
            rows.empty()) {
            rows = {{L"builtin:default", L"builtin", get("theme.default"), L"", L"", L""}};
        }
        themes_ = std::move(rows);
        SendMessageW(control(kTheme), CB_RESETCONTENT, 0, 0);
        SendMessageW(control(kThemeLibrary), LB_RESETCONTENT, 0, 0);
        for (const auto& theme : themes_) {
            const std::wstring label = themeListLabel(theme, strings_);
            SendMessageW(control(kTheme), CB_ADDSTRING, 0,
                         reinterpret_cast<LPARAM>(label.c_str()));
            SendMessageW(control(kThemeLibrary), LB_ADDSTRING, 0,
                         reinterpret_cast<LPARAM>(label.c_str()));
        }
        selectThemeById(selectedTheme.empty() ? L"builtin:default" : selectedTheme);
    }
    void restart() {
        std::wstring output;
        setStatus(runControl({L"--restart-engine"}, output) ? get("restart.done")
                                                            : get("error.command"));
    }
    bool applyPresentation() {
        if (interactionTest_ || livePreviewContractTest_) {
            ++liveApplyCount_;
            return !forceLiveApplyFailure_;
        }
        const auto modeIndex = SendMessageW(control(kAppearance), CB_GETCURSEL, 0, 0);
        const wchar_t* const mode =
            modeIndex == 1 ? L"light" : (modeIndex == 2 ? L"dark" : L"system");
        const wchar_t* const orientation =
            SendMessageW(control(kHorizontal), BM_GETCHECK, 0, 0) == BST_CHECKED
                ? L"horizontal"
            : SendMessageW(control(kVertical), BM_GETCHECK, 0, 0) == BST_CHECKED ? L"vertical"
                                                                                 : L"automatic";
        const auto pageSizeIndex = SendMessageW(control(kPageSize), CB_GETCURSEL, 0, 0);
        if (pageSizeIndex == CB_ERR || pageSizeIndex < 0 || pageSizeIndex > 8)
            return false;
        const std::wstring pageSize = std::to_wstring(pageSizeIndex + 1);
        const std::wstring maxWidth = comboText(kMaxWidth);
        const std::wstring scrollCellWidth = comboText(kScrollCellWidth);
        const std::wstring fontSize = comboText(kFontSize);
        const std::wstring cornerRadius = comboText(kCornerRadius);
        const std::wstring opacity = comboText(kOpacity);
        const auto preeditModeIndex = SendMessageW(control(kPreeditMode), CB_GETCURSEL, 0, 0);
        const wchar_t* const preeditMode = preeditModeIndex == 1 ? L"panel" : L"inline";
        const wchar_t* const shadow =
            SendMessageW(control(kShadow), BM_GETCHECK, 0, 0) == BST_CHECKED ? L"enabled"
                                                                             : L"disabled";
        const std::wstring font = comboText(kFont);
        std::wstring output;
        const bool ok =
            !font.empty() && !maxWidth.empty() && !scrollCellWidth.empty() &&
            !fontSize.empty() && !cornerRadius.empty() && !opacity.empty() &&
            runControl({L"--set-presentation", mode, selectedThemeId(), orientation,
                        SendMessageW(control(kScrollMode), BM_GETCHECK, 0, 0) == BST_CHECKED
                            ? L"enabled"
                            : L"disabled",
                        pageSize, font, maxWidth, scrollCellWidth, fontSize, cornerRadius,
                        shadow, opacity, preeditMode},
                       output);
        return ok;
    }
    bool resetPresentation() {
        if (interactionTest_ || livePreviewContractTest_) {
            ++resetApplyCount_;
            return !forceLiveApplyFailure_;
        }
        std::wstring output;
        const bool ok = runControl({L"--reset-presentation"}, output);
        if (ok)
            loadState();
        return ok;
    }
    bool confirmDialog(const char* titleKey, const char* bodyKey, UINT flags = MB_OKCANCEL) const {
        if (interactionTest_ || livePreviewContractTest_)
            return true;
        const std::wstring title = std::wstring(get("app.title")) + L" — " + get(titleKey);
        return ::MessageBoxW(m_hWnd, get(bodyKey), title.c_str(),
                             flags | MB_ICONWARNING | MB_DEFBUTTON2) == IDOK;
    }
    void showTrustFailureDialog() {
        if (interactionTest_)
            return;
        const std::wstring title =
            std::wstring(get("app.title")) + L" — " + get("dialog.trust_failure.title");
        ::MessageBoxW(m_hWnd, get("dialog.trust_failure.body"), title.c_str(),
                      MB_OK | MB_ICONERROR);
    }
    void liveApplyPresentation() {
        const bool ok = applyPresentation();
        if (ok) {
            presentationDirty_ = false;
            setSaveStatus(get("status.saved"));
            (void)ensureProductionPreview();
        } else {
            presentationDirty_ = true;
            setSaveStatus(get("error.command"));
        }
    }
    void repair() {
        if (!confirmDialog("dialog.repair.title", "dialog.repair.body"))
            return;
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
            !parsePackages(output, packages_, repositoryAvailable_, repositoryError_)) {
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
        updatePackageDetail();
        if (!refreshed)
            setStatus(get("packages.online_error"));
        else if (!repositoryAvailable_)
            setStatus(repositoryTrustMessage(repositoryAvailable_, repositoryError_, strings_));
    }

    void updatePackageDetail() {
        const int selected = selectedPackage();
        if (selected < 0) {
            ::SetWindowTextW(control(kPackageDetail), L"");
            return;
        }
        const auto& package = packages_[static_cast<std::size_t>(selected)];
        std::wstring detail = L"ID: " + package.id + L"\r\n";
        detail += localeValue(strings_, "packages.detail.type", L"Type") + L": " +
                  packageTypeLabel(package, strings_) + L"\r\n";
        if (!package.summary.empty())
            detail += L"\r\n" + package.summary + L"\r\n";
        std::wstring output;
        if (!runControl({L"--packages-detail", package.id}, output)) {
            detail += L"\r\n" + localeValue(strings_, "packages.detail_limited",
                                             L"Detailed package metadata is unavailable.");
            ::SetWindowTextW(control(kPackageDetail), detail.c_str());
            return;
        }
        try {
            const auto document = nlohmann::json::parse(narrow(output));
            const auto available = jsonStringOrNull(document, "available_version");
            const auto installed = jsonStringOrNull(document, "installed_version");
            const auto state = jsonStringOrNull(document, "state");
            const auto manifest = jsonStringOrNull(document, "manifest_sha256");
            const auto sourceCommit = jsonStringOrNull(document, "source_commit");
            const auto repositoryError = jsonStringOrNull(document, "repository_error");
            detail += L"\r\n" + localeValue(strings_, "packages.detail.repository", L"Repository") +
                      L": ";
            const bool repositoryAvailable = document.at("repository_available").get<bool>();
            detail += repositoryAvailable
                          ? localeValue(strings_, "packages.detail.available", L"available")
                          : localeValue(strings_, "packages.detail.unavailable", L"unavailable");
            if (!repositoryAvailable)
                detail += L"\r\n" +
                          repositoryTrustMessage(repositoryAvailable, repositoryError, strings_);
            if (!available.empty())
                detail += L"\r\n" + localeValue(strings_, "packages.detail.available_version",
                                                 L"Available") +
                          L": " + available;
            if (!installed.empty())
                detail += L"\r\n" + localeValue(strings_, "packages.detail.installed_version",
                                                 L"Installed") +
                          L": " + installed;
            if (!state.empty())
                detail += L"\r\n" + localeValue(strings_, "packages.detail.state", L"State") +
                          L": " + state;
            detail += L"\r\n" + localeValue(strings_, "packages.detail.permissions",
                                             L"Permissions") +
                      L": " +
                      jsonArraySummary(document.at("permissions"),
                                       localeValue(strings_, "packages.detail.none", L"none"));
            detail += L"\r\n" + localeValue(strings_, "packages.detail.dependencies",
                                             L"Dependencies") +
                      L": " +
                      jsonArraySummary(document.at("dependencies"),
                                       localeValue(strings_, "packages.detail.none", L"none"));
            detail += L"\r\n" + localeValue(strings_, "packages.detail.config_surface",
                                             L"Config surface") +
                      L": " +
                      jsonArraySummary(document.at("config_surface"),
                                       localeValue(strings_, "packages.detail.none", L"none"));
            if (!sourceCommit.empty())
                detail += L"\r\n" + localeValue(strings_, "packages.detail.source_commit",
                                                 L"Source commit") +
                          L": " + sourceCommit;
            if (!manifest.empty())
                detail += L"\r\n" + localeValue(strings_, "packages.detail.manifest",
                                                 L"Manifest SHA-256") +
                          L": " + manifest;
        } catch (const nlohmann::json::exception&) {
            detail += L"\r\n" + localeValue(strings_, "packages.detail_limited",
                                             L"Detailed package metadata is unavailable.");
        }
        ::SetWindowTextW(control(kPackageDetail), detail.c_str());
    }

    void updatePackageActions() {
        const int selected = selectedPackage();
        const PackageRow* package =
            selected < 0 ? nullptr : &packages_[static_cast<std::size_t>(selected)];
        const bool safeInstalled = package && !packageUnsafeForInstalledAction(*package);
        ::EnableWindow(control(kPackageInstall),
                       package && !packageUnsafeForInstall(*package, repositoryAvailable_) &&
                           repositoryAvailable_ && !package->available.empty());
        ::EnableWindow(control(kPackageToggle),
                       safeInstalled && !package->installed.empty());
        ::EnableWindow(control(kPackageRemove),
                       safeInstalled && !package->installed.empty());
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
        if (package.state == L"trust-failed") {
            setStatus(get("packages.trust_failed"));
            showTrustFailureDialog();
            return;
        }
        if (!repositoryAvailable_) {
            setStatus(repositoryTrustMessage(repositoryAvailable_, repositoryError_, strings_));
            return;
        }
        if (packageUnsafeForInstall(package, repositoryAvailable_)) {
            setStatus(packageBlockedActionMessage(package, strings_, repositoryAvailable_,
                                                  repositoryError_));
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
        if (packageUnsafeForInstalledAction(packages_[static_cast<std::size_t>(selected)])) {
            setStatus(packageBlockedActionMessage(packages_[static_cast<std::size_t>(selected)],
                                                  strings_, repositoryAvailable_,
                                                  repositoryError_));
            return;
        }
        if (!confirmDialog("dialog.remove_package.title", "dialog.remove_package.body"))
            return;
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
        if (packageUnsafeForInstalledAction(package)) {
            setStatus(packageBlockedActionMessage(package, strings_, repositoryAvailable_,
                                                  repositoryError_));
            return;
        }
        std::wstring output;
        const bool ok = runControl({L"--packages-state", package.id,
                                    package.state == L"disabled" ? L"enabled" : L"disabled"},
                                   output);
        setStatus(ok ? get("packages.changed") : get("error.command"));
        refreshPackages(false);
    }

    void stopProductionPreview() {
        previewActiveForContract_ = false;
    }

    bool ensureProductionPreview() {
        if (!previewActiveForContract_) {
            ++previewLaunchCount_;
            previewActiveForContract_ = true;
        }
        InvalidateRect(nullptr, FALSE);
        return true;
    }

    void preview() {
        const bool ok = applyPresentation();
        presentationDirty_ = !ok;
        setSaveStatus(ok ? get("status.saved") : get("error.command"));
        if (ok)
            (void)ensureProductionPreview();
    }

    bool ensureDWrite() {
        if (writeFactory_)
            return true;
        return SUCCEEDED(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory),
                                             reinterpret_cast<IUnknown**>(&writeFactory_)));
    }
    IDWriteTextFormat* makeFormat(std::wstring_view family, float size,
                                  DWRITE_FONT_WEIGHT weight = DWRITE_FONT_WEIGHT_NORMAL,
                                  DWRITE_TEXT_ALIGNMENT alignment =
                                      DWRITE_TEXT_ALIGNMENT_LEADING,
                                  DWRITE_PARAGRAPH_ALIGNMENT paragraph =
                                      DWRITE_PARAGRAPH_ALIGNMENT_NEAR) {
        if (!ensureDWrite())
            return nullptr;
        IDWriteTextFormat* format = nullptr;
        const std::wstring fontFamily =
            family.empty() ? std::wstring(L"Segoe UI") : std::wstring(family);
        if (FAILED(writeFactory_->CreateTextFormat(
                fontFamily.c_str(), nullptr, weight, DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL, px(size), L"", &format))) {
            return nullptr;
        }
        format->SetTextAlignment(alignment);
        format->SetParagraphAlignment(paragraph);
        return format;
    }
    IDWriteTextFormat* makeFormat(float size, DWRITE_FONT_WEIGHT weight = DWRITE_FONT_WEIGHT_NORMAL,
                                  DWRITE_TEXT_ALIGNMENT alignment =
                                      DWRITE_TEXT_ALIGNMENT_LEADING,
                                  DWRITE_PARAGRAPH_ALIGNMENT paragraph =
                                      DWRITE_PARAGRAPH_ALIGNMENT_NEAR) {
        return makeFormat(L"Segoe UI", size, weight, alignment, paragraph);
    }
    void drawText(ID2D1SolidColorBrush* brush, std::wstring_view text, float left, float top,
                  float right, float bottom, float size,
                  DWRITE_FONT_WEIGHT weight = DWRITE_FONT_WEIGHT_NORMAL,
                  DWRITE_TEXT_ALIGNMENT alignment = DWRITE_TEXT_ALIGNMENT_LEADING,
                  DWRITE_PARAGRAPH_ALIGNMENT paragraph = DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
                  std::wstring_view family = L"Segoe UI") {
        if (!target_ || !brush || text.empty())
            return;
        IDWriteTextFormat* format = makeFormat(family, size, weight, alignment, paragraph);
        if (!format)
            return;
        constexpr D2D1_DRAW_TEXT_OPTIONS kDrawOptions =
            static_cast<D2D1_DRAW_TEXT_OPTIONS>(
                static_cast<UINT32>(D2D1_DRAW_TEXT_OPTIONS_CLIP) | 0x4U);
        target_->DrawTextW(text.data(), static_cast<UINT32>(text.size()), format,
                           logicalRect(left, top, right, bottom), brush, kDrawOptions);
        format->Release();
    }
    [[nodiscard]] float measureTextWidth(std::wstring_view text, float size,
                                         DWRITE_FONT_WEIGHT weight,
                                         std::wstring_view family) {
        if (!ensureDWrite() || text.empty())
            return 0.0F;
        IDWriteTextFormat* format = makeFormat(family, size, weight);
        if (!format)
            return 0.0F;
        IDWriteTextLayout* layout = nullptr;
        const HRESULT result = writeFactory_->CreateTextLayout(
            text.data(), static_cast<UINT32>(text.size()), format, px(4096.0F), px(128.0F),
            &layout);
        format->Release();
        if (FAILED(result) || !layout)
            return 0.0F;
        DWRITE_TEXT_METRICS metrics{};
        const HRESULT metricsResult = layout->GetMetrics(&metrics);
        layout->Release();
        if (FAILED(metricsResult))
            return 0.0F;
        return metrics.widthIncludingTrailingWhitespace * 96.0F / static_cast<float>(dpi_);
    }
    void fillRound(ID2D1SolidColorBrush* brush, float left, float top, float right, float bottom,
                   float radius) {
        target_->FillRoundedRectangle(
            D2D1::RoundedRect(logicalRect(left, top, right, bottom), px(radius), px(radius)),
            brush);
    }
    void strokeRound(ID2D1SolidColorBrush* brush, float left, float top, float right,
                     float bottom, float radius, float width = 1.0F) {
        target_->DrawRoundedRectangle(
            D2D1::RoundedRect(logicalRect(left, top, right, bottom), px(radius), px(radius)),
            brush, px(width));
    }
    void drawToggle(ID2D1SolidColorBrush* brush, bool enabled, float left, float top) {
        const auto tokens = designTokens();
        brush->SetColor(d2dColor(enabled ? tokens.accent : RGB(172, 178, 188)));
        fillRound(brush, left, top, left + 44, top + 24, 12);
        brush->SetColor(d2dColor(RGB(255, 255, 255)));
        const float knobLeft = enabled ? left + 22 : left + 2;
        target_->FillEllipse(D2D1::Ellipse(D2D1::Point2F(px(knobLeft + 10), px(top + 12)),
                                           px(10), px(10)),
                             brush);
    }
    void drawModernNav(ID2D1SolidColorBrush* brush, int clientHeight) {
        const auto tokens = designTokens();
        modernHits_.clear();
        brush->SetColor(d2dColor(tokens.navigationBackground));
        target_->FillRectangle(D2D1::RectF(0, 0, px(232), static_cast<float>(clientHeight)),
                               brush);
        drawText(brush, L"Fcitx5", 56, 30, 200, 58, 15, DWRITE_FONT_WEIGHT_SEMI_BOLD);
        const struct {
            int page;
            ModernAction action;
            const wchar_t* label;
            int y;
        } items[] = {{kNavGeneral, ModernAction::navGeneral, get("nav.general"), 92},
                     {kNavAppearance, ModernAction::navAppearance, get("nav.appearance"), 136},
                     {kNavTheme, ModernAction::navShortcuts, get("nav.theme"), 180},
                     {kNavPackages, ModernAction::navPackages, get("nav.packages"), 224},
                     {kNavDiagnostics, ModernAction::navUpdates, get("nav.diagnostics"), 292},
                     {kNavRepair, ModernAction::navRepair, get("nav.repair"), 336}};
        brush->SetColor(d2dColor(RGB(214, 218, 226)));
        target_->DrawLine(D2D1::Point2F(px(24), px(268)), D2D1::Point2F(px(208), px(268)),
                          brush, px(1));
        for (const auto& item : items) {
            const bool selected = selectedPage_ == item.page;
            if (selected) {
                brush->SetColor(d2dColor(RGB(255, 255, 255)));
                fillRound(brush, 18, static_cast<float>(item.y), 214,
                          static_cast<float>(item.y + 34), 8);
                brush->SetColor(d2dColor(tokens.accent));
                fillRound(brush, 20, static_cast<float>(item.y + 7), 24,
                          static_cast<float>(item.y + 27), 2);
            }
            brush->SetColor(d2dColor(selected ? tokens.accent : tokens.subtleText));
            drawText(brush, item.label, 38, static_cast<float>(item.y + 7), 204,
                     static_cast<float>(item.y + 31), 14,
                     selected ? DWRITE_FONT_WEIGHT_SEMI_BOLD : DWRITE_FONT_WEIGHT_NORMAL);
            addModernHit(18, item.y, 214, item.y + 34, item.action);
        }
    }
    void drawSectionTitle(ID2D1SolidColorBrush* brush, std::wstring_view title, float y) {
        brush->SetColor(d2dColor(designTokens().text));
        drawText(brush, title, 288, y, modernRowRight(), y + 28, 16,
                 DWRITE_FONT_WEIGHT_SEMI_BOLD);
    }
    void drawSettingRow(ID2D1SolidColorBrush* brush, std::wstring_view title,
                        std::wstring_view description, std::wstring_view value, float y,
                        ModernAction action = ModernAction::none) {
        const auto tokens = designTokens();
        const float rowRight = modernRowRight();
        const float valueLeft = (std::max)(560.0F, rowRight - 230.0F);
        const float textRight = value.empty() ? rowRight - 22.0F : valueLeft - 20.0F;
        brush->SetColor(d2dColor(tokens.surface));
        fillRound(brush, 288, y, rowRight, y + 64, 8);
        brush->SetColor(d2dColor(RGB(226, 230, 236)));
        strokeRound(brush, 288, y, rowRight, y + 64, 8);
        brush->SetColor(d2dColor(tokens.text));
        drawText(brush, title, 310, y + 10, textRight, y + 32, 14,
                 DWRITE_FONT_WEIGHT_SEMI_BOLD);
        brush->SetColor(d2dColor(tokens.subtleText));
        drawText(brush, description, 310, y + 34, textRight, y + 56, 12);
        if (!value.empty()) {
            brush->SetColor(d2dColor(tokens.text));
            drawText(brush, value, valueLeft, y + 18, rowRight - 22.0F, y + 44, 14,
                     DWRITE_FONT_WEIGHT_NORMAL,
                     DWRITE_TEXT_ALIGNMENT_TRAILING);
        }
        if (action != ModernAction::none)
            addModernHit(288, static_cast<int>(y), static_cast<int>(rowRight),
                         static_cast<int>(y + 64), action);
    }
    void drawCompactSetting(ID2D1SolidColorBrush* brush, std::wstring_view title,
                            std::wstring_view value, float left, float top, float right,
                            ModernAction action, bool enabled = true) {
        const auto tokens = designTokens();
        brush->SetColor(d2dColor(enabled ? tokens.surface : RGB(244, 246, 249)));
        fillRound(brush, left, top, right, top + 52, 8);
        brush->SetColor(d2dColor(RGB(226, 230, 236)));
        strokeRound(brush, left, top, right, top + 52, 8);
        brush->SetColor(d2dColor(enabled ? tokens.subtleText : RGB(118, 124, 134)));
        drawText(brush, title, left + 14, top + 8, right - 14, top + 25, 11,
                 DWRITE_FONT_WEIGHT_NORMAL);
        brush->SetColor(d2dColor(enabled ? tokens.text : RGB(118, 124, 134)));
        drawText(brush, value, left + 14, top + 26, right - 14, top + 46, 13,
                 enabled ? DWRITE_FONT_WEIGHT_SEMI_BOLD : DWRITE_FONT_WEIGHT_NORMAL);
        if (enabled && action != ModernAction::none)
            addModernHit(static_cast<int>(left), static_cast<int>(top), static_cast<int>(right),
                         static_cast<int>(top + 52), action);
    }
    void cycleComboAndApply(int comboId) {
        const HWND combo = control(comboId);
        const LRESULT count = SendMessageW(combo, CB_GETCOUNT, 0, 0);
        LRESULT selected = SendMessageW(combo, CB_GETCURSEL, 0, 0);
        if (count <= 0)
            return;
        selected = selected == CB_ERR ? 0 : ((selected + 1) % count);
        SendMessageW(combo, CB_SETCURSEL, static_cast<WPARAM>(selected), 0);
        BOOL handled = FALSE;
        (void)onDirty(0, static_cast<WORD>(comboId), combo, handled);
    }
    void drawSegment(ID2D1SolidColorBrush* brush, std::wstring_view label, float left, float top,
                     float right, bool selected, ModernAction action) {
        const auto tokens = designTokens();
        brush->SetColor(d2dColor(selected ? tokens.accent : RGB(245, 247, 250)));
        fillRound(brush, left, top, right, top + 36, 8);
        brush->SetColor(d2dColor(selected ? RGB(255, 255, 255) : tokens.text));
        drawText(brush, label, left, top + 8, right, top + 30, 14,
                 selected ? DWRITE_FONT_WEIGHT_SEMI_BOLD : DWRITE_FONT_WEIGHT_NORMAL,
                 DWRITE_TEXT_ALIGNMENT_CENTER);
        addModernHit(static_cast<int>(left), static_cast<int>(top), static_cast<int>(right),
                     static_cast<int>(top + 36), action);
    }
    [[nodiscard]] float selectedCandidateTextSize() const {
        const std::wstring text = comboText(kFontSize);
        if (text == L"16")
            return 16.0F;
        if (text == L"20")
            return 20.0F;
        if (text == L"22")
            return 22.0F;
        if (text == L"24")
            return 24.0F;
        return 18.0F;
    }
    [[nodiscard]] float selectedPreviewCornerRadius() const {
        const std::wstring text = comboText(kCornerRadius);
        if (text == L"0")
            return 0.0F;
        if (text == L"8")
            return 8.0F;
        if (text == L"16")
            return 16.0F;
        if (text == L"24")
            return 24.0F;
        return 12.0F;
    }
    [[nodiscard]] float selectedPreviewOpacity() const {
        const std::wstring text = comboText(kOpacity);
        if (text == L"0.75")
            return 0.75F;
        if (text == L"0.85")
            return 0.85F;
        if (text == L"0.90")
            return 0.90F;
        if (text == L"0.95")
            return 0.95F;
        return 1.0F;
    }
    [[nodiscard]] const wchar_t* candidatePreviewSampleText() const noexcept {
        return L"ni hao 😊 ，。！？ 你 你好 输入法 fcitx Windows Next 😀 🎉 ⌨️";
    }
    [[nodiscard]] bool candidatePreviewSampleCoversRequiredContent() const {
        const std::wstring_view sample(candidatePreviewSampleText());
        return sample.find(L"你") != std::wstring_view::npos &&
               sample.find(L"fcitx") != std::wstring_view::npos &&
               sample.find(L"，。！？") != std::wstring_view::npos &&
               sample.find(L"😀") != std::wstring_view::npos &&
               sample.find(L"🎉") != std::wstring_view::npos &&
               sample.find(L"⌨️") != std::wstring_view::npos;
    }
    void drawPreviewPill(ID2D1SolidColorBrush* brush, std::wstring_view label, float left,
                         float top, float right, COLORREF fill, COLORREF text) {
        brush->SetColor(d2dColor(fill));
        fillRound(brush, left, top, right, top + 22, 11);
        brush->SetColor(d2dColor(text));
        drawText(brush, label, left + 10, top + 3, right - 10, top + 20, 11,
                 DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_TEXT_ALIGNMENT_CENTER);
    }
    void drawCandidateEntry(ID2D1SolidColorBrush* brush, std::wstring_view label,
                            std::wstring_view text, std::wstring_view comment,
                            const fcitx::windows::ui::RenderItemSegments& segments,
                            float textSize, float commentSize, COLORREF labelText,
                            COLORREF candidateText, COLORREF commentText,
                            std::wstring_view family) {
        brush->SetColor(d2dColor(labelText));
        drawText(brush, label, segments.label.left, segments.label.top, segments.label.right,
                 segments.label.bottom, textSize,
                 DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_TEXT_ALIGNMENT_LEADING,
                 DWRITE_PARAGRAPH_ALIGNMENT_CENTER, family);
        brush->SetColor(d2dColor(candidateText));
        drawText(brush, text, segments.text.left, segments.text.top, segments.text.right,
                 segments.text.bottom, textSize,
                 DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_TEXT_ALIGNMENT_LEADING,
                 DWRITE_PARAGRAPH_ALIGNMENT_CENTER, family);
        if (segments.drawComment && !comment.empty()) {
            brush->SetColor(d2dColor(commentText));
            drawText(brush, comment, segments.comment.left, segments.comment.top,
                     segments.comment.right, segments.comment.bottom, commentSize,
                     DWRITE_FONT_WEIGHT_NORMAL, DWRITE_TEXT_ALIGNMENT_LEADING,
                     DWRITE_PARAGRAPH_ALIGNMENT_CENTER, family);
        }
    }
    void drawCandidatePreview(ID2D1SolidColorBrush* brush, bool automatic, bool vertical,
                              LRESULT mode, float rowRight) {
        const bool darkMode = mode == 2;
        const bool lightMode = mode == 1;
        const bool verticalLayout = !automatic && vertical;
        const float previewTop = 148.0F;
        const float previewBottom = 260.0F;
        const float previewLeft = 288.0F;
        const float previewRight = rowRight;
        const float radius = selectedPreviewCornerRadius();
        const float opacity = selectedPreviewOpacity();
        const std::wstring fontFamily = comboText(kFont).empty() ? L"Segoe UI" : comboText(kFont);
        const float textSize = selectedCandidateTextSize();
        const float previewTextSize = (std::min)(22.0F, textSize);
        const float previewCommentSize = (std::max)(10.0F, previewTextSize * 0.82F);
        const COLORREF surface =
            darkMode ? RGB(35, 37, 42) : (lightMode ? RGB(252, 253, 255) : RGB(246, 249, 252));
        const COLORREF border = darkMode ? RGB(85, 91, 102) : RGB(226, 230, 236);
        const COLORREF primaryText = darkMode ? RGB(248, 250, 252) : designTokens().text;
        const COLORREF subtleText = darkMode ? RGB(196, 204, 216) : designTokens().subtleText;
        const COLORREF pillFill = darkMode ? RGB(56, 63, 72) : RGB(234, 239, 246);
        const COLORREF labelText = darkMode ? RGB(115, 235, 176) : designTokens().accent;
        const COLORREF selectionFill =
            darkMode ? RGB(78, 92, 115) : RGB(221, 235, 255);
        const auto colorWithAlpha = [](COLORREF color, float alpha) {
            return D2D1::ColorF(GetRValue(color) / 255.0F, GetGValue(color) / 255.0F,
                                GetBValue(color) / 255.0F, alpha);
        };

        brush->SetColor(colorWithAlpha(surface, opacity));
        fillRound(brush, previewLeft, previewTop, previewRight, previewBottom, radius);
        brush->SetColor(d2dColor(border));
        strokeRound(brush, previewLeft, previewTop, previewRight, previewBottom, radius);

        const wchar_t* const modeLabel =
            darkMode ? modernText(L"Dark", L"深色")
                     : (lightMode ? modernText(L"Light", L"浅色")
                                  : modernText(L"System", L"系统"));
        const wchar_t* const layoutLabel =
            automatic ? modernText(L"Auto · horizontal", L"自动 · 横排")
            : verticalLayout ? modernText(L"Vertical", L"竖排")
                             : modernText(L"Horizontal", L"横排");
        drawPreviewPill(brush, modeLabel, previewRight - 216.0F, previewTop + 12.0F,
                        previewRight - 132.0F, pillFill, primaryText);
        drawPreviewPill(brush, layoutLabel, previewRight - 124.0F, previewTop + 12.0F,
                        previewRight - 18.0F, pillFill, primaryText);

        brush->SetColor(d2dColor(subtleText));
        const float preeditY = previewTop + 14.0F;
        drawText(brush, L"ni hao 😊  ，。！？", previewLeft + 28.0F, preeditY,
                 previewRight - 232.0F, preeditY + 26.0F, 13,
                 DWRITE_FONT_WEIGHT_NORMAL, DWRITE_TEXT_ALIGNMENT_LEADING,
                 DWRITE_PARAGRAPH_ALIGNMENT_CENTER, fontFamily);

        struct PreviewCandidate {
            std::wstring_view label;
            std::wstring_view text;
            std::wstring_view comment;
        };
        const std::array verticalCandidates{
            PreviewCandidate{L"1", L"输入法", L"shūrùfǎ"},
            PreviewCandidate{L"2", L"输入", L"shūrù"},
            PreviewCandidate{L"3", L"中文", L"zhōngwén"},
        };
        const std::array horizontalCandidates{
            PreviewCandidate{L"1", L"输入法", L"shūrùfǎ"},
            PreviewCandidate{L"2", L"输入", L"shūrù"},
            PreviewCandidate{L"3", L"中文", L"zhōngwén"},
        };
        const auto drawProductionLayoutPreview = [&](const auto& candidates,
                                                     fcitx::windows::ui::Orientation orientation) {
            std::vector<fcitx::windows::ui::Size> itemSizes;
            itemSizes.reserve(candidates.size());
            for (const auto& candidate : candidates) {
                const float labelWidth =
                    measureTextWidth(candidate.label, previewTextSize,
                                     DWRITE_FONT_WEIGHT_SEMI_BOLD, fontFamily);
                const float textWidth =
                    measureTextWidth(candidate.text, previewTextSize,
                                     DWRITE_FONT_WEIGHT_SEMI_BOLD, fontFamily);
                const float commentWidth =
                    measureTextWidth(candidate.comment, previewCommentSize,
                                     DWRITE_FONT_WEIGHT_NORMAL, fontFamily);
                itemSizes.push_back({labelWidth + textWidth + commentWidth + 36.0F, 28.0F});
            }
            fcitx::windows::ui::LayoutInput input{
                orientation,
                std::move(itemSizes),
                {previewLeft + 28.0F, previewTop + 44.0F},
                0.0F,
                {previewLeft + 28.0F, previewTop + 44.0F, previewRight - 18.0F,
                 previewBottom - 10.0F},
                previewRight - previewLeft - 46.0F,
                0.0F,
                0.0F,
                4.0F,
                12.0F,
                fcitx::windows::ui::Placement::below};
            const auto layout = fcitx::windows::ui::layout(input);
            std::vector<fcitx::windows::ui::RenderItemInput> renderInputs;
            renderInputs.reserve(layout.items.size());
            for (std::size_t local = 0; local < layout.items.size(); ++local) {
                const auto sourceIndex = layout.itemIndices[local];
                if (sourceIndex >= candidates.size())
                    continue;
                const auto& candidate = candidates[sourceIndex];
                const auto& rectangle = layout.items[local];
                renderInputs.push_back({
                    rectangle,
                    measureTextWidth(candidate.label, previewTextSize,
                                     DWRITE_FONT_WEIGHT_SEMI_BOLD, fontFamily),
                    measureTextWidth(candidate.text, previewTextSize,
                                     DWRITE_FONT_WEIGHT_SEMI_BOLD, fontFamily),
                    measureTextWidth(candidate.comment, previewCommentSize,
                                     DWRITE_FONT_WEIGHT_NORMAL, fontFamily),
                    !candidate.label.empty(),
                });
            }
            const auto renderSegments =
                fcitx::windows::ui::renderSegments(orientation, false, renderInputs);
            for (std::size_t local = 0; local < layout.items.size(); ++local) {
                const auto sourceIndex = layout.itemIndices[local];
                if (sourceIndex >= candidates.size() || local >= renderSegments.size())
                    continue;
                const auto& rectangle = layout.items[local];
                const auto& candidate = candidates[sourceIndex];
                if (sourceIndex == 0) {
                    brush->SetColor(d2dColor(selectionFill));
                    fillRound(brush, rectangle.left - 4.0F, rectangle.top - 2.0F,
                              rectangle.right + 4.0F, rectangle.bottom + 2.0F, radius);
                }
                drawCandidateEntry(brush, candidate.label, candidate.text, candidate.comment,
                                   renderSegments[local], previewTextSize, previewCommentSize,
                                   labelText, primaryText, subtleText, fontFamily);
            }
        };
        if (verticalLayout) {
            drawProductionLayoutPreview(verticalCandidates,
                                        fcitx::windows::ui::Orientation::vertical);
        } else {
            drawProductionLayoutPreview(horizontalCandidates,
                                        fcitx::windows::ui::Orientation::horizontal);
        }
    }
    void drawModernAdvancedAppearance(ID2D1SolidColorBrush* brush, float rowRight) {
        const auto boolText = [&](bool enabled) {
            return enabled ? modernText(L"Enabled", L"已启用")
                           : modernText(L"Disabled", L"已禁用");
        };
        drawSectionTitle(brush, modernText(L"Supported renderer settings", L"已支持的渲染设置"),
                         270);
        const float gap = 12.0F;
        const float cardWidth = (rowRight - 288.0F - (gap * 2.0F)) / 3.0F;
        const float x0 = 288.0F;
        const float x1 = x0 + cardWidth + gap;
        const float x2 = x1 + cardWidth + gap;
        const float y0 = 304.0F;
        const float y1 = 366.0F;
        const float y2 = 428.0F;
        drawCompactSetting(brush, modernText(L"Page size", L"每页候选"), comboText(kPageSize),
                           x0, y0, x0 + cardWidth, ModernAction::cyclePageSize);
        drawCompactSetting(brush, modernText(L"Max width", L"最大宽度"), comboText(kMaxWidth),
                           x1, y0, x1 + cardWidth, ModernAction::cycleMaxWidth);
        drawCompactSetting(brush, modernText(L"Scroll cell", L"滚动单元"),
                           comboText(kScrollCellWidth), x2, y0, rowRight,
                           ModernAction::cycleScrollCellWidth);
        drawCompactSetting(brush, modernText(L"Corner radius", L"圆角"), comboText(kCornerRadius),
                           x0, y1, x0 + cardWidth, ModernAction::cycleCornerRadius);
        drawCompactSetting(brush, modernText(L"Opacity", L"透明度"), comboText(kOpacity),
                           x1, y1, x1 + cardWidth, ModernAction::cycleOpacity);
        drawCompactSetting(brush, modernText(L"Shadow", L"阴影"),
                           boolText(SendMessageW(control(kShadow), BM_GETCHECK, 0, 0) ==
                                    BST_CHECKED),
                           x2, y1, rowRight, ModernAction::toggleShadow);
        drawCompactSetting(brush, modernText(L"Preedit", L"预编辑"),
                           comboText(kPreeditMode), x0, y2, x0 + cardWidth,
                           ModernAction::cyclePreeditMode);
        drawCompactSetting(brush, modernText(L"Font size", L"字体大小"), comboText(kFontSize),
                           x1, y2, x1 + cardWidth, ModernAction::cycleFontSize);
        drawCompactSetting(brush, modernText(L"Scroll mode", L"滚动模式"),
                           boolText(SendMessageW(control(kScrollMode), BM_GETCHECK, 0, 0) ==
                                    BST_CHECKED),
                           x2, y2, rowRight, ModernAction::toggleScrollMode);
        drawCompactSetting(brush, modernText(L"Font family", L"字体族"), comboText(kFont),
                           x0, 490.0F, x1 + cardWidth, ModernAction::editFont);

        brush->SetColor(d2dColor(designTokens().subtleText));
        drawText(brush,
                 modernText(L"Annotation, label, colors, and spacing: supported by theme files; Settings controls pending.",
                            L"注释、标签、颜色、间距：主题文件已支持，设置控件待补。"),
                 x0, 552.0F, rowRight, 584.0F, 12);
    }
    void drawModernGeneral(ID2D1SolidColorBrush* brush) {
        const bool startup =
            SendMessageW(control(kStartup), BM_GETCHECK, 0, 0) == BST_CHECKED;
        drawSectionTitle(brush, modernText(L"Startup", L"启动"), 114);
        drawSettingRow(brush, modernText(L"Start Fcitx5 after Windows sign-in",
                                         L"登录 Windows 后启动 Fcitx5"),
                       modernText(L"Starts the launcher for your user session.",
                                  L"为当前用户会话启动输入法服务。"),
                       L"", 148, ModernAction::toggleStartup);
        drawToggle(brush, startup, modernRowRight() - 66.0F, 168);
        drawSectionTitle(brush, modernText(L"Enabled input methods", L"已启用的输入法"), 248);
        float y = 282;
        if (inputMethods_.empty()) {
            drawSettingRow(brush, modernText(L"No input methods loaded", L"未读取到输入法"),
                           modernText(L"Use Repair if this stays empty after installation.",
                                      L"安装后仍为空时，请使用诊断与修复。"),
                           L"", y);
            drawSettingRow(brush, get("operation.input_methods.refresh"),
                           modernText(L"Reload the enabled input method list.",
                                      L"重新读取已启用输入法列表。"),
                           L"›", y + 76, ModernAction::inputMethodRefresh);
            return;
        }
        for (std::size_t index = 0; index < inputMethods_.size() && index < 4; ++index) {
            const auto& method = inputMethods_[index];
            const std::wstring value =
                method.selected ? modernText(L"Current", L"当前") : L"";
            drawSettingRow(brush, method.nativeName.empty() ? method.name : method.nativeName,
                           method.name, value, y, ModernAction::inputMethodCard);
            modernHits_.back().index = static_cast<int>(index);
            y += 76;
        }
        brush->SetColor(d2dColor(designTokens().accent));
        drawSettingRow(brush, get("operation.input_methods.refresh"),
                       modernText(L"Reload the enabled input method list.",
                                  L"重新读取已启用输入法列表。"),
                       L"›", y, ModernAction::inputMethodRefresh);
    }
    void drawModernAppearance(ID2D1SolidColorBrush* brush) {
        const auto mode = SendMessageW(control(kAppearance), CB_GETCURSEL, 0, 0);
        const bool automatic =
            SendMessageW(control(kAutomatic), BM_GETCHECK, 0, 0) == BST_CHECKED;
        const bool vertical = SendMessageW(control(kVertical), BM_GETCHECK, 0, 0) == BST_CHECKED;
        drawSectionTitle(brush, modernText(L"Candidate preview", L"候选框预览"), 114);
        const float rowRight = modernRowRight();
        drawCandidatePreview(brush, automatic, vertical, mode, rowRight);
        addModernHit(288, 148, static_cast<int>(rowRight), 260, ModernAction::preview);
        if (appearanceAdvanced_) {
            drawModernAdvancedAppearance(brush, rowRight);
            return;
        }
        drawSectionTitle(brush, modernText(L"Theme", L"主题"), 270);
        const float segmentWidth = (std::min)(174.0F, (rowRight - 288.0F - 32.0F) / 3.0F);
        drawSegment(brush, modernText(L"Follow system", L"跟随系统"), 288, 302,
                    288 + segmentWidth, mode == 0,
                    ModernAction::selectModeSystem);
        drawSegment(brush, modernText(L"Light", L"浅色"), 304 + segmentWidth, 302,
                    304 + (segmentWidth * 2), mode == 1,
                    ModernAction::selectModeLight);
        drawSegment(brush, modernText(L"Dark", L"深色"), 320 + (segmentWidth * 2), 302,
                    320 + (segmentWidth * 3), mode == 2,
                    ModernAction::selectModeDark);
        drawSectionTitle(brush, modernText(L"Candidate layout", L"候选布局"), 346);
        brush->SetColor(d2dColor(designTokens().subtleText));
        drawText(brush, modernText(L"Automatically adapts to input method and candidate content.",
                                   L"根据输入法和候选内容自动选择。"),
                 288, 374, 920, 396, 12);
        drawSegment(brush, get("candidate.automatic"), 288, 410, 420, automatic,
                    ModernAction::selectLayoutAutomatic);
        drawSegment(brush, get("candidate.horizontal"), 436, 410, 568,
                    !automatic && !vertical, ModernAction::selectLayoutHorizontal);
        drawSegment(brush, get("candidate.vertical"), 584, 410, 716, !automatic && vertical,
                    ModernAction::selectLayoutVertical);
        drawSettingRow(brush, modernText(L"Text size", L"文字大小"),
                       modernText(L"Candidate text size.", L"候选文字大小。"),
                       comboText(kFontSize), 476, ModernAction::cycleFontSize);
        drawSettingRow(brush, modernText(L"Font", L"字体"),
                       modernText(L"Preview exposes CJK and emoji fallback.",
                                  L"预览会暴露中文和 emoji fallback。"),
                       comboText(kFont), 552, ModernAction::editFont);
        drawSettingRow(brush, modernText(L"Advanced appearance", L"高级外观"),
                       modernText(L"Expand supported renderer tuning.", L"展开已支持的渲染参数。"),
                       L"›", 628, ModernAction::toggleTechnicalDetails);
    }
    void drawModernShortcuts(ID2D1SolidColorBrush* brush) {
        drawSectionTitle(brush, modernText(L"Keyboard shortcuts", L"快捷键"), 114);
        drawSettingRow(brush, modernText(L"Managed by the active input method",
                                         L"由当前输入法管理"),
                       modernText(L"This page stays reachable for future shortcut settings.",
                                  L"此页面保留为后续快捷键设置入口。"),
                       L"", 148);
    }
    void drawModernUpdates(ID2D1SolidColorBrush* brush) {
        drawSectionTitle(brush, modernText(L"Fcitx5 for Windows Next", L"Fcitx5 for Windows Next"),
                         114);
        drawSettingRow(brush, modernText(L"Version", L"版本"),
                       modernText(L"Installed application version.", L"当前安装的应用版本。"),
                       widen(fcitx::windows::version()), 148);
        drawSettingRow(brush, modernText(L"Update status", L"更新状态"),
                       repositoryAvailable_
                           ? modernText(L"Component repository is reachable.",
                                        L"组件仓库可用。")
                           : modernText(L"No online repository is currently available.",
                                        L"当前没有可用在线仓库。"),
                       repositoryAvailable_ ? L"✓" : L"—", 224);
        drawSettingRow(brush, modernText(L"Check for updates", L"检查更新"),
                       modernText(L"Refresh product and component metadata.",
                                  L"刷新产品与组件元数据。"),
                       L"›", 300, ModernAction::packageRefresh);
        drawSettingRow(brush, modernText(L"Automatic updates", L"自动更新"),
                       modernText(L"Keep the application and components current.",
                                  L"保持应用和组件为最新状态。"),
                       L"", 376);
        drawToggle(brush, true, modernRowRight() - 66.0F, 396);
        drawSettingRow(brush, modernText(L"Update channel", L"更新通道"),
                       modernText(L"Stable channel", L"稳定版通道"), L"Stable", 452);
        drawSettingRow(brush, modernText(L"Component updates", L"组件更新"),
                       modernText(L"Input methods, add-ons, and dictionaries.",
                                  L"输入法、插件和词库。"),
                       L"›", 528, ModernAction::navPackages);
    }
    void drawModernRepair(ID2D1SolidColorBrush* brush) {
        drawSectionTitle(brush, modernText(L"System status", L"系统状态"), 114);
        const wchar_t* ok = L"✓";
        drawSettingRow(brush, modernText(L"TSF registration", L"TSF 注册"),
                       modernText(L"Single Fcitx5 profile is registered.", L"单一 Fcitx5 profile 已注册。"),
                       ok, 148);
        drawSettingRow(brush, modernText(L"Engine", L"输入引擎"),
                       modernText(L"Engine process can be restarted by the launcher.",
                                  L"Launcher 可重启输入引擎进程。"),
                       ok, 224);
        drawSettingRow(brush, modernText(L"Candidate UI", L"候选 UI"),
                       modernText(L"Renderer resources are available.", L"渲染资源可用。"),
                       ok, 300);
        drawSettingRow(brush, modernText(L"Configuration", L"配置"),
                       modernText(L"Typed configuration parser is available.", L"类型化配置解析可用。"),
                       ok, 376);
        drawSettingRow(brush, modernText(L"Recheck", L"重新检查"),
                       modernText(L"Refresh diagnostics without changing user dictionaries.",
                                  L"刷新诊断，不删除用户词库。"),
                       L"›", 472, ModernAction::diagnostics);
        drawSettingRow(brush, modernText(L"Repair Fcitx5", L"修复 Fcitx5"),
                       modernText(L"Re-register input method and restore required components.",
                                  L"重新注册输入法并恢复必要组件。"),
                       L"›", 548, ModernAction::repair);
    }
    void drawModernPackages(ID2D1SolidColorBrush* brush) {
        drawSectionTitle(brush, modernText(L"Repository", L"仓库"), 114);
        const std::wstring trustMessage =
            repositoryAvailable_
                ? modernText(L"Only signed compatible Windows packages are shown.",
                             L"仅显示已签名且兼容 Windows 的组件包。")
                : repositoryTrustMessage(repositoryAvailable_, repositoryError_, strings_);
        drawSettingRow(brush,
                       repositoryAvailable_
                           ? modernText(L"Trusted online repository", L"受信任在线仓库")
                           : modernText(L"Official repository not configured", L"官方仓库尚未配置"),
                       trustMessage,
                       repositoryAvailable_ ? L"✓" : L"—", 148, ModernAction::packageRefresh);

        drawSectionTitle(brush, modernText(L"Components", L"组件"), 224);
        float y = 258;
        for (std::size_t index = 0; index < packages_.size() && index < 2; ++index) {
            const auto& package = packages_[index];
            std::wstring meta =
                (package.installed.empty() ? L"—" : package.installed) + L" · " +
                packageTypeLabel(package, strings_);
            if (!package.available.empty() && package.available != package.installed)
                meta += L" · " + package.available;
            drawSettingRow(brush, package.title.empty() ? package.id : package.title,
                           package.summary.empty() ? meta : package.summary,
                           packageStateLabel(package, strings_, repositoryAvailable_), y,
                           ModernAction::packageCard);
            modernHits_.back().index = static_cast<int>(index);
            y += 76;
        }
        if (packages_.empty()) {
            drawSettingRow(brush, modernText(L"No installed components", L"没有已安装组件"),
                            modernText(L"Use refresh after installation completes.",
                                       L"安装完成后可刷新组件列表。"),
                            L"", y);
            drawSettingRow(brush, modernText(L"Refresh components", L"刷新组件"),
                           modernText(L"Reload installed add-ons and repository metadata.",
                                      L"重新读取已安装插件和仓库元数据。"),
                           L"›", y + 76, ModernAction::packageRefresh);
            return;
        }
        drawSectionTitle(brush, modernText(L"Selected component", L"选中组件"), 430);
        const int selected = selectedPackage();
        const PackageRow* package =
            selected < 0 ? nullptr : &packages_[static_cast<std::size_t>(selected)];
        const bool bundled = package && package->state == L"bundled";
        const bool unsafe =
            package && (bundled || packageUnsafeForInstall(*package, repositoryAvailable_));
        const bool hasAvailable =
            package && !unsafe && !bundled && repositoryAvailable_ && !package->available.empty();
        const bool hasInstalled = package && !bundled && !package->installed.empty() &&
                                  package->state != L"trust-failed" &&
                                  package->state != L"incompatible";
        const std::wstring unavailableReason =
            !package ? modernText(L"Select a component first.", L"请先选择组件。")
            : bundled ? get("packages.bundled_readonly")
            : package->state == L"trust-failed" ? get("packages.trust_failed")
            : package->state == L"incompatible" ? packageStateLabel(*package, strings_)
            : package->state == L"pending-restart" ? packageStateLabel(*package, strings_)
            : !repositoryAvailable_ ? repositoryTrustMessage(repositoryAvailable_,
                                                             repositoryError_, strings_)
                                    : modernText(L"No signed compatible package is available.",
                                                 L"没有可用的已签名兼容组件包。");
        const float gap = 12.0F;
        const float cardWidth = (modernRowRight() - 288.0F - (gap * 2.0F)) / 3.0F;
        const float x0 = 288.0F;
        const float x1 = x0 + cardWidth + gap;
        const float x2 = x1 + cardWidth + gap;
        drawCompactSetting(brush, package && package->installed.empty()
                                      ? modernText(L"Install", L"安装")
                                      : modernText(L"Update", L"更新"),
                           hasAvailable ? L"›" : L"—", x0, 464, x0 + cardWidth,
                           hasAvailable ? ModernAction::packageInstallOrUpdate
                                        : ModernAction::none,
                           hasAvailable);
        drawCompactSetting(brush, package && package->state == L"disabled"
                                      ? modernText(L"Enable", L"启用")
                                      : modernText(L"Disable", L"禁用"),
                           hasInstalled ? L"›" : L"—", x1, 464, x1 + cardWidth,
                           hasInstalled ? ModernAction::packageToggle : ModernAction::none,
                           hasInstalled);
        drawCompactSetting(brush, modernText(L"Remove", L"卸载"),
                           hasInstalled ? L"›" : L"—", x2, 464, modernRowRight(),
                           hasInstalled ? ModernAction::packageRemove : ModernAction::none,
                           hasInstalled);
        brush->SetColor(d2dColor(designTokens().subtleText));
        drawText(brush,
                 (hasAvailable || hasInstalled)
                     ? modernText(L"Actions return to the last confirmed inventory state on failure.",
                                  L"操作失败时会回到最后确认的组件状态。")
                     : unavailableReason,
                 288, 528, modernRowRight(), 560, 12);
    }
    void drawModernPage(ID2D1SolidColorBrush* brush) {
        const auto tokens = designTokens();
        brush->SetColor(d2dColor(tokens.surface));
        fillRound(brush, 256, 32, modernSurfaceRight(), 688, 10);
        brush->SetColor(d2dColor(tokens.text));
        const std::wstring pageTitle =
            selectedPage_ == kNavGeneral       ? get("nav.general")
            : selectedPage_ == kNavAppearance  ? get("nav.appearance")
            : selectedPage_ == kNavTheme       ? get("nav.theme")
            : selectedPage_ == kNavDiagnostics ? get("nav.diagnostics")
            : selectedPage_ == kNavRepair      ? get("nav.repair")
                                               : get("nav.packages");
        drawText(brush, pageTitle, 288, 64, modernRowRight(), 102, 28,
                 DWRITE_FONT_WEIGHT_SEMI_BOLD);
        switch (selectedPage_) {
        case kNavGeneral:
            drawModernGeneral(brush);
            break;
        case kNavAppearance:
            drawModernAppearance(brush);
            break;
        case kNavTheme:
            drawModernShortcuts(brush);
            break;
        case kNavDiagnostics:
            drawModernUpdates(brush);
            break;
        case kNavRepair:
            drawModernRepair(brush);
            break;
        case kNavPackages:
            drawModernPackages(brush);
            break;
        }
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
            const auto tokens = designTokens();
            target_->BeginDraw();
            target_->Clear(d2dColor(tokens.appBackground));
            ID2D1SolidColorBrush* brush = nullptr;
            target_->CreateSolidColorBrush(d2dColor(tokens.navigationBackground), &brush);
            if (brush) {
                RECT client{};
                GetClientRect(&client);
                drawModernNav(brush, client.bottom);
                drawModernPage(brush);
                brush->Release();
            }
            if (target_->EndDraw() == D2DERR_RECREATE_TARGET) {
                target_->Release();
                target_ = nullptr;
            }
        }
        EndPaint(&paint);
        return 0;
    }
    LRESULT onDrawItem(UINT, WPARAM, LPARAM lparam, BOOL&) {
        const auto* item = reinterpret_cast<const DRAWITEMSTRUCT*>(lparam);
        if (!item || item->CtlID < kNavGeneral || item->CtlID > kNavPackages)
            return FALSE;
        const auto tokens = designTokens();
        const bool selected = static_cast<int>(item->CtlID) == selectedPage_;
        HBRUSH background =
            CreateSolidBrush(selected ? tokens.surface : tokens.navigationBackground);
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
        SetTextColor(item->hDC, selected ? tokens.accent : tokens.subtleText);
        RECT text = item->rcItem;
        text.left += 16;
        wchar_t label[128]{};
        ::GetWindowTextW(item->hwndItem, label, static_cast<int>(std::size(label)));
        DrawTextW(item->hDC, label, -1, &text, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        if ((item->itemState & ODS_FOCUS) != 0)
            DrawFocusRect(item->hDC, &item->rcItem);
        return TRUE;
    }
    void invokeModernAction(const ModernHitTarget& hit) {
        BOOL handled = FALSE;
        switch (hit.action) {
        case ModernAction::navGeneral:
            showPage(kNavGeneral);
            break;
        case ModernAction::navAppearance:
            showPage(kNavAppearance);
            break;
        case ModernAction::navShortcuts:
            showPage(kNavTheme);
            break;
        case ModernAction::navUpdates:
            showPage(kNavDiagnostics);
            break;
        case ModernAction::navRepair:
            showPage(kNavRepair);
            break;
        case ModernAction::navPackages:
            showPage(kNavPackages);
            break;
        case ModernAction::toggleStartup:
            SendMessageW(control(kStartup), BM_CLICK, 0, 0);
            break;
        case ModernAction::inputMethodRefresh:
            if (interactionTest_) {
                cover(kCoveredInputMethodRefresh);
            } else if (loadInputMethods()) {
                setSaveStatus(get("operation.status.success"));
            } else {
                setSaveStatus(get("error.command"));
            }
            break;
        case ModernAction::inputMethodCard:
            if (hit.index >= 0 && static_cast<std::size_t>(hit.index) < inputMethods_.size()) {
                SendMessageW(control(kInputMethod), CB_SETCURSEL,
                             static_cast<WPARAM>(hit.index), 0);
                if (interactionTest_) {
                    for (auto& method : inputMethods_)
                        method.selected = false;
                    inputMethods_[static_cast<std::size_t>(hit.index)].selected = true;
                    cover(kCoveredInputMethodSetDefault);
                    setSaveStatus(get("status.saved"));
                } else {
                    const bool ok = applyInputMethod();
                    setSaveStatus(ok ? get("status.saved") : get("error.command"));
                }
            }
            break;
        case ModernAction::selectModeSystem:
        case ModernAction::selectModeLight:
        case ModernAction::selectModeDark: {
            const int index = hit.action == ModernAction::selectModeLight
                                  ? 1
                                  : (hit.action == ModernAction::selectModeDark ? 2 : 0);
            SendMessageW(control(kAppearance), CB_SETCURSEL, index, 0);
            (void)onDirty(0, kAppearance, control(kAppearance), handled);
            break;
        }
        case ModernAction::selectLayoutAutomatic:
        case ModernAction::selectLayoutVertical:
        case ModernAction::selectLayoutHorizontal: {
            SendMessageW(control(kAutomatic), BM_SETCHECK,
                         hit.action == ModernAction::selectLayoutAutomatic ? BST_CHECKED
                                                                           : BST_UNCHECKED,
                         0);
            SendMessageW(control(kVertical), BM_SETCHECK,
                         hit.action == ModernAction::selectLayoutVertical ? BST_CHECKED
                                                                          : BST_UNCHECKED,
                         0);
            SendMessageW(control(kHorizontal), BM_SETCHECK,
                         hit.action == ModernAction::selectLayoutHorizontal ? BST_CHECKED
                                                                            : BST_UNCHECKED,
                         0);
            (void)onDirty(0, kAutomatic, control(kAutomatic), handled);
            break;
        }
        case ModernAction::cyclePageSize:
            cycleComboAndApply(kPageSize);
            break;
        case ModernAction::cycleMaxWidth:
            cycleComboAndApply(kMaxWidth);
            break;
        case ModernAction::cycleScrollCellWidth:
            cycleComboAndApply(kScrollCellWidth);
            break;
        case ModernAction::cycleFontSize: {
            if (interactionTest_)
                cover(kCoveredFontSize);
            cycleComboAndApply(kFontSize);
            break;
        }
        case ModernAction::cycleCornerRadius:
            cycleComboAndApply(kCornerRadius);
            break;
        case ModernAction::cycleOpacity:
            cycleComboAndApply(kOpacity);
            break;
        case ModernAction::cyclePreeditMode:
            cycleComboAndApply(kPreeditMode);
            break;
        case ModernAction::toggleShadow:
            SendMessageW(control(kShadow), BM_SETCHECK,
                         SendMessageW(control(kShadow), BM_GETCHECK, 0, 0) == BST_CHECKED
                             ? BST_UNCHECKED
                             : BST_CHECKED,
                         0);
            (void)onDirty(0, kShadow, control(kShadow), handled);
            break;
        case ModernAction::toggleScrollMode:
            SendMessageW(control(kScrollMode), BM_SETCHECK,
                         SendMessageW(control(kScrollMode), BM_GETCHECK, 0, 0) == BST_CHECKED
                             ? BST_UNCHECKED
                             : BST_CHECKED,
                         0);
            (void)onDirty(0, kScrollMode, control(kScrollMode), handled);
            break;
        case ModernAction::editFont:
            if (interactionTest_)
                cover(kCoveredFontEdit);
            showModernFontEditor();
            break;
        case ModernAction::preview:
            (void)onPreview(0, 0, nullptr, handled);
            break;
        case ModernAction::resetAppearance:
            (void)onResetAppearance(0, 0, nullptr, handled);
            break;
        case ModernAction::packageRefresh:
            (void)onPackageRefresh(0, 0, nullptr, handled);
            break;
        case ModernAction::packageInstallOrUpdate:
            (void)onPackageInstall(0, 0, nullptr, handled);
            break;
        case ModernAction::packageToggle:
            (void)onPackageToggle(0, 0, nullptr, handled);
            break;
        case ModernAction::packageRemove:
            (void)onPackageRemove(0, 0, nullptr, handled);
            break;
        case ModernAction::diagnostics:
            (void)onDiagnostics(0, 0, nullptr, handled);
            break;
        case ModernAction::repair:
            (void)onRepair(0, 0, nullptr, handled);
            break;
        case ModernAction::toggleTechnicalDetails:
            appearanceAdvanced_ = !appearanceAdvanced_;
            SendMessageW(control(kAppearanceAdvanced), BM_SETCHECK,
                         appearanceAdvanced_ ? BST_CHECKED : BST_UNCHECKED, 0);
            showPage(kNavAppearance);
            break;
        case ModernAction::packageCard:
            if (hit.index >= 0 && static_cast<std::size_t>(hit.index) < packages_.size()) {
                SendMessageW(control(kPackages), LB_SETCURSEL, hit.index, 0);
                updatePackageDetail();
            }
            break;
        case ModernAction::none:
            break;
        }
        InvalidateRect(nullptr, TRUE);
    }
    LRESULT onModernClick(UINT, WPARAM, LPARAM lparam, BOOL&) {
        const POINT point{GET_X_LPARAM(lparam), GET_Y_LPARAM(lparam)};
        for (const auto& hit : modernHits_) {
            if (::PtInRect(&hit.rect, point)) {
                invokeModernAction(hit);
                return 0;
            }
        }
        return 0;
    }
    LRESULT onModernKeyDown(UINT, WPARAM wparam, LPARAM, BOOL&) {
        if (wparam != VK_DOWN && wparam != VK_UP && wparam != VK_RETURN && wparam != VK_SPACE)
            return 0;
        if (wparam == VK_DOWN || wparam == VK_UP) {
            const int pages[] = {kNavGeneral, kNavAppearance, kNavTheme,
                                 kNavPackages, kNavDiagnostics, kNavRepair};
            int current = 0;
            for (int index = 0; index < static_cast<int>(std::size(pages)); ++index) {
                if (pages[index] == selectedPage_) {
                    current = index;
                    break;
                }
            }
            current += wparam == VK_DOWN ? 1 : -1;
            if (current < 0)
                current = static_cast<int>(std::size(pages)) - 1;
            if (current >= static_cast<int>(std::size(pages)))
                current = 0;
            showPage(pages[current]);
        }
        return 0;
    }
    LRESULT onColorStatic(UINT, WPARAM wparam, LPARAM, BOOL&) {
        const auto tokens = designTokens();
        const HDC dc = reinterpret_cast<HDC>(wparam);
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, tokens.text);
        return reinterpret_cast<LRESULT>(GetStockObject(HOLLOW_BRUSH));
    }
    LRESULT onDestroy(UINT, WPARAM, LPARAM, BOOL&) {
        if (statusTimer_)
            ::KillTimer(m_hWnd, statusTimer_);
        if (font_)
            DeleteObject(font_);
        if (titleFont_)
            DeleteObject(titleFont_);
        if (brandIcon_)
            DestroyIcon(brandIcon_);
        if (target_)
            target_->Release();
        if (writeFactory_)
            writeFactory_->Release();
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
        } else if (selectedPage_ == kNavAppearance) {
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
        }
        preview();
        return 0;
    }
    LRESULT onResetAppearance(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_)
            cover(kCoveredAppearanceReset);
        if (!confirmDialog("dialog.reset_appearance.title", "dialog.reset_appearance.body"))
            return 0;
        const bool ok = resetPresentation();
        presentationDirty_ = !ok;
        setSaveStatus(ok ? get("status.saved") : get("error.command"));
        if (ok) {
            appearanceAdvanced_ = false;
            SendMessageW(control(kAppearanceAdvanced), BM_SETCHECK, BST_UNCHECKED, 0);
            layoutControls();
            (void)ensureProductionPreview();
        }
        return 0;
    }
    LRESULT onAppearanceAdvanced(WORD, WORD, HWND, BOOL&) {
        if (interactionTest_)
            cover(kCoveredAppearanceAdvanced);
        appearanceAdvanced_ =
            SendMessageW(control(kAppearanceAdvanced), BM_GETCHECK, 0, 0) == BST_CHECKED;
        showPage(kNavAppearance);
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
        updatePackageDetail();
        return 0;
    }
    LRESULT onThemeSelection(WORD, WORD, HWND, BOOL&) {
        const LRESULT selected = SendMessageW(control(kTheme), CB_GETCURSEL, 0, 0);
        if (selected != CB_ERR && static_cast<std::size_t>(selected) < themes_.size()) {
            SendMessageW(control(kThemeLibrary), LB_SETCURSEL, selected, 0);
            updateThemeDetail();
        }
        presentationDirty_ = true;
        liveApplyPresentation();
        return 0;
    }
    LRESULT onThemeLibrarySelection(WORD, WORD, HWND, BOOL&) {
        const LRESULT selected = SendMessageW(control(kThemeLibrary), LB_GETCURSEL, 0, 0);
        if (selected != LB_ERR && static_cast<std::size_t>(selected) < themes_.size())
            SendMessageW(control(kTheme), CB_SETCURSEL, selected, 0);
        updateThemeDetail();
        presentationDirty_ = true;
        liveApplyPresentation();
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
    void setStatus(std::wstring_view text) {
        const std::wstring owned(text);
        setStatus(owned.c_str());
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
        if (id == kStartup || id == kInputMethod) {
            generalDirty_ = true;
            setSaveStatus(get("status.unsaved"));
        } else {
            if (id == kFont)
                hideModernFontEditor();
            presentationDirty_ = true;
            liveApplyPresentation();
        }
        return 0;
    }

    Strings strings_;
    HFONT font_{};
    HFONT titleFont_{};
    HICON brandIcon_{};
    ID2D1Factory* factory_{};
    ID2D1HwndRenderTarget* target_{};
    IDWriteFactory* writeFactory_{};
    UINT_PTR statusTimer_{};
    std::vector<ModernHitTarget> modernHits_;
    std::vector<PackageRow> packages_;
    std::vector<InputMethodRow> inputMethods_;
    std::vector<ThemeRow> themes_;
    int selectedPage_{kNavGeneral};
    bool generalDirty_{};
    bool presentationDirty_{};
    bool appearanceAdvanced_{};
    bool repositoryAvailable_{};
    std::wstring repositoryError_;
    bool fontEditActive_{};
    bool uiContractTest_{};
    bool interactionTest_{};
    bool livePreviewContractTest_{};
    bool legacyVisualContractTest_{};
    bool previewActiveForContract_{};
    bool forceLiveApplyFailure_{};
    unsigned long long actionCoverage_{};
    unsigned previewLaunchCount_{};
    unsigned liveApplyCount_{};
    unsigned resetApplyCount_{};
    UINT dpi_{96};
};

} // namespace

CAppModule _Module;

int WINAPI wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE, _In_ PWSTR commandLine,
                    _In_ int showCommand) {
    enableDpiAwareness();
    setCurrentProcessAppUserModelId(fcitx::windows::kReleaseIdentity.settings_app_user_model_id);
    const auto parsedCommandLine = parseCommandLine(commandLine ? commandLine : L"");
    if (!parsedCommandLine.valid)
        return 1;
    const std::wstring_view command(parsedCommandLine.command);
    if (command == L"--check-i18n")
        return checkI18n() ? 0 : 2;
    if (command == L"--check-resources")
        return checkResources() ? 0 : 2;
    if (command == L"--self-test")
        return checkI18n() && checkResources() ? 0 : 2;
    const bool uiContractTest = command == L"--ui-contract-test";
    const bool uiInteractionTest = command == L"--ui-interaction-test";
    const bool uiVisualContractTest = command == L"--ui-visual-contract-test";
    const bool uiLivePreviewContractTest = command == L"--ui-live-preview-contract-test";
    const bool showDiagnostics = command == L"--diagnostics";
    if (!command.empty() && !uiContractTest && !uiInteractionTest &&
        !uiVisualContractTest && !uiLivePreviewContractTest && !showDiagnostics)
        return 1;
    const wchar_t* locale = localeFileForOverride(parsedCommandLine.localeOverride);
    if (!locale)
        return 1;
    Strings strings;
    if (!loadLocale(executableDirectory() / L"locales" / locale, strings))
        return 2;
    if (FAILED(_Module.Init(nullptr, instance)))
        return 3;
    CMessageLoop loop;
    _Module.AddMessageLoop(&loop);
    ConfigWindow window(std::move(strings));
    std::wstring title = window.title();
    if (!window.Create(nullptr, CWindow::rcDefault, title.c_str(),
                       WS_OVERLAPPEDWINDOW & ~WS_MAXIMIZEBOX)) {
        _Module.RemoveMessageLoop();
        _Module.Term();
        return 4;
    }
    enableNativeWindowEffects(window);
    window.resizeToDefaultClient();
    window.CenterWindow();
    if (uiContractTest || uiInteractionTest || uiVisualContractTest ||
        uiLivePreviewContractTest) {
        const bool passed = uiContractTest         ? window.verifyUiContract()
                            : uiVisualContractTest ? window.verifyVisualContract()
                            : uiLivePreviewContractTest
                                ? window.verifyLivePreviewContract()
                                : window.verifyInteractionCoverage();
        window.DestroyWindow();
        _Module.RemoveMessageLoop();
        _Module.Term();
        return passed ? 0 : 5;
    }
    if (showDiagnostics)
        window.selectPage(kNavRepair);
    window.ShowWindow(showCommand);
    const int result = loop.Run();
    _Module.RemoveMessageLoop();
    _Module.Term();
    return result;
}
