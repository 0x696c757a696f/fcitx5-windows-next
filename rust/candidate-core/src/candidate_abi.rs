#![deny(unsafe_op_in_unsafe_fn)]

use crate::{
    CandidateSemanticItem, CandidateSemanticSnapshot, CandidateSnapshotIdentity, CandidateTheme,
    CandidateUiApplyResult, CandidateUiColor, CandidateUiConfig, CandidateUiInput,
    CandidateUiMeasurement, CandidateUiPlan, CandidateUiState, CandidateUiText,
    Fcitx5CandidateLayoutPoint, Fcitx5CandidateLayoutRect, Fcitx5CandidateModelSnapshot,
    Fcitx5CandidateUtf8, Orientation, Point, PresentationOrientation, Rect, MAX_CANDIDATES,
    MAX_CANDIDATE_TEXT_UTF8,
};
use std::ffi::c_void;
use std::panic;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateUiInput {
    pub snapshot: Fcitx5CandidateModelSnapshot,
    pub locale: Fcitx5CandidateUtf8,
    pub caret: Fcitx5CandidateLayoutPoint,
    pub caret_height: f32,
    pub work_area: Fcitx5CandidateLayoutRect,
    pub candidate_bulk: u8,
    pub orientation: u32,
    pub scroll_mode: u8,
    pub page_size: u32,
    pub max_width: f32,
    pub scroll_cell_width: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub row_gap: f32,
    pub column_gap: f32,
    pub item_padding_x: f32,
    pub item_padding_y: f32,
    pub label_gap: f32,
    pub candidate_font_size: f32,
    pub theme: u32,
    pub opacity: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateUiMeasurement {
    pub label_width: f32,
    pub text_width: f32,
    pub comment_width: f32,
    pub height: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateUiTextOutput {
    pub candidate_index: usize,
    pub label: Fcitx5CandidateUtf8,
    pub text: Fcitx5CandidateUtf8,
    pub comment: Fcitx5CandidateUtf8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateUiColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateUiPlanOutput {
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub popup_visible: u8,
    pub orientation: u32,
    pub placement: u32,
    pub window: Fcitx5CandidateLayoutRect,
    pub preedit: Fcitx5CandidateUtf8,
    pub preedit_rect: Fcitx5CandidateLayoutRect,
    pub has_preedit_rect: u8,
    pub background: Fcitx5CandidateUiColor,
    pub border: Fcitx5CandidateUiColor,
    pub text: Fcitx5CandidateUiColor,
    pub annotation: Fcitx5CandidateUiColor,
    pub label: Fcitx5CandidateUiColor,
    pub selected_background: Fcitx5CandidateUiColor,
    pub selected_text: Fcitx5CandidateUiColor,
    pub scrollbar: Fcitx5CandidateUiColor,
    pub opacity: f32,
    pub scrollbar_track: Fcitx5CandidateLayoutRect,
    pub scrollbar_thumb: Fcitx5CandidateLayoutRect,
    pub has_scrollbar: u8,
    pub item_count: usize,
    pub uia_item_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateUiRenderItemOutput {
    pub candidate_index: usize,
    pub id: u64,
    pub label: Fcitx5CandidateUtf8,
    pub text: Fcitx5CandidateUtf8,
    pub comment: Fcitx5CandidateUtf8,
    pub item_rect: Fcitx5CandidateLayoutRect,
    pub label_rect: Fcitx5CandidateLayoutRect,
    pub text_rect: Fcitx5CandidateLayoutRect,
    pub comment_rect: Fcitx5CandidateLayoutRect,
    pub has_comment_rect: u8,
    pub selected: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateUiUiaItemOutput {
    pub candidate_index: usize,
    pub name: Fcitx5CandidateUtf8,
    pub selected: u8,
}

struct CandidateUiAbiState {
    core: CandidateUiState,
    measurement_texts: Vec<CandidateUiText>,
    plan: CandidateUiPlan,
    identity: CandidateSnapshotIdentity,
}

impl Default for CandidateUiAbiState {
    fn default() -> Self {
        Self {
            core: CandidateUiState::default(),
            measurement_texts: Vec::new(),
            plan: CandidateUiPlan::default(),
            identity: CandidateSnapshotIdentity {
                engine_epoch: 0,
                context_id: 0,
                composition_id: 0,
                revision: 0,
            },
        }
    }
}

/// Creates one Rust-owned Candidate UI state. The returned opaque handle has no HWND or COM state.
#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_candidate_ui_create() -> *mut c_void {
    Box::into_raw(Box::<CandidateUiAbiState>::default()).cast()
}

/// Destroys a Candidate UI handle returned by `fcitx5_candidate_ui_create`.
///
/// # Safety
///
/// `state` must be null or a live, unique handle returned by
/// `fcitx5_candidate_ui_create` that has not already been destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_candidate_ui_destroy(state: *mut c_void) {
    if state.is_null() {
        return;
    }
    // SAFETY: the caller supplies one unique opaque handle from create.
    unsafe { drop(Box::from_raw(state.cast::<CandidateUiAbiState>())) };
}

/// Applies one complete Candidate snapshot and resolved visual configuration.
///
/// Returns 0 for applied, 1 for duplicate, 2 for stale, and 3 for rejected input.
///
/// # Safety
///
/// `state` must be a live handle and `input` must point to a readable structure.
/// All spans nested in `input` must remain readable for this call only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_candidate_ui_apply(
    state: *mut c_void,
    input: *const Fcitx5CandidateUiInput,
) -> u32 {
    panic::catch_unwind(|| {
        if state.is_null() || input.is_null() {
            return 3;
        }
        // SAFETY: non-null checks above; caller owns valid values for this call.
        let input = unsafe { *input };
        // SAFETY: the handle came from create and is exclusively owned by the caller.
        let state = unsafe { &mut *state.cast::<CandidateUiAbiState>() };
        // SAFETY: nested spans are valid for this call by the FFI contract.
        let Some(input) = (unsafe { input_from_ffi(input) }) else {
            return 3;
        };
        let identity = input.snapshot.identity;
        let result = state.core.apply(input);
        if result == CandidateUiApplyResult::Applied {
            state.identity = identity;
            state.measurement_texts = state.core.measurement_texts();
            state.plan = CandidateUiPlan::default();
        }
        match result {
            CandidateUiApplyResult::Applied => 0,
            CandidateUiApplyResult::Duplicate => 1,
            CandidateUiApplyResult::Stale => 2,
            CandidateUiApplyResult::Invalid => 3,
        }
    })
    .unwrap_or(3)
}

/// Returns text owned by Rust that the DirectWrite adapter must measure before building a plan.
///
/// # Safety
///
/// `state` must be a live handle. `output` may be null only when `capacity` is
/// zero; otherwise it must designate writable storage for `capacity` records.
/// Returned string spans are borrowed from `state` and become invalid after the
/// next apply or destroy call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_candidate_ui_measurement_texts(
    state: *mut c_void,
    output: *mut Fcitx5CandidateUiTextOutput,
    capacity: usize,
) -> usize {
    if state.is_null() || (output.is_null() && capacity != 0) {
        return 0;
    }
    // SAFETY: `state` is a live opaque handle per the function contract.
    let state = unsafe { &mut *state.cast::<CandidateUiAbiState>() };
    let needed = state.measurement_texts.len();
    if output.is_null() || capacity < needed {
        return needed;
    }
    // SAFETY: the caller provided `capacity` writable records and capacity >= needed.
    let output = unsafe { std::slice::from_raw_parts_mut(output, needed) };
    for (target, source) in output.iter_mut().zip(&state.measurement_texts) {
        *target = Fcitx5CandidateUiTextOutput {
            candidate_index: source.candidate_index,
            label: ffi_utf8(&source.label),
            text: ffi_utf8(&source.text),
            comment: ffi_utf8(&source.comment),
        };
    }
    needed
}

/// Builds a complete immutable render and UIA plan from adapter-provided DWrite measurements.
///
/// # Safety
///
/// `state` and `output` must be live/writable. `measurements` may be null only
/// when `measurement_count` is zero. `items` and `uia_items` may be null only
/// when their matching capacities are zero. All output spans borrow `state` and
/// remain valid until the next apply, plan build, or destroy call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_candidate_ui_build_plan(
    state: *mut c_void,
    measurements: *const Fcitx5CandidateUiMeasurement,
    measurement_count: usize,
    output: *mut Fcitx5CandidateUiPlanOutput,
    items: *mut Fcitx5CandidateUiRenderItemOutput,
    item_capacity: usize,
    uia_items: *mut Fcitx5CandidateUiUiaItemOutput,
    uia_item_capacity: usize,
) -> u8 {
    panic::catch_unwind(|| {
        if state.is_null()
            || output.is_null()
            || (measurements.is_null() && measurement_count != 0)
            || (items.is_null() && item_capacity != 0)
            || (uia_items.is_null() && uia_item_capacity != 0)
        {
            return 0;
        }
        // SAFETY: pointer/length contract checked above; slice is copied immediately.
        let measurements = if measurement_count == 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(measurements, measurement_count) }
        };
        let Some(measurements) = measurements
            .iter()
            .map(measurement_from_ffi)
            .collect::<Option<Vec<_>>>()
        else {
            return 0;
        };
        // SAFETY: `state` is a live opaque handle per the function contract.
        let state = unsafe { &mut *state.cast::<CandidateUiAbiState>() };
        state.plan = state.core.render_plan(&measurements);
        // SAFETY: output is writable by the function contract.
        unsafe { *output = plan_output(&state.plan, state.identity) };
        if item_capacity < state.plan.items.len() || uia_item_capacity < state.plan.uia.items.len()
        {
            return 0;
        }
        if !state.plan.items.is_empty() {
            // SAFETY: capacities were verified against Rust-owned plan lengths.
            let items = unsafe { std::slice::from_raw_parts_mut(items, state.plan.items.len()) };
            for (target, source) in items.iter_mut().zip(&state.plan.items) {
                *target = Fcitx5CandidateUiRenderItemOutput {
                    candidate_index: source.candidate_index,
                    id: source.id,
                    label: ffi_utf8(&source.label),
                    text: ffi_utf8(&source.text),
                    comment: ffi_utf8(&source.comment),
                    item_rect: rect_to_ffi(source.item_rect),
                    label_rect: rect_to_ffi(source.label_rect),
                    text_rect: rect_to_ffi(source.text_rect),
                    comment_rect: source
                        .comment_rect
                        .map_or_else(Fcitx5CandidateLayoutRect::default, rect_to_ffi),
                    has_comment_rect: u8::from(source.comment_rect.is_some()),
                    selected: u8::from(source.selected),
                };
            }
        }
        if !state.plan.uia.items.is_empty() {
            // SAFETY: capacities were verified against Rust-owned plan lengths.
            let uia_items =
                unsafe { std::slice::from_raw_parts_mut(uia_items, state.plan.uia.items.len()) };
            for (target, source) in uia_items.iter_mut().zip(&state.plan.uia.items) {
                *target = Fcitx5CandidateUiUiaItemOutput {
                    candidate_index: source.candidate_index,
                    name: ffi_utf8(&source.name),
                    selected: u8::from(source.selected),
                };
            }
        }
        1
    })
    .unwrap_or(0)
}

