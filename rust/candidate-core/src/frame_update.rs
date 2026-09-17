//! Rust frame-update orchestration (082 slice 2).
//!
//! Replaces the C++ `CandidateWindow::update()` stitching: one function runs
//! the frozen model→presentation→visual→measure→layout→assembly sequence and
//! reports the host action. All underlying semantics already live in this
//! crate; this module only sequences them.

#![deny(unsafe_op_in_unsafe_fn)]

use fcitx5_protocol_core::KeyResponse;

use crate::axis_layout::{AxisLayoutInput, CandidateLayoutOptions, OverflowBehavior, WritingMode};
use crate::measure_ffi::{measure_visual_items, MeasureLoopParams};
use crate::renderer::MeasureEngine;
use crate::{
    fcitx5_candidate_horizontal_natural_downgrade, CandidateClickGuardState,
    CandidateFocusWatchState, CandidateModel, CandidatePresentationState, CandidateScrollState,
    CandidateSemanticItem, CandidateSemanticSnapshot, CandidateSnapshotIdentity, CandidateText,
    CandidateVisualArena, Fcitx5CandidateLayoutRect, Fcitx5CandidateVisualBuildConfig,
    Fcitx5CandidateVisualBuildInput, Orientation, Placement, Rect, Size,
};

/// Frozen `NativeOrientation` vocabulary from the C++ config boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameOrientation {
    Automatic,
    Vertical,
    Horizontal,
}

/// Frozen `NativeOverflow` vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameOverflow {
    Paging,
    Scrolling,
    Wrapping,
}

/// Frozen `NativeWritingMode` vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameWriting {
    Horizontal,
    VerticalRl,
    VerticalLr,
}

/// Flat, vector-free snapshot of the host's render configuration.
#[derive(Clone, Copy, Debug)]
pub struct FrameConfig {
    pub orientation: FrameOrientation,
    pub overflow: FrameOverflow,
    pub writing: FrameWriting,
    /// `preeditMode == panel`.
    pub preedit_panel: bool,
    /// FFI label-style vocabulary (0 plain .. 4 circled).
    pub label_style: u32,
    pub label_visible: bool,
    pub scroll_mode: bool,
    pub max_width_dip: f32,
    pub padding_x_dip: f32,
    pub padding_y_dip: f32,
    pub row_gap_dip: f32,
    pub column_gap_dip: f32,
    pub item_padding_x_dip: f32,
    pub item_padding_y_dip: f32,
    pub label_gap_dip: f32,
    pub font_size_dip: f32,
    pub label_font_scale: f32,
    pub annotation_font_scale: f32,
}

/// Persistent caret state mirrored from the C++ `lastCaret_` member.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameCaret {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub dpi: f32,
    pub valid: bool,
}

impl FrameCaret {
    fn from_protocol(caret: &fcitx5_protocol_core::CaretRect) -> Self {
        Self {
            left: caret.left as f32,
            top: caret.top as f32,
            right: caret.right as f32,
            bottom: caret.bottom as f32,
            dpi: caret.dpi as f32,
            valid: true,
        }
    }
}

/// The Rust-owned state objects one frame update touches.
pub struct FrameState<'a> {
    pub model: &'a mut CandidateModel,
    pub presentation: &'a mut CandidatePresentationState,
    pub scroll: &'a mut CandidateScrollState,
    pub click_guard: &'a mut CandidateClickGuardState,
    pub focus_watch: &'a mut CandidateFocusWatchState,
    pub arena: &'a mut CandidateVisualArena,
    pub measure: &'a mut MeasureEngine,
    pub content_locale: &'a str,
    pub configured_labels: &'a [String],
}

/// What the host must do after the Rust-side frame work completes.
#[derive(Clone, Debug)]
pub enum FrameUpdateOutcome {
    /// Stale/invalid snapshot or a duplicate presentation: keep everything.
    Ignored,
    /// Reset presentation state and hide the popup.
    Dismiss,
    /// Popup-blocked composition: hide the popup without resetting.
    HidePopup,
    /// Paint proceeds with these outputs.
    Proceed(FrameUpdateOutputs),
}

