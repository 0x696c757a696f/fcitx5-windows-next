#include "candidate_layout.h"
#include "candidate_interaction.h"
#include "candidate_model.h"
#include "config_model.h"
#include "peer_verification.h"
#include "pipe_client.h"
#include "pipe_security.h"
#include "protocol.h"
#include "runtime_identity.h"

#include <fcitx5_windows/release_identity.h>

#include <ShlObj.h>
#include <Shellapi.h>
#include <Windows.h>
#include <d2d1.h>
#include <dwrite.h>
#include <wrl/client.h>

#include <array>
#include <cerrno>
#include <charconv>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <memory>
#include <optional>
#include <span>
#include <sstream>
#include <string>
#include <string_view>
#include <thread>
#include <vector>

namespace {

using Microsoft::WRL::ComPtr;
constexpr UINT kSnapshotMessage = WM_APP + 1;
constexpr UINT_PTR kFocusWatchTimer = 1;
constexpr UINT_PTR kClickGuardTimer = 2;
constexpr wchar_t kVisualConfigChangedMessage[] =
    L"Fcitx5WindowsNext.VisualConfigChanged.v1";
constexpr wchar_t kCandidateDismissMessage[] =
    L"Fcitx5WindowsNext.CandidateDismiss.v1";

UINT visualConfigChangedMessage() noexcept {
    static const UINT message = RegisterWindowMessageW(kVisualConfigChangedMessage);
    return message;
}

UINT candidateDismissMessage() noexcept {
    static const UINT message = RegisterWindowMessageW(kCandidateDismissMessage);
    return message;
}

struct CandidateVisual {
    std::wstring label;
    std::wstring text;
    std::wstring comment;
};

void enableDpiAwareness() {
    using SetContext = BOOL(WINAPI*)(HANDLE);
    const HMODULE user32 = GetModuleHandleW(L"user32.dll");
    const auto setContext =
        user32
            ? reinterpret_cast<SetContext>(GetProcAddress(user32, "SetProcessDpiAwarenessContext"))
            : nullptr;
    if (setContext && setContext(reinterpret_cast<HANDLE>(-4)))
        return;
    (void)SetProcessDPIAware();
}

void enableNativeWindowEffects(HWND window) noexcept {
    const HMODULE dwm = LoadLibraryW(L"dwmapi.dll");
    if (!dwm)
        return;
    using SetWindowAttribute = HRESULT(WINAPI*)(HWND, DWORD, const void*, DWORD);
    const auto setAttribute =
        reinterpret_cast<SetWindowAttribute>(GetProcAddress(dwm, "DwmSetWindowAttribute"));
    if (setAttribute) {
        constexpr DWORD kWindowCornerPreference = 33;
        constexpr DWORD kRound = 2;
        (void)setAttribute(window, kWindowCornerPreference, &kRound, sizeof(kRound));
    }
    FreeLibrary(dwm);
}

bool utf8ToWide(std::string_view input, std::wstring& output) {
    if (input.empty()) {
        output.clear();
        return true;
    }
    const int size = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                                         static_cast<int>(input.size()), nullptr, 0);
    if (size <= 0)
        return false;
    output.resize(static_cast<std::size_t>(size));
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                               static_cast<int>(input.size()), output.data(), size) == size;
}

std::optional<std::string> readBoundedFile(const std::filesystem::path& path, std::size_t maximum) {
    std::error_code error;
    const auto size = std::filesystem::file_size(path, error);
    if (error || size > maximum)
        return std::nullopt;
    std::ifstream stream(path, std::ios::binary);
    if (!stream)
        return std::nullopt;
    std::string contents(static_cast<std::size_t>(size), '\0');
    if (size != 0 && !stream.read(contents.data(), static_cast<std::streamsize>(size)))
        return std::nullopt;
    return contents;
}

bool systemUsesDarkAppearance() noexcept {
    DWORD light = 1;
    DWORD size = sizeof(light);
    if (RegGetValueW(
            HKEY_CURRENT_USER, L"Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
            L"AppsUseLightTheme", RRF_RT_REG_DWORD, nullptr, &light, &size) != ERROR_SUCCESS)
        return false;
    return light == 0;
}

std::filesystem::path executableDirectory() {
    std::wstring path(32'768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    if (size == 0 || size >= path.size())
        return {};
    path.resize(size);
    return std::filesystem::path(path).parent_path();
}

bool parseUnsigned(std::wstring_view text, std::uint64_t& value) noexcept {
    if (text.empty()) return false;
    wchar_t* end = nullptr;
    errno = 0;
    const auto parsed = _wcstoui64(text.data(), &end, 10);
    if (errno == ERANGE || end != text.data() + text.size()) return false;
    value = parsed;
    return true;
}

int runCandidateSelectionTest(int argumentCount, wchar_t** arguments) {
    if (argumentCount != 9 ||
        std::wstring_view(arguments[1]) != L"--candidate-select-test") return 64;
    std::uint64_t targetProcessId = 0;
    std::uint64_t engineEpoch = 0;
    std::uint64_t contextId = 0;
    std::uint64_t compositionId = 0;
    std::uint64_t revision = 0;
    std::uint64_t candidateId = 0;
    if (!parseUnsigned(arguments[3], targetProcessId) || targetProcessId > UINT32_MAX ||
        !parseUnsigned(arguments[4], engineEpoch) ||
        !parseUnsigned(arguments[5], contextId) ||
        !parseUnsigned(arguments[6], compositionId) ||
        !parseUnsigned(arguments[7], revision) ||
        !parseUnsigned(arguments[8], candidateId)) return 65;
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity)) return 66;
    fcitx::windows::ipc::PipeClient client(
        fcitx::windows::platform::makeLocalEndpointName(identity, L"engine"),
        fcitx::windows::ipc::PeerPolicy::exact(arguments[2]));
    return client.selectCandidate(static_cast<std::uint32_t>(targetProcessId), engineEpoch,
                                  contextId, compositionId, revision, candidateId)
               ? 0
               : 67;
}

std::filesystem::path localDataDirectory() {
    const auto executable = executableDirectory();
    if (!executable.empty() && std::filesystem::exists(executable / L"portable.flag")) {
        return executable / L"data";
    }
    if (!executable.empty() &&
        std::filesystem::exists(executable.parent_path() / L"portable.flag")) {
        return executable.parent_path() / L"data";
    }
    PWSTR path = nullptr;
    if (FAILED(SHGetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_DEFAULT, nullptr, &path)))
        return {};
    std::filesystem::path result(path);
    CoTaskMemFree(path);
    return result / fcitx::windows::kReleaseIdentity.data_directory;
}