unsafe fn input_from_ffi(input: Fcitx5CandidateUiInput) -> Option<CandidateUiInput> {
    let snapshot = unsafe { semantic_snapshot_from_ffi(input.snapshot) }?;
    let locale = unsafe { utf8_from_ffi(input.locale) }?.to_owned();
    let work_area = rect_from_ffi(input.work_area)?;
    let config = CandidateUiConfig {
        orientation: match input.orientation {
            0 => PresentationOrientation::Automatic,
            1 => PresentationOrientation::Vertical,
            2 => PresentationOrientation::Horizontal,
            _ => return None,
        },
        scroll_mode: input.scroll_mode != 0,
        page_size: input.page_size,
        max_width: finite_nonnegative(input.max_width)?,
        scroll_cell_width: finite_nonnegative(input.scroll_cell_width)?,
        padding_x: finite_nonnegative(input.padding_x)?,
        padding_y: finite_nonnegative(input.padding_y)?,
        row_gap: finite_nonnegative(input.row_gap)?,
        column_gap: finite_nonnegative(input.column_gap)?,
        item_padding_x: finite_nonnegative(input.item_padding_x)?,
        item_padding_y: finite_nonnegative(input.item_padding_y)?,
        label_gap: finite_nonnegative(input.label_gap)?,
        candidate_font_size: finite_nonnegative(input.candidate_font_size)?,
        theme: match input.theme {
            0 => CandidateTheme::Light,
            1 => CandidateTheme::Dark,
            2 => CandidateTheme::HighContrast,
            _ => return None,
        },
        opacity: finite_nonnegative(input.opacity)?,
    };
    Some(CandidateUiInput {
        snapshot,
        locale,
        caret: Point {
            x: finite(input.caret.x)?,
            y: finite(input.caret.y)?,
        },
        caret_height: finite_nonnegative(input.caret_height)?,
        work_area,
        candidate_bulk: input.candidate_bulk != 0,
        config,
    })
}

