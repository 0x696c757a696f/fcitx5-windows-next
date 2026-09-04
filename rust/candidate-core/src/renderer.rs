//! Runtime candidate-window renderer (081A: first Rust slice of the
//! D2D→tiny-skia cutover, task CANDIDATE-RENDERER-D2D-TINY-SKIA-CUTOVER-001).
#![forbid(unsafe_code)]
//!
//! One paint path over frozen `axis_layout` geometry. The caller already ran
//! the three-axis layout and hands over window rect + per-item rects + scroll
//! offset + candidate strings + a resolved theme; this module produces a
//! complete BGRA window bitmap through the windui tiny-skia canvas and the
//! DirectWrite text engine. Draw order mirrors the legacy C++ `paintOnce`
//! (survey docs/tasks/081-ui-survey.md §paintOnce): background → preedit text
//! + divider → per-candidate label/text/comment → selection rounded rect →
//! scrollbar → outer rounded border.
//!
//! All geometry here is window-local: `axis_result.window` may sit anywhere
//! on the work area and the caller blits the returned bitmap at that origin.
//! Text rows never wrap or spill below their own row (each segment is drawn
//! through a clip of its measured rect), matching the frozen C++ contract.

use tiny_skia::Pixmap;
use windui::geometry::{Color as WindColor, Rect as WindRect};
use windui::render::{Canvas, Paint as WindPaint, SkiaCanvas};
use windui::spec::Align as WindAlign;
use windui::text::{DWriteEngine, TextEngine, TextStyle as WindTextStyle};

use crate::axis_layout::{AxisLayoutItem, AxisLayoutResult, WritingMode};
use crate::Rect;

/// ARGB color resolved by the theme/config boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl RenderColor {
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    fn wind(self) -> WindColor {
        WindColor::rgba(self.red, self.green, self.blue, self.alpha)
    }
}

/// Fully resolved colors + selection inflate + corner radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderTheme {
    pub background: RenderColor,
    pub text: RenderColor,
    pub selected_background: RenderColor,
    pub selected_text: RenderColor,
    pub comment_color: RenderColor,
    pub border: RenderColor,
    pub scrollbar: RenderColor,
    pub preedit_background: RenderColor,
    pub preedit_text: RenderColor,
    /// Rounded-rect inflation around the selected item (logical px).
    pub selection_inflate_x: f32,
    pub selection_inflate_y: f32,
    /// Shared window/item corner radius (logical px).
    pub corner_radius: f32,
}

impl Default for RenderTheme {
    fn default() -> Self {
        // WeChat-green selection on white (the shipped light look).
        Self {
            background: RenderColor::rgba(255, 255, 255, 255),
            text: RenderColor::rgba(32, 33, 36, 255),
            selected_background: RenderColor::rgba(240, 250, 231, 255),
            selected_text: RenderColor::rgba(96, 193, 7, 255),
            comment_color: RenderColor::rgba(120, 120, 120, 255),
            border: RenderColor::rgba(215, 215, 215, 255),
            scrollbar: RenderColor::rgba(128, 128, 128, 180),
            preedit_background: RenderColor::rgba(255, 255, 255, 255),
            preedit_text: RenderColor::rgba(32, 33, 36, 255),
            selection_inflate_x: 2.0,
            selection_inflate_y: 2.0,
            corner_radius: 8.0,
        }
    }
}

/// Font sizes and text-metric parameters the geometry was measured with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderGeometry {
    pub font_size: f32,
    pub label_font_size: f32,
    pub comment_font_size: f32,
    pub label_gap: f32,
    pub item_padding_x: f32,
    pub item_padding_y: f32,
    /// Height of the preedit row (incl. divider) above the candidate rows.
    pub preedit_height: f32,
}

impl Default for RenderGeometry {
    fn default() -> Self {
        Self {
            font_size: 18.0,
            label_font_size: 18.0 * 0.85,
            comment_font_size: 18.0 * 0.80,
            label_gap: 4.0,
            item_padding_x: 8.0,
            item_padding_y: 6.0,
            preedit_height: 34.0,
        }
    }
}

/// One candidate's visible strings (label already formatted for display).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateRenderData {
    pub label: String,
    pub text: String,
    pub comment: String,
}