/// Everything the host stores for the paint path after a `Proceed`.
#[derive(Clone, Debug)]
pub struct FrameUpdateOutputs {
    /// `caret.dpi / 96` — the caller stores it as its paint DPI scale.
    pub font_scale: f32,
    pub window_left: f32,
    pub window_top: f32,
    pub window_width: f32,
    pub window_height: f32,
    /// `None` when there is no preedit block this frame.
    pub preedit_panel: Option<Rect>,
    pub preedit_divider_y: f32,
    pub has_scrollbar: bool,
    pub item_rects: Vec<Rect>,
    pub visible_indices: Vec<usize>,
    /// Panel-mode preedit text (UTF-8; empty when none).
    pub preedit_utf8: Vec<u8>,
    pub selection_inflate_x: f32,
    pub selection_inflate_y: f32,
    /// Final presentation orientation after the automatic downgrade check
    /// (true = horizontal).
    pub horizontal: bool,
}

fn frame_overflow(config: &FrameConfig) -> OverflowBehavior {
    // Frozen C++ mapping: non-horizontal writing collapses to paging, wrapping
    // stays wrapping, scrolling stays scrolling.
    match config.writing {
        FrameWriting::VerticalRl | FrameWriting::VerticalLr => OverflowBehavior::Paging,
        _ => match config.overflow {
            FrameOverflow::Paging => OverflowBehavior::Paging,
            FrameOverflow::Scrolling => OverflowBehavior::Scrolling,
            FrameOverflow::Wrapping => OverflowBehavior::Wrapping,
        },
    }
}

fn frame_writing(config: &FrameConfig) -> WritingMode {
    match config.writing {
        FrameWriting::Horizontal => WritingMode::Horizontal,
        FrameWriting::VerticalRl => WritingMode::VerticalRl,
        FrameWriting::VerticalLr => WritingMode::VerticalLr,
    }
}

fn configured_presentation_orientation(config: &FrameConfig) -> crate::PresentationOrientation {
    // Frozen C++ mapping: vertical writing forces vertical, then the
    // configured orientation, else automatic.
    match config.writing {
        FrameWriting::VerticalRl | FrameWriting::VerticalLr => {
            crate::PresentationOrientation::Vertical
        }
        FrameWriting::Horizontal => match config.orientation {
            FrameOrientation::Vertical => crate::PresentationOrientation::Vertical,
            FrameOrientation::Horizontal => crate::PresentationOrientation::Horizontal,
            FrameOrientation::Automatic => crate::PresentationOrientation::Automatic,
        },
    }
}