unsafe fn semantic_snapshot_from_ffi(
    snapshot: Fcitx5CandidateModelSnapshot,
) -> Option<CandidateSemanticSnapshot> {
    if snapshot.candidate_count > MAX_CANDIDATES {
        return None;
    }
    if snapshot.candidate_count != 0 && snapshot.candidates.is_null() {
        return None;
    }
    let candidates = if snapshot.candidate_count == 0 {
        &[][..]
    } else {
        // SAFETY: caller promises `candidate_count` readable records for this call.
        unsafe { std::slice::from_raw_parts(snapshot.candidates, snapshot.candidate_count) }
    };
    let mut semantic_candidates = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        semantic_candidates.push(CandidateSemanticItem {
            id: candidate.id,
            label: unsafe { utf8_from_ffi(candidate.label) }?.to_owned(),
            text: unsafe { utf8_from_ffi(candidate.text) }?.to_owned(),
            comment: unsafe { utf8_from_ffi(candidate.comment) }?.to_owned(),
        });
    }
    Some(CandidateSemanticSnapshot {
        identity: CandidateSnapshotIdentity {
            engine_epoch: snapshot.engine_epoch,
            context_id: snapshot.context_id,
            composition_id: snapshot.composition_id,
            revision: snapshot.revision,
        },
        preedit: unsafe { utf8_from_ffi(snapshot.preedit) }?.to_owned(),
        auxiliary_up: unsafe { utf8_from_ffi(snapshot.auxiliary_up) }?.to_owned(),
        auxiliary_down: unsafe { utf8_from_ffi(snapshot.auxiliary_down) }?.to_owned(),
        candidates: semantic_candidates,
        selected: (snapshot.has_selected != 0).then_some(snapshot.selected),
        page: snapshot.page,
        total: snapshot.total,
        visibility: snapshot.visibility,
        popup_allowed: snapshot.popup_allowed != 0,
    })
}

