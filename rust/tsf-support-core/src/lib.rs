#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

const RECOVERY_DIRECTORY: &str = "recovery";
const DISABLED_MARKER: &str = "tsf-activation-disabled.v1";
const ATTEMPT_PREFIX: &str = "tsf-activation-attempt.";
const ATTEMPT_SUFFIX: &str = ".v1";
const RELEASE_DATA_DIRECTORY_FALLBACK: &str = "Fcitx5";

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Fcitx5TsfActivationAttemptAbi {
    pub fail_open: u8,
    pub active: u8,
    pub reason_len: usize,
    pub attempt_path_len: usize,
}

fn release_data_directory() -> &'static str {
    option_env!("FCITX_RELEASE_DATA_DIRECTORY").unwrap_or(RELEASE_DATA_DIRECTORY_FALLBACK)
}

fn recovery_directory(data_root: &Path) -> PathBuf {
    data_root.join(RECOVERY_DIRECTORY)
}

fn marker_path(data_root: &Path) -> PathBuf {
    recovery_directory(data_root).join(DISABLED_MARKER)
}

fn sanitize_reason(reason: &str) -> String {
    let mut output = String::with_capacity(reason.len().min(160));
    for value in reason.bytes() {
        if output.len() >= 160 {
            break;
        }
        if value < 0x20 || value == b'=' {
            output.push('_');
        } else {
            output.push(value as char);
        }
    }
    if output.is_empty() {
        "unspecified".to_owned()
    } else {
        output
    }
}

fn valid_test_namespace(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn test_data_root() -> Option<PathBuf> {
    let namespace = std::env::var("FCITX5_TEST_NAMESPACE").ok()?;
    if !valid_test_namespace(&namespace) {
        return None;
    }
    let root = PathBuf::from(std::env::var_os("FCITX5_TEST_DATA_ROOT")?);
    root.is_absolute().then_some(root)
}

fn default_data_root() -> Option<PathBuf> {
    if let Some(root) = test_data_root() {
        return Some(root);
    }
    let local_app_data = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    Some(local_app_data.join(release_data_directory()))
}

fn path_from_wide(path: *const u16, len: usize) -> Option<PathBuf> {
    if path.is_null() {
        return (len == 0).then(PathBuf::new);
    }
    // SAFETY: The C++ adapter passes a valid UTF-16 buffer with exactly `len`
    // elements for the duration of this call.
    let slice = unsafe { std::slice::from_raw_parts(path, len) };
    Some(PathBuf::from(OsString::from_wide(slice)))
}

fn utf8_from_raw(text: *const u8, len: usize) -> String {
    if text.is_null() || len == 0 {
        return String::new();
    }
    // SAFETY: The C++ adapter passes a valid byte buffer with exactly `len`
    // elements for the duration of this call.
    let slice = unsafe { std::slice::from_raw_parts(text, len) };
    String::from_utf8_lossy(slice).into_owned()
}

fn write_utf16_to_buffer(value: &Path, out: *mut u16, capacity: usize) -> usize {
    let wide: Vec<u16> = value.as_os_str().encode_wide().collect();
    if !out.is_null() && capacity != 0 {
        let count = wide.len().min(capacity);
        // SAFETY: The caller supplied writable storage for `capacity` u16
        // values. We copy at most that many initialized elements.
        unsafe { std::ptr::copy_nonoverlapping(wide.as_ptr(), out, count) };
    }
    wide.len()
}

fn write_utf8_to_buffer(value: &str, out: *mut u8, capacity: usize) -> usize {
    let bytes = value.as_bytes();
    if !out.is_null() && capacity != 0 {
        let count = bytes.len().min(capacity);
        // SAFETY: The caller supplied writable storage for `capacity` bytes. We
        // copy at most that many initialized elements.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, count) };
    }
    bytes.len()
}

fn atomic_write_utf8(destination: &Path, text: &str) -> bool {
    let Some(parent) = destination.parent() else {
        return false;
    };
    if fs::create_dir_all(parent).is_err() {
        return false;
    }
    let temporary =
        destination.with_extension(format!("tmp.{}.{}", std::process::id(), monotonic_millis()));
    let write_result = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .and_then(|mut file| {
            file.write_all(text.as_bytes())?;
            file.sync_all()
        });
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
        return false;
    }
    if !replace_file(&temporary, destination) {
        let _ = fs::remove_file(&temporary);
        return false;
    }
    true
}

