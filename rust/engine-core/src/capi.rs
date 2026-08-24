//! C ABI for the Engine context/composition/revision ledger (E2).
//!
//! All functions return an `FcitxEngineCoreErrorC` code: `0` success,
//! `1` stale state, `2` invalid candidate id. `end_result` writes the
//! resulting composition id and revision through output pointers.
//!
//! Every entry point is contained behind a panic boundary that fails closed
//! (returns the stale error) instead of unwinding across the FFI edge.

#![allow(unsafe_code)]

use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};

use crate::{
    classify_input_method_switch, navigation, CaretRect, ContextKey, ContextLedger, ImSwitchAction,
    LedgerError,
};

/// Matches `ClientContextKey` and the C `FcitxEngineContextKeyC`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxEngineContextKeyC {
    pub process_id: u32,
    pub connection_id: u64,
    pub context_id: u64,
}

pub const FCITX_ENGINE_CORE_OK: i32 = 0;
pub const FCITX_ENGINE_CORE_STALE: i32 = 1;
pub const FCITX_ENGINE_CORE_INVALID_CANDIDATE: i32 = 2;

fn error_code(error: LedgerError) -> i32 {
    match error {
        LedgerError::StaleState => FCITX_ENGINE_CORE_STALE,
        LedgerError::InvalidCandidate => FCITX_ENGINE_CORE_INVALID_CANDIDATE,
    }
}

fn from_c(key: *const FcitxEngineContextKeyC) -> Option<ContextKey> {
    if key.is_null() {
        return None;
    }
    // SAFETY: caller provides a valid (or null) key pointer.
    let key = unsafe { *key };
    Some(ContextKey::new(
        key.process_id,
        key.connection_id,
        key.context_id,
    ))
}

/// Creates an empty ledger. Returns an opaque handle the caller must free
/// with `fcitx5_engine_core_ledger_free`.
#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_engine_core_ledger_new() -> *mut c_void {
    let result =
        panic::catch_unwind(|| Box::into_raw(Box::new(ContextLedger::new())) as *mut c_void);
    result.unwrap_or(std::ptr::null_mut())
}

/// Frees a ledger created by `fcitx5_engine_core_ledger_new`. Null is a no-op.
///
/// # Safety
/// `ledger` must be null or a pointer returned by `fcitx5_engine_core_ledger_new`
/// that has not been freed before.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_ledger_free(ledger: *mut c_void) {
    if ledger.is_null() {
        return;
    }
    let _ = panic::catch_unwind(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        unsafe {
            drop(Box::from_raw(ledger as *mut ContextLedger));
        }
    });
}

/// Drops all ledger state for a context key.
///
/// # Safety
/// `ledger` must be a valid live handle; `key` must be valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_ledger_forget(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
) {
    let Some(key) = from_c(key) else {
        return;
    };
    if ledger.is_null() {
        return;
    }
    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &mut *(ledger as *mut ContextLedger) };
        ledger.forget(key);
    }));
}

/// Validates a key request against current ledger state (mirrors the C++
/// `processKey` stale check).
///
/// # Safety
/// `ledger` must be a valid live handle; `key` must be valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_ledger_begin_key(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    revision: u64,
    composition_id: u64,
) -> i32 {
    let Some(key) = from_c(key) else {
        return FCITX_ENGINE_CORE_STALE;
    };
    if ledger.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &*(ledger as *const ContextLedger) };
        ledger.begin_key(key, revision, composition_id)
    }));
    match result {
        Ok(Ok(())) => FCITX_ENGINE_CORE_OK,
        Ok(Err(error)) => error_code(error),
        Err(_) => FCITX_ENGINE_CORE_STALE,
    }
}

/// Validates a candidate-selection request (mirrors the C++
/// `selectCandidate` stale check).
///
/// # Safety
/// `ledger` must be a valid live handle; `key` must be valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_ledger_select_candidate(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    revision: u64,
    composition_id: u64,
    candidate_id: u64,
) -> i32 {
    let Some(key) = from_c(key) else {
        return FCITX_ENGINE_CORE_STALE;
    };
    if ledger.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &*(ledger as *const ContextLedger) };
        ledger.select_candidate(key, revision, composition_id, candidate_id)
    }));
    match result {
        Ok(Ok(())) => FCITX_ENGINE_CORE_OK,
        Ok(Err(error)) => error_code(error),
        Err(_) => FCITX_ENGINE_CORE_STALE,
    }
}