/// Everything the renderer needs for one complete window bitmap.
#[derive(Clone, Debug)]
pub struct RenderWindowInput<'a> {
    /// Frozen three-axis geometry (window + per-item rects + scroll offset).
    pub axis_result: &'a AxisLayoutResult,
    /// Candidate strings in the same order as `axis_result.items`.
    pub candidates: &'a [CandidateRenderData],
    /// Resolved colors + selection inflate + corner radius.
    pub theme: &'a RenderTheme,
    /// Font sizes / gaps / preedit row height.
    pub geometry: &'a RenderGeometry,
    /// Preedit text drawn above the candidate rows when present.
    pub preedit: Option<&'a str>,
    /// Logical→physical scale (1.0 at 96 DPI).
    pub dpi_scale: f32,
    /// When set, the preedit row also gets `preedit_background` (legacy
    /// high-contrast surface); otherwise only text + divider are painted.
    pub high_contrast: bool,
    /// Highlighted candidate index into `candidates`/`axis_result.items`.
    /// `None` or out of range paints no selection.
    pub selected: Option<usize>,
}

/// Total scrollable extent of the whole content along the viewport axis
/// (clamped scroll_override may leave natural content beyond the applied
/// offset). Unit px.
fn total_scrollable(axis: &AxisLayoutResult) -> f32 {
    let width = axis.content_size.width;
    let height = axis.content_size.height;
    let window_w = (axis.window.right - axis.window.left).max(0.0);
    let window_h = (axis.window.bottom - axis.window.top).max(0.0);
    (height - window_h)
        .max(0.0)
        .max((width - window_w).max(0.0))
}

/// A complete candidate-window bitmap in BGRA row-major order, window-local.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderWindowOutput {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// `width * 4` (BGRA, tightly packed rows).
    pub stride: u32,
}

impl RenderWindowOutput {
    /// BGRA pixel at `(x, y)` or `None` when out of bounds.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let start = (y * self.stride + x * 4) as usize;
        self.pixels
            .get(start..start + 4)
            .map(|p| [p[0], p[1], p[2], p[3]])
    }
}

/// Render the complete candidate-window bitmap described by `input`.
///
/// The bitmap covers `axis_result.window` with a window-local origin (the
/// caller blits it at the window's work-area position). Items whose rect does
/// not intersect the window are skipped. Degenerate (zero-area) windows yield
/// an empty output so callers treat "nothing to paint" uniformly.
#[must_use]
pub fn render_candidate_window(input: &RenderWindowInput<'_>) -> RenderWindowOutput {
    let axis = input.axis_result;
    let window_w = (axis.window.right - axis.window.left).ceil().max(0.0);
    let window_h = (axis.window.bottom - axis.window.top).ceil().max(0.0);
    if window_w < 1.0 || window_h < 1.0 {
        return RenderWindowOutput {
            pixels: Vec::new(),
            width: window_w as u32,
            height: window_h as u32,
            stride: 0,
        };
    }
    let scale = input.dpi_scale.clamp(0.25, 8.0);
    let width = (window_w * scale).ceil() as u32;
    let height = (window_h * scale).ceil() as u32;
    if width == 0 || height == 0 {
        return RenderWindowOutput {
            pixels: Vec::new(),
            width,
            height,
            stride: 0,
        };
    }
    let stride = width * 4;
    let Some(mut pixmap) = Pixmap::new(width, height) else {
        return RenderWindowOutput {
            pixels: Vec::new(),
            width,
            height,
            stride,
        };
    };
    let mut engine = DWriteEngine::new();
    engine.set_scale(scale);
    let mut canvas = SkiaCanvas::with_text(&mut pixmap, &mut engine, scale);

    // 1. Background.
    canvas.fill_rect(
        0.0,
        0.0,
        window_w,
        window_h,
        &WindPaint::fill(input.theme.background.wind()),
    );

    // 2. Preedit row (text + divider; high-contrast fills the row background).
    if let Some(preedit) = input.preedit.filter(|text| !text.is_empty()) {
        draw_preedit(&mut canvas, input, window_w, window_h, preedit);
    }

    // 3. Selection rect under the selected row's text.
    if let Some(selected) = input.selected {
        if let Some(item) = axis.items.get(selected).filter(|item| item.visible) {
            draw_selection(&mut canvas, input, window_w, window_h, item);
        }
    }

    // 4. Candidate rows.
    for (index, item) in axis.items.iter().enumerate() {
        if !item.visible {
            continue;
        }
        let Some(candidate) = input.candidates.get(index) else {
            continue;
        };
        let selected = input.selected == Some(index);
        if item.writing == WritingMode::Horizontal {
            draw_candidate_horizontal(
                &mut canvas,
                input,
                window_w,
                window_h,
                item,
                candidate,
                selected,
            );
        } else {
            draw_candidate_vertical(
                &mut canvas,
                input,
                window_w,
                window_h,
                item,
                candidate,
                selected,
            );
        }
    }

    // 5. Scrollbar (right edge) when the content overflows its viewport.
    draw_scrollbar(&mut canvas, input, window_w, window_h, axis);

    // 6. Outer rounded border.
    draw_border(&mut canvas, input, window_w, window_h);

    drop(canvas);
    RenderWindowOutput {
        pixels: pixmap_to_bgra(&pixmap),
        width,
        height,
        stride,
    }
}

