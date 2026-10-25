//! Pointer, wheel, click-guard and focus-watch state for the candidate window, plus hit testing and selection intents.

#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

#[derive(Default)]
pub struct CandidateScrollState {
    /// Manual scroll offset in DIP px; `None` mirrors the C++ sentinel of a
    /// never-scrolled viewport (auto scroll-into-view).
    override_px: Option<f32>,
}

impl CandidateScrollState {
    const LINE_HEIGHT: f32 = 28.0;
    const MAX_OFFSET: f32 = 8000.0;

    pub fn wheel(&mut self, delta: i32) -> f32 {
        let current = self.override_px.unwrap_or(0.0);
        let next =
            (current + (delta as f32 / 120.0) * Self::LINE_HEIGHT).clamp(0.0, Self::MAX_OFFSET);
        self.override_px = Some(next);
        next
    }

    /// One wheel step from the viewport's current offset, kept inside
    /// `0..=max`. Turning the wheel forward (positive delta) scrolls back
    /// toward the first candidates.
    pub fn wheel_viewport(&mut self, current: f32, max: f32, delta: i32) -> f32 {
        let next = (current - (delta as f32 / 120.0) * Self::LINE_HEIGHT).clamp(0.0, max.max(0.0));
        self.override_px = Some(next);
        next
    }

    #[must_use]
    pub fn override_px(&self) -> f32 {
        self.override_px.unwrap_or(-1.0)
    }

