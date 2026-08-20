#include "config_model.h"

#include <toml++/toml.hpp>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <set>
#include <sstream>
#include <utility>

namespace fcitx::windows::config {
namespace {

constexpr std::size_t kMaxConfigBytes = 256U * 1024U;

bool setError(ParseError& error, std::string message, const toml::source_region* source = nullptr) {
    error.message = std::move(message);
    if (source) {
        error.line = source->begin.line;
        error.column = source->begin.column;
    }
    return false;
}

bool allowed(const toml::table& table, std::initializer_list<std::string_view> keys,
             std::string_view path, ParseError& error) {
    for (const auto& [key, value] : table) {
        if (std::find(keys.begin(), keys.end(), key.str()) == keys.end()) {
            return setError(error, "unknown key: " + std::string(path) + std::string(key.str()),
                            &value.source());
        }
    }
    return true;
}

template <typename T>
bool optionalValue(const toml::table& table, std::string_view key, std::optional<T>& output,
                   ParseError& error) {
    const auto* node = table.get(key);
    if (!node)
        return true;
    auto value = node->value<T>();
    if (!value)
        return setError(error, "wrong type for " + std::string(key), &node->source());
    output = std::move(*value);
    return true;
}

bool ranged(const std::optional<double>& value, double minimum, double maximum,
            std::string_view name, ParseError& error) {
    if (!value)
        return true;
    return std::isfinite(*value) && *value >= minimum && *value <= maximum
               ? true
               : setError(error, std::string(name) + " is outside its allowed range");
}

bool parseFont(const toml::table& parent, std::string_view key, Font& output, bool allowSize,
               bool allowWeight, bool allowScale, ParseError& error) {
    const auto* table = parent[key].as_table();
    if (!table)
        return !parent.contains(key) || setError(error, "font entry must be a table");
    std::vector<std::string_view> permitted{"families"};
    if (allowSize)
        permitted.emplace_back("size_dip");
    if (allowWeight)
        permitted.emplace_back("weight");
    if (allowScale)
        permitted.emplace_back("scale");
    for (const auto& [name, value] : *table) {
        if (std::find(permitted.begin(), permitted.end(), name.str()) == permitted.end()) {
            return setError(error, "unknown font key: " + std::string(name.str()), &value.source());
        }
    }
    if (const auto* node = table->get("families")) {
        const auto* array = node->as_array();
        if (!array || array->empty() || array->size() > 8) {
            return setError(error, "font families must contain 1 to 8 strings", &node->source());
        }
        std::vector<std::string> families;
        for (const auto& item : *array) {
            auto family = item.value<std::string>();
            if (!family || family->empty() || family->size() > 128) {
                return setError(error, "invalid font family", &item.source());
            }
            families.emplace_back(std::move(*family));
        }
        output.families = std::move(families);
    }
    if (!optionalValue(*table, "size_dip", output.size, error) ||
        !optionalValue(*table, "weight", output.weight, error) ||
        !optionalValue(*table, "scale", output.scale, error))
        return false;
    if (!ranged(output.size, 8, 72, "font size", error) ||
        !ranged(output.scale, 0.5, 1.5, "font scale", error))
        return false;
    if (output.weight &&
        (*output.weight < 100 || *output.weight > 900 || *output.weight % 100 != 0)) {
        return setError(error, "font weight must be 100..900 in steps of 100");
    }
    return true;
}

bool validColor(std::string_view value) {
    if (value.size() != 7 && value.size() != 9)
        return false;
    if (value.front() != '#')
        return false;
    return std::all_of(value.begin() + 1, value.end(), [](char character) {
        return (character >= '0' && character <= '9') || (character >= 'a' && character <= 'f') ||
               (character >= 'A' && character <= 'F');
    });
}

bool validThemeReference(std::string_view value) {
    if (value == "builtin:default")
        return true;
    if (value.empty() || value.size() > 64)
        return false;
    return std::all_of(value.begin(), value.end(),
                       [](unsigned char character) {
                           return (character >= 'a' && character <= 'z') ||
                                  (character >= '0' && character <= '9') || character == '.' ||
                                  character == '_' || character == '-';
                       }) &&
           ((value.front() >= 'a' && value.front() <= 'z') ||
            (value.front() >= '0' && value.front() <= '9'));
}

template <typename T>
void mergeOptional(std::optional<T>& destination, const std::optional<T>& source) {
    if (source)
        destination = source;
}

void mergeFont(Font& destination, const Font& source) {
    mergeOptional(destination.families, source.families);
    mergeOptional(destination.size, source.size);
    mergeOptional(destination.weight, source.weight);
    mergeOptional(destination.scale, source.scale);
}

} // namespace

bool parseConfig(std::string_view text, Config& output, ParseError& error) noexcept {
    output = {};
    error = {};
    if (text.size() > kMaxConfigBytes)
        return setError(error, "config.toml exceeds 256 KiB");
    try {
        const toml::table root = toml::parse(text);
        if (!allowed(root, {"format_version", "appearance", "candidate", "input_methods",
                            "hotkeys", "fonts"},
                     "", error))
            return false;
        const auto version = root["format_version"].value<std::int64_t>();
        if (!version || *version != 1)
            return setError(error, "format_version must be exactly 1");

        if (const auto* appearance = root["appearance"].as_table()) {
            if (!allowed(*appearance, {"mode", "theme"}, "appearance.", error))
                return false;
            std::optional<std::string> mode;
            if (!optionalValue(*appearance, "mode", mode, error) ||
                !optionalValue(*appearance, "theme", output.theme, error))
                return false;
            if (output.theme && !validThemeReference(*output.theme)) {
                return setError(error,
                                "appearance.theme must be builtin:default or a safe package id");
            }
            if (mode) {
                if (*mode == "system")
                    output.appearanceMode = AppearanceMode::system;
                else if (*mode == "light")
                    output.appearanceMode = AppearanceMode::light;
                else if (*mode == "dark")
                    output.appearanceMode = AppearanceMode::dark;
                else
                    return setError(error, "appearance.mode must be system, light, or dark");
            }
        } else if (root.contains("appearance")) {
            return setError(error, "appearance must be a table");
        }

        if (const auto* candidate = root["candidate"].as_table()) {
            if (!allowed(*candidate,
                         {"orientation", "page_size", "scroll_mode", "max_width_dip",
                          "scroll_cell_width_dip", "opacity", "geometry", "label", "colors"},
                         "candidate.", error))
                return false;
            std::optional<std::string> orientation;
            if (!optionalValue(*candidate, "orientation", orientation, error) ||
                !optionalValue(*candidate, "page_size", output.candidatePageSize, error) ||
                !optionalValue(*candidate, "max_width_dip", output.maxWidth, error) ||
                !optionalValue(*candidate, "scroll_cell_width_dip", output.scrollCellWidth,
                               error) ||
                !optionalValue(*candidate, "scroll_mode", output.scrollMode, error) ||
                !optionalValue(*candidate, "opacity", output.opacity, error))
                return false;
            if (orientation) {
                if (*orientation == "vertical")
                    output.orientation = Orientation::vertical;
                else if (*orientation == "horizontal")
                    output.orientation = Orientation::horizontal;
                else
                    return setError(error, "candidate.orientation must be vertical or horizontal");
            }
            if (!ranged(output.candidatePageSize, 1, 9, "candidate.page_size", error) ||
                !ranged(output.maxWidth, 160, 2048, "candidate.max_width_dip", error) ||
                !ranged(output.scrollCellWidth, 40, 160, "candidate.scroll_cell_width_dip",
                        error) ||
                !ranged(output.opacity, 0.2, 1.0, "candidate.opacity", error))
                return false;

            if (const auto* geometry = (*candidate)["geometry"].as_table()) {
                if (!allowed(*geometry,
                             {"padding_x_dip", "padding_y_dip", "item_padding_x_dip",
                              "item_padding_y_dip", "row_gap_dip", "column_gap_dip",
                              "border_width_dip", "corner_radius_dip", "shadow"},
                             "candidate.geometry.", error))
                    return false;
                if (!optionalValue(*geometry, "padding_x_dip", output.geometry.paddingX, error) ||
                    !optionalValue(*geometry, "padding_y_dip", output.geometry.paddingY, error) ||
                    !optionalValue(*geometry, "item_padding_x_dip", output.geometry.itemPaddingX,
                                   error) ||
                    !optionalValue(*geometry, "item_padding_y_dip", output.geometry.itemPaddingY,
                                   error) ||
                    !optionalValue(*geometry, "row_gap_dip", output.geometry.rowGap, error) ||
                    !optionalValue(*geometry, "column_gap_dip", output.geometry.columnGap, error) ||
                    !optionalValue(*geometry, "border_width_dip", output.geometry.borderWidth,
                                   error) ||
                    !optionalValue(*geometry, "corner_radius_dip", output.geometry.cornerRadius,
                                   error) ||
                    !optionalValue(*geometry, "shadow", output.geometry.shadow, error))
                    return false;
                for (const auto* entry :
                     {&output.geometry.paddingX, &output.geometry.paddingY,
                      &output.geometry.itemPaddingX, &output.geometry.itemPaddingY,
                      &output.geometry.rowGap, &output.geometry.columnGap}) {
                    if (!ranged(*entry, 0, 64, "geometry spacing", error))
                        return false;
                }
                if (!ranged(output.geometry.borderWidth, 0, 8, "border width", error) ||
                    !ranged(output.geometry.cornerRadius, 0, 64, "corner radius", error))
                    return false;
            } else if (candidate->contains("geometry"))
                return setError(error, "geometry must be a table");

            if (const auto* label = (*candidate)["label"].as_table()) {
                if (!allowed(*label, {"visible", "style", "font_scale", "gap_dip"},
                             "candidate.label.", error))
                    return false;
                std::optional<std::string> style;
                if (!optionalValue(*label, "visible", output.label.visible, error) ||
                    !optionalValue(*label, "style", style, error) ||
                    !optionalValue(*label, "font_scale", output.label.fontScale, error) ||
                    !optionalValue(*label, "gap_dip", output.label.gap, error))
                    return false;
                if (style) {
                    if (*style == "plain")
                        output.label.style = LabelStyle::plain;
                    else if (*style == "dot")
                        output.label.style = LabelStyle::dot;
                    else if (*style == "paren")
                        output.label.style = LabelStyle::paren;
                    else if (*style == "bracket")
                        output.label.style = LabelStyle::bracket;
                    else if (*style == "circled")
                        output.label.style = LabelStyle::circled;
                    else
                        return setError(error, "invalid candidate label style");
                }
                if (!ranged(output.label.fontScale, 0.5, 1.5, "label font scale", error) ||
                    !ranged(output.label.gap, 0, 64, "label gap", error))
                    return false;
            } else if (candidate->contains("label"))
                return setError(error, "label must be a table");

            if (const auto* colors = (*candidate)["colors"].as_table()) {
                static constexpr std::array colorNames{"background",
                                                       "border",
                                                       "preedit_text",
                                                       "label_text",
                                                       "candidate_text",
                                                       "comment_text",
                                                       "selected_background",
                                                       "selected_label_text",
                                                       "selected_candidate_text",
                                                       "selected_comment_text",
                                                       "shadow"};
                for (const auto& [name, node] : *colors) {
                    if (std::find(colorNames.begin(), colorNames.end(), name.str()) ==
                        colorNames.end())
                        return setError(error, "unknown semantic color: " + std::string(name.str()),
                                        &node.source());
                    const auto color = node.value<std::string>();
                    if (!color || !validColor(*color))
                        return setError(error, "color must be #RRGGBB or #RRGGBBAA",
                                        &node.source());
                    output.colors.emplace(name.str(), *color);
                }
            } else if (candidate->contains("colors"))
                return setError(error, "colors must be a table");
        } else if (root.contains("candidate"))
            return setError(error, "candidate must be a table");

        if (const auto* inputMethods = root["input_methods"].as_table()) {
            if (!allowed(*inputMethods, {"enabled", "default"}, "input_methods.", error))
                return false;
            if (const auto* enabled = (*inputMethods)["enabled"].as_array()) {
                for (const auto& item : *enabled) {
                    const auto value = item.value<std::string>();
                    if (!value || value->empty() || value->find(' ') != std::string::npos)
                        return setError(error, "input_methods.enabled must be non-empty ids",
                                        &item.source());
                    if (std::find(output.enabledInputMethods.begin(),
                                  output.enabledInputMethods.end(),
                                  *value) == output.enabledInputMethods.end())
                        output.enabledInputMethods.push_back(*value);
                }
                if (output.enabledInputMethods.empty())
                    return setError(error, "input_methods.enabled must not be empty");
            }
            if (!optionalValue(*inputMethods, "default", output.defaultInputMethod, error))
                return false;
            if (output.defaultInputMethod && !output.enabledInputMethods.empty() &&
                std::find(output.enabledInputMethods.begin(), output.enabledInputMethods.end(),
                          *output.defaultInputMethod) == output.enabledInputMethods.end())
                return setError(error, "input_methods.default must be in enabled");
        } else if (root.contains("input_methods"))
            return setError(error, "input_methods must be a table");

        if (const auto* hotkeys = root["hotkeys"].as_table()) {
            if (!allowed(*hotkeys, {"toggle_input_method", "next_input_method"}, "hotkeys.",
                         error))
                return false;
            if (!optionalValue(*hotkeys, "toggle_input_method", output.hotkeyToggleInputMethod,
                               error) ||
                !optionalValue(*hotkeys, "next_input_method", output.hotkeyNextInputMethod,
                               error))
                return false;
        } else if (root.contains("hotkeys"))
            return setError(error, "hotkeys must be a table");

        if (const auto* fonts = root["fonts"].as_table()) {
            if (!allowed(*fonts, {"ui", "candidate", "annotation", "monospace"}, "fonts.", error))
                return false;
            if (!parseFont(*fonts, "ui", output.uiFont, false, false, false, error) ||
                !parseFont(*fonts, "candidate", output.candidateFont, true, true, false, error) ||
                !parseFont(*fonts, "annotation", output.annotationFont, false, false, true,
                           error) ||
                !parseFont(*fonts, "monospace", output.monospaceFont, false, false, false, error))
                return false;
        } else if (root.contains("fonts"))
            return setError(error, "fonts must be a table");
        return true;
    } catch (const toml::parse_error& exception) {
        error.message = exception.description();
        error.line = exception.source().begin.line;
        error.column = exception.source().begin.column;
        return false;
    } catch (...) {
        return setError(error, "unexpected configuration error");
    }
}

std::string defaultConfigToml() {
    return R"(# Fcitx5 for Windows 用户配置。保存为 UTF-8（无 BOM）和 LF。
# 未写出的字段会继承当前主题；Reset 会删除 override，而不是复制默认值。
format_version = 1

[appearance]
# system 跟随 Windows；也可选 light 或 dark。此设置实时生效。
mode = "system"
# builtin:default 不执行脚本，也不读取网络资源。
theme = "builtin:default"

[candidate]
# vertical（纵向，默认）或 horizontal（横向）。
orientation = "vertical"
# 横向时表示每行候选数；纵向时表示每列候选数。范围 1–9。
page_size = 5
# 卷轴模式：引擎提供 BulkCandidateList 时，方向键/PageUp/PageDown/Home/End
# 按当前布局方向连续滚动候选（对齐 fcitx5-macos 的 ScrollConfig）。
scroll_mode = true
# 候选窗最大宽度，单位 DIP，范围 160–2048。
max_width_dip = 860.0
# 卷轴模式单个候选格子的目标宽度，单位 DIP，范围 40–160；长候选会省略。
scroll_cell_width_dip = 96.0
# 整体不透明度，范围 0.20–1.00。
opacity = 1.0

[input_methods]
# 启用的输入法（有序，顺序即切换顺序）。可用的 id 见 --get-input-methods。
enabled = ["pinyin", "rime", "wbx"]
# 默认（激活）输入法。
default = "pinyin"

[hotkeys]
# 中英文切换：在当前输入法与 inactive（直接英文）之间切换。
toggle_input_method = "Ctrl+Space"
# 下一个输入法（按 enabled 顺序循环）。
next_input_method = "Ctrl+Shift"

[candidate.geometry]
# 以下尺寸均为 DIP，会随显示器 DPI 自动缩放。
padding_x_dip = 10.0
padding_y_dip = 6.0
item_padding_x_dip = 8.0
item_padding_y_dip = 4.0
row_gap_dip = 1.0
column_gap_dip = 12.0
border_width_dip = 1.0
corner_radius_dip = 12.0
shadow = true

[candidate.label]
# 只改变序号外观，不改变真正的选词按键。
visible = true
style = "dot" # plain | dot | paren | bracket | circled
font_scale = 0.85
gap_dip = 4.0

[fonts.ui]
families = ["system"]

[fonts.candidate]
# 按顺序 fallback；最后仍使用 DirectWrite 系统 fallback 补字。
families = ["Microsoft YaHei", "system"]
size_dip = 18.0 # 范围 8–72 DIP
weight = 400    # 100–900，步进 100

[fonts.annotation]
families = ["inherit"]
scale = 0.80

[fonts.monospace]
families = ["Cascadia Mono", "Consolas", "system"]
)";
}

