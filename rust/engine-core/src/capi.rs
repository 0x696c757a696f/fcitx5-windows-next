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

use crate::{ContextKey, ContextLedger, LedgerError};

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