/// Applies the end-of-result composition lifecycle and revision bump.
/// Writes the new `(composition_id, revision)` through the output pointers.
///
/// # Safety
/// `ledger` must be a valid live handle; `key` must be valid or null;
/// `out_composition_id`/`out_revision` must be writable or null (a null
/// output pointer fails closed without writing).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_ledger_end_result(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    has_content: i32,
    out_composition_id: *mut u64,
    out_revision: *mut u64,
) -> i32 {
    let Some(key) = from_c(key) else {
        return FCITX_ENGINE_CORE_STALE;
    };
    if ledger.is_null() || out_composition_id.is_null() || out_revision.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &mut *(ledger as *mut ContextLedger) };
        ledger.end_result(key, has_content != 0)
    }));
    match result {
        Ok((composition, revision)) => {
            // SAFETY: output pointers checked above.
            unsafe {
                *out_composition_id = composition;
                *out_revision = revision;
            }
            FCITX_ENGINE_CORE_OK
        }
        Err(_) => FCITX_ENGINE_CORE_STALE,
    }
}

#[cfg(test)]
#[path = "capi_tests.rs"]
mod capi_tests;

// ---------------------------------------------------------------------------
// E3 event-shape consolidation: unified handle_key_event
// ---------------------------------------------------------------------------

/// Flattened key-event facts for the unified decision
/// (`FcitxEngineKeyEventC` in `engine_core_ffi.h`).
#[repr(C)]
pub struct FcitxEngineKeyEventC {
    pub key_sym: u32,
    pub key_flags: u32,
    pub is_release: u8,
    pub hotkey_toggle: *const std::os::raw::c_char,
    pub hotkey_next: *const std::os::raw::c_char,
    pub surrounding_text_valid: u8,
    pub current_surrounding_valid: u8,
    pub has_request_im: u8,
    pub request_im_valid: u8,
    pub default_im_valid: u8,
    pub default_im_nonempty: u8,
    pub current_eq_request: u8,
    pub current_eq_default: u8,
    pub im_overridden: u8,
    pub has_candidates: u8,
    pub view: FcitxCandidateViewC,
    pub config: FcitxCandidateConfigC,
    pub has_override: u8,
    pub override_value: u32,
}

/// Unified decision output (`FcitxEngineKeyDecisionC`).
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FcitxEngineKeyDecisionC {
    pub surrounding_action: i32,
    pub surrounding_update: u8,
    pub im_selection: i32,
    pub im_switch: i32,
    pub candidate_consume: u8,
    pub candidate_action: i32,
    pub candidate_value: u32,
    pub clear_override: u8,
    pub forward_key: u8,
}

const FLAG_SHIFT: u32 = 1 << 0;
const FLAG_CONTROL: u32 = 1 << 1;
const FLAG_ALT: u32 = 1 << 2;

fn cstr_option(pointer: *const std::os::raw::c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: caller provides a valid NUL-terminated string.
    let text = unsafe { std::ffi::CStr::from_ptr(pointer) };
    Some(text.to_string_lossy().into_owned())
}