unsafe fn utf8_from_ffi<'a>(value: Fcitx5CandidateUtf8) -> Option<&'a str> {
    if value.len > MAX_CANDIDATE_TEXT_UTF8 {
        return None;
    }
    if value.ptr.is_null() {
        return (value.len == 0).then_some("");
    }
    // SAFETY: caller guarantees a readable span for this ABI call; it is copied immediately.
    let bytes = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    if bytes.contains(&0) {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

fn measurement_from_ffi(value: &Fcitx5CandidateUiMeasurement) -> Option<CandidateUiMeasurement> {
    Some(CandidateUiMeasurement::new(
        finite_nonnegative(value.label_width)?,
        finite_nonnegative(value.text_width)?,
        finite_nonnegative(value.comment_width)?,
        finite_nonnegative(value.height)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_plan_handles_empty_output_buffers_without_dereferencing_null() {
        let state = fcitx5_candidate_ui_create();
        assert!(!state.is_null());
        let mut output = Fcitx5CandidateUiPlanOutput::default();
        let result = unsafe {
            fcitx5_candidate_ui_build_plan(
                state,
                std::ptr::null(),
                0,
                &mut output,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                0,
            )
        };
        assert_eq!(result, 1);
        assert_eq!(output.item_count, 0);
        assert_eq!(output.uia_item_count, 0);
        unsafe { fcitx5_candidate_ui_destroy(state) };
    }

    #[test]
    fn build_plan_rejects_non_finite_measurements() {
        let state = fcitx5_candidate_ui_create();
        let mut output = Fcitx5CandidateUiPlanOutput::default();
        let measurements = [Fcitx5CandidateUiMeasurement {
            text_width: f32::NAN,
            ..Fcitx5CandidateUiMeasurement::default()
        }];
        let result = unsafe {
            fcitx5_candidate_ui_build_plan(
                state,
                measurements.as_ptr(),
                measurements.len(),
                &mut output,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                0,
            )
        };
        assert_eq!(result, 0);
        unsafe { fcitx5_candidate_ui_destroy(state) };
    }

    #[test]
    fn plan_output_retains_the_applied_snapshot_identity() {
        let snapshot = Fcitx5CandidateModelSnapshot {
            engine_epoch: 7,
            context_id: 8,
            composition_id: 9,
            revision: 10,
            preedit: Fcitx5CandidateUtf8::default(),
            auxiliary_up: Fcitx5CandidateUtf8::default(),
            auxiliary_down: Fcitx5CandidateUtf8::default(),
            candidates: std::ptr::null(),
            candidate_count: 0,
            selected: 0,
            has_selected: 0,
            page: 0,
            total: 0,
            visibility: 0,
            popup_allowed: 1,
        };
        let input = Fcitx5CandidateUiInput {
            snapshot,
            locale: Fcitx5CandidateUtf8::default(),
            caret: Fcitx5CandidateLayoutPoint::default(),
            caret_height: 0.0,
            work_area: Fcitx5CandidateLayoutRect {
                right: 1.0,
                bottom: 1.0,
                ..Fcitx5CandidateLayoutRect::default()
            },
            candidate_bulk: 0,
            orientation: 0,
            scroll_mode: 0,
            page_size: 1,
            max_width: 1.0,
            scroll_cell_width: 1.0,
            padding_x: 0.0,
            padding_y: 0.0,
            row_gap: 0.0,
            column_gap: 0.0,
            item_padding_x: 0.0,
            item_padding_y: 0.0,
            label_gap: 0.0,
            candidate_font_size: 1.0,
            theme: 0,
            opacity: 1.0,
        };
        let state = fcitx5_candidate_ui_create();
        assert_eq!(unsafe { fcitx5_candidate_ui_apply(state, &input) }, 0);
        let mut output = Fcitx5CandidateUiPlanOutput::default();
        assert_eq!(
            unsafe {
                fcitx5_candidate_ui_build_plan(
                    state,
                    std::ptr::null(),
                    0,
                    &mut output,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    0,
                )
            },
            1
        );
        assert_eq!(
            (
                output.engine_epoch,
                output.context_id,
                output.composition_id,
                output.revision
            ),
            (7, 8, 9, 10)
        );
        unsafe { fcitx5_candidate_ui_destroy(state) };
    }
}

fn finite(value: f32) -> Option<f32> {
    value.is_finite().then_some(value)
}

fn finite_nonnegative(value: f32) -> Option<f32> {
    (value.is_finite() && value >= 0.0).then_some(value)
}

fn rect_from_ffi(value: Fcitx5CandidateLayoutRect) -> Option<Rect> {
    let rect = Rect {
        left: finite(value.left)?,
        top: finite(value.top)?,
        right: finite(value.right)?,
        bottom: finite(value.bottom)?,
    };
    (rect.right > rect.left && rect.bottom > rect.top).then_some(rect)
}

fn ffi_utf8(value: &str) -> Fcitx5CandidateUtf8 {
    Fcitx5CandidateUtf8 {
        ptr: value.as_ptr(),
        len: value.len(),
    }
}

fn rect_to_ffi(value: Rect) -> Fcitx5CandidateLayoutRect {
    Fcitx5CandidateLayoutRect {
        left: value.left,
        top: value.top,
        right: value.right,
        bottom: value.bottom,
    }
}

fn color_to_ffi(value: CandidateUiColor) -> Fcitx5CandidateUiColor {
    Fcitx5CandidateUiColor {
        red: value.red,
        green: value.green,
        blue: value.blue,
        alpha: value.alpha,
    }
}

fn plan_output(
    plan: &CandidateUiPlan,
    identity: CandidateSnapshotIdentity,
) -> Fcitx5CandidateUiPlanOutput {
    Fcitx5CandidateUiPlanOutput {
        engine_epoch: identity.engine_epoch,
        context_id: identity.context_id,
        composition_id: identity.composition_id,
        revision: identity.revision,
        popup_visible: u8::from(plan.popup_visible),
        orientation: match plan.orientation {
            Orientation::Vertical => 0,
            Orientation::Horizontal => 1,
        },
        placement: match plan.placement {
            crate::Placement::Unlocked => 0,
            crate::Placement::Below => 1,
            crate::Placement::Above => 2,
        },
        window: rect_to_ffi(plan.window),
        preedit: ffi_utf8(&plan.preedit),
        preedit_rect: plan
            .preedit_rect
            .map_or_else(Fcitx5CandidateLayoutRect::default, rect_to_ffi),
        has_preedit_rect: u8::from(plan.preedit_rect.is_some()),
        background: color_to_ffi(plan.colors.background),
        border: color_to_ffi(plan.colors.border),
        text: color_to_ffi(plan.colors.text),
        annotation: color_to_ffi(plan.colors.annotation),
        label: color_to_ffi(plan.colors.label),
        selected_background: color_to_ffi(plan.colors.selected_background),
        selected_text: color_to_ffi(plan.colors.selected_text),
        scrollbar: color_to_ffi(plan.colors.scrollbar),
        opacity: plan.opacity,
        scrollbar_track: plan
            .scrollbar_track
            .map_or_else(Fcitx5CandidateLayoutRect::default, rect_to_ffi),
        scrollbar_thumb: plan
            .scrollbar_thumb
            .map_or_else(Fcitx5CandidateLayoutRect::default, rect_to_ffi),
        has_scrollbar: u8::from(plan.scrollbar_track.is_some() && plan.scrollbar_thumb.is_some()),
        item_count: plan.items.len(),
        uia_item_count: plan.uia.items.len(),
    }
}
