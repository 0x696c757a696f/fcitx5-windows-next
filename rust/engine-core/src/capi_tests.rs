#![deny(unsafe_op_in_unsafe_fn)]

// SAFETY: every ABI call below uses only test-owned handles and pointers that
// remain valid and uniquely writable for the duration of that call.
use super::{
    fcitx5_engine_core_ledger_begin_key, fcitx5_engine_core_ledger_end_result,
    fcitx5_engine_core_ledger_forget, fcitx5_engine_core_ledger_free,
    fcitx5_engine_core_ledger_new, fcitx5_engine_core_ledger_select_candidate,
    FcitxEngineContextKeyC, FCITX_ENGINE_CORE_INVALID_CANDIDATE, FCITX_ENGINE_CORE_OK,
    FCITX_ENGINE_CORE_STALE,
};

fn key() -> FcitxEngineContextKeyC {
    FcitxEngineContextKeyC {
        process_id: 42,
        connection_id: 3,
        context_id: 11,
    }
}

fn new_ledger() -> *mut std::ffi::c_void {
    fcitx5_engine_core_ledger_new()
}

fn free_ledger(ledger: *mut std::ffi::c_void) {
    unsafe { fcitx5_engine_core_ledger_free(ledger) };
}

#[test]
fn ledger_new_free_roundtrip() {
    let ledger = new_ledger();
    assert!(!ledger.is_null());
    free_ledger(ledger);
    // Null free is a no-op.
    unsafe { fcitx5_engine_core_ledger_free(std::ptr::null_mut()) };
}

#[test]
fn begin_key_accepts_unknown_context() {
    let ledger = new_ledger();
    let key = key();
    assert_eq!(
        unsafe { fcitx5_engine_core_ledger_begin_key(ledger, &key, 0, 0) },
        FCITX_ENGINE_CORE_OK
    );
    free_ledger(ledger);
}

#[test]
fn begin_key_rejects_stale() {
    let ledger = new_ledger();
    let key = key();
    assert_eq!(
        unsafe { fcitx5_engine_core_ledger_begin_key(ledger, &key, 1, 0) },
        FCITX_ENGINE_CORE_STALE
    );
    free_ledger(ledger);
}

