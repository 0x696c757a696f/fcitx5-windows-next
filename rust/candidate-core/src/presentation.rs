//! Candidate presentation state: orientation decisions, placement and composition stability.

#![forbid(unsafe_code)]

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CandidateText {
    pub text: String,
    pub comment: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct CompositionIdentity {
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AutomaticOrientationInput<'a> {
    pub candidates: &'a [CandidateText],
    pub locale: &'a str,
    pub work_area: Rect,
    pub caret_x: f32,
    pub scale: f32,
    pub page_size: u32,
}

impl CandidatePresentationState {
    /// Applies the frozen update-dismiss decision chain from the shipping C++
    /// `update()`: stale/invalid never re-decide, hidden/empty/invalid-caret
    /// dismiss, and a popup-blocked composition only hides.
    pub(crate) fn decide(
        &self,
        visibility: u8,
        candidate_count: usize,
        caret_valid: bool,
        popup_allowed: bool,
    ) -> u8 {
        if visibility == 0 || candidate_count == 0 || !caret_valid {
            return 1;
        }
        if !popup_allowed {
            return 2;
        }
        0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CandidatePresentationUpdate {
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub selected: usize,
    pub has_selected: u8,
    pub candidate_count: usize,
    pub page: u32,
    pub page_size: u32,
    pub candidate_bulk: u8,
    pub configured_scroll_mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CandidatePresentationOutput {
    pub selected: usize,
    pub has_selected: u8,
    pub scroll_mode: u8,
    pub scroll_expanded: u8,
    pub scroll_columns: usize,
    pub ordinary_start: usize,
    pub ordinary_count: usize,
    pub candidate_bulk: u8,
    pub page_size: u32,
    pub placement: u32,
}

#[derive(Clone, Debug)]
pub struct CandidatePresentationState {
    pub(crate) identity: Option<CompositionIdentity>,
    pub(crate) revision: u64,
    pub(crate) selected: Option<usize>,
    pub(crate) scroll_mode: bool,
    pub(crate) scroll_expanded: bool,
    pub(crate) scroll_columns: usize,
    pub(crate) ordinary_start: usize,
    pub(crate) ordinary_count: usize,
    pub(crate) candidate_count: usize,
    pub(crate) candidate_bulk: bool,
    pub(crate) page_size: u32,
    pub(crate) placement: Placement,
    pub(crate) layout: CompositionLayoutState,
}

impl Default for CandidatePresentationState {
    fn default() -> Self {
        Self {
            identity: None,
            revision: 0,
            selected: None,
            scroll_mode: false,
            scroll_expanded: false,
            scroll_columns: 6,
            ordinary_start: 0,
            ordinary_count: 0,
            candidate_count: 0,
            candidate_bulk: false,
            page_size: 0,
            placement: Placement::Unlocked,
            layout: CompositionLayoutState::default(),
        }
    }
}

impl CandidatePresentationState {
    /// Applies the presentation metadata for one accepted semantic snapshot.
    /// Returns 0 for applied, 1 for duplicate, 2 for stale, and 3 for invalid.
    pub(crate) fn apply(&mut self, input: CandidatePresentationUpdate) -> u32 {
        if input.engine_epoch == 0
            || input.context_id == 0
            || input.revision == 0
            || input.candidate_count > MAX_CANDIDATES
            || (input.has_selected != 0 && input.selected >= input.candidate_count)
        {
            return 3;
        }
        let identity = CompositionIdentity {
            engine_epoch: input.engine_epoch,
            context_id: input.context_id,
            composition_id: input.composition_id,
        };
        if let Some(previous) = self.identity {
            if input.engine_epoch < previous.engine_epoch
                || (input.engine_epoch == previous.engine_epoch
                    && input.context_id == previous.context_id
                    && input.composition_id != 0
                    && previous.composition_id != 0
                    && input.composition_id < previous.composition_id)
            {
                return 2;
            }
            if identity == previous {
                if input.revision == self.revision {
                    return 1;
                }
                if input.revision < self.revision {
                    return 2;
                }
            }
        }
        if self.identity != Some(identity) {
            self.identity = Some(identity);
            self.layout.begin(identity);
            self.revision = 0;
            self.scroll_expanded = false;
            self.placement = Placement::Unlocked;
        }
        self.revision = input.revision;
        self.candidate_bulk = input.candidate_bulk != 0;
        self.candidate_count = input.candidate_count;
        self.page_size = input.page_size;
        self.selected = (input.has_selected != 0).then_some(input.selected);
        self.scroll_columns = (input.page_size as usize).clamp(1, 9);
        let scroll_eligible = input.configured_scroll_mode != 0
            && input.candidate_bulk != 0
            && input.page_size != 0
            && input.candidate_count > input.page_size as usize;
        let focus_beyond_first_page = self
            .selected
            .is_some_and(|selected| selected >= input.page_size as usize);
        self.scroll_expanded =
            scroll_eligible && (self.scroll_expanded || input.page > 0 || focus_beyond_first_page);
        self.scroll_mode = scroll_eligible && self.scroll_expanded;
        self.ordinary_count = if input.page_size == 0 {
            input.candidate_count
        } else {
            (input.page_size as usize).min(input.candidate_count)
        };
        self.ordinary_start = if input.candidate_bulk != 0 && !self.scroll_mode {
            (input.page as usize)
                .saturating_mul(input.page_size as usize)
                .min(input.candidate_count - self.ordinary_count)
        } else {
            0
        };
        if !self.scroll_mode && input.candidate_bulk == 0 {
            if let Some(selected) = self.selected {
                if selected < self.ordinary_count
                    && self.ordinary_start + selected < input.candidate_count
                {
                    self.selected = Some(self.ordinary_start + selected);
                }
            }
        }
        0
    }

    pub fn output(&self) -> CandidatePresentationOutput {
        CandidatePresentationOutput {
            selected: self.selected.unwrap_or_default(),
            has_selected: u8::from(self.selected.is_some()),
            scroll_mode: u8::from(self.scroll_mode),
            scroll_expanded: u8::from(self.scroll_expanded),
            scroll_columns: self.scroll_columns,
            ordinary_start: self.ordinary_start,
            ordinary_count: self.ordinary_count,
            candidate_bulk: u8::from(self.candidate_bulk),
            page_size: self.page_size,
            placement: placement_to_ffi(self.placement),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn set_placement(&mut self, placement: Placement) {
        self.placement = placement;
    }

    /// Writes the presentation render order (candidate indices) into `indices`
    /// and returns its length, mirroring the C++ render-plan contract.
    pub(crate) fn render_plan(&self, indices: &mut [usize]) -> Option<usize> {
        let (start, count) = if self.scroll_mode {
            (0, self.candidate_count)
        } else {
            (self.ordinary_start, self.ordinary_count)
        };
        let end = start.checked_add(count)?;
        if end > self.candidate_count || count > indices.len() {
            return None;
        }
        for (slot, candidate_index) in indices[..count].iter_mut().zip(start..end) {
            *slot = candidate_index;
        }
        Some(count)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CompositionLayoutState {
    pub(crate) identity: Option<CompositionIdentity>,
    pub(crate) auto_orientation: Option<Orientation>,
}

impl CompositionLayoutState {
    pub fn begin(&mut self, identity: CompositionIdentity) {
        if self.identity != Some(identity) {
            self.identity = Some(identity);
            self.auto_orientation = None;
        }
    }

    pub(crate) fn resolve_orientation(
        &mut self,
        configured: PresentationOrientation,
        input: AutomaticOrientationInput<'_>,
    ) -> Orientation {
        match configured {
            PresentationOrientation::Vertical => Orientation::Vertical,
            PresentationOrientation::Horizontal => Orientation::Horizontal,
            PresentationOrientation::Automatic => {
                if let Some(orientation) = self.auto_orientation {
                    return orientation;
                }
                let orientation = resolve_automatic_orientation(
                    input.candidates,
                    input.locale,
                    input.work_area,
                    input.caret_x,
                    input.scale,
                    input.page_size,
                );
                self.auto_orientation = Some(orientation);
                orientation
            }
        }
    }
}

pub(crate) fn resolve_automatic_orientation(
    candidates: &[CandidateText],
    locale: &str,
    work_area: Rect,
    caret_x: f32,
    scale: f32,
    page_size: u32,
) -> Orientation {
    let mut has_long_annotation = false;
    let mut compact_candidates = !candidates.is_empty();
    for candidate in candidates {
        has_long_annotation = has_long_annotation || candidate.comment.len() > 18;
        compact_candidates =
            compact_candidates && candidate.text.len() <= 6 && candidate.comment.len() <= 18;
    }
    let near_right_threshold = 360.0 * scale;
    let edge_constrained = work_area.right - caret_x < near_right_threshold;
    let page_size = if page_size == 0 { 9 } else { page_size } as usize;
    let compact_cjk = locale_prefers_compact_horizontal(locale)
        && compact_candidates
        && candidates.len() <= page_size.max(1);
    if compact_cjk && !has_long_annotation && !edge_constrained {
        Orientation::Horizontal
    } else {
        Orientation::Vertical
    }
}

pub(crate) fn locale_prefers_compact_horizontal(locale: &str) -> bool {
    let lower = locale.to_ascii_lowercase();
    lower.starts_with("zh") || lower.starts_with("ja") || lower.starts_with("ko")
}

#[cfg(test)]
mod candidate_presentation_decision_tests {
    use super::CandidatePresentationState;

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
}
