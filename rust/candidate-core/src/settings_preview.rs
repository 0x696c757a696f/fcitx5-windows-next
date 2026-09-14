//! Settings preview adapter for the shipping candidate layout and renderer.
#![forbid(unsafe_code)]

use fcitx5_config_core::{
    CandidateOrientation as ConfigOrientation, ConfigSnapshot, OverflowBehavior as ConfigOverflow,
    WritingMode as ConfigWriting,
};

use crate::axis_layout::{self, CandidateLayoutOptions, OverflowBehavior, WritingMode};
use crate::renderer::{
    render_candidate_window, CandidateRenderData, MeasureEngine, RenderColor, RenderGeometry,
    RenderTheme, RenderWindowOutput,
};
use crate::{
    format_candidate_label, CandidateLabelStyle, Orientation, Placement, Point, Rect, Size,
};

/// Bounded logical dimensions of the embedded Settings preview.
pub const SETTINGS_PREVIEW_WIDTH_DIP: f32 = 560.0;
pub const SETTINGS_PREVIEW_HEIGHT_DIP: f32 = 176.0;

/// Rendered production preview plus the resolved accessibility marker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsCandidatePreview {
    /// BGRA bitmap from [`render_candidate_window`].
    pub bitmap: RenderWindowOutput,
    /// The caller requested the production high-contrast color path.
    pub high_contrast: bool,
}

#[derive(Clone, Copy)]
struct Rgba {
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
}

fn parse_rgba(value: &str) -> Option<Rgba> {
    let bytes = value.as_bytes();
    if (bytes.len() != 7 && bytes.len() != 9) || bytes.first() != Some(&b'#') {
        return None;
    }
    let nibble = |byte: u8| match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    };
    let byte = |index: usize| -> Option<f32> {
        let high = nibble(bytes[index])?;
        let low = nibble(bytes[index + 1])?;
        Some(f32::from(high * 16 + low) / 255.0)
    };
    Some(Rgba {
        red: byte(1)?,
        green: byte(3)?,
        blue: byte(5)?,
        alpha: if bytes.len() == 9 { byte(7)? } else { 1.0 },
    })
}

fn color(snapshot: &ConfigSnapshot, name: &str, fallback: Rgba) -> Rgba {
    snapshot
        .candidate()
        .colors()
        .get(name)
        .and_then(|value| parse_rgba(value))
        .unwrap_or(fallback)
}

fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn render_color(value: Rgba) -> RenderColor {
    RenderColor::rgba(
        byte(value.red),
        byte(value.green),
        byte(value.blue),
        byte(value.alpha),
    )
}

fn render_theme(snapshot: &ConfigSnapshot, high_contrast: bool) -> RenderTheme {
    let text = color(
        snapshot,
        "candidate_text",
        Rgba {
            red: 0.13,
            green: 0.13,
            blue: 0.14,
            alpha: 1.0,
        },
    );
    let background = color(
        snapshot,
        "background",
        Rgba {
            red: 0.97,
            green: 0.98,
            blue: 0.98,
            alpha: 1.0,
        },
    );
    if high_contrast {
        return RenderTheme {
            background: RenderColor::rgba(0, 0, 0, 255),
            text: RenderColor::rgba(255, 255, 255, 255),
            selected_background: RenderColor::rgba(0, 0, 128, 255),
            selected_text: RenderColor::rgba(255, 255, 255, 255),
            comment_color: RenderColor::rgba(255, 255, 0, 255),
            border: RenderColor::rgba(255, 255, 255, 255),
            scrollbar: RenderColor::rgba(255, 255, 255, 255),
            preedit_background: RenderColor::rgba(0, 0, 0, 255),
            preedit_text: RenderColor::rgba(255, 255, 255, 255),
            ..RenderTheme::default()
        };
    }
    RenderTheme {
        background: render_color(background),
        text: render_color(text),
        selected_background: render_color(color(
            snapshot,
            "selected_background",
            Rgba {
                red: 0.027,
                green: 0.757,
                blue: 0.376,
                alpha: 1.0,
            },
        )),
        selected_text: render_color(color(
            snapshot,
            "selected_candidate_text",
            Rgba {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                alpha: 1.0,
            },
        )),
        comment_color: render_color(color(snapshot, "comment_text", text)),
        border: render_color(color(snapshot, "border", background)),
        scrollbar: render_color(color(snapshot, "scrollbar", text)),
        preedit_background: render_color(background),
        preedit_text: render_color(color(snapshot, "preedit_text", text)),
        selection_inflate_x: 2.0,
        selection_inflate_y: 2.0,
        corner_radius: snapshot.candidate().geometry().corner_radius_dip(),
    }
}

