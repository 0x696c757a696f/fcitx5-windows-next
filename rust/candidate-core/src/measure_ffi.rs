//! C ABI for host-side text measurement (081C slice).
//!
//! This module is the only unsafe boundary for the measurement engine; the
//! engine itself is safe Rust in `renderer::MeasureEngine`.

use core::ffi::c_void;

use crate::renderer::{Fcitx5CandidateMeasureSize, MeasureEngine};

/// Creates the shipping measurement engine.
#[no_mangle]
pub extern "C" fn fcitx5_candidate_measure_create() -> *mut c_void {
    Box::into_raw(Box::new(MeasureEngine::new())).cast()
}

/// # Safety
///
/// `engine` must be null or a pointer returned by
/// `fcitx5_candidate_measure_create` that has not already been destroyed.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_measure_destroy(engine: *mut c_void) {
    if !engine.is_null() {
        // SAFETY: caller guarantees this is the unique allocation from create.
        unsafe { drop(Box::from_raw(engine.cast::<MeasureEngine>())) };
    }
}

/// # Safety
///
/// `engine` must be a valid measure-engine pointer, `text` must reference
/// `text_len` readable bytes, and `out` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_measure_text_utf8(
    engine: *mut c_void,
    text: *const u8,
    text_len: usize,
    font_size: f32,
    dpi_scale: f32,
    out: *mut Fcitx5CandidateMeasureSize,
) -> u8 {
    if engine.is_null() || out.is_null() || (text_len > 0 && text.is_null()) {
        return 0;
    }
    // SAFETY: the C ABI contract requires this byte slice to be valid.
    let bytes = unsafe { std::slice::from_raw_parts(text, text_len) };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return 0;
    };
    // SAFETY: caller guarantees the engine allocation is valid for this call.
    let engine = unsafe { &mut *engine.cast::<MeasureEngine>() };
    let (width, height) = engine.measure(text, font_size, dpi_scale);
    // SAFETY: non-null checked above; caller provides writable output storage.
    unsafe {
        *out = Fcitx5CandidateMeasureSize { width, height };
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_text_utf8_rejects_invalid_utf8_and_null_outputs() {
        let engine = fcitx5_candidate_measure_create();
        assert!(!engine.is_null());
        let mut size = Fcitx5CandidateMeasureSize::default();
        // SAFETY: engine came from create; valid buffers are supplied.
        let ok = unsafe {
            fcitx5_candidate_measure_text_utf8(engine, "abc".as_ptr(), 3, 14.0, 1.0, &mut size)
        };
        assert_eq!(ok, 1);
        assert!(size.width > 0.0 && size.height > 0.0);
        // SAFETY: single valid bytes slice for a rejected call.
        let bad = unsafe {
            fcitx5_candidate_measure_text_utf8(
                engine,
                [0xFF, 0xFE].as_ptr(),
                2,
                14.0,
                1.0,
                &mut size,
            )
        };
        assert_eq!(bad, 0, "invalid UTF-8 must fail, not replace text");
        // SAFETY: null out must be rejected without dereferencing.
        let null_out = unsafe {
            fcitx5_candidate_measure_text_utf8(
                engine,
                "abc".as_ptr(),
                3,
                14.0,
                1.0,
                core::ptr::null_mut(),
            )
        };
        assert_eq!(null_out, 0);
        // SAFETY: engine is the unique allocation from create.
        unsafe { fcitx5_candidate_measure_destroy(engine) };
    }
}