fcitx::windows::config::Config loadVisualConfig(bool safeMode) {
    using namespace fcitx::windows::config;
    Config defaults;
    ParseError error;
    if (!parseConfig(defaultConfigToml(), defaults, error))
        return {};
    Config user;
    const auto data = localDataDirectory();
    if (!safeMode && !data.empty()) {
        if (const auto text = readBoundedFile(data / L"config.toml", 256U * 1024U)) {
            Config parsed;
            if (parseConfig(*text, parsed, error))
                user = std::move(parsed);
        }
    }
    const AppearanceMode mode =
        user.appearanceMode.value_or(defaults.appearanceMode.value_or(AppearanceMode::system));
    const bool dark = mode == AppearanceMode::dark ||
                      (mode == AppearanceMode::system && systemUsesDarkAppearance());
    const std::string themeId =
        safeMode ? "builtin:default" : user.theme.value_or("builtin:default");
    std::filesystem::path themePath;
    if (themeId == "builtin:default") {
        themePath = executableDirectory() / L"resources" / L"themes" / L"default" / L"theme.toml";
    } else if (!data.empty()) {
        std::wstring wideId;
        if (utf8ToWide(themeId, wideId))
            themePath = data / L"themes" / wideId / L"theme.toml";
    }
    if (!themePath.empty()) {
        if (const auto text = readBoundedFile(themePath, 512U * 1024U)) {
            Theme theme;
            if (parseTheme(*text, theme, error) &&
                (themeId == "builtin:default" || theme.id == themeId)) {
                return mergeConfig(defaults, resolveTheme(theme, dark, user));
            }
        }
    }
    return mergeConfig(defaults, user);
}

D2D1_COLOR_F parseColor(const fcitx::windows::config::Config& config, std::string_view name,
                        D2D1_COLOR_F fallback) {
    const auto found = config.colors.find(std::string(name));
    if (found == config.colors.end())
        return fallback;
    const auto& text = found->second;
    const auto component = [&](std::size_t offset) {
        return static_cast<float>(std::stoul(text.substr(offset, 2), nullptr, 16)) / 255.0F;
    };
    try {
        return D2D1::ColorF(component(1), component(3), component(5),
                            text.size() == 9 ? component(7) : 1.0F);
    } catch (...) {
        return fallback;
    }
}

