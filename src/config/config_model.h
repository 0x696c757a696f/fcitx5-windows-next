#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

namespace fcitx::windows::config {

enum class AppearanceMode { system, light, dark };
enum class Orientation { vertical, horizontal };
enum class LabelStyle { plain, dot, paren, bracket, circled };

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
};

struct Font {
    std::optional<std::vector<std::string>> families;
    std::optional<double> size;
    std::optional<std::int64_t> weight;
    std::optional<double> scale;
};

struct Config {
    std::optional<AppearanceMode> appearanceMode;
    std::optional<std::string> theme;
    std::optional<Orientation> orientation;
    std::optional<bool> scrollMode;
    std::optional<double> maxWidth;
    std::optional<double> scrollCellWidth;
    std::optional<double> opacity;
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

struct Theme {
    std::string id;
    std::string name;
    std::string version;
    std::string license;
    std::string description;
    Config common;
    Config light;
    Config dark;
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
                                          std::string_view scrollCellWidthDip = {}) noexcept;
[[nodiscard]] bool parseTheme(std::string_view text, Theme& output, ParseError& error) noexcept;
[[nodiscard]] Config mergeConfig(const Config& base, const Config& overrideConfig);
[[nodiscard]] Config resolveTheme(const Theme& theme, bool dark, const Config& userOverride);

} // namespace fcitx::windows::config