/// Unified Event→Action decision for a key request (E3 consolidation).
/// Writes `out_decision` and returns FCITX_ENGINE_CORE_OK (0); returns
/// FCITX_ENGINE_CORE_STALE (1) on null input (fail closed).
///
/// # Safety
/// `event`/`out_decision` must be valid or null; hotkey strings must be
/// NUL-terminated or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_handle_key_event(
    event: *const FcitxEngineKeyEventC,
    out_decision: *mut FcitxEngineKeyDecisionC,
) -> i32 {
    if event.is_null() || out_decision.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller provides valid pointers (checked above).
        let event = unsafe { &*event };
        crate::handle_key_event(&crate::EngineKeyEvent {
            key_sym: event.key_sym,
            ctrl: event.key_flags & FLAG_CONTROL != 0,
            shift: event.key_flags & FLAG_SHIFT != 0,
            alt: event.key_flags & FLAG_ALT != 0,
            plain_shortcut: event.key_flags & (FLAG_SHIFT | FLAG_CONTROL | FLAG_ALT) == 0,
            is_release: event.is_release != 0,
            hotkey_toggle: cstr_option(event.hotkey_toggle).as_deref(),
            hotkey_next: cstr_option(event.hotkey_next).as_deref(),
            surrounding_text_valid: event.surrounding_text_valid != 0,
            current_surrounding_valid: event.current_surrounding_valid != 0,
            has_request_im: event.has_request_im != 0,
            request_im_valid: event.request_im_valid != 0,
            default_im_valid: event.default_im_valid != 0,
            default_im_nonempty: event.default_im_nonempty != 0,
            current_eq_request: event.current_eq_request != 0,
            current_eq_default: event.current_eq_default != 0,
            im_overridden: event.im_overridden != 0,
            has_candidates: event.has_candidates != 0,
            candidate_count: event.view.count,
            candidate_list_size: event.view.list_size,
            candidate_cursor: event.view.cursor,
            candidate_bulk_cursor: event.view.bulk_cursor,
            candidate_has_bulk_cursor: event.view.has_bulk_cursor != 0,
            candidate_has_bulk: event.view.has_bulk != 0,
            candidate_pageable: event.view.pageable != 0,
            candidate_has_prev: event.view.has_prev != 0,
            candidate_has_next: event.view.has_next != 0,
            scroll_mode: event.config.scroll_mode != 0,
            vertical: event.config.vertical != 0,
            candidate_page_size: if event.config.candidate_page_size < 0 {
                None
            } else {
                Some(event.config.candidate_page_size)
            },
            has_override: event.has_override != 0,
            override_value: event.override_value,
        })
    }));
    match result {
        Ok(decision) => {
            let (candidate_consume, candidate_action, candidate_value) = match decision.candidate {
                Some(candidate) => (
                    if candidate.consume { 1 } else { 0 },
                    candidate_action_code(candidate.action),
                    candidate_action_value(candidate.action),
                ),
                None => (0, FCITX_ENGINE_CORE_CANDIDATE_ACTION_NONE, 0),
            };
            let im_switch = match decision.im_switch {
                Some(ImSwitchAction::Toggle) => FCITX_ENGINE_CORE_IM_ACTION_TOGGLE,
                Some(ImSwitchAction::Next) => FCITX_ENGINE_CORE_IM_ACTION_NEXT,
                None => 0,
            };
            let im_selection = match decision.im_selection {
                crate::InputMethodSelection::NoChange => FCITX_ENGINE_CORE_IM_SELECTION_NONE,
                crate::InputMethodSelection::SelectRequest => {
                    FCITX_ENGINE_CORE_IM_SELECTION_REQUEST
                }
                crate::InputMethodSelection::SelectDefault => {
                    FCITX_ENGINE_CORE_IM_SELECTION_DEFAULT
                }
            };
            let surrounding_action = match decision.surrounding.action {
                crate::SurroundingTextAction::Set => FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_SET,
                crate::SurroundingTextAction::Invalidate => {
                    FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_INVALIDATE
                }
            };
            let output = FcitxEngineKeyDecisionC {
                surrounding_action,
                surrounding_update: if decision.surrounding.update { 1 } else { 0 },
                im_selection,
                im_switch,
                candidate_consume,
                candidate_action,
                candidate_value,
                clear_override: if decision.clear_override { 1 } else { 0 },
                forward_key: if decision.forward_key { 1 } else { 0 },
            };
            // SAFETY: output pointer checked above.
            unsafe {
                *out_decision = output;
            }
            FCITX_ENGINE_CORE_OK
        }
        Err(_) => FCITX_ENGINE_CORE_STALE,
    }
}

// ---------------------------------------------------------------------------
// E3-3: surrounding-text and input-method-selection decisions
// ---------------------------------------------------------------------------

/// Decision output for the surrounding-text decision
/// (`FcitxSurroundingTextDecisionC`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxSurroundingTextDecisionC {
    pub action: i32,
    pub update: u8,
}

pub const FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_SET: i32 = 0;
pub const FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_INVALIDATE: i32 = 1;

