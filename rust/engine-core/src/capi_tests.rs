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
