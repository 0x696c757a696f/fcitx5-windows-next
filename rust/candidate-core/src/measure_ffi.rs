//! C ABI for host-side text measurement (081C slice).
//!
//! This module is the only unsafe boundary for the measurement engine; the
//! engine itself is safe Rust in `renderer::MeasureEngine`.

#![forbid(unsafe_code)]

use crate::renderer::{
    grapheme_clusters, horizontal_available_width, horizontal_candidate_anchors,
    horizontal_run_plan_with_anchors, HorizontalColumnInput, MeasureEngine,
    VERTICAL_GLYPH_STEP_RATIO,
};
use crate::{Fcitx5CandidateLayoutSize, Fcitx5CandidateVisualBuildOutput};

/// Parameters for one measure-loop run (frozen from the C++ `update()`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct MeasureLoopParams {
    pub horizontal: bool,
    pub vertical: bool,
    pub scroll_mode: bool,
    pub label_gap: f32,
    pub item_padding_x: f32,
    pub item_padding_y: f32,
    pub font_size: f32,
    pub label_font_size: f32,
    pub comment_font_size: f32,
    pub dpi_scale: f32,
    /// Hard total window width. Zero preserves the unbounded ABI helper.
    pub max_width: f32,
    /// Outer window padding included in `max_width`.
    pub window_padding_x: f32,
    /// Hard total window height for vertical text. Zero keeps the legacy
    /// unbounded helper behavior.
    pub max_height: f32,
    /// Outer vertical window padding included in `max_height`.
    pub window_padding_y: f32,
    /// Candidates this many positions apart share label/text/comment
    /// columns; see `renderer::horizontal_candidate_anchors`.
    pub anchor_stride: usize,
}