bool parseTheme(std::string_view text, Theme& output, ParseError& error) noexcept {
    output = {};
    error = {};
    if (text.size() > 512U * 1024U)
        return setError(error, "theme.toml exceeds 512 KiB");
    try {
        const toml::table root = toml::parse(text);
        if (!allowed(root, {"format_version", "theme", "common", "light", "dark"}, "", error))
            return false;
        const auto version = root["format_version"].value<std::int64_t>();
        if (!version || *version != 1)
            return setError(error, "format_version must be exactly 1");
        const auto* metadata = root["theme"].as_table();
        if (!metadata || !allowed(*metadata, {"id", "name", "version", "license", "description"},
                                  "theme.", error))
            return false;
        auto required = [&](std::string_view key, std::string& destination) {
            const auto value = (*metadata)[key].value<std::string>();
            if (!value || value->empty())
                return false;
            destination = *value;
            return true;
        };
        if (!required("id", output.id) || !required("name", output.name) ||
            !required("version", output.version) || !required("license", output.license)) {
            return setError(error, "theme metadata id/name/version/license are required");
        }
        if (const auto description = (*metadata)["description"].value<std::string>())
            output.description = *description;
        if (output.id.size() > 64 || output.id.empty() ||
            !std::all_of(output.id.begin(), output.id.end(),
                         [](unsigned char character) {
                             return (character >= 'a' && character <= 'z') ||
                                    (character >= '0' && character <= '9') || character == '.' ||
                                    character == '_' || character == '-';
                         }) ||
            (!(output.id.front() >= 'a' && output.id.front() <= 'z') &&
             !(output.id.front() >= '0' && output.id.front() <= '9'))) {
            return setError(error, "invalid theme id");
        }
        auto parseBranch = [&](std::string_view key, Config& destination, bool colorsOnly) {
            const auto* branch = root[key].as_table();
            if (!branch)
                return !root.contains(key) ||
                       setError(error, std::string(key) + " must be a table");
            if (!allowed(*branch,
                         colorsOnly ? std::initializer_list<std::string_view>{"candidate"}
                                    : std::initializer_list<std::string_view>{"candidate", "fonts"},
                         std::string(key) + ".", error))
                return false;
            if (colorsOnly) {
                const auto* candidate = (*branch)["candidate"].as_table();
                if (!candidate ||
                    !allowed(*candidate, {"colors"}, std::string(key) + ".candidate.", error))
                    return false;
            }
            std::ostringstream serialized;
            serialized << "format_version = 1\n" << toml::toml_formatter(*branch);
            ParseError branchError;
            if (!parseConfig(serialized.str(), destination, branchError)) {
                error = std::move(branchError);
                error.message = std::string(key) + ": " + error.message;
                return false;
            }
            return true;
        };
        return parseBranch("common", output.common, false) &&
               parseBranch("light", output.light, true) && parseBranch("dark", output.dark, true);
    } catch (const toml::parse_error& exception) {
        error.message = exception.description();
        error.line = exception.source().begin.line;
        error.column = exception.source().begin.column;
        return false;
    } catch (...) {
        return setError(error, "unexpected theme configuration error");
    }
}

