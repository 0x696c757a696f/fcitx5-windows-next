#pragma once

#include <cstddef>
#include <cstdint>

// Narrow C ABI for the Rust-owned resolved Config snapshot. Native consumers
// borrow all UTF-8 spans from the opaque handle and must destroy the handle
// only after they have copied the values they need.
extern "C" {

struct Fcitx5ConfigUtf8 {
    const std::uint8_t* ptr;
    std::size_t len;
};

struct Fcitx5ConfigUtf16 {
    const std::uint16_t* ptr;
    std::size_t len;
};

struct Fcitx5ConfigSnapshot {
    std::uint32_t recoverySource;
    std::uint32_t formatVersion;
    Fcitx5ConfigUtf8 uiLanguage;
    Fcitx5ConfigUtf8 appearanceMode;
    Fcitx5ConfigUtf8 appearanceTheme;
    // Legacy projection derived from `candidateLayoutType` until the renderer
    // cutover lands; kept at these offsets so pre-cutover consumers compile.
    Fcitx5ConfigUtf8 candidateOrientation;
    std::uint8_t candidatePageSize;
    std::uint8_t candidateScrollMode;
    float candidateMaxWidthDip;
    float candidateScrollCellWidthDip;
    float candidateOpacity;
    Fcitx5ConfigUtf8 candidatePreeditMode;
    float candidatePaddingXDip;
    float candidatePaddingYDip;
    float candidateItemPaddingXDip;
    float candidateItemPaddingYDip;
    float candidateRowGapDip;
    float candidateColumnGapDip;
    float candidateBorderWidthDip;
    float candidateCornerRadiusDip;
    std::uint8_t candidateShadow;
    std::uint8_t candidateLabelVisible;
    Fcitx5ConfigUtf8 candidateLabelStyle;
    float candidateLabelFontScale;
    float candidateLabelGapDip;
    std::size_t candidateLabelCount;
    std::size_t candidateColorCount;
    std::size_t uiFontFamilyCount;
    std::size_t candidateFontFamilyCount;
    float candidateFontSizeDip;
    std::uint16_t candidateFontWeight;
    std::size_t annotationFontFamilyCount;
    float annotationFontScale;
    std::size_t monospaceFontFamilyCount;
    std::size_t inputMethodCount;
    Fcitx5ConfigUtf8 defaultInputMethod;
    Fcitx5ConfigUtf8 hotkeyToggleInputMethod;
    Fcitx5ConfigUtf8 hotkeyNextInputMethod;
    // Unified candidate layout type
    // (`automatic`/`stacked`/`flow`/`scroll`/`vertical_text`).
    Fcitx5ConfigUtf8 candidateLayoutType;
};

[[nodiscard]] void* fcitx5_config_snapshot_load_current_utf16(Fcitx5ConfigUtf16 path);
[[nodiscard]] void* fcitx5_config_snapshot_load_visual_utf16(
    Fcitx5ConfigUtf16 currentPath, Fcitx5ConfigUtf16 installationRoot,
    Fcitx5ConfigUtf16 dataRoot, std::uint8_t safeMode, std::uint8_t systemDark);
void fcitx5_config_snapshot_destroy(void* handle);
[[nodiscard]] std::uint8_t fcitx5_config_snapshot_view(void* handle,
                                                       Fcitx5ConfigSnapshot* output);
[[nodiscard]] Fcitx5ConfigUtf8 fcitx5_config_snapshot_input_method_at(void* handle,
                                                                     std::size_t index);
[[nodiscard]] Fcitx5ConfigUtf8 fcitx5_config_snapshot_candidate_label_at(void* handle,
                                                                        std::size_t index);
[[nodiscard]] Fcitx5ConfigUtf8 fcitx5_config_snapshot_font_family_at(void* handle,
                                                                    std::uint32_t kind,
                                                                    std::size_t index);
[[nodiscard]] std::uint8_t fcitx5_config_snapshot_candidate_color_at(
    void* handle, std::size_t index, Fcitx5ConfigUtf8* name, Fcitx5ConfigUtf8* value);

} // extern "C"

inline constexpr std::uint32_t kFcitx5ConfigFontUi = 0;
inline constexpr std::uint32_t kFcitx5ConfigFontCandidate = 1;
inline constexpr std::uint32_t kFcitx5ConfigFontAnnotation = 2;
inline constexpr std::uint32_t kFcitx5ConfigFontMonospace = 3;
