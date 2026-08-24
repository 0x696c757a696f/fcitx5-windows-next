use super::{
    classify_input_method_switch, CaretRect, ContextKey, ContextLedger, ImSwitchAction,
    LedgerError, KEY_SYM_ALT_L, KEY_SYM_CONTROL_L, KEY_SYM_NONE, KEY_SYM_SHIFT_L, KEY_SYM_SPACE,
};

fn key_a() -> ContextKey {
    ContextKey::new(100, 1, 7)
}

fn key_b() -> ContextKey {
    ContextKey::new(200, 2, 9)
}

#[test]
fn new_ledger_reports_zero_state() {
    let ledger = ContextLedger::new();
    assert_eq!(ledger.revision_of(key_a()), 0);
    assert_eq!(ledger.composition_of(key_a()), 0);
}

#[test]
fn begin_key_accepts_unknown_context_at_zero() {
    let ledger = ContextLedger::new();
    assert!(ledger.begin_key(key_a(), 0, 0).is_ok());
}

#[test]
fn begin_key_rejects_stale_metadata() {
    let ledger = ContextLedger::new();
    assert_eq!(
        ledger.begin_key(key_a(), 1, 0),
        Err(LedgerError::StaleState)
    );
    assert_eq!(
        ledger.begin_key(key_a(), 0, 1),
        Err(LedgerError::StaleState)
    );
}

#[test]
fn end_result_allocates_composition_on_first_content() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    assert_eq!(composition, 1);
    assert_eq!(revision, 1);
    assert_eq!(ledger.composition_of(key_a()), 1);
    assert_eq!(ledger.revision_of(key_a()), 1);
}

#[test]
fn end_result_keeps_composition_while_content_active() {
    let mut ledger = ContextLedger::new();
    let (first, _) = ledger.end_result(key_a(), true);
    assert_eq!(first, 1);
    let (second, revision) = ledger.end_result(key_a(), true);
    assert_eq!(second, 1);
    assert_eq!(revision, 2);
}

#[test]
fn end_result_resets_composition_without_content() {
    let mut ledger = ContextLedger::new();
    let (_, _) = ledger.end_result(key_a(), true);
    let (composition, revision) = ledger.end_result(key_a(), false);
    assert_eq!(composition, 0);
    assert_eq!(revision, 2);
    // Next content allocates a fresh composition id.
    let (composition, revision) = ledger.end_result(key_a(), true);
    assert_eq!(composition, 2);
    assert_eq!(revision, 3);
}

#[test]
fn end_result_increments_revision_even_without_content() {
    let mut ledger = ContextLedger::new();
    let (_, revision) = ledger.end_result(key_a(), false);
    assert_eq!(revision, 1);
    let (_, revision) = ledger.end_result(key_a(), false);
    assert_eq!(revision, 2);
}

#[test]
fn begin_key_follows_end_result_state() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    assert!(ledger.begin_key(key_a(), revision, composition).is_ok());
    // Pre-result metadata is now stale.
    assert_eq!(
        ledger.begin_key(key_a(), 0, 0),
        Err(LedgerError::StaleState)
    );
}

#[test]
fn select_candidate_accepts_encoded_candidate() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    // candidate id = (composition << 8) | (index + 1)
    let candidate_id = (composition << 8) | 1;
    assert!(ledger
        .select_candidate(key_a(), revision, composition, candidate_id)
        .is_ok());
}

#[test]
fn select_candidate_rejects_zero_id() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    assert_eq!(
        ledger.select_candidate(key_a(), revision, composition, 0),
        Err(LedgerError::InvalidCandidate)
    );
}

#[test]
fn select_candidate_rejects_wrong_composition_bits() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    let wrong = (composition.wrapping_add(1) << 8) | 1;
    assert_eq!(
        ledger.select_candidate(key_a(), revision, composition, wrong),
        Err(LedgerError::InvalidCandidate)
    );
}

#[test]
fn select_candidate_rejects_stale_metadata() {
    let mut ledger = ContextLedger::new();
    let (composition, _) = ledger.end_result(key_a(), true);
    assert_eq!(
        ledger.select_candidate(key_a(), 0, composition, (composition << 8) | 1),
        Err(LedgerError::StaleState)
    );
}

