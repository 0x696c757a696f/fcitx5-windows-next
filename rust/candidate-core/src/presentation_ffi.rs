//! C ABI over the candidate presentation state.

#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

#[no_mangle]
pub extern "C" fn fcitx5_candidate_presentation_create() -> *mut c_void {
    Box::into_raw(Box::<CandidatePresentationState>::default()) as *mut c_void
}

#[no_mangle]
/// # Safety
///
/// `state` must be either null or a pointer returned by
/// `fcitx5_candidate_presentation_create` that has not already been destroyed.
pub unsafe extern "C" fn fcitx5_candidate_presentation_destroy(state: *mut c_void) {
    if state.is_null() {
        return;
    }
    // SAFETY: the API contract makes this a unique allocation from the matching create export.
    drop(unsafe { Box::from_raw(state.cast::<CandidatePresentationState>()) });
}

#[no_mangle]
/// # Safety
///
/// `state` must be either null or a valid pointer returned by
/// `fcitx5_candidate_presentation_create`.
pub unsafe extern "C" fn fcitx5_candidate_presentation_reset(state: *mut c_void) {
    if state.is_null() {
        return;
    }
    // SAFETY: the API contract supplies a live uniquely borrowed presentation allocation.
    unsafe { &mut *state.cast::<CandidatePresentationState>() }.reset();
}

