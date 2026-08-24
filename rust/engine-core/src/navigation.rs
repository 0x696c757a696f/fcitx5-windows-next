//! Candidate navigation decision (E3-2).
//!
//! This module is the Rust-authoritative Event→Action decision for the
//! candidate-navigation block of `FcitxRuntime::processKey`. It mirrors the
//! frozen C++ semantics exactly — branch order, boundary handling, and the
//! `candidate_navigation.h` target helpers — so the C++ adapter can flatten
//! the Fcitx candidate state, ask Rust what to do, and only execute the
//! returned action.
//!
//! The decision is pure: every input is a scalar fact (key, candidate view,
//! config, current override) and every output is a `CandidateDecision` that
//! the C++ adapter executes against Fcitx objects (`candidate.select`,
//! `pageable->next/prev`, ledger override writes).

use crate::KEY_SYM_SPACE;

/// Keysym constants mirroring `fcitx-utils/keysymgen.h` (navigation keys).
pub const KEY_SYM_APOSTROPHE: u32 = 0x0027; // '
pub const KEY_SYM_PLUS: u32 = 0x002b; // +
pub const KEY_SYM_COMMA: u32 = 0x002c; // ,
pub const KEY_SYM_MINUS: u32 = 0x002d; // -
pub const KEY_SYM_PERIOD: u32 = 0x002e; // .
pub const KEY_SYM_0: u32 = 0x0030; // 0
pub const KEY_SYM_1: u32 = 0x0031; // 1
pub const KEY_SYM_2: u32 = 0x0032; // 2
pub const KEY_SYM_3: u32 = 0x0033; // 3
pub const KEY_SYM_4: u32 = 0x0034; // 4
pub const KEY_SYM_5: u32 = 0x0035; // 5
pub const KEY_SYM_6: u32 = 0x0036; // 6
pub const KEY_SYM_7: u32 = 0x0037; // 7
pub const KEY_SYM_8: u32 = 0x0038; // 8
pub const KEY_SYM_9: u32 = 0x0039; // 9
pub const KEY_SYM_SEMICOLON: u32 = 0x003b; // ;
pub const KEY_SYM_EQUAL: u32 = 0x003d; // =
pub const KEY_SYM_BRACKETLEFT: u32 = 0x005b; // [
pub const KEY_SYM_BRACKETRIGHT: u32 = 0x005d; // ]
pub const KEY_SYM_UNDERSCORE: u32 = 0x005f; // _
pub const KEY_SYM_RETURN: u32 = 0xff0d; // Return
pub const KEY_SYM_LEFT: u32 = 0xff51; // Left arrow
pub const KEY_SYM_UP: u32 = 0xff52; // Up arrow
pub const KEY_SYM_RIGHT: u32 = 0xff53; // Right arrow
pub const KEY_SYM_DOWN: u32 = 0xff54; // Down arrow

/// `protocol::kMaxCandidates` (candidate count/field clamp).
pub const MAX_CANDIDATES: i32 = 128;

/// Navigation target helper mirroring `candidate_navigation.h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowNavigationTarget {
    pub index: usize,
    pub before_start: bool,
    pub after_end: bool,
}

fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Mirrors `rowSelectionTarget`.
pub fn row_selection_target(
    focus: usize,
    column: usize,
    columns: usize,
    candidate_count: usize,
) -> Option<usize> {
    if columns == 0 || column >= columns || focus >= candidate_count {
        return None;
    }
    let target = focus - focus % columns + column;
    if target >= candidate_count {
        return None;
    }
    Some(target)
}

/// Mirrors `rowSelectionColumn`.
pub fn row_selection_column(
    focus: usize,
    candidate: usize,
    columns: usize,
    candidate_count: usize,
) -> Option<usize> {
    if columns == 0
        || focus >= candidate_count
        || candidate >= candidate_count
        || focus / columns != candidate / columns
    {
        return None;
    }
    Some(candidate % columns)
}

/// Mirrors `columnSelectionTarget`.
pub fn column_selection_target(
    focus: usize,
    row: usize,
    rows: usize,
    candidate_count: usize,
) -> Option<usize> {
    if rows == 0 || row >= rows || focus >= candidate_count {
        return None;
    }
    let target = focus - focus % rows + row;
    if target >= candidate_count {
        return None;
    }
    Some(target)
}