/// tiny-skia stores premultiplied RGBA; the C++ blit / GDI
/// `SetDIBitsToDevice` / tests consume straight BGRA.
fn pixmap_to_bgra(pixmap: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixmap.data().len());
    for pixel in pixmap.data().chunks_exact(4) {
        let alpha = pixel[3];
        let un = |channel: u8| {
            if alpha == 0 {
                0
            } else {
                ((channel as u32 * 255) / alpha as u32).min(255) as u8
            }
        };
        out.extend_from_slice(&[un(pixel[2]), un(pixel[1]), un(pixel[0]), alpha]);
    }
    out
}

fn draw_preedit(
    canvas: &mut SkiaCanvas<'_>,
    input: &RenderWindowInput<'_>,
    window_w: f32,
    window_h: f32,
    preedit: &str,
) {
    let geometry = input.geometry;
    let row_height = geometry.preedit_height.max(8.0).min(window_h);
    if input.high_contrast {
        canvas.fill_rect(
            0.0,
            0.0,
            window_w,
            row_height,
            &WindPaint::fill(input.theme.preedit_background.wind()),
        );
    }
    draw_text_clipped(
        canvas,
        preedit,
        Rect {
            left: geometry.item_padding_x,
            top: 0.0,
            right: window_w - geometry.item_padding_x,
            bottom: row_height,
        },
        input.theme.preedit_text.wind(),
        WindAlign::Start,
        &text_style(geometry.font_size),
    );
    // Divider at the bottom of the preedit row (semi-transparent border).
    let divider_y = (row_height - 1.0).max(0.0);
    let mut border = input.theme.border.wind();
    border.a = 115;
    canvas.fill_rect(0.0, divider_y, window_w, 1.0, &WindPaint::fill(border));
}

fn draw_selection(
    canvas: &mut SkiaCanvas<'_>,
    input: &RenderWindowInput<'_>,
    window_w: f32,
    window_h: f32,
    item: &AxisLayoutItem,
) {
    let origin = &input.axis_result.window;
    let inflate_x = input.theme.selection_inflate_x.max(0.0);
    let inflate_y = input.theme.selection_inflate_y.max(0.0);
    let x = (item.rect.left - origin.left - inflate_x).max(0.0);
    let y = (item.rect.top - origin.top - inflate_y).max(0.0);
    let right = (item.rect.right - origin.left + inflate_x).min(window_w);
    let bottom = (item.rect.bottom - origin.top + inflate_y).min(window_h);
    let w = (right - x).max(0.0);
    let h = (bottom - y).max(0.0);
    if w > 0.0 && h > 0.0 {
        canvas.fill_round_rect(
            x,
            y,
            w,
            h,
            input.theme.corner_radius.max(0.0),
            &WindPaint::fill(input.theme.selected_background.wind()),
        );
    }
}

