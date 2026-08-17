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

namespace {

namespace fs = std::filesystem;
using Strings = std::unordered_map<std::string, std::wstring>;

std::wstring widen(std::string_view value) {
    if (value.empty()) return {};
    const int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
                                          static_cast<int>(value.size()), nullptr, 0);
    if (count <= 0) return {};
    std::wstring result(static_cast<std::size_t>(count), L'\0');
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
                               static_cast<int>(value.size()), result.data(), count) == count
               ? result
               : std::wstring{};
}

std::string narrow(std::wstring_view value) {
    if (value.empty()) return {};
    const int count = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                                          static_cast<int>(value.size()), nullptr, 0, nullptr,
                                          nullptr);
    if (count <= 0) return {};
    std::string result(static_cast<std::size_t>(count), '\0');
    return WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                               static_cast<int>(value.size()), result.data(), count, nullptr,
                               nullptr) == count
               ? result
               : std::string{};
}

bool parseString(std::string_view text, std::size_t& offset, std::string& value) {
    if (offset >= text.size() || text[offset++] != '"') return false;
    value.clear();
    while (offset < text.size()) {
        const unsigned char character = text[offset++];
        if (character == '"') return true;
        if (character < 0x20U) return false;
        if (character != '\\') {
            value.push_back(static_cast<char>(character));
            continue;
        }
        if (offset >= text.size()) return false;
        switch (text[offset++]) {
        case '"': value.push_back('"'); break;
        case '\\': value.push_back('\\'); break;
        case '/': value.push_back('/'); break;
        case 'b': value.push_back('\b'); break;
        case 'f': value.push_back('\f'); break;
        case 'n': value.push_back('\n'); break;
        case 'r': value.push_back('\r'); break;
        case 't': value.push_back('\t'); break;
        default: return false;
        }
    }
    return false;
}

bool parseFlatJson(std::string_view text, Strings& strings) {
    if (text.size() > 2U * 1024U * 1024U) return false;
    strings.clear();
    std::size_t offset = 0;
    const auto whitespace = [&] {
        while (offset < text.size() && (text[offset] == ' ' || text[offset] == '\t' ||
                                        text[offset] == '\r' || text[offset] == '\n')) ++offset;
    };
    whitespace();
    if (offset >= text.size() || text[offset++] != '{') return false;
    bool sawVersion = false;
    for (;;) {
        whitespace();
        if (offset < text.size() && text[offset] == '}') {
            ++offset;
            whitespace();
            return offset == text.size() && sawVersion;
        }
        std::string key;
        if (!parseString(text, offset, key)) return false;
        whitespace();
        if (offset >= text.size() || text[offset++] != ':') return false;
        whitespace();
        if (key == "format_version") {
            if (sawVersion || offset >= text.size() || text[offset++] != '1') return false;
            sawVersion = true;
        } else {
            std::string value;
            if (!parseString(text, offset, value) || strings.contains(key)) return false;
            std::wstring wide = widen(value);
            if (!value.empty() && wide.empty()) return false;
            strings.emplace(std::move(key), std::move(wide));
        }
        whitespace();
        if (offset >= text.size()) return false;
        if (text[offset] == '}') continue;
        if (text[offset++] != ',') return false;
    }
}

bool loadLocale(const fs::path& path, Strings& strings) {
    std::ifstream stream(path, std::ios::binary);
    if (!stream) return false;
    const std::string text(std::istreambuf_iterator<char>(stream), {});
    return parseFlatJson(text, strings);
}

fs::path executableDirectory() {
    std::wstring path(32768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    if (size == 0 || size >= path.size()) return {};
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
            if (character == L'"') result.append(backslashes + 1, L'\\');
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
    if (!fs::exists(executable)) return false;
    std::wstring command = quote(executable.wstring());
    for (const auto& argument : arguments) command += L" " + quote(argument);
    SECURITY_ATTRIBUTES attributes{sizeof(attributes), nullptr, TRUE};
    HANDLE readPipe = nullptr;
    HANDLE writePipe = nullptr;
    if (!CreatePipe(&readPipe, &writePipe, &attributes, 0)) return false;
    SetHandleInformation(readPipe, HANDLE_FLAG_INHERIT, 0);
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    startup.dwFlags = STARTF_USESTDHANDLES;
    startup.hStdOutput = writePipe;
    startup.hStdError = writePipe;
    PROCESS_INFORMATION process{};
    const BOOL created = CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr,
                                        TRUE, CREATE_NO_WINDOW, nullptr,
                                        executable.parent_path().c_str(), &startup, &process);
    CloseHandle(writePipe);
    if (!created) {
        CloseHandle(readPipe);
        return false;
    }
    const DWORD wait = WaitForSingleObject(process.hProcess, 3000);
    if (wait == WAIT_TIMEOUT) TerminateProcess(process.hProcess, ERROR_TIMEOUT);
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
        !loadLocale(localeDirectory / L"zh-CN.json", chinese) ||
        english.size() != chinese.size()) return false;
    for (const auto& [key, value] : english) {
        (void)value;
        if (!chinese.contains(key)) return false;
    }
    return true;
}

