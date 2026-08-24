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

// ---------------------------------------------------------------------------
// E3-3: surrounding-text and input-method-selection decision corpus
// ---------------------------------------------------------------------------

use super::{
    decide_input_method_selection, decide_surrounding_text, InputMethodSelection,
    SurroundingTextAction, SurroundingTextDecision,
};

#[test]
fn surrounding_text_valid_sets() {
    assert_eq!(
        decide_surrounding_text(true, false),
        SurroundingTextDecision {
            action: SurroundingTextAction::Set,
            update: true,
        }
    );
    assert_eq!(
        decide_surrounding_text(true, true),
        SurroundingTextDecision {
            action: SurroundingTextAction::Set,
            update: true,
        }
    );
}

#[test]
fn surrounding_text_invalid_with_valid_state_invalidates_and_updates() {
    assert_eq!(
        decide_surrounding_text(false, true),
        SurroundingTextDecision {
            action: SurroundingTextAction::Invalidate,
            update: true,
        }
    );
}

#[test]
fn surrounding_text_invalid_with_invalid_state_invalidates_without_update() {
    assert_eq!(
        decide_surrounding_text(false, false),
        SurroundingTextDecision {
            action: SurroundingTextAction::Invalidate,
            update: false,
        }
    );
}

fn select(
    has_request: bool,
    request_valid: bool,
    default_valid: bool,
    default_nonempty: bool,
    current_eq_request: bool,
    current_eq_default: bool,
    overridden: bool,
) -> InputMethodSelection {
    decide_input_method_selection(
        has_request,
        request_valid,
        default_valid,
        default_nonempty,
        current_eq_request,
        current_eq_default,
        overridden,
    )
}

#[test]
fn im_selection_valid_request_wins() {
    // request valid + current differs -> SelectRequest.
    assert_eq!(
        select(true, true, true, true, false, false, false),
        InputMethodSelection::SelectRequest
    );
}

#[test]
fn im_selection_falls_back_to_default() {
    // request empty -> default valid + differs -> SelectDefault.
    assert_eq!(
        select(false, false, true, true, false, false, false),
        InputMethodSelection::SelectDefault
    );
}

#[test]
fn im_selection_no_change_when_current_matches() {
    // request valid but current already equals it.
    assert_eq!(
        select(true, true, true, true, true, false, false),
        InputMethodSelection::NoChange
    );
    // default valid and current already equals it.
    assert_eq!(
        select(false, false, true, true, false, true, false),
        InputMethodSelection::NoChange
    );
}

#[test]
fn im_selection_respects_override_marker() {
    // overridden -> no switch even when everything else matches.
    assert_eq!(
        select(true, true, true, true, false, false, true),
        InputMethodSelection::NoChange
    );
}

#[test]
fn im_selection_rejects_invalid_selected() {
    // request provided but invalid -> falls to default; default invalid too.
    assert_eq!(
        select(true, false, false, true, false, false, false),
        InputMethodSelection::NoChange
    );
    // request valid but default invalid when request empty.
    assert_eq!(
        select(false, false, false, true, false, false, false),
        InputMethodSelection::NoChange
    );
}

#[test]
fn im_selection_rejects_empty_default() {
    assert_eq!(
        select(false, false, true, false, false, false, false),
        InputMethodSelection::NoChange
    );
}

#[test]
fn im_selection_ignores_invalid_request_but_uses_default() {
    // has_request but entry(request) invalid -> use default (valid + differs).
    assert_eq!(
        select(true, false, true, true, false, false, false),
        InputMethodSelection::SelectDefault
    );
}

// ---------------------------------------------------------------------------
// E3 event-shape consolidation: handle_key_event corpus
// ---------------------------------------------------------------------------

mod handle_tests {
    use super::super::navigation::{CandidateAction, KEY_SYM_DOWN, KEY_SYM_SEMICOLON};
    use super::super::{
        handle_key_event, EngineKeyEvent, ImSwitchAction, InputMethodSelection,
        SurroundingTextAction,
    };
    use super::super::{KEY_SYM_ALT_L, KEY_SYM_CONTROL_L, KEY_SYM_SHIFT_L, KEY_SYM_SPACE};