fn draw_candidate_horizontal(
    canvas: &mut SkiaCanvas<'_>,
    input: &RenderWindowInput<'_>,
    window_w: f32,
    window_h: f32,
    item: &AxisLayoutItem,
    candidate: &CandidateRenderData,
    selected: bool,
) {
    let origin = &input.axis_result.window;
    let left = (item.rect.left - origin.left).clamp(0.0, window_w);
    let top = (item.rect.top - origin.top).clamp(0.0, window_h);
    let right = (item.rect.right - origin.left).clamp(left, window_w);
    let bottom = (item.rect.bottom - origin.top).clamp(top, window_h);
    let bounds = Rect {
        left,
        top,
        right,
        bottom,
    };
    let geometry = input.geometry;
    let pad_x = geometry.item_padding_x.max(0.0);
    let pad_y = geometry.item_padding_y.max(0.0);
    let label_gap = geometry.label_gap.max(0.0);
    let content_left = (bounds.left + pad_x).min(bounds.right);
    let content_right = (bounds.right - pad_x).max(content_left);
    let row_top = (bounds.top + pad_y).max(0.0);
    let row_bottom = (bounds.bottom - pad_y).max(row_top);
    let row_height = (row_bottom - row_top).max(1.0);
    let style_text = text_style(geometry.font_size);
    let style_label = text_style(geometry.label_font_size);
    let style_comment = text_style(geometry.comment_font_size);

    let text_w = canvas.measure_text(&candidate.text, &style_text).w.max(0) as f32;

    // Per-segment geometry mirrors render_segments + the frozen C++ adapter:
    // label right-aligned in its own cell, then text, then comment when it
    // still fits. Every segment is clipped to its rect (never wraps or spills
    // onto the row below).
    let mut cursor = content_left;
    let label_w = geometry.label_font_size.max(1.0) * 1.6;
    if !candidate.label.is_empty() {
        let label_cell_right = (cursor + label_w).min(content_right);
        if label_cell_right > cursor {
            let label_rect = Rect {
                left: cursor,
                top: row_top,
                right: label_cell_right,
                bottom: row_top + row_height,
            };
            draw_text_clipped(
                canvas,
                &candidate.label,
                label_rect,
                if selected {
                    input.theme.selected_text.wind()
                } else {
                    input.theme.text.wind()
                },
                WindAlign::End,
                &style_label,
            );
        }
        cursor = (label_cell_right + label_gap).min(content_right);
    }
    if !candidate.text.is_empty() {
        let text_left = cursor.min(content_right);
        let available = (content_right - text_left).max(0.0);
        let draw_w = text_w.min(available).max(0.0);
        let text_rect = Rect {
            left: text_left,
            top: row_top,
            right: text_left + draw_w,
            bottom: row_top + row_height,
        };
        draw_text_clipped(
            canvas,
            &candidate.text,
            text_rect,
            if selected {
                input.theme.selected_text.wind()
            } else {
                input.theme.text.wind()
            },
            WindAlign::Start,
            &style_text,
        );
        cursor = text_left + draw_w;
    }
    if !candidate.comment.is_empty() {
        let comment_w = canvas
            .measure_text(&candidate.comment, &style_comment)
            .w
            .max(0) as f32;
        let comment_left = (cursor + label_gap).min(content_right);
        let available = (content_right - comment_left).max(0.0);
        if comment_w <= available {
            draw_text_clipped(
                canvas,
                &candidate.comment,
                Rect {
                    left: comment_left,
                    top: row_top,
                    right: content_right,
                    bottom: row_top + row_height,
                },
                input.theme.comment_color.wind(),
                WindAlign::Start,
                &style_comment,
            );
        }
    }
}

