//! Candidate labels: styles, slot plans, scroll-label reservations and render segments.

#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateScrollLabel {
    pub reserve: u8,
    pub show: u8,
    pub slot: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabelStyle {
    Plain,
    Dot,
    Paren,
    Bracket,
    Circled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabelDisplay {
    Always,
    SelectedScope,
    Hidden,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabelScope {
    Item,
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabelAlign {
    Right,
    Left,
    Center,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabelWidthStrategy {
    Fixed,
    PageMax,
    GridMax,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateLabelSlotConfig {
    pub display: CandidateLabelDisplay,
    pub scope: CandidateLabelScope,
    pub reserve_when_hidden: bool,
    pub align: CandidateLabelAlign,
    pub width_strategy: CandidateLabelWidthStrategy,
    pub min_width: f32,
    pub gap: f32,
}

impl Default for CandidateLabelSlotConfig {
    fn default() -> Self {
        Self {
            display: CandidateLabelDisplay::Always,
            scope: CandidateLabelScope::Item,
            reserve_when_hidden: true,
            align: CandidateLabelAlign::Right,
            width_strategy: CandidateLabelWidthStrategy::PageMax,
            min_width: 0.0,
            gap: 4.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateLabelSlotSource {
    pub candidate_index: usize,
    pub row: usize,
    pub column: usize,
    pub label_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateLabelSlotPlanItem {
    pub reserve_label: bool,
    pub show_label: bool,
    pub label_slot_width: f32,
    pub label_gap: f32,
    pub label_align: CandidateLabelAlign,
    pub text_origin_offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateLabelSlotPlan {
    pub label_slot_width: f32,
    pub items: Vec<CandidateLabelSlotPlanItem>,
    pub stable_text_origin: bool,
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
pub struct Fcitx5CandidateScrollReservation {
    /// 0 skip, 1 reserve the slot, 2 reserve + show the label.
    pub action: u8,
    pub slot: u32,
}

/// Computes the scroll-label reservation for one candidate.
///
/// `source_label` is 0/1 whether the candidate carries its own label; only
/// generated candidates receive a synthesized scroll slot. Mirrors the
/// shipping C++ `applyScrollLabelReservations` loop body.
#[no_mangle]
pub extern "C" fn fcitx5_candidate_scroll_reservation_for(
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

pub fn format_candidate_label(
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

pub fn candidate_label_slot_plan(
    config: CandidateLabelSlotConfig,
    sources: &[CandidateLabelSlotSource],
    selected_index: usize,
) -> CandidateLabelSlotPlan {
    let widest_label = sources.iter().fold(0.0_f32, |width, source| {
        if source.label_width.is_finite() && source.label_width > 0.0 {
            width.max(source.label_width)
        } else {
            width
        }
    });
    let label_slot_width = match config.width_strategy {
        CandidateLabelWidthStrategy::Fixed => config.min_width.max(0.0),
        CandidateLabelWidthStrategy::PageMax | CandidateLabelWidthStrategy::GridMax => {
            config.min_width.max(0.0).max(widest_label)
        }
    };
    let selected = sources
        .iter()
        .find(|source| source.candidate_index == selected_index)
        .copied();
    let label_gap = config.gap.max(0.0);
    let items = sources
        .iter()
        .map(|source| {
            let selected_scope = match (config.scope, selected) {
                (CandidateLabelScope::Item, _) => source.candidate_index == selected_index,
                (CandidateLabelScope::Row, Some(selected)) => source.row == selected.row,
                (CandidateLabelScope::Column, Some(selected)) => source.column == selected.column,
                (_, None) => false,
            };
            let show_label = match config.display {
                CandidateLabelDisplay::Always => true,
                CandidateLabelDisplay::SelectedScope => selected_scope,
                CandidateLabelDisplay::Hidden => false,
            };
            let reserve_label = show_label || config.reserve_when_hidden;
            let text_origin_offset = if reserve_label && label_slot_width > 0.0 {
                label_slot_width + label_gap
            } else {
                0.0
            };
            CandidateLabelSlotPlanItem {
                reserve_label,
                show_label,
                label_slot_width,
                label_gap: if reserve_label { label_gap } else { 0.0 },
                label_align: config.align,
                text_origin_offset,
            }
        })
        .collect::<Vec<_>>();
    let mut text_origins = items
        .iter()
        .filter(|item| item.reserve_label)
        .map(|item| item.text_origin_offset);
    let first_origin = text_origins.next();
    let stable_text_origin = first_origin
        .is_none_or(|first| text_origins.all(|origin| (origin - first).abs() <= f32::EPSILON));
    CandidateLabelSlotPlan {
        label_slot_width,
        items,
        stable_text_origin,
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

#[no_mangle]
pub extern "C" fn fcitx5_candidate_scroll_label_policy(
    candidate_index: usize,
    selected_index: usize,
    page_size: usize,
    total_candidates: usize,
) -> Fcitx5CandidateScrollLabel {
    scroll_label_policy(candidate_index, selected_index, page_size, total_candidates)
}

/// # Safety
///
/// `source_label`, `custom_prefix`, and `custom_suffix` must point to readable
/// UTF-16 buffers for their declared lengths, or be null only when the length
/// is zero. `output` may be null for size queries or point to writable UTF-16
/// storage for `output_capacity` code units. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_format_label_utf16(
    slot: u32,
    style: u32,
    source_label: Fcitx5CandidateUtf16,
    custom_prefix: Fcitx5CandidateUtf16,
    custom_suffix: Fcitx5CandidateUtf16,
    output: *mut u16,
    output_capacity: usize,
) -> usize {
    let Some(style) = label_style_from_ffi(style) else {
        return 0;
    };
    // SAFETY: the ABI keeps the source label UTF-16 span readable for this owned conversion.
    let Some(source_label) = (unsafe { utf16_from_ffi(source_label) }) else {
        return 0;
    };
    // SAFETY: the ABI keeps the prefix UTF-16 span readable for this owned conversion.
    let Some(custom_prefix) = (unsafe { utf16_from_ffi(custom_prefix) }) else {
        return 0;
    };
    // SAFETY: the ABI keeps the suffix UTF-16 span readable for this owned conversion.
    let Some(custom_suffix) = (unsafe { utf16_from_ffi(custom_suffix) }) else {
        return 0;
    };
    let source_label = String::from_utf16_lossy(source_label);
    let custom_prefix = String::from_utf16_lossy(custom_prefix);
    let custom_suffix = String::from_utf16_lossy(custom_suffix);
    let formatted =
        format_candidate_label(slot, &source_label, style, &custom_prefix, &custom_suffix);
    write_wide_units(
        &formatted.encode_utf16().collect::<Vec<_>>(),
        output,
        output_capacity,
    )
}

pub fn candidate_render_segments(
    items: &[Fcitx5CandidateRenderItemInput],
) -> (Vec<Fcitx5CandidateRenderItemOutput>, f32) {
    const LABEL_CELL_SAFETY_PADDING: f32 = 2.0;
    let label_column_width = items.iter().fold(0.0_f32, |width, item| {
        if item.reserve_label != 0 || item.has_label != 0 {
            width.max(item.label_width.max(0.0) + LABEL_CELL_SAFETY_PADDING)
        } else {
            width
        }
    });
    let outputs = items
        .iter()
        .map(|source| {
            let reserve_label = source.reserve_label != 0 || source.has_label != 0;
            let effective_label_column_width = if reserve_label {
                label_column_width
            } else {
                0.0
            };
            let label_gap = if reserve_label && label_column_width > 0.0 {
                source.label_gap.max(0.0)
            } else {
                0.0
            };
            let text_left = source.bounds.left + effective_label_column_width + label_gap;
            let text_available = (source.bounds.right - text_left).max(1.0);
            let text_draw_width = source.text_width.max(0.0).min(text_available);
            let comment_left = text_left + text_draw_width + 4.0;
            let comment_available = (source.bounds.right - comment_left).max(1.0);
            let draw_comment = u8::from(
                source.comment_width <= 0.0 || source.comment_width <= comment_available + 2.0,
            );
            Fcitx5CandidateRenderItemOutput {
                label: Fcitx5CandidateLayoutRect {
                    left: source.bounds.left,
                    top: source.bounds.top,
                    right: source.bounds.left + effective_label_column_width,
                    bottom: source.bounds.bottom,
                },
                text: Fcitx5CandidateLayoutRect {
                    left: text_left,
                    top: source.bounds.top,
                    right: source.bounds.right,
                    bottom: source.bounds.bottom,
                },
                comment: Fcitx5CandidateLayoutRect {
                    left: comment_left,
                    top: source.bounds.top,
                    right: source.bounds.right,
                    bottom: source.bounds.bottom,
                },
                draw_comment,
            }
        })
        .collect::<Vec<_>>();
    (outputs, label_column_width)
}

#[no_mangle]
/// # Safety
///
/// `items` and `out_items` must be valid for `item_count` elements when
/// `item_count` is non-zero. Pointers are not retained.
pub unsafe extern "C" fn fcitx5_candidate_render_segments(
    items: *const Fcitx5CandidateRenderItemInput,
    item_count: usize,
    _horizontal_layout: u8,
    _scroll_mode: u8,
    out_items: *mut Fcitx5CandidateRenderItemOutput,
    out_label_column_width: *mut f32,
) -> i32 {
    if out_label_column_width.is_null()
        || (item_count > 0 && (items.is_null() || out_items.is_null()))
    {
        return 1;
    }
    let items = if item_count == 0 {
        &[]
    } else {
        // SAFETY: the validated items pointer covers `item_count` readable render-item inputs.
        unsafe { std::slice::from_raw_parts(items, item_count) }
    };
    let out_items = if item_count == 0 {
        &mut []
    } else {
        // SAFETY: the validated output pointer covers `item_count` writable segment outputs.
        unsafe { std::slice::from_raw_parts_mut(out_items, item_count) }
    };
    let (segments, label_column_width) = candidate_render_segments(items);
    // SAFETY: the validated label-width pointer is writable for one f32 result.
    unsafe {
        *out_label_column_width = label_column_width;
    }
    out_items.copy_from_slice(&segments);
    0
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
