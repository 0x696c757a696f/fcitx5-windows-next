//! FFI surface for the Rust frame-update orchestration (082 slice 3).
//!
//! The C++ shell passes its six Rust state pointers, one flat decoded
//! response, one flat render configuration, and the environment; Rust runs
//! the frozen update sequence and returns the host action plus the paint
//! outputs. This deletes the C++ call-stitching without moving the Win32
//! glue yet.

use core::ffi::c_void;

use fcitx5_protocol_core::{CandidateRecord, CaretRect, KeyResponse, Metadata, Status};

use crate::frame_update::{
    frame_update, FrameCaret, FrameConfig, FrameOrientation, FrameOverflow, FrameState,
    FrameUpdateOutcome, FrameWriting,
};
use crate::renderer::MeasureEngine;
use crate::{
    CandidateClickGuardState, CandidateFocusWatchState, CandidateModel, CandidatePresentationState,
    CandidateScrollState, CandidateVisualArena, Rect,
};

/// Flat per-candidate record for the frame-update input.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateFrameRecord {
    pub id: u64,
    pub label: *const u8,
    pub label_len: usize,
    pub text: *const u8,
    pub text_len: usize,
    pub comment: *const u8,
    pub comment_len: usize,
}

/// Flat decoded response consumed by the orchestrator.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateFrameResponse {
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub preedit: *const u8,
    pub preedit_len: usize,
    pub status: u32,
    pub selected_candidate: u32,
    pub candidate_page: u32,
    pub candidate_page_size: u32,
    pub candidate_total: u32,
    pub candidate_visibility: u8,
    pub candidate_bulk: u8,
    pub candidate_end: u8,
    pub popup_allowed: u8,
    pub caret_valid: u8,
    pub caret_left: i32,
    pub caret_top: i32,
    pub caret_right: i32,
    pub caret_bottom: i32,
    pub caret_dpi: u32,
    pub candidates: *const Fcitx5CandidateFrameRecord,
    pub candidate_count: usize,
}

/// Flat render configuration consumed by the orchestrator.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateFrameUpdateConfig {
    pub orientation: u8,
    pub overflow: u8,
    pub writing: u8,
    pub preedit_panel: u8,
    pub label_style: u32,
    pub label_visible: u8,
    pub scroll_mode: u8,
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

/// Persistent caret mirrored in/out of the orchestrator.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateFrameCaretState {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub dpi: f32,
    pub valid: u8,
}

/// The six Rust state pointers plus environment for one frame update.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateFrameUpdateInput {
    pub model: *const c_void,
    pub presentation: *mut c_void,
    pub scroll: *mut c_void,
    pub click_guard: *mut c_void,
    pub focus_watch: *mut c_void,
    pub arena: *mut c_void,
    pub measure_engine: *mut c_void,
    pub response: Fcitx5CandidateFrameResponse,
    pub config: Fcitx5CandidateFrameUpdateConfig,
    pub work_area: crate::Fcitx5CandidateLayoutRect,
    pub focus_pid: u32,
    pub interaction_test: u8,
    pub content_locale: *const u8,
    pub content_locale_len: usize,
    pub configured_labels: *const crate::Fcitx5CandidateUtf8,
    pub configured_label_count: usize,
    pub last_caret: *mut Fcitx5CandidateFrameCaretState,
}

/// Host action + paint outputs for one frame update.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateFrameUpdateOutput {
    /// 0 keep, 1 dismiss (reset + hide), 2 hide popup only.
    pub action: u8,
    pub font_scale: f32,
    pub window_left: f32,
    pub window_top: f32,
    pub window_width: f32,
    pub window_height: f32,
    pub preedit_panel: crate::Fcitx5CandidateLayoutRect,
    pub has_preedit_panel: u8,
    pub preedit_divider_y: f32,
    pub has_scrollbar: u8,
    pub item_count: usize,
    pub visible_count: usize,
    pub selection_inflate_x: f32,
    pub selection_inflate_y: f32,
}

fn config_from_ffi(config: &Fcitx5CandidateFrameUpdateConfig) -> Option<FrameConfig> {
    let orientation = match config.orientation {
        0 => FrameOrientation::Automatic,
        1 => FrameOrientation::Vertical,
        2 => FrameOrientation::Horizontal,
        _ => return None,
    };
    let overflow = match config.overflow {
        0 => FrameOverflow::Paging,
        1 => FrameOverflow::Scrolling,
        2 => FrameOverflow::Wrapping,
        _ => return None,
    };
    let writing = match config.writing {
        0 => FrameWriting::Horizontal,
        1 => FrameWriting::VerticalRl,
        2 => FrameWriting::VerticalLr,
        _ => return None,
    };
    Some(FrameConfig {
        orientation,
        overflow,
        writing,
        preedit_panel: config.preedit_panel != 0,
        label_style: config.label_style,
        label_visible: config.label_visible != 0,
        scroll_mode: config.scroll_mode != 0,
        max_width_dip: config.max_width_dip,
        padding_x_dip: config.padding_x_dip,
        padding_y_dip: config.padding_y_dip,
        row_gap_dip: config.row_gap_dip,
        column_gap_dip: config.column_gap_dip,
        item_padding_x_dip: config.item_padding_x_dip,
        item_padding_y_dip: config.item_padding_y_dip,
        label_gap_dip: config.label_gap_dip,
        font_size_dip: config.font_size_dip,
        label_font_scale: config.label_font_scale,
        annotation_font_scale: config.annotation_font_scale,
    })
}