/// Vertical writing modes: the windui DWrite engine has no vertical text
/// flow, so each glyph is typeset horizontally, one glyph per row, top →
/// bottom, centered in the column cell (CJK reads as a vertical column of
/// upright glyphs). This mirrors the frozen candidate_poc `render_vertical_text`
/// interim rendering note (per-glyph vertical glyph drawing stays a later
/// renderer slice; the label draws once above the glyph stack).
fn draw_candidate_vertical(
    canvas: &mut SkiaCanvas<'_>,
    input: &RenderWindowInput<'_>,
    window_w: f32,
    window_h: f32,
    item: &AxisLayoutItem,
    candidate: &CandidateRenderData,
    selected: bool,
) {
    let origin = &input.axis_result.window;
    let left = (item.rect.left - origin.left).clamp(0.0, window_w);
    let top = (item.rect.top - origin.top).clamp(0.0, window_h);
    let right = (item.rect.right - origin.left).clamp(left, window_w);
    let bottom = (item.rect.bottom - origin.top).clamp(top, window_h);
    let geometry = input.geometry;
    let pad_x = geometry.item_padding_x.max(0.0);
    let color = if selected {
        input.theme.selected_text.wind()
    } else {
        input.theme.text.wind()
    };
    let glyph_step = (geometry.font_size * 1.6).ceil().max(1.0);
    let column_left = (left + pad_x).min(right);
    let column_right = (right - pad_x).max(column_left);
    let mut cell_text =
        |text: &str, cell_top: f32, cell_bottom: f32, size: f32, align: WindAlign| {
            let cell_top = cell_top.max(top);
            let cell_bottom = cell_bottom.min(bottom);
            if cell_bottom > cell_top {
                draw_text_clipped(
                    canvas,
                    text,
                    Rect {
                        left: column_left,
                        top: cell_top,
                        right: column_right,
                        bottom: cell_bottom,
                    },
                    color,
                    align,
                    &text_style(size),
                );
            }
        };
    let mut y = top;
    // Label draws once above the glyph stack (column head), like the frozen
    // vertical_text screenshot renderer.
    if !candidate.label.is_empty() {
        cell_text(
            &candidate.label,
            y,
            y + geometry.font_size,
            geometry.label_font_size,
            WindAlign::Center,
        );
        y += geometry.font_size;
    }
    for glyph in candidate.text.chars() {
        if y >= bottom {
            break;
        }
        let glyph_text: String = glyph.to_string();
        cell_text(
            &glyph_text,
            y,
            y + glyph_step,
            geometry.font_size,
            WindAlign::Center,
        );
        y += glyph_step;
    }
}

/// Frozen legacy scrollbar: 4px rounded bar on the window right edge, from
/// `item_padding_y` to `height - item_padding_y`, drawn only when the vertical
/// list viewport is active (`viewport_offset` carries a vertical scroll).
/// Thumb length ∝ viewport extent / content extent (min 18px), position ∝
/// the applied offset / total scrollable extent. Horizontal-row and
/// vertical-column scrolling had no legacy bar and draw none.
fn draw_scrollbar(
    canvas: &mut SkiaCanvas<'_>,
    input: &RenderWindowInput<'_>,
    window_w: f32,
    window_h: f32,
    axis: &AxisLayoutResult,
) {
    let geometry = input.geometry;
    let Some((_dx, dy)) = axis.viewport_offset else {
        return;
    };
    if dy <= 0.0 || axis.content_size.height <= window_h {
        return;
    }
    let pad_y = geometry.item_padding_y.max(0.0);
    let track_x = window_w - 6.0;
    let track_y = pad_y;
    let track_h = (window_h - pad_y * 2.0).max(1.0);
    let content = axis.content_size.height.max(window_h);
    let scrollable = total_scrollable(axis);
    let length = (track_h * window_h / content).clamp(18.0, track_h);
    let travel = (track_h - length).max(0.0);
    let thumb_y = track_y + travel * (dy / scrollable.max(1e-3)).clamp(0.0, 1.0);
    let radius = 2.0;
    canvas.fill_round_rect(
        track_x,
        track_y,
        4.0,
        track_h,
        radius,
        &WindPaint::fill(input.theme.scrollbar.wind()),
    );
    canvas.fill_round_rect(
        track_x,
        thumb_y,
        4.0,
        length,
        radius,
        &WindPaint::fill(input.theme.selected_text.wind()),
    );
}

fn draw_border(
    canvas: &mut SkiaCanvas<'_>,
    input: &RenderWindowInput<'_>,
    window_w: f32,
    window_h: f32,
) {
    let width = 1.0_f32;
    if window_w > width && window_h > width {
        let inset = width / 2.0;
        canvas.stroke_round_rect(
            inset,
            inset,
            window_w - width,
            window_h - width,
            input.theme.corner_radius.max(0.0),
            width,
            &WindPaint::fill(input.theme.border.wind()),
        );
    }
}

