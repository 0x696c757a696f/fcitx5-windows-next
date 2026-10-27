//! Candidate labels: styles, slot plans, scroll-label reservations and render segments.

#![forbid(unsafe_code)]

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Fcitx5CandidateScrollLabel {
    pub reserve: u8,
    pub show: u8,
    pub slot: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CandidateLabelStyle {
    Plain,
    Dot,
    Paren,
    Bracket,
    Circled,
}

pub(crate) fn scroll_label_policy(
    candidate_index: usize,
    selected_index: usize,
    page_size: usize,
    total_candidates: usize,
) -> Fcitx5CandidateScrollLabel {
    if page_size == 0 || total_candidates <= page_size || candidate_index >= total_candidates {
        return Fcitx5CandidateScrollLabel::default();
    }
    let slot = candidate_index % page_size + 1;
    Fcitx5CandidateScrollLabel {
        reserve: 1,
        show: (candidate_index / page_size == selected_index / page_size) as u8,
        slot: slot.min(u32::MAX as usize) as u32,
    }
}

/// Resolved per-candidate scroll-label reservation (081C glue slice).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Fcitx5CandidateScrollReservation {
    /// 0 skip, 1 reserve the slot, 2 reserve + show the label.
    pub action: u8,
    pub slot: u32,
}

/// Computes the scroll-label reservation for one candidate.
///
/// `source_label` is 0/1 whether the candidate carries its own label; only
/// generated candidates receive a synthesized scroll slot. Mirrors the
/// shipping C++ `applyScrollLabelReservations` loop body.
pub(crate) extern "C" fn fcitx5_candidate_scroll_reservation_for(
    candidate_index: usize,
    source_label: u8,
    selected_index: usize,
    scroll_mode: u8,
    labels_visible: u8,
    scroll_columns: usize,
    total_candidates: usize,
) -> Fcitx5CandidateScrollReservation {
    if source_label != 0 {
        return Fcitx5CandidateScrollReservation::default();
    }
    if scroll_mode == 0 || labels_visible == 0 || scroll_columns == 0 {
        return Fcitx5CandidateScrollReservation::default();
    }
    let policy = scroll_label_policy(
        candidate_index,
        selected_index,
        scroll_columns,
        total_candidates,
    );
    if policy.reserve == 0 {
        return Fcitx5CandidateScrollReservation::default();
    }
    Fcitx5CandidateScrollReservation {
        action: if policy.show != 0 { 2 } else { 1 },
        slot: policy.slot,
    }
}

pub(crate) fn format_candidate_label(
    slot: u32,
    source_label: &str,
    style: CandidateLabelStyle,
    custom_prefix: &str,
    custom_suffix: &str,
) -> String {
    let label = if source_label.is_empty() {
        if slot == 0 { 1 } else { slot }.to_string()
    } else {
        source_label.to_owned()
    };
    if !custom_prefix.is_empty() || !custom_suffix.is_empty() {
        return format!("{custom_prefix}{label}{custom_suffix}");
    }
    match style {
        CandidateLabelStyle::Plain => label,
        CandidateLabelStyle::Dot => format!("{label}."),
        CandidateLabelStyle::Paren => format!("({label})"),
        CandidateLabelStyle::Bracket => format!("[{label}]"),
        CandidateLabelStyle::Circled => {
            let mut chars = label.chars();
            match (chars.next(), chars.next()) {
                (Some(character @ '1'..='9'), None) => {
                    char::from_u32(0x2460 + character as u32 - '1' as u32)
                        .unwrap_or(character)
                        .to_string()
                }
                _ => label,
            }
        }
    }
}

pub(crate) fn label_style_from_ffi(value: u32) -> Option<CandidateLabelStyle> {
    match value {
        0 => Some(CandidateLabelStyle::Plain),
        1 => Some(CandidateLabelStyle::Dot),
        2 => Some(CandidateLabelStyle::Paren),
        3 => Some(CandidateLabelStyle::Bracket),
        4 => Some(CandidateLabelStyle::Circled),
        _ => None,
    }
}

#[cfg(test)]
mod candidate_scroll_reservation_tests {
    use super::fcitx5_candidate_scroll_reservation_for;

    #[test]
    fn reservations_follow_the_frozen_scroll_label_policy() {
        // Source labels keep their own label; no synthesized slot.
        let source = fcitx5_candidate_scroll_reservation_for(3, 1, 0, 1, 1, 6, 60);
        assert_eq!(source.action, 0);
        // Non-scroll or hidden labels never reserve.
        let off = fcitx5_candidate_scroll_reservation_for(3, 0, 0, 0, 1, 6, 60);
        assert_eq!(off.action, 0);
        let hidden = fcitx5_candidate_scroll_reservation_for(3, 0, 0, 1, 0, 6, 60);
        assert_eq!(hidden.action, 0);
        // Current page reserves and shows (slot = index % page + 1).
        let show = fcitx5_candidate_scroll_reservation_for(3, 0, 3, 1, 1, 6, 60);
        assert_eq!(show.action, 2);
        assert_eq!(show.slot, 4);
        // Other pages reserve but hide.
        let reserve = fcitx5_candidate_scroll_reservation_for(9, 0, 3, 1, 1, 6, 60);
        assert_eq!(reserve.action, 1);
        assert_eq!(reserve.slot, 4);
    }
}