    fn base_event() -> EngineKeyEvent<'static> {
        EngineKeyEvent {
            key_sym: 0x61,
            ctrl: false,
            shift: false,
            alt: false,
            plain_shortcut: true,
            is_release: false,
            hotkey_toggle: Some("Ctrl+Space"),
            hotkey_next: Some("Ctrl+Shift"),
            surrounding_text_valid: true,
            current_surrounding_valid: false,
            has_request_im: false,
            request_im_valid: false,
            default_im_valid: true,
            default_im_nonempty: true,
            current_eq_request: false,
            current_eq_default: true,
            im_overridden: false,
            has_candidates: false,
            candidate_count: 0,
            candidate_list_size: 0,
            candidate_cursor: 0,
            candidate_bulk_cursor: -1,
            candidate_has_bulk_cursor: false,
            candidate_has_bulk: false,
            candidate_pageable: false,
            candidate_has_prev: false,
            candidate_has_next: false,
            scroll_mode: false,
            vertical: false,
            candidate_page_size: None,
            has_override: false,
            override_value: 0,
        }
    }

    #[test]
    fn handle_forward_key_clears_override() {
        let event = base_event();
        let decision = handle_key_event(&event);
        assert_eq!(decision.surrounding.action, SurroundingTextAction::Set);
        assert_eq!(decision.im_selection, InputMethodSelection::NoChange);
        assert_eq!(decision.im_switch, None);
        assert!(decision.candidate.is_none());
        assert!(decision.clear_override);
        assert!(decision.forward_key);
    }

    #[test]
    fn handle_modifier_forward_keeps_override() {
        let mut event = base_event();
        event.key_sym = KEY_SYM_SHIFT_L;
        let decision = handle_key_event(&event);
        assert!(!decision.clear_override);
        assert!(decision.forward_key);
        event.key_sym = KEY_SYM_CONTROL_L;
        assert!(!handle_key_event(&event).clear_override);
        event.key_sym = KEY_SYM_ALT_L;
        assert!(!handle_key_event(&event).clear_override);
    }

    #[test]
    fn handle_hotkey_switch_is_main_path() {
        let mut event = base_event();
        event.key_sym = KEY_SYM_SPACE;
        event.ctrl = true;
        event.plain_shortcut = false;
        let decision = handle_key_event(&event);
        assert_eq!(decision.im_switch, Some(ImSwitchAction::Toggle));
        assert!(!decision.forward_key);
        assert!(!decision.clear_override);
        assert!(decision.candidate.is_none());
    }

    #[test]
    fn handle_candidate_navigation_is_main_path() {
        let mut event = base_event();
        event.has_candidates = true;
        event.candidate_count = 10;
        event.candidate_list_size = 10;
        event.candidate_pageable = true;
        event.key_sym = KEY_SYM_SEMICOLON;
        let decision = handle_key_event(&event);
        assert_eq!(decision.im_switch, None);
        assert!(!decision.forward_key);
        let candidate = decision.candidate.expect("candidate decision");
        assert!(candidate.consume);
        assert_eq!(candidate.action, CandidateAction::SelectAndClear(1));
    }

    #[test]
    fn handle_candidate_scroll_down_sets_override() {
        let mut event = base_event();
        event.has_candidates = true;
        event.candidate_count = 20;
        event.candidate_list_size = 10;
        event.candidate_has_bulk = true;
        event.candidate_pageable = true;
        event.scroll_mode = true;
        event.vertical = true;
        event.candidate_page_size = Some(5);
        event.key_sym = KEY_SYM_DOWN;
        event.has_override = true;
        event.override_value = 1;
        let decision = handle_key_event(&event);
        let candidate = decision.candidate.expect("candidate decision");
        assert!(candidate.consume);
        assert_eq!(candidate.action, CandidateAction::SetOverride(2));
    }

    #[test]
    fn handle_release_event_skips_decisions() {
        let mut event = base_event();
        event.is_release = true;
        event.key_sym = KEY_SYM_SPACE;
        event.ctrl = true;
        let decision = handle_key_event(&event);
        assert_eq!(decision.im_switch, None);
        assert!(decision.candidate.is_none());
        assert!(!decision.clear_override);
        assert!(decision.forward_key);
    }

    #[test]
    fn handle_im_selection_decided_always() {
        let mut event = base_event();
        event.current_eq_default = false; // default differs -> SelectDefault
        let decision = handle_key_event(&event);
        assert_eq!(decision.im_selection, InputMethodSelection::SelectDefault);
        // But a hotkey match is still the main path.
        event.key_sym = KEY_SYM_SPACE;
        event.ctrl = true;
        event.plain_shortcut = false;
        let decision = handle_key_event(&event);
        assert_eq!(decision.im_selection, InputMethodSelection::SelectDefault);
        assert_eq!(decision.im_switch, Some(ImSwitchAction::Toggle));
    }
}