bool checkResources() {
    std::wstring output;
    return runExecutable(executableDirectory() / L"fcitx5-ui.exe",
                         {L"--self-test", L"--safe-mode"}, output);
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

class ConfigWindow final : public CWindowImpl<ConfigWindow> {
public:
    DECLARE_WND_CLASS_EX(L"Fcitx5ConfigWindow", CS_HREDRAW | CS_VREDRAW, COLOR_WINDOW)

    explicit ConfigWindow(Strings strings) : strings_(std::move(strings)) {}
    const wchar_t* title() const { return get("app.title"); }

    BEGIN_MSG_MAP(ConfigWindow)
        MESSAGE_HANDLER(WM_CREATE, onCreate)
        MESSAGE_HANDLER(WM_PAINT, onPaint)
        MESSAGE_HANDLER(WM_DESTROY, onDestroy)
        COMMAND_ID_HANDLER(kApply, onApply)
        COMMAND_ID_HANDLER(kRestart, onRestart)
        COMMAND_ID_HANDLER(kDiagnostics, onDiagnostics)
        COMMAND_ID_HANDLER(kRepair, onRepair)
        COMMAND_ID_HANDLER(kPreview, onPreview)
    END_MSG_MAP()

private:
    const wchar_t* get(const char* key) const {
        const auto iterator = strings_.find(key);
        return iterator == strings_.end() ? L"" : iterator->second.c_str();
    }
    HWND control(int id) const { return GetDlgItem(id); }
    void add(const wchar_t* type, const wchar_t* label, DWORD style, int x, int y, int width,
             int height, int id = 0) {
        HWND child = CreateWindowExW(0, type, label, WS_CHILD | WS_VISIBLE | style, x, y, width,
                                     height, m_hWnd,
                                     reinterpret_cast<HMENU>(static_cast<std::intptr_t>(id)),
                                     _Module.GetModuleInstance(), nullptr);
        SendMessageW(child, WM_SETFONT, reinterpret_cast<WPARAM>(font_), TRUE);
    }
    LRESULT onCreate(UINT, WPARAM, LPARAM, BOOL&) {
        NONCLIENTMETRICSW metrics{sizeof(metrics)};
        SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, sizeof(metrics), &metrics, 0);
        font_ = CreateFontIndirectW(&metrics.lfMessageFont);
        add(L"STATIC", get("nav.general"), 0, 24, 88, 150, 28);
        add(L"STATIC", get("nav.appearance"), 0, 24, 124, 150, 28);
        add(L"STATIC", get("nav.theme"), 0, 24, 160, 150, 28);
        add(L"STATIC", get("nav.diagnostics"), 0, 24, 196, 150, 28);
        add(L"STATIC", get("nav.repair"), 0, 24, 232, 150, 28);
        add(L"BUTTON", get("general.startup"), BS_AUTOCHECKBOX | WS_TABSTOP, 230, 84, 300, 28,
            kStartup);
        add(L"STATIC", get("general.input_method"), 0, 230, 126, 140, 24);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST, 390, 122, 280, 120, kInputMethod);
        SendMessageW(control(kInputMethod), CB_ADDSTRING, 0,
                     reinterpret_cast<LPARAM>(get("input.pinyin")));
        SendMessageW(control(kInputMethod), CB_SETCURSEL, 0, 0);
        ::EnableWindow(control(kInputMethod), FALSE);
        add(L"STATIC", get("appearance.mode"), 0, 230, 168, 140, 24);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 390, 164, 280, 150, kAppearance);
        for (const wchar_t* item : {get("mode.system"), get("mode.light"), get("mode.dark")})
            SendMessageW(control(kAppearance), CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(item));
        SendMessageW(control(kAppearance), CB_SETCURSEL, 0, 0);
        add(L"STATIC", get("theme.label"), 0, 230, 210, 140, 24);
        add(WC_COMBOBOXW, L"", CBS_DROPDOWNLIST | WS_TABSTOP, 390, 206, 280, 150, kTheme);
        SendMessageW(control(kTheme), CB_ADDSTRING, 0,
                     reinterpret_cast<LPARAM>(get("theme.default")));
        SendMessageW(control(kTheme), CB_SETCURSEL, 0, 0);
        add(L"STATIC", get("font.label"), 0, 230, 252, 140, 24);
        add(L"EDIT", L"Microsoft YaHei", WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL, 390, 248,
            280, 30, kFont);
        add(L"STATIC", get("candidate.layout"), 0, 230, 294, 140, 24);
        add(L"BUTTON", get("candidate.vertical"), BS_AUTORADIOBUTTON | WS_GROUP | WS_TABSTOP,
            390, 290, 120, 28, kVertical);
        add(L"BUTTON", get("candidate.horizontal"), BS_AUTORADIOBUTTON | WS_TABSTOP, 520, 290,
            130, 28, kHorizontal);
        SendMessageW(control(kVertical), BM_SETCHECK, BST_CHECKED, 0);
        add(L"BUTTON", get("action.apply"), BS_DEFPUSHBUTTON | WS_TABSTOP, 550, 330, 120, 34,
            kApply);
        add(L"BUTTON", get("action.preview"), WS_TABSTOP, 230, 352, 140, 34, kPreview);
        add(L"BUTTON", get("action.restart"), WS_TABSTOP, 230, 400, 140, 34, kRestart);
        add(L"BUTTON", get("action.diagnostics"), WS_TABSTOP, 382, 400, 140, 34, kDiagnostics);
        add(L"BUTTON", get("action.repair"), WS_TABSTOP, 534, 400, 136, 34, kRepair);
        add(L"EDIT", L"", WS_BORDER | ES_MULTILINE | ES_READONLY | WS_VSCROLL, 230, 448, 440,
            86, kStatus);
        loadState();
        return 0;
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
        }
        if (runControl({L"--get-startup"}, output))
            SendMessageW(control(kStartup), BM_SETCHECK,
                         output.find(L"\"enabled\":true") != std::wstring::npos ? BST_CHECKED :
                                                                                   BST_UNCHECKED,
                         0);
        refresh();
    }
    void refresh() {
        std::wstring output;
        ::SetWindowTextW(control(kStatus),
                         runControl({L"--status"}, output) ? output.c_str()
                                                           : get("error.command"));
    }
    void restart() {
        std::wstring output;
        ::SetWindowTextW(control(kStatus), runControl({L"--restart-engine"}, output)
                                               ? get("restart.done")
                                               : get("error.command"));
    }
    void apply() {
        const auto modeIndex = SendMessageW(control(kAppearance), CB_GETCURSEL, 0, 0);
        const wchar_t* const mode = modeIndex == 1 ? L"light" :
                                    (modeIndex == 2 ? L"dark" : L"system");
        const wchar_t* const orientation =
            SendMessageW(control(kHorizontal), BM_GETCHECK, 0, 0) == BST_CHECKED
                ? L"horizontal"
                : L"vertical";
        wchar_t fontBuffer[129]{};
        ::GetWindowTextW(control(kFont), fontBuffer, static_cast<int>(std::size(fontBuffer)));
        const std::wstring font(fontBuffer);
        std::wstring output;
        std::wstring startupOutput;
        const bool ok = !font.empty() &&
                        runControl({L"--set-startup",
                                    SendMessageW(control(kStartup), BM_GETCHECK, 0, 0) == BST_CHECKED
                                        ? L"enabled"
                                        : L"disabled"},
                                   startupOutput) &&
                        runControl({L"--set-presentation", mode, L"builtin:default",
                                    orientation, font}, output);
        ::SetWindowTextW(control(kStatus), ok ? get("status.saved") : get("error.command"));
    }
    void repair() {
        const fs::path directory = executableDirectory();
        const fs::path root = directory.filename() == L"bin" ? directory.parent_path() : directory;
        fs::path dll = root / L"tsf/x64/fcitx5-tsf.dll";
        if (!fs::exists(dll)) dll = directory / L"fcitx5-tsf.dll";
        const fs::path registration = directory / L"fcitx5-register.exe";
        const std::wstring arguments = L"--repair --dll " + quote(dll.wstring());
        const auto result = reinterpret_cast<std::intptr_t>(ShellExecuteW(
            m_hWnd, L"runas", registration.c_str(), arguments.c_str(),
            directory.c_str(), SW_HIDE));
        ::SetWindowTextW(control(kStatus),
                         result > 32 ? get("repair.started") : get("error.command"));
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
        if (!CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, FALSE, 0,
                            nullptr, executable.parent_path().c_str(), &startup, &process)) {
            ::SetWindowTextW(control(kStatus), get("error.command"));
            return;
        }
        CloseHandle(process.hThread);
        previewProcess_ = process.hProcess;
    }

    LRESULT onPaint(UINT, WPARAM, LPARAM, BOOL&) {
        PAINTSTRUCT paint{};
        BeginPaint(&paint);
        if (!factory_) D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, &factory_);
        if (factory_ && !target_) {
            RECT rectangle{};
            GetClientRect(&rectangle);
            factory_->CreateHwndRenderTarget(
                D2D1::RenderTargetProperties(),
                D2D1::HwndRenderTargetProperties(
                    m_hWnd, D2D1::SizeU(rectangle.right, rectangle.bottom)),
                &target_);
        }
        if (target_) {
            target_->BeginDraw();
            target_->Clear(D2D1::ColorF(0xF5F7FA));
            ID2D1SolidColorBrush* brush = nullptr;
            target_->CreateSolidColorBrush(D2D1::ColorF(0xFFFFFF), &brush);
            if (brush) {
                target_->FillRoundedRectangle(
                    D2D1::RoundedRect(D2D1::RectF(204, 68, 706, 374), 12, 12), brush);
                target_->FillRoundedRectangle(
                    D2D1::RoundedRect(D2D1::RectF(204, 386, 706, 550), 12, 12), brush);
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
    LRESULT onDestroy(UINT, WPARAM, LPARAM, BOOL&) {
        if (previewProcess_) {
            if (WaitForSingleObject(previewProcess_, 0) == WAIT_TIMEOUT)
                TerminateProcess(previewProcess_, 0);
            CloseHandle(previewProcess_);
        }
        if (font_) DeleteObject(font_);
        if (target_) target_->Release();
        if (factory_) factory_->Release();
        PostQuitMessage(0);
        return 0;
    }
    LRESULT onApply(WORD, WORD, HWND, BOOL&) { apply(); return 0; }
    LRESULT onRestart(WORD, WORD, HWND, BOOL&) { restart(); return 0; }
    LRESULT onDiagnostics(WORD, WORD, HWND, BOOL&) { refresh(); return 0; }
    LRESULT onRepair(WORD, WORD, HWND, BOOL&) { repair(); return 0; }
    LRESULT onPreview(WORD, WORD, HWND, BOOL&) { preview(); return 0; }

    Strings strings_;
    HFONT font_{};
    ID2D1Factory* factory_{};
    ID2D1HwndRenderTarget* target_{};
    HANDLE previewProcess_{};
};

} // namespace