    pub fn reset(&mut self) {
        self.override_px = None;
    }
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_scroll_state_create() -> *mut c_void {
    Box::into_raw(Box::<CandidateScrollState>::default()).cast()
}

#[no_mangle]
/// # Safety
///
/// `state` must be null or a pointer returned by
/// `fcitx5_candidate_scroll_state_create` that has not already been destroyed.
pub unsafe extern "C" fn fcitx5_candidate_scroll_state_destroy(state: *mut c_void) {
    if !state.is_null() {
        // SAFETY: caller guarantees this is the unique allocation from create.
        unsafe { drop(Box::from_raw(state.cast::<CandidateScrollState>())) };
    }
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_scroll_state_create`.
pub unsafe extern "C" fn fcitx5_candidate_scroll_state_wheel(
    state: *mut c_void,
    delta: i32,
) -> f32 {
    if state.is_null() {
        return 0.0;
    }
    // SAFETY: caller guarantees the state allocation is valid for this call.
    unsafe { (&mut *state.cast::<CandidateScrollState>()).wheel(delta) }
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_scroll_state_create`.
pub unsafe extern "C" fn fcitx5_candidate_scroll_state_override(state: *mut c_void) -> f32 {
    if state.is_null() {
        return -1.0;
    }
    // SAFETY: caller guarantees the state allocation is valid for this call.
    unsafe { (*(state.cast::<CandidateScrollState>())).override_px() }
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_scroll_state_create`.
pub unsafe extern "C" fn fcitx5_candidate_scroll_state_reset(state: *mut c_void) {
    if !state.is_null() {
        // SAFETY: caller guarantees the state allocation is valid for this call.
        unsafe { (&mut *state.cast::<CandidateScrollState>()).reset() };
    }
}

#[cfg(test)]
mod candidate_focus_watch_tests {
    use super::CandidateFocusWatchState;

    #[test]
    fn validity_follows_captured_target_and_interaction_contract() {
        let mut state = CandidateFocusWatchState::default();
        assert!(!state.is_valid(1234, false), "zero target is invalid");
        assert!(
            state.is_valid(1234, true),
            "interaction test is always valid"
        );
        state.target_process_id = 1234;
        assert!(state.is_valid(1234, false));
        assert!(!state.is_valid(5678, false));
        state.target_process_id = 0;
        assert!(state.should_dismiss(0, true), "broadcast pid 0 dismisses");
        state.target_process_id = 1234;
        assert!(state.should_dismiss(1234, true));
        assert!(!state.should_dismiss(5678, true));
        assert!(!state.should_dismiss(1234, false));
    }
}

#[cfg(test)]
mod candidate_click_guard_tests {
    use super::CandidateClickGuardState;

    #[test]
    fn begin_rejects_concurrent_clicks_and_timer_expires_the_guard() {
        let mut state = CandidateClickGuardState::default();
        assert!(state.begin());
        assert!(!state.begin(), "second click during flight is rejected");
        assert!(state.expire(), "timer expiry releases the guard");
        assert!(!state.expire(), "expiry without in-flight click");
        assert!(state.begin(), "clear guard accepts new clicks");
        state.clear();
        assert!(!state.expire());
        assert!(state.begin());
    }
}

#[cfg(test)]
mod candidate_scroll_state_tests {
    use super::CandidateScrollState;

    #[test]
    fn wheel_accumulates_clamped_line_offsets() {
        let mut state = CandidateScrollState::default();
        assert_eq!(state.override_px(), -1.0);
        assert_eq!(state.wheel(120), 28.0);
        assert_eq!(state.wheel(-120), 0.0);
        assert_eq!(state.wheel(-120), 0.0, "clamped at zero");
        state.reset();
        assert_eq!(state.override_px(), -1.0, "reset restores auto scroll");
    }

    #[test]
    fn viewport_wheel_starts_from_the_shown_offset_and_stays_in_range() {
        let mut state = CandidateScrollState::default();
        assert_eq!(
            state.wheel_viewport(100.0, 200.0, -120),
            128.0,
            "back = down"
        );
        assert_eq!(
            state.wheel_viewport(100.0, 200.0, 120),
            72.0,
            "forward = up"
        );
        assert_eq!(state.wheel_viewport(190.0, 200.0, -240), 200.0);
        assert_eq!(state.wheel_viewport(10.0, 200.0, 120), 0.0);
        assert_eq!(state.override_px(), 0.0);
    }

    #[test]
    fn wheel_respects_maximum_offset() {
        let mut state = CandidateScrollState::default();
        for _ in 0..10_000 {
            state.wheel(120);
        }
        assert_eq!(state.override_px(), CandidateScrollState::MAX_OFFSET);
    }
}

#[derive(Default)]
pub struct CandidateClickGuardState {
    in_flight: bool,
}

impl CandidateClickGuardState {
    /// Begins a guarded click; returns `false` when one is already in flight.
    pub fn begin(&mut self) -> bool {
        !std::mem::replace(&mut self.in_flight, true)
    }

    pub fn clear(&mut self) {
        self.in_flight = false;
    }

    /// Expires the guard timer; returns whether a click was in flight.
    pub fn expire(&mut self) -> bool {
        std::mem::take(&mut self.in_flight)
    }
}

#[derive(Default)]
pub struct CandidateFocusWatchState {
    /// Foreground process id captured when the popup was presented; 0 = none.
    target_process_id: u32,
}

impl CandidateFocusWatchState {
    #[must_use]
    pub fn target(&self) -> u32 {
        self.target_process_id
    }

    pub fn reset(&mut self) {
        self.target_process_id = 0;
    }

    pub fn set_target(&mut self, pid: u32) {
        self.target_process_id = pid;
    }
}

impl CandidateFocusWatchState {
    /// Whether `foreground_pid` still matches the presented target.mirrors the
    /// C++ contract: interaction tests always report valid, and a zero target
    /// is never valid.
    #[must_use]
    pub fn is_valid(&self, foreground_pid: u32, interaction_test: bool) -> bool {
        if interaction_test {
            return true;
        }
        self.target_process_id != 0 && self.target_process_id == foreground_pid
    }

    /// Whether a dismissal broadcast should dismiss this popup.
    #[must_use]
    pub fn should_dismiss(&self, broadcast_pid: u32, same_context: bool) -> bool {
        (broadcast_pid == 0 || broadcast_pid == self.target_process_id) && same_context
    }
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_focus_watch_create() -> *mut c_void {
    Box::into_raw(Box::<CandidateFocusWatchState>::default()).cast()
}

#[no_mangle]
/// # Safety
///
/// `state` must be null or a pointer returned by
/// `fcitx5_candidate_focus_watch_create` that has not already been destroyed.
pub unsafe extern "C" fn fcitx5_candidate_focus_watch_destroy(state: *mut c_void) {
    if !state.is_null() {
        // SAFETY: caller guarantees this is the unique allocation from create.
        unsafe { drop(Box::from_raw(state.cast::<CandidateFocusWatchState>())) };
    }
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_focus_watch_create`.
pub unsafe extern "C" fn fcitx5_candidate_focus_watch_set_target(state: *mut c_void, pid: u32) {
    if !state.is_null() {
        // SAFETY: caller guarantees the state allocation is valid for this call.
        unsafe { (*state.cast::<CandidateFocusWatchState>()).target_process_id = pid };
    }
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_focus_watch_create`.
pub unsafe extern "C" fn fcitx5_candidate_focus_watch_target(state: *mut c_void) -> u32 {
    if state.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees the state allocation is valid for this call.
    unsafe { (*state.cast::<CandidateFocusWatchState>()).target_process_id }
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_focus_watch_create`.
pub unsafe extern "C" fn fcitx5_candidate_focus_watch_is_valid(
    state: *mut c_void,
    foreground_pid: u32,
    interaction_test: u8,
) -> u8 {
    if state.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees the state allocation is valid for this call.
    let state = unsafe { &*state.cast::<CandidateFocusWatchState>() };
    u8::from(state.is_valid(foreground_pid, interaction_test != 0))
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_focus_watch_create`.
pub unsafe extern "C" fn fcitx5_candidate_focus_watch_should_dismiss(
    state: *mut c_void,
    broadcast_pid: u32,
    same_context: u8,
) -> u8 {
    if state.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees the state allocation is valid for this call.
    let state = unsafe { &*state.cast::<CandidateFocusWatchState>() };
    u8::from(state.should_dismiss(broadcast_pid, same_context != 0))
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_focus_watch_create`.
pub unsafe extern "C" fn fcitx5_candidate_focus_watch_reset(state: *mut c_void) {
    if !state.is_null() {
        // SAFETY: caller guarantees the state allocation is valid for this call.
        unsafe { (*state.cast::<CandidateFocusWatchState>()).target_process_id = 0 };
    }
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_click_guard_create() -> *mut c_void {
    Box::into_raw(Box::<CandidateClickGuardState>::default()).cast()
}

#[no_mangle]
/// # Safety
///
/// `state` must be null or a pointer returned by
/// `fcitx5_candidate_click_guard_create` that has not already been destroyed.
pub unsafe extern "C" fn fcitx5_candidate_click_guard_destroy(state: *mut c_void) {
    if !state.is_null() {
        // SAFETY: caller guarantees this is the unique allocation from create.
        unsafe { drop(Box::from_raw(state.cast::<CandidateClickGuardState>())) };
    }
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_click_guard_create`.
pub unsafe extern "C" fn fcitx5_candidate_click_guard_begin(state: *mut c_void) -> u8 {
    if state.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees the state allocation is valid for this call.
    u8::from(unsafe { (&mut *state.cast::<CandidateClickGuardState>()).begin() })
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_click_guard_create`.
pub unsafe extern "C" fn fcitx5_candidate_click_guard_clear(state: *mut c_void) {
    if !state.is_null() {
        // SAFETY: caller guarantees the state allocation is valid for this call.
        unsafe { (&mut *state.cast::<CandidateClickGuardState>()).clear() };
    }
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_click_guard_create`.
pub unsafe extern "C" fn fcitx5_candidate_click_guard_expire(state: *mut c_void) -> u8 {
    if state.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees the state allocation is valid for this call.
    u8::from(unsafe { (&mut *state.cast::<CandidateClickGuardState>()).expire() })
}

#[derive(Default)]
pub struct CandidatePointerState {
    pressed: Option<usize>,
}

impl CandidatePointerState {
    pub fn press(&mut self, index: Option<usize>) {
        self.pressed = index;
    }

    pub fn release(&mut self, index: Option<usize>) -> Option<usize> {
        let pressed = self.pressed.take();
        if pressed == index {
            pressed
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.pressed = None;
    }
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_pointer_state_create() -> *mut c_void {
    Box::into_raw(Box::<CandidatePointerState>::default()).cast()
}

#[no_mangle]
/// # Safety
///
/// `state` must be null or a pointer returned by
/// `fcitx5_candidate_pointer_state_create` that has not already been destroyed.
pub unsafe extern "C" fn fcitx5_candidate_pointer_state_destroy(state: *mut c_void) {
    if !state.is_null() {
        // SAFETY: caller guarantees this is the unique allocation from create.
        unsafe { drop(Box::from_raw(state.cast::<CandidatePointerState>())) };
    }
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_pointer_state_create`.
pub unsafe extern "C" fn fcitx5_candidate_pointer_state_press(
    state: *mut c_void,
    has_index: u8,
    index: usize,
) {
    if state.is_null() {
        return;
    }
    // SAFETY: caller guarantees the state allocation is valid for this call.
    unsafe {
        (&mut *state.cast::<CandidatePointerState>()).press((has_index != 0).then_some(index))
    };
}

#[no_mangle]
/// # Safety
///
/// `state` and `out_index` must be valid pointers. The state must have been
/// returned by `fcitx5_candidate_pointer_state_create`.
pub unsafe extern "C" fn fcitx5_candidate_pointer_state_release(
    state: *mut c_void,
    has_index: u8,
    index: usize,
    out_index: *mut usize,
) -> u8 {
    if state.is_null() || out_index.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees the state allocation is valid for this call.
    let selected = unsafe {
        (&mut *state.cast::<CandidatePointerState>()).release((has_index != 0).then_some(index))
    };
    let Some(selected) = selected else {
        return 0;
    };
    // SAFETY: non-null checked above and caller provides writable output storage.
    unsafe { *out_index = selected };
    1
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_pointer_state_create`.
pub unsafe extern "C" fn fcitx5_candidate_pointer_state_clear(state: *mut c_void) {
    if !state.is_null() {
        // SAFETY: caller guarantees the state allocation is valid for this call.
        unsafe { (&mut *state.cast::<CandidatePointerState>()).clear() };
    }
}

#[cfg(test)]
mod candidate_pointer_state_tests {
    use super::CandidatePointerState;

    #[test]
    fn release_dispatches_only_the_same_pressed_candidate() {
        let mut state = CandidatePointerState::default();
        state.press(Some(3));
        assert_eq!(state.release(Some(3)), Some(3));
        state.press(Some(3));
        assert_eq!(state.release(Some(2)), None);
        state.press(Some(3));
        state.clear();
        assert_eq!(state.release(Some(3)), None);
    }
}

#[no_mangle]
/// # Safety
///
/// `rects` must be valid for `rect_count` elements when `rect_count` is
/// non-zero. `out_index` must point to writable storage. Pointers are not
/// retained.
pub unsafe extern "C" fn fcitx5_candidate_hit_test(
    rects: *const Fcitx5CandidateLayoutRect,
    rect_count: usize,
    x: f32,
    y: f32,
    out_index: *mut usize,
) -> u8 {
    if out_index.is_null() || (rect_count > 0 && rects.is_null()) {
        return 0;
    }
    let rects = if rect_count == 0 {
        &[]
    } else {
        // SAFETY: the validated rect pointer covers `rect_count` readable rectangles.
        unsafe { std::slice::from_raw_parts(rects, rect_count) }
    };
    for (index, rect) in rects.iter().enumerate() {
        if x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom {
            // SAFETY: the non-null output pointer is writable for one index.
            unsafe {
                *out_index = index;
            }
            return 1;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_selection_intent(
    target_process_id: u32,
    engine_epoch: u64,
    context_id: u64,
    composition_id: u64,
    revision: u64,
    candidate_id: u64,
) -> Fcitx5CandidateSelectionIntent {
    let intent = Fcitx5CandidateSelectionIntent {
        target_process_id,
        engine_epoch,
        context_id,
        composition_id,
        revision,
        candidate_id,
    };
    if selection_intent_valid(intent) {
        intent
    } else {
        Fcitx5CandidateSelectionIntent::default()
    }
}

pub fn selection_intent_valid(intent: Fcitx5CandidateSelectionIntent) -> bool {
    intent.target_process_id != 0
        && intent.engine_epoch != 0
        && intent.context_id != 0
        && intent.composition_id != 0
        && intent.revision != 0
        && intent.candidate_id != 0
}