// ---------------------------------------------------------------------------
// E4: engine epoch
// ---------------------------------------------------------------------------

use super::{generate_engine_epoch, validate_engine_epoch};

#[test]
fn engine_epoch_generation_is_plausible_filetime() {
    let epoch = generate_engine_epoch();
    // FILETIME for 2026 is ~1.34e17 (100ns since 1601); assert magnitude.
    assert!(epoch > 130_000_000_000_000_000, "epoch too small: {epoch}");
    assert!(epoch < 140_000_000_000_000_000, "epoch too large: {epoch}");
}

#[test]
fn engine_epoch_generation_monotonic_enough() {
    let first = generate_engine_epoch();
    let second = generate_engine_epoch();
    assert!(second >= first);
}

#[test]
fn engine_epoch_validation_matches() {
    assert!(validate_engine_epoch(42, 42));
    assert!(!validate_engine_epoch(41, 42));
    assert!(!validate_engine_epoch(42, 43));
}

// ---------------------------------------------------------------------------
// E4-2: request sequence and deadline policy
// ---------------------------------------------------------------------------

use super::{accept_frame_sequence, key_request_timeout_ms};

#[test]
fn frame_sequence_accepts_strictly_newer_ids() {
    assert!(accept_frame_sequence(1, 0));
    assert!(accept_frame_sequence(10, 5));
    assert!(!accept_frame_sequence(0, 0)); // duplicate rejected
    assert!(!accept_frame_sequence(5, 5));
    assert!(!accept_frame_sequence(4, 5)); // stale rejected
}

#[test]
fn key_request_timeout_policy() {
    // Cold context (revision 0) gets the widened first-context deadline.
    assert_eq!(key_request_timeout_ms(0), 2500);
    // Warm keys get the tight input deadline.
    assert_eq!(key_request_timeout_ms(1), 75);
    assert_eq!(key_request_timeout_ms(42), 75);
}

// ---------------------------------------------------------------------------
// E5-1: snapshot/status canonicalization
// ---------------------------------------------------------------------------

use super::{content_locale_for_input_method, status_short_label, ContentLocale};

#[test]
fn content_locale_maps_input_method_families() {
    assert_eq!(content_locale_for_input_method("mozc"), ContentLocale::JaJp);
    assert_eq!(
        content_locale_for_input_method("mozcjp"),
        ContentLocale::JaJp
    );
    assert_eq!(
        content_locale_for_input_method("hangul"),
        ContentLocale::KoKr
    );
    assert_eq!(
        content_locale_for_input_method("keyboard-us"),
        ContentLocale::EnUs
    );
    assert_eq!(content_locale_for_input_method("rime"), ContentLocale::ZhCn);
    assert_eq!(
        content_locale_for_input_method("pinyin"),
        ContentLocale::ZhCn
    );
    assert_eq!(
        content_locale_for_input_method("libime"),
        ContentLocale::ZhCn
    );
    assert_eq!(
        content_locale_for_input_method("libime-pinyin"),
        ContentLocale::ZhCn
    );
    assert_eq!(
        content_locale_for_input_method("unknown"),
        ContentLocale::None
    );
    assert_eq!(content_locale_for_input_method(""), ContentLocale::None);
}