#[test]
fn end_result_roundtrip_writes_outputs() {
    let ledger = new_ledger();
    let key = key();
    let mut composition = u64::MAX;
    let mut revision = u64::MAX;
    assert_eq!(
        unsafe {
            fcitx5_engine_core_ledger_end_result(ledger, &key, 1, &mut composition, &mut revision)
        },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(composition, 1);
    assert_eq!(revision, 1);
    // Follow-up key request with the produced metadata is accepted.
    assert_eq!(
        unsafe { fcitx5_engine_core_ledger_begin_key(ledger, &key, revision, composition) },
        FCITX_ENGINE_CORE_OK
    );
    // Pre-result metadata is stale.
    assert_eq!(
        unsafe { fcitx5_engine_core_ledger_begin_key(ledger, &key, 0, 0) },
        FCITX_ENGINE_CORE_STALE
    );
    free_ledger(ledger);
}

#[test]
fn end_result_resets_composition_without_content() {
    let ledger = new_ledger();
    let key = key();
    let mut composition = 0;
    let mut revision = 0;
    unsafe {
        fcitx5_engine_core_ledger_end_result(ledger, &key, 1, &mut composition, &mut revision);
    }
    assert_eq!(composition, 1);
    assert_eq!(revision, 1);
    unsafe {
        fcitx5_engine_core_ledger_end_result(ledger, &key, 0, &mut composition, &mut revision);
    }
    assert_eq!(composition, 0);
    assert_eq!(revision, 2);
    free_ledger(ledger);
}

#[test]
fn select_candidate_error_codes() {
    let ledger = new_ledger();
    let key = key();
    let mut composition = 0;
    let mut revision = 0;
    unsafe {
        fcitx5_engine_core_ledger_end_result(ledger, &key, 1, &mut composition, &mut revision);
    }
    let candidate_id = (composition << 8) | 1;
    assert_eq!(
        unsafe {
            fcitx5_engine_core_ledger_select_candidate(
                ledger,
                &key,
                revision,
                composition,
                candidate_id,
            )
        },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        unsafe {
            fcitx5_engine_core_ledger_select_candidate(ledger, &key, revision, composition, 0)
        },
        FCITX_ENGINE_CORE_INVALID_CANDIDATE
    );
    assert_eq!(
        unsafe {
            fcitx5_engine_core_ledger_select_candidate(ledger, &key, 0, composition, candidate_id)
        },
        FCITX_ENGINE_CORE_STALE
    );
    free_ledger(ledger);
}

#[test]
fn forget_resets_context() {
    let ledger = new_ledger();
    let key = key();
    let mut composition = 0;
    let mut revision = 0;
    unsafe {
        fcitx5_engine_core_ledger_end_result(ledger, &key, 1, &mut composition, &mut revision);
    }
    unsafe { fcitx5_engine_core_ledger_forget(ledger, &key) };
    assert_eq!(
        unsafe { fcitx5_engine_core_ledger_begin_key(ledger, &key, 0, 0) },
        FCITX_ENGINE_CORE_OK
    );
    free_ledger(ledger);
}

#[test]
fn null_pointers_fail_closed() {
    assert_eq!(
        unsafe {
            fcitx5_engine_core_ledger_begin_key(std::ptr::null_mut(), std::ptr::null(), 0, 0)
        },
        FCITX_ENGINE_CORE_STALE
    );
    assert_eq!(
        unsafe {
            fcitx5_engine_core_ledger_end_result(
                std::ptr::null_mut(),
                std::ptr::null(),
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        },
        FCITX_ENGINE_CORE_STALE
    );
    // Null output pointers fail closed without writing.
    let ledger = new_ledger();
    let key = key();
    assert_eq!(
        unsafe {
            fcitx5_engine_core_ledger_end_result(
                ledger,
                &key,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        },
        FCITX_ENGINE_CORE_STALE
    );
    free_ledger(ledger);
}

// ---------------------------------------------------------------------------
// E2 extension: per-context product state C ABI
// ---------------------------------------------------------------------------

use super::{
    fcitx5_engine_core_caret, fcitx5_engine_core_clear_selected_override,
    fcitx5_engine_core_input_method_overridden, fcitx5_engine_core_popup_allowed,
    fcitx5_engine_core_selected_override, fcitx5_engine_core_set_caret,
    fcitx5_engine_core_set_input_method_overridden, fcitx5_engine_core_set_popup_allowed,
    fcitx5_engine_core_set_selected_override,
};
use crate::CaretRect;

#[test]
fn caret_c_abi_roundtrip() {
    let ledger = new_ledger();
    let key = key();
    let caret = CaretRect {
        valid: 1,
        left: -10,
        top: 20,
        right: 8,
        bottom: 30,
        dpi: 144,
    };
    assert_eq!(
        unsafe { fcitx5_engine_core_set_caret(ledger, &key, &caret) },
        FCITX_ENGINE_CORE_OK
    );
    let mut out = CaretRect {
        valid: 0,
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
        dpi: 0,
    };
    assert_eq!(
        unsafe { fcitx5_engine_core_caret(ledger, &key, &mut out) },
        1
    );
    assert_eq!(out, caret);
    free_ledger(ledger);
}

#[test]
fn caret_c_abi_absent_returns_zero() {
    let ledger = new_ledger();
    let key = key();
    let mut out = CaretRect {
        valid: 0,
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
        dpi: 0,
    };
    assert_eq!(
        unsafe { fcitx5_engine_core_caret(ledger, &key, &mut out) },
        0
    );
    free_ledger(ledger);
}

#[test]
fn popup_allowed_c_abi_roundtrip() {
    let ledger = new_ledger();
    let key = key();
    assert_eq!(
        unsafe { fcitx5_engine_core_set_popup_allowed(ledger, &key, 0) },
        FCITX_ENGINE_CORE_OK
    );
    let mut out = 1;
    assert_eq!(
        unsafe { fcitx5_engine_core_popup_allowed(ledger, &key, &mut out) },
        1
    );
    assert_eq!(out, 0);
    assert_eq!(
        unsafe { fcitx5_engine_core_set_popup_allowed(ledger, &key, 1) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_popup_allowed(ledger, &key, &mut out) },
        1
    );
    assert_eq!(out, 1);
    free_ledger(ledger);
}

#[test]
fn selected_override_c_abi_set_query_clear() {
    let ledger = new_ledger();
    let key = key();
    let mut out = 0;
    assert_eq!(
        unsafe { fcitx5_engine_core_selected_override(ledger, &key, &mut out) },
        0
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_set_selected_override(ledger, &key, 4) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_selected_override(ledger, &key, &mut out) },
        1
    );
    assert_eq!(out, 4);
    // Some(0) is still reported as present.
    assert_eq!(
        unsafe { fcitx5_engine_core_set_selected_override(ledger, &key, 0) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_selected_override(ledger, &key, &mut out) },
        1
    );
    assert_eq!(out, 0);
    assert_eq!(
        unsafe { fcitx5_engine_core_clear_selected_override(ledger, &key) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_selected_override(ledger, &key, &mut out) },
        0
    );
    free_ledger(ledger);
}

#[test]
fn input_method_overridden_c_abi() {
    let ledger = new_ledger();
    let key = key();
    let mut out = 0;
    assert_eq!(
        unsafe { fcitx5_engine_core_input_method_overridden(ledger, &key, &mut out) },
        0
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_set_input_method_overridden(ledger, &key, 1) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_input_method_overridden(ledger, &key, &mut out) },
        1
    );
    assert_eq!(out, 1);
    assert_eq!(
        unsafe { fcitx5_engine_core_set_input_method_overridden(ledger, &key, 0) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_input_method_overridden(ledger, &key, &mut out) },
        0
    );
    free_ledger(ledger);
}

// ---------------------------------------------------------------------------
// E3: input-method switch decision C ABI
// ---------------------------------------------------------------------------

use super::{
    fcitx5_engine_core_classify_input_method_switch, FCITX_ENGINE_CORE_IM_ACTION_NEXT,
    FCITX_ENGINE_CORE_IM_ACTION_TOGGLE,
};
use crate::KEY_SYM_SPACE;

fn cstr(value: &str) -> std::ffi::CString {
    std::ffi::CString::new(value).expect("no NUL in test hotkey")
}

#[test]
fn classify_c_abi_toggle_and_next() {
    let toggle = cstr("Ctrl+Space");
    let next = cstr("Ctrl+Shift");
    let mut action = 0;
    assert_eq!(
        unsafe {
            fcitx5_engine_core_classify_input_method_switch(
                1,
                0,
                0,
                KEY_SYM_SPACE,
                toggle.as_ptr(),
                next.as_ptr(),
                &mut action,
            )
        },
        1
    );
    assert_eq!(action, FCITX_ENGINE_CORE_IM_ACTION_TOGGLE);
    assert_eq!(
        unsafe {
            fcitx5_engine_core_classify_input_method_switch(
                1,
                1,
                0,
                crate::KEY_SYM_SHIFT_L,
                toggle.as_ptr(),
                next.as_ptr(),
                &mut action,
            )
        },
        1
    );
    assert_eq!(action, FCITX_ENGINE_CORE_IM_ACTION_NEXT);
}

#[test]
fn classify_c_abi_no_match_and_null_hotkeys() {
    let toggle = cstr("Ctrl+Space");
    let next = cstr("Ctrl+Shift");
    let mut action = -1;
    // Plain 'A' with Ctrl: not a hotkey.
    assert_eq!(
        unsafe {
            fcitx5_engine_core_classify_input_method_switch(
                1,
                0,
                0,
                0x41,
                toggle.as_ptr(),
                next.as_ptr(),
                &mut action,
            )
        },
        0
    );
    // Null hotkeys = not configured.
    assert_eq!(
        unsafe {
            fcitx5_engine_core_classify_input_method_switch(
                1,
                0,
                0,
                KEY_SYM_SPACE,
                std::ptr::null(),
                std::ptr::null(),
                &mut action,
            )
        },
        0
    );
    // Null output pointer fails closed.
    assert_eq!(
        unsafe {
            fcitx5_engine_core_classify_input_method_switch(
                1,
                0,
                0,
                KEY_SYM_SPACE,
                toggle.as_ptr(),
                next.as_ptr(),
                std::ptr::null_mut(),
            )
        },
        0
    );
}
// ---------------------------------------------------------------------------
// E3-2: candidate navigation decision C ABI
// ---------------------------------------------------------------------------

use super::{
    fcitx5_engine_core_decide_candidate_action, FcitxCandidateConfigC, FcitxCandidateDecisionC,
    FcitxCandidateViewC, FCITX_ENGINE_CORE_CANDIDATE_ACTION_NONE,
    FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_NEXT_AND_SET_OVERRIDE,
    FCITX_ENGINE_CORE_CANDIDATE_ACTION_SELECT_AND_CLEAR,
};
use crate::navigation::KEY_SYM_SEMICOLON;

fn view10() -> FcitxCandidateViewC {
    FcitxCandidateViewC {
        count: 10,
        list_size: 10,
        cursor: 0,
        bulk_cursor: -1,
        has_bulk_cursor: 0,
        has_bulk: 0,
        pageable: 1,
        has_prev: 0,
        has_next: 0,
    }
}

fn config_plain() -> FcitxCandidateConfigC {
    FcitxCandidateConfigC {
        scroll_mode: 0,
        vertical: 0,
        candidate_page_size: -1,
    }
}

#[test]
fn decide_c_abi_semicolon_selects_second() {
    let view = view10();
    let config = config_plain();
    let mut out = FcitxCandidateDecisionC {
        consume: 0,
        action: -1,
        value: 0,
    };
    assert_eq!(
        unsafe {
            fcitx5_engine_core_decide_candidate_action(
                KEY_SYM_SEMICOLON,
                1,
                &view,
                &config,
                0,
                0,
                &mut out,
            )
        },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(out.consume, 1);
    assert_eq!(
        out.action,
        FCITX_ENGINE_CORE_CANDIDATE_ACTION_SELECT_AND_CLEAR
    );
    assert_eq!(out.value, 1);
}

#[test]
fn decide_c_abi_space_commits_override() {
    let view = view10();
    let config = config_plain();
    let mut out = FcitxCandidateDecisionC {
        consume: 0,
        action: -1,
        value: 0,
    };
    assert_eq!(
        unsafe {
            fcitx5_engine_core_decide_candidate_action(0x20, 1, &view, &config, 1, 3, &mut out)
        },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        out.action,
        FCITX_ENGINE_CORE_CANDIDATE_ACTION_SELECT_AND_CLEAR
    );
    assert_eq!(out.value, 3);
}

#[test]
fn decide_c_abi_scroll_next_page() {
    let view = FcitxCandidateViewC {
        count: 20,
        list_size: 10,
        cursor: 0,
        bulk_cursor: 19,
        has_bulk_cursor: 1,
        has_bulk: 1,
        pageable: 1,
        has_prev: 0,
        has_next: 1,
    };
    let config = FcitxCandidateConfigC {
        scroll_mode: 1,
        vertical: 1,
        candidate_page_size: 5,
    };
    let mut out = FcitxCandidateDecisionC {
        consume: 0,
        action: -1,
        value: 0,
    };
    // '=' next page in scroll viewport at the end -> PageNext + override 0.
    assert_eq!(
        unsafe {
            fcitx5_engine_core_decide_candidate_action(0x3d, 1, &view, &config, 1, 19, &mut out)
        },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        out.action,
        FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_NEXT_AND_SET_OVERRIDE
    );
    assert_eq!(out.value, 0);
}

#[test]
fn decide_c_abi_ordinary_key_not_consumed() {
    let view = view10();
    let config = config_plain();
    let mut out = FcitxCandidateDecisionC {
        consume: 0,
        action: -1,
        value: 0,
    };
    assert_eq!(
        unsafe {
            fcitx5_engine_core_decide_candidate_action(0x61, 1, &view, &config, 0, 0, &mut out)
        },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(out.consume, 0);
    assert_eq!(out.action, FCITX_ENGINE_CORE_CANDIDATE_ACTION_NONE);
}

#[test]
fn decide_c_abi_null_input_fails_closed() {
    let view = view10();
    let config = config_plain();
    let mut out = FcitxCandidateDecisionC {
        consume: 0,
        action: -1,
        value: 0,
    };
    assert_eq!(
        unsafe {
            fcitx5_engine_core_decide_candidate_action(
                KEY_SYM_SEMICOLON,
                1,
                std::ptr::null(),
                &config,
                0,
                0,
                &mut out,
            )
        },
        FCITX_ENGINE_CORE_STALE
    );
    assert_eq!(
        unsafe {
            fcitx5_engine_core_decide_candidate_action(
                KEY_SYM_SEMICOLON,
                1,
                &view,
                std::ptr::null(),
                0,
                0,
                &mut out,
            )
        },
        FCITX_ENGINE_CORE_STALE
    );
    assert_eq!(
        unsafe {
            fcitx5_engine_core_decide_candidate_action(
                KEY_SYM_SEMICOLON,
                1,
                &view,
                &config,
                0,
                0,
                std::ptr::null_mut(),
            )
        },
        FCITX_ENGINE_CORE_STALE
    );
}

// ---------------------------------------------------------------------------
// E3-3: surrounding-text and input-method-selection C ABI
// ---------------------------------------------------------------------------

use super::{
    fcitx5_engine_core_decide_input_method_selection, fcitx5_engine_core_decide_surrounding_text,
    FcitxSurroundingTextDecisionC, FCITX_ENGINE_CORE_IM_SELECTION_DEFAULT,
    FCITX_ENGINE_CORE_IM_SELECTION_NONE, FCITX_ENGINE_CORE_IM_SELECTION_REQUEST,
    FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_INVALIDATE,
    FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_SET,
};

#[test]
fn surrounding_text_c_abi_set_and_invalidate() {
    let mut out = FcitxSurroundingTextDecisionC {
        action: -1,
        update: 0,
    };
    assert_eq!(
        unsafe { fcitx5_engine_core_decide_surrounding_text(1, 0, &mut out) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(out.action, FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_SET);
    assert_eq!(out.update, 1);
    assert_eq!(
        unsafe { fcitx5_engine_core_decide_surrounding_text(0, 1, &mut out) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        out.action,
        FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_INVALIDATE
    );
    assert_eq!(out.update, 1);
    assert_eq!(
        unsafe { fcitx5_engine_core_decide_surrounding_text(0, 0, &mut out) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        out.action,
        FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_INVALIDATE
    );
    assert_eq!(out.update, 0);
    assert_eq!(
        unsafe { fcitx5_engine_core_decide_surrounding_text(1, 1, std::ptr::null_mut()) },
        FCITX_ENGINE_CORE_STALE
    );
}

#[test]
fn input_method_selection_c_abi() {
    let mut out = -1;
    // Valid request, current differs -> SelectRequest.
    assert_eq!(
        unsafe { fcitx5_engine_core_decide_input_method_selection(1, 1, 1, 1, 0, 0, 0, &mut out) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(out, FCITX_ENGINE_CORE_IM_SELECTION_REQUEST);
    // No request -> valid default, differs -> SelectDefault.
    assert_eq!(
        unsafe { fcitx5_engine_core_decide_input_method_selection(0, 0, 1, 1, 0, 0, 0, &mut out) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(out, FCITX_ENGINE_CORE_IM_SELECTION_DEFAULT);
    // Current already matches -> none.
    assert_eq!(
        unsafe { fcitx5_engine_core_decide_input_method_selection(1, 1, 1, 1, 1, 0, 0, &mut out) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(out, FCITX_ENGINE_CORE_IM_SELECTION_NONE);
    // Overridden -> none.
    assert_eq!(
        unsafe { fcitx5_engine_core_decide_input_method_selection(1, 1, 1, 1, 0, 0, 1, &mut out) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(out, FCITX_ENGINE_CORE_IM_SELECTION_NONE);
    // Null output -> fail closed.
    assert_eq!(
        unsafe {
            fcitx5_engine_core_decide_input_method_selection(
                1,
                1,
                1,
                1,
                0,
                0,
                0,
                std::ptr::null_mut(),
            )
        },
        FCITX_ENGINE_CORE_STALE
    );
}

// ---------------------------------------------------------------------------
// E3 event-shape consolidation: handle_key_event C ABI
// ---------------------------------------------------------------------------

use super::{fcitx5_engine_core_handle_key_event, FcitxEngineKeyDecisionC, FcitxEngineKeyEventC};
use std::ffi::CString;

fn key_event_c() -> FcitxEngineKeyEventC {
    FcitxEngineKeyEventC {
        key_sym: 0x61,
        key_flags: 0,
        is_release: 0,
        hotkey_toggle: std::ptr::null(),
        hotkey_next: std::ptr::null(),
        surrounding_text_valid: 1,
        current_surrounding_valid: 0,
        has_request_im: 0,
        request_im_valid: 0,
        default_im_valid: 1,
        default_im_nonempty: 1,
        current_eq_request: 0,
        current_eq_default: 1,
        im_overridden: 0,
        has_candidates: 0,
        view: FcitxCandidateViewC {
            count: 0,
            list_size: 0,
            cursor: 0,
            bulk_cursor: -1,
            has_bulk_cursor: 0,
            has_bulk: 0,
            pageable: 0,
            has_prev: 0,
            has_next: 0,
        },
        config: FcitxCandidateConfigC {
            scroll_mode: 0,
            vertical: 0,
            candidate_page_size: -1,
        },
        has_override: 0,
        override_value: 0,
    }
}

#[test]
fn handle_key_event_c_abi_forward() {
    let event = key_event_c();
    let mut out = FcitxEngineKeyDecisionC::default();
    assert_eq!(
        unsafe { fcitx5_engine_core_handle_key_event(&event, &mut out) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(
        out.surrounding_action,
        FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_SET
    );
    assert_eq!(out.surrounding_update, 1);
    assert_eq!(out.im_selection, FCITX_ENGINE_CORE_IM_SELECTION_NONE);
    assert_eq!(out.im_switch, 0);
    assert_eq!(out.candidate_consume, 0);
    assert_eq!(
        out.candidate_action,
        FCITX_ENGINE_CORE_CANDIDATE_ACTION_NONE
    );
    assert_eq!(out.clear_override, 1);
    assert_eq!(out.forward_key, 1);
}

#[test]
fn handle_key_event_c_abi_hotkey() {
    let toggle = CString::new("Ctrl+Space").unwrap();
    let next = CString::new("Ctrl+Shift").unwrap();
    let mut event = key_event_c();
    event.key_sym = 0x20;
    event.key_flags = 0x2; // kKeyFlagControl
    event.hotkey_toggle = toggle.as_ptr();
    event.hotkey_next = next.as_ptr();
    let mut out = FcitxEngineKeyDecisionC::default();
    assert_eq!(
        unsafe { fcitx5_engine_core_handle_key_event(&event, &mut out) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(out.im_switch, FCITX_ENGINE_CORE_IM_ACTION_TOGGLE);
    assert_eq!(out.forward_key, 0);
    assert_eq!(out.clear_override, 0);

    let mut next_event = key_event_c();
    next_event.key_sym = 0xffe1;
    next_event.key_flags = 0x3; // kKeyFlagControl | kKeyFlagShift
    next_event.hotkey_toggle = toggle.as_ptr();
    next_event.hotkey_next = next.as_ptr();
    let mut next_out = FcitxEngineKeyDecisionC::default();
    assert_eq!(
        unsafe { fcitx5_engine_core_handle_key_event(&next_event, &mut next_out) },
        FCITX_ENGINE_CORE_OK
    );
    assert_eq!(next_out.im_switch, FCITX_ENGINE_CORE_IM_ACTION_NEXT);
    assert_eq!(next_out.forward_key, 0);
    assert_eq!(next_out.clear_override, 0);
}

#[test]
fn handle_key_event_c_abi_null_fails_closed() {
    let event = key_event_c();
    assert_eq!(
        unsafe { fcitx5_engine_core_handle_key_event(std::ptr::null(), std::ptr::null_mut()) },
        FCITX_ENGINE_CORE_STALE
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_handle_key_event(&event, std::ptr::null_mut()) },
        FCITX_ENGINE_CORE_STALE
    );
}

// ---------------------------------------------------------------------------
// E5-2: snapshot validation C ABI
// ---------------------------------------------------------------------------

use super::{fcitx5_engine_core_validate_snapshot, FcitxEngineSnapshotC};

fn snapshot_c() -> FcitxEngineSnapshotC {
    FcitxEngineSnapshotC {
        handled: 1,
        commit_utf8_len: 3,
        preedit_utf8_len: 2,
        preedit_caret_utf8: 1,
        composition_id: 1,
        revision: 1,
        candidate_count: 5,
        candidate_label_len_max: 1,
        candidate_text_len_max: 4,
        candidate_comment_len_max: 8,
        content_locale_utf8_len: 5,
        selected_candidate: 0,
        candidate_page: 0,
        candidate_total: 5,
        candidate_visibility: 1,
        candidate_page_size: 5,
        candidate_bulk: 0,
        candidate_end: 1,
        delete_surrounding_text: 0,
        delete_surrounding_offset: 0,
        delete_surrounding_size: 0,
        forward_key: 0,
        forward_key_sym: 0,
        forward_key_states: 0,
        forward_key_code: 0,
        forward_key_release: 0,
        caret_valid: 0,
        popup_allowed: 1,
    }
}

#[test]
fn validate_snapshot_c_abi() {
    let snapshot = snapshot_c();
    assert_eq!(
        unsafe { fcitx5_engine_core_validate_snapshot(&snapshot) },
        1
    );
    // Oversized candidate count.
    let mut bad = snapshot_c();
    bad.candidate_count = 129;
    assert_eq!(unsafe { fcitx5_engine_core_validate_snapshot(&bad) }, 0);
    // Oversized commit.
    let mut bad = snapshot_c();
    bad.commit_utf8_len = 16 * 1024 + 1;
    assert_eq!(unsafe { fcitx5_engine_core_validate_snapshot(&bad) }, 0);
    // Null fails closed.
    assert_eq!(
        unsafe { fcitx5_engine_core_validate_snapshot(std::ptr::null()) },
        0
    );
}

// ---------------------------------------------------------------------------
// E4-3: per-connection session ABI
// ---------------------------------------------------------------------------

use super::{
    fcitx5_engine_core_session_accept_frame, fcitx5_engine_core_session_begin_hello,
    fcitx5_engine_core_session_complete_request, fcitx5_engine_core_session_create,
    fcitx5_engine_core_session_destroy,
};

fn new_session() -> *mut std::ffi::c_void {
    fcitx5_engine_core_session_create()
}

fn free_session(session: *mut std::ffi::c_void) {
    unsafe { fcitx5_engine_core_session_destroy(session) };
}

#[test]
fn session_c_abi_create_destroy_roundtrip() {
    let session = new_session();
    assert!(!session.is_null());
    free_session(session);
    // Null destroy is a no-op.
    unsafe { fcitx5_engine_core_session_destroy(std::ptr::null_mut()) };
}

#[test]
fn session_c_abi_hello_then_frame() {
    let session = new_session();
    assert_eq!(
        unsafe { fcitx5_engine_core_session_begin_hello(session, 1, 77, 77, 100, 100) },
        1
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_session_accept_frame(session, 2, 77, 77, 42, 42) },
        1
    );
    // Duplicate handshake rejected.
    assert_eq!(
        unsafe { fcitx5_engine_core_session_begin_hello(session, 3, 77, 77, 100, 100) },
        0
    );
    // Accepted but not completed: still retryable.
    assert_eq!(
        unsafe { fcitx5_engine_core_session_accept_frame(session, 2, 77, 77, 42, 42) },
        1
    );
    // Completed: the same id is now stale.
    assert_eq!(
        unsafe { fcitx5_engine_core_session_complete_request(session, 2) },
        1
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_session_accept_frame(session, 2, 77, 77, 42, 42) },
        0
    );
    free_session(session);
}

#[test]
fn session_c_abi_handshake_rejections() {
    let session = new_session();
    // Session mismatch.
    assert_eq!(
        unsafe { fcitx5_engine_core_session_begin_hello(session, 1, 78, 77, 100, 100) },
        0
    );
    // Process mismatch.
    assert_eq!(
        unsafe { fcitx5_engine_core_session_begin_hello(session, 1, 77, 77, 101, 100) },
        0
    );
    // Valid hello still possible after rejections.
    assert_eq!(
        unsafe { fcitx5_engine_core_session_begin_hello(session, 1, 77, 77, 100, 100) },
        1
    );
    free_session(session);
}

#[test]
fn session_c_abi_fails_closed_on_null() {
    assert_eq!(
        unsafe {
            fcitx5_engine_core_session_begin_hello(std::ptr::null_mut(), 1, 77, 77, 100, 100)
        },
        0
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_session_accept_frame(std::ptr::null_mut(), 1, 77, 77, 42, 42) },
        0
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_session_complete_request(std::ptr::null_mut(), 1) },
        0
    );
}

#[test]
fn session_c_abi_complete_request_advances_last_id() {
    let session = new_session();
    assert_eq!(
        unsafe { fcitx5_engine_core_session_begin_hello(session, 1, 77, 77, 100, 100) },
        1
    );
    // Accepted but not completed: retryable.
    assert_eq!(
        unsafe { fcitx5_engine_core_session_accept_frame(session, 2, 77, 77, 42, 42) },
        1
    );
    assert_eq!(
        unsafe { fcitx5_engine_core_session_complete_request(session, 2) },
        1
    );
    // Now stale.
    assert_eq!(
        unsafe { fcitx5_engine_core_session_accept_frame(session, 2, 77, 77, 42, 42) },
        0
    );
    free_session(session);
}
