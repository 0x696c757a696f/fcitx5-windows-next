#include "config_model.h"

#include <fstream>
#include <iostream>
#include <sstream>

int main() {
    using namespace fcitx::windows::config;
    Config config;
    ParseError error;
    const auto defaults = defaultConfigToml();
    if (!parseConfig(defaults, config, error) || config.orientation != Orientation::vertical ||
        config.appearanceMode != AppearanceMode::system || config.colors.size() != 11) {
        std::cerr << "annotated default config rejected: " << error.message << '\n';
        return 1;
    }
    constexpr std::string_view invalidCases[]{
        "format_version = 2\n",
        "format_version = 1\nunknown = true\n",
        "format_version = 1\n[candidate]\nopacity = 1.01\n",
        "format_version = 1\n[candidate]\npage_size = 7\n",
        "format_version = 1\n[candidate.colors]\nbackground = 'red'\n",
        "format_version = 1\n[fonts.candidate]\nfamilies = []\n",
        "format_version = 1\n[appearance]\ntheme = '../escape'\n",
        "format_version = 1\nformat_version = 1\n"};
    for (const auto invalid : invalidCases) {
        if (parseConfig(invalid, config, error)) {
            std::cerr << "invalid config accepted: " << invalid << '\n';
            return 1;
        }
    }
    if (!parseConfig("format_version = 1\n[candidate]\norientation = 'horizontal'\n", config,
                     error) ||
        config.orientation != Orientation::horizontal) {
        return 1;
    }
    std::ifstream themeFile("resources/themes/default/theme.toml", std::ios::binary);
    std::ostringstream themeText;
    themeText << themeFile.rdbuf();
    Theme theme;
    if (!themeFile || !parseTheme(themeText.str(), theme, error) || theme.id != "builtin.default" ||
        theme.light.colors.empty() || theme.dark.colors.empty()) {
        std::cerr << "annotated theme rejected: " << error.message << '\n';
        return 1;
    }
    Config userOverride;
    userOverride.orientation = Orientation::horizontal;
    userOverride.colors["candidate_text"] = "#112233FF";
    const auto darkResolved = resolveTheme(theme, true, userOverride);
    if (darkResolved.orientation != Orientation::horizontal ||
        darkResolved.colors.at("candidate_text") != "#112233FF" ||
        darkResolved.colors.at("background") != "#242629F7") {
        std::cerr << "theme/user merge order failed\n";
        return 1;
    }
    const auto resetResolved = resolveTheme(theme, true, Config{});
    if (resetResolved.orientation != Orientation::vertical ||
        resetResolved.colors.at("candidate_text") != "#FFFFFFFF") {
        std::cerr << "reset did not restore inherited theme value\n";
        return 1;
    }
    if (parseTheme("format_version=1\n[theme]\nid='Bad ID'\nname='x'\nversion='1'\nlicense='MIT'\n",
                   theme, error))
        return 1;
    std::string updated;
    if (!updatePresentationToml(defaults, "dark", "builtin:default", "horizontal", "enabled",
                                "Microsoft YaHei", updated, error) ||
        !updated.starts_with("# Fcitx5 for Windows") || !parseConfig(updated, config, error) ||
        config.appearanceMode != AppearanceMode::dark ||
        config.orientation != Orientation::horizontal || config.scrollMode != true ||
        config.maxWidth != 860.0 || config.colors.size() != 11 || !config.candidateFont.families ||
        config.candidateFont.families->front() != "Microsoft YaHei") {
        std::cerr << "typed presentation update failed: " << error.message << '\n';
        return 1;
    }
    if (updatePresentationToml(defaults, "invalid", "builtin:default", "vertical", "disabled",
                               "system", updated, error))
        return 1;
    return 0;
}