/// Decides the surrounding-text action (E3-3). Writes `out_decision` and
/// returns FCITX_ENGINE_CORE_OK (0); returns FCITX_ENGINE_CORE_STALE (1) on
/// null output (fail closed).
///
/// # Safety
/// `out_decision` must be writable or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_decide_surrounding_text(
    request_valid: i32,
    current_valid: i32,
    out_decision: *mut FcitxSurroundingTextDecisionC,
) -> i32 {
    if out_decision.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(|| {
        crate::decide_surrounding_text(request_valid != 0, current_valid != 0)
    });
    match result {
        Ok(decision) => {
            let action = match decision.action {
                crate::SurroundingTextAction::Set => FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_SET,
                crate::SurroundingTextAction::Invalidate => {
                    FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_INVALIDATE
                }
            };
            let output = FcitxSurroundingTextDecisionC {
                action,
                update: if decision.update { 1 } else { 0 },
            };
            // SAFETY: output pointer checked above.
            unsafe {
                *out_decision = output;
            }
            FCITX_ENGINE_CORE_OK
        }
        Err(_) => FCITX_ENGINE_CORE_STALE,
    }
}

pub const FCITX_ENGINE_CORE_IM_SELECTION_NONE: i32 = 0;
pub const FCITX_ENGINE_CORE_IM_SELECTION_REQUEST: i32 = 1;
pub const FCITX_ENGINE_CORE_IM_SELECTION_DEFAULT: i32 = 2;

/// Decides the input-method selection (E3-3). Returns FCITX_ENGINE_CORE_OK
/// (0) and writes `out_selection` (NONE/REQUEST/DEFAULT); returns
/// FCITX_ENGINE_CORE_STALE (1) on null output (fail closed).
///
/// # Safety
/// `out_selection` must be writable or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_decide_input_method_selection(
    has_request_im: i32,
    request_im_valid: i32,
    default_im_valid: i32,
    default_im_nonempty: i32,
    current_eq_request: i32,
    current_eq_default: i32,
    overridden: i32,
    out_selection: *mut i32,
) -> i32 {
    if out_selection.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(|| {
        crate::decide_input_method_selection(
            has_request_im != 0,
            request_im_valid != 0,
            default_im_valid != 0,
            default_im_nonempty != 0,
            current_eq_request != 0,
            current_eq_default != 0,
            overridden != 0,
        )
    });
    match result {
        Ok(selection) => {
            let code = match selection {
                crate::InputMethodSelection::NoChange => FCITX_ENGINE_CORE_IM_SELECTION_NONE,
                crate::InputMethodSelection::SelectRequest => {
                    FCITX_ENGINE_CORE_IM_SELECTION_REQUEST
                }
                crate::InputMethodSelection::SelectDefault => {
                    FCITX_ENGINE_CORE_IM_SELECTION_DEFAULT
                }
            };
            // SAFETY: output pointer checked above.
            unsafe {
                *out_selection = code;
            }
            FCITX_ENGINE_CORE_OK
        }
        Err(_) => FCITX_ENGINE_CORE_STALE,
    }
}

// ---------------------------------------------------------------------------
// E3-2: Event → Action — candidate navigation decision
// ---------------------------------------------------------------------------

/// Candidate view facts flattened by the C++ adapter from the Fcitx candidate
/// list (`FcitxCandidateViewC` in `engine_core_ffi.h`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxCandidateViewC {
    pub count: i32,
    pub list_size: i32,
    pub cursor: i32,
    pub bulk_cursor: i32,
    pub has_bulk_cursor: u8,
    pub has_bulk: u8,
    pub pageable: u8,
    pub has_prev: u8,
    pub has_next: u8,
}

/// Candidate-navigation config facts (`FcitxCandidateConfigC`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxCandidateConfigC {
    pub scroll_mode: u8,
    pub vertical: u8,
    pub candidate_page_size: i32,
}

/// Decision output (`FcitxCandidateDecisionC`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxCandidateDecisionC {
    pub consume: u8,
    pub action: i32,
    pub value: u32,
}