class CandidateWindow final {
  public:
    bool create(HINSTANCE instance, bool visible, bool safeMode, bool interactionTest = false) {
        safeMode_ = safeMode;
        interactionTest_ = interactionTest;
        if (!interactionTest_) {
            fcitx::windows::platform::RuntimeIdentity identity;
            std::wstring executable(32'768, L'\0');
            const DWORD executableSize = GetModuleFileNameW(
                nullptr, executable.data(), static_cast<DWORD>(executable.size()));
            if (!fcitx::windows::platform::queryCurrentIdentity(identity) ||
                executableSize == 0 || executableSize >= executable.size()) return false;
            executable.resize(executableSize);
            const auto engine =
                (std::filesystem::path(executable).parent_path() / L"fcitx5-engine.exe").wstring();
            candidateClient_ = std::make_unique<fcitx::windows::ipc::PipeClient>(
                fcitx::windows::platform::makeLocalEndpointName(identity, L"engine"),
                fcitx::windows::ipc::PeerPolicy::exact(engine));
        }
        visualConfig_ = loadVisualConfig(safeMode_);
        WNDCLASSW windowClass{};
        windowClass.hInstance = instance;
        const std::wstring windowClassName =
            std::wstring(fcitx::windows::kReleaseIdentity.local_object_prefix) + L".Candidate";
        windowClass.lpszClassName = windowClassName.c_str();
        windowClass.lpfnWndProc = windowProcedure;
        windowClass.hCursor = LoadCursorW(nullptr, IDC_ARROW);
        windowClass.style = CS_DROPSHADOW;
        RegisterClassW(&windowClass);
        window_ =
            CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_LAYERED,
                            windowClass.lpszClassName, L"", WS_POPUP, 100, 100, 360, 120, nullptr,
                            nullptr, instance, this);
        if (!window_)
            return false;
        if (SetTimer(window_, kFocusWatchTimer, 100, nullptr) == 0)
            return false;
        enableNativeWindowEffects(window_);
        const LONG_PTR styles = GetWindowLongPtrW(window_, GWL_EXSTYLE);
        if ((styles & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE)) !=
                (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) ||
            (styles & WS_EX_APPWINDOW) != 0)
            return false;
        const auto opacity = visualConfig_.opacity.value_or(1.0);
        SetLayeredWindowAttributes(
            window_, 0, static_cast<BYTE>(std::clamp(opacity, 0.2, 1.0) * 255.0), LWA_ALPHA);
        if (visible)
            ShowWindow(window_, SW_SHOWNOACTIVATE);
        return createDeviceResources();
    }

    int run() {
        MSG message{};
        while (GetMessageW(&message, nullptr, 0, 0) > 0) {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        return static_cast<int>(message.wParam);
    }

    [[nodiscard]] HWND handle() const noexcept { return window_; }

    void simulateDeviceLossForTest() noexcept { renderTarget_.Reset(); }

    void showSyntheticPreview(bool scrollDemo) {
        fcitx::windows::protocol::KeyResponse response;
        response.metadata.engineEpoch = 1;
        response.metadata.contextId = 1;
        response.metadata.compositionId = 1;
        response.metadata.revision = 1;
        if (scrollDemo) {
            static constexpr std::array<std::string_view, 42> words{
                "我", "哦", "窝", "沃", "握", "卧", "涡", "蜗", "渥", "幄", "斡", "龌", "喔", "莴",
                "倭", "硪", "挝", "肟", "偓", "涴", "踒", "猧", "婐", "捰", "瓁", "馧", "焥", "腛",
                "濣", "瞃", "擭", "雘", "臒", "檴", "嚄", "濩", "获", "惑", "豁", "霍", "藿", "镬"};
            response.candidates.reserve(60U);
            for (std::size_t index = 0; index < 60U; ++index) {
                const std::string text = index < words.size() ? std::string(words[index])
                                                              : "候选" + std::to_string(index + 1U);
                response.candidates.push_back(
                    {index + 1U, std::to_string(index % 6U + 1U), text, {}});
            }
            response.selectedCandidate = 18;
            response.candidatePage = 3;
            response.candidatePageSize = 6;
            response.candidateBulk = true;
            visualConfig_.scrollMode = true;
        } else {
            response.candidates = {{1, "1", "输入法", "shūrùfǎ"},
                                   {2, "2", "输入", "shūrù"},
                                   {3, "3", "中文", "zhōngwén"}};
            response.selectedCandidate = 0;
            response.candidatePageSize = 3;
            response.candidateBulk = false;
        }
        response.candidateEnd = true;
        response.candidateTotal = static_cast<std::uint32_t>(response.candidates.size());
        response.candidateVisibility = 1;
        response.caret = {true, 100, 100, 102, 124, 96};
        update(response);
    }

    [[nodiscard]] bool runInteractionSelfTest() {
        if (itemRects_.size() < 2U || visibleIndices_.size() < 2U)
            return false;
        const auto& rectangle = itemRects_[1];
        const LONG x = static_cast<LONG>((rectangle.left + rectangle.right) / 2.0F);
        const LONG y = static_cast<LONG>((rectangle.top + rectangle.bottom) / 2.0F);
        POINT screen{x, y};
        ClientToScreen(window_, &screen);
        const LPARAM screenPoint = MAKELPARAM(static_cast<WORD>(screen.x),
                                               static_cast<WORD>(screen.y));
        const LPARAM clientPoint = MAKELPARAM(static_cast<WORD>(x), static_cast<WORD>(y));
        if (SendMessageW(window_, WM_NCHITTEST, 0, screenPoint) != HTCLIENT ||
            SendMessageW(window_, WM_MOUSEACTIVATE, 0, 0) != MA_NOACTIVATE)
            return false;
        SendMessageW(window_, WM_LBUTTONDOWN, MK_LBUTTON, clientPoint);
        SendMessageW(window_, WM_LBUTTONUP, 0, clientPoint);
        if (!capturedTestIntent_ || !capturedTestIntent_->valid() ||
            capturedTestIntent_->candidateId != 2U)
            return false;
        const auto& current = model_.current();
        if (!current)
            return false;
        SendMessageW(window_, candidateDismissMessage(), targetForegroundProcessId_,
                     static_cast<LPARAM>(current->contextId + 1U));
        if (!IsWindowVisible(window_))
            return false;
        SendMessageW(window_, candidateDismissMessage(), targetForegroundProcessId_,
                     static_cast<LPARAM>(current->contextId));
        return !IsWindowVisible(window_);
    }

    // Refresh only the visual configuration and text formats, without
    // reflowing the current model. Used from update(): the caller continues to
    // rebuild the candidate list with the new config, so calling reflow here
    // would consume/reset the model and leave the outer update with an empty
    // current() snapshot.
    void refreshVisualConfig() {
        visualConfig_ = loadVisualConfig(safeMode_);
        textFormat_.Reset();
        labelFormat_.Reset();
        annotationFormat_.Reset();
        const auto opacity = visualConfig_.opacity.value_or(1.0);
        SetLayeredWindowAttributes(
            window_, 0, static_cast<BYTE>(std::clamp(opacity, 0.2, 1.0) * 255.0), LWA_ALPHA);
        (void)createDeviceResources();
    }

    void reloadVisualConfig() {
        refreshVisualConfig();
        reflowCurrentModel();
    }

    bool paintOnce() {
        if (!createDeviceResources())
            return false;
        renderTarget_->BeginDraw();
        HIGHCONTRASTW contrast{};
        contrast.cbSize = sizeof(contrast);
        const bool highContrast =
            SystemParametersInfoW(SPI_GETHIGHCONTRAST, sizeof(contrast), &contrast, 0) &&
            (contrast.dwFlags & HCF_HIGHCONTRASTON) != 0;
        const COLORREF systemBackground = GetSysColor(COLOR_WINDOW);
        const COLORREF systemForeground = GetSysColor(COLOR_WINDOWTEXT);
        const auto background = highContrast ? D2D1::ColorF(GetRValue(systemBackground) / 255.0F,
                                                            GetGValue(systemBackground) / 255.0F,
                                                            GetBValue(systemBackground) / 255.0F)
                                             : parseColor(visualConfig_, "background",
                                                          D2D1::ColorF(0.97F, 0.98F, 0.98F));
        const auto foreground = highContrast ? D2D1::ColorF(GetRValue(systemForeground) / 255.0F,
                                                            GetGValue(systemForeground) / 255.0F,
                                                            GetBValue(systemForeground) / 255.0F)
                                             : parseColor(visualConfig_, "candidate_text",
                                                          D2D1::ColorF(0.13F, 0.13F, 0.14F));
        const auto selectedBackground =
            highContrast ? D2D1::ColorF(GetRValue(GetSysColor(COLOR_HIGHLIGHT)) / 255.0F,
                                        GetGValue(GetSysColor(COLOR_HIGHLIGHT)) / 255.0F,
                                        GetBValue(GetSysColor(COLOR_HIGHLIGHT)) / 255.0F)
                         : parseColor(visualConfig_, "selected_background",
                                      D2D1::ColorF(0.82F, 0.89F, 0.99F));
        const auto selectedForeground =
            highContrast ? D2D1::ColorF(GetRValue(GetSysColor(COLOR_HIGHLIGHTTEXT)) / 255.0F,
                                        GetGValue(GetSysColor(COLOR_HIGHLIGHTTEXT)) / 255.0F,
                                        GetBValue(GetSysColor(COLOR_HIGHLIGHTTEXT)) / 255.0F)
                         : parseColor(visualConfig_, "selected_candidate_text",
                                      D2D1::ColorF(0.09F, 0.31F, 0.65F));
        renderTarget_->Clear(background);
        ComPtr<ID2D1SolidColorBrush> textBrush;
        ComPtr<ID2D1SolidColorBrush> labelBrush;
        ComPtr<ID2D1SolidColorBrush> commentBrush;
        ComPtr<ID2D1SolidColorBrush> selectionBrush;
        ComPtr<ID2D1SolidColorBrush> selectedTextBrush;
        ComPtr<ID2D1SolidColorBrush> selectedLabelBrush;
        ComPtr<ID2D1SolidColorBrush> selectedCommentBrush;
        ComPtr<ID2D1SolidColorBrush> borderBrush;
        const auto labelColor =
            highContrast ? foreground : parseColor(visualConfig_, "label_text", foreground);
        const auto commentColor =
            highContrast ? foreground : parseColor(visualConfig_, "comment_text", foreground);
        const auto selectedLabelColor =
            highContrast ? selectedForeground
                         : parseColor(visualConfig_, "selected_label_text", selectedForeground);
        const auto selectedCommentColor =
            highContrast ? selectedForeground
                         : parseColor(visualConfig_, "selected_comment_text", selectedForeground);
        const auto borderColor =
            highContrast ? foreground
                         : parseColor(visualConfig_, "border", D2D1::ColorF(0.82F, 0.82F, 0.82F));
        if (FAILED(renderTarget_->CreateSolidColorBrush(foreground, &textBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(labelColor, &labelBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(commentColor, &commentBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedBackground, &selectionBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedForeground, &selectedTextBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedLabelColor, &selectedLabelBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedCommentColor,
                                                        &selectedCommentBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(borderColor, &borderBrush)))
            return false;
        const std::vector<CandidateVisual> fallback{{L"1. ", L"你", L"nǐ"}, {L"2. ", L"呢", L""}};
        const auto& lines = candidates_.empty() ? fallback : candidates_;
        float fallbackTop = 8.0F;
        const std::size_t paintCount =
            visibleIndices_.empty() ? lines.size() : visibleIndices_.size();
        if (scrollMode_ && itemRects_.size() > 6U) {
            borderBrush->SetOpacity(0.55F);
            for (std::size_t row = 6U; row < itemRects_.size(); row += 6U) {
                const float y = (itemRects_[row - 1U].bottom + itemRects_[row].top) / 2.0F;
                renderTarget_->DrawLine(D2D1::Point2F(12.0F, y),
                                        D2D1::Point2F(renderTarget_->GetSize().width - 12.0F, y),
                                        borderBrush.Get(), 1.0F);
            }
            borderBrush->SetOpacity(1.0F);
        }
        for (std::size_t local = 0; local < paintCount; ++local) {
            const std::size_t index = visibleIndices_.empty() ? local : visibleIndices_[local];
            if (index >= lines.size())
                continue;
            const auto& candidate = lines[index];
            const D2D1_RECT_F bounds = itemRects_.size() == paintCount
                                           ? itemRects_[local]
                                           : D2D1::RectF(12, fallbackTop, 348, fallbackTop + 32);
            const bool selected = selected_ && *selected_ == index;
            if (selected) {
                const float radius =
                    static_cast<float>(visualConfig_.geometry.cornerRadius.value_or(8.0));
                const auto size = renderTarget_->GetSize();
                const D2D1_RECT_F selection =
                    D2D1::RectF((std::max)(0.0F, bounds.left - selectionInflateX_),
                                (std::max)(0.0F, bounds.top - selectionInflateY_),
                                (std::min)(size.width, bounds.right + selectionInflateX_),
                                (std::min)(size.height, bounds.bottom + selectionInflateY_));
                renderTarget_->FillRoundedRectangle(D2D1::RoundedRect(selection, radius, radius),
                                                    selectionBrush.Get());
            }
            float left = bounds.left;
            const auto drawSegment = [&](const std::wstring& value, IDWriteTextFormat* format,
                                         ID2D1Brush* brush) {
                if (value.empty())
                    return;
                ComPtr<IDWriteTextLayout> layout;
                DWRITE_TEXT_METRICS metrics{};
                if (FAILED(writeFactory_->CreateTextLayout(
                        value.data(), static_cast<UINT32>(value.size()), format,
                        (std::max)(1.0F, bounds.right - left),
                        (std::max)(1.0F, bounds.bottom - bounds.top), &layout)) ||
                    FAILED(layout->GetMetrics(&metrics)))
                    return;
                const D2D1_RECT_F segment =
                    D2D1::RectF(left, bounds.top, bounds.right, bounds.bottom);
                // Clip instead of wrapping: a long label/comment that exceeds
                // the remaining row width must not wrap onto the candidate
                // row below and visually overlap it.
                renderTarget_->DrawTextW(value.data(), static_cast<UINT32>(value.size()), format,
                                         segment, brush, D2D1_DRAW_TEXT_OPTIONS_CLIP);
                left += metrics.widthIncludingTrailingWhitespace;
            };
            drawSegment(candidate.label, labelFormat_.Get(),
                        selected ? selectedLabelBrush.Get() : labelBrush.Get());
            drawSegment(candidate.text, textFormat_.Get(),
                        selected ? selectedTextBrush.Get() : textBrush.Get());
            drawSegment(candidate.comment, annotationFormat_.Get(),
                        selected ? selectedCommentBrush.Get() : commentBrush.Get());
            fallbackTop += 32.0F;
        }
        if (hasScrollbar_) {
            renderTarget_->FillRoundedRectangle(D2D1::RoundedRect(scrollbarTrack_, 2.0F, 2.0F),
                                                borderBrush.Get());
            renderTarget_->FillRoundedRectangle(D2D1::RoundedRect(scrollbarThumb_, 2.0F, 2.0F),
                                                selectedTextBrush.Get());
        }
        const auto targetSize = renderTarget_->GetSize();
        const float borderWidth =
            static_cast<float>(visualConfig_.geometry.borderWidth.value_or(1.0));
        if (borderWidth > 0.0F && targetSize.width > borderWidth &&
            targetSize.height > borderWidth) {
            const float inset = borderWidth / 2.0F;
            const float radius =
                static_cast<float>(visualConfig_.geometry.cornerRadius.value_or(8.0));
            renderTarget_->DrawRoundedRectangle(
                D2D1::RoundedRect(
                    D2D1::RectF(inset, inset, targetSize.width - inset, targetSize.height - inset),
                    radius, radius),
                borderBrush.Get(), borderWidth);
        }
        const HRESULT result = renderTarget_->EndDraw();
        if (result == D2DERR_RECREATE_TARGET) {
            renderTarget_.Reset();
            return createDeviceResources();
        }
        return SUCCEEDED(result);
    }

    void update(const fcitx::windows::protocol::KeyResponse& response) {
        using namespace fcitx::windows;
        // Re-read the visual config when the file changed. The launcher may
        // start this process before the config window saves a new orientation,
        // and the HWND_BROADCAST reload message can race with window creation;
        // comparing the file's last write time keeps the candidate window in
        // sync with the saved config on every snapshot without per-key IO cost.
        if (!safeMode_) {
            const auto data = localDataDirectory();
            const auto path = data / L"config.toml";
            std::error_code error;
            const auto written = std::filesystem::last_write_time(path, error);
            if (!error && written != configWriteTime_) {
                configWriteTime_ = written;
                refreshVisualConfig();
            }
        }
        candidate::Snapshot snapshot;
        snapshot.engineEpoch = response.metadata.engineEpoch;
        snapshot.contextId = response.metadata.contextId;
        snapshot.compositionId = response.metadata.compositionId;
        snapshot.revision = response.metadata.revision;
        snapshot.preedit = response.preeditUtf8;
        snapshot.selected = response.selectedCandidate == UINT32_MAX
                                ? std::optional<std::size_t>{}
                                : std::optional<std::size_t>{response.selectedCandidate};
        snapshot.page = response.candidatePage;
        snapshot.total = response.candidateTotal;
        snapshot.visibility = response.candidateVisibility == 2 ? candidate::Visibility::prediction
                              : response.candidateVisibility == 1
                                  ? candidate::Visibility::composition
                                  : candidate::Visibility::hidden;
        snapshot.candidates.reserve(response.candidates.size());
        for (const auto& source : response.candidates) {
            snapshot.candidates.push_back(
                candidate::Item{source.id, source.labelUtf8, source.textUtf8, source.commentUtf8});
        }
        const auto applied = model_.apply(std::move(snapshot));
        if (applied == candidate::ApplyResult::stale || applied == candidate::ApplyResult::invalid)
            return;
        clickInFlight_ = false;
        KillTimer(window_, kClickGuardTimer);
        lastCandidateBulk_ = response.candidateBulk;
        lastCandidatePageSize_ = response.candidatePageSize;
        if (response.caret.valid)
            lastCaret_ = response.caret;
        const float requestedFontScale = static_cast<float>(lastCaret_.dpi) / 96.0F;
        if (requestedFontScale != fontDpiScale_) {
            fontDpiScale_ = requestedFontScale;
            textFormat_.Reset();
            labelFormat_.Reset();
            annotationFormat_.Reset();
            if (!createDeviceResources())
                return;
        }
        candidates_.clear();
        itemRects_.clear();
        visibleIndices_.clear();
        renderIndices_.clear();
        const auto& current = *model_.current();
        selected_ = current.selected;
        if (compositionId_ != current.compositionId) {
            placement_ = ui::Placement::unlocked;
            compositionId_ = current.compositionId;
            scrollExpanded_ = response.candidatePage > 0U;
            lastCandidatePage_.reset();
        }
        const bool scrollEligible = visualConfig_.scrollMode.value_or(false) &&
                                    response.candidateBulk && response.candidatePageSize > 0U &&
                                    current.candidates.size() > response.candidatePageSize;
        if (scrollEligible && lastCandidatePage_ && response.candidatePage != *lastCandidatePage_) {
            scrollExpanded_ = !(*lastCandidatePage_ == 1U && response.candidatePage == 0U);
        }
        lastCandidatePage_ = response.candidatePage;
        scrollMode_ = scrollEligible && scrollExpanded_;
        const std::size_t ordinaryCount =
            response.candidatePageSize == 0U
                ? current.candidates.size()
                : std::min<std::size_t>(response.candidatePageSize, current.candidates.size());
        const std::size_t ordinaryStart =
            response.candidateBulk && !scrollMode_
                ? std::min<std::size_t>(static_cast<std::size_t>(response.candidatePage) *
                                            response.candidatePageSize,
                                        current.candidates.size() - ordinaryCount)
                : 0U;
        // In the ordinary (non-bulk) lane the engine reports the selected
        // candidate as a page-local index, while candidates_ is built from the
        // full list and renderIndices_ addresses it with a global offset
        // (ordinaryStart). Translate the selection to the global index so the
        // paint loop can match it against visibleIndices_.
        if (!scrollMode_ && !response.candidateBulk && selected_ &&
            *selected_ < ordinaryCount && ordinaryStart + *selected_ < current.candidates.size()) {
            selected_ = ordinaryStart + *selected_;
        }
        for (std::size_t candidateIndex = 0; candidateIndex < current.candidates.size();
             ++candidateIndex) {
            const auto& candidate = current.candidates[candidateIndex];
            std::wstring label;
            std::wstring text;
            std::wstring comment;
            if (!utf8ToWide(candidate.label, label) || !utf8ToWide(candidate.text, text) ||
                !utf8ToWide(candidate.comment, comment))
                continue;
            CandidateVisual visual;
            if (scrollMode_) {
                const std::size_t selectedRow = selected_.value_or(0U) / 6U;
                if (candidateIndex / 6U == selectedRow)
                    label = std::to_wstring(candidateIndex % 6U + 1U);
                else
                    label.clear();
            } else if (response.candidateBulk && candidateIndex >= ordinaryStart &&
                       candidateIndex < ordinaryStart + ordinaryCount) {
                label = std::to_wstring(candidateIndex - ordinaryStart + 1U);
            }
            if (visualConfig_.label.visible.value_or(true) && !label.empty()) {
                using fcitx::windows::config::LabelStyle;
                switch (visualConfig_.label.style.value_or(LabelStyle::dot)) {
                case LabelStyle::plain:
                    visual.label = label + L" ";
                    break;
                case LabelStyle::dot:
                    visual.label = label + L". ";
                    break;
                case LabelStyle::paren:
                    visual.label = L"(" + label + L") ";
                    break;
                case LabelStyle::bracket:
                    visual.label = L"[" + label + L"] ";
                    break;
                case LabelStyle::circled:
                    if (label.size() == 1 && label[0] >= L'1' && label[0] <= L'9')
                        visual.label.assign(1, static_cast<wchar_t>(0x2460 + label[0] - L'1'));
                    else
                        visual.label = label;
                    visual.label += L" ";
                    break;
                }
            }
            visual.text = std::move(text);
            if (!comment.empty())
                visual.comment = L"  " + comment;
            candidates_.emplace_back(std::move(visual));
        }
        if (scrollMode_) {
            for (std::size_t index = 0; index < candidates_.size(); ++index)
                renderIndices_.push_back(index);
        } else {
            for (std::size_t index = ordinaryStart;
                 index < std::min(candidates_.size(), ordinaryStart + ordinaryCount); ++index)
                renderIndices_.push_back(index);
        }
        if (current.visibility == candidate::Visibility::hidden || candidates_.empty() ||
            !lastCaret_.valid) {
            dismissPresentation();
            return;
        }
        targetForegroundWindow_ = GetForegroundWindow();
        targetForegroundProcessId_ = 0;
        if (targetForegroundWindow_)
            GetWindowThreadProcessId(targetForegroundWindow_, &targetForegroundProcessId_);
        POINT caretPoint{lastCaret_.left, lastCaret_.top};
        HMONITOR monitor = MonitorFromPoint(caretPoint, MONITOR_DEFAULTTONEAREST);
        MONITORINFO monitorInfo{};
        monitorInfo.cbSize = sizeof(monitorInfo);
        GetMonitorInfoW(monitor, &monitorInfo);
        const float scale = static_cast<float>(lastCaret_.dpi) / 96.0F;
        const float itemPaddingX =
            static_cast<float>(visualConfig_.geometry.itemPaddingX.value_or(6.0) * scale);
        const float itemPaddingY =
            static_cast<float>(visualConfig_.geometry.itemPaddingY.value_or(4.0) * scale);
        selectionInflateX_ = itemPaddingX * 0.65F;
        selectionInflateY_ = itemPaddingY * 0.55F;
        std::vector<ui::Size> items;
        items.reserve(renderIndices_.size());
        for (const auto candidateIndex : renderIndices_) {
            const auto& candidate = candidates_[candidateIndex];
            float width = 0.0F;
            float height = 0.0F;
            const auto measure = [&](const std::wstring& value, IDWriteTextFormat* format) {
                if (value.empty())
                    return true;
                ComPtr<IDWriteTextLayout> textLayout;
                DWRITE_TEXT_METRICS metrics{};
                if (!writeFactory_ || !format ||
                    FAILED(writeFactory_->CreateTextLayout(value.data(),
                                                           static_cast<UINT32>(value.size()),
                                                           format, 4096.0F, 512.0F, &textLayout)) ||
                    FAILED(textLayout->GetMetrics(&metrics)))
                    return false;
                width += metrics.widthIncludingTrailingWhitespace;
                height = (std::max)(height, metrics.height);
                return true;
            };
            if (measure(candidate.label, labelFormat_.Get()) &&
                measure(candidate.text, textFormat_.Get()) &&
                measure(candidate.comment, annotationFormat_.Get())) {
                items.push_back({width + itemPaddingX * 2, height + itemPaddingY * 2});
            } else {
                items.push_back({336 * scale, 32 * scale});
            }
        }
        const ui::LayoutInput input{
            visualConfig_.orientation.value_or(config::Orientation::vertical) ==
                    config::Orientation::horizontal
                ? ui::Orientation::horizontal
                : ui::Orientation::vertical,
            std::move(items),
            {static_cast<float>(lastCaret_.left), static_cast<float>(lastCaret_.top)},
            static_cast<float>((std::max)(1, lastCaret_.bottom - lastCaret_.top)),
            {static_cast<float>(monitorInfo.rcWork.left),
             static_cast<float>(monitorInfo.rcWork.top),
             static_cast<float>(monitorInfo.rcWork.right),
             static_cast<float>(monitorInfo.rcWork.bottom)},
            static_cast<float>(visualConfig_.maxWidth.value_or(720.0) * scale),
            static_cast<float>(visualConfig_.geometry.paddingX.value_or(8.0) * scale),
            static_cast<float>(visualConfig_.geometry.paddingY.value_or(6.0) * scale),
            static_cast<float>(visualConfig_.geometry.rowGap.value_or(2.0) * scale),
            static_cast<float>(visualConfig_.geometry.columnGap.value_or(8.0) * scale),
            placement_,
            scrollMode_,
            6U,
            6U,
            selected_.value_or(0U)};
        const auto layout = ui::layout(input);
        placement_ = layout.placement;
        const LONG left = static_cast<LONG>(layout.window.left);
        const LONG top = static_cast<LONG>(layout.window.top);
        const LONG width = static_cast<LONG>(layout.window.right - layout.window.left);
        const LONG height = static_cast<LONG>(layout.window.bottom - layout.window.top);
        for (std::size_t local = 0; local < layout.items.size(); ++local) {
            const auto& item = layout.items[local];
            itemRects_.push_back(D2D1::RectF(item.left - layout.window.left + itemPaddingX,
                                             item.top - layout.window.top + itemPaddingY,
                                             item.right - layout.window.left - itemPaddingX,
                                             item.bottom - layout.window.top - itemPaddingY));
            visibleIndices_.push_back(renderIndices_[layout.itemIndices[local]]);
        }
        hasScrollbar_ = layout.hasScrollbar;
        scrollbarTrack_ = D2D1::RectF(layout.scrollbarTrack.left - layout.window.left,
                                      layout.scrollbarTrack.top - layout.window.top,
                                      layout.scrollbarTrack.right - layout.window.left,
                                      layout.scrollbarTrack.bottom - layout.window.top);
        scrollbarThumb_ = D2D1::RectF(layout.scrollbarThumb.left - layout.window.left,
                                      layout.scrollbarThumb.top - layout.window.top,
                                      layout.scrollbarThumb.right - layout.window.left,
                                      layout.scrollbarThumb.bottom - layout.window.top);
        SetWindowPos(window_, HWND_TOPMOST, left, top, width, height,
                     SWP_NOACTIVATE | SWP_SHOWWINDOW);
        if (renderTarget_)
            renderTarget_->Resize(
                D2D1::SizeU(static_cast<UINT32>(width), static_cast<UINT32>(height)));
        InvalidateRect(window_, nullptr, FALSE);
    }

    void reflowCurrentModel() {
        if (!model_.current()) {
            InvalidateRect(window_, nullptr, FALSE);
            return;
        }
        const auto current = *model_.current();
        fcitx::windows::protocol::KeyResponse response;
        response.metadata.engineEpoch = current.engineEpoch;
        response.metadata.contextId = current.contextId;
        response.metadata.compositionId = current.compositionId;
        response.metadata.revision = current.revision;
        response.preeditUtf8 = current.preedit;
        response.selectedCandidate = current.selected
                                         ? static_cast<std::uint32_t>(*current.selected)
                                         : UINT32_MAX;
        response.candidatePage = current.page;
        response.candidatePageSize = lastCandidatePageSize_;
        response.candidateTotal = current.total;
        response.candidateBulk = lastCandidateBulk_;
        response.candidateEnd = true;
        response.candidateVisibility =
            current.visibility == fcitx::windows::candidate::Visibility::prediction
                ? 2U
            : current.visibility == fcitx::windows::candidate::Visibility::composition
                ? 1U
                : 0U;
        response.caret = lastCaret_;
        response.candidates.reserve(current.candidates.size());
        for (const auto& item : current.candidates) {
            response.candidates.push_back(
                {item.id, item.label, item.text, item.comment});
        }
        model_.reset();
        update(response);
    }

    void dismissPresentation() noexcept {
        ShowWindow(window_, SW_HIDE);
        if (GetCapture() == window_)
            ReleaseCapture();
        pressedCandidate_.reset();
        clickInFlight_ = false;
        KillTimer(window_, kClickGuardTimer);
        model_.reset();
        candidates_.clear();
        itemRects_.clear();
        visibleIndices_.clear();
        renderIndices_.clear();
        selected_.reset();
        compositionId_ = 0;
        targetForegroundWindow_ = nullptr;
        targetForegroundProcessId_ = 0;
    }

    [[nodiscard]] bool foregroundTargetIsValid() const noexcept {
        if (interactionTest_)
            return true;
        if (!targetForegroundProcessId_)
            return false;
        const HWND foreground = GetForegroundWindow();
        DWORD processId = 0;
        if (foreground)
            GetWindowThreadProcessId(foreground, &processId);
        return processId == targetForegroundProcessId_;
    }

    [[nodiscard]] bool dispatchCandidate(std::size_t localIndex) {
        if (clickInFlight_ || localIndex >= visibleIndices_.size() ||
            !foregroundTargetIsValid())
            return false;
        const std::size_t targetIndex = visibleIndices_[localIndex];
        if (targetIndex >= candidates_.size())
            return false;
        const auto& current = model_.current();
        if (!current || targetIndex >= current->candidates.size())
            return false;
        const auto intent = fcitx::windows::ui::makeCandidateSelectionIntent(
            targetForegroundProcessId_, current->engineEpoch, current->contextId,
            current->compositionId, current->revision, current->candidates[targetIndex].id);
        if (!intent.valid())
            return false;
        clickInFlight_ = true;
        SetTimer(window_, kClickGuardTimer, 750, nullptr);
        if (interactionTest_) {
            capturedTestIntent_ = intent;
            return true;
        }
        if (!candidateClient_ ||
            !candidateClient_->selectCandidate(
                intent.targetProcessId, intent.engineEpoch, intent.contextId,
                intent.compositionId, intent.revision, intent.candidateId)) {
            clickInFlight_ = false;
            KillTimer(window_, kClickGuardTimer);
            return false;
        }
        return true;
    }

  private:
    static LRESULT CALLBACK windowProcedure(HWND window, UINT message, WPARAM wparam,
                                            LPARAM lparam) {
        CandidateWindow* self = nullptr;
        if (message == WM_NCCREATE) {
            self = static_cast<CandidateWindow*>(
                reinterpret_cast<CREATESTRUCTW*>(lparam)->lpCreateParams);
            SetWindowLongPtrW(window, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(self));
        } else {
            self = reinterpret_cast<CandidateWindow*>(GetWindowLongPtrW(window, GWLP_USERDATA));
        }
        if (self && message == WM_PAINT) {
            PAINTSTRUCT paint{};
            BeginPaint(window, &paint);
            self->paintOnce();
            EndPaint(window, &paint);
            return 0;
        }
        if (self && message == kSnapshotMessage) {
            std::unique_ptr<fcitx::windows::protocol::KeyResponse> response(
                reinterpret_cast<fcitx::windows::protocol::KeyResponse*>(lparam));
            self->update(*response);
            return 0;
        }
        if (self && message == visualConfigChangedMessage()) {
            self->reloadVisualConfig();
            return 0;
        }
        if (self && message == candidateDismissMessage()) {
            const auto sourceContext = static_cast<std::uint64_t>(lparam);
            const auto& current = self->model_.current();
            const bool sameContext = sourceContext == 0 ||
                                     (current && sourceContext == current->contextId);
            if ((wparam == 0 ||
                 static_cast<DWORD>(wparam) == self->targetForegroundProcessId_) &&
                sameContext)
                self->dismissPresentation();
            return 0;
        }
        if (self && message == WM_TIMER) {
            if (wparam == kFocusWatchTimer && IsWindowVisible(window) &&
                !self->foregroundTargetIsValid()) {
                self->dismissPresentation();
            } else if (wparam == kClickGuardTimer) {
                self->clickInFlight_ = false;
                KillTimer(window, kClickGuardTimer);
            }
            return 0;
        }
        if (self && message == WM_DPICHANGED) {
            const auto* suggested = reinterpret_cast<const RECT*>(lparam);
            SetWindowPos(window, nullptr, suggested->left, suggested->top,
                         suggested->right - suggested->left, suggested->bottom - suggested->top,
                         SWP_NOACTIVATE | SWP_NOZORDER);
            if (self->renderTarget_) {
                self->renderTarget_->SetDpi(96.0F, 96.0F);
            }
            return 0;
        }
        if (self && (message == WM_SETTINGCHANGE || message == WM_THEMECHANGED ||
                     message == WM_SYSCOLORCHANGE)) {
            self->reloadVisualConfig();
            return 0;
        }
        if (self && message == WM_MOUSEACTIVATE)
            return MA_NOACTIVATE;
        if (self && message == WM_LBUTTONDOWN) {
            const float x = static_cast<float>(static_cast<short>(LOWORD(lparam)));
            const float y = static_cast<float>(static_cast<short>(HIWORD(lparam)));
            self->pressedCandidate_ =
                fcitx::windows::ui::hitTestCandidate(self->itemRects_, x, y);
            if (self->pressedCandidate_)
                SetCapture(window);
            return 0;
        }
        if (self && message == WM_LBUTTONUP) {
            const float x = static_cast<float>(static_cast<short>(LOWORD(lparam)));
            const float y = static_cast<float>(static_cast<short>(HIWORD(lparam)));
            const auto released =
                fcitx::windows::ui::hitTestCandidate(self->itemRects_, x, y);
            const auto pressed = self->pressedCandidate_;
            self->pressedCandidate_.reset();
            if (GetCapture() == window)
                ReleaseCapture();
            if (pressed && released == pressed)
                (void)self->dispatchCandidate(*pressed);
            return 0;
        }
        if (self && (message == WM_CANCELMODE || message == WM_CAPTURECHANGED)) {
            self->pressedCandidate_.reset();
            return 0;
        }
        if (message == WM_NCHITTEST)
            return HTCLIENT;
        if (message == WM_DESTROY) {
            PostQuitMessage(0);
            return 0;
        }
        return DefWindowProcW(window, message, wparam, lparam);
    }

    bool createDeviceResources() {
        if (!d2dFactory_ && FAILED(D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED,
                                                     d2dFactory_.GetAddressOf())))
            return false;
        if (!writeFactory_ &&
            FAILED(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory),
                                       reinterpret_cast<IUnknown**>(writeFactory_.GetAddressOf()))))
            return false;
        std::wstring fontFamily = L"Segoe UI";
        if (visualConfig_.candidateFont.families) {
            for (const auto& family : *visualConfig_.candidateFont.families) {
                if (family != "system" && family != "inherit" && utf8ToWide(family, fontFamily))
                    break;
            }
        }
        const auto createFormat = [&](const fcitx::windows::config::Font& font, double scale,
                                      ComPtr<IDWriteTextFormat>& format) {
            if (format)
                return true;
            std::wstring family = fontFamily;
            if (font.families) {
                for (const auto& candidate : *font.families) {
                    if (candidate != "system" && candidate != "inherit" &&
                        utf8ToWide(candidate, family))
                        break;
                }
            }
            if (FAILED(writeFactory_->CreateTextFormat(
                    family.c_str(), nullptr,
                    static_cast<DWRITE_FONT_WEIGHT>(
                        font.weight.value_or(visualConfig_.candidateFont.weight.value_or(400))),
                    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL,
                    static_cast<float>(
                        font.size.value_or(visualConfig_.candidateFont.size.value_or(16.0)) *
                        scale * fontDpiScale_),
                    L"zh-CN", &format)))
                return false;
            // Single line with ellipsis trimming: a label/comment longer than
            // the remaining row width must not wrap onto the candidate row
            // below (which visually overlaps the next candidate).
            if (FAILED(format->SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)))
                return false;
            DWRITE_TRIMMING trimming{DWRITE_TRIMMING_GRANULARITY_CHARACTER, 0, 0};
            ComPtr<IDWriteInlineObject> ellipsis;
            if (FAILED(writeFactory_->CreateEllipsisTrimmingSign(format.Get(), &ellipsis)) ||
                FAILED(format->SetTrimming(&trimming, ellipsis.Get())))
                return false;
            return true;
        };
        if (!createFormat(visualConfig_.candidateFont, 1.0, textFormat_) ||
            !createFormat(visualConfig_.candidateFont, visualConfig_.label.fontScale.value_or(0.85),
                          labelFormat_) ||
            !createFormat(visualConfig_.annotationFont,
                          visualConfig_.annotationFont.scale.value_or(0.85), annotationFormat_))
            return false;
        if (!renderTarget_) {
            RECT client{};
            GetClientRect(window_, &client);
            const auto size =
                D2D1::SizeU(static_cast<UINT32>(client.right), static_cast<UINT32>(client.bottom));
            if (FAILED(d2dFactory_->CreateHwndRenderTarget(
                    D2D1::RenderTargetProperties(), D2D1::HwndRenderTargetProperties(window_, size),
                    &renderTarget_)))
                return false;
            renderTarget_->SetDpi(96.0F, 96.0F);
        }
        return true;
    }

    HWND window_{};
    ComPtr<ID2D1Factory> d2dFactory_;
    ComPtr<IDWriteFactory> writeFactory_;
    ComPtr<IDWriteTextFormat> textFormat_;
    ComPtr<IDWriteTextFormat> labelFormat_;
    ComPtr<IDWriteTextFormat> annotationFormat_;
    ComPtr<ID2D1HwndRenderTarget> renderTarget_;
    std::vector<CandidateVisual> candidates_;
    std::vector<D2D1_RECT_F> itemRects_;
    std::vector<std::size_t> visibleIndices_;
    std::vector<std::size_t> renderIndices_;
    std::optional<std::size_t> selected_;
    std::optional<std::size_t> pressedCandidate_;
    fcitx::windows::config::Config visualConfig_;
    fcitx::windows::candidate::CandidateModel model_;
    fcitx::windows::protocol::CaretRect lastCaret_;
    fcitx::windows::ui::Placement placement_{fcitx::windows::ui::Placement::unlocked};
    std::uint64_t compositionId_{};
    bool safeMode_{};
    bool scrollMode_{};
    bool scrollExpanded_{};
    bool lastCandidateBulk_{};
    std::uint32_t lastCandidatePageSize_{};
    std::optional<std::uint32_t> lastCandidatePage_;
    bool hasScrollbar_{};
    float fontDpiScale_{1.0F};
    float selectionInflateX_{};
    float selectionInflateY_{};
    D2D1_RECT_F scrollbarTrack_{};
    D2D1_RECT_F scrollbarThumb_{};
    HWND targetForegroundWindow_{};
    DWORD targetForegroundProcessId_{};
    bool clickInFlight_{};
    bool interactionTest_{};
    std::optional<fcitx::windows::ui::CandidateSelectionIntent> capturedTestIntent_;
    std::unique_ptr<fcitx::windows::ipc::PipeClient> candidateClient_;
    std::filesystem::file_time_type configWriteTime_{};
};