fn response_from_ffi(response: &Fcitx5CandidateFrameResponse) -> Option<KeyResponse> {
    if response.engine_epoch == 0 || response.context_id == 0 || response.revision == 0 {
        return None;
    }
    let bytes = |ptr: *const u8, len: usize| -> Vec<u8> {
        if len == 0 || ptr.is_null() {
            Vec::new()
        } else {
            // SAFETY: the C ABI contract requires len readable bytes.
            unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()
        }
    };
    let candidates = if response.candidate_count == 0 {
        Vec::new()
    } else if response.candidates.is_null() {
        return None;
    } else {
        // SAFETY: the C ABI contract requires candidate_count valid records.
        let records =
            unsafe { std::slice::from_raw_parts(response.candidates, response.candidate_count) };
        records
            .iter()
            .map(|record| CandidateRecord {
                id: record.id,
                label_utf8: bytes(record.label, record.label_len),
                text_utf8: bytes(record.text, record.text_len),
                comment_utf8: bytes(record.comment, record.comment_len),
            })
            .collect()
    };
    Some(KeyResponse {
        metadata: Metadata {
            engine_epoch: response.engine_epoch,
            context_id: response.context_id,
            composition_id: response.composition_id,
            revision: response.revision,
            ..Metadata::default()
        },
        status: Status::Ok,
        handled: true,
        preedit_utf8: bytes(response.preedit, response.preedit_len),

        selected_candidate: response.selected_candidate,
        candidate_page: response.candidate_page,
        candidate_page_size: response.candidate_page_size,
        candidate_total: response.candidate_total,
        candidate_visibility: response.candidate_visibility,
        candidate_bulk: response.candidate_bulk != 0,
        candidate_end: response.candidate_end != 0,
        popup_allowed: response.popup_allowed != 0,
        caret: CaretRect {
            valid: response.caret_valid != 0,
            left: response.caret_left,
            top: response.caret_top,
            right: response.caret_right,
            bottom: response.caret_bottom,
            dpi: response.caret_dpi,
        },
        candidates,
        ..KeyResponse::default()
    })
}

