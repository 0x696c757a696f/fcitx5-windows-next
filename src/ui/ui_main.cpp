#include "candidate_layout.h"
#include "candidate_model.h"
#include "config_model.h"
#include "peer_verification.h"
#include "pipe_security.h"
#include "protocol.h"
#include "runtime_identity.h"

#include <Windows.h>
#include <d2d1.h>
#include <dwrite.h>
#include <ShlObj.h>
#include <wrl/client.h>

#include <array>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <memory>
#include <optional>
#include <sstream>
#include <string>
#include <string_view>
#include <thread>
#include <vector>

namespace {

using Microsoft::WRL::ComPtr;
constexpr UINT kSnapshotMessage = WM_APP + 1;

struct CandidateVisual {
    std::wstring label;
    std::wstring text;
    std::wstring comment;
};

void enableDpiAwareness() {
    using SetContext = BOOL(WINAPI*)(HANDLE);
    const HMODULE user32 = GetModuleHandleW(L"user32.dll");
    const auto setContext = user32
                                ? reinterpret_cast<SetContext>(GetProcAddress(
                                      user32, "SetProcessDpiAwarenessContext"))
                                : nullptr;
    if (setContext && setContext(reinterpret_cast<HANDLE>(-4))) return;
    (void)SetProcessDPIAware();
}

bool utf8ToWide(std::string_view input, std::wstring& output) {
    if (input.empty()) {
        output.clear();
        return true;
    }
    const int size = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                                         static_cast<int>(input.size()), nullptr, 0);
    if (size <= 0) return false;
    output.resize(static_cast<std::size_t>(size));
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                               static_cast<int>(input.size()), output.data(), size) == size;
}

std::optional<std::string> readBoundedFile(const std::filesystem::path& path,
                                           std::size_t maximum) {
    std::error_code error;
    const auto size = std::filesystem::file_size(path, error);
    if (error || size > maximum) return std::nullopt;
    std::ifstream stream(path, std::ios::binary);
    if (!stream) return std::nullopt;
    std::string contents(static_cast<std::size_t>(size), '\0');
    if (size != 0 && !stream.read(contents.data(), static_cast<std::streamsize>(size)))
        return std::nullopt;
    return contents;
}