bool updatePresentationToml(std::string_view source, std::string_view appearanceMode,
                            std::string_view theme, std::string_view orientation,
                            std::string_view scrollMode, std::string_view candidatePageSize,
                            std::string_view candidateFont, std::string& output,
                            ParseError& error, std::string_view maxWidthDip,
                            std::string_view scrollCellWidthDip) noexcept {
    output.clear();
    error = {};
    if ((appearanceMode != "system" && appearanceMode != "light" && appearanceMode != "dark") ||
        (orientation != "vertical" && orientation != "horizontal") ||
        (scrollMode != "enabled" && scrollMode != "disabled") || theme.empty() ||
        theme.size() > 128 || candidatePageSize.empty() || candidatePageSize.size() > 2U ||
        candidateFont.empty() || candidateFont.size() > 128) {
        return setError(error, "invalid presentation setting");
    }
    int pageSize = 0;
    for (const char digit : candidatePageSize) {
        if (digit < '0' || digit > '9')
            return setError(error, "invalid presentation setting");
        pageSize = pageSize * 10 + digit - '0';
    }
    if (pageSize < 1 || pageSize > 9)
        return setError(error, "invalid presentation setting");
    const auto parseDip = [&](std::string_view value, double minimum, double maximum,
                              double& parsed) {
        if (value.empty())
            return true;
        if (value.size() > 8U)
            return false;
        std::string text(value);
        std::size_t consumed = 0;
        try {
            parsed = std::stod(text, &consumed);
        } catch (...) {
            return false;
        }
        return consumed == text.size() && parsed >= minimum && parsed <= maximum;
    };
    double maxWidth = 0.0;
    double scrollCellWidth = 0.0;
    if (!parseDip(maxWidthDip, 160.0, 2048.0, maxWidth) ||
        !parseDip(scrollCellWidthDip, 40.0, 160.0, scrollCellWidth)) {
        return setError(error, "invalid presentation setting");
    }
    try {
        toml::table root = toml::parse(source.empty() ? defaultConfigToml() : source);
        if (!root["appearance"].as_table())
            root.insert_or_assign("appearance", toml::table{});
        auto& appearance = *root["appearance"].as_table();
        appearance.insert_or_assign("mode", appearanceMode);
        appearance.insert_or_assign("theme", theme);
        if (!root["candidate"].as_table())
            root.insert_or_assign("candidate", toml::table{});
        auto& candidate = *root["candidate"].as_table();
        candidate.insert_or_assign("orientation", orientation);
        candidate.insert_or_assign("page_size", pageSize);
        candidate.insert_or_assign("scroll_mode", scrollMode == "enabled");
        if (!maxWidthDip.empty())
            candidate.insert_or_assign("max_width_dip", maxWidth);
        if (!scrollCellWidthDip.empty())
            candidate.insert_or_assign("scroll_cell_width_dip", scrollCellWidth);
        if (!root["fonts"].as_table())
            root.insert_or_assign("fonts", toml::table{});
        auto& fonts = *root["fonts"].as_table();
        if (!fonts["candidate"].as_table())
            fonts.insert_or_assign("candidate", toml::table{});
        auto& candidateFonts = *fonts["candidate"].as_table();
        toml::array families;
        families.push_back(candidateFont);
        families.push_back("system");
        candidateFonts.insert_or_assign("families", std::move(families));

        std::ostringstream stream;
        stream << "# Fcitx5 for Windows 用户配置。UTF-8（无 BOM）、LF。\n"
                  "# 此文件由 typed Control API 写入；可手工编辑，但未知字段会被严格拒绝。\n"
                  "# appearance 控制明暗与主题；candidate 控制候选布局；fonts 控制字体 fallback。\n"
               << toml::toml_formatter(root) << '\n';
        Config validated;
        if (!parseConfig(stream.str(), validated, error))
            return false;
        output = stream.str();
        return true;
    } catch (const toml::parse_error& exception) {
        error.message = exception.description();
        error.line = exception.source().begin.line;
        error.column = exception.source().begin.column;
        return false;
    } catch (...) {
        return setError(error, "unexpected presentation update error");
    }
}