/// Runs the frozen frame-update sequence over the caller's Rust state.
///
/// # Safety
///
/// Every state pointer must be a valid object of its corresponding type
/// (created by this crate's `*_create` exports) or null; `input.last_caret`
/// must point to writable storage; the output item/visible buffers must cover
/// `MAX_CANDIDATES` entries when non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_frame_update(
    input: *const Fcitx5CandidateFrameUpdateInput,
    output: *mut Fcitx5CandidateFrameUpdateOutput,
    out_item_rects: *mut crate::Rect,
    out_visible_indices: *mut usize,
    out_preedit_utf8: *mut u8,
    out_preedit_capacity: usize,
    out_preedit_len: *mut usize,
) -> u8 {
    if input.is_null() || output.is_null() {
        return 0;
    }
    // SAFETY: non-null checked above; callers provide initialized storage.
    let input = unsafe { &*input };
    if input.last_caret.is_null() {
        return 0;
    }
    let Some(config) = config_from_ffi(&input.config) else {
        return 0;
    };
    let Some(response) = response_from_ffi(&input.response) else {
        return 0;
    };
    if input.model.is_null()
        || input.presentation.is_null()
        || input.scroll.is_null()
        || input.click_guard.is_null()
        || input.focus_watch.is_null()
        || input.arena.is_null()
        || input.measure_engine.is_null()
    {
        return 0;
    }
    // SAFETY: each pointer is the unique allocation from its create export.
    let model = unsafe { &mut *(input.model as *mut CandidateModel) };
    let presentation = unsafe { &mut *(input.presentation as *mut CandidatePresentationState) };
    let scroll = unsafe { &mut *(input.scroll as *mut CandidateScrollState) };
    let click_guard = unsafe { &mut *(input.click_guard as *mut CandidateClickGuardState) };
    let focus_watch = unsafe { &mut *(input.focus_watch as *mut CandidateFocusWatchState) };
    let arena = unsafe { &mut *(input.arena as *mut CandidateVisualArena) };
    let measure = unsafe { &mut *(input.measure_engine as *mut MeasureEngine) };
    let configured_labels = if input.configured_label_count == 0 {
        &[]
    } else if input.configured_labels.is_null() {
        return 0;
    } else {
        unsafe { std::slice::from_raw_parts(input.configured_labels, input.configured_label_count) }
    };
    let content_locale = if input.content_locale_len == 0 || input.content_locale.is_null() {
        String::new()
    } else {
        // SAFETY: the C ABI contract requires content_locale_len readable bytes.
        String::from_utf8_lossy(unsafe {
            std::slice::from_raw_parts(input.content_locale, input.content_locale_len)
        })
        .into_owned()
    };
    let configured_label_strings: Vec<String> = configured_labels
        .iter()
        .map(|label| {
            if label.len == 0 {
                String::new()
            } else {
                // SAFETY: the C ABI contract requires len readable bytes.
                String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(label.ptr, label.len) })
                    .into_owned()
            }
        })
        .collect();
    // SAFETY: non-null checked above; caller provides writable caret storage.
    let last_caret_state = unsafe { &mut *input.last_caret };
    let mut last_caret = FrameCaret {
        left: last_caret_state.left,
        top: last_caret_state.top,
        right: last_caret_state.right,
        bottom: last_caret_state.bottom,
        dpi: last_caret_state.dpi,
        valid: last_caret_state.valid != 0,
    };
    let mut state = FrameState {
        model,
        presentation,
        scroll,
        click_guard,
        focus_watch,
        arena,
        measure,
        content_locale: &content_locale,
        configured_labels: &configured_label_strings,
    };
    let outcome = frame_update(
        &mut state,
        config,
        &response,
        &mut last_caret,
        Rect {
            left: input.work_area.left,
            top: input.work_area.top,
            right: input.work_area.right,
            bottom: input.work_area.bottom,
        },
        input.focus_pid,
    );
    // Persist the caret for the next frame.
    last_caret_state.left = last_caret.left;
    last_caret_state.top = last_caret.top;
    last_caret_state.right = last_caret.right;
    last_caret_state.bottom = last_caret.bottom;
    last_caret_state.dpi = last_caret.dpi;
    last_caret_state.valid = u8::from(last_caret.valid);
    match outcome {
        FrameUpdateOutcome::Ignored => {
            // SAFETY: caller provides writable output storage.
            unsafe { (*output).action = 0 };
            1
        }
        FrameUpdateOutcome::Dismiss => {
            // SAFETY: caller provides writable output storage.
            unsafe { (*output).action = 1 };
            1
        }
        FrameUpdateOutcome::HidePopup => {
            // SAFETY: caller provides writable output storage.
            unsafe { (*output).action = 2 };
            1
        }
        FrameUpdateOutcome::Proceed(outputs) => {
            let item_count = outputs.item_rects.len().min(crate::MAX_CANDIDATES);
            let visible_count = outputs.visible_indices.len().min(crate::MAX_CANDIDATES);
            // SAFETY: the caller provides buffers covering MAX_CANDIDATES.
            unsafe {
                if !out_item_rects.is_null() {
                    for (slot, rect) in outputs.item_rects.iter().take(item_count).enumerate() {
                        out_item_rects.add(slot).write(Rect {
                            left: rect.left,
                            top: rect.top,
                            right: rect.right,
                            bottom: rect.bottom,
                        });
                    }
                }
                if !out_visible_indices.is_null() {
                    for (slot, index) in outputs
                        .visible_indices
                        .iter()
                        .take(visible_count)
                        .enumerate()
                    {
                        out_visible_indices.add(slot).write(*index);
                    }
                }
                (*output).action = 0;
                (*output).font_scale = outputs.font_scale;
                (*output).window_left = outputs.window_left;
                (*output).window_top = outputs.window_top;
                (*output).window_width = outputs.window_width;
                (*output).window_height = outputs.window_height;
                (*output).preedit_panel = crate::Fcitx5CandidateLayoutRect {
                    left: outputs.preedit_panel.map(|rect| rect.left).unwrap_or(0.0),
                    top: outputs.preedit_panel.map(|rect| rect.top).unwrap_or(0.0),
                    right: outputs.preedit_panel.map(|rect| rect.right).unwrap_or(0.0),
                    bottom: outputs.preedit_panel.map(|rect| rect.bottom).unwrap_or(0.0),
                };
                (*output).has_preedit_panel = u8::from(outputs.preedit_panel.is_some());
                (*output).preedit_divider_y = outputs.preedit_divider_y;
                (*output).has_scrollbar = u8::from(outputs.has_scrollbar);
                (*output).item_count = item_count;
                (*output).visible_count = visible_count;
                (*output).selection_inflate_x = outputs.selection_inflate_x;
                (*output).selection_inflate_y = outputs.selection_inflate_y;
            }
            let preedit_len = outputs.preedit_utf8.len();
            let copy_len = preedit_len.min(out_preedit_capacity);
            if !out_preedit_utf8.is_null() && copy_len > 0 {
                // SAFETY: caller provides writable storage of out_preedit_capacity.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        outputs.preedit_utf8.as_ptr(),
                        out_preedit_utf8,
                        copy_len,
                    );
                }
            }
            // SAFETY: caller provides writable output storage.
            unsafe { *out_preedit_len = preedit_len };
            1
        }
    }
}