#[test]
fn forget_resets_context_state() {
    let mut ledger = ContextLedger::new();
    let (_, _) = ledger.end_result(key_a(), true);
    ledger.forget(key_a());
    assert_eq!(ledger.revision_of(key_a()), 0);
    assert_eq!(ledger.composition_of(key_a()), 0);
    assert!(ledger.begin_key(key_a(), 0, 0).is_ok());
}

#[test]
fn contexts_are_isolated() {
    let mut ledger = ContextLedger::new();
    let (composition_a, revision_a) = ledger.end_result(key_a(), true);
    let (composition_b, revision_b) = ledger.end_result(key_b(), true);
    assert_eq!(composition_a, 1);
    assert_eq!(composition_b, 2);
    assert_eq!(revision_a, 1);
    assert_eq!(revision_b, 1);
    // key_a state does not affect key_b.
    assert!(ledger.begin_key(key_b(), revision_b, composition_b).is_ok());
}

#[test]
fn composition_allocation_wraps_and_skips_zero() {
    let mut ledger = ContextLedger::new();
    ledger.next_composition_id = u64::MAX;
    let (composition, _) = ledger.end_result(key_a(), true);
    assert_eq!(composition, u64::MAX);
    // Second allocation wraps: id 0 is reserved and skipped.
    let (composition, _) = ledger.end_result(key_a(), false);
    assert_eq!(composition, 0);
    let (composition, _) = ledger.end_result(key_a(), true);
    assert_eq!(composition, 1);
    assert_eq!(ledger.next_composition_id, 2);
}

// ---------------------------------------------------------------------------
// E2 extension: per-context product state maps
// ---------------------------------------------------------------------------

#[test]
fn caret_set_get_roundtrip() {
    let mut ledger = ContextLedger::new();
    assert_eq!(ledger.caret(key_a()), None);
    let caret = CaretRect {
        valid: 1,
        left: -100,
        top: 200,
        right: -98,
        bottom: 222,
        dpi: 144,
    };
    ledger.set_caret(key_a(), caret);
    assert_eq!(ledger.caret(key_a()), Some(caret));
    // Contexts are isolated.
    assert_eq!(ledger.caret(key_b()), None);
    ledger.forget(key_a());
    assert_eq!(ledger.caret(key_a()), None);
}

#[test]
fn popup_allowed_set_get_roundtrip() {
    let mut ledger = ContextLedger::new();
    assert_eq!(ledger.popup_allowed(key_a()), None);
    ledger.set_popup_allowed(key_a(), false);
    assert_eq!(ledger.popup_allowed(key_a()), Some(false));
    ledger.set_popup_allowed(key_a(), true);
    assert_eq!(ledger.popup_allowed(key_a()), Some(true));
    ledger.forget(key_a());
    assert_eq!(ledger.popup_allowed(key_a()), None);
}

#[test]
fn selected_override_set_query_clear() {
    let mut ledger = ContextLedger::new();
    assert_eq!(ledger.selected_override(key_a()), None);
    ledger.set_selected_override(key_a(), 3);
    assert_eq!(ledger.selected_override(key_a()), Some(3));
    // A stored Some(0) is still reported as present (C++ `found->second`
    // semantics: the optional is engaged even when the value is 0).
    ledger.set_selected_override(key_a(), 0);
    assert_eq!(ledger.selected_override(key_a()), Some(0));
    ledger.clear_selected_override(key_a());
    assert_eq!(ledger.selected_override(key_a()), None);
    // forget also clears the override.
    ledger.set_selected_override(key_a(), 7);
    ledger.forget(key_a());
    assert_eq!(ledger.selected_override(key_a()), None);
}

#[test]
fn input_method_overridden_default_false() {
    let mut ledger = ContextLedger::new();
    assert!(!ledger.input_method_overridden(key_a()));
    ledger.set_input_method_overridden(key_a(), true);
    assert!(ledger.input_method_overridden(key_a()));
    ledger.set_input_method_overridden(key_a(), false);
    assert!(!ledger.input_method_overridden(key_a()));
    ledger.forget(key_a());
    assert!(!ledger.input_method_overridden(key_a()));
}