CAppModule _Module;

int WINAPI wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE,
                    _In_ PWSTR commandLine, _In_ int showCommand) {
    const std::wstring_view command(commandLine);
    if (command == L"--check-i18n") return checkI18n() ? 0 : 2;
    if (command == L"--check-resources") return checkResources() ? 0 : 2;
    if (command == L"--self-test") return checkI18n() && checkResources() ? 0 : 2;
    const LANGID language = GetUserDefaultUILanguage();
    const wchar_t* locale = PRIMARYLANGID(language) == LANG_CHINESE ? L"zh-CN.json"
                                                                    : L"en-US.json";
    Strings strings;
    if (!loadLocale(executableDirectory() / L"locales" / locale, strings)) return 2;
    if (FAILED(_Module.Init(nullptr, instance))) return 3;
    CMessageLoop loop;
    _Module.AddMessageLoop(&loop);
    ConfigWindow window(std::move(strings));
    if (!window.Create(nullptr, CWindow::rcDefault, window.title(),
                       WS_OVERLAPPEDWINDOW & ~WS_MAXIMIZEBOX)) {
        _Module.RemoveMessageLoop();
        _Module.Term();
        return 4;
    }
    window.ResizeClient(780, 620);
    window.CenterWindow();
    window.ShowWindow(showCommand);
    const int result = loop.Run();
    _Module.RemoveMessageLoop();
    _Module.Term();
    return result;
}
