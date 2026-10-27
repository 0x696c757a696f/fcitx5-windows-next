//! Resolves theme colors into paint colors, honoring high contrast and selection legibility.

#![forbid(unsafe_code)]

/// Host-supplied config colors in 0..1 float RGBA (081C theme slice).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateConfigColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Resolved 8-bit render theme colors for one paint.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateResolvedColors {
    pub background: [u8; 3],
    pub text: [u8; 3],
    pub selected_background: [u8; 3],
    pub selected_text: [u8; 3],
    pub comment: [u8; 3],
    pub border: [u8; 3],
    pub preedit_text: [u8; 3],
}

pub(crate) const COLOR_WINDOW_COLORREF_BGR: [u8; 3] = [255, 255, 255];

pub(crate) const COLOR_WINDOWTEXT_COLORREF_BGR: [u8; 3] = [0, 0, 0];

pub(crate) const COLOR_HIGHLIGHT_COLORREF_BGR: [u8; 3] = [215, 120, 0];

pub(crate) const COLOR_HIGHLIGHTTEXT_COLORREF_BGR: [u8; 3] = [255, 255, 255];

pub(crate) fn clamp_color_byte(
    value: f32,
    high_contrast: bool,
    system: [u8; 3],
    channel: usize,
) -> u8 {
    if high_contrast {
        return system[channel];
    }
    ((value.clamp(0.0, 1.0)) * 255.0).round() as u8
}

/// Resolves one config color into the C++ channel order (R, G, B).
///
/// `system` must already be ordered to match the extraction indices used by
/// the C++ `to_u8` lambda: channel 2 = R, channel 1 = G, channel 0 = B, i.e.
/// the COLORREF little-endian byte layout (B, G, R).
pub(crate) fn resolve_rgb(
    color: Fcitx5CandidateConfigColor,
    high_contrast: bool,
    colorref_bgr: [u8; 3],
) -> [u8; 3] {
    // COLORREF layout: 0x00BBGGRR → byte[0]=B, byte[1]=G, byte[2]=R.
    // to_u8 channel 2 selects R, 1 selects G, 0 selects B.
    [
        clamp_color_byte(color.r, high_contrast, colorref_bgr, 2),
        clamp_color_byte(color.g, high_contrast, colorref_bgr, 1),
        clamp_color_byte(color.b, high_contrast, colorref_bgr, 0),
    ]
}

pub(crate) fn srgb_luminance(color: [u8; 3]) -> f32 {
    let channel = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
}

pub(crate) fn contrast_ratio(first: [u8; 3], second: [u8; 3]) -> f32 {
    let first = srgb_luminance(first);
    let second = srgb_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

pub(crate) fn readable_selected_text(
    selected_text: [u8; 3],
    selected_background: [u8; 3],
) -> [u8; 3] {
    // A persisted/custom theme may carry the old green-on-green pair even
    // when the built-in theme is correct. Keep the user's background, but
    // fail closed to whichever neutral text color has stronger contrast when
    // the requested foreground is not readable.
    // Candidate text is rendered at a large UI size; preserve the product's
    // white-on-WeChat-green appearance once it clears the large-text floor.
    // The shipped 27px candidate text is large UI text. Keep the white
    // WeChat-green treatment for the stock accent (#07c160), while still
    // rejecting genuinely invisible custom foregrounds.
    const MIN_SELECTED_TEXT_CONTRAST: f32 = 2.0;
    if contrast_ratio(selected_text, selected_background) >= MIN_SELECTED_TEXT_CONTRAST {
        return selected_text;
    }
    let white = [255, 255, 255];
    let black = [0, 0, 0];
    if contrast_ratio(white, selected_background) >= MIN_SELECTED_TEXT_CONTRAST {
        white
    } else {
        black
    }
}

/// Resolves the eight paint colors from config colors and the high-contrast
/// policy, mirroring the frozen shipping `paintOnceToDC` color mapping.
#[must_use]
pub fn resolve_paint_colors(
    background: Fcitx5CandidateConfigColor,
    candidate_text: Fcitx5CandidateConfigColor,
    selected_background: Fcitx5CandidateConfigColor,
    selected_text: Fcitx5CandidateConfigColor,
    comment_text: Fcitx5CandidateConfigColor,
    border: Fcitx5CandidateConfigColor,
    preedit_text: Fcitx5CandidateConfigColor,
    high_contrast: bool,
) -> Fcitx5CandidateResolvedColors {
    let hc = high_contrast;
    let selected_background = resolve_rgb(selected_background, hc, COLOR_HIGHLIGHT_COLORREF_BGR);
    let selected_text = readable_selected_text(
        resolve_rgb(selected_text, hc, COLOR_HIGHLIGHTTEXT_COLORREF_BGR),
        selected_background,
    );
    Fcitx5CandidateResolvedColors {
        background: resolve_rgb(background, hc, COLOR_WINDOW_COLORREF_BGR),
        text: resolve_rgb(candidate_text, hc, COLOR_WINDOWTEXT_COLORREF_BGR),
        selected_background,
        selected_text,
        comment: resolve_rgb(comment_text, hc, COLOR_WINDOWTEXT_COLORREF_BGR),
        border: resolve_rgb(border, hc, COLOR_WINDOWTEXT_COLORREF_BGR),
        preedit_text: resolve_rgb(preedit_text, hc, COLOR_WINDOWTEXT_COLORREF_BGR),
    }
}

/// Resolved label and selected-item secondary colors for one paint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fcitx5CandidateSecondaryColors {
    pub label: [u8; 3],
    pub selected_label: [u8; 3],
    pub selected_comment: [u8; 3],
}

