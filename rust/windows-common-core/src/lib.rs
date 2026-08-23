#![deny(unsafe_op_in_unsafe_fn)]

use std::env;
use std::ffi::c_void;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

const VERSION_FALLBACK: &str = env!("CARGO_PKG_VERSION");
const RELEASE_CHANNEL_FALLBACK: &str = "stable";
const ENDPOINT_MAX_WIDE_UNITS: usize = 32_768;
const SMALL_TEXT_FILE_MAX_BYTES: u64 = 64 * 1024;

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
    write_wide_units(&wide, out, capacity)
}

fn write_wide_path(value: &Path, out: *mut u16, capacity: usize) -> usize {
    let wide: Vec<u16> = value.as_os_str().encode_wide().collect();
    write_wide_units(&wide, out, capacity)
}

fn write_wide_units(wide: &[u16], out: *mut u16, capacity: usize) -> usize {
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

fn path_from_raw(path: *const u16, len: usize) -> Option<PathBuf> {
    let value = wide_string_from_raw(path, len)?;
    (!value.is_empty()).then(|| PathBuf::from(value))
}

fn environment_generation() -> Option<String> {
    env::var("FCITX5_RELEASE_GENERATION")
        .ok()
        .filter(|value| valid_generation(value))
}

fn read_small_text_file(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > SMALL_TEXT_FILE_MAX_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn parse_plain_generation(bytes: &str) -> Option<String> {
    let candidate = bytes.trim_matches(['\n', '\r', ' ', '\t']);
    (candidate.is_ascii() && valid_generation(candidate)).then(|| candidate.to_owned())
}

fn parse_current_generation(json: &str) -> Option<String> {
    let key_position = json.find("\"current_generation\"")?;
    let colon = json[key_position..].find(':')? + key_position;
    let mut quote = colon + 1;
    while let Some(character) = json.as_bytes().get(quote) {
        if !matches!(character, b' ' | b'\t' | b'\r' | b'\n') {
            break;
        }
        quote += 1;
    }
    if json.as_bytes().get(quote) != Some(&b'"') {
        return None;
    }
    let begin = quote + 1;
    let end = json[begin..].find('"')? + begin;
    if end == begin {
        return None;
    }
    let candidate = &json[begin..end];
    (candidate.is_ascii() && valid_generation(candidate)).then(|| candidate.to_owned())
}

fn leaf_lower(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn install_root_for_module(module_path: &Path) -> Option<PathBuf> {
    let directory = module_path.parent()?;
    let parent_name = leaf_lower(directory);
    let grand_parent = directory.parent().unwrap_or_else(|| Path::new(""));
    let grand_parent_name = leaf_lower(grand_parent);
    if matches!(parent_name.as_str(), "x64" | "x86") && grand_parent_name == "tsf" {
        return directory.parent()?.parent().map(Path::to_path_buf);
    }
    if parent_name == "bin"
        && directory
            .parent()
            .and_then(|path| path.file_name())
            .and_then(|value| value.to_str())
            .is_some_and(valid_generation)
        && directory
            .parent()
            .and_then(Path::parent)
            .is_some_and(|path| leaf_lower(path) == "runtime")
    {
        return directory
            .parent()?
            .parent()?
            .parent()
            .map(Path::to_path_buf);
    }
    if matches!(parent_name.as_str(), "bin" | "management") {
        return directory.parent().map(Path::to_path_buf);
    }
    if directory
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(valid_generation)
        && grand_parent_name == "runtime"
    {
        return directory.parent()?.parent().map(Path::to_path_buf);
    }
    None
}

fn runtime_generation_for_runtime_module(module_path: &Path) -> Option<String> {
    let directory = module_path.parent()?;
    if leaf_lower(directory) == "bin"
        && directory
            .parent()
            .and_then(|path| path.file_name())
            .and_then(|value| value.to_str())
            .is_some_and(valid_generation)
        && directory
            .parent()
            .and_then(Path::parent)
            .is_some_and(|path| leaf_lower(path) == "runtime")
    {
        return directory
            .parent()?
            .file_name()
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned);
    }
    if directory
        .parent()
        .is_none_or(|path| leaf_lower(path) != "runtime")
    {
        return None;
    }
    let generation = directory.file_name()?.to_str()?;
    valid_generation(generation).then(|| generation.to_owned())
}

fn current_runtime_generation_from_install_root(root: &Path) -> Option<String> {
    if let Some(generation) = environment_generation() {
        return Some(generation);
    }
    if root.as_os_str().is_empty() {
        return None;
    }
    parse_current_generation(&read_small_text_file(&root.join("current.json"))?)
}

fn current_runtime_generation_for_module(module_path: &Path) -> Option<String> {
    if let Some(generation) = environment_generation() {
        return Some(generation);
    }
    if module_path.as_os_str().is_empty() {
        return None;
    }
    if let Some(generation) = runtime_generation_for_runtime_module(module_path) {
        return Some(generation);
    }
    if let Some(generation) = module_path
        .parent()
        .and_then(|directory| read_small_text_file(&directory.join("fcitx5-tsf.generation")))
        .and_then(|bytes| parse_plain_generation(&bytes))
    {
        return Some(generation);
    }
    let root = install_root_for_module(module_path)?;
    current_runtime_generation_from_install_root(&root)
}

fn portable_data_root_for_module(module_path: &Path) -> Option<PathBuf> {
    if module_path.as_os_str().is_empty() {
        return None;
    }
    if let Some(root) = install_root_for_module(module_path) {
        if root.join("portable.flag").exists() {
            return Some(root.join("data"));
        }
    }
    let directory = module_path.parent()?;
    if directory.join("portable.flag").exists() {
        return Some(directory.join("data"));
    }
    let parent = directory.parent()?;
    parent
        .join("portable.flag")
        .exists()
        .then(|| parent.join("data"))
}

fn may_launch_user_engine(
    service_account: bool,
    session_id: u32,
    secure_desktop: bool,
    user_sid: &str,
) -> bool {
    !service_account && session_id != 0 && !secure_desktop && !user_sid.is_empty()
}

fn pipe_security_sddl(service_account: bool, session_id: u32, user_sid: &str) -> Option<String> {
    if service_account || session_id == 0 || user_sid.is_empty() {
        return None;
    }
    Some(format!(
        "D:P(A;;GA;;;SY)(A;;GA;;;{user_sid})S:(ML;;NW;;;ME)"
    ))
}

fn pipe_security_descriptor(
    service_account: bool,
    session_id: u32,
    user_sid: &str,
) -> Option<*mut c_void> {
    let sddl = pipe_security_sddl(service_account, session_id, user_sid)?;
    let mut wide: Vec<u16> = sddl.encode_utf16().collect();
    wide.push(0);
    let mut descriptor: *mut c_void = std::ptr::null_mut();
    // SAFETY: `wide` is NUL-terminated UTF-16 and `descriptor` is a valid out
    // pointer. On success, Windows returns a LocalAlloc-owned descriptor that
    // the C++ RAII wrapper releases with LocalFree.
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    };
    (ok != 0 && !descriptor.is_null()).then_some(descriptor)
}

const SDDL_REVISION_1: u32 = 1;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        string_security_descriptor: *const u16,
        string_sddl_revision: u32,
        security_descriptor: *mut *mut c_void,
        security_descriptor_size: *mut u32,
    ) -> i32;
}