/// Mirrors `columnSelectionRow`.
pub fn column_selection_row(
    focus: usize,
    candidate: usize,
    rows: usize,
    candidate_count: usize,
) -> Option<usize> {
    if rows == 0
        || focus >= candidate_count
        || candidate >= candidate_count
        || focus / rows != candidate / rows
    {
        return None;
    }
    Some(candidate % rows)
}

/// Mirrors `rowNavigationTarget`.
pub fn row_navigation_target(
    focus: usize,
    columns: usize,
    candidate_count: usize,
    forward: bool,
    preserve_column: bool,
) -> RowNavigationTarget {
    if columns == 0 || focus >= candidate_count {
        return RowNavigationTarget {
            index: focus,
            before_start: false,
            after_end: false,
        };
    }
    let base = if preserve_column {
        focus
    } else {
        focus - focus % columns
    };
    if forward {
        if base > candidate_count || columns >= candidate_count - base {
            return RowNavigationTarget {
                index: focus,
                before_start: false,
                after_end: true,
            };
        }
        return RowNavigationTarget {
            index: base + columns,
            before_start: false,
            after_end: false,
        };
    }
    if base < columns {
        return RowNavigationTarget {
            index: focus,
            before_start: true,
            after_end: false,
        };
    }
    RowNavigationTarget {
        index: base - columns,
        before_start: false,
        after_end: false,
    }
}

/// Mirrors `columnNavigationTarget`.
pub fn column_navigation_target(
    focus: usize,
    rows: usize,
    candidate_count: usize,
    forward: bool,
    preserve_row: bool,
) -> RowNavigationTarget {
    if rows == 0 || focus >= candidate_count {
        return RowNavigationTarget {
            index: focus,
            before_start: false,
            after_end: false,
        };
    }
    let row = if preserve_row { focus % rows } else { 0 };
    let column_start = focus - focus % rows;
    if forward {
        let next_column_start = column_start + rows;
        if next_column_start >= candidate_count {
            return RowNavigationTarget {
                index: focus,
                before_start: false,
                after_end: true,
            };
        }
        return RowNavigationTarget {
            index: (next_column_start + row).min(candidate_count - 1),
            before_start: false,
            after_end: false,
        };
    }
    if column_start < rows {
        return RowNavigationTarget {
            index: focus,
            before_start: true,
            after_end: false,
        };
    }
    let previous_column_start = column_start - rows;
    RowNavigationTarget {
        index: (previous_column_start + row).min(candidate_count - 1),
        before_start: false,
        after_end: false,
    }
}

/// Mirrors `sameColumnNavigationTarget`.
pub fn same_column_navigation_target(
    focus: usize,
    rows: usize,
    candidate_count: usize,
    forward: bool,
) -> RowNavigationTarget {
    if rows == 0 || focus >= candidate_count {
        return RowNavigationTarget {
            index: focus,
            before_start: false,
            after_end: false,
        };
    }
    let column_start = focus - focus % rows;
    let column_end = (column_start + rows).min(candidate_count);
    if forward {
        if focus + 1 >= column_end {
            return RowNavigationTarget {
                index: focus,
                before_start: false,
                after_end: true,
            };
        }
        return RowNavigationTarget {
            index: focus + 1,
            before_start: false,
            after_end: false,
        };
    }
    if focus == column_start {
        return RowNavigationTarget {
            index: focus,
            before_start: true,
            after_end: false,
        };
    }
    RowNavigationTarget {
        index: focus - 1,
        before_start: false,
        after_end: false,
    }
}

/// Action decided for a candidate-navigation key event. The C++ adapter
/// executes the side effects against Fcitx objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateAction {
    /// No candidate action; the key is not consumed (ordinary key event).
    None,
    /// The key is consumed but no candidate action applies (e.g. scroll
    /// digit with an invalid target, or a page key with no page to turn).
    ConsumeOnly,
    /// Select the candidate at `value` and clear the highlight override.
    SelectAndClear(u32),
    /// Move the highlight override to `value` without committing.
    SetOverride(u32),
    /// Turn the pageable list to the next page and set override `value`.
    PageNextAndSetOverride(u32),
    /// Turn the pageable list to the previous page and set override `value`.
    PagePrevAndSetOverride(u32),
}

