//! Pointer, wheel, click-guard and focus-watch state for the candidate window, plus hit testing and selection intents.

#![forbid(unsafe_code)]

use super::*;

#[derive(Default)]
pub struct CandidateScrollState {
    /// Manual scroll offset in DIP px; `None` mirrors the C++ sentinel of a
    /// never-scrolled viewport (auto scroll-into-view).
    override_px: Option<f32>,
}

impl CandidateScrollState {
    const LINE_HEIGHT: f32 = 28.0;

    /// One wheel step from the viewport's current offset, kept inside
    /// `0..=max`. Turning the wheel forward (positive delta) scrolls back
    /// toward the first candidates.
    pub fn wheel_viewport(&mut self, current: f32, max: f32, delta: i32) -> f32 {
        let next = (current - (delta as f32 / 120.0) * Self::LINE_HEIGHT).clamp(0.0, max.max(0.0));
        self.override_px = Some(next);
        next
    }

    #[must_use]
    pub(crate) fn override_px(&self) -> f32 {
        self.override_px.unwrap_or(-1.0)
    }

    pub fn reset(&mut self) {
        self.override_px = None;
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

    pub(crate) fn set_target(&mut self, pid: u32) {
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