#[test]
fn status_short_label_ascii_pair_or_first_code_point() {
    assert_eq!(status_short_label(b""), b"");
    assert_eq!(status_short_label(b"ab"), b"ab");
    assert_eq!(status_short_label(b"abc"), b"ab");
    assert_eq!(status_short_label("中文".as_bytes()), "中".as_bytes());
    // Mixed leading ASCII then multibyte: the ASCII lead is one code point.
    assert_eq!(status_short_label(b"a\xe4\xb8\xad"), b"a");
    // Non-ASCII lead: first code point (3-byte).
    assert_eq!(status_short_label("中".as_bytes()), "中".as_bytes());
    // Overlong (invalid UTF-8) lead is still classified by the leading byte.
    assert_eq!(status_short_label(b"\xc0\x80"), b"\xc0\x80");
}

// ---------------------------------------------------------------------------
// E5-2: snapshot DTO limits validation
// ---------------------------------------------------------------------------

use super::{validate_snapshot, SnapshotFacts};

fn facts() -> SnapshotFacts {
    SnapshotFacts {
        commit_utf8_len: 3,
        preedit_utf8_len: 2,
        candidate_count: 5,
        candidate_label_len_max: 1,
        candidate_text_len_max: 4,
        candidate_comment_len_max: 8,
        content_locale_utf8_len: 5,
    }
}

#[test]
fn snapshot_validation_accepts_normal_facts() {
    assert!(validate_snapshot(&facts()));
}

#[test]
fn snapshot_validation_rejects_oversized_fields() {
    let mut f = facts();
    f.commit_utf8_len = 16 * 1024 + 1;
    assert!(!validate_snapshot(&f));
    f = facts();
    f.preedit_utf8_len = 16 * 1024 + 1;
    assert!(!validate_snapshot(&f));
    f = facts();
    f.candidate_count = 129;
    assert!(!validate_snapshot(&f));
    f = facts();
    f.candidate_label_len_max = 4097;
    assert!(!validate_snapshot(&f));
    f = facts();
    f.candidate_text_len_max = 4097;
    assert!(!validate_snapshot(&f));
    f = facts();
    f.candidate_comment_len_max = 4097;
    assert!(!validate_snapshot(&f));
    f = facts();
    f.content_locale_utf8_len = 36;
    assert!(!validate_snapshot(&f));
}

#[test]
fn snapshot_validation_accepts_boundary_values() {
    let mut f = facts();
    f.commit_utf8_len = 16 * 1024;
    f.preedit_utf8_len = 16 * 1024;
    f.candidate_count = 128;
    f.candidate_label_len_max = 4096;
    f.candidate_text_len_max = 4096;
    f.candidate_comment_len_max = 4096;
    f.content_locale_utf8_len = 35;
    assert!(validate_snapshot(&f));
}

// ---------------------------------------------------------------------------
// E5-3: canonical snapshot blob codec + pending store
// ---------------------------------------------------------------------------

mod snapshot_tests {
    use super::super::snapshot::{decode_snapshot, encode_snapshot, Candidate, EngineSnapshot};
    use super::super::ContextLedger;