fn same_principal_and_session(
    peer_session_id: u32,
    peer_service_account: bool,
    peer_user_sid: &str,
    current_session_id: u32,
    current_user_sid: &str,
) -> bool {
    peer_session_id == current_session_id
        && !peer_service_account
        && peer_user_sid.eq_ignore_ascii_case(current_user_sid)
        && !peer_user_sid.is_empty()
}

fn peer_development_policy_allowed(development_exception_enabled: bool) -> bool {
    development_exception_enabled
}

#[allow(clippy::too_many_arguments)]
fn executable_files_match(
    left_volume_serial_number: u32,
    left_file_index_high: u32,
    left_file_index_low: u32,
    left_number_of_links: u32,
    left_contains_reparse_point: bool,
    left_final_path: &str,
    right_volume_serial_number: u32,
    right_file_index_high: u32,
    right_file_index_low: u32,
    right_number_of_links: u32,
    right_contains_reparse_point: bool,
    right_final_path: &str,
) -> bool {
    !left_contains_reparse_point
        && !right_contains_reparse_point
        && left_number_of_links == 1
        && right_number_of_links == 1
        && left_volume_serial_number == right_volume_serial_number
        && left_file_index_high == right_file_index_high
        && left_file_index_low == right_file_index_low
        && !left_final_path.is_empty()
        && !right_final_path.is_empty()
        && left_final_path.to_uppercase() == right_final_path.to_uppercase()
}

fn basic_file_identities_match(
    left_volume_serial_number: u32,
    left_file_index_high: u32,
    left_file_index_low: u32,
    right_volume_serial_number: u32,
    right_file_index_high: u32,
    right_file_index_low: u32,
) -> bool {
    left_volume_serial_number == right_volume_serial_number
        && left_file_index_high == right_file_index_high
        && left_file_index_low == right_file_index_low
}

