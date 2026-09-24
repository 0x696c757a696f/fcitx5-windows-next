//! C ABI for host-side text measurement (081C slice).
//!
//! This module is the only unsafe boundary for the measurement engine; the
//! engine itself is safe Rust in `renderer::MeasureEngine`.

#![deny(unsafe_op_in_unsafe_fn)]

use core::ffi::c_void;

use crate::renderer::{
    grapheme_clusters, horizontal_available_width, horizontal_column_anchors,
    horizontal_run_plan_with_anchors, Fcitx5CandidateMeasureSize, HorizontalColumnInput,
    MeasureEngine, VERTICAL_GLYPH_STEP_RATIO,
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
    let field = |value: &crate::Fcitx5CandidateUtf8| -> String {
        // SAFETY: arena-owned strings for a live build are valid UTF-8.
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(value.ptr, value.len) })
            .into_owned()
    };
    let fields: Vec<(String, String, String, String)> = indices
        .iter()
        .map(|index| {
            let output = &outputs[*index];
            (
                field(&output.label),
                field(&output.reserved_label),
                field(&output.text),
                field(&output.comment),
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
    let horizontal_anchors = horizontal_column_anchors(
        &column_inputs,
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
            let label = field(&output.label);
            let text = field(&output.text);
            let glyph_step = (params.font_size * VERTICAL_GLYPH_STEP_RATIO)
                .ceil()
                .max(1.0);
            let label_width = engine
                .measure(&label, params.label_font_size, params.dpi_scale)
                .0
                .max(0.0);
            let mut glyph_width = 0.0_f32;
            let clusters = grapheme_clusters(&text);
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
                horizontal_anchors,
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
        horizontal_anchors.effective_width,
    ))
}

/// Measures rendered-item sizes exactly like the shipping C++ `update()`
/// measure loop: optional scroll-label column width, per-item
/// reserved-label/text/comment runs with a fallback cell, and the preedit
/// panel including its padding. `outputs` must be the live arena build whose
/// strings are still valid.
///
/// # Safety
///
/// `engine` must be a valid measure-engine pointer; `outputs` must be valid
/// for `output_count` elements; `indices` must be valid for `index_count`
/// elements and each index must be `< output_count`; `out_items` must cover
/// `index_count` writable sizes; `out_preedit_panel` and
/// `out_scroll_label_column_width` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_measure_visual_items(
    engine: *mut c_void,
    outputs: *const Fcitx5CandidateVisualBuildOutput,
    output_count: usize,
    indices: *const usize,
    index_count: usize,
    horizontal: u8,
    scroll_mode: u8,
    label_gap: f32,
    item_padding_x: f32,
    item_padding_y: f32,
    font_size: f32,
    label_font_size: f32,
    comment_font_size: f32,
    preedit: *const u8,
    preedit_len: usize,
    dpi_scale: f32,
    out_items: *mut Fcitx5CandidateLayoutSize,
    out_preedit_panel: *mut Fcitx5CandidateLayoutSize,
    out_scroll_label_column_width: *mut f32,
) -> usize {
    if engine.is_null()
        || out_items.is_null()
        || out_preedit_panel.is_null()
        || out_scroll_label_column_width.is_null()
        || (output_count > 0 && outputs.is_null())
        || (index_count > 0 && indices.is_null())
    {
        return 0;
    }
    // SAFETY: caller guarantees the engine allocation is valid for this call.
    let engine = unsafe { &mut *engine.cast::<MeasureEngine>() };
    // SAFETY: the C ABI contract requires these slices to be valid.
    let outputs = if output_count == 0 {
        &[]
    } else {
        // SAFETY: non-null was checked above and the ABI supplies `output_count` readable records.
        unsafe { std::slice::from_raw_parts(outputs, output_count) }
    };
    let indices = if index_count == 0 {
        &[]
    } else {
        // SAFETY: non-null was checked above and the ABI supplies `index_count` readable indices.
        unsafe { std::slice::from_raw_parts(indices, index_count) }
    };
    let preedit_text = if preedit_len > 0 && !preedit.is_null() {
        // SAFETY: the C ABI contract requires preedit_len readable bytes.
        Some(String::from_utf8_lossy(unsafe {
            std::slice::from_raw_parts(preedit, preedit_len)
        }))
    } else {
        None
    };
    let params = MeasureLoopParams {
        horizontal: horizontal != 0,
        vertical: false,
        scroll_mode: scroll_mode != 0,
        label_gap,
        item_padding_x,
        item_padding_y,
        font_size,
        label_font_size,
        comment_font_size,
        dpi_scale,
        max_width: 0.0,
        window_padding_x: 0.0,
        max_height: 0.0,
        window_padding_y: 0.0,
    };
    let Some((items, panel, scroll_label_column_width, _horizontal_effective_width)) =
        measure_visual_items(engine, outputs, indices, preedit_text.as_deref(), &params)
    else {
        return 0;
    };
    if items.len() != index_count {
        return 0;
    }
    // SAFETY: non-null checked above; caller provides writable output storage.
    unsafe {
        if !items.is_empty() {
            std::ptr::copy_nonoverlapping(items.as_ptr(), out_items, items.len());
        }
        *out_preedit_panel = panel;
        *out_scroll_label_column_width = scroll_label_column_width;
    }
    index_count
}