/// One clipped no-wrap text run inside `rect` (vertical centering comes from
/// the DWrite engine's block contract). `End`-aligned text is anchored so the
/// visible run ends at the rect's right edge (layout widened on the align
/// side, same trick as candidate_poc).
fn draw_text_clipped(
    canvas: &mut SkiaCanvas<'_>,
    text: &str,
    rect: Rect,
    color: WindColor,
    align: WindAlign,
    style: &WindTextStyle<'_>,
) {
    if text.is_empty() || rect.right <= rect.left || rect.bottom <= rect.top {
        return;
    }
    let clip = windui_text_rect(rect);
    canvas.save();
    canvas.clip_rect(clip);
    let layout = match align {
        WindAlign::End => WindRect::new(clip.right() - 8192, clip.y, 8192, clip.h),
        WindAlign::Center => WindRect::new(clip.x + clip.w / 2 - 4096, clip.y, 8192, clip.h),
        _ => WindRect::new(clip.x, clip.y, 8192, clip.h),
    };
    canvas.draw_text(text, layout, color, align, style);
    canvas.restore();
}

fn text_style(size: f32) -> WindTextStyle<'static> {
    WindTextStyle {
        family: Some("Microsoft YaHei UI"),
        size: size.max(1.0),
        weight: 400,
        italic: false,
        line_height: None,
    }
}