fn wide_nul(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}

fn replace_file(source: &Path, destination: &Path) -> bool {
    let source = wide_nul(source);
    let destination = wide_nul(destination);
    // SAFETY: Both path buffers are NUL-terminated and live for the duration of
    // the call. MOVEFILE_REPLACE_EXISTING preserves the old C++ guard behavior.
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .is_ok()
}

fn read_reason(marker: &Path) -> String {
    let Ok(text) = fs::read_to_string(marker) else {
        return String::new();
    };
    let Some(position) = text.find("reason=") else {
        return String::new();
    };
    let start = position + "reason=".len();
    let end = text[start..]
        .find('\n')
        .map(|offset| start + offset)
        .unwrap_or(text.len());
    text[start..end].to_owned()
}

fn is_attempt_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.len() > ATTEMPT_PREFIX.len() + ATTEMPT_SUFFIX.len()
        && name.starts_with(ATTEMPT_PREFIX)
        && name.ends_with(ATTEMPT_SUFFIX)
}

fn stale_attempt_exists(directory: &Path, threshold: Duration) -> bool {
    let Ok(entries) = fs::read_dir(directory) else {
        return false;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_attempt_file(&path) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if now
            .duration_since(modified)
            .is_ok_and(|age| age > threshold)
        {
            return true;
        }
    }
    false
}

fn remove_attempt_files(directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if is_attempt_file(&path) {
            let _ = fs::remove_file(path);
        }
    }
}

