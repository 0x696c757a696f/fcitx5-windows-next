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

use crate::{CaretRect, ContextKey, ContextLedger, LedgerError};

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
) -> u8 {
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
) -> u8 {
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
) -> u8 {
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
) -> u8 {
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