#[test]
fn product_state_maps_survive_ledger_lifecycle() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    let caret = CaretRect {
        valid: 0,
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
        dpi: 96,
    };
    ledger.set_caret(key_a(), caret);
    ledger.set_popup_allowed(key_a(), true);
    ledger.set_selected_override(key_a(), 2);
    ledger.set_input_method_overridden(key_a(), true);
    // Ledger state and product state coexist per context.
    assert_eq!(ledger.revision_of(key_a()), revision);
    assert_eq!(ledger.composition_of(key_a()), composition);
    assert_eq!(ledger.caret(key_a()), Some(caret));
    assert_eq!(ledger.popup_allowed(key_a()), Some(true));
    assert_eq!(ledger.selected_override(key_a()), Some(2));
    assert!(ledger.input_method_overridden(key_a()));
}

// ---------------------------------------------------------------------------
// E3: input-method switch hotkey decision corpus
// ---------------------------------------------------------------------------

const TOGGLE: &str = "Ctrl+Space";
const NEXT: &str = "Ctrl+Shift";

fn classify(ctrl: bool, shift: bool, alt: bool, sym: u32) -> Option<ImSwitchAction> {
    classify_input_method_switch(ctrl, shift, alt, sym, Some(TOGGLE), Some(NEXT))
}

#[test]
fn ctrl_space_toggles() {
    assert_eq!(
        classify(true, false, false, KEY_SYM_SPACE),
        Some(ImSwitchAction::Toggle)
    );
}

#[test]
fn ctrl_shift_nexts() {
    assert_eq!(
        classify(true, true, false, KEY_SYM_SHIFT_L),
        Some(ImSwitchAction::Next)
    );
}

#[test]
fn ctrl_shift_space_toggles() {
    // "Ctrl+Shift+Space" is the default toggle string; the decision matches
    // the toggle hotkey before the next hotkey.
    assert_eq!(
        classify_input_method_switch(
            true,
            true,
            false,
            KEY_SYM_SPACE,
            Some("Ctrl+Shift+Space"),
            Some(NEXT)
        ),
        Some(ImSwitchAction::Toggle)
    );
}

#[test]
fn alt_shift_toggles() {
    assert_eq!(
        classify_input_method_switch(
            false,
            true,
            true,
            KEY_SYM_SHIFT_L,
            Some("Alt+Shift"),
            Some(NEXT)
        ),
        Some(ImSwitchAction::Toggle)
    );
}

#[test]
fn modifier_combinations_do_not_match() {
    // Shift+Space without Ctrl: not a hotkey.
    assert_eq!(classify(false, true, false, KEY_SYM_SPACE), None);
    // Ctrl+Alt+Space: Alt disqualifies Ctrl+Space.
    assert_eq!(classify(true, false, true, KEY_SYM_SPACE), None);
    // Shift_L with only Ctrl (no Shift): not Ctrl+Shift.
    assert_eq!(classify(true, false, false, KEY_SYM_SHIFT_L), None);
    // Shift_L with only Shift (no Ctrl/Alt): nothing.
    assert_eq!(classify(false, true, false, KEY_SYM_SHIFT_L), None);
}

#[test]
fn ordinary_keys_never_match_hotkeys() {
    assert_eq!(classify(true, false, false, 0x41), None);
    assert_eq!(classify(true, false, false, KEY_SYM_CONTROL_L), None);
    assert_eq!(classify(false, false, true, KEY_SYM_ALT_L), None);
}

#[test]
fn none_keysym_never_matches() {
    assert_eq!(classify(true, false, false, KEY_SYM_NONE), None);
    assert_eq!(
        classify_input_method_switch(true, false, false, KEY_SYM_NONE, Some(TOGGLE), Some(NEXT)),
        None
    );
}

#[test]
fn unconfigured_hotkeys_do_not_match() {
    assert_eq!(
        classify_input_method_switch(true, false, false, KEY_SYM_SPACE, None, Some(NEXT)),
        None
    );
    assert_eq!(
        classify_input_method_switch(true, false, false, KEY_SYM_SPACE, Some(TOGGLE), None),
        Some(ImSwitchAction::Toggle)
    );
    assert_eq!(
        classify_input_method_switch(true, false, false, KEY_SYM_SPACE, None, None),
        None
    );
}

#[test]
fn unknown_hotkey_string_never_matches() {
    assert_eq!(
        classify_input_method_switch(
            true,
            false,
            false,
            KEY_SYM_SPACE,
            Some("F11"),
            Some("Ctrl+Shift")
        ),
        None
    );
}