pub const FCITX_ENGINE_CORE_CANDIDATE_ACTION_NONE: i32 = 0;
pub const FCITX_ENGINE_CORE_CANDIDATE_ACTION_CONSUME_ONLY: i32 = 1;
pub const FCITX_ENGINE_CORE_CANDIDATE_ACTION_SELECT_AND_CLEAR: i32 = 2;
pub const FCITX_ENGINE_CORE_CANDIDATE_ACTION_SET_OVERRIDE: i32 = 3;
pub const FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_NEXT_AND_SET_OVERRIDE: i32 = 4;
pub const FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_PREV_AND_SET_OVERRIDE: i32 = 5;

fn candidate_action_code(action: navigation::CandidateAction) -> i32 {
    match action {
        navigation::CandidateAction::None => FCITX_ENGINE_CORE_CANDIDATE_ACTION_NONE,
        navigation::CandidateAction::ConsumeOnly => FCITX_ENGINE_CORE_CANDIDATE_ACTION_CONSUME_ONLY,
        navigation::CandidateAction::SelectAndClear(_) => {
            FCITX_ENGINE_CORE_CANDIDATE_ACTION_SELECT_AND_CLEAR
        }
        navigation::CandidateAction::SetOverride(_) => {
            FCITX_ENGINE_CORE_CANDIDATE_ACTION_SET_OVERRIDE
        }
        navigation::CandidateAction::PageNextAndSetOverride(_) => {
            FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_NEXT_AND_SET_OVERRIDE
        }
        navigation::CandidateAction::PagePrevAndSetOverride(_) => {
            FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_PREV_AND_SET_OVERRIDE
        }
    }
}

fn candidate_action_value(action: navigation::CandidateAction) -> u32 {
    match action {
        navigation::CandidateAction::SelectAndClear(value)
        | navigation::CandidateAction::SetOverride(value)
        | navigation::CandidateAction::PageNextAndSetOverride(value)
        | navigation::CandidateAction::PagePrevAndSetOverride(value) => value,
        _ => 0,
    }
}

/// Decides the action for a non-release candidate-navigation key (E3-2).
/// Writes `out_decision` and returns FCITX_ENGINE_CORE_OK (0) on success;
/// returns FCITX_ENGINE_CORE_STALE (1) on null input (fail closed).
///
/// # Safety
/// `view`/`config`/`out_decision` must be valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_decide_candidate_action(
    key_sym: u32,
    plain_shortcut: i32,
    view: *const FcitxCandidateViewC,
    config: *const FcitxCandidateConfigC,
    has_override: i32,
    override_value: u32,
    out_decision: *mut FcitxCandidateDecisionC,
) -> i32 {
    if view.is_null() || config.is_null() || out_decision.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller provides valid pointers (checked above).
        let view = unsafe { *view };
        let config = unsafe { *config };
        navigation::decide_candidate_action(
            key_sym,
            plain_shortcut != 0,
            view.count,
            view.list_size,
            view.cursor,
            view.bulk_cursor,
            view.has_bulk_cursor != 0,
            view.has_bulk != 0,
            view.pageable != 0,
            view.has_prev != 0,
            view.has_next != 0,
            config.scroll_mode != 0,
            config.vertical != 0,
            if config.candidate_page_size < 0 {
                None
            } else {
                Some(config.candidate_page_size)
            },
            if has_override != 0 {
                Some(override_value)
            } else {
                None
            },
        )
    }));
    match result {
        Ok(decision) => {
            let output = FcitxCandidateDecisionC {
                consume: if decision.consume { 1 } else { 0 },
                action: candidate_action_code(decision.action),
                value: candidate_action_value(decision.action),
            };
            // SAFETY: output pointer checked above.
            unsafe {
                *out_decision = output;
            }
            FCITX_ENGINE_CORE_OK
        }
        Err(_) => FCITX_ENGINE_CORE_STALE,
    }
}

// ---------------------------------------------------------------------------
// E3: Event → Action — input-method switch decision
// ---------------------------------------------------------------------------

pub const FCITX_ENGINE_CORE_IM_ACTION_TOGGLE: i32 = 1;
pub const FCITX_ENGINE_CORE_IM_ACTION_NEXT: i32 = 2;