fn monotonic_millis() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn disable_guard(data_root: &Path, reason: &str) -> bool {
    let sanitized = sanitize_reason(reason);
    atomic_write_utf8(
        &marker_path(data_root),
        &format!("format_version=1\nreason={sanitized}\n"),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_tsf_activation_guard_default_data_root(
    out: *mut u16,
    capacity: usize,
) -> usize {
    default_data_root()
        .map(|path| write_utf16_to_buffer(&path, out, capacity))
        .unwrap_or_default()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_tsf_activation_guard_marker_path(
    data_root: *const u16,
    data_root_len: usize,
    out: *mut u16,
    capacity: usize,
) -> usize {
    path_from_wide(data_root, data_root_len)
        .map(|root| write_utf16_to_buffer(&marker_path(&root), out, capacity))
        .unwrap_or_default()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_tsf_activation_guard_status(
    data_root: *const u16,
    data_root_len: usize,
    reason_out: *mut u8,
    reason_capacity: usize,
    reason_len: *mut usize,
) -> u8 {
    let Some(root) = path_from_wide(data_root, data_root_len) else {
        if !reason_len.is_null() {
            // SAFETY: reason_len is an optional writable out pointer.
            unsafe { reason_len.write(0) };
        }
        return 0;
    };
    let marker = marker_path(&root);
    let disabled = marker.is_file();
    let reason = if disabled {
        read_reason(&marker)
    } else {
        String::new()
    };
    let written_len = write_utf8_to_buffer(&reason, reason_out, reason_capacity);
    if !reason_len.is_null() {
        // SAFETY: reason_len is an optional writable out pointer.
        unsafe { reason_len.write(written_len) };
    }
    u8::from(disabled)
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_tsf_activation_guard_disable(
    data_root: *const u16,
    data_root_len: usize,
    reason: *const u8,
    reason_len: usize,
) -> u8 {
    let Some(root) = path_from_wide(data_root, data_root_len) else {
        return 0;
    };
    let reason = utf8_from_raw(reason, reason_len);
    u8::from(disable_guard(&root, &reason))
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_tsf_activation_guard_clear(
    data_root: *const u16,
    data_root_len: usize,
) -> u8 {
    let Some(root) = path_from_wide(data_root, data_root_len) else {
        return 0;
    };
    let directory = recovery_directory(&root);
    let _ = fs::remove_file(marker_path(&root));
    remove_attempt_files(&directory);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_tsf_activation_attempt_begin(
    data_root: *const u16,
    data_root_len: usize,
    stale_threshold_seconds: u64,
    reason_out: *mut u8,
    reason_capacity: usize,
    attempt_path_out: *mut u16,
    attempt_path_capacity: usize,
) -> Fcitx5TsfActivationAttemptAbi {
    let Some(root) = path_from_wide(data_root, data_root_len) else {
        return Fcitx5TsfActivationAttemptAbi::default();
    };
    let marker = marker_path(&root);
    if marker.is_file() {
        let reason = read_reason(&marker);
        let reason = if reason.is_empty() {
            "disabled_marker".to_owned()
        } else {
            reason
        };
        return Fcitx5TsfActivationAttemptAbi {
            fail_open: 1,
            active: 0,
            reason_len: write_utf8_to_buffer(&reason, reason_out, reason_capacity),
            attempt_path_len: 0,
        };
    }
    let directory = recovery_directory(&root);
    if stale_attempt_exists(&directory, Duration::from_secs(stale_threshold_seconds)) {
        let reason = "previous_activation_did_not_finish";
        let _ = disable_guard(&root, reason);
        remove_attempt_files(&directory);
        return Fcitx5TsfActivationAttemptAbi {
            fail_open: 1,
            active: 0,
            reason_len: write_utf8_to_buffer(reason, reason_out, reason_capacity),
            attempt_path_len: 0,
        };
    }
    if fs::create_dir_all(&directory).is_err() {
        return Fcitx5TsfActivationAttemptAbi::default();
    }
    let attempt_path = directory.join(format!(
        "{ATTEMPT_PREFIX}{}.{}{ATTEMPT_SUFFIX}",
        std::process::id(),
        monotonic_millis()
    ));
    if !atomic_write_utf8(&attempt_path, "format_version=1\nstate=activating\n") {
        return Fcitx5TsfActivationAttemptAbi::default();
    }
    Fcitx5TsfActivationAttemptAbi {
        fail_open: 0,
        active: 1,
        reason_len: 0,
        attempt_path_len: write_utf16_to_buffer(
            &attempt_path,
            attempt_path_out,
            attempt_path_capacity,
        ),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_tsf_activation_attempt_finish(
    attempt_path: *const u16,
    attempt_path_len: usize,
) {
    if let Some(path) = path_from_wide(attempt_path, attempt_path_len) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "fcitx5-tsf-support-core-test-{}",
            std::process::id()
        ))
    }

    #[test]
    fn guard_disable_clear_and_stale_attempt_policy_roundtrip() {
        let root = temp_root();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        assert!(!marker_path(&root).is_file());
        assert!(disable_guard(&root, "manual=recovery\n"));
        assert!(marker_path(&root).is_file());
        assert_eq!(read_reason(&marker_path(&root)), "manual_recovery_");
        assert!(disable_guard(&root, "second"));
        assert_eq!(read_reason(&marker_path(&root)), "second");

        let root_wide: Vec<u16> = root.as_os_str().encode_wide().collect();
        assert_eq!(
            fcitx5_tsf_activation_guard_clear(root_wide.as_ptr(), root_wide.len()),
            1
        );
        assert!(!marker_path(&root).exists());

        let directory = recovery_directory(&root);
        fs::create_dir_all(&directory).unwrap();
        let stale = directory.join("tsf-activation-attempt.stale.v1");
        fs::write(&stale, "format_version=1\nstate=activating\n").unwrap();
        assert!(!stale_attempt_exists(&directory, Duration::from_secs(120)));
        assert!(stale_attempt_exists(&directory, Duration::from_secs(0)));
        let _ = fs::remove_dir_all(root);
    }
}