/// Creates the shipping measurement engine.
#[no_mangle]
pub extern "C" fn fcitx5_candidate_measure_create() -> *mut c_void {
    Box::into_raw(Box::new(MeasureEngine::new())).cast()
}

/// # Safety
///
/// `engine` must be null or a pointer returned by
/// `fcitx5_candidate_measure_create` that has not already been destroyed.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_measure_destroy(engine: *mut c_void) {
    if !engine.is_null() {
        // SAFETY: caller guarantees this is the unique allocation from create.
        unsafe { drop(Box::from_raw(engine.cast::<MeasureEngine>())) };
    }
}

/// # Safety
///
/// `engine` must be a valid measure-engine pointer, `text` must reference
/// `text_len` readable bytes, and `out` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_measure_text_utf8(
    engine: *mut c_void,
    text: *const u8,
    text_len: usize,
    font_size: f32,
    dpi_scale: f32,
    out: *mut Fcitx5CandidateMeasureSize,
) -> u8 {
    if engine.is_null() || out.is_null() || (text_len > 0 && text.is_null()) {
        return 0;
    }
    // SAFETY: the C ABI contract requires this byte slice to be valid.
    let bytes = unsafe { std::slice::from_raw_parts(text, text_len) };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return 0;
    };
    // SAFETY: caller guarantees the engine allocation is valid for this call.
    let engine = unsafe { &mut *engine.cast::<MeasureEngine>() };
    let (width, height) = engine.measure(text, font_size, dpi_scale);
    // SAFETY: non-null checked above; caller provides writable output storage.
    unsafe {
        *out = Fcitx5CandidateMeasureSize { width, height };
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_text_utf8_rejects_invalid_utf8_and_null_outputs() {
        let engine = fcitx5_candidate_measure_create();
        assert!(!engine.is_null());
        let mut size = Fcitx5CandidateMeasureSize::default();
        // SAFETY: engine came from create; valid buffers are supplied.
        let ok = unsafe {
            fcitx5_candidate_measure_text_utf8(engine, "abc".as_ptr(), 3, 14.0, 1.0, &mut size)
        };
        assert_eq!(ok, 1);
        assert!(size.width > 0.0 && size.height > 0.0);
        // SAFETY: single valid bytes slice for a rejected call.
        let bad = unsafe {
            fcitx5_candidate_measure_text_utf8(
                engine,
                [0xFF, 0xFE].as_ptr(),
                2,
                14.0,
                1.0,
                &mut size,
            )
        };
        assert_eq!(bad, 0, "invalid UTF-8 must fail, not replace text");
        // SAFETY: null out must be rejected without dereferencing.
        let null_out = unsafe {
            fcitx5_candidate_measure_text_utf8(
                engine,
                "abc".as_ptr(),
                3,
                14.0,
                1.0,
                core::ptr::null_mut(),
            )
        };
        assert_eq!(null_out, 0);
        // SAFETY: engine is the unique allocation from create.
        unsafe { fcitx5_candidate_measure_destroy(engine) };
    }

    #[test]
    fn measure_visual_items_follows_the_frozen_update_measure_loop() {
        let engine = fcitx5_candidate_measure_create();
        let mut arena = crate::CandidateVisualArena::default();
        let inputs = [
            crate::Fcitx5CandidateVisualBuildInput {
                label: str_field("1"),
                text: str_field("你"),
                comment: str_field("nǐ"),
                label_style: 1,
                labels_visible: 1,
                reservation_action: 0,
                reservation_slot: 0,
            },
            crate::Fcitx5CandidateVisualBuildInput {
                label: str_field(""),
                text: str_field("好"),
                comment: str_field(""),
                label_style: 1,
                labels_visible: 1,
                reservation_action: 0,
                reservation_slot: 0,
            },
        ];
        let config = crate::Fcitx5CandidateVisualBuildConfig {
            configured_labels: core::ptr::null(),
            configured_label_count: 0,
        };
        let mut outputs = [crate::Fcitx5CandidateVisualBuildOutput::null_output(); 2];
        // SAFETY: fixture storage is live and buffers satisfy the visual-build ABI.
        let built = unsafe {
            crate::fcitx5_candidate_visual_build(
                &mut arena as *mut _ as *mut c_void,
                inputs.as_ptr(),
                2,
                &config,
                outputs.as_mut_ptr(),
            )
        };
        assert_eq!(built, 2);
        let indices = [0usize, 1];
        let mut items = [crate::Fcitx5CandidateLayoutSize::default(); 2];
        let mut preedit = crate::Fcitx5CandidateLayoutSize::default();
        let mut scroll_col = 0.0_f32;
        // SAFETY: engine and all fixture buffers are live and satisfy the measurement ABI.
        let measured = unsafe {
            fcitx5_candidate_measure_visual_items(
                engine,
                outputs.as_ptr(),
                2,
                indices.as_ptr(),
                2,
                1,
                0,
                4.0,
                10.0,
                8.0,
                14.0,
                12.0,
                11.0,
                core::ptr::null(),
                0,
                1.0,
                items.as_mut_ptr(),
                out_preedit(&mut preedit),
                &mut scroll_col,
            )
        };
        assert_eq!(measured, 2);
        for item in &items {
            assert!(
                item.width > 20.0 && item.height > 8.0,
                "padded cell {item:?}"
            );
        }
        assert_eq!(scroll_col, 0.0, "non-scroll mode has no label column");
        assert_eq!(preedit.width, 0.0, "no preedit input → zero panel");
        // SAFETY: engine is the unique allocation from create.
        unsafe { fcitx5_candidate_measure_destroy(engine) };
    }

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
            label: str_field("1"),
            text: str_field("你好吗"),
            comment: str_field(""),
            label_style: 1,
            labels_visible: 1,
            reservation_action: 0,
            reservation_slot: 0,
        }];
        let config = crate::Fcitx5CandidateVisualBuildConfig {
            configured_labels: core::ptr::null(),
            configured_label_count: 0,
        };
        let mut outputs = [crate::Fcitx5CandidateVisualBuildOutput::null_output(); 1];
        // SAFETY: fixture storage is live and buffers satisfy the visual-build ABI.
        let built = unsafe {
            crate::fcitx5_candidate_visual_build(
                &mut arena as *mut _ as *mut c_void,
                inputs.as_ptr(),
                1,
                &config,
                outputs.as_mut_ptr(),
            )
        };
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
            label: str_field("3."),
            text: str_field("Windows Next"),
            comment: str_field(""),
            label_style: 1,
            labels_visible: 1,
            reservation_action: 0,
            reservation_slot: 0,
        }];
        let config = crate::Fcitx5CandidateVisualBuildConfig {
            configured_labels: core::ptr::null(),
            configured_label_count: 0,
        };
        let mut outputs = [crate::Fcitx5CandidateVisualBuildOutput::null_output(); 1];
        // SAFETY: fixture storage is live and buffers satisfy the visual-build ABI.
        let built = unsafe {
            crate::fcitx5_candidate_visual_build(
                &mut arena as *mut _ as *mut c_void,
                inputs.as_ptr(),
                1,
                &config,
                outputs.as_mut_ptr(),
            )
        };
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

    fn out_preedit(
        slot: &mut crate::Fcitx5CandidateLayoutSize,
    ) -> *mut crate::Fcitx5CandidateLayoutSize {
        slot as *mut _
    }

    fn str_field(value: &str) -> crate::Fcitx5CandidateUtf8 {
        crate::Fcitx5CandidateUtf8 {
            ptr: value.as_ptr(),
            len: value.len(),
        }
    }
}