fn axis_options(snapshot: &ConfigSnapshot) -> CandidateLayoutOptions {
    let options = snapshot.candidate().layout_options();
    CandidateLayoutOptions {
        orientation: match options.orientation {
            ConfigOrientation::Horizontal => Orientation::Horizontal,
            ConfigOrientation::Vertical => Orientation::Vertical,
        },
        overflow: match options.overflow {
            ConfigOverflow::Paging => OverflowBehavior::Paging,
            ConfigOverflow::Scrolling => OverflowBehavior::Scrolling,
            ConfigOverflow::Wrapping => OverflowBehavior::Wrapping,
        },
        writing_mode: match options.writing_mode {
            ConfigWriting::Horizontal => WritingMode::Horizontal,
            ConfigWriting::VerticalRl => WritingMode::VerticalRl,
            ConfigWriting::VerticalLr => WritingMode::VerticalLr,
        },
    }
}

fn label_style(value: &str) -> CandidateLabelStyle {
    match value {
        "plain" => CandidateLabelStyle::Plain,
        "paren" => CandidateLabelStyle::Paren,
        "bracket" => CandidateLabelStyle::Bracket,
        "circled" => CandidateLabelStyle::Circled,
        _ => CandidateLabelStyle::Dot,
    }
}

fn preview_candidates(snapshot: &ConfigSnapshot) -> Vec<CandidateRenderData> {
    const SAMPLE: [(&str, &str); 9] = [
        ("你", "first"),
        ("你好", "phrase"),
        ("输入法", "CJK"),
        ("Windows Next", "Latin"),
        ("，。！？", "punctuation"),
        ("😀🎉⌨️", "emoji"),
        ("Rime", "plugin"),
        ("候选", "comment"),
        ("稳定", "preview"),
    ];
    let label = snapshot.candidate().label();
    SAMPLE
        .iter()
        .enumerate()
        .map(|(index, (text, comment))| CandidateRenderData {
            label: if label.visible() {
                format_candidate_label(
                    (index + 1) as u32,
                    label.sequence().get(index).map_or("", String::as_str),
                    label_style(label.style()),
                    "",
                    "",
                )
            } else {
                String::new()
            },
            text: (*text).to_owned(),
            comment: (*comment).to_owned(),
        })
        .collect()
}