#[test]
fn toggle_wins_when_both_hotkeys_identical() {
    assert_eq!(
        classify_input_method_switch(
            true,
            false,
            false,
            KEY_SYM_SPACE,
            Some(TOGGLE),
            Some(TOGGLE)
        ),
        Some(ImSwitchAction::Toggle)
    );
}

// ---------------------------------------------------------------------------
// E3-2: candidate navigation decision corpus
// ---------------------------------------------------------------------------

mod navigation_tests {
    use super::super::navigation::{
        column_navigation_target, column_selection_target, decide_candidate_action,
        row_navigation_target, row_selection_target, same_column_navigation_target,
        CandidateAction, CandidateDecision, KEY_SYM_0, KEY_SYM_2, KEY_SYM_9, KEY_SYM_APOSTROPHE,
        KEY_SYM_COMMA, KEY_SYM_DOWN, KEY_SYM_EQUAL, KEY_SYM_LEFT, KEY_SYM_MINUS, KEY_SYM_RETURN,
        KEY_SYM_RIGHT, KEY_SYM_SEMICOLON, KEY_SYM_UP,
    };
    use crate::KEY_SYM_SPACE;

    // Default non-scroll, non-bulk view: 10 candidates, cursor 0, pageable
    // with neither prev nor next.
    fn view10() -> (i32, i32, i32, i32, bool, bool, bool, bool, bool) {
        (10, 10, 0, -1, false, false, true, false, false)
    }

    #[allow(clippy::too_many_arguments)]
    fn decide(
        sym: u32,
        plain: bool,
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
        page_size: Option<i32>,
        override_value: Option<u32>,
    ) -> CandidateDecision {
        decide_candidate_action(
            sym,
            plain,
            count,
            list_size,
            cursor,
            bulk_cursor,
            has_bulk_cursor,
            has_bulk,
            pageable,
            has_prev,
            has_next,
            scroll_mode,
            vertical,
            page_size,
            override_value,
        )
    }

    fn view10_decide(sym: u32, plain: bool, override_value: Option<u32>) -> CandidateDecision {
        let (count, list_size, cursor, bulk_cursor, hbc, hb, pageable, hp, hn) = view10();
        decide(
            sym,
            plain,
            count,
            list_size,
            cursor,
            bulk_cursor,
            hbc,
            hb,
            pageable,
            hp,
            hn,
            false,
            false,
            None,
            override_value,
        )
    }

    // -- target helpers ----------------------------------------------------

    #[test]
    fn row_selection_target_matches_cpp() {
        // focus 4 is row 0 (4/5), column 1 -> target 0*5+1 = 1.
        assert_eq!(row_selection_target(4, 1, 5, 20), Some(1));
        // focus 4, column 4 -> 4.
        assert_eq!(row_selection_target(4, 4, 5, 20), Some(4));
        // focus 6 is row 1 (6/5), column 3 -> 5+3 = 8.
        assert_eq!(row_selection_target(6, 3, 5, 20), Some(8));
        // focus 19 is row 3, column 4 -> 15+4 = 19 (valid).
        assert_eq!(row_selection_target(19, 4, 5, 20), Some(19));
        // focus 19, column 4 but count 19 -> 19 >= 19 invalid.
        assert_eq!(row_selection_target(19, 4, 5, 19), None);
    }

    #[test]
    fn column_selection_target_matches_cpp() {
        // focus 4, rows 3: 4%3=1, columnStart=3, row 1 -> 3+1 = 4.
        assert_eq!(column_selection_target(4, 1, 3, 20), Some(4));
        // focus 4, row 2 -> 3+2 = 5.
        assert_eq!(column_selection_target(4, 2, 3, 20), Some(5));
        // focus 4, row 2, count 5 -> 5 >= 5 invalid.
        assert_eq!(column_selection_target(4, 2, 3, 5), None);
        // focus 4, row 0 -> 3.
        assert_eq!(column_selection_target(4, 0, 3, 20), Some(3));
    }

    #[test]
    fn navigation_target_helpers_match_cpp() {
        // rowNavigationTarget(4, 5, 20, true, false): base 0 -> 5
        assert_eq!(row_navigation_target(4, 5, 20, true, false).index, 5);
        // rowNavigationTarget(4, 5, 20, false, false): base 0 < 5 -> beforeStart
        let back = row_navigation_target(4, 5, 20, false, false);
        assert!(back.before_start && back.index == 4);
        // columnNavigationTarget(4, 3, 20, true, false): next 6+0 -> 6
        assert_eq!(column_navigation_target(4, 3, 20, true, false).index, 6);
        // sameColumnNavigationTarget(4, 3, 20, true): 5
        assert_eq!(same_column_navigation_target(4, 3, 20, true).index, 5);
    }

