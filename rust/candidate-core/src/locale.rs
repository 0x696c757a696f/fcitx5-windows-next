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