/// Decides whether a non-release key event triggers the configured
/// input-method switch hotkey. Returns 1 and writes `out_action` (TOGGLE or
/// NEXT) when a hotkey matches; 0 when nothing matches or on null output.
///
/// `hotkey_toggle`/`hotkey_next` are NUL-terminated UTF-8 strings or null
/// (null means "not configured").
///
/// # Safety
/// `hotkey_toggle`/`hotkey_next` must be valid NUL-terminated strings or
/// null; `out_action` must be writable or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_classify_input_method_switch(
    ctrl: i32,
    shift: i32,
    alt: i32,
    key_sym: u32,
    hotkey_toggle: *const std::os::raw::c_char,
    hotkey_next: *const std::os::raw::c_char,
    out_action: *mut i32,
) -> i32 {
    if out_action.is_null() {
        return 0;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let toggler = if hotkey_toggle.is_null() {
            None
        } else {
            // SAFETY: caller provides a valid NUL-terminated string.
            let text = unsafe { std::ffi::CStr::from_ptr(hotkey_toggle) };
            Some(text.to_str().unwrap_or(""))
        };
        let next = if hotkey_next.is_null() {
            None
        } else {
            // SAFETY: caller provides a valid NUL-terminated string.
            let text = unsafe { std::ffi::CStr::from_ptr(hotkey_next) };
            Some(text.to_str().unwrap_or(""))
        };
        classify_input_method_switch(ctrl != 0, shift != 0, alt != 0, key_sym, toggler, next)
    }));
    let Some(action) = result.unwrap_or(None) else {
        return 0;
    };
    let code = match action {
        ImSwitchAction::Toggle => FCITX_ENGINE_CORE_IM_ACTION_TOGGLE,
        ImSwitchAction::Next => FCITX_ENGINE_CORE_IM_ACTION_NEXT,
    };
    // SAFETY: output pointer checked above.
    unsafe {
        *out_action = code;
    }
    1
}

// ---------------------------------------------------------------------------
// Per-context product state (E2 extension: carets / popupAllowed /
// selectedOverride / inputMethodOverridden)
// ---------------------------------------------------------------------------

/// Stores the last-known caret rectangle for a context.
///
/// # Safety
/// `ledger` must be a valid live handle; `key`/`caret` must be valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_set_caret(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    caret: *const CaretRect,
) -> i32 {
    let Some(key) = from_c(key) else {
        return FCITX_ENGINE_CORE_STALE;
    };
    if ledger.is_null() || caret.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &mut *(ledger as *mut ContextLedger) };
        // SAFETY: caller provides a valid caret pointer (checked above).
        let caret = unsafe { *caret };
        ledger.set_caret(key, caret);
    }));
    if result.is_ok() {
        FCITX_ENGINE_CORE_OK
    } else {
        FCITX_ENGINE_CORE_STALE
    }
}

/// Returns the stored caret for a context. Returns 1 and writes `out_caret`
/// when present; 0 when absent or on null input (nothing written).
///
/// # Safety
/// `ledger` must be a valid live handle; `key`/`out_caret` must be valid or
/// null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_caret(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    out_caret: *mut CaretRect,
) -> i32 {
    let Some(key) = from_c(key) else {
        return 0;
    };
    if ledger.is_null() || out_caret.is_null() {
        return 0;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &*(ledger as *const ContextLedger) };
        ledger.caret(key)
    }));
    match result {
        Ok(Some(caret)) => {
            // SAFETY: output pointer checked above.
            unsafe {
                *out_caret = caret;
            }
            1
        }
        _ => 0,
    }
}

/// Stores the popup policy for a context.
///
/// # Safety
/// `ledger` must be a valid live handle; `key` must be valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_set_popup_allowed(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    allowed: i32,
) -> i32 {
    let Some(key) = from_c(key) else {
        return FCITX_ENGINE_CORE_STALE;
    };
    if ledger.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &mut *(ledger as *mut ContextLedger) };
        ledger.set_popup_allowed(key, allowed != 0);
    }));
    if result.is_ok() {
        FCITX_ENGINE_CORE_OK
    } else {
        FCITX_ENGINE_CORE_STALE
    }
}