/// Logical float rect → windui i32 rect (floor/ceil so the clip fully covers).
fn windui_text_rect(rect: Rect) -> WindRect {
    let x = rect.left.floor() as i32;
    let y = rect.top.floor() as i32;
    let right = rect.right.ceil() as i32;
    let bottom = rect.bottom.ceil() as i32;
    WindRect::new(x, y, (right - x).max(1), (bottom - y).max(1))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::axis_layout::{
        layout as axis_layout, AxisLayoutInput, CandidateLayoutOptions, OverflowBehavior,
    };

    const WHITE: RenderColor = RenderColor::rgba(255, 255, 255, 255);
    const GREEN: RenderColor = RenderColor::rgba(96, 193, 7, 255);
    const GREEN_BG: RenderColor = RenderColor::rgba(240, 250, 231, 255);

    fn vertical_input(sizes: &[(f32, f32)]) -> AxisLayoutInput {
        AxisLayoutInput {
            options: CandidateLayoutOptions {
                orientation: crate::Orientation::Vertical,
                overflow: OverflowBehavior::Paging,
                writing_mode: WritingMode::Horizontal,
            },
            items: sizes
                .iter()
                .map(|(w, h)| crate::Size {
                    width: *w,
                    height: *h,
                })
                .collect(),
            caret: crate::Point { x: 200.0, y: 300.0 },
            caret_height: 20.0,
            work_area: crate::Rect {
                left: 0.0,
                top: 0.0,
                right: 800.0,
                bottom: 600.0,
            },
            max_width: 0.0,
            max_height: 0.0,
            padding_x: 8.0,
            padding_y: 6.0,
            row_gap: 2.0,
            column_gap: 8.0,
            page_size: 0,
            selected: 1,
            scroll_override: None,
            placement: crate::Placement::Below,
            ..AxisLayoutInput::default()
        }
    }

    fn candidates() -> Vec<CandidateRenderData> {
        vec![
            CandidateRenderData {
                label: "1".to_owned(),
                text: "你".to_owned(),
                comment: "nǐ".to_owned(),
            },
            CandidateRenderData {
                label: "2".to_owned(),
                text: "好".to_owned(),
                comment: "hǎo".to_owned(),
            },
            CandidateRenderData {
                label: "3".to_owned(),
                text: "汉".to_owned(),
                comment: String::new(),
            },
        ]
    }

    fn geometry() -> RenderGeometry {
        RenderGeometry {
            font_size: 18.0,
            label_font_size: 16.0,
            comment_font_size: 14.0,
            label_gap: 4.0,
            item_padding_x: 8.0,
            item_padding_y: 6.0,
            preedit_height: 30.0,
        }
    }

    fn theme() -> RenderTheme {
        RenderTheme {
            background: WHITE,
            text: RenderColor::rgba(40, 40, 40, 255),
            selected_background: GREEN_BG,
            selected_text: GREEN,
            comment_color: RenderColor::rgba(120, 120, 120, 255),
            border: RenderColor::rgba(215, 215, 215, 255),
            scrollbar: RenderColor::rgba(128, 128, 128, 180),
            preedit_background: WHITE,
            preedit_text: RenderColor::rgba(40, 40, 40, 255),
            selection_inflate_x: 2.0,
            selection_inflate_y: 2.0,
            corner_radius: 8.0,
        }
    }

    #[test]
    fn render_three_candidates_dimensions_bg_and_selection() {
        let axis = axis_layout(&vertical_input(&[(80.0, 24.0); 3]));
        assert_eq!(axis.items.len(), 3);
        let output = render_candidate_window(&RenderWindowInput {
            axis_result: &axis,
            candidates: &candidates(),
            theme: &theme(),
            geometry: &geometry(),
            preedit: None,
            dpi_scale: 1.0,
            high_contrast: false,
            selected: Some(1),
        });
        assert!(!output.pixels.is_empty());
        assert_eq!(
            output.width,
            (axis.window.right - axis.window.left).ceil() as u32
        );
        assert_eq!(
            output.height,
            (axis.window.bottom - axis.window.top).ceil() as u32
        );
        assert_eq!(output.stride, output.width * 4);
        assert_eq!(
            output.pixels.len(),
            (output.stride * output.height) as usize
        );

        // BGRA near the window's top-left corner (inside, past the 1px border):
        // background white.
        let px = output.pixel(2, 2).unwrap();
        assert_eq!([px[0], px[1], px[2], px[3]], [255, 255, 255, 255]);
        // Selection fill interior, clear of the glyph runs (text starts at
        // item.left + item_padding_x = window-local 16): sample just inside the
        // inflated rounded rect's left edge at row center.
        let rect = &axis.items[1].rect;
        let sel_left = ((rect.left - axis.window.left) - 2.0).max(0.0) as u32 + 1;
        let cy = ((rect.top - axis.window.top + rect.bottom - axis.window.top) / 2.0) as u32;
        let sel = output.pixel(sel_left, cy).unwrap();
        assert!(
            (sel[2] as i16 - GREEN_BG.red as i16).abs() <= 2
                && (sel[1] as i16 - GREEN_BG.green as i16).abs() <= 2
                && (sel[0] as i16 - GREEN_BG.blue as i16).abs() <= 2,
            "selection interior is the WeChat-green tint (got BGRA {sel:?})"
        );
        // Interior of an unselected row (left of its text start) stays white.
        let rect0 = &axis.items[0].rect;
        let x0 = ((rect0.left - axis.window.left) + 2.0) as u32;
        let y0 = ((rect0.top - axis.window.top + rect0.bottom - axis.window.top) / 2.0) as u32;
        let unsel = output
            .pixel(x0.min(output.width - 1), y0.min(output.height - 1))
            .unwrap();
        assert!(
            unsel[0] >= 250 && unsel[1] >= 250 && unsel[2] >= 250,
            "unselected row background stays white (got {unsel:?})"
        );
    }

    #[test]
    fn out_of_range_selection_renders_no_selection() {
        let axis = axis_layout(&vertical_input(&[(80.0, 24.0); 2]));
        let output = render_candidate_window(&RenderWindowInput {
            axis_result: &axis,
            candidates: &candidates()[..2],
            theme: &theme(),
            geometry: &geometry(),
            preedit: None,
            dpi_scale: 1.0,
            high_contrast: false,
            selected: Some(99),
        });
        assert!(!output.pixels.is_empty());
        let rect = &axis.items[0].rect;
        let x = ((rect.left - axis.window.left) + 2.0) as u32;
        let y = ((rect.top - axis.window.top + rect.bottom - axis.window.top) / 2.0) as u32;
        let p = output
            .pixel(x.min(output.width - 1), y.min(output.height - 1))
            .unwrap();
        assert!(
            p[0] >= 250 && p[1] >= 250 && p[2] >= 250,
            "no selection painted when selected index out of range (got {p:?})"
        );
    }

    #[test]
    fn empty_items_yield_empty_output() {
        let axis = AxisLayoutResult::default();
        let output = render_candidate_window(&RenderWindowInput {
            axis_result: &axis,
            candidates: &[],
            theme: &theme(),
            geometry: &geometry(),
            preedit: None,
            dpi_scale: 1.0,
            high_contrast: false,
            selected: None,
        });
        assert!(output.pixels.is_empty());
        assert_eq!(output.width, 0);
        assert_eq!(output.stride, 0);
    }
}
