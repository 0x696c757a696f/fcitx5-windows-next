#pragma once

#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

namespace fcitx::windows::config {

enum class AppearanceMode { system, light, dark };
enum class Orientation { automatic, vertical, horizontal };
enum class LabelStyle { plain, dot, paren, bracket, circled };
enum class PreeditMode { inline_, panel };
enum class UiLanguage { system, enUS, zhCN };

struct Geometry {
    std::optional<double> paddingX;
    std::optional<double> paddingY;
    std::optional<double> itemPaddingX;
    std::optional<double> itemPaddingY;
    std::optional<double> rowGap;
    std::optional<double> columnGap;
    std::optional<double> borderWidth;
    std::optional<double> cornerRadius;
    std::optional<bool> shadow;
};

struct Label {
    std::optional<bool> visible;
    std::optional<LabelStyle> style;
    std::optional<double> fontScale;
    std::optional<double> gap;
    std::optional<std::vector<std::string>> sequence;
};

struct Font {
    std::optional<std::vector<std::string>> families;
    std::optional<double> size;
    std::optional<std::int64_t> weight;
    std::optional<double> scale;
};

struct Config {
    std::optional<UiLanguage> uiLanguage;
    std::optional<AppearanceMode> appearanceMode;
    std::optional<std::string> theme;
    std::optional<Orientation> orientation;
    std::optional<bool> scrollMode;
    std::optional<double> maxWidth;
    std::optional<double> scrollCellWidth;
    std::optional<double> opacity;
    std::optional<PreeditMode> preeditMode;
    std::optional<int> candidatePageSize;
    std::vector<std::string> enabledInputMethods;
    std::optional<std::string> defaultInputMethod;
    std::optional<std::string> hotkeyToggleInputMethod;
    std::optional<std::string> hotkeyNextInputMethod;
    Geometry geometry;
    Label label;
    Font uiFont;
    Font candidateFont;
    Font annotationFont;
    Font monospaceFont;
    std::unordered_map<std::string, std::string> colors;
};

struct ParseError {
    std::string message;
    std::size_t line{};
    std::size_t column{};
};

[[nodiscard]] bool parseConfig(std::string_view text, Config& output, ParseError& error) noexcept;
[[nodiscard]] std::string defaultConfigToml();
[[nodiscard]] bool updatePresentationToml(std::string_view source, std::string_view appearanceMode,
                                          std::string_view theme, std::string_view orientation,
                                          std::string_view scrollMode,
                                          std::string_view candidatePageSize,
                                          std::string_view candidateFont, std::string& output,
                                          ParseError& error,
                                          std::string_view maxWidthDip = {},
                                          std::string_view scrollCellWidthDip = {},
                                          std::string_view candidateFontSizeDip = {},
                                          std::string_view cornerRadiusDip = {},
                                          std::string_view shadow = {},
                                          std::string_view opacity = {},
                                          std::string_view preeditMode = {}) noexcept;
[[nodiscard]] bool resetPresentationToml(std::string_view source, std::string& output,
                                         ParseError& error) noexcept;
[[nodiscard]] bool resolveThemeConfig(std::string_view themeText, std::string_view requestedId,
                                      bool builtin, bool dark, Config& output,
                                      ParseError& error) noexcept;
[[nodiscard]] std::filesystem::path resolveThemePath(
    const std::filesystem::path& installationRoot, const std::filesystem::path& dataRoot,
    std::string_view requestedId, bool builtin) noexcept;
[[nodiscard]] std::optional<std::string> readBoundedFile(
    const std::filesystem::path& path, std::size_t maximum) noexcept;
[[nodiscard]] Config mergeConfig(const Config& base, const Config& overrideConfig);

} // namespace fcitx::windows::config