bool readExact(HANDLE pipe, void* destination, std::size_t size) {
    auto* bytes = static_cast<std::uint8_t*>(destination);
    std::size_t offset = 0;
    while (offset < size) {
        DWORD read = 0;
        if (!ReadFile(pipe, bytes + offset, static_cast<DWORD>(size - offset), &read, nullptr) ||
            read == 0)
            return false;
        offset += read;
    }
    return true;
}

void servePresentation(HWND window, bool testOnce) {
    using namespace fcitx::windows;
    platform::RuntimeIdentity identity;
    platform::PipeSecurity security;
    if (!platform::queryCurrentIdentity(identity) ||
        !platform::PipeSecurity::create(identity, security))
        return;
    std::wstring executable(32'768, L'\0');
    const DWORD size =
        GetModuleFileNameW(nullptr, executable.data(), static_cast<DWORD>(executable.size()));
    if (size == 0 || size >= executable.size())
        return;
    executable.resize(size);
    const auto engine =
        (std::filesystem::path(executable).parent_path() / "fcitx5-engine.exe").wstring();
    const auto pipeName = platform::makeLocalEndpointName(identity, L"presentation");
    for (;;) {
        HANDLE pipe = CreateNamedPipeW(
            pipeName.c_str(), PIPE_ACCESS_INBOUND,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS, 1,
            static_cast<DWORD>(protocol::kMaxHotFrameSize),
            static_cast<DWORD>(protocol::kMaxHotFrameSize), 25, security.attributes());
        if (pipe == INVALID_HANDLE_VALUE)
            return;
        const bool connected =
            ConnectNamedPipe(pipe, nullptr) != FALSE || GetLastError() == ERROR_PIPE_CONNECTED;
        platform::ProcessIdentity peer;
        if (connected && ipc::verifyPipeClient(pipe, identity, &peer) &&
            platform::pathsReferToSameFile(peer.executablePath, engine)) {
            for (;;) {
                std::array<std::uint8_t, protocol::kHeaderSize> header{};
                if (!readExact(pipe, header.data(), header.size()))
                    break;
                protocol::MessageType type{};
                std::uint32_t bodySize = 0;
                protocol::Metadata metadata;
                if (!protocol::decodeHeader(header, type, bodySize, metadata) ||
                    type != protocol::MessageType::keyResponse)
                    break;
                std::vector<std::uint8_t> frame(header.begin(), header.end());
                frame.resize(protocol::kHeaderSize + bodySize);
                if (bodySize && !readExact(pipe, frame.data() + protocol::kHeaderSize, bodySize))
                    break;
                protocol::FrameView view;
                auto response = std::make_unique<protocol::KeyResponse>();
                if (!protocol::decodeFrame(frame, view) || !protocol::decode(view, *response))
                    break;
                if (!PostMessageW(window, kSnapshotMessage, 0,
                                  reinterpret_cast<LPARAM>(response.get())))
                    return;
                (void)response.release();
                if (testOnce) {
                    PostMessageW(window, WM_CLOSE, 0, 0);
                    return;
                }
            }
        }
        DisconnectNamedPipe(pipe);
        CloseHandle(pipe);
    }
}

} // namespace