/// Result of the candidate-navigation decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateDecision {
    /// Whether the key event is consumed (`filterAndAccept` + result).
    pub consume: bool,
    pub action: CandidateAction,
}

/// Decides the action for a non-release candidate-navigation key.
///
/// Mirrors `FcitxRuntime::processKey` branch-for-branch:
///
/// 1. scroll + plain shortcut digits/`;`/`'` → select target, clear override;
/// 2. scroll + next/prev (Up/Down or page keys) → navigate, maybe page, set
///    override;
/// 3. pageable + next/prev page keys → turn page, set override 0;
/// 4. ordinary paging `;`/`'` → select second/third candidate;
/// 5. plain shortcut Left/Right → move highlight override;
/// 6. Space/Return with a highlight override → commit the highlighted
///    candidate;
/// 7. anything else → not consumed.
#[allow(clippy::too_many_arguments)]
pub fn decide_candidate_action(
    key_sym: u32,
    plain_shortcut: bool,
    count: i32,
    list_size: i32,
    cursor: i32,
    bulk_cursor: i32,
    has_bulk_cursor: bool,
    has_bulk: bool,
    pageable: bool,
    has_prev: bool,
    has_next: bool,
    scroll_mode: bool,
    vertical: bool,
    candidate_page_size: Option<i32>,
    current_override: Option<u32>,
) -> CandidateDecision {
    let bounded = clamp_i32(count, 0, MAX_CANDIDATES);
    let scroll = scroll_mode && has_bulk;
    let dimension = clamp_i32(candidate_page_size.unwrap_or(list_size), 1, MAX_CANDIDATES);
    let dimension_u = dimension as usize;
    let bounded_u = bounded.max(0) as usize;

    let next_page = key_sym == KEY_SYM_EQUAL
        || key_sym == KEY_SYM_PLUS
        || key_sym == KEY_SYM_PERIOD
        || key_sym == KEY_SYM_BRACKETRIGHT;
    let prev_page = key_sym == KEY_SYM_MINUS
        || key_sym == KEY_SYM_UNDERSCORE
        || key_sym == KEY_SYM_COMMA
        || key_sym == KEY_SYM_BRACKETLEFT;
    let up_down = key_sym == KEY_SYM_UP || key_sym == KEY_SYM_DOWN;

    let raw_focus = match current_override {
        Some(value) => value as i32,
        None if has_bulk_cursor => {
            if bulk_cursor >= 0 {
                bulk_cursor
            } else {
                cursor
            }
        }
        None => cursor,
    };
    let focus = clamp_i32(raw_focus, 0, (bounded - 1).max(0)) as usize;

    // 1. scroll viewport number-row / ';' / '\'' selection.
    if scroll && plain_shortcut {
        let mut column: Option<usize> = None;
        let mut consume = false;
        if key_sym == KEY_SYM_0 {
            consume = true;
        } else if (KEY_SYM_1..=KEY_SYM_9).contains(&key_sym) {
            consume = true;
            let digit = (key_sym - KEY_SYM_1) as usize;
            if digit < dimension_u {
                column = Some(digit);
            }
        } else if key_sym == KEY_SYM_SEMICOLON {
            consume = true;
            column = Some(1);
        } else if key_sym == KEY_SYM_APOSTROPHE {
            consume = true;
            column = Some(2);
        }
        if consume {
            let target = match column {
                Some(column) => {
                    if vertical {
                        column_selection_target(focus, column, dimension_u, bounded_u)
                    } else {
                        row_selection_target(focus, column, dimension_u, bounded_u)
                    }
                }
                None => None,
            };
            let action = match target {
                Some(target) => CandidateAction::SelectAndClear(target as u32),
                None => CandidateAction::ConsumeOnly,
            };
            return CandidateDecision {
                consume: true,
                action,
            };
        }
    }

    let scroll_next = next_page || (scroll && key_sym == KEY_SYM_DOWN);
    let scroll_prev = prev_page || (scroll && key_sym == KEY_SYM_UP);

    // 2. scroll viewport navigation (Up/Down scroll one row; page keys jump
    //    column tops; pageable turns are wrapped with override resets).
    if scroll && (scroll_next || scroll_prev) {
        let available = bounded.max(0) as usize;
        let navigation = if vertical && up_down {
            same_column_navigation_target(focus, dimension_u, available, scroll_next)
        } else if vertical {
            column_navigation_target(focus, dimension_u, available, scroll_next, false)
        } else {
            row_navigation_target(focus, dimension_u, available, scroll_next, up_down)
        };
        let mut target = navigation.index as i32;
        let mut action = CandidateAction::SetOverride(clamp_i32(
            target,
            0,
            (available as i32 - 1).max(0),
        ) as u32);
        if navigation.before_start && !(vertical && up_down) && pageable && has_prev {
            target = if !vertical && up_down {
                (focus % dimension_u) as i32
            } else {
                0
            };
            action = CandidateAction::PagePrevAndSetOverride(clamp_i32(
                target,
                0,
                (available as i32 - 1).max(0),
            ) as u32);
        } else if navigation.after_end && !(vertical && up_down) && pageable && has_next {
            target = if !vertical && up_down {
                (focus % dimension_u) as i32
            } else {
                0
            };
            action = CandidateAction::PageNextAndSetOverride(clamp_i32(
                target,
                0,
                (available as i32 - 1).max(0),
            ) as u32);
        }
        return CandidateDecision {
            consume: true,
            action,
        };
    }

    // 3. pageable prev/next page keys (highlight jumps to the first
    //    candidate of the new page).
    if pageable && (next_page || prev_page) {
        let action = if next_page && has_next {
            CandidateAction::PageNextAndSetOverride(0)
        } else if prev_page && has_prev {
            CandidateAction::PagePrevAndSetOverride(0)
        } else {
            CandidateAction::ConsumeOnly
        };
        return CandidateDecision {
            consume: true,
            action,
        };
    }

    // 4. ordinary paging ';' selects the second candidate, '\'' the third.
    if !scroll && (key_sym == KEY_SYM_SEMICOLON || key_sym == KEY_SYM_APOSTROPHE) {
        let target = if key_sym == KEY_SYM_SEMICOLON { 1 } else { 2 };
        if target < bounded_u {
            return CandidateDecision {
                consume: true,
                action: CandidateAction::SelectAndClear(target as u32),
            };
        }
    }

    // 5. Left/Right move the highlight without committing.
    if plain_shortcut && (key_sym == KEY_SYM_LEFT || key_sym == KEY_SYM_RIGHT) && bounded > 0 {
        let next_focus = if scroll && vertical {
            column_navigation_target(
                focus,
                dimension_u,
                bounded_u,
                key_sym == KEY_SYM_RIGHT,
                true,
            )
            .index
        } else {
            let delta = if key_sym == KEY_SYM_RIGHT { 1 } else { -1 };
            clamp_i32(focus as i32 + delta, 0, bounded - 1) as usize
        };
        return CandidateDecision {
            consume: true,
            action: CandidateAction::SetOverride(next_focus as u32),
        };
    }

    // 6. Space/Return commit the highlighted candidate.
    if key_sym == KEY_SYM_SPACE || key_sym == KEY_SYM_RETURN {
        if let Some(override_value) = current_override {
            let override_focus = override_value as i32;
            if override_focus >= 0 && override_focus < bounded {
                return CandidateDecision {
                    consume: true,
                    action: CandidateAction::SelectAndClear(override_focus as u32),
                };
            }
        }
    }

    CandidateDecision {
        consume: false,
        action: CandidateAction::None,
    }
}

/// Decides the scroll-mode candidate label offset (mirrors the C++
/// `columnSelectionRow`/`rowSelectionColumn` choice in `collectResult`).
pub fn scroll_label_offset(
    vertical: bool,
    cursor: usize,
    index: usize,
    dimension: usize,
    size: usize,
) -> Option<usize> {
    if vertical {
        column_selection_row(cursor, index, dimension, size)
    } else {
        row_selection_column(cursor, index, dimension, size)
    }
}
