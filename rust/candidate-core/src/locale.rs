//! Content-locale validation and the DirectWrite locale fallback.

#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

pub(crate) const MAX_CONTENT_LOCALE_UTF8: usize = 35;

pub(crate) const LOCALE_NAME_MAX_LENGTH: usize = 85;

pub(crate) const DEFAULT_DWRITE_LOCALE: &[u16] = &[
    b'e' as u16,
    b'n' as u16,
    b'-' as u16,
    b'U' as u16,
    b'S' as u16,
];

#[must_use]
pub fn default_dwrite_locale() -> Vec<u16> {
    let mut locale = [0_u16; LOCALE_NAME_MAX_LENGTH];
    // SAFETY: `locale` is writable storage for exactly the documented buffer length.
    let length = unsafe { GetUserDefaultLocaleName(locale.as_mut_ptr(), locale.len() as i32) };
    if length > 1 && length as usize <= locale.len() {
        return locale[..length as usize - 1].to_vec();
    }
    DEFAULT_DWRITE_LOCALE.to_vec()
}

#[must_use]
pub fn content_locale_valid(locale: &[u8]) -> bool {
    if locale.is_empty() || locale.len() > MAX_CONTENT_LOCALE_UTF8 {
        return false;
    }
    let mut has_letter = false;
    for character in locale {
        let letter = character.is_ascii_alphabetic();
        has_letter |= letter;
        if !letter && !character.is_ascii_digit() && *character != b'-' {
            return false;
        }
    }
    has_letter
}

#[must_use]
pub fn content_locale_or_default(locale: &[u8]) -> Vec<u16> {
    if !content_locale_valid(locale) {
        return default_dwrite_locale();
    }
    locale
        .iter()
        .map(|character| u16::from(*character))
        .collect()
}

#[no_mangle]
/// # Safety
///
/// `locale_out` may be null for size queries or point to writable UTF-16
/// storage for `locale_capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_candidate_default_dwrite_locale_utf16(
    locale_out: *mut u16,
    locale_capacity: usize,
) -> usize {
    write_wide_units(&default_dwrite_locale(), locale_out, locale_capacity)
}

#[no_mangle]
/// # Safety
///
/// `locale` must either point to `locale.len` readable bytes or be null when
/// `locale.len` is zero. The pointer is not retained.
pub unsafe extern "C" fn fcitx5_candidate_content_locale_valid_utf8(
    locale: Fcitx5CandidateUtf8,
) -> u8 {
    // SAFETY: the ABI keeps the locale byte span readable for this call and it is not retained.
    let Some(locale) = (unsafe { bytes_from_ffi(locale) }) else {
        return 0;
    };
    content_locale_valid(locale) as u8
}

#[no_mangle]
/// # Safety
///
/// `locale` must either point to `locale.len` readable bytes or be null when
/// `locale.len` is zero. `locale_out` may be null for size queries or point to
/// writable UTF-16 storage for `locale_capacity` code units. No pointer is
/// retained.
pub unsafe extern "C" fn fcitx5_candidate_content_locale_or_default_utf16(
    locale: Fcitx5CandidateUtf8,
    locale_out: *mut u16,
    locale_capacity: usize,
) -> usize {
    // SAFETY: the ABI keeps the locale byte span readable for this call and it is not retained.
    let locale = (unsafe { bytes_from_ffi(locale) }).unwrap_or_default();
    write_wide_units(
        &content_locale_or_default(locale),
        locale_out,
        locale_capacity,
    )
}

#[no_mangle]
/// # Safety
///
/// `locale` must either point to `locale.len` readable bytes or be null when
/// `locale.len` is zero. The pointer is not retained.
pub unsafe extern "C" fn fcitx5_candidate_locale_prefers_compact_horizontal_utf8(
    locale: Fcitx5CandidateUtf8,
) -> u8 {
    // SAFETY: the ABI keeps the locale byte span readable for this call and it is not retained.
    let Some(locale) = (unsafe { bytes_from_ffi(locale) }) else {
        return 0;
    };
    let Ok(locale) = std::str::from_utf8(locale) else {
        return 0;
    };
    locale_prefers_compact_horizontal(locale) as u8
}