    // -- branch 1: scroll digit selection ----------------------------------

    #[test]
    fn scroll_digit_selects_target() {
        // scroll vertical, focus 4, digit '2' -> column 1 ->
        // columnSelectionTarget(4, 1, 5, 20): 4%5=4, columnStart=0, target=0+1=1.
        let d = decide(
            KEY_SYM_2,
            true,
            20,
            10,
            0,
            0,
            true,
            true,
            true,
            false,
            false,
            true,
            true,
            Some(5),
            Some(4),
        );
        assert_eq!(
            d,
            CandidateDecision {
                consume: true,
                action: CandidateAction::SelectAndClear(1),
            }
        );
    }

    #[test]
    fn scroll_digit_invalid_target_consumes_without_action() {
        // Only 3 candidates, focus 2, digit '9' -> column 8 >= dimension 3 -> None.
        let d = decide(
            KEY_SYM_9,
            true,
            3,
            10,
            0,
            0,
            true,
            true,
            true,
            false,
            false,
            true,
            true,
            Some(3),
            Some(2),
        );
        assert_eq!(d.action, CandidateAction::ConsumeOnly);
        assert!(d.consume);
    }

    #[test]
    fn scroll_zero_consumes_without_selecting() {
        // '0' sets column None -> ConsumeOnly.
        let d = decide(
            KEY_SYM_0,
            true,
            20,
            10,
            0,
            0,
            true,
            true,
            true,
            false,
            false,
            true,
            true,
            Some(5),
            Some(4),
        );
        assert_eq!(d.action, CandidateAction::ConsumeOnly);
        assert!(d.consume);
    }

    #[test]
    fn scroll_semicolon_selects_second_column() {
        // vertical, ';' -> column 1 -> columnSelectionTarget(4, 1, 5, 20) = 1.
        let d = decide(
            KEY_SYM_SEMICOLON,
            true,
            20,
            10,
            0,
            0,
            true,
            true,
            true,
            false,
            false,
            true,
            true,
            Some(5),
            Some(4),
        );
        assert_eq!(d.action, CandidateAction::SelectAndClear(1));
    }

    // -- branch 2: scroll navigation ---------------------------------------

    #[test]
    fn scroll_down_sets_override() {
        // scroll vertical, focus 1, Down -> sameColumnNavigationTarget(1, 5, 20, true) -> 2.
        let d = decide(
            KEY_SYM_DOWN,
            true,
            20,
            10,
            0,
            0,
            true,
            true,
            true,
            false,
            false,
            true,
            true,
            Some(5),
            Some(1),
        );
        assert_eq!(d.action, CandidateAction::SetOverride(2));
        assert!(d.consume);
    }

    #[test]
    fn scroll_up_at_start_reports_before_start_no_page() {
        // focus 0, Up -> beforeStart; vertical Up/Down never turns pages ->
        // SetOverride(0).
        let d = decide(
            KEY_SYM_UP,
            true,
            20,
            10,
            0,
            0,
            true,
            true,
            true,
            false,
            false,
            true,
            true,
            Some(5),
            Some(0),
        );
        assert_eq!(d.action, CandidateAction::SetOverride(0));
    }

    #[test]
    fn scroll_page_key_turns_prev_page() {
        // scroll vertical, '-' (prevPage), focus 0 -> columnNavigationTarget
        // beforeStart; page keys (not Up/Down) may turn pages.
        let d = decide(
            KEY_SYM_MINUS,
            true,
            20,
            10,
            0,
            0,
            true,
            true,
            true,
            true,
            false,
            true,
            true,
            Some(5),
            Some(0),
        );
        assert_eq!(d.action, CandidateAction::PagePrevAndSetOverride(0));
    }

    #[test]
    fn scroll_page_key_turns_next_page() {
        // scroll vertical, '=' (nextPage), focus 19 -> afterEnd; has_next ->
        // PageNextAndSetOverride(0).
        let d = decide(
            KEY_SYM_EQUAL,
            true,
            20,
            10,
            0,
            0,
            true,
            true,
            true,
            false,
            true,
            true,
            true,
            Some(5),
            Some(19),
        );
        assert_eq!(d.action, CandidateAction::PageNextAndSetOverride(0));
    }

