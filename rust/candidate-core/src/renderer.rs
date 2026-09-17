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

use std::collections::HashMap;

use tiny_skia::Pixmap;
use windui::geometry::{Color as WindColor, Rect as WindRect};
use windui::render::{Canvas, Paint as WindPaint, SkiaCanvas};
use windui::spec::Align as WindAlign;
use windui::text::{DWriteEngine, TextEngine, TextStyle as WindTextStyle};

use crate::axis_layout::{AxisLayoutItem, AxisLayoutResult, WritingMode};
use crate::theme_tokens::{
    WECHAT_GREEN_RGB, WECHAT_SELECTION_RADIUS_DIP, WECHAT_WINDOW_RADIUS_DIP, WHITE_RGB,
};
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

/// Fully resolved colors + selection inset + outer-surface corner radius.
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
    /// Outer floating candidate-surface radius (logical px). The selected
    /// candidate is an inset pill, derived two px tighter than this surface.
    pub corner_radius: f32,
}

impl Default for RenderTheme {
    fn default() -> Self {
        Self {
            background: RenderColor::rgba(255, 255, 255, 255),
            text: RenderColor::rgba(32, 33, 36, 255),
            selected_background: RenderColor::rgba(
                WECHAT_GREEN_RGB[0],
                WECHAT_GREEN_RGB[1],
                WECHAT_GREEN_RGB[2],
                255,
            ),
            selected_text: RenderColor::rgba(WHITE_RGB[0], WHITE_RGB[1], WHITE_RGB[2], 255),
            comment_color: RenderColor::rgba(120, 120, 120, 255),
            border: RenderColor::rgba(215, 215, 215, 255),
            scrollbar: RenderColor::rgba(128, 128, 128, 180),
            preedit_background: RenderColor::rgba(255, 255, 255, 255),
            preedit_text: RenderColor::rgba(32, 33, 36, 255),
            selection_inflate_x: 2.0,
            selection_inflate_y: 2.0,
            corner_radius: WECHAT_WINDOW_RADIUS_DIP,
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
    /// Whether the horizontal candidate row is a scrolling viewport.
    ///
    /// This is configuration state, not a derived overflow state: a row can
    /// fit today and still need the scroll label column reserved so its text
    /// origin does not jump when the next candidate arrives.
    pub scroll_mode: bool,
    /// Hard total window width used by the measure and paint passes.
    pub max_width: f32,
    /// Outer horizontal padding already included in `max_width`.
    pub window_padding_x: f32,
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
            scroll_mode: false,
            max_width: 0.0,
            window_padding_x: 0.0,
        }
    }
}