/// Resolves the label color and the label/comment colors of the selected
/// item. Selected colors are kept readable on `selected_background`, which
/// must already be resolved by [`resolve_paint_colors`].
#[must_use]
pub fn resolve_secondary_colors(
    label_text: Fcitx5CandidateConfigColor,
    selected_label_text: Fcitx5CandidateConfigColor,
    selected_comment_text: Fcitx5CandidateConfigColor,
    selected_background: [u8; 3],
    high_contrast: bool,
) -> Fcitx5CandidateSecondaryColors {
    let selected = |color| {
        readable_selected_text(
            resolve_rgb(color, high_contrast, COLOR_HIGHLIGHTTEXT_COLORREF_BGR),
            selected_background,
        )
    };
    Fcitx5CandidateSecondaryColors {
        label: resolve_rgb(label_text, high_contrast, COLOR_WINDOWTEXT_COLORREF_BGR),
        selected_label: selected(selected_label_text),
        selected_comment: selected(selected_comment_text),
    }
}

#[cfg(test)]
mod candidate_paint_color_tests {
    use super::*;

    fn color(r: f32, g: f32, b: f32) -> Fcitx5CandidateConfigColor {
        Fcitx5CandidateConfigColor { r, g, b, a: 1.0 }
    }

    #[test]
    fn paint_colors_clamp_and_scale_config_colors() {
        let resolved = resolve_paint_colors(
            color(0.97, 0.98, 0.98),
            color(0.13, 0.13, 0.14),
            color(0.027, 0.757, 0.376),
            color(0.027, 0.757, 0.376),
            color(0.13, 0.13, 0.14),
            color(0.82, 0.82, 0.82),
            color(0.13, 0.13, 0.14),
            false,
        );
        assert_eq!(resolved.background, [247, 250, 250]);
        assert_eq!(resolved.selected_background, [7, 193, 96]);
        assert_eq!(resolved.selected_text, [255, 255, 255]);
        assert_eq!(resolved.border, [209, 209, 209]);
    }

    #[test]
    fn selected_text_fails_closed_when_a_theme_requests_green_on_green() {
        let resolved = resolve_paint_colors(
            color(1.0, 1.0, 1.0),
            color(0.1, 0.1, 0.1),
            color(0.027, 0.757, 0.376),
            color(0.027, 0.757, 0.376),
            color(0.1, 0.1, 0.1),
            color(0.8, 0.8, 0.8),
            color(0.1, 0.1, 0.1),
            false,
        );
        assert_eq!(resolved.selected_text, [255, 255, 255]);
        assert!(contrast_ratio(resolved.selected_text, resolved.selected_background) >= 2.0);
    }

    #[test]
    fn high_contrast_maps_colors_to_system_palette() {
        let resolved = resolve_paint_colors(
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            true,
        );
        assert_eq!(resolved.background, COLOR_WINDOW_COLORREF_BGR);
        assert_eq!(resolved.text, COLOR_WINDOWTEXT_COLORREF_BGR);
        assert_eq!(
            resolved.selected_background,
            [0, 120, 215],
            "COLORREF [B,G,R] extracts to RGB"
        );
        assert_eq!(resolved.selected_text, COLOR_HIGHLIGHTTEXT_COLORREF_BGR);
    }

    #[test]
    fn selected_secondary_colors_stay_readable_on_the_selection() {
        let resolved = resolve_secondary_colors(
            color(0.5, 0.5, 0.5),
            color(1.0, 1.0, 1.0),
            color(0.027, 0.757, 0.376),
            [7, 193, 96],
            false,
        );
        assert_eq!(resolved.label, [128, 128, 128]);
        assert_eq!(resolved.selected_label, [255, 255, 255]);
        assert_eq!(resolved.selected_comment, [255, 255, 255]);
    }

    #[test]
    fn high_contrast_maps_secondary_colors_to_system_palette() {
        let resolved = resolve_secondary_colors(
            color(0.5, 0.5, 0.5),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            [0, 120, 215],
            true,
        );
        assert_eq!(resolved.label, COLOR_WINDOWTEXT_COLORREF_BGR);
        assert_eq!(resolved.selected_label, COLOR_HIGHLIGHTTEXT_COLORREF_BGR);
        assert_eq!(resolved.selected_comment, COLOR_HIGHLIGHTTEXT_COLORREF_BGR);
    }
}