/// Renders the embedded Settings preview through the same candidate layout and
/// bitmap renderer used by the shipping candidate window.
///
/// Invalid dimensions or DPI are rejected so callers can clear their preview
/// instead of retaining stale pixels.
pub fn render_settings_candidate_preview(
    snapshot: &ConfigSnapshot,
    dpi_scale: f32,
    width_dip: f32,
    height_dip: f32,
    high_contrast: bool,
) -> Result<SettingsCandidatePreview, String> {
    if !dpi_scale.is_finite() || !(0.5..=4.0).contains(&dpi_scale) {
        return Err("settings candidate preview DPI must be within 0.5..=4.0".to_owned());
    }
    if !width_dip.is_finite()
        || !height_dip.is_finite()
        || !(1.0..=2048.0).contains(&width_dip)
        || !(1.0..=2048.0).contains(&height_dip)
    {
        return Err("settings candidate preview surface must be finite and bounded".to_owned());
    }

    let candidate = snapshot.candidate();
    let font_family = snapshot
        .fonts()
        .candidate()
        .families()
        .first()
        .map_or("Microsoft YaHei UI", String::as_str);
    let geometry = RenderGeometry {
        font_size: snapshot.fonts().candidate().size_dip(),
        label_font_size: snapshot.fonts().candidate().size_dip() * candidate.label().font_scale(),
        comment_font_size: snapshot.fonts().candidate().size_dip()
            * snapshot.fonts().annotation().scale(),
        label_gap: candidate.label().gap_dip(),
        item_padding_x: candidate.geometry().item_padding_x_dip(),
        item_padding_y: candidate.geometry().item_padding_y_dip(),
        preedit_height: 34.0,
    };
    let candidates = preview_candidates(snapshot);
    let mut measure = MeasureEngine::new();
    let sizes = candidates
        .iter()
        .map(|item| {
            let (label_width, label_height) = measure.measure_with_family(
                &item.label,
                font_family,
                geometry.label_font_size,
                dpi_scale,
            );
            let (text_width, text_height) =
                measure.measure_with_family(&item.text, font_family, geometry.font_size, dpi_scale);
            let (comment_width, comment_height) = measure.measure_with_family(
                &item.comment,
                font_family,
                geometry.comment_font_size,
                dpi_scale,
            );
            Size {
                width: geometry.item_padding_x * 2.0
                    + label_width
                    + if item.label.is_empty() {
                        0.0
                    } else {
                        geometry.label_gap
                    }
                    + text_width
                    + if item.comment.is_empty() {
                        0.0
                    } else {
                        geometry.label_gap + comment_width
                    },
                height: geometry.item_padding_y * 2.0
                    + label_height
                        .max(text_height)
                        .max(comment_height)
                        .max(geometry.font_size),
            }
        })
        .collect();
    let axis_result = axis_layout::layout(&axis_layout::AxisLayoutInput {
        options: axis_options(snapshot),
        items: sizes,
        caret: Point::default(),
        caret_height: 0.0,
        work_area: Rect {
            left: 0.0,
            top: 0.0,
            right: width_dip,
            bottom: height_dip,
        },
        max_width: width_dip.min(candidate.max_width_dip()),
        max_height: height_dip,
        padding_x: candidate.geometry().padding_x_dip(),
        padding_y: candidate.geometry().padding_y_dip(),
        row_gap: candidate.geometry().row_gap_dip(),
        column_gap: candidate.geometry().column_gap_dip(),
        page_size: usize::from(candidate.page_size()),
        selected: 0,
        scroll_override: None,
        placement: Placement::Unlocked,
    });
    let preedit = (candidate.preedit_mode() == "panel").then_some("ni hao · preview");
    let bitmap = render_candidate_window(&crate::renderer::RenderWindowInput {
        axis_result: &axis_result,
        candidates: &candidates,
        theme: &render_theme(snapshot, high_contrast),
        geometry: &geometry,
        font_family,
        preedit,
        dpi_scale,
        high_contrast,
        selected: Some(0),
    });
    if bitmap.pixels.is_empty() {
        return Err("settings candidate preview produced no pixels".to_owned());
    }
    Ok(SettingsCandidatePreview {
        bitmap,
        high_contrast,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcitx5_config_core::{ConfigCommand, ConfigCore, ConfigEdit, FileStore};
    use std::path::Path;

    fn snapshot(layout: &str) -> ConfigSnapshot {
        let store = FileStore::new();
        let mut core = ConfigCore::compiled_defaults();
        core.execute(
            ConfigCommand::Set(ConfigEdit::CandidateLayoutType(layout.to_owned())),
            &store,
            Path::new("settings-preview.toml"),
        )
        .expect("preview fixture layout is valid");
        core.preview()
    }

    #[test]
    fn production_renderer_covers_each_persisted_layout_mode() {
        for layout in ["automatic", "stacked", "flow", "scroll", "vertical_text"] {
            let preview = render_settings_candidate_preview(
                &snapshot(layout),
                1.0,
                SETTINGS_PREVIEW_WIDTH_DIP,
                SETTINGS_PREVIEW_HEIGHT_DIP,
                false,
            )
            .expect("every persisted layout mode should render");
            assert!(!preview.bitmap.pixels.is_empty(), "{layout}");
        }
    }

    #[test]
    fn invalid_surface_fails_without_retaining_pixels() {
        assert!(
            render_settings_candidate_preview(&snapshot("flow"), 1.0, 0.0, 100.0, false).is_err()
        );
        assert!(
            render_settings_candidate_preview(&snapshot("flow"), 4.5, 100.0, 100.0, false).is_err()
        );
    }

    #[test]
    fn high_contrast_uses_the_production_marker() {
        let preview = render_settings_candidate_preview(
            &snapshot("scroll"),
            1.5,
            SETTINGS_PREVIEW_WIDTH_DIP,
            SETTINGS_PREVIEW_HEIGHT_DIP,
            true,
        )
        .expect("high contrast preview renders");
        assert!(preview.high_contrast);
        assert!(!preview.bitmap.pixels.is_empty());
    }

    #[test]
    fn resolved_candidate_font_changes_production_preview_pixels() {
        let store = FileStore::new();
        let path = Path::new("settings-preview-font.toml");
        let mut core = ConfigCore::compiled_defaults();
        let baseline = render_settings_candidate_preview(
            &core.preview(),
            1.0,
            SETTINGS_PREVIEW_WIDTH_DIP,
            SETTINGS_PREVIEW_HEIGHT_DIP,
            false,
        )
        .expect("default font preview renders");
        core.execute(
            ConfigCommand::Set(ConfigEdit::CandidateFontFamilies(vec![
                "Consolas".to_owned()
            ])),
            &store,
            path,
        )
        .expect("font draft is valid");
        let changed = render_settings_candidate_preview(
            &core.preview(),
            1.0,
            SETTINGS_PREVIEW_WIDTH_DIP,
            SETTINGS_PREVIEW_HEIGHT_DIP,
            false,
        )
        .expect("changed font preview renders");
        assert_ne!(baseline.bitmap.pixels, changed.bitmap.pixels);
    }
}
