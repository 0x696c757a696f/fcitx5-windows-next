#![forbid(unsafe_code)]

// Ported from WindInput wind-ui candidate window/theme behavior.
// Source: https://github.com/huanfeng/WindInput, commit 2214bede43b4153f0fdc463928cf3c50184ec2ef.
// License: MIT, Copyright (c) 2026 WindInput Contributors.

use crate::theme_tokens::{
    WECHAT_GREEN_RGB, WECHAT_SELECTION_RADIUS_DIP, WECHAT_WINDOW_RADIUS_DIP, WHITE_RGB,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QingfengOrientation {
    Horizontal,
    Vertical,
    Grid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QingfengThemeMode {
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QingfengCandidateVisualInput {
    pub label: String,
    pub text: String,
    pub comment: String,
    pub selected: bool,
    pub show_label: bool,
    pub reserve_label: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QingfengRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl QingfengRect {
    pub fn width(self) -> f32 {
        (self.right - self.left).max(0.0)
    }

    pub fn height(self) -> f32 {
        (self.bottom - self.top).max(0.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QingfengColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl QingfengColor {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn colorref(self) -> u32 {
        u32::from(self.r) | (u32::from(self.g) << 8) | (u32::from(self.b) << 16)
    }

    fn over(self, bg: Self) -> Self {
        let alpha = u16::from(self.a);
        let blend = |fg: u8, bg: u8| {
            ((u16::from(fg) * alpha + u16::from(bg) * (255 - alpha) + 127) / 255) as u8
        };
        Self::rgb(
            blend(self.r, bg.r),
            blend(self.g, bg.g),
            blend(self.b, bg.b),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QingfengCandidateTypography {
    pub candidate_font_size: f32,
    pub label_font_size: f32,
    pub comment_font_size: f32,
    pub row_height: f32,
}

impl Default for QingfengCandidateTypography {
    fn default() -> Self {
        Self {
            candidate_font_size: 22.0,
            label_font_size: 18.0,
            comment_font_size: 16.0,
            row_height: 42.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QingfengCandidateTheme {
    pub typography: QingfengCandidateTypography,
    pub window_padding: f32,
    pub window_radius: f32,
    pub window_border_width: f32,
    pub item_padding_y: f32,
    pub item_padding_x: f32,
    pub item_radius: f32,
    pub index_text_gap: f32,
    pub comment_text_gap: f32,
    pub background: QingfengColor,
    pub border: QingfengColor,
    pub text: QingfengColor,
    pub selected_text: QingfengColor,
    pub label: QingfengColor,
    pub selected_background: QingfengColor,
    pub hover_background: QingfengColor,
}

impl QingfengCandidateTheme {
    pub fn scaled(self, dpi_scale: f32) -> Self {
        let scale = dpi_scale.clamp(0.5, 4.0);
        Self {
            window_padding: self.window_padding * scale,
            window_radius: self.window_radius * scale,
            window_border_width: self.window_border_width.max(1.0),
            item_padding_y: self.item_padding_y * scale,
            item_padding_x: self.item_padding_x * scale,
            item_radius: self.item_radius * scale,
            index_text_gap: self.index_text_gap * scale,
            comment_text_gap: self.comment_text_gap * scale,
            typography: self.typography,
            ..self
        }
    }
}

impl Default for QingfengCandidateTheme {
    fn default() -> Self {
        Self::light()
    }
}

impl QingfengCandidateTheme {
    pub fn light() -> Self {
        let background = QingfengColor::rgb(255, 255, 255);
        Self {
            typography: QingfengCandidateTypography::default(),
            window_padding: 5.0,
            window_radius: WECHAT_WINDOW_RADIUS_DIP,
            window_border_width: 1.0,
            // Keep a 34-DIP text box for a 22-DIP candidate run. DirectWrite
            // needs the extra descent headroom for Latin `gjpqy` and fallback
            // emoji; a non-overlapping rectangle is not enough.
            item_padding_y: 4.0,
            item_padding_x: 10.0,
            item_radius: WECHAT_SELECTION_RADIUS_DIP,
            index_text_gap: 1.0,
            comment_text_gap: 6.0,
            background,
            border: QingfengColor::rgb(226, 229, 235),
            text: QingfengColor::rgb(74, 74, 74),
            selected_text: QingfengColor::rgb(WHITE_RGB[0], WHITE_RGB[1], WHITE_RGB[2]),
            label: QingfengColor::rgb(154, 160, 174),
            selected_background: QingfengColor::rgb(
                WECHAT_GREEN_RGB[0],
                WECHAT_GREEN_RGB[1],
                WECHAT_GREEN_RGB[2],
            ),
            hover_background: QingfengColor {
                r: 0,
                g: 0,
                b: 0,
                a: 0x08,
            }
            .over(background),
        }
    }

    pub fn dark() -> Self {
        let background = QingfengColor::rgb(24, 24, 24);
        Self {
            typography: QingfengCandidateTypography::default(),
            window_padding: 5.0,
            window_radius: WECHAT_WINDOW_RADIUS_DIP,
            window_border_width: 1.0,
            item_padding_y: 4.0,
            item_padding_x: 10.0,
            item_radius: WECHAT_SELECTION_RADIUS_DIP,
            index_text_gap: 1.0,
            comment_text_gap: 6.0,
            background,
            border: QingfengColor::rgb(48, 48, 48),
            text: QingfengColor::rgb(237, 237, 237),
            selected_text: QingfengColor::rgb(WHITE_RGB[0], WHITE_RGB[1], WHITE_RGB[2]),
            label: QingfengColor::rgb(127, 127, 127),
            selected_background: QingfengColor::rgb(
                WECHAT_GREEN_RGB[0],
                WECHAT_GREEN_RGB[1],
                WECHAT_GREEN_RGB[2],
            ),
            hover_background: QingfengColor {
                r: 255,
                g: 255,
                b: 255,
                a: 0x12,
            }
            .over(background),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QingfengCandidateVisualItem {
    pub label_text: String,
    pub text: String,
    pub comment: String,
    pub selected: bool,
    pub item_rect: QingfengRect,
    pub label_rect: QingfengRect,
    pub text_rect: QingfengRect,
    pub comment_rect: Option<QingfengRect>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QingfengCandidateVisualPlan {
    pub source: &'static str,
    pub orientation: QingfengOrientation,
    pub theme: QingfengCandidateTheme,
    pub window: QingfengRect,
    pub items: Vec<QingfengCandidateVisualItem>,
}

pub const WINDINPUT_QINGFENG_CANDIDATE_SOURCE: &str =
    "WindInput wind-ui candidate_window.rs + data/themes/_qingfeng/theme.toml";

pub fn qingfeng_candidate_visual_plan(
    orientation: QingfengOrientation,
    mode: QingfengThemeMode,
    inputs: &[QingfengCandidateVisualInput],
    label_slot_width: f32,
    dpi_scale: f32,
) -> QingfengCandidateVisualPlan {
    let theme = match mode {
        QingfengThemeMode::Light => QingfengCandidateTheme::light(),
        QingfengThemeMode::Dark => QingfengCandidateTheme::dark(),
    }
    .scaled(dpi_scale);
    let scale = dpi_scale.clamp(0.5, 4.0);
    let row_h = theme.typography.row_height * scale;
    let text_char_w = (theme.typography.candidate_font_size + 2.0) * scale;
    let label_char_w = theme.typography.label_font_size * 0.62 * scale;
    let comment_char_w = theme.typography.comment_font_size * 0.72 * scale;
    let gap = match orientation {
        QingfengOrientation::Vertical => 4.0,
        QingfengOrientation::Horizontal | QingfengOrientation::Grid => 10.0,
    } * dpi_scale.clamp(0.5, 4.0);
    let columns = if orientation == QingfengOrientation::Grid {
        3
    } else if orientation == QingfengOrientation::Horizontal {
        inputs.len().max(1)
    } else {
        1
    };
    let item_width = |input: &QingfengCandidateVisualInput| {
        let label_w = if input.reserve_label {
            label_slot_width.max(input.label.chars().count() as f32 * label_char_w)
        } else {
            0.0
        };
        theme.item_padding_x * 2.0
            + label_w
            + if label_w > 0.0 {
                theme.index_text_gap
            } else {
                0.0
            }
            // Keep the complete CJK/emoji glyph budget. A fixed maximum
            // produces a non-overlapping rectangle that still clips glyphs.
            + (input.text.chars().count() as f32 * text_char_w).max(18.0)
            + if input.comment.is_empty() {
                0.0
            } else {
                theme.comment_text_gap + input.comment.chars().count() as f32 * comment_char_w
            }
    };
    let widest_cell = inputs.iter().map(item_width).fold(0.0_f32, f32::max);
    let column_widths = if orientation == QingfengOrientation::Horizontal {
        inputs.iter().map(item_width).collect::<Vec<_>>()
    } else if orientation == QingfengOrientation::Grid {
        let mut widths = vec![18.0_f32; columns];
        for (index, input) in inputs.iter().enumerate() {
            let column = index % columns;
            widths[column] = widths[column].max(item_width(input));
        }
        widths
    } else {
        vec![widest_cell.max(18.0)]
    };
    let mut items = Vec::with_capacity(inputs.len());
    for (index, input) in inputs.iter().enumerate() {
        let row = index / columns;
        let column = index % columns;
        let left = theme.window_padding
            + column_widths.iter().take(column).sum::<f32>()
            + column as f32 * gap;
        let top = theme.window_padding + row as f32 * (row_h + gap);
        let item = QingfengRect {
            left,
            top,
            right: left + column_widths[column],
            bottom: top + row_h,
        };
        let label_w = if input.reserve_label {
            label_slot_width.max(input.label.chars().count() as f32 * label_char_w)
        } else {
            0.0
        };
        let content_top = item.top + theme.item_padding_y;
        let content_bottom = item.bottom - theme.item_padding_y;
        // Labels/comments are smaller than candidates. Shift their layout box
        // down by the size delta so their glyph baselines share the candidate
        // baseline instead of merely sharing the row's vertical center.
        let label_baseline_offset =
            (theme.typography.candidate_font_size - theme.typography.label_font_size).max(0.0)
                * scale;
        let comment_baseline_offset =
            (theme.typography.candidate_font_size - theme.typography.comment_font_size).max(0.0)
                * scale;
        let label = QingfengRect {
            left: item.left + theme.item_padding_x,
            top: content_top + label_baseline_offset,
            right: item.left + theme.item_padding_x + label_w,
            bottom: (content_bottom + label_baseline_offset).min(item.bottom),
        };
        let text_left = if label_w > 0.0 {
            label.right + theme.index_text_gap
        } else {
            item.left + theme.item_padding_x
        };
        let comment_w = if input.comment.is_empty() {
            0.0
        } else {
            input.comment.chars().count() as f32 * comment_char_w
        };
        let text_right = if comment_w > 0.0 {
            item.right - theme.item_padding_x - theme.comment_text_gap - comment_w
        } else {
            item.right - theme.item_padding_x
        };
        let comment = (comment_w > 0.0).then_some(QingfengRect {
            left: text_right + theme.comment_text_gap,
            top: content_top + comment_baseline_offset,
            right: item.right - theme.item_padding_x,
            bottom: (content_bottom + comment_baseline_offset).min(item.bottom),
        });
        items.push(QingfengCandidateVisualItem {
            label_text: if input.show_label {
                input.label.clone()
            } else {
                String::new()
            },
            text: input.text.clone(),
            comment: input.comment.clone(),
            selected: input.selected,
            item_rect: item,
            label_rect: label,
            text_rect: QingfengRect {
                left: text_left,
                top: content_top,
                right: text_right.max(text_left + 12.0),
                bottom: content_bottom,
            },
            comment_rect: comment,
        });
    }
    let rows = inputs.len().div_ceil(columns).max(1);
    let window = QingfengRect {
        left: 0.0,
        top: 0.0,
        right: theme.window_padding * 2.0
            + column_widths.iter().sum::<f32>()
            + (columns - 1) as f32 * gap,
        bottom: theme.window_padding * 2.0 + rows as f32 * row_h + (rows - 1) as f32 * gap,
    };
    QingfengCandidateVisualPlan {
        source: WINDINPUT_QINGFENG_CANDIDATE_SOURCE,
        orientation,
        theme,
        window,
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qingfeng_plan_uses_windinput_candidate_tokens() {
        let plan = qingfeng_candidate_visual_plan(
            QingfengOrientation::Vertical,
            QingfengThemeMode::Light,
            &[QingfengCandidateVisualInput {
                label: "10.".to_owned(),
                text: "对齐很长的候选文本".to_owned(),
                comment: String::new(),
                selected: true,
                show_label: true,
                reserve_label: true,
            }],
            30.0,
            1.0,
        );

        assert_eq!(plan.source, WINDINPUT_QINGFENG_CANDIDATE_SOURCE);
        assert_eq!(plan.theme.window_radius, WECHAT_WINDOW_RADIUS_DIP);
        assert_eq!(plan.theme.item_radius, WECHAT_SELECTION_RADIUS_DIP);
        assert_eq!(
            plan.theme.selected_background,
            QingfengColor::rgb(7, 193, 96)
        );
        assert_eq!(plan.theme.selected_text, QingfengColor::rgb(255, 255, 255));
        assert!(plan.items[0].label_rect.width() >= 30.0);
        assert!(plan.items[0].text_rect.left > plan.items[0].label_rect.right);
        assert_eq!(
            plan.items[0].label_rect.top - plan.items[0].text_rect.top,
            4.0
        );
    }

    #[test]
    fn cjk_text_rect_keeps_full_glyph_budget_beside_comment() {
        let plan = qingfeng_candidate_visual_plan(
            QingfengOrientation::Vertical,
            QingfengThemeMode::Light,
            &[QingfengCandidateVisualInput {
                label: "4.".to_owned(),
                text: "水".to_owned(),
                comment: "~b".to_owned(),
                selected: false,
                show_label: true,
                reserve_label: true,
            }],
            36.0,
            1.5,
        );
        assert!(plan.items[0].text_rect.width() >= 22.0 * 1.5);
    }

    #[test]
    fn candidate_text_box_keeps_descender_headroom() {
        let plan = qingfeng_candidate_visual_plan(
            QingfengOrientation::Horizontal,
            QingfengThemeMode::Light,
            &[QingfengCandidateVisualInput {
                label: "1.".to_owned(),
                text: "gjpqy emoji 😀 e\u{301}".to_owned(),
                comment: String::new(),
                selected: true,
                show_label: true,
                reserve_label: true,
            }],
            20.0,
            1.0,
        );

        assert!(
            plan.items[0].text_rect.height() >= plan.theme.typography.candidate_font_size + 12.0,
            "candidate text box must leave DirectWrite descent/fallback headroom"
        );
    }

    #[test]
    fn horizontal_items_expand_individually_for_full_cjk_and_emoji_glyphs() {
        let plan = qingfeng_candidate_visual_plan(
            QingfengOrientation::Horizontal,
            QingfengThemeMode::Light,
            &[
                QingfengCandidateVisualInput {
                    label: "1.".to_owned(),
                    text: "点赞".to_owned(),
                    comment: String::new(),
                    selected: true,
                    show_label: true,
                    reserve_label: true,
                },
                QingfengCandidateVisualInput {
                    label: "2.".to_owned(),
                    text: "👍".to_owned(),
                    comment: String::new(),
                    selected: false,
                    show_label: true,
                    reserve_label: true,
                },
                QingfengCandidateVisualInput {
                    label: "3.".to_owned(),
                    text: "短暂".to_owned(),
                    comment: String::new(),
                    selected: false,
                    show_label: true,
                    reserve_label: true,
                },
            ],
            20.0,
            1.0,
        );
        for item in &plan.items {
            assert!(
                item.text_rect.width() >= item.text.chars().count() as f32 * 20.0,
                "{} must fit its full glyph budget",
                item.text
            );
        }
        assert!(plan.items[1].item_rect.left >= plan.items[0].item_rect.right);
        assert!(plan.items[2].item_rect.left >= plan.items[1].item_rect.right);
    }

    #[test]
    fn grid_sizes_each_column_to_its_own_widest_candidate() {
        let plan = qingfeng_candidate_visual_plan(
            QingfengOrientation::Grid,
            QingfengThemeMode::Light,
            &[
                QingfengCandidateVisualInput {
                    label: "1.".to_owned(),
                    text: "短".to_owned(),
                    comment: String::new(),
                    selected: false,
                    show_label: true,
                    reserve_label: true,
                },
                QingfengCandidateVisualInput {
                    label: "2.".to_owned(),
                    text: "很长的候选文本".to_owned(),
                    comment: String::new(),
                    selected: true,
                    show_label: true,
                    reserve_label: true,
                },
                QingfengCandidateVisualInput {
                    label: "3.".to_owned(),
                    text: "中等".to_owned(),
                    comment: String::new(),
                    selected: false,
                    show_label: true,
                    reserve_label: true,
                },
            ],
            20.0,
            1.0,
        );
        let widest = plan
            .items
            .iter()
            .map(|item| item.item_rect.width())
            .fold(0.0_f32, f32::max);
        assert!(plan.items[0].item_rect.width() < widest);
        assert!(plan.items[2].item_rect.width() < widest);
        assert!(
            plan.window.width() < 10.0 + widest * 3.0 + 20.0,
            "grid must not give every column the longest candidate width"
        );
    }

    #[test]
    fn qingfeng_plan_has_wechat_like_light_and_dark_palettes() {
        let light = qingfeng_candidate_visual_plan(
            QingfengOrientation::Horizontal,
            QingfengThemeMode::Light,
            &[],
            0.0,
            1.0,
        );
        let dark = qingfeng_candidate_visual_plan(
            QingfengOrientation::Horizontal,
            QingfengThemeMode::Dark,
            &[],
            0.0,
            1.0,
        );

        assert_eq!(light.theme.background, QingfengColor::rgb(255, 255, 255));
        assert_eq!(
            light.theme.selected_background,
            QingfengColor::rgb(7, 193, 96)
        );
        assert_eq!(light.theme.selected_text, QingfengColor::rgb(255, 255, 255));
        assert_eq!(dark.theme.background, QingfengColor::rgb(24, 24, 24));
        assert_eq!(
            dark.theme.selected_background,
            QingfengColor::rgb(7, 193, 96)
        );
        assert_eq!(dark.theme.selected_text, QingfengColor::rgb(255, 255, 255));
    }
}