/// Returns the stored popup policy for a context. Returns 1 and writes
/// `out_allowed` when present; 0 when absent or on null input.
///
/// # Safety
/// `ledger` must be a valid live handle; `key`/`out_allowed` must be valid or
/// null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_popup_allowed(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    out_allowed: *mut i32,
) -> i32 {
    let Some(key) = from_c(key) else {
        return 0;
    };
    if ledger.is_null() || out_allowed.is_null() {
        return 0;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &*(ledger as *const ContextLedger) };
        ledger.popup_allowed(key)
    }));
    match result {
        Ok(Some(allowed)) => {
            // SAFETY: output pointer checked above.
            unsafe {
                *out_allowed = if allowed { 1 } else { 0 };
            }
            1
        }
        _ => 0,
    }
}

/// Sets the candidate-highlight override for a context.
///
/// # Safety
/// `ledger` must be a valid live handle; `key` must be valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_set_selected_override(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    value: u32,
) -> i32 {
    let Some(key) = from_c(key) else {
        return FCITX_ENGINE_CORE_STALE;
    };
    if ledger.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &mut *(ledger as *mut ContextLedger) };
        ledger.set_selected_override(key, value);
    }));
    if result.is_ok() {
        FCITX_ENGINE_CORE_OK
    } else {
        FCITX_ENGINE_CORE_STALE
    }
}

/// Clears the candidate-highlight override for a context.
///
/// # Safety
/// `ledger` must be a valid live handle; `key` must be valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_clear_selected_override(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
) -> i32 {
    let Some(key) = from_c(key) else {
        return FCITX_ENGINE_CORE_STALE;
    };
    if ledger.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &mut *(ledger as *mut ContextLedger) };
        ledger.clear_selected_override(key);
    }));
    if result.is_ok() {
        FCITX_ENGINE_CORE_OK
    } else {
        FCITX_ENGINE_CORE_STALE
    }
}

/// Returns the candidate-highlight override for a context. Returns 1 and
/// writes `out_value` when set (a stored `Some(0)` is reported as set);
/// 0 when absent or on null input.
///
/// # Safety
/// `ledger` must be a valid live handle; `key`/`out_value` must be valid or
/// null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_selected_override(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    out_value: *mut u32,
) -> i32 {
    let Some(key) = from_c(key) else {
        return 0;
    };
    if ledger.is_null() || out_value.is_null() {
        return 0;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &*(ledger as *const ContextLedger) };
        ledger.selected_override(key)
    }));
    match result {
        Ok(Some(value)) => {
            // SAFETY: output pointer checked above.
            unsafe {
                *out_value = value;
            }
            1
        }
        _ => 0,
    }
}

/// Marks a context as input-method-overridden.
///
/// # Safety
/// `ledger` must be a valid live handle; `key` must be valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_set_input_method_overridden(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    overridden: i32,
) -> i32 {
    let Some(key) = from_c(key) else {
        return FCITX_ENGINE_CORE_STALE;
    };
    if ledger.is_null() {
        return FCITX_ENGINE_CORE_STALE;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &mut *(ledger as *mut ContextLedger) };
        ledger.set_input_method_overridden(key, overridden != 0);
    }));
    if result.is_ok() {
        FCITX_ENGINE_CORE_OK
    } else {
        FCITX_ENGINE_CORE_STALE
    }
}

/// Returns whether a context is marked input-method-overridden. Returns 1 and
/// writes `out_overridden` when the context has a stored marker; 0 when the
/// context is unknown or on null input (the C++ default is `false`).
///
/// # Safety
/// `ledger` must be a valid live handle; `key`/`out_overridden` must be valid
/// or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_engine_core_input_method_overridden(
    ledger: *mut c_void,
    key: *const FcitxEngineContextKeyC,
    out_overridden: *mut i32,
) -> i32 {
    let Some(key) = from_c(key) else {
        return 0;
    };
    if ledger.is_null() || out_overridden.is_null() {
        return 0;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller guarantees the handle came from `ledger_new`.
        let ledger = unsafe { &*(ledger as *const ContextLedger) };
        ledger.input_method_overridden(key)
    }));
    match result {
        Ok(true) => {
            // SAFETY: output pointer checked above.
            unsafe {
                *out_overridden = 1;
            }
            1
        }
        Ok(false) | Err(_) => 0,
    }
}