/// The frozen measure loop over live arena outputs: optional scroll-label
/// column width, per-item reserved/text/comment runs, and the padded preedit
/// panel. Returns `(items, preedit_panel, scroll_label_column_width)` or
/// The fourth return value is the shared horizontal effective content width
/// required by the renderer. `None` is returned when any render index is out
/// of range.
pub(crate) fn measure_visual_items(
    engine: &mut MeasureEngine,
    outputs: &[Fcitx5CandidateVisualBuildOutput],
    indices: &[usize],
    preedit: Option<&str>,
    params: &MeasureLoopParams,
) -> Option<(
    Vec<Fcitx5CandidateLayoutSize>,
    Fcitx5CandidateLayoutSize,
    f32,
    f32,
)> {
    if indices.iter().any(|index| *index >= outputs.len()) {
        return None;
    }
    let fields: Vec<(String, String, String, String)> = indices
        .iter()
        .map(|index| {
            let output = &outputs[*index];
            (
                output.label.clone(),
                output.reserved_label.clone(),
                output.text.clone(),
                output.comment.clone(),
            )
        })
        .collect();
    let column_inputs: Vec<_> = fields
        .iter()
        .map(
            |(label, reserved_label, text, comment)| HorizontalColumnInput {
                label,
                reserved_label,
                text,
                comment,
            },
        )
        .collect();
    let mut scroll_label_column_width = 0.0_f32;
    if params.scroll_mode && params.horizontal {
        for (_, reserved, _, _) in &fields {
            if !reserved.is_empty() {
                let (width, _) = engine.measure(reserved, params.label_font_size, params.dpi_scale);
                scroll_label_column_width = scroll_label_column_width.max(width);
            }
        }
    }
    let horizontal_available = horizontal_available_width(
        params.max_width,
        params.window_padding_x,
        params.item_padding_x,
        params.scroll_mode,
    );
    let horizontal_anchors = horizontal_candidate_anchors(
        &column_inputs,
        params.anchor_stride,
        if params.scroll_mode && params.horizontal {
            scroll_label_column_width
        } else {
            0.0
        },
        horizontal_available,
        params.label_gap,
        params.label_font_size,
        params.font_size,
        params.comment_font_size,
        |value, size| engine.measure(value, size, params.dpi_scale),
    );
    let mut items = Vec::with_capacity(indices.len());
    for (position, index) in indices.iter().enumerate() {
        let output = &outputs[*index];
        let (width, height) = if params.vertical {
            let label = output.label.as_str();
            let text = output.text.as_str();
            let glyph_step = (params.font_size * VERTICAL_GLYPH_STEP_RATIO)
                .ceil()
                .max(1.0);
            let label_width = engine
                .measure(label, params.label_font_size, params.dpi_scale)
                .0
                .max(0.0);
            let mut glyph_width = 0.0_f32;
            let clusters = grapheme_clusters(text);
            for cluster in &clusters {
                let (cluster_width, _) =
                    engine.measure(cluster, params.font_size, params.dpi_scale);
                glyph_width = glyph_width.max(cluster_width.max(0.0));
            }
            let label_height = if label.is_empty() {
                0.0
            } else {
                params.font_size
            };
            // A vertical-writing candidate is indivisible for selection, but
            // its glyph column is not: continue long text in adjacent columns
            // before it reaches the work-area edge. Keep the same row budget
            // in measurement and painting so the final grapheme is never
            // silently clipped by the renderer's window bounds.
            let available_height = if params.max_height > 0.0 {
                (params.max_height
                    - params.window_padding_y.max(0.0) * 2.0
                    - params.item_padding_y.max(0.0) * 2.0
                    - label_height)
                    .max(glyph_step)
            } else {
                f32::INFINITY
            };
            let rows_per_column = (available_height / glyph_step).floor().max(1.0) as usize;
            let columns = clusters.len().div_ceil(rows_per_column).max(1);
            (
                label_width.max(glyph_width) * columns as f32,
                label_height + clusters.len().min(rows_per_column) as f32 * glyph_step,
            )
        } else {
            let (label, _, text, comment) = &fields[position];
            let plan = horizontal_run_plan_with_anchors(
                label,
                text,
                comment,
                horizontal_available,
                params.label_gap,
                params.label_font_size,
                params.font_size,
                params.comment_font_size,
                horizontal_anchors[position],
                |value, size| engine.measure(value, size, params.dpi_scale),
            );
            (plan.width, plan.height)
        };
        items.push(Fcitx5CandidateLayoutSize {
            width: width + params.item_padding_x * 2.0,
            height: height + params.item_padding_y * 2.0,
        });
    }
    let mut panel = Fcitx5CandidateLayoutSize::default();
    if let Some(preedit) = preedit {
        if !preedit.is_empty() {
            let (run_width, run_height) =
                engine.measure(preedit, params.font_size, params.dpi_scale);
            panel = Fcitx5CandidateLayoutSize {
                width: run_width + params.item_padding_x * 2.0,
                height: run_height + params.item_padding_y * 2.0,
            };
        }
    }
    Some((
        items,
        panel,
        scroll_label_column_width,
        horizontal_anchors
            .iter()
            .map(|anchors| anchors.effective_width)
            .fold(0.0_f32, f32::max),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::horizontal_column_anchors;

    #[test]
    fn measurement_and_render_use_identical_horizontal_anchors_without_comments() {
        let mut engine = MeasureEngine::new();
        let candidates = [
            HorizontalColumnInput {
                label: "1.",
                reserved_label: "1.",
                text: "你",
                comment: "",
            },
            HorizontalColumnInput {
                label: "100.",
                reserved_label: "100.",
                text: "中文输入法",
                comment: "",
            },
        ];
        let params = MeasureLoopParams {
            horizontal: true,
            vertical: false,
            scroll_mode: false,
            label_gap: 4.0,
            item_padding_x: 8.0,
            item_padding_y: 6.0,
            font_size: 18.0,
            label_font_size: 15.3,
            comment_font_size: 14.4,
            dpi_scale: 1.0,
            max_width: 240.0,
            window_padding_x: 12.0,
            max_height: 0.0,
            window_padding_y: 0.0,
            anchor_stride: 1,
        };
        let available = horizontal_available_width(
            params.max_width,
            params.window_padding_x,
            params.item_padding_x,
            params.scroll_mode,
        );
        let anchors = horizontal_column_anchors(
            &candidates,
            0.0,
            available,
            params.label_gap,
            params.label_font_size,
            params.font_size,
            params.comment_font_size,
            |value, size| engine.measure(value, size, params.dpi_scale),
        );
        for candidate in candidates {
            let plan = horizontal_run_plan_with_anchors(
                candidate.label,
                candidate.text,
                candidate.comment,
                available,
                params.label_gap,
                params.label_font_size,
                params.font_size,
                params.comment_font_size,
                anchors,
                |value, size| engine.measure(value, size, params.dpi_scale),
            );
            assert_eq!(plan.anchors, anchors);
            assert!(plan.lines.iter().all(|line| line.comment.is_empty()));
        }
    }

    #[test]
    fn vertical_column_height_covers_every_glyph() {
        let mut engine = MeasureEngine::new();
        let mut arena = crate::CandidateVisualArena::default();
        let inputs = [crate::Fcitx5CandidateVisualBuildInput {
            label: "1".as_bytes(),
            text: "你好吗".as_bytes(),
            comment: "".as_bytes(),
            label_style: 1,
            labels_visible: 1,
            reservation_action: 0,
            reservation_slot: 0,
        }];
        let config = crate::Fcitx5CandidateVisualBuildConfig {
            configured_labels: &[],
        };
        let built = arena.build(&inputs[..1], &config);
        let outputs = arena.built_outputs().to_vec();
        assert_eq!(built, 1);
        let indices = [0usize];
        let font_size = 16.0_f32;
        let item_padding_y = 4.0_f32;
        let glyph_step = (font_size * VERTICAL_GLYPH_STEP_RATIO).ceil().max(1.0);
        let vertical = MeasureLoopParams {
            horizontal: false,
            vertical: true,
            scroll_mode: false,
            label_gap: 4.0,
            item_padding_x: 10.0,
            item_padding_y,
            font_size,
            label_font_size: 12.0,
            comment_font_size: 11.0,
            dpi_scale: 1.0,
            max_width: 0.0,
            window_padding_x: 0.0,
            max_height: 0.0,
            window_padding_y: 0.0,
            anchor_stride: 1,
        };
        let (vertical_items, _, _, _) =
            measure_visual_items(&mut engine, &outputs, &indices, None, &vertical)
                .expect("vertical measure loop must succeed");
        assert!(
            vertical_items[0].height >= 3.0 * glyph_step + item_padding_y * 2.0,
            "vertical column must cover all 3 glyph rows: {:?}",
            vertical_items[0]
        );

        // A skin-tone modifier must join its base emoji on one row, and a
        // regional-indicator flag must pair two at a time.
        assert_eq!(grapheme_clusters("👍🏽").len(), 1);
        assert_eq!(grapheme_clusters("🇺🇸").len(), 1);

        // The frozen horizontal measure is one short row, so the vertical
        // height requirement only holds once the vertical path is selected.
        let horizontal = MeasureLoopParams {
            horizontal: true,
            vertical: false,
            ..vertical
        };
        let (horizontal_items, _, _, _) =
            measure_visual_items(&mut engine, &outputs, &indices, None, &horizontal)
                .expect("horizontal measure loop must succeed");
        assert!(
            horizontal_items[0].height < 3.0 * glyph_step + item_padding_y * 2.0,
            "horizontal single-row measure must stay below the vertical column height: {:?}",
            horizontal_items[0]
        );
    }

    #[test]
    fn long_vertical_text_wraps_into_measured_columns_before_window_edge() {
        let mut engine = MeasureEngine::new();
        let mut arena = crate::CandidateVisualArena::default();
        let inputs = [crate::Fcitx5CandidateVisualBuildInput {
            label: "3.".as_bytes(),
            text: "Windows Next".as_bytes(),
            comment: "".as_bytes(),
            label_style: 1,
            labels_visible: 1,
            reservation_action: 0,
            reservation_slot: 0,
        }];
        let config = crate::Fcitx5CandidateVisualBuildConfig {
            configured_labels: &[],
        };
        let built = arena.build(&inputs[..1], &config);
        let outputs = arena.built_outputs().to_vec();
        assert_eq!(built, 1);
        let params = MeasureLoopParams {
            horizontal: false,
            vertical: true,
            scroll_mode: false,
            label_gap: 4.0,
            item_padding_x: 8.0,
            item_padding_y: 8.0,
            font_size: 46.0,
            label_font_size: 39.0,
            comment_font_size: 36.0,
            dpi_scale: 2.0,
            max_width: 280.0,
            window_padding_x: 8.0,
            max_height: 728.0,
            window_padding_y: 8.0,
            anchor_stride: 1,
        };
        let (items, _, _, _) = measure_visual_items(&mut engine, &outputs, &[0], None, &params)
            .expect("vertical measure loop must succeed");
        let glyph_count = grapheme_clusters("Windows Next").len();
        let single_column_width = engine.measure("W", params.font_size, params.dpi_scale).0;
        assert!(items[0].width > single_column_width + params.item_padding_x * 2.0);
        assert!(
            items[0].height + params.window_padding_y * 2.0 <= params.max_height,
            "wrapped candidate must fit the viewport: {:?}",
            items[0]
        );
        assert!(
            glyph_count > 8,
            "fixture must exercise a second text column"
        );
    }
}