bool systemUsesDarkAppearance() noexcept {
    DWORD light = 1;
    DWORD size = sizeof(light);
    if (RegGetValueW(HKEY_CURRENT_USER,
                     L"Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                     L"AppsUseLightTheme", RRF_RT_REG_DWORD, nullptr, &light, &size) !=
        ERROR_SUCCESS) return false;
    return light == 0;
}

std::filesystem::path executableDirectory() {
    std::wstring path(32'768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    if (size == 0 || size >= path.size()) return {};
    path.resize(size);
    return std::filesystem::path(path).parent_path();
}

std::filesystem::path localDataDirectory() {
    const auto executable = executableDirectory();
    if (!executable.empty() &&
        std::filesystem::exists(executable / L"portable.flag")) {
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
    return result / L"Fcitx5";
}

fcitx::windows::config::Config loadVisualConfig(bool safeMode) {
    using namespace fcitx::windows::config;
    Config defaults;
    ParseError error;
    if (!parseConfig(defaultConfigToml(), defaults, error)) return {};
    Config user;
    const auto data = localDataDirectory();
    if (!safeMode && !data.empty()) {
        if (const auto text = readBoundedFile(data / L"config.toml", 256U * 1024U)) {
            Config parsed;
            if (parseConfig(*text, parsed, error)) user = std::move(parsed);
        }
    }
    const AppearanceMode mode = user.appearanceMode.value_or(
        defaults.appearanceMode.value_or(AppearanceMode::system));
    const bool dark = mode == AppearanceMode::dark ||
                      (mode == AppearanceMode::system && systemUsesDarkAppearance());
    const std::string themeId = safeMode ? "builtin:default"
                                         : user.theme.value_or("builtin:default");
    std::filesystem::path themePath;
    if (themeId == "builtin:default") {
        themePath = executableDirectory() / L"resources" / L"themes" / L"default" /
                    L"theme.toml";
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

D2D1_COLOR_F parseColor(const fcitx::windows::config::Config& config,
                        std::string_view name, D2D1_COLOR_F fallback) {
    const auto found = config.colors.find(std::string(name));
    if (found == config.colors.end()) return fallback;
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
    bool create(HINSTANCE instance, bool visible, bool safeMode) {
        safeMode_ = safeMode;
        visualConfig_ = loadVisualConfig(safeMode_);
        WNDCLASSW windowClass{};
        windowClass.hInstance = instance;
        windowClass.lpszClassName = L"Fcitx5WindowsNextCandidate";
        windowClass.lpfnWndProc = windowProcedure;
        windowClass.hCursor = LoadCursorW(nullptr, IDC_ARROW);
        RegisterClassW(&windowClass);
        window_ = CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST |
                                      WS_EX_LAYERED,
                                  windowClass.lpszClassName, L"", WS_POPUP,
                                  100, 100, 360, 120, nullptr, nullptr, instance, this);
        if (!window_) return false;
        const LONG_PTR styles = GetWindowLongPtrW(window_, GWL_EXSTYLE);
        if ((styles & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE)) !=
                (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) ||
            (styles & WS_EX_APPWINDOW) != 0) return false;
        const auto opacity = visualConfig_.opacity.value_or(1.0);
        SetLayeredWindowAttributes(
            window_, 0, static_cast<BYTE>(std::clamp(opacity, 0.2, 1.0) * 255.0), LWA_ALPHA);
        if (visible) ShowWindow(window_, SW_SHOWNOACTIVATE);
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

    void showSyntheticPreview() {
        fcitx::windows::protocol::KeyResponse response;
        response.metadata.engineEpoch = 1;
        response.metadata.contextId = 1;
        response.metadata.compositionId = 1;
        response.metadata.revision = 1;
        response.candidates = {{1, "1", "输入法", "shūrùfǎ"},
                               {2, "2", "输入", "shūrù"},
                               {3, "3", "中文", "zhōngwén"}};
        response.selectedCandidate = 0;
        response.candidateTotal = static_cast<std::uint32_t>(response.candidates.size());
        response.candidateVisibility = 1;
        response.caret = {true, 100, 100, 102, 124, 96};
        update(response);
    }

    void reloadVisualConfig() {
        visualConfig_ = loadVisualConfig(safeMode_);
        textFormat_.Reset();
        labelFormat_.Reset();
        annotationFormat_.Reset();
        const auto opacity = visualConfig_.opacity.value_or(1.0);
        SetLayeredWindowAttributes(
            window_, 0, static_cast<BYTE>(std::clamp(opacity, 0.2, 1.0) * 255.0), LWA_ALPHA);
        (void)createDeviceResources();
        InvalidateRect(window_, nullptr, FALSE);
    }

    bool paintOnce() {
        if (!createDeviceResources()) return false;
        renderTarget_->BeginDraw();
        HIGHCONTRASTW contrast{};
        contrast.cbSize = sizeof(contrast);
        const bool highContrast =
            SystemParametersInfoW(SPI_GETHIGHCONTRAST, sizeof(contrast), &contrast, 0) &&
            (contrast.dwFlags & HCF_HIGHCONTRASTON) != 0;
        const COLORREF systemBackground = GetSysColor(COLOR_WINDOW);
        const COLORREF systemForeground = GetSysColor(COLOR_WINDOWTEXT);
        const auto background = highContrast
                                    ? D2D1::ColorF(GetRValue(systemBackground) / 255.0F,
                                                   GetGValue(systemBackground) / 255.0F,
                                                   GetBValue(systemBackground) / 255.0F)
                                    : parseColor(visualConfig_, "background",
                                                 D2D1::ColorF(0.97F, 0.98F, 0.98F));
        const auto foreground = highContrast
                                    ? D2D1::ColorF(GetRValue(systemForeground) / 255.0F,
                                                   GetGValue(systemForeground) / 255.0F,
                                                   GetBValue(systemForeground) / 255.0F)
                                    : parseColor(visualConfig_, "candidate_text",
                                                 D2D1::ColorF(0.13F, 0.13F, 0.14F));
        const auto selectedBackground = highContrast
                                            ? D2D1::ColorF(
                                                  GetRValue(GetSysColor(COLOR_HIGHLIGHT)) / 255.0F,
                                                  GetGValue(GetSysColor(COLOR_HIGHLIGHT)) / 255.0F,
                                                  GetBValue(GetSysColor(COLOR_HIGHLIGHT)) / 255.0F)
                                            : parseColor(visualConfig_, "selected_background",
                                                         D2D1::ColorF(0.82F, 0.89F, 0.99F));
        const auto selectedForeground = highContrast
                                            ? D2D1::ColorF(
                                                  GetRValue(GetSysColor(COLOR_HIGHLIGHTTEXT)) /
                                                      255.0F,
                                                  GetGValue(GetSysColor(COLOR_HIGHLIGHTTEXT)) /
                                                      255.0F,
                                                  GetBValue(GetSysColor(COLOR_HIGHLIGHTTEXT)) /
                                                      255.0F)
                                            : parseColor(
                                                  visualConfig_, "selected_candidate_text",
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
        const auto labelColor = highContrast
                                    ? foreground
                                    : parseColor(visualConfig_, "label_text", foreground);
        const auto commentColor = highContrast
                                      ? foreground
                                      : parseColor(visualConfig_, "comment_text", foreground);
        const auto selectedLabelColor = highContrast
                                            ? selectedForeground
                                            : parseColor(visualConfig_, "selected_label_text",
                                                         selectedForeground);
        const auto selectedCommentColor = highContrast
                                              ? selectedForeground
                                              : parseColor(visualConfig_,
                                                           "selected_comment_text",
                                                           selectedForeground);
        const auto borderColor = highContrast
                                     ? foreground
                                     : parseColor(visualConfig_, "border",
                                                  D2D1::ColorF(0.82F, 0.82F, 0.82F));
        if (FAILED(renderTarget_->CreateSolidColorBrush(foreground, &textBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(labelColor, &labelBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(commentColor, &commentBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedBackground, &selectionBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedForeground,
                                                         &selectedTextBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedLabelColor,
                                                         &selectedLabelBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedCommentColor,
                                                         &selectedCommentBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(borderColor, &borderBrush)))
            return false;
        const std::vector<CandidateVisual> fallback{{L"1. ", L"你", L"nǐ"},
                                                     {L"2. ", L"呢", L""}};
        const auto& lines = candidates_.empty() ? fallback : candidates_;
        float fallbackTop = 8.0F;
        for (std::size_t index = 0; index < lines.size(); ++index) {
            const auto& candidate = lines[index];
            const D2D1_RECT_F bounds = itemRects_.size() == lines.size()
                                           ? itemRects_[index]
                                           : D2D1::RectF(12, fallbackTop, 348,
                                                         fallbackTop + 32);
            const bool selected = selected_ && *selected_ == index;
            if (selected) {
                const float radius = static_cast<float>(
                    visualConfig_.geometry.cornerRadius.value_or(8.0));
                renderTarget_->FillRoundedRectangle(
                    D2D1::RoundedRect(bounds, radius, radius), selectionBrush.Get());
            }
            float left = bounds.left;
            const auto drawSegment = [&](const std::wstring& value, IDWriteTextFormat* format,
                                         ID2D1Brush* brush) {
                if (value.empty()) return;
                ComPtr<IDWriteTextLayout> layout;
                DWRITE_TEXT_METRICS metrics{};
                if (FAILED(writeFactory_->CreateTextLayout(
                        value.data(), static_cast<UINT32>(value.size()), format,
                        (std::max)(1.0F, bounds.right - left),
                        (std::max)(1.0F, bounds.bottom - bounds.top), &layout)) ||
                    FAILED(layout->GetMetrics(&metrics))) return;
                const D2D1_RECT_F segment = D2D1::RectF(
                    left, bounds.top, bounds.right, bounds.bottom);
                renderTarget_->DrawTextW(value.data(), static_cast<UINT32>(value.size()),
                                         format, segment, brush);
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
        const auto targetSize = renderTarget_->GetSize();
        const float borderWidth = static_cast<float>(
            visualConfig_.geometry.borderWidth.value_or(1.0));
        if (borderWidth > 0.0F && targetSize.width > borderWidth &&
            targetSize.height > borderWidth) {
            const float inset = borderWidth / 2.0F;
            const float radius = static_cast<float>(
                visualConfig_.geometry.cornerRadius.value_or(8.0));
            renderTarget_->DrawRoundedRectangle(
                D2D1::RoundedRect(
                    D2D1::RectF(inset, inset, targetSize.width - inset,
                                targetSize.height - inset),
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
        snapshot.visibility = response.candidateVisibility == 2
                                  ? candidate::Visibility::prediction
                                  : response.candidateVisibility == 1
                                        ? candidate::Visibility::composition
                                        : candidate::Visibility::hidden;
        snapshot.candidates.reserve(response.candidates.size());
        for (const auto& source : response.candidates) {
            snapshot.candidates.push_back(candidate::Item{
                source.id, source.labelUtf8, source.textUtf8, source.commentUtf8});
        }
        const auto applied = model_.apply(std::move(snapshot));
        if (applied == candidate::ApplyResult::stale ||
            applied == candidate::ApplyResult::invalid) return;
        if (response.caret.valid) lastCaret_ = response.caret;
        candidates_.clear();
        itemRects_.clear();
        const auto& current = *model_.current();
        selected_ = current.selected;
        for (const auto& candidate : current.candidates) {
            std::wstring label;
            std::wstring text;
            std::wstring comment;
            if (!utf8ToWide(candidate.label, label) ||
                !utf8ToWide(candidate.text, text) ||
                !utf8ToWide(candidate.comment, comment)) continue;
            CandidateVisual visual;
            if (visualConfig_.label.visible.value_or(true) && !label.empty()) {
                using fcitx::windows::config::LabelStyle;
                switch (visualConfig_.label.style.value_or(LabelStyle::dot)) {
                case LabelStyle::plain: visual.label = label + L" "; break;
                case LabelStyle::dot: visual.label = label + L". "; break;
                case LabelStyle::paren: visual.label = L"(" + label + L") "; break;
                case LabelStyle::bracket: visual.label = L"[" + label + L"] "; break;
                case LabelStyle::circled:
                    if (label.size() == 1 && label[0] >= L'1' && label[0] <= L'9')
                        visual.label.assign(
                            1, static_cast<wchar_t>(0x2460 + label[0] - L'1'));
                    else
                        visual.label = label;
                    visual.label += L" ";
                    break;
                }
            }
            visual.text = std::move(text);
            if (!comment.empty()) visual.comment = L"  " + comment;
            candidates_.emplace_back(std::move(visual));
        }
        if (current.visibility == candidate::Visibility::hidden || candidates_.empty() ||
            !lastCaret_.valid) {
            ShowWindow(window_, SW_HIDE);
            return;
        }
        POINT caretPoint{lastCaret_.left, lastCaret_.top};
        HMONITOR monitor = MonitorFromPoint(caretPoint, MONITOR_DEFAULTTONEAREST);
        MONITORINFO monitorInfo{};
        monitorInfo.cbSize = sizeof(monitorInfo);
        GetMonitorInfoW(monitor, &monitorInfo);
        const float scale = static_cast<float>(lastCaret_.dpi) / 96.0F;
        const float itemPaddingX = static_cast<float>(
            visualConfig_.geometry.itemPaddingX.value_or(6.0) * scale);
        const float itemPaddingY = static_cast<float>(
            visualConfig_.geometry.itemPaddingY.value_or(4.0) * scale);
        std::vector<ui::Size> items;
        items.reserve(candidates_.size());
        for (const auto& candidate : candidates_) {
            float width = 0.0F;
            float height = 0.0F;
            const auto measure = [&](const std::wstring& value, IDWriteTextFormat* format) {
                if (value.empty()) return true;
                ComPtr<IDWriteTextLayout> textLayout;
                DWRITE_TEXT_METRICS metrics{};
                if (!writeFactory_ || !format || FAILED(writeFactory_->CreateTextLayout(
                        value.data(), static_cast<UINT32>(value.size()), format,
                        4096.0F, 512.0F, &textLayout)) ||
                    FAILED(textLayout->GetMetrics(&metrics))) return false;
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
        if (compositionId_ != current.compositionId) {
            placement_ = ui::Placement::unlocked;
            compositionId_ = current.compositionId;
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
            placement_};
        const auto layout = ui::layout(input);
        placement_ = layout.placement;
        const LONG left = static_cast<LONG>(layout.window.left);
        const LONG top = static_cast<LONG>(layout.window.top);
        const LONG width = static_cast<LONG>(layout.window.right - layout.window.left);
        const LONG height = static_cast<LONG>(layout.window.bottom - layout.window.top);
        for (const auto& item : layout.items) {
            itemRects_.push_back(D2D1::RectF(item.left - layout.window.left + itemPaddingX,
                                             item.top - layout.window.top + itemPaddingY,
                                             item.right - layout.window.left - itemPaddingX,
                                             item.bottom - layout.window.top - itemPaddingY));
        }
        SetWindowPos(window_, HWND_TOPMOST, left, top, width, height,
                     SWP_NOACTIVATE | SWP_SHOWWINDOW);
        if (renderTarget_) renderTarget_->Resize(
            D2D1::SizeU(static_cast<UINT32>(width), static_cast<UINT32>(height)));
        InvalidateRect(window_, nullptr, FALSE);
    }

private:
    static LRESULT CALLBACK windowProcedure(HWND window, UINT message,
                                             WPARAM wparam, LPARAM lparam) {
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
        if (self && message == WM_DPICHANGED) {
            const auto* suggested = reinterpret_cast<const RECT*>(lparam);
            SetWindowPos(window, nullptr, suggested->left, suggested->top,
                         suggested->right - suggested->left,
                         suggested->bottom - suggested->top,
                         SWP_NOACTIVATE | SWP_NOZORDER);
            if (self->renderTarget_) {
                const float dpi = static_cast<float>(LOWORD(wparam));
                self->renderTarget_->SetDpi(dpi, dpi);
            }
            return 0;
        }
        if (self && (message == WM_SETTINGCHANGE || message == WM_THEMECHANGED ||
                     message == WM_SYSCOLORCHANGE)) {
            self->reloadVisualConfig();
            return 0;
        }
        if (message == WM_NCHITTEST) return HTTRANSPARENT;
        if (message == WM_DESTROY) {
            PostQuitMessage(0);
            return 0;
        }
        return DefWindowProcW(window, message, wparam, lparam);
    }

    bool createDeviceResources() {
        if (!d2dFactory_ && FAILED(D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED,
                                                     d2dFactory_.GetAddressOf()))) return false;
        if (!writeFactory_ && FAILED(DWriteCreateFactory(
                                  DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory),
                                  reinterpret_cast<IUnknown**>(writeFactory_.GetAddressOf()))))
            return false;
        std::wstring fontFamily = L"Segoe UI";
        if (visualConfig_.candidateFont.families) {
            for (const auto& family : *visualConfig_.candidateFont.families) {
                if (family != "system" && family != "inherit" &&
                    utf8ToWide(family, fontFamily)) break;
            }
        }
        const auto createFormat = [&](const fcitx::windows::config::Font& font,
                                      double scale,
                                      ComPtr<IDWriteTextFormat>& format) {
            if (format) return true;
            std::wstring family = fontFamily;
            if (font.families) {
                for (const auto& candidate : *font.families) {
                    if (candidate != "system" && candidate != "inherit" &&
                        utf8ToWide(candidate, family)) break;
                }
            }
            return SUCCEEDED(writeFactory_->CreateTextFormat(
                family.c_str(), nullptr,
                static_cast<DWRITE_FONT_WEIGHT>(font.weight.value_or(
                    visualConfig_.candidateFont.weight.value_or(400))),
                DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL,
                static_cast<float>(font.size.value_or(
                    visualConfig_.candidateFont.size.value_or(16.0)) * scale),
                L"zh-CN", &format));
        };
        if (!createFormat(visualConfig_.candidateFont, 1.0, textFormat_) ||
            !createFormat(visualConfig_.candidateFont,
                          visualConfig_.label.fontScale.value_or(0.85), labelFormat_) ||
            !createFormat(visualConfig_.annotationFont,
                          visualConfig_.annotationFont.scale.value_or(0.85),
                          annotationFormat_)) return false;
        if (!renderTarget_) {
            RECT client{};
            GetClientRect(window_, &client);
            const auto size = D2D1::SizeU(static_cast<UINT32>(client.right),
                                         static_cast<UINT32>(client.bottom));
            if (FAILED(d2dFactory_->CreateHwndRenderTarget(
                    D2D1::RenderTargetProperties(),
                    D2D1::HwndRenderTargetProperties(window_, size), &renderTarget_))) return false;
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
    std::optional<std::size_t> selected_;
    fcitx::windows::config::Config visualConfig_;
    fcitx::windows::candidate::CandidateModel model_;
    fcitx::windows::protocol::CaretRect lastCaret_;
    fcitx::windows::ui::Placement placement_{fcitx::windows::ui::Placement::unlocked};
    std::uint64_t compositionId_{};
    bool safeMode_{};
};

bool readExact(HANDLE pipe, void* destination, std::size_t size) {
    auto* bytes = static_cast<std::uint8_t*>(destination);
    std::size_t offset = 0;
    while (offset < size) {
        DWORD read = 0;
        if (!ReadFile(pipe, bytes + offset, static_cast<DWORD>(size - offset), &read, nullptr) ||
            read == 0) return false;
        offset += read;
    }
    return true;
}

void servePresentation(HWND window, bool testOnce) {
    using namespace fcitx::windows;
    platform::RuntimeIdentity identity;
    platform::PipeSecurity security;
    if (!platform::queryCurrentIdentity(identity) ||
        !platform::PipeSecurity::create(identity, security)) return;
    std::wstring executable(32'768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, executable.data(),
                                          static_cast<DWORD>(executable.size()));
    if (size == 0 || size >= executable.size()) return;
    executable.resize(size);
    const auto engine =
        (std::filesystem::path(executable).parent_path() / "fcitx5-engine.exe").wstring();
    const auto pipeName = platform::makeLocalEndpointName(identity, L"presentation");
    for (;;) {
        HANDLE pipe = CreateNamedPipeW(
            pipeName.c_str(), PIPE_ACCESS_INBOUND,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1, static_cast<DWORD>(protocol::kMaxHotFrameSize),
            static_cast<DWORD>(protocol::kMaxHotFrameSize), 25, security.attributes());
        if (pipe == INVALID_HANDLE_VALUE) return;
        const bool connected = ConnectNamedPipe(pipe, nullptr) != FALSE ||
                               GetLastError() == ERROR_PIPE_CONNECTED;
        platform::ProcessIdentity peer;
        if (connected && ipc::verifyPipeClient(pipe, identity, &peer) &&
            platform::pathsReferToSameFile(peer.executablePath, engine)) {
            for (;;) {
                std::array<std::uint8_t, protocol::kHeaderSize> header{};
                if (!readExact(pipe, header.data(), header.size())) break;
                protocol::MessageType type{};
                std::uint32_t bodySize = 0;
                protocol::Metadata metadata;
                if (!protocol::decodeHeader(header, type, bodySize, metadata) ||
                    type != protocol::MessageType::keyResponse) break;
                std::vector<std::uint8_t> frame(header.begin(), header.end());
                frame.resize(protocol::kHeaderSize + bodySize);
                if (bodySize && !readExact(pipe, frame.data() + protocol::kHeaderSize, bodySize))
                    break;
                protocol::FrameView view;
                auto response = std::make_unique<protocol::KeyResponse>();
                if (!protocol::decodeFrame(frame, view) || !protocol::decode(view, *response)) break;
                if (!PostMessageW(window, kSnapshotMessage, 0,
                                  reinterpret_cast<LPARAM>(response.get()))) return;
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

int WINAPI wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE,
                    _In_ PWSTR commandLine, _In_ int) {
    enableDpiAwareness();
    const std::wstring_view arguments = commandLine ? commandLine : L"";
    const bool selfTest = arguments.find(L"--self-test") != std::wstring_view::npos;
    const bool simulateDeviceLoss =
        arguments.find(L"--simulate-device-loss") != std::wstring_view::npos;
    const bool demo = arguments.find(L"--demo") != std::wstring_view::npos;
    const bool testOnce = arguments.find(L"--test-once") != std::wstring_view::npos;
    const bool safeMode = arguments.find(L"--safe-mode") != std::wstring_view::npos;
    CandidateWindow window;
    if (!window.create(instance, demo, safeMode) || !window.paintOnce()) return 1;
    if (demo) window.showSyntheticPreview();
    if (simulateDeviceLoss) {
        window.simulateDeviceLossForTest();
        if (!window.paintOnce()) return 1;
    }
    if (selfTest) return 0;
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