Config mergeConfig(const Config& base, const Config& overrideConfig) {
    Config result = base;
    mergeOptional(result.appearanceMode, overrideConfig.appearanceMode);
    mergeOptional(result.theme, overrideConfig.theme);
    mergeOptional(result.orientation, overrideConfig.orientation);
    mergeOptional(result.scrollMode, overrideConfig.scrollMode);
    mergeOptional(result.maxWidth, overrideConfig.maxWidth);
    mergeOptional(result.scrollCellWidth, overrideConfig.scrollCellWidth);
    mergeOptional(result.opacity, overrideConfig.opacity);
    mergeOptional(result.geometry.paddingX, overrideConfig.geometry.paddingX);
    mergeOptional(result.geometry.paddingY, overrideConfig.geometry.paddingY);
    mergeOptional(result.geometry.itemPaddingX, overrideConfig.geometry.itemPaddingX);
    mergeOptional(result.geometry.itemPaddingY, overrideConfig.geometry.itemPaddingY);
    mergeOptional(result.geometry.rowGap, overrideConfig.geometry.rowGap);
    mergeOptional(result.geometry.columnGap, overrideConfig.geometry.columnGap);
    mergeOptional(result.geometry.borderWidth, overrideConfig.geometry.borderWidth);
    mergeOptional(result.geometry.cornerRadius, overrideConfig.geometry.cornerRadius);
    mergeOptional(result.geometry.shadow, overrideConfig.geometry.shadow);
    mergeOptional(result.label.visible, overrideConfig.label.visible);
    mergeOptional(result.label.style, overrideConfig.label.style);
    mergeOptional(result.label.fontScale, overrideConfig.label.fontScale);
    mergeOptional(result.label.gap, overrideConfig.label.gap);
    mergeFont(result.uiFont, overrideConfig.uiFont);
    mergeFont(result.candidateFont, overrideConfig.candidateFont);
    mergeFont(result.annotationFont, overrideConfig.annotationFont);
    mergeFont(result.monospaceFont, overrideConfig.monospaceFont);
    for (const auto& [name, color] : overrideConfig.colors)
        result.colors[name] = color;
    return result;
}

Config resolveTheme(const Theme& theme, bool dark, const Config& userOverride) {
    return mergeConfig(mergeConfig(theme.common, dark ? theme.dark : theme.light), userOverride);
}

} // namespace fcitx::windows::config