    fn sample() -> EngineSnapshot {
        EngineSnapshot {
            handled: true,
            preedit_caret_utf8: 2,
            composition_id: 7,
            revision: 4,
            selected_candidate: 1,
            candidate_page: 0,
            candidate_total: 2,
            candidate_visibility: 1,
            candidate_page_size: 2,
            candidate_bulk: false,
            candidate_end: true,
            delete_surrounding_text: true,
            delete_surrounding_offset: -1,
            delete_surrounding_size: 2,
            forward_key: true,
            forward_key_sym: 0x1234,
            forward_key_states: 0x7,
            forward_key_code: 42,
            forward_key_release: false,
            caret_valid: true,
            caret_left: -100,
            caret_top: 200,
            caret_right: -98,
            caret_bottom: 222,
            caret_dpi: 144,
            popup_allowed: true,
            commit_utf8: b"hi".to_vec(),
            preedit_utf8: vec![0xe4, 0xbd, 0xa0],
            content_locale_utf8: b"zh-CN".to_vec(),
            candidates: vec![Candidate {
                id: (7 << 8) | 1,
                label: b"1".to_vec(),
                text: vec![0xe4, 0xbd, 0xa0],
                comment: vec![0x6e, 0xc7, 0x90],
            }],
        }
    }

    #[test]
    fn snapshot_blob_roundtrip() {
        let snapshot = sample();
        let blob = encode_snapshot(&snapshot);
        let decoded = decode_snapshot(&blob).expect("valid blob");
        assert_eq!(decoded, snapshot);
    }

    #[test]
    fn snapshot_blob_rejects_malformed_input() {
        assert!(decode_snapshot(&[]).is_none());
        assert!(decode_snapshot(&[0xff; 4]).is_none());
        // Truncated candidate count with candidates.
        let blob = encode_snapshot(&sample());
        assert!(decode_snapshot(&blob[..blob.len() - 1]).is_none());
        // Trailing garbage.
        let mut bad = blob.clone();
        bad.push(0);
        assert!(decode_snapshot(&bad).is_none());
    }

    #[test]
    fn snapshot_blob_rejects_oversized_candidate_count() {
        let mut snapshot = sample();
        snapshot.candidates = (0..129)
            .map(|i| Candidate {
                id: i,
                label: Vec::new(),
                text: Vec::new(),
                comment: Vec::new(),
            })
            .collect();
        let blob = encode_snapshot(&snapshot);
        assert!(decode_snapshot(&blob).is_none());
    }

    #[test]
    fn pending_store_put_take_revision_ordering() {
        let mut ledger = ContextLedger::new();
        let key = super::key_a();
        assert!(ledger.snapshot_take(key, 0).is_none());
        // put with revision 5.
        ledger.snapshot_put(key, 5, sample());
        // Request revision >= stored revision is rejected.
        assert!(ledger.snapshot_take(key, 5).is_none());
        assert!(ledger.snapshot_take(key, 6).is_none());
        // Strictly older request revision takes and removes the entry.
        let taken = ledger
            .snapshot_take(key, 4)
            .expect("stale request must take");
        assert_eq!(taken.revision, 4);
        assert!(ledger.snapshot_take(key, 4).is_none());
    }

    #[test]
    fn pending_store_forget_clears() {
        let mut ledger = ContextLedger::new();
        let key = super::key_b();
        ledger.snapshot_put(key, 1, sample());
        ledger.forget(key);
        assert!(ledger.snapshot_take(key, 0).is_none());
    }
}

// ---------------------------------------------------------------------------
// E6: scroll label offset canonicalization
// ---------------------------------------------------------------------------

use super::navigation::scroll_label_offset;

#[test]
fn scroll_label_offset_vertical_uses_column_row() {
    // Vertical: column_selection_row(4, 7, 5, 20): 4/5=0, 7/5=1 -> different
    // rows -> None.
    assert_eq!(scroll_label_offset(true, 4, 7, 5, 20), None);
    // Same row: column_selection_row(4, 2, 5, 20): 4/5=0, 2/5=0 -> 2%5=2.
    assert_eq!(scroll_label_offset(true, 4, 2, 5, 20), Some(2));
}

#[test]
fn scroll_label_offset_horizontal_uses_row_column() {
    // Horizontal: row_selection_column(4, 6, 5, 20): 4/5=0, 6/5=1 -> None.
    assert_eq!(scroll_label_offset(false, 4, 6, 5, 20), None);
    // Same row: row_selection_column(4, 1, 5, 20) -> 1%5=1.
    assert_eq!(scroll_label_offset(false, 4, 1, 5, 20), Some(1));
}