/// One candidate's visible strings (label already formatted for display).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateRenderData {
    pub label: String,
    /// Label reserved by the candidate model even when it is hidden.
    pub reserved_label: String,
    pub text: String,
    pub comment: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HorizontalSegment {
    Label,
    Text,
    Comment,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HorizontalColumnInput<'a> {
    pub label: &'a str,
    pub reserved_label: &'a str,
    pub text: &'a str,
    pub comment: &'a str,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct HorizontalColumnAnchors {
    pub label_x: f32,
    pub text_x: f32,
    pub comment_x: f32,
    pub label_width: f32,
    pub text_width: f32,
    pub comment_width: f32,
    /// Effective content width required to keep every non-empty segment on
    /// one row with distinct columns. A configured max width below this
    /// minimum is expanded by the owning window layout rather than turning
    /// the candidate into a vertical stack.
    pub effective_width: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct HorizontalRunLine {
    pub label: String,
    pub text: String,
    pub comment: String,
    pub width: f32,
    pub height: f32,
    pub label_x: f32,
    pub text_x: f32,
    pub comment_x: f32,
    /// Stable label-column width used before candidate text.
    pub label_slot_width: f32,
    label_width: f32,
    text_width: f32,
    comment_width: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct HorizontalRunPlan {
    pub lines: Vec<HorizontalRunLine>,
    pub width: f32,
    pub height: f32,
    pub anchors: HorizontalColumnAnchors,
}

fn line_with_anchors(anchors: HorizontalColumnAnchors) -> HorizontalRunLine {
    HorizontalRunLine {
        label_x: anchors.label_x,
        text_x: anchors.text_x,
        comment_x: anchors.comment_x,
        label_slot_width: anchors.label_width,
        ..HorizontalRunLine::default()
    }
}

/// Return the raw candidate-content budget shared by measurement and paint.
/// `horizontal_column_anchors` consumes the two inter-column gaps from this
/// value; callers must not pass an already measured item width here.
pub(crate) fn horizontal_available_width(
    max_width: f32,
    window_padding_x: f32,
    item_padding_x: f32,
    scroll_mode: bool,
) -> f32 {
    if scroll_mode || !max_width.is_finite() || max_width <= 0.0 {
        return f32::INFINITY;
    }
    (max_width.max(1.0) - window_padding_x.max(0.0) * 2.0 - item_padding_x.max(0.0) * 2.0).max(1.0)
}

/// Compute one set of columns for the entire visible candidate set.
///
/// The returned x values are relative to the candidate content origin. When
/// a finite budget is too small, `effective_width` expands to the minimum
/// viable one-row window; every segment keeps its x anchor and wrapping
/// remains grapheme-safe.
pub(crate) fn horizontal_column_anchors<F>(
    candidates: &[HorizontalColumnInput<'_>],
    reserved_label_width: f32,
    max_width: f32,
    label_gap: f32,
    label_font_size: f32,
    text_font_size: f32,
    comment_font_size: f32,
    mut measure: F,
) -> HorizontalColumnAnchors
where
    F: FnMut(&str, f32) -> (f32, f32),
{
    let mut label_width = reserved_label_width.max(0.0);
    let mut text_width = 0.0_f32;
    let mut comment_width = 0.0_f32;
    for candidate in candidates {
        if !candidate.label.is_empty() {
            label_width = label_width.max(measure(candidate.label, label_font_size).0.max(0.0));
        }
        if !candidate.reserved_label.is_empty() {
            label_width = label_width.max(
                measure(candidate.reserved_label, label_font_size)
                    .0
                    .max(0.0),
            );
        }
        if !candidate.text.is_empty() {
            text_width = text_width.max(measure(candidate.text, text_font_size).0.max(0.0));
        }
        if !candidate.comment.is_empty() {
            comment_width =
                comment_width.max(measure(candidate.comment, comment_font_size).0.max(0.0));
        }
    }
    let gap = label_gap.max(0.0);
    let required_text_width = candidates
        .iter()
        .filter(|candidate| !candidate.text.is_empty())
        .flat_map(|candidate| grapheme_clusters(candidate.text))
        .map(|cluster| measure(cluster, text_font_size).0.max(0.0))
        .reduce(f32::max)
        .unwrap_or(0.0);
    let required_comment_width = candidates
        .iter()
        .filter(|candidate| !candidate.comment.is_empty())
        .flat_map(|candidate| grapheme_clusters(candidate.comment))
        .map(|cluster| measure(cluster, comment_font_size).0.max(0.0))
        .reduce(f32::max)
        .unwrap_or(0.0);
    let effective_width = if max_width.is_finite() {
        // Preserve a single-row column model. If the configured width cannot
        // fit the first grapheme of each populated column, the window must
        // grow to this minimum; stacking segments would change their shared
        // baseline and make the candidate internally inconsistent.
        max_width
            .max(1.0)
            .max(label_width + required_text_width + required_comment_width + gap * 2.0)
    } else {
        f32::INFINITY
    };
    if effective_width.is_finite() {
        let content_budget = (effective_width - gap * 2.0).max(1.0);
        label_width = label_width.min(content_budget);
        let available_after_label = (content_budget - label_width).max(0.0);
        if required_text_width > 0.0 && required_comment_width > 0.0 {
            text_width = text_width
                .min((available_after_label - required_comment_width).max(required_text_width))
                .max(required_text_width);
            comment_width = comment_width
                .min((available_after_label - text_width).max(required_comment_width))
                .max(required_comment_width);
        } else if required_text_width > 0.0 {
            text_width = text_width
                .min(available_after_label)
                .max(required_text_width);
            comment_width = 0.0;
        } else if required_comment_width > 0.0 {
            comment_width = comment_width
                .min(available_after_label)
                .max(required_comment_width);
            text_width = 0.0;
        } else {
            text_width = text_width.min(available_after_label);
            comment_width = comment_width.min((available_after_label - text_width).max(0.0));
        }
    }
    let text_x = label_width + gap;
    let comment_x = text_x + text_width + gap;
    HorizontalColumnAnchors {
        label_x: 0.0,
        text_x,
        comment_x,
        label_width,
        text_width,
        comment_width,
        effective_width,
    }
}

/// Make the exact horizontal lines used by both measurement and painting.
/// Breaks happen only between the repository's grapheme clusters, so a long
/// candidate or comment grows vertically instead of being silently dropped.
#[cfg(test)]
pub(crate) fn horizontal_run_plan<F>(
    label: &str,
    reserved_label_width: f32,
    text: &str,
    comment: &str,
    max_width: f32,
    label_gap: f32,
    label_font_size: f32,
    text_font_size: f32,
    comment_font_size: f32,
    mut measure: F,
) -> HorizontalRunPlan
where
    F: FnMut(&str, f32) -> (f32, f32),
{
    let candidate = [HorizontalColumnInput {
        label,
        reserved_label: "",
        text,
        comment,
    }];
    let anchors = horizontal_column_anchors(
        &candidate,
        reserved_label_width,
        max_width,
        label_gap,
        label_font_size,
        text_font_size,
        comment_font_size,
        &mut measure,
    );
    horizontal_run_plan_with_anchors(
        label,
        text,
        comment,
        max_width,
        label_gap,
        label_font_size,
        text_font_size,
        comment_font_size,
        anchors,
        measure,
    )
}

pub(crate) fn horizontal_run_plan_with_anchors<F>(
    label: &str,
    text: &str,
    comment: &str,
    max_width: f32,
    _label_gap: f32,
    label_font_size: f32,
    text_font_size: f32,
    comment_font_size: f32,
    anchors: HorizontalColumnAnchors,
    mut measure: F,
) -> HorizontalRunPlan
where
    F: FnMut(&str, f32) -> (f32, f32),
{
    let budget = if anchors.effective_width.is_finite() {
        max_width.max(anchors.effective_width).max(1.0)
    } else if max_width.is_finite() {
        max_width.max(1.0)
    } else {
        f32::INFINITY
    };
    let mut lines = Vec::new();
    let mut line = line_with_anchors(anchors);

    let push_cluster = |line: &mut HorizontalRunLine,
                        segment: HorizontalSegment,
                        cluster: &str,
                        font_size: f32,
                        measure: &mut F|
     -> bool {
        let (cluster_width, cluster_height) = measure(cluster, font_size);
        let cluster_width = cluster_width.max(0.0);
        let (segment_x, segment_width, segment_capacity) = match segment {
            HorizontalSegment::Label => {
                (anchors.label_x, &mut line.label_width, anchors.label_width)
            }
            HorizontalSegment::Text => (anchors.text_x, &mut line.text_width, anchors.text_width),
            HorizontalSegment::Comment => (
                anchors.comment_x,
                &mut line.comment_width,
                anchors.comment_width,
            ),
        };
        let right = segment_x + *segment_width + cluster_width;
        let first_cluster = line.width <= f32::EPSILON && *segment_width <= f32::EPSILON;
        let fits_slot = *segment_width + cluster_width <= segment_capacity + 0.001;
        if right > budget + 0.001 {
            return false;
        }
        if !fits_slot && !(first_cluster && right <= budget) {
            return false;
        }
        let target = match segment {
            HorizontalSegment::Label => &mut line.label,
            HorizontalSegment::Text => &mut line.text,
            HorizontalSegment::Comment => &mut line.comment,
        };
        target.push_str(cluster);
        *segment_width += cluster_width;
        line.width = line.width.max(right);
        line.height = line.height.max(cluster_height);
        true
    };

    for (segment, value, font_size) in [
        (HorizontalSegment::Label, label, label_font_size),
        (HorizontalSegment::Text, text, text_font_size),
        (HorizontalSegment::Comment, comment, comment_font_size),
    ] {
        let clusters = grapheme_clusters(value);
        if clusters.is_empty() {
            continue;
        }
        for cluster in clusters {
            if !push_cluster(&mut line, segment, cluster, font_size, &mut measure) {
                if line.width > 0.0 {
                    lines.push(line);
                    line = line_with_anchors(anchors);
                }
                // The effective inline width reserves the first grapheme of
                // every populated column, so retrying on a fresh line keeps
                // the same x anchor without dropping or overflowing it.
                let pushed = push_cluster(&mut line, segment, cluster, font_size, &mut measure);
                debug_assert!(
                    pushed,
                    "inline fallback overflow: segment={segment:?} cluster={cluster:?} anchors={anchors:?} budget={budget}"
                );
            }
        }
    }
    if line.width > 0.0 {
        lines.push(line);
    }
    let width = lines.iter().map(|line| line.width).fold(0.0, f32::max);
    let height = lines.iter().map(|line| line.height).sum();
    HorizontalRunPlan {
        lines,
        width,
        height,
        anchors,
    }
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
    /// Candidate font family selected by the resolved Config snapshot.
    pub font_family: &'a str,
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

#[derive(Default)]
struct TextMeasurementCache {
    // DirectWrite layout creation is the expensive part of a paint. Keep a
    // tiny per-frame cache keyed by normalized font size; the active render
    // has one family/style, so the text is the remaining key.
    by_size: HashMap<u32, Vec<(String, (f32, f32))>>,
}

impl TextMeasurementCache {
    fn measure(
        &mut self,
        canvas: &mut SkiaCanvas<'_>,
        family: &str,
        value: &str,
        size: f32,
    ) -> (f32, f32) {
        let size = size.max(1.0);
        let bucket = self.by_size.entry(size.to_bits()).or_default();
        if let Some((_, measured)) = bucket.iter().find(|(cached, _)| cached == value) {
            return *measured;
        }
        let style = text_style(family, size);
        let measured = canvas.measure_text(value, &style);
        let measured = (measured.w.max(0) as f32, measured.h.max(0) as f32);
        bucket.push((value.to_owned(), measured));
        measured
    }
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
    let (window_w, window_h, width, height) = render_pixel_extent(
        axis,
        input.preedit,
        input.geometry.preedit_height,
        input.dpi_scale,
    );
    if width == 0 || height == 0 {
        return RenderWindowOutput {
            pixels: Vec::new(),
            width,
            height,
            stride: 0,
        };
    }
    let stride = width.saturating_mul(4);
    let Some(mut pixmap) = Pixmap::new(width, height) else {
        return RenderWindowOutput {
            pixels: Vec::new(),
            width,
            height,
            stride,
        };
    };
    let scale = input.dpi_scale.clamp(0.25, 8.0);
    let mut engine = DWriteEngine::new();
    engine.set_scale(scale);
    let mut canvas = SkiaCanvas::with_text(&mut pixmap, &mut engine, scale);
    let mut measurement_cache = TextMeasurementCache::default();

    // 1. Background.
    canvas.fill_rect(
        0.0,
        0.0,
        window_w,
        window_h,
        &WindPaint::fill(input.theme.background.wind()),
    );

    // 2. Preedit row (text + divider; high-contrast fills the row background).
    let preedit_offset = if let Some(preedit) = input.preedit.filter(|text| !text.is_empty()) {
        let row_height = input.geometry.preedit_height.max(8.0).min(window_h);
        draw_preedit(&mut canvas, input, window_w, window_h, preedit);
        row_height
    } else {
        0.0
    };

    // 3. Selection rect under the selected row's text.
    // When preedit is present, offset all candidate rows below the preedit panel.
    let y_offset = preedit_offset;
    let scroll_label_slot_width = if input.geometry.scroll_mode {
        horizontal_scroll_label_slot_width(&input.candidates, |value| {
            measurement_cache
                .measure(
                    &mut canvas,
                    input.font_family,
                    value,
                    input.geometry.label_font_size,
                )
                .0
        })
    } else {
        0.0
    };
    let horizontal_inputs: Vec<_> = input
        .candidates
        .iter()
        .map(|candidate| HorizontalColumnInput {
            label: &candidate.label,
            reserved_label: &candidate.reserved_label,
            text: &candidate.text,
            comment: &candidate.comment,
        })
        .collect();
    let horizontal_budget = horizontal_available_width(
        input.geometry.max_width,
        input.geometry.window_padding_x,
        input.geometry.item_padding_x,
        input.geometry.scroll_mode,
    );
    let horizontal_anchors = horizontal_column_anchors(
        &horizontal_inputs,
        scroll_label_slot_width,
        horizontal_budget,
        input.geometry.label_gap,
        input.geometry.label_font_size,
        input.geometry.font_size,
        input.geometry.comment_font_size,
        |value, size| measurement_cache.measure(&mut canvas, input.font_family, value, size),
    );
    if let Some(selected) = input.selected {
        if let Some(item) = axis.items.get(selected).filter(|item| item.visible) {
            draw_selection(&mut canvas, input, window_w, window_h, item, y_offset);
        }
    }

    // 4. Candidate rows.
    for (index, item) in axis.items.iter().enumerate() {
        if !item.visible {
            continue;
        }
        let item_left = item.rect.left - axis.window.left;
        let item_top = item.rect.top - axis.window.top + y_offset;
        let item_right = item.rect.right - axis.window.left;
        let item_bottom = item.rect.bottom - axis.window.top + y_offset;
        if item_right <= 0.0 || item_left >= window_w || item_bottom <= 0.0 || item_top >= window_h
        {
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
                y_offset,
                horizontal_anchors,
                &mut measurement_cache,
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
                y_offset,
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

/// Return the exact logical and physical bitmap extent used by the renderer.
/// The ABI uses this for its size query so a query never performs a paint.
pub(crate) fn render_pixel_extent(
    axis: &AxisLayoutResult,
    preedit: Option<&str>,
    preedit_height: f32,
    dpi_scale: f32,
) -> (f32, f32, u32, u32) {
    let window_w = (axis.window.right - axis.window.left).ceil().max(0.0);
    let mut window_h = (axis.window.bottom - axis.window.top).ceil().max(0.0);
    if preedit.is_some_and(|text| !text.is_empty()) {
        window_h += preedit_height.max(8.0);
    }
    if window_w < 1.0 || window_h < 1.0 {
        return (window_w, window_h, window_w as u32, window_h as u32);
    }
    let scale = dpi_scale.clamp(0.25, 8.0);
    (
        window_w,
        window_h,
        (window_w * scale).ceil() as u32,
        (window_h * scale).ceil() as u32,
    )
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
        &text_style(input.font_family, geometry.font_size),
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
    y_offset: f32,
) {
    let origin = &input.axis_result.window;
    let inflate_x = input.theme.selection_inflate_x.max(0.0);
    let inflate_y = input.theme.selection_inflate_y.max(0.0);
    let x = (item.rect.left - origin.left - inflate_x).max(0.0);
    let y = (item.rect.top - origin.top - inflate_y + y_offset).max(0.0);
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
            selection_corner_radius(input.theme.corner_radius, w, h),
            &WindPaint::fill(input.theme.selected_background.wind()),
        );
    }
}

fn selection_corner_radius(window_radius: f32, width: f32, height: f32) -> f32 {
    WECHAT_SELECTION_RADIUS_DIP
        .min((window_radius - 2.0).max(0.0))
        .min(width.min(height) / 2.0)
}

fn draw_candidate_horizontal(
    canvas: &mut SkiaCanvas<'_>,
    input: &RenderWindowInput<'_>,
    window_w: f32,
    window_h: f32,
    item: &AxisLayoutItem,
    candidate: &CandidateRenderData,
    selected: bool,
    _y_offset: f32,
    horizontal_anchors: HorizontalColumnAnchors,
    measurement_cache: &mut TextMeasurementCache,
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
    let style_text = text_style(input.font_family, geometry.font_size);
    let style_label = text_style(input.font_family, geometry.label_font_size);
    let style_comment = text_style(input.font_family, geometry.comment_font_size);

    // Scrolling keeps its natural item width and viewport semantics. Paging
    // and wrapping use the visible content width, which is also the budget
    // used by measure_ffi::measure_visual_items.
    let natural_content_width = (item.rect.right - item.rect.left - pad_x * 2.0).max(1.0);
    let available_width = if geometry.scroll_mode {
        natural_content_width
    } else {
        horizontal_available_width(
            geometry.max_width,
            geometry.window_padding_x,
            geometry.item_padding_x,
            false,
        )
    };
    let plan = horizontal_run_plan_with_anchors(
        &candidate.label,
        &candidate.text,
        &candidate.comment,
        available_width,
        label_gap,
        geometry.label_font_size,
        geometry.font_size,
        geometry.comment_font_size,
        horizontal_anchors,
        |value, size| measurement_cache.measure(canvas, input.font_family, value, size),
    );
    let text_color = if selected {
        input.theme.selected_text.wind()
    } else {
        input.theme.text.wind()
    };
    let mut line_top = row_top;
    for line in &plan.lines {
        if line_top >= row_bottom {
            break;
        }
        let line_height = line.height.max(1.0);
        let line_bottom = (line_top + line_height).min(row_bottom);
        let mut baseline = line_top;
        for (value, style) in [
            (&line.label, &style_label),
            (&line.text, &style_text),
            (&line.comment, &style_comment),
        ] {
            if !value.is_empty() {
                baseline = baseline.max(line_top + canvas.text_line_metrics(value, style).ascent);
            }
        }
        if line.label_slot_width > 0.0 && !line.label.is_empty() {
            let label_w = measurement_cache
                .measure(
                    canvas,
                    input.font_family,
                    &line.label,
                    geometry.label_font_size,
                )
                .0;
            let label_left = (content_left + line.label_x).min(content_right);
            let right = (label_left + label_w).min(content_right);
            if right > label_left {
                let rect = horizontal_segment_rect(
                    canvas,
                    &line.label,
                    &style_label,
                    label_left,
                    right,
                    baseline,
                    line_bottom,
                );
                draw_text_clipped(
                    canvas,
                    &line.label,
                    rect,
                    text_color,
                    WindAlign::Start,
                    &style_label,
                );
            }
        }
        if !line.text.is_empty() {
            let text_w = measurement_cache
                .measure(canvas, input.font_family, &line.text, geometry.font_size)
                .0;
            let text_left = (content_left + line.text_x).min(content_right);
            let text_right = if !line.comment.is_empty() {
                (content_left + line.comment_x - label_gap).min(content_right)
            } else {
                content_right
            };
            let right = (text_left + text_w).min(text_right.max(text_left));
            if right > text_left {
                let rect = horizontal_segment_rect(
                    canvas,
                    &line.text,
                    &style_text,
                    text_left,
                    right,
                    baseline,
                    line_bottom,
                );
                draw_text_clipped(
                    canvas,
                    &line.text,
                    rect,
                    text_color,
                    WindAlign::Start,
                    &style_text,
                );
            }
        }
        if !line.comment.is_empty() {
            let comment_left = (content_left + line.comment_x).min(content_right);
            if comment_left < content_right {
                let rect = horizontal_segment_rect(
                    canvas,
                    &line.comment,
                    &style_comment,
                    comment_left,
                    content_right,
                    baseline,
                    line_bottom,
                );
                draw_text_clipped(
                    canvas,
                    &line.comment,
                    rect,
                    input.theme.comment_color.wind(),
                    WindAlign::Start,
                    &style_comment,
                );
            }
        }
        line_top += line_height;
    }
}

fn horizontal_segment_rect(
    canvas: &mut SkiaCanvas<'_>,
    text: &str,
    style: &WindTextStyle<'_>,
    left: f32,
    right: f32,
    baseline: f32,
    row_bottom: f32,
) -> Rect {
    let metrics = canvas.text_line_metrics(text, style);
    Rect {
        left,
        top: (baseline - metrics.ascent).max(0.0),
        right,
        bottom: (baseline + metrics.descent).min(row_bottom),
    }
}

fn horizontal_scroll_label_slot_width<F>(candidates: &[CandidateRenderData], mut measure: F) -> f32
where
    F: FnMut(&str) -> f32,
{
    candidates
        .iter()
        .map(|candidate| measure(&candidate.reserved_label).max(0.0))
        .fold(0.0, f32::max)
}

/// Shared vertical glyph-row advance, expressed as a ratio of the candidate
/// font size. The measure loop and the vertical renderer must use this exact
/// step so the measured column height always covers every drawn glyph.
pub(crate) const VERTICAL_GLYPH_STEP_RATIO: f32 = 1.6;

/// Splits `text` into the clusters the vertical renderer stacks one per row.
/// A new cluster starts at every char that is not a continuation: combining
/// marks, variation selectors, skin-tone modifiers, the keycap, ZWJ (and the
/// char right after it) all attach to the previous cluster; regional-indicator
/// flags pair two at a time. This keeps an emoji or flag on one row without a
/// Unicode segmentation dependency.
pub(crate) fn grapheme_clusters(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    fn is_extend(ch: char) -> bool {
        matches!(
            ch,
            '\u{20E3}' // keycap (also inside the combining range below)
                | '\u{0300}'..='\u{036F}'
                | '\u{1AB0}'..='\u{1AFF}'
                | '\u{20D0}'..='\u{20FF}'
                | '\u{FE20}'..='\u{FE2F}'
                | '\u{FE00}'..='\u{FE0F}'
                | '\u{E0100}'..='\u{E01EF}'
                | '\u{1F3FB}'..='\u{1F3FF}'
        )
    }
    fn is_regional(ch: char) -> bool {
        ('\u{1F1E6}'..='\u{1F1FF}').contains(&ch)
    }

    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut clusters = Vec::new();
    let mut start = 0usize;
    let mut regional_run = 0usize;
    for i in 0..chars.len() {
        let (byte, ch) = chars[i];
        let new_cluster = i != 0
            && !(is_extend(ch)
                || ch == '\u{200D}'
                || chars[i - 1].1 == '\u{200D}'
                || (is_regional(chars[i - 1].1) && is_regional(ch) && regional_run % 2 == 1));
        if is_regional(ch) {
            regional_run = if i > 0 && is_regional(chars[i - 1].1) {
                regional_run + 1
            } else {
                1
            };
        } else {
            regional_run = 0;
        }
        if new_cluster {
            clusters.push(&text[start..byte]);
            start = byte;
        }
    }
    clusters.push(&text[start..]);
    clusters
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
    y_offset: f32,
) {
    let origin = &input.axis_result.window;
    let left = (item.rect.left - origin.left).clamp(0.0, window_w);
    let top = (item.rect.top - origin.top + y_offset).clamp(0.0, window_h);
    let right = (item.rect.right - origin.left).clamp(left, window_w);
    let bottom = (item.rect.bottom - origin.top + y_offset).clamp(top, window_h);
    let geometry = input.geometry;
    let pad_x = geometry.item_padding_x.max(0.0);
    let color = if selected {
        input.theme.selected_text.wind()
    } else {
        input.theme.text.wind()
    };
    let glyph_step = (geometry.font_size * VERTICAL_GLYPH_STEP_RATIO)
        .ceil()
        .max(1.0);
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
                    &text_style(input.font_family, size),
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
    for glyph in grapheme_clusters(&candidate.text) {
        if y >= bottom {
            break;
        }
        cell_text(
            glyph,
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

fn text_style<'a>(family: &'a str, size: f32) -> WindTextStyle<'a> {
    WindTextStyle {
        family: (!family.trim().is_empty()).then_some(family),
        size: size.max(1.0),
        weight: 400,
        italic: false,
        line_height: None,
    }
}

/// Owns the windui DirectWrite engine for host-side text measurement.
pub struct MeasureEngine {
    engine: DWriteEngine,
    cache: HashMap<(String, String, u32, u32), (f32, f32)>,
}

impl MeasureEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: DWriteEngine::new(),
            cache: HashMap::new(),
        }
    }

    /// Logical-DIP width/height of a single-line UTF-8 run.
    #[must_use]
    pub fn measure(&mut self, text: &str, font_size: f32, dpi_scale: f32) -> (f32, f32) {
        self.measure_with_family(text, "Microsoft YaHei UI", font_size, dpi_scale)
    }

    /// Logical-DIP width/height with the resolved candidate font family.
    #[must_use]
    pub fn measure_with_family(
        &mut self,
        text: &str,
        family: &str,
        font_size: f32,
        dpi_scale: f32,
    ) -> (f32, f32) {
        let size = font_size.max(1.0);
        let key = (
            text.to_owned(),
            family.to_owned(),
            size.to_bits(),
            dpi_scale.to_bits(),
        );
        if let Some(measured) = self.cache.get(&key) {
            return *measured;
        }
        self.engine.set_scale(dpi_scale);
        let style = text_style(family, size);
        let size = TextEngine::measure(&mut self.engine, text, &style, None);
        let measured = (size.w as f32, size.h as f32);
        self.cache.insert(key, measured);
        measured
    }
}

impl Default for MeasureEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Measured logical-DIP size of one text run.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateMeasureSize {
    pub width: f32,
    pub height: f32,
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
    const GREEN: RenderColor = RenderColor::rgba(255, 255, 255, 255);
    const GREEN_BG: RenderColor = RenderColor::rgba(7, 193, 96, 255);

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
                reserved_label: "1".to_owned(),
                text: "你".to_owned(),
                comment: "nǐ".to_owned(),
            },
            CandidateRenderData {
                label: "2".to_owned(),
                reserved_label: "2".to_owned(),
                text: "好".to_owned(),
                comment: "hǎo".to_owned(),
            },
            CandidateRenderData {
                label: "3".to_owned(),
                reserved_label: "3".to_owned(),
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
            scroll_mode: false,
            max_width: 0.0,
            window_padding_x: 0.0,
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
            corner_radius: WECHAT_WINDOW_RADIUS_DIP,
        }
    }

    #[test]
    fn default_theme_uses_project_wechat_selection_tokens() {
        let theme = RenderTheme::default();
        assert_eq!(theme.selected_background, GREEN_BG);
        assert_eq!(theme.selected_text, GREEN);
        assert_eq!(theme.corner_radius, WECHAT_WINDOW_RADIUS_DIP);
    }

    fn anchor_measure(value: &str, size: f32) -> (f32, f32) {
        (grapheme_clusters(value).len() as f32 * size, size)
    }

    #[test]
    fn horizontal_columns_are_stable_across_candidate_lengths() {
        let candidates = vec![
            HorizontalColumnInput {
                label: "1.",
                reserved_label: "1.",
                text: "你",
                comment: "注",
            },
            HorizontalColumnInput {
                label: "10.",
                reserved_label: "10.",
                text: "你好",
                comment: "注释",
            },
            HorizontalColumnInput {
                label: "100.",
                reserved_label: "100.",
                text: "这是一个明显更长的候选正文",
                comment: "这是一段明显更长的注释",
            },
        ];
        let anchors = horizontal_column_anchors(
            &candidates,
            0.0,
            f32::INFINITY,
            4.0,
            15.3,
            18.0,
            14.4,
            anchor_measure,
        );
        let mut reordered = candidates.clone();
        reordered.reverse();
        assert_eq!(
            anchors,
            horizontal_column_anchors(
                &reordered,
                0.0,
                f32::INFINITY,
                4.0,
                15.3,
                18.0,
                14.4,
                anchor_measure,
            )
        );
        assert!(anchors.label_x < anchors.text_x && anchors.text_x < anchors.comment_x);
    }

    #[test]
    fn horizontal_wrapping_preserves_segment_column_anchors() {
        let candidate = HorizontalColumnInput {
            label: "100.",
            reserved_label: "100.",
            text: "候选文字候选文字😀👨‍👩‍👧‍👦",
            comment: "注释信息注释信息这是一个很长的注释",
        };
        let anchors = horizontal_column_anchors(
            &[candidate],
            0.0,
            160.0,
            4.0,
            15.3,
            18.0,
            14.4,
            anchor_measure,
        );
        let plan = horizontal_run_plan_with_anchors(
            candidate.label,
            candidate.text,
            candidate.comment,
            160.0,
            4.0,
            15.3,
            18.0,
            14.4,
            anchors,
            anchor_measure,
        );
        assert!(plan.lines.len() > 1, "fixture must wrap: {plan:?}");
        assert_eq!(
            plan.lines
                .iter()
                .map(|line| line.label.as_str())
                .collect::<String>(),
            candidate.label
        );
        assert_eq!(
            plan.lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<String>(),
            candidate.text
        );
        assert_eq!(
            plan.lines
                .iter()
                .map(|line| line.comment.as_str())
                .collect::<String>(),
            candidate.comment
        );
        for line in &plan.lines {
            if !line.label.is_empty() {
                assert_eq!(line.label_x, anchors.label_x);
            }
            if !line.text.is_empty() {
                assert_eq!(line.text_x, anchors.text_x);
            }
            if !line.comment.is_empty() {
                assert_eq!(line.comment_x, anchors.comment_x);
            }
        }
    }

    #[test]
    fn tiny_width_expands_inline_columns_without_dropping_candidate_text() {
        let candidate = HorizontalColumnInput {
            label: "100.",
            reserved_label: "100.",
            text: "你好",
            comment: "这是一个非常非常长的候选注释 annotation",
        };
        let assert_case = |budget: f32| {
            let anchors = horizontal_column_anchors(
                &[candidate],
                0.0,
                budget,
                4.0,
                15.3,
                18.0,
                14.4,
                anchor_measure,
            );
            let plan = horizontal_run_plan_with_anchors(
                candidate.label,
                candidate.text,
                candidate.comment,
                budget,
                4.0,
                15.3,
                18.0,
                14.4,
                anchors,
                anchor_measure,
            );
            assert!(
                anchors.text_width > 0.0,
                "text needs a drawable slot: {anchors:?}"
            );
            assert!(
                anchors.effective_width >= budget,
                "effective width must not shrink the requested budget: {anchors:?}"
            );
            assert!(anchors.text_x > anchors.label_x);
            assert!(anchors.comment_x > anchors.text_x);
            assert_eq!(
                plan.lines
                    .iter()
                    .map(|line| line.text.as_str())
                    .collect::<String>(),
                candidate.text
            );
            assert_eq!(
                plan.lines
                    .iter()
                    .map(|line| line.comment.as_str())
                    .collect::<String>(),
                candidate.comment
            );
            for line in &plan.lines {
                assert!(
                    line.width <= anchors.effective_width + 0.001,
                    "line exceeds budget: {line:?}"
                );
                if !line.text.is_empty() {
                    assert!(line.text_width > 0.0);
                }
                if !line.comment.is_empty() {
                    assert!(line.comment_width > 0.0);
                }
                if !line.text.is_empty() && !line.comment.is_empty() {
                    assert!(line.text_x + line.text_width <= line.comment_x + 0.001);
                }
            }
        };

        // Positive text slot: normal inline columns remain unchanged.
        assert_case(140.0);
        // Former zero-slot input: expand the minimum viable one-row window
        // instead of stacking segments or allowing a grapheme to overflow.
        assert_case(80.0);
    }

    fn first_dark_pixel(
        output: &RenderWindowOutput,
        x_start: u32,
        x_end: u32,
        y_start: u32,
        y_end: u32,
    ) -> Option<u32> {
        (x_start..x_end.min(output.width)).find(|x| {
            (y_start..y_end.min(output.height)).any(|y| {
                let Some([blue, green, red, _]) = output.pixel(*x, y) else {
                    return false;
                };
                red < 180 && green < 180 && blue < 180
            })
        })
    }

    fn last_dark_pixel_y(
        output: &RenderWindowOutput,
        x_start: u32,
        x_end: u32,
        y_start: u32,
        y_end: u32,
    ) -> Option<u32> {
        (y_start..y_end.min(output.height)).rev().find(|y| {
            (x_start..x_end.min(output.width)).any(|x| {
                let Some([blue, green, red, _]) = output.pixel(x, *y) else {
                    return false;
                };
                red < 180 && green < 180 && blue < 180
            })
        })
    }

    #[test]
    fn rendered_horizontal_columns_do_not_drift() {
        let axis = axis_layout(&vertical_input(&[(420.0, 120.0); 3]));
        let candidates = vec![
            CandidateRenderData {
                label: "1.".to_owned(),
                reserved_label: "1.".to_owned(),
                text: "A".to_owned(),
                comment: "C".to_owned(),
            },
            CandidateRenderData {
                label: "10.".to_owned(),
                reserved_label: "10.".to_owned(),
                text: "AB".to_owned(),
                comment: "CC".to_owned(),
            },
            CandidateRenderData {
                label: "100.".to_owned(),
                reserved_label: "100.".to_owned(),
                text: "A long candidate".to_owned(),
                comment: "C long annotation".to_owned(),
            },
        ];
        let geometry = geometry();
        let output = render_candidate_window(&RenderWindowInput {
            axis_result: &axis,
            candidates: &candidates,
            theme: &theme(),
            geometry: &geometry,
            font_family: "Microsoft YaHei UI",
            preedit: None,
            dpi_scale: 1.0,
            high_contrast: false,
            selected: None,
        });
        let mut measure_pixmap = Pixmap::new(1, 1).expect("measurement pixmap");
        let mut measure_engine = DWriteEngine::new();
        let mut measure_canvas =
            SkiaCanvas::with_text(&mut measure_pixmap, &mut measure_engine, 1.0);
        let mut measurement_cache = TextMeasurementCache::default();
        let anchor_inputs: Vec<_> = candidates
            .iter()
            .map(|candidate| HorizontalColumnInput {
                label: &candidate.label,
                reserved_label: &candidate.reserved_label,
                text: &candidate.text,
                comment: &candidate.comment,
            })
            .collect();
        let anchors = horizontal_column_anchors(
            &anchor_inputs,
            0.0,
            404.0,
            geometry.label_gap,
            geometry.label_font_size,
            geometry.font_size,
            geometry.comment_font_size,
            |value, size| {
                measurement_cache.measure(&mut measure_canvas, "Microsoft YaHei UI", value, size)
            },
        );
        let mut columns = [Vec::new(), Vec::new(), Vec::new()];
        for item in &axis.items {
            let content_left =
                (item.rect.left - axis.window.left) as u32 + geometry.item_padding_x as u32;
            let top = (item.rect.top - axis.window.top) as u32 + geometry.item_padding_y as u32;
            let bottom = (item.rect.bottom - axis.window.top) as u32;
            let expected = [
                content_left + anchors.label_x as u32,
                content_left + anchors.text_x as u32,
                content_left + anchors.comment_x as u32,
            ];
            columns[0].push(
                first_dark_pixel(
                    &output,
                    expected[0].saturating_sub(2),
                    expected[0] + 30,
                    top,
                    bottom,
                )
                .expect("label glyph must be visible"),
            );
            columns[1].push(
                first_dark_pixel(
                    &output,
                    expected[1].saturating_sub(2),
                    expected[1] + 30,
                    top,
                    bottom,
                )
                .expect("text glyph must be visible"),
            );
            columns[2].push(
                first_dark_pixel(
                    &output,
                    expected[2].saturating_sub(2),
                    expected[2] + 30,
                    top,
                    bottom,
                )
                .expect("comment glyph must be visible"),
            );
        }
        for column in columns {
            let min = *column.iter().min().unwrap();
            let max = *column.iter().max().unwrap();
            assert!(max - min <= 2, "column drifted: {column:?}");
        }
    }

    #[test]
    fn rendered_horizontal_segments_share_visual_baseline() {
        let axis = axis_layout(&vertical_input(&[(420.0, 120.0)]));
        let candidates = vec![CandidateRenderData {
            label: "g".to_owned(),
            reserved_label: "g".to_owned(),
            text: "g".to_owned(),
            comment: "g".to_owned(),
        }];
        let geometry = RenderGeometry {
            font_size: 24.0,
            label_font_size: 18.0,
            comment_font_size: 15.0,
            ..geometry()
        };
        let output = render_candidate_window(&RenderWindowInput {
            axis_result: &axis,
            candidates: &candidates,
            theme: &theme(),
            geometry: &geometry,
            font_family: "Microsoft YaHei UI",
            preedit: None,
            dpi_scale: 1.0,
            high_contrast: false,
            selected: None,
        });
        let mut measure_pixmap = Pixmap::new(1, 1).expect("measurement pixmap");
        let mut measure_engine = DWriteEngine::new();
        let mut measure_canvas =
            SkiaCanvas::with_text(&mut measure_pixmap, &mut measure_engine, 1.0);
        let mut measurement_cache = TextMeasurementCache::default();
        let anchors = horizontal_column_anchors(
            &[HorizontalColumnInput {
                label: "g",
                reserved_label: "g",
                text: "g",
                comment: "g",
            }],
            0.0,
            f32::INFINITY,
            geometry.label_gap,
            geometry.label_font_size,
            geometry.font_size,
            geometry.comment_font_size,
            |value, size| {
                measurement_cache.measure(&mut measure_canvas, "Microsoft YaHei UI", value, size)
            },
        );
        let item = &axis.items[0];
        let content_left =
            (item.rect.left - axis.window.left) as u32 + geometry.item_padding_x as u32;
        let top = (item.rect.top - axis.window.top) as u32 + geometry.item_padding_y as u32;
        let bottom = (item.rect.bottom - axis.window.top) as u32;
        let styles = [
            text_style("Microsoft YaHei UI", geometry.label_font_size),
            text_style("Microsoft YaHei UI", geometry.font_size),
            text_style("Microsoft YaHei UI", geometry.comment_font_size),
        ];
        let expected_baseline = top as f32
            + styles
                .iter()
                .map(|style| measure_canvas.text_line_metrics("g", style).ascent)
                .fold(0.0, f32::max);
        let bottoms = [anchors.label_x, anchors.text_x, anchors.comment_x].map(|x| {
            last_dark_pixel_y(
                &output,
                content_left + x as u32,
                content_left + x as u32 + 30,
                top,
                bottom,
            )
            .expect("baseline glyph must be visible")
        });
        let min = *bottoms.iter().min().expect("three segment pixels");
        let max = *bottoms.iter().max().expect("three segment pixels");
        assert!(expected_baseline > top as f32);
        assert!(
            bottoms
                .iter()
                .all(|bottom| (*bottom as f32 - expected_baseline).abs() <= 8.0),
            "glyph pixels disagree with DWrite baseline metrics: {bottoms:?}, expected={expected_baseline}"
        );
        assert!(max - min <= 2, "visual baselines drifted: {bottoms:?}");
    }

    #[test]
    fn long_horizontal_candidate_and_comment_wrap_without_loss() {
        let label = "1.";
        let text = "候选文字候选文字";
        let comment = "注释信息注释信息";
        let plan = horizontal_run_plan(
            label,
            0.0,
            text,
            comment,
            240.0,
            4.0,
            14.0,
            18.0,
            14.0,
            |value, size| (grapheme_clusters(value).len() as f32 * size, size),
        );
        assert!(plan.lines.len() > 1, "long runs must grow vertically");
        assert!(plan.width <= 240.0, "bounded plan: {plan:?}");
        assert_eq!(
            plan.lines
                .iter()
                .map(|line| line.label.as_str())
                .collect::<String>(),
            label
        );
        assert_eq!(
            plan.lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<String>(),
            text
        );
        assert_eq!(
            plan.lines
                .iter()
                .map(|line| line.comment.as_str())
                .collect::<String>(),
            comment
        );
        assert!(plan.height > 18.0, "wrapped item must grow: {plan:?}");
    }

    #[test]
    fn hidden_scroll_label_reservation_keeps_text_origin_stable() {
        let measure =
            |value: &str, _size: f32| (grapheme_clusters(value).len() as f32 * 10.0, 18.0);
        let shown = horizontal_run_plan(
            "1.",
            30.0,
            "候选",
            "",
            f32::INFINITY,
            4.0,
            14.0,
            18.0,
            14.0,
            measure,
        );
        let hidden = horizontal_run_plan(
            "",
            30.0,
            "候选",
            "",
            f32::INFINITY,
            4.0,
            14.0,
            18.0,
            14.0,
            measure,
        );
        assert_eq!(shown.lines[0].label_slot_width, 30.0);
        assert_eq!(hidden.lines[0].label_slot_width, 30.0);
        assert_eq!(shown.width, hidden.width);
        assert!(hidden.lines[0].label.is_empty());
    }

    #[test]
    fn fitted_horizontal_scroll_row_uses_configured_label_reservation() {
        let mut input = vertical_input(&[(120.0, 30.0); 2]);
        input.options.orientation = crate::Orientation::Horizontal;
        input.options.overflow = OverflowBehavior::Scrolling;
        input.max_width = 400.0;
        let axis = axis_layout(&input);
        assert!(
            axis.viewport_offset.is_none(),
            "the regression case must fit without a derived viewport offset"
        );

        let mut geometry = geometry();
        geometry.scroll_mode = true;
        let output = render_candidate_window(&RenderWindowInput {
            axis_result: &axis,
            candidates: &candidates()[..2],
            theme: &theme(),
            geometry: &geometry,
            font_family: "Microsoft YaHei UI",
            preedit: None,
            dpi_scale: 1.0,
            high_contrast: false,
            selected: None,
        });
        assert!(!output.pixels.is_empty());
        assert_eq!(
            horizontal_scroll_label_slot_width(&candidates(), |value| {
                grapheme_clusters(value).len() as f32 * 16.0
            }),
            16.0
        );
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
            font_family: "Microsoft YaHei UI",
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
        let selection_top = ((rect.top - axis.window.top) - 2.0).max(0.0) as u32;
        let selection_left = ((rect.left - axis.window.left) - 2.0).max(0.0) as u32;
        let selection_corner = output.pixel(selection_left, selection_top).unwrap();
        assert!(
            selection_corner[0] >= 245
                && selection_corner[1] >= 245
                && selection_corner[2] >= 245,
            "the selected candidate has a rounded pill corner, not a square fill (got BGRA {selection_corner:?})"
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
            font_family: "Microsoft YaHei UI",
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
            font_family: "Microsoft YaHei UI",
            preedit: None,
            dpi_scale: 1.0,
            high_contrast: false,
            selected: None,
        });
        assert!(output.pixels.is_empty());
        assert_eq!(output.width, 0);
        assert_eq!(output.stride, 0);
    }

    #[test]
    fn measure_engine_reports_nonzero_cjk_metrics() {
        let mut engine = MeasureEngine::new();
        let (width, height) = engine.measure("\u{4f60}\u{597d}\u{5417}", 14.0, 1.0);
        assert!(
            width > 0.0 && height > 0.0,
            "CJK run must measure {width}x{height}"
        );
        let scaled = {
            let (w, h) = engine.measure("\u{4f60}\u{597d}\u{5417}", 14.0, 2.0);
            (w, h)
        };
        assert!(
            scaled.0 >= width && scaled.1 >= height,
            "higher DPI scale must not shrink CJK metrics"
        );
    }

    #[test]
    fn zwj_family_is_one_vertical_grapheme_cluster() {
        assert_eq!(grapheme_clusters("👨‍👩‍👧‍👦"), vec!["👨‍👩‍👧‍👦"]);
    }
}
