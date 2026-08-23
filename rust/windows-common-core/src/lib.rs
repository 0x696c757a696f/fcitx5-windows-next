#![deny(unsafe_op_in_unsafe_fn)]

const VERSION_FALLBACK: &str = env!("CARGO_PKG_VERSION");
const RELEASE_CHANNEL_FALLBACK: &str = "stable";
const ENDPOINT_MAX_WIDE_UNITS: usize = 32_768;

fn version() -> &'static str {
    option_env!("FCITX_WINDOWS_VERSION").unwrap_or(VERSION_FALLBACK)
}

fn release_channel() -> &'static str {
    option_env!("FCITX_RELEASE_CHANNEL_NAME").unwrap_or(RELEASE_CHANNEL_FALLBACK)
}

fn pipe_prefix() -> &'static str {
    match release_channel() {
        "beta" => "Fcitx5WindowsNext.beta.v2",
        "nightly" => "Fcitx5WindowsNext.nightly.v2",
        _ => "Fcitx5WindowsNext.stable.v2",
    }
}

fn local_object_prefix() -> &'static str {
    match release_channel() {
        "beta" => "Fcitx5WindowsNext.Beta",
        "nightly" => "Fcitx5WindowsNext.Nightly",
        _ => "Fcitx5WindowsNext.Stable",
    }
}

fn valid_channel(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn valid_generation(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

fn wide_string_from_raw(text: *const u16, len: usize) -> Option<String> {
    if text.is_null() {
        return (len == 0).then(String::new);
    }
    // SAFETY: The caller supplies exactly `len` readable UTF-16 code units. The
    // slice is copied into an owned String and not retained.
    String::from_utf16(unsafe { std::slice::from_raw_parts(text, len) }).ok()
}

fn write_wide_string(value: &str, out: *mut u16, capacity: usize) -> usize {
    let wide: Vec<u16> = value.encode_utf16().collect();
    if !out.is_null() && capacity != 0 {
        let count = wide.len().min(capacity);
        if count != 0 {
            // SAFETY: The caller supplied writable storage for `capacity` u16
            // values. We copy at most that many initialized code units.
            unsafe { std::ptr::copy_nonoverlapping(wide.as_ptr(), out, count) };
        }
    }
    wide.len()
}

fn local_name(
    pipe: bool,
    user_sid: &str,
    session_id: u32,
    generation: &str,
    channel: &str,
    test_namespace: &str,
) -> Option<String> {
    if user_sid.is_empty()
        || session_id == 0
        || !valid_generation(generation)
        || !valid_channel(channel)
        || (!test_namespace.is_empty() && !valid_channel(test_namespace))
    {
        return None;
    }
    let namespace_part = if test_namespace.is_empty() {
        String::new()
    } else {
        format!(".Test.{test_namespace}")
    };
    let result = if pipe {
        format!(
            r"\\.\pipe\{}.{user_sid}.Session.{session_id}.Generation.{generation}{namespace_part}.{channel}",
            pipe_prefix()
        )
    } else {
        format!(
            "Local\\{}.{user_sid}.Session.{session_id}.Generation.{generation}{namespace_part}.{channel}",
            local_object_prefix()
        )
    };
    (result.encode_utf16().count() < ENDPOINT_MAX_WIDE_UNITS).then_some(result)
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_version() -> *const u8 {
    version().as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_version_len() -> usize {
    version().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_release_channel() -> *const u8 {
    release_channel().as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_release_channel_len() -> usize {
    release_channel().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_architecture() -> u32 {
    if cfg!(target_pointer_width = "64") {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
/// # Safety
///
/// UTF-16 input pointers must be null only when their corresponding length is
/// zero, or point to valid UTF-16 buffers with exactly the provided lengths.
/// `output` may be null for size queries or writable UTF-16 storage for
/// `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_local_name_utf16(
    kind: u32,
    user_sid: *const u16,
    user_sid_len: usize,
    session_id: u32,
    generation: *const u16,
    generation_len: usize,
    channel: *const u16,
    channel_len: usize,
    test_namespace: *const u16,
    test_namespace_len: usize,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(user_sid) = wide_string_from_raw(user_sid, user_sid_len) else {
        return 0;
    };
    let Some(generation) = wide_string_from_raw(generation, generation_len) else {
        return 0;
    };
    let Some(channel) = wide_string_from_raw(channel, channel_len) else {
        return 0;
    };
    let Some(test_namespace) = wide_string_from_raw(test_namespace, test_namespace_len) else {
        return 0;
    };
    let Some(name) = local_name(
        kind == 0,
        &user_sid,
        session_id,
        &generation,
        &channel,
        &test_namespace,
    ) else {
        return 0;
    };
    write_wide_string(&name, output, capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_and_release_channel_are_stable_static_strings() {
        assert!(!version().is_empty());
        assert_eq!(version().as_ptr(), fcitx5_windows_common_version());
        assert_eq!(version().len(), fcitx5_windows_common_version_len());
        assert!(!release_channel().is_empty());
        assert_eq!(
            release_channel().as_ptr(),
            fcitx5_windows_common_release_channel()
        );
        assert_eq!(
            release_channel().len(),
            fcitx5_windows_common_release_channel_len()
        );
    }

    #[test]
    fn architecture_matches_target_pointer_width() {
        let expected = if cfg!(target_pointer_width = "64") {
            1
        } else {
            0
        };
        assert_eq!(fcitx5_windows_common_architecture(), expected);
    }

    #[test]
    fn local_endpoint_and_object_names_match_cpp_contract() {
        let pipe = local_name(true, "S-1-5-21-test", 7, "00000042", "engine", "")
            .expect("valid pipe name");
        assert_eq!(
            pipe,
            r"\\.\pipe\Fcitx5WindowsNext.stable.v2.S-1-5-21-test.Session.7.Generation.00000042.engine"
        );
        let namespaced = local_name(false, "S-1-5-21-test", 7, "00000042", "engine", "abc-1")
            .expect("valid object name");
        assert_eq!(
            namespaced,
            "Local\\Fcitx5WindowsNext.Stable.S-1-5-21-test.Session.7.Generation.00000042.Test.abc-1.engine"
        );
        assert!(local_name(true, "", 7, "00000042", "engine", "").is_none());
        assert!(local_name(true, "S", 0, "00000042", "engine", "").is_none());
        assert!(local_name(true, "S", 7, "../bad", "engine", "").is_none());
        assert!(local_name(true, "S", 7, "00000042", "Engine", "").is_none());
        assert!(local_name(true, "S", 7, "00000042", "engine", "../bad").is_none());
    }
}
