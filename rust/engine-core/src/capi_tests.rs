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