/// Runs the frozen `update()` sequence over Rust-owned state.
pub fn frame_update(
    state: &mut FrameState<'_>,
    config: FrameConfig,
    response: &KeyResponse,
    last_caret: &mut FrameCaret,
    work_area: Rect,
    focus_pid: u32,
) -> FrameUpdateOutcome {
    // 1. Model apply. The C++ host filters only stale (2) and invalid (3);
    //    duplicates continue.
    let snapshot = CandidateSemanticSnapshot {
        identity: CandidateSnapshotIdentity {
            engine_epoch: response.metadata.engine_epoch,
            context_id: response.metadata.context_id,
            composition_id: response.metadata.composition_id,
            revision: response.metadata.revision,
        },
        preedit: String::from_utf8_lossy(&response.preedit_utf8).into_owned(),
        auxiliary_up: String::new(),
        auxiliary_down: String::new(),
        candidates: response
            .candidates
            .iter()
            .map(|candidate| CandidateSemanticItem {
                id: candidate.id,
                label: String::from_utf8_lossy(&candidate.label_utf8).into_owned(),
                text: String::from_utf8_lossy(&candidate.text_utf8).into_owned(),
                comment: String::from_utf8_lossy(&candidate.comment_utf8).into_owned(),
            })
            .collect(),
        selected: (response.selected_candidate != u32::MAX)
            .then_some(response.selected_candidate as usize),
        page: response.candidate_page,
        total: response.candidate_total,
        visibility: response.candidate_visibility,
        popup_allowed: response.popup_allowed,
    };
    let applied = state.model.apply_semantic_snapshot(snapshot);
    if applied == 2 || applied == 3 {
        return FrameUpdateOutcome::Ignored;
    }
    let Some(current) = state.model.semantic_snapshot() else {
        return FrameUpdateOutcome::Ignored;
    };

    // A composition candidate window cannot survive an empty preedit. The
    // engine may briefly expose its old Fcitx candidate list while processing
    // Backspace; fail closed here instead of repainting that stale list.
    if current.visibility == 1 && current.preedit.is_empty() {
        return FrameUpdateOutcome::HidePopup;
    }

    // 2. Presentation apply.
    let presentation_applied = state
        .presentation
        .apply(crate::CandidatePresentationUpdate {
            engine_epoch: current.identity.engine_epoch,
            context_id: current.identity.context_id,
            composition_id: current.identity.composition_id,
            revision: current.identity.revision,
            selected: current.selected.unwrap_or(0),
            has_selected: u8::from(current.selected.is_some()),
            candidate_count: current.candidates.len(),
            page: response.candidate_page,
            page_size: response.candidate_page_size,
            candidate_bulk: u8::from(response.candidate_bulk),
            configured_scroll_mode: u8::from(config.scroll_mode),
        });
    if presentation_applied == 2 || presentation_applied == 3 {
        return FrameUpdateOutcome::Ignored;
    }
    state.click_guard.clear();

    // 3. Caret + font scale.
    if response.caret.valid {
        *last_caret = FrameCaret::from_protocol(&response.caret);
    }
    let scale = last_caret.dpi / 96.0;

    // 4. Preedit text (panel mode only).
    let preedit_utf8: Vec<u8> = if config.preedit_panel && !current.preedit.is_empty() {
        current.preedit.as_bytes().to_vec()
    } else {
        Vec::new()
    };

    // 5. Visual build with scroll-label reservations.
    let presentation_output = state.presentation.output();
    let selected_index = (presentation_output.has_selected != 0)
        .then_some(presentation_output.selected)
        .unwrap_or(0);
    let configured_labels: &[String] = state.configured_labels;
    let mut build_inputs = Vec::with_capacity(current.candidates.len());
    for (index, candidate) in current.candidates.iter().enumerate() {
        let reservation = crate::fcitx5_candidate_scroll_reservation_for(
            index,
            u8::from(!candidate.label.is_empty()),
            selected_index,
            u8::from(config.scroll_mode),
            u8::from(config.label_visible),
            presentation_output.scroll_columns,
            current.candidates.len(),
        );
        build_inputs.push(Fcitx5CandidateVisualBuildInput {
            label: crate::Fcitx5CandidateUtf8 {
                ptr: candidate.label.as_ptr(),
                len: candidate.label.len(),
            },
            text: crate::Fcitx5CandidateUtf8 {
                ptr: candidate.text.as_ptr(),
                len: candidate.text.len(),
            },
            comment: crate::Fcitx5CandidateUtf8 {
                ptr: candidate.comment.as_ptr(),
                len: candidate.comment.len(),
            },
            label_style: config.label_style,
            labels_visible: u8::from(config.label_visible),
            reservation_action: reservation.action,
            reservation_slot: reservation.slot,
        });
    }
    let configured_refs: Vec<crate::Fcitx5CandidateUtf8> = configured_labels
        .iter()
        .map(|label| crate::Fcitx5CandidateUtf8 {
            ptr: label.as_ptr(),
            len: label.len(),
        })
        .collect();
    let build_config = Fcitx5CandidateVisualBuildConfig {
        configured_labels: configured_refs.as_ptr(),
        configured_label_count: configured_refs.len(),
    };
    let built = state.arena.build(&build_inputs, &build_config);
    if built != current.candidates.len() {
        return FrameUpdateOutcome::Dismiss;
    }

    // 6. Render plan.
    let mut render_indices = vec![0_usize; current.candidates.len()];
    let Some(render_count) = state.presentation.render_plan(&mut render_indices) else {
        return FrameUpdateOutcome::Dismiss;
    };
    render_indices.truncate(render_count);

    // 7. Dismiss/hide decision chain.
    let decision = state.presentation.decide(
        current.visibility,
        current.candidates.len(),
        last_caret.valid,
        current.popup_allowed,
    );
    if decision == 1 {
        return FrameUpdateOutcome::Dismiss;
    }
    if decision == 2 {
        return FrameUpdateOutcome::HidePopup;
    }

    // 8. Focus-watch target capture.
    state.focus_watch.set_target(focus_pid);

    // 9. Presentation orientation.
    let resolved = state.presentation.layout.resolve_orientation(
        configured_presentation_orientation(&config),
        crate::AutomaticOrientationInput {
            candidates: &current
                .candidates
                .iter()
                .map(|candidate| CandidateText {
                    text: candidate.text.clone(),
                    comment: candidate.comment.clone(),
                })
                .collect::<Vec<_>>(),
            locale: state.content_locale,
            work_area,
            caret_x: last_caret.left,
            scale,
            page_size: response.candidate_page_size,
        },
    );
    let mut horizontal = resolved == Orientation::Horizontal;

    let configured_max_width = config.max_width_dip * scale;
    let work_width = (work_area.right - work_area.left).max(0.0);
    let measurement_max_width = if horizontal && !config.scroll_mode && work_width > 0.0 {
        configured_max_width.min(work_width)
    } else {
        configured_max_width
    };

    // 10. Measure loop over the live arena outputs.
    let measure_params = MeasureLoopParams {
        horizontal,
        vertical: matches!(
            config.writing,
            FrameWriting::VerticalRl | FrameWriting::VerticalLr
        ),
        scroll_mode: config.scroll_mode,
        label_gap: config.label_gap_dip * scale,
        item_padding_x: config.item_padding_x_dip * scale,
        item_padding_y: config.item_padding_y_dip * scale,
        font_size: config.font_size_dip * scale,
        label_font_size: config.font_size_dip * config.label_font_scale * scale,
        comment_font_size: config.font_size_dip * config.annotation_font_scale * scale,
        dpi_scale: scale,
        max_width: measurement_max_width,
        window_padding_x: config.padding_x_dip * scale,
    };
    let Some((items, preedit_panel_size, _scroll_label_column, horizontal_effective_width)) =
        measure_visual_items(
            state.measure,
            state.arena.built_outputs(),
            &render_indices,
            Some(&String::from_utf8_lossy(&preedit_utf8)),
            &measure_params,
        )
    else {
        return FrameUpdateOutcome::Dismiss;
    };
    let preedit_panel_width = preedit_panel_size.width;
    let preedit_panel_height = preedit_panel_size.height;
    // A too-small configured max width cannot contain every grapheme in each
    // fixed horizontal column. The renderer reports the minimum viable
    // item width through measurement; expand the owning window to that width
    // instead of allowing axis layout to clip a label, text, or comment.
    let measured_item_window_width = items
        .iter()
        .map(|item| item.width)
        .chain([preedit_panel_width])
        .fold(0.0_f32, f32::max)
        + measure_params.window_padding_x * 2.0;
    let shared_column_window_width = if horizontal_effective_width.is_finite() {
        horizontal_effective_width
            + measure_params.item_padding_x * 2.0
            + measure_params.window_padding_x * 2.0
    } else {
        0.0
    };
    let minimum_horizontal_window_width =
        measured_item_window_width.max(shared_column_window_width);
    let layout_max_width = if horizontal && !config.scroll_mode {
        configured_max_width
            .max(minimum_horizontal_window_width)
            .min(work_width)
    } else {
        configured_max_width
    };

    // 11. Automatic-orientation downgrade.
    if config.orientation == FrameOrientation::Automatic && horizontal {
        let item_widths: Vec<f32> = items.iter().map(|item| item.width).collect();
        let work_width = (work_area.right - work_area.left).max(0.0);
        let hard_limit = layout_max_width.min(work_width);
        // SAFETY: `fcitx5_candidate_horizontal_natural_downgrade` reads only
        // `item_widths.as_ptr()` for `item_widths.len()` elements (both valid
        // above) and the scalar arguments; it writes nothing and cannot retain
        // the pointer beyond the call.
        if unsafe {
            fcitx5_candidate_horizontal_natural_downgrade(
                item_widths.as_ptr(),
                item_widths.len(),
                config.padding_x_dip * scale,
                config.column_gap_dip * scale,
                preedit_panel_width,
                hard_limit,
            )
        } != 0
        {
            horizontal = false;
        }
    }

    // 12. Axis layout.
    let axis_result = crate::axis_layout::layout(&AxisLayoutInput {
        options: CandidateLayoutOptions {
            orientation: if horizontal {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            },
            overflow: frame_overflow(&config),
            writing_mode: frame_writing(&config),
        },
        items: items
            .iter()
            .map(|item| Size {
                width: item.width,
                height: item.height,
            })
            .collect(),
        caret: crate::Point {
            x: last_caret.left,
            y: last_caret.top,
        },
        caret_height: (last_caret.bottom - last_caret.top).max(1.0),
        work_area,
        max_width: layout_max_width,
        max_height: 0.0,
        padding_x: config.padding_x_dip * scale,
        padding_y: config.padding_y_dip * scale,
        row_gap: config.row_gap_dip * scale,
        column_gap: config.column_gap_dip * scale,
        page_size: response.candidate_page_size as usize,
        selected: selected_index,
        scroll_override: {
            let override_px = state.scroll.override_px();
            (override_px >= 0.0).then_some(override_px)
        },
        placement: Placement::Unlocked,
    });
    state.presentation.set_placement(axis_result.placement);

    // 13. Window assembly (stable width, work-area clamps, window-local item
    // rects, scrollbar rects).
    let (viewport_dx, viewport_dy) = axis_result.viewport_offset.unwrap_or((0.0, 0.0));
    let assembly_items: Vec<crate::Fcitx5CandidateAssemblyItem> = axis_result
        .items
        .iter()
        .map(|item| crate::Fcitx5CandidateAssemblyItem {
            x: item.rect.left,
            y: item.rect.top,
            w: item.rect.right - item.rect.left,
            h: item.rect.bottom - item.rect.top,
        })
        .collect();
    let assembly_input = crate::Fcitx5CandidateWindowAssemblyInput {
        presentation: state.presentation as *const CandidatePresentationState
            as *const core::ffi::c_void,
        window_x: axis_result.window.left,
        window_y: axis_result.window.top,
        window_w: axis_result.window.right - axis_result.window.left,
        window_h: axis_result.window.bottom - axis_result.window.top,
        placement: crate::placement_to_ffi(axis_result.placement),
        preedit_panel_height,
        preedit_panel_width,
        row_gap: config.row_gap_dip * scale,
        max_width: layout_max_width,
        work_left: work_area.left,
        work_top: work_area.top,
        work_right: work_area.right,
        work_bottom: work_area.bottom,
        item_padding_x: measure_params.item_padding_x,
        item_padding_y: measure_params.item_padding_y,
        viewport_dx,
        viewport_dy,
        items: assembly_items.as_ptr(),
        item_count: assembly_items.len(),
    };
    let mut assembly = crate::Fcitx5CandidateWindowAssemblyOutput::default();
    let mut assembly_rects = vec![Fcitx5CandidateLayoutRect::default(); assembly_items.len()];
    if !crate::window_assembly(
        &assembly_input,
        &mut assembly,
        assembly_rects.as_mut_slice(),
    ) {
        return FrameUpdateOutcome::Dismiss;
    }

    FrameUpdateOutcome::Proceed(FrameUpdateOutputs {
        font_scale: scale,
        window_left: assembly.window_left,
        window_top: assembly.window_top,
        window_width: assembly.window_width,
        window_height: assembly.window_height,
        preedit_panel: (assembly.preedit_divider_y != 0.0).then_some(Rect {
            left: assembly.preedit_panel.left,
            top: assembly.preedit_panel.top,
            right: assembly.preedit_panel.right,
            bottom: assembly.preedit_panel.bottom,
        }),
        preedit_divider_y: assembly.preedit_divider_y,
        has_scrollbar: assembly.has_scrollbar != 0,
        item_rects: assembly_rects
            .into_iter()
            .take(assembly.item_count)
            .map(|rect| Rect {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            })
            .collect(),
        visible_indices: render_indices
            .into_iter()
            .take(assembly.item_count)
            .collect(),
        preedit_utf8,
        selection_inflate_x: config.item_padding_x_dip * scale * 0.65,
        selection_inflate_y: config.item_padding_y_dip * scale * 0.55,
        horizontal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcitx5_protocol_core::{CandidateRecord, Metadata, Status};

    fn config() -> FrameConfig {
        FrameConfig {
            orientation: FrameOrientation::Automatic,
            overflow: FrameOverflow::Paging,
            writing: FrameWriting::Horizontal,
            preedit_panel: true,
            label_style: 1,
            label_visible: true,
            scroll_mode: false,
            max_width_dip: 720.0,
            padding_x_dip: 10.0,
            padding_y_dip: 8.0,
            row_gap_dip: 8.0,
            column_gap_dip: 10.0,
            item_padding_x_dip: 12.0,
            item_padding_y_dip: 8.0,
            label_gap_dip: 6.0,
            font_size_dip: 14.0,
            label_font_scale: 0.8,
            annotation_font_scale: 0.8,
        }
    }

    fn response() -> KeyResponse {
        KeyResponse {
            metadata: Metadata {
                request_id: 1,
                engine_epoch: 9,
                session_id: 1,
                context_id: 2,
                composition_id: 3,
                revision: 4,
                ..Metadata::default()
            },
            status: Status::Ok,
            handled: true,
            preedit_utf8: b"ni".to_vec(),
            candidates: vec![
                CandidateRecord {
                    id: 1,
                    label_utf8: b"1".to_vec(),
                    text_utf8: "你".as_bytes().to_vec(),
                    comment_utf8: "nǐ".as_bytes().to_vec(),
                },
                CandidateRecord {
                    id: 2,
                    label_utf8: b"2".to_vec(),
                    text_utf8: "好".as_bytes().to_vec(),
                    comment_utf8: Vec::new(),
                },
            ],
            selected_candidate: 0,
            candidate_total: 2,
            candidate_visibility: 1,
            candidate_page_size: 5,
            candidate_end: true,
            caret: fcitx5_protocol_core::CaretRect {
                valid: true,
                left: 100,
                top: 100,
                right: 102,
                bottom: 124,
                dpi: 96,
            },
            ..KeyResponse::default()
        }
    }

    fn work_area() -> Rect {
        Rect {
            left: 0.0,
            top: 0.0,
            right: 1920.0,
            bottom: 1040.0,
        }
    }

    #[test]
    fn frame_update_proceeds_through_the_frozen_sequence() {
        let mut model = CandidateModel::default();
        let mut presentation = CandidatePresentationState::default();
        let mut scroll = CandidateScrollState::default();
        let mut click_guard = CandidateClickGuardState::default();
        let mut focus_watch = CandidateFocusWatchState::default();
        let mut arena = CandidateVisualArena::default();
        let mut measure = MeasureEngine::new();
        let mut last_caret = FrameCaret::default();
        let configured: Vec<String> = Vec::new();
        let mut state = FrameState {
            model: &mut model,
            presentation: &mut presentation,
            scroll: &mut scroll,
            click_guard: &mut click_guard,
            focus_watch: &mut focus_watch,
            arena: &mut arena,
            measure: &mut measure,
            content_locale: "en-US",
            configured_labels: &configured,
        };
        let outcome = frame_update(
            &mut state,
            config(),
            &response(),
            &mut last_caret,
            work_area(),
            4321,
        );
        let FrameUpdateOutcome::Proceed(outputs) = outcome else {
            panic!("expected Proceed, got {outcome:?}");
        };
        assert_eq!(outputs.font_scale, 1.0);
        assert_eq!(outputs.visible_indices.len(), 2);
        assert_eq!(outputs.item_rects.len(), 2);
        assert!(outputs.window_width > 0.0 && outputs.window_height > 0.0);
        assert!(
            outputs.window_left + outputs.window_width <= work_area().right + f32::EPSILON,
            "window clamped into the work area"
        );
        assert!(outputs.preedit_panel.is_some(), "panel preedit present");
        assert_eq!(outputs.preedit_utf8, b"ni".to_vec());
        assert!(!outputs.has_scrollbar);
        // Focus watch captured the presented target.
        assert_eq!(state.focus_watch.target_process_id, 4321);
    }

    #[test]
    fn frame_update_ignores_stale_revisions() {
        let mut model = CandidateModel::default();
        // Seed a newer revision.
        let seed = response();
        let newer = KeyResponse {
            metadata: Metadata {
                revision: 9,
                ..seed.metadata
            },
            ..seed.clone()
        };
        let mut presentation = CandidatePresentationState::default();
        let mut scroll = CandidateScrollState::default();
        let mut click_guard = CandidateClickGuardState::default();
        let mut focus_watch = CandidateFocusWatchState::default();
        let mut arena = CandidateVisualArena::default();
        let mut measure = MeasureEngine::new();
        let mut last_caret = FrameCaret::default();
        let configured: Vec<String> = Vec::new();
        {
            let mut state = FrameState {
                model: &mut model,
                presentation: &mut presentation,
                scroll: &mut scroll,
                click_guard: &mut click_guard,
                focus_watch: &mut focus_watch,
                arena: &mut arena,
                measure: &mut measure,
                content_locale: "en-US",
                configured_labels: &configured,
            };
            assert!(matches!(
                frame_update(
                    &mut state,
                    config(),
                    &newer,
                    &mut last_caret,
                    work_area(),
                    1
                ),
                FrameUpdateOutcome::Proceed(_)
            ));
        }
        // Now the older revision (4 < 9) must be ignored.
        let mut state = FrameState {
            model: &mut model,
            presentation: &mut presentation,
            scroll: &mut scroll,
            click_guard: &mut click_guard,
            focus_watch: &mut focus_watch,
            arena: &mut arena,
            measure: &mut measure,
            content_locale: "en-US",
            configured_labels: &configured,
        };
        assert!(matches!(
            frame_update(&mut state, config(), &seed, &mut last_caret, work_area(), 1),
            FrameUpdateOutcome::Ignored
        ));
    }

    #[test]
    fn frame_update_hides_when_popup_blocked() {
        let blocked = KeyResponse {
            popup_allowed: false,
            ..response()
        };
        let mut model = CandidateModel::default();
        let mut presentation = CandidatePresentationState::default();
        let mut scroll = CandidateScrollState::default();
        let mut click_guard = CandidateClickGuardState::default();
        let mut focus_watch = CandidateFocusWatchState::default();
        let mut arena = CandidateVisualArena::default();
        let mut measure = MeasureEngine::new();
        let mut last_caret = FrameCaret::default();
        let configured: Vec<String> = Vec::new();
        let mut state = FrameState {
            model: &mut model,
            presentation: &mut presentation,
            scroll: &mut scroll,
            click_guard: &mut click_guard,
            focus_watch: &mut focus_watch,
            arena: &mut arena,
            measure: &mut measure,
            content_locale: "en-US",
            configured_labels: &configured,
        };
        assert!(matches!(
            frame_update(
                &mut state,
                config(),
                &blocked,
                &mut last_caret,
                work_area(),
                1
            ),
            FrameUpdateOutcome::HidePopup
        ));
    }

    #[test]
    fn frame_update_dismisses_hidden_visibility() {
        // Frozen model contract: a Hidden snapshot carries no candidates.
        let hidden = KeyResponse {
            candidate_visibility: 0,
            candidates: Vec::new(),
            selected_candidate: u32::MAX,
            ..response()
        };
        let mut model = CandidateModel::default();
        let mut presentation = CandidatePresentationState::default();
        let mut scroll = CandidateScrollState::default();
        let mut click_guard = CandidateClickGuardState::default();
        let mut focus_watch = CandidateFocusWatchState::default();
        let mut arena = CandidateVisualArena::default();
        let mut measure = MeasureEngine::new();
        let mut last_caret = FrameCaret::default();
        let configured: Vec<String> = Vec::new();
        let mut state = FrameState {
            model: &mut model,
            presentation: &mut presentation,
            scroll: &mut scroll,
            click_guard: &mut click_guard,
            focus_watch: &mut focus_watch,
            arena: &mut arena,
            measure: &mut measure,
            content_locale: "en-US",
            configured_labels: &configured,
        };
        let outcome = frame_update(
            &mut state,
            config(),
            &hidden,
            &mut last_caret,
            work_area(),
            1,
        );
        assert!(
            matches!(outcome, FrameUpdateOutcome::Dismiss),
            "expected Dismiss, got {outcome:?}"
        );
    }

    #[test]
    fn frame_update_hides_stale_composition_candidates_after_backspace() {
        let stale = KeyResponse {
            preedit_utf8: Vec::new(),
            candidate_visibility: 1,
            ..response()
        };
        let mut model = CandidateModel::default();
        let mut presentation = CandidatePresentationState::default();
        let mut scroll = CandidateScrollState::default();
        let mut click_guard = CandidateClickGuardState::default();
        let mut focus_watch = CandidateFocusWatchState::default();
        let mut arena = CandidateVisualArena::default();
        let mut measure = MeasureEngine::new();
        let mut last_caret = FrameCaret::default();
        let configured = Vec::new();
        let mut state = FrameState {
            model: &mut model,
            presentation: &mut presentation,
            scroll: &mut scroll,
            click_guard: &mut click_guard,
            focus_watch: &mut focus_watch,
            arena: &mut arena,
            measure: &mut measure,
            content_locale: "en-US",
            configured_labels: &configured,
        };
        assert!(matches!(
            frame_update(
                &mut state,
                config(),
                &stale,
                &mut last_caret,
                work_area(),
                1,
            ),
            FrameUpdateOutcome::HidePopup
        ));
    }

    #[test]
    fn horizontal_measurement_stays_inside_narrow_work_area_at_high_dpi() {
        let mut long_response = response();
        long_response.metadata.revision = 10;
        long_response.candidate_page_size = 2;
        long_response.candidate_total = 2;
        long_response.caret.dpi = 144;
        long_response.candidates = vec![
            CandidateRecord {
                id: 1,
                label_utf8: b"1".to_vec(),
                text_utf8: b"candidate with a deliberately long text value".to_vec(),
                comment_utf8: b"annotation that remains inside the candidate cell".to_vec(),
            },
            CandidateRecord {
                id: 2,
                label_utf8: b"2".to_vec(),
                text_utf8: b"short".to_vec(),
                comment_utf8: Vec::new(),
            },
        ];

        let mut narrow_config = config();
        narrow_config.orientation = FrameOrientation::Horizontal;
        narrow_config.max_width_dip = 720.0;
        narrow_config.scroll_mode = false;
        narrow_config.font_size_dip = 27.0;
        let narrow_work_area = Rect {
            left: 0.0,
            top: 0.0,
            right: 1024.0,
            bottom: 4000.0,
        };

        let mut model = CandidateModel::default();
        let mut presentation = CandidatePresentationState::default();
        let mut scroll = CandidateScrollState::default();
        let mut click_guard = CandidateClickGuardState::default();
        let mut focus_watch = CandidateFocusWatchState::default();
        let mut arena = CandidateVisualArena::default();
        let mut measure = MeasureEngine::new();
        let mut last_caret = FrameCaret::default();
        let configured: Vec<String> = Vec::new();
        let mut state = FrameState {
            model: &mut model,
            presentation: &mut presentation,
            scroll: &mut scroll,
            click_guard: &mut click_guard,
            focus_watch: &mut focus_watch,
            arena: &mut arena,
            measure: &mut measure,
            content_locale: "en-US",
            configured_labels: &configured,
        };

        let FrameUpdateOutcome::Proceed(outputs) = frame_update(
            &mut state,
            narrow_config,
            &long_response,
            &mut last_caret,
            narrow_work_area,
            4321,
        ) else {
            panic!("expected Proceed for long candidate geometry regression");
        };
        assert!(outputs.window_width <= narrow_work_area.right + f32::EPSILON);
        assert!(
            outputs.visible_indices.contains(&0),
            "selected candidate must remain visible: {:?}",
            outputs.visible_indices
        );
        for rect in &outputs.item_rects {
            assert!(
                rect.left >= -f32::EPSILON
                    && rect.top >= -f32::EPSILON
                    && rect.right <= outputs.window_width + f32::EPSILON
                    && rect.bottom <= outputs.window_height + f32::EPSILON,
                "item rect must stay inside client window: rect={rect:?}, window={}x{}",
                outputs.window_width,
                outputs.window_height
            );
            assert!(
                rect.right >= rect.left && rect.bottom >= rect.top,
                "item rect must have non-negative area: {rect:?}"
            );
        }
    }
}