int WINAPI wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE, _In_ PWSTR commandLine, _In_ int) {
    enableDpiAwareness();
    int argumentCount = 0;
    wchar_t** argumentValues = CommandLineToArgvW(GetCommandLineW(), &argumentCount);
    if (argumentValues && argumentCount > 1 &&
        std::wstring_view(argumentValues[1]) == L"--candidate-select-test") {
        const int result = runCandidateSelectionTest(argumentCount, argumentValues);
        LocalFree(argumentValues);
        return result;
    }
    if (argumentValues) LocalFree(argumentValues);
    const std::wstring_view arguments = commandLine ? commandLine : L"";
    const bool selfTest = arguments.find(L"--self-test") != std::wstring_view::npos;
    const bool interactionSelfTest =
        arguments.find(L"--interaction-self-test") != std::wstring_view::npos;
    const bool reloadTest = arguments.find(L"--reload-test") != std::wstring_view::npos;
    const bool simulateDeviceLoss =
        arguments.find(L"--simulate-device-loss") != std::wstring_view::npos;
    const bool scrollDemo = arguments.find(L"--scroll-demo") != std::wstring_view::npos;
    const bool demo = interactionSelfTest || scrollDemo ||
                      arguments.find(L"--demo") != std::wstring_view::npos;
    const bool testOnce = arguments.find(L"--test-once") != std::wstring_view::npos;
    const bool safeMode = arguments.find(L"--safe-mode") != std::wstring_view::npos;
    CandidateWindow window;
    if (!window.create(instance, demo, safeMode, interactionSelfTest) || !window.paintOnce())
        return 1;
    if (demo)
        window.showSyntheticPreview(scrollDemo);
    if (interactionSelfTest)
        return window.runInteractionSelfTest() ? 0 : 2;
    if (simulateDeviceLoss) {
        window.simulateDeviceLossForTest();
        if (!window.paintOnce())
            return 1;
    }
    if (reloadTest) {
        window.showSyntheticPreview(false);
        SendMessageW(window.handle(), visualConfigChangedMessage(), 0, 0);
        if (!window.paintOnce())
            return 1;
    }
    if (selfTest)
        return 0;
    const auto parentMarker = arguments.find(L"--parent-pid ");
    if (parentMarker != std::wstring_view::npos) {
        const wchar_t* number = arguments.data() + parentMarker + 13;
        wchar_t* end = nullptr;
        const unsigned long parentId = std::wcstoul(number, &end, 10);
        if (end != number && parentId != 0) {
            const HANDLE parent = OpenProcess(SYNCHRONIZE, FALSE, parentId);
            if (parent) {
                const HWND handle = window.handle();
                std::thread([parent, handle] {
                    WaitForSingleObject(parent, INFINITE);
                    CloseHandle(parent);
                    PostMessageW(handle, WM_CLOSE, 0, 0);
                }).detach();
            }
        }
    }
    std::thread(servePresentation, window.handle(), testOnce).detach();
    return window.run();
}