fn path_is_reparse_point_or_untrusted(path: &Path) -> bool {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
        .unwrap_or(true)
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

#[unsafe(no_mangle)]
/// # Safety
///
/// `module_path` must point to exactly `module_path_len` readable UTF-16 code
/// units. `output` may be null for size queries or writable UTF-16 storage for
/// `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_current_generation_for_module_utf16(
    module_path: *const u16,
    module_path_len: usize,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(module_path) = path_from_raw(module_path, module_path_len) else {
        return 0;
    };
    let Some(generation) = current_runtime_generation_for_module(&module_path) else {
        return 0;
    };
    write_wide_string(&generation, output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `install_root` must point to exactly `install_root_len` readable UTF-16 code
/// units. `output` may be null for size queries or writable UTF-16 storage for
/// `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_current_generation_from_install_root_utf16(
    install_root: *const u16,
    install_root_len: usize,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(install_root) = path_from_raw(install_root, install_root_len) else {
        return 0;
    };
    let Some(generation) = current_runtime_generation_from_install_root(&install_root) else {
        return 0;
    };
    write_wide_string(&generation, output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `module_path` must point to exactly `module_path_len` readable UTF-16 code
/// units. `output` may be null for size queries or writable UTF-16 storage for
/// `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_installation_root_for_module_utf16(
    module_path: *const u16,
    module_path_len: usize,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(module_path) = path_from_raw(module_path, module_path_len) else {
        return 0;
    };
    let Some(root) = install_root_for_module(&module_path) else {
        return 0;
    };
    write_wide_path(&root, output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `module_path` must point to exactly `module_path_len` readable UTF-16 code
/// units. `output` may be null for size queries or writable UTF-16 storage for
/// `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_portable_data_root_for_module_utf16(
    module_path: *const u16,
    module_path_len: usize,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(module_path) = path_from_raw(module_path, module_path_len) else {
        return 0;
    };
    let Some(root) = portable_data_root_for_module(&module_path) else {
        return 0;
    };
    write_wide_path(&root, output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `user_sid` must be null only when `user_sid_len` is zero, or point to a valid
/// UTF-16 buffer with exactly `user_sid_len` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_may_launch_user_engine_utf16(
    service_account: u8,
    session_id: u32,
    secure_desktop: u8,
    user_sid: *const u16,
    user_sid_len: usize,
) -> u8 {
    let Some(user_sid) = wide_string_from_raw(user_sid, user_sid_len) else {
        return 0;
    };
    may_launch_user_engine(
        service_account != 0,
        session_id,
        secure_desktop != 0,
        &user_sid,
    ) as u8
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `user_sid` must be null only when `user_sid_len` is zero, or point to a valid
/// UTF-16 buffer with exactly `user_sid_len` code units. `output` may be null
/// for size queries or writable UTF-16 storage for `capacity` code units. No
/// pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_pipe_security_sddl_utf16(
    service_account: u8,
    session_id: u32,
    user_sid: *const u16,
    user_sid_len: usize,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(user_sid) = wide_string_from_raw(user_sid, user_sid_len) else {
        return 0;
    };
    let Some(sddl) = pipe_security_sddl(service_account != 0, session_id, &user_sid) else {
        return 0;
    };
    write_wide_string(&sddl, output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `user_sid` must be null only when `user_sid_len` is zero, or point to a valid
/// UTF-16 buffer with exactly `user_sid_len` code units. The returned pointer is
/// null on failure; non-null pointers are allocated by Windows and must be
/// released with `LocalFree`.
pub unsafe extern "C" fn fcitx5_windows_common_pipe_security_descriptor_utf16(
    service_account: u8,
    session_id: u32,
    user_sid: *const u16,
    user_sid_len: usize,
) -> *mut c_void {
    let Some(user_sid) = wide_string_from_raw(user_sid, user_sid_len) else {
        return std::ptr::null_mut();
    };
    pipe_security_descriptor(service_account != 0, session_id, &user_sid)
        .unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
/// # Safety
///
/// SID pointers must be null only when their corresponding length is zero, or
/// point to valid UTF-16 buffers with exactly the provided lengths. No pointer
/// is retained.
pub unsafe extern "C" fn fcitx5_windows_common_same_principal_session_utf16(
    peer_session_id: u32,
    peer_service_account: u8,
    peer_user_sid: *const u16,
    peer_user_sid_len: usize,
    current_session_id: u32,
    current_user_sid: *const u16,
    current_user_sid_len: usize,
) -> u8 {
    let Some(peer_user_sid) = wide_string_from_raw(peer_user_sid, peer_user_sid_len) else {
        return 0;
    };
    let Some(current_user_sid) = wide_string_from_raw(current_user_sid, current_user_sid_len)
    else {
        return 0;
    };
    same_principal_and_session(
        peer_session_id,
        peer_service_account != 0,
        &peer_user_sid,
        current_session_id,
        &current_user_sid,
    ) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_peer_development_policy_allowed(
    development_exception_enabled: u8,
) -> u8 {
    peer_development_policy_allowed(development_exception_enabled != 0) as u8
}

#[unsafe(no_mangle)]
/// # Safety
///
/// Final-path pointers must be null only when their corresponding length is
/// zero, or point to valid UTF-16 buffers with exactly the provided lengths. No
/// pointer is retained.
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fcitx5_windows_common_executable_files_match_utf16(
    left_volume_serial_number: u32,
    left_file_index_high: u32,
    left_file_index_low: u32,
    left_number_of_links: u32,
    left_contains_reparse_point: u8,
    left_final_path: *const u16,
    left_final_path_len: usize,
    right_volume_serial_number: u32,
    right_file_index_high: u32,
    right_file_index_low: u32,
    right_number_of_links: u32,
    right_contains_reparse_point: u8,
    right_final_path: *const u16,
    right_final_path_len: usize,
) -> u8 {
    let Some(left_final_path) = wide_string_from_raw(left_final_path, left_final_path_len) else {
        return 0;
    };
    let Some(right_final_path) = wide_string_from_raw(right_final_path, right_final_path_len)
    else {
        return 0;
    };
    executable_files_match(
        left_volume_serial_number,
        left_file_index_high,
        left_file_index_low,
        left_number_of_links,
        left_contains_reparse_point != 0,
        &left_final_path,
        right_volume_serial_number,
        right_file_index_high,
        right_file_index_low,
        right_number_of_links,
        right_contains_reparse_point != 0,
        &right_final_path,
    ) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_basic_file_identities_match(
    left_volume_serial_number: u32,
    left_file_index_high: u32,
    left_file_index_low: u32,
    right_volume_serial_number: u32,
    right_file_index_high: u32,
    right_file_index_low: u32,
) -> u8 {
    basic_file_identities_match(
        left_volume_serial_number,
        left_file_index_high,
        left_file_index_low,
        right_volume_serial_number,
        right_file_index_high,
        right_file_index_low,
    ) as u8
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `path` must point to exactly `path_len` readable UTF-16 code units. The
/// function returns true for invalid/unreadable paths to preserve the
/// fail-closed executable identity policy.
pub unsafe extern "C" fn fcitx5_windows_common_path_is_reparse_point_utf16(
    path: *const u16,
    path_len: usize,
) -> u8 {
    let Some(path) = path_from_raw(path, path_len) else {
        return 1;
    };
    path_is_reparse_point_or_untrusted(&path) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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

    #[test]
    fn runtime_generation_and_install_roots_match_cpp_contract() {
        let root = env::temp_dir().join(format!(
            "fcitx5-windows-common-generation-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tsf").join("x64")).expect("create tsf root");
        fs::create_dir_all(root.join("runtime").join("00000041").join("bin"))
            .expect("create runtime root");
        fs::write(root.join("portable.flag"), b"portable\n").expect("write portable flag");
        fs::write(
            root.join("current.json"),
            b"{\n  \"format_version\": 1,\n  \"current_generation\": \"00000042\",\n  \"previous_generation\": \"00000041\",\n  \"build_id\": \"build-42\"\n}\n",
        )
        .expect("write current generation");
        fs::write(
            root.join("tsf").join("x64").join("fcitx5-tsf.generation"),
            b"00000044\n",
        )
        .expect("write tsf generation");
        let tsf_module = root.join("tsf").join("x64").join("fcitx5-tsf.dll");
        let runtime_module = root
            .join("runtime")
            .join("00000041")
            .join("fcitx5-engine.exe");
        let runtime_bin_module = root
            .join("runtime")
            .join("00000041")
            .join("bin")
            .join("fcitx5-engine.exe");
        assert_eq!(
            current_runtime_generation_from_install_root(&root).as_deref(),
            Some("00000042")
        );
        assert_eq!(
            current_runtime_generation_for_module(&tsf_module).as_deref(),
            Some("00000044")
        );
        assert_eq!(
            current_runtime_generation_for_module(&runtime_module).as_deref(),
            Some("00000041")
        );
        assert_eq!(
            current_runtime_generation_for_module(&runtime_bin_module).as_deref(),
            Some("00000041")
        );
        assert_eq!(
            install_root_for_module(&runtime_bin_module).as_deref(),
            Some(root.as_path())
        );
        assert_eq!(
            portable_data_root_for_module(&runtime_bin_module).as_deref(),
            Some(root.join("data").as_path())
        );
        fs::File::create(root.join("current.json"))
            .expect("truncate current")
            .write_all(b"{\"current_generation\":\"../bad\"}\n")
            .expect("write invalid current");
        assert_eq!(current_runtime_generation_from_install_root(&root), None);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn user_engine_launch_policy_matches_cpp_contract() {
        assert!(may_launch_user_engine(false, 1, false, "S-1-5-21-test"));
        assert!(!may_launch_user_engine(true, 1, false, "S-1-5-21-test"));
        assert!(!may_launch_user_engine(false, 0, false, "S-1-5-21-test"));
        assert!(!may_launch_user_engine(false, 1, true, "S-1-5-21-test"));
        assert!(!may_launch_user_engine(false, 1, false, ""));
    }

    #[test]
    fn pipe_security_sddl_matches_cpp_contract() {
        assert_eq!(
            pipe_security_sddl(false, 7, "S-1-5-21-test").as_deref(),
            Some("D:P(A;;GA;;;SY)(A;;GA;;;S-1-5-21-test)S:(ML;;NW;;;ME)")
        );
        assert_eq!(pipe_security_sddl(true, 7, "S-1-5-21-test"), None);
        assert_eq!(pipe_security_sddl(false, 0, "S-1-5-21-test"), None);
        assert_eq!(pipe_security_sddl(false, 7, ""), None);
    }

    #[test]
    fn peer_verification_policy_matches_cpp_contract() {
        assert!(same_principal_and_session(
            7,
            false,
            "S-1-5-21-TEST",
            7,
            "s-1-5-21-test"
        ));
        assert!(!same_principal_and_session(
            8,
            false,
            "S-1-5-21-test",
            7,
            "S-1-5-21-test"
        ));
        assert!(!same_principal_and_session(
            7,
            true,
            "S-1-5-21-test",
            7,
            "S-1-5-21-test"
        ));
        assert!(!same_principal_and_session(
            7,
            false,
            "",
            7,
            "S-1-5-21-test"
        ));
        assert!(peer_development_policy_allowed(true));
        assert!(!peer_development_policy_allowed(false));
    }

    #[test]
    fn executable_file_match_policy_matches_cpp_contract() {
        assert!(executable_files_match(
            11,
            22,
            33,
            1,
            false,
            r"\\?\C:\Program Files\Fcitx5\fcitx5-engine.exe",
            11,
            22,
            33,
            1,
            false,
            r"\\?\c:\program files\fcitx5\FCITX5-ENGINE.EXE",
        ));
        assert!(!executable_files_match(
            11,
            22,
            33,
            2,
            false,
            r"\\?\C:\Program Files\Fcitx5\fcitx5-engine.exe",
            11,
            22,
            33,
            1,
            false,
            r"\\?\C:\Program Files\Fcitx5\fcitx5-engine.exe",
        ));
        assert!(!executable_files_match(
            11,
            22,
            33,
            1,
            true,
            r"\\?\C:\Program Files\Fcitx5\fcitx5-engine.exe",
            11,
            22,
            33,
            1,
            false,
            r"\\?\C:\Program Files\Fcitx5\fcitx5-engine.exe",
        ));
        assert!(!executable_files_match(
            11,
            22,
            33,
            1,
            false,
            "",
            11,
            22,
            33,
            1,
            false,
            r"\\?\C:\Program Files\Fcitx5\fcitx5-engine.exe",
        ));
    }

    #[test]
    fn basic_file_identity_match_policy_matches_cpp_contract() {
        assert!(basic_file_identities_match(11, 22, 33, 11, 22, 33));
        assert!(!basic_file_identities_match(12, 22, 33, 11, 22, 33));
        assert!(!basic_file_identities_match(11, 23, 33, 11, 22, 33));
        assert!(!basic_file_identities_match(11, 22, 34, 11, 22, 33));
    }

    #[test]
    fn path_reparse_policy_fails_closed_like_cpp_contract() {
        let missing = env::temp_dir().join(format!(
            "fcitx5-windows-common-missing-reparse-{}",
            std::process::id()
        ));
        let _ = fs::remove_file(&missing);
        assert!(path_is_reparse_point_or_untrusted(&missing));
    }
}