    // -- branch 3: page keys -----------------------------------------------

    #[test]
    fn page_next_with_page_turns_and_resets_override() {
        // '=' with has_next -> PageNextAndSetOverride(0).
        let d = decide(
            KEY_SYM_EQUAL,
            true,
            10,
            10,
            0,
            -1,
            false,
            false,
            true,
            false,
            true,
            false,
            false,
            None,
            None,
        );
        assert_eq!(d.action, CandidateAction::PageNextAndSetOverride(0));
    }

    #[test]
    fn page_prev_without_page_consumes_only() {
        let d = decide(
            KEY_SYM_COMMA,
            true,
            10,
            10,
            0,
            -1,
            false,
            false,
            true,
            false,
            false,
            false,
            false,
            None,
            None,
        );
        assert_eq!(d.action, CandidateAction::ConsumeOnly);
        assert!(d.consume);
    }

    // -- branch 4: ordinary ';' / '\'' -------------------------------------

    #[test]
    fn ordinary_semicolon_selects_second_candidate() {
        let d = view10_decide(KEY_SYM_SEMICOLON, true, None);
        assert_eq!(d.action, CandidateAction::SelectAndClear(1));
        assert!(d.consume);
    }

    #[test]
    fn ordinary_apostrophe_selects_third_candidate() {
        let d = view10_decide(KEY_SYM_APOSTROPHE, true, None);
        assert_eq!(d.action, CandidateAction::SelectAndClear(2));
    }

    #[test]
    fn ordinary_semicolon_out_of_range_not_consumed() {
        // Only 1 candidate: target 1 >= 1 -> not consumed.
        let d = decide(
            KEY_SYM_SEMICOLON,
            true,
            1,
            1,
            0,
            -1,
            false,
            false,
            true,
            false,
            false,
            false,
            false,
            None,
            None,
        );
        assert!(!d.consume);
        assert_eq!(d.action, CandidateAction::None);
    }

    // -- branch 5: Left/Right ----------------------------------------------

    #[test]
    fn right_moves_highlight() {
        let d = view10_decide(KEY_SYM_RIGHT, true, Some(3));
        assert_eq!(d.action, CandidateAction::SetOverride(4));
    }

    #[test]
    fn left_at_start_clamps_to_zero() {
        let d = view10_decide(KEY_SYM_LEFT, true, Some(0));
        assert_eq!(d.action, CandidateAction::SetOverride(0));
    }

    #[test]
    fn right_without_override_uses_list_cursor() {
        let d = view10_decide(KEY_SYM_RIGHT, true, None);
        assert_eq!(d.action, CandidateAction::SetOverride(1));
    }

    // -- branch 6: Space/Return --------------------------------------------

    #[test]
    fn space_commits_highlighted_candidate() {
        let d = view10_decide(KEY_SYM_SPACE, true, Some(3));
        assert_eq!(d.action, CandidateAction::SelectAndClear(3));
    }

    #[test]
    fn space_without_override_not_consumed() {
        let d = view10_decide(KEY_SYM_SPACE, true, None);
        assert!(!d.consume);
        assert_eq!(d.action, CandidateAction::None);
    }

    #[test]
    fn return_commits_highlighted_candidate() {
        let d = view10_decide(KEY_SYM_RETURN, true, Some(2));
        assert_eq!(d.action, CandidateAction::SelectAndClear(2));
    }

    #[test]
    fn space_override_out_of_range_not_consumed() {
        // bounded = 10, override 15 out of range.
        let d = view10_decide(KEY_SYM_SPACE, true, Some(15));
        assert!(!d.consume);
    }

    // -- no match ----------------------------------------------------------

    #[test]
    fn ordinary_key_not_consumed() {
        let d = view10_decide(0x61, true, None);
        assert!(!d.consume);
        assert_eq!(d.action, CandidateAction::None);
    }

    #[test]
    fn shifted_digit_not_a_scroll_shortcut() {
        // plain=false: scroll digit branch skipped.
        let d = decide(
            KEY_SYM_2,
            false,
            20,
            10,
            0,
            0,
            true,
            true,
            true,
            false,
            false,
            true,
            true,
            Some(5),
            Some(4),
        );
        assert!(!d.consume);
        assert_eq!(d.action, CandidateAction::None);
    }
}