#[no_mangle]
/// # Safety
///
/// `state` and `input` must be valid pointers for the duration of this call.
/// Neither pointer is retained.
pub unsafe extern "C" fn fcitx5_candidate_presentation_apply(
    state: *mut c_void,
    input: *const CandidatePresentationUpdate,
) -> u32 {
    if state.is_null() || input.is_null() {
        return 3;
    }
    // SAFETY: the API contract supplies live state/input storage for this call without retained pointers.
    unsafe { &mut *state.cast::<CandidatePresentationState>() }.apply(unsafe { *input })
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_presentation_create`, and `output` must point to writable
/// storage for one output value.
pub unsafe extern "C" fn fcitx5_candidate_presentation_current(
    state: *mut c_void,
    output: *mut CandidatePresentationOutput,
) -> u8 {
    if state.is_null() || output.is_null() {
        return 0;
    }
    // SAFETY: the API contract supplies live state plus writable output storage.
    unsafe {
        *output = (&*state.cast::<CandidatePresentationState>()).output();
    }
    1
}

#[no_mangle]
/// Writes the Rust-owned visible candidate order for the current presentation.
///
/// # Safety
///
/// `state` must be a valid presentation pointer, `output` must point to writable
/// storage, and `indices` must point to `capacity` writable `usize` values when
/// `capacity` is non-zero. No pointer is retained.
pub unsafe extern "C" fn fcitx5_candidate_presentation_render_plan(
    state: *mut c_void,
    indices: *mut usize,
    capacity: usize,
    output: *mut Fcitx5CandidatePresentationRenderPlan,
) -> u8 {
    if state.is_null() || output.is_null() || (capacity != 0 && indices.is_null()) {
        return 0;
    }
    // SAFETY: the API contract supplies a live presentation allocation for this read.
    let state = unsafe { &*state.cast::<CandidatePresentationState>() };
    let (start, count) = if state.scroll_mode {
        (0, state.candidate_count)
    } else {
        (state.ordinary_start, state.ordinary_count)
    };
    let Some(end) = start.checked_add(count) else {
        return 0;
    };
    if end > state.candidate_count || count > capacity {
        return 0;
    }
    let target = if count == 0 {
        &mut []
    } else {
        // SAFETY: non-null was checked above and `count <= capacity` was verified.
        unsafe { std::slice::from_raw_parts_mut(indices, count) }
    };
    for (slot, candidate_index) in target.iter_mut().zip(start..end) {
        *slot = candidate_index;
    }
    // SAFETY: the API contract supplies writable output storage for one render plan.
    unsafe {
        *output = Fcitx5CandidatePresentationRenderPlan {
            selected: state.selected.unwrap_or_default(),
            has_selected: u8::from(state.selected.is_some()),
            render_count: count,
        };
    }
    1
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_presentation_create`.
pub unsafe extern "C" fn fcitx5_candidate_presentation_set_placement(
    state: *mut c_void,
    placement: u32,
) -> u8 {
    let Some(placement) = placement_from_ffi(placement) else {
        return 0;
    };
    if state.is_null() {
        return 0;
    }
    // SAFETY: the API contract supplies a live uniquely borrowed presentation allocation.
    unsafe { &mut *state.cast::<CandidatePresentationState>() }.set_placement(placement);
    1
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_presentation_create`. Candidate arrays and locale bytes
/// must be valid for their declared lengths and are not retained.
pub unsafe extern "C" fn fcitx5_candidate_presentation_resolve_orientation(
    state: *mut c_void,
    configured: u32,
    candidates: *const Fcitx5CandidatePresentationText,
    candidate_count: usize,
    locale: Fcitx5CandidateUtf8,
    work_area: Fcitx5CandidateLayoutRect,
    caret_x: f32,
    scale: f32,
    page_size: u32,
) -> u32 {
    if state.is_null() || (candidate_count != 0 && candidates.is_null()) {
        return 0;
    }
    let Some(configured) = (match configured {
        0 => Some(PresentationOrientation::Automatic),
        1 => Some(PresentationOrientation::Vertical),
        2 => Some(PresentationOrientation::Horizontal),
        _ => None,
    }) else {
        return 0;
    };
    // SAFETY: the ABI keeps the locale byte span readable for this conversion and it is not retained.
    let locale = unsafe { bytes_from_ffi(locale) }
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .unwrap_or_default();
    let candidate_inputs = if candidate_count == 0 {
        &[]
    } else {
        // SAFETY: the validated candidates pointer covers `candidate_count` readable records.
        unsafe { std::slice::from_raw_parts(candidates, candidate_count) }
    };
    let Some(candidates) = candidate_inputs
        .iter()
        .map(|candidate| {
            // SAFETY: the ABI keeps each candidate text span readable during this owned conversion.
            let text = unsafe { bytes_from_ffi(candidate.text) }?;
            // SAFETY: the ABI keeps each candidate comment span readable during this owned conversion.
            let comment = unsafe { bytes_from_ffi(candidate.comment) }?;
            Some(CandidateText {
                text: std::str::from_utf8(text).ok()?.to_owned(),
                comment: std::str::from_utf8(comment).ok()?.to_owned(),
            })
        })
        .collect::<Option<Vec<_>>>()
    else {
        return 0;
    };
    // SAFETY: the non-null state pointer is a uniquely borrowed presentation allocation for this call.
    let orientation = unsafe { &mut *state.cast::<CandidatePresentationState>() }
        .layout
        .resolve_orientation(
            configured,
            AutomaticOrientationInput {
                candidates: &candidates,
                locale,
                work_area: Rect {
                    left: work_area.left,
                    top: work_area.top,
                    right: work_area.right,
                    bottom: work_area.bottom,
                },
                caret_x,
                scale,
                page_size,
            },
        );
    match orientation {
        Orientation::Vertical => 0,
        Orientation::Horizontal => 1,
    }
}

#[cfg(test)]
mod candidate_presentation_decision_tests {
    use super::{CandidatePresentationDecision, CandidatePresentationState};

    #[test]
    fn decide_follows_the_frozen_update_dismiss_chain() {
        let state = CandidatePresentationState::default();
        assert_eq!(
            state.decide(0, 5, true, true),
            1,
            "hidden visibility dismisses"
        );
        assert_eq!(
            state.decide(1, 0, true, true),
            1,
            "empty candidates dismiss"
        );
        assert_eq!(
            state.decide(1, 5, false, true),
            1,
            "invalid caret dismisses"
        );
        assert_eq!(
            state.decide(1, 5, true, false),
            2,
            "popup-blocked composition only hides"
        );
        assert_eq!(state.decide(2, 5, true, true), 0, "normal update paints");
    }

    #[test]
    fn decision_abi_rejects_null_state_or_output() {
        let state = CandidatePresentationState::default();
        let mut decision = CandidatePresentationDecision::default();
        // SAFETY: this test passes a live presentation state and writable decision output.
        let ok = unsafe {
            super::fcitx5_candidate_presentation_decide(
                (&state as *const CandidatePresentationState).cast(),
                1,
                5,
                1,
                1,
                0,
                &mut decision,
            )
        };
        assert_eq!(ok, 1);
        assert_eq!(decision.action, 0);
    }
}
