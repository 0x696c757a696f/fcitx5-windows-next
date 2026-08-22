#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_void, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::ptr::null_mut;

type Dword = u32;
type Lstatus = i32;
type Hkey = *mut c_void;

const ERROR_SUCCESS: Lstatus = 0;
const ERROR_FILE_NOT_FOUND: Lstatus = 2;
const KEY_QUERY_VALUE: Dword = 0x0001;
const KEY_SET_VALUE: Dword = 0x0002;
const REG_SZ: Dword = 1;
const HKEY_CURRENT_USER: Hkey = 0x8000_0001_usize as Hkey;
const RUN_KEY: &[u16] = &[
    b'S' as u16,
    b'o' as u16,
    b'f' as u16,
    b't' as u16,
    b'w' as u16,
    b'a' as u16,
    b'r' as u16,
    b'e' as u16,
    b'\\' as u16,
    b'M' as u16,
    b'i' as u16,
    b'c' as u16,
    b'r' as u16,
    b'o' as u16,
    b's' as u16,
    b'o' as u16,
    b'f' as u16,
    b't' as u16,
    b'\\' as u16,
    b'W' as u16,
    b'i' as u16,
    b'n' as u16,
    b'd' as u16,
    b'o' as u16,
    b'w' as u16,
    b's' as u16,
    b'\\' as u16,
    b'C' as u16,
    b'u' as u16,
    b'r' as u16,
    b'r' as u16,
    b'e' as u16,
    b'n' as u16,
    b't' as u16,
    b'V' as u16,
    b'e' as u16,
    b'r' as u16,
    b's' as u16,
    b'i' as u16,
    b'o' as u16,
    b'n' as u16,
    b'\\' as u16,
    b'R' as u16,
    b'u' as u16,
    b'n' as u16,
    0,
];
const CONTROL_SCHEMA_JSON: &str = concat!(
    r#"{"format_version":1,"commands":["#,
    r#""status","restart_engine","shutdown","validate_config","apply_config","#,
    r#""reset_config","reset_presentation","get_startup","set_startup","#,
    r#""get_presentation","set_presentation","get_input_methods","set_input_method","#,
    r#""themes_list","themes_detail","addons_list","packages_list","packages_detail","#,
    r#""packages_refresh","packages_install","packages_update","packages_state","#,
    r#""packages_remove","packages_repair","get_tsf_guard","reset_tsf_guard"],"#,
    r#""sensitive_input":false,"package_network_owner":"fcitx5-downloader.exe"}"#
);

#[repr(C)]
pub struct Fcitx5ControlUtf16 {
    ptr: *const u16,
    len: usize,
}

#[repr(C)]
pub struct Fcitx5ControlUtf8 {
    ptr: *const u8,
    len: usize,
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        h_key: Hkey,
        sub_key: *const u16,
        options: Dword,
        sam_desired: Dword,
        result: *mut Hkey,
    ) -> Lstatus;
    fn RegCreateKeyExW(
        h_key: Hkey,
        sub_key: *const u16,
        reserved: Dword,
        class: *mut u16,
        options: Dword,
        sam_desired: Dword,
        security_attributes: *mut c_void,
        result: *mut Hkey,
        disposition: *mut Dword,
    ) -> Lstatus;
    fn RegQueryValueExW(
        h_key: Hkey,
        value_name: *const u16,
        reserved: *mut Dword,
        value_type: *mut Dword,
        data: *mut u8,
        data_size: *mut Dword,
    ) -> Lstatus;
    fn RegSetValueExW(
        h_key: Hkey,
        value_name: *const u16,
        reserved: Dword,
        value_type: Dword,
        data: *const u8,
        data_size: Dword,
    ) -> Lstatus;
    fn RegDeleteValueW(h_key: Hkey, value_name: *const u16) -> Lstatus;
    fn RegCloseKey(h_key: Hkey) -> Lstatus;
}

struct RegistryKey(Hkey);

impl RegistryKey {
    fn get(&self) -> Hkey {
        self.0
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

fn string_from_utf16(value: Fcitx5ControlUtf16) -> Option<OsString> {
    if value.ptr.is_null() {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    Some(OsString::from_wide(slice))
}

fn wide_z(value: &std::ffi::OsStr) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_wide().collect();
    wide.push(0);
    wide
}

fn quote(value: &std::ffi::OsStr) -> OsString {
    let wide: Vec<u16> = value.encode_wide().collect();
    let mut result = Vec::with_capacity(wide.len() + 2);
    result.push(b'"' as u16);
    let mut backslashes = 0_usize;
    for character in wide {
        if character == b'\\' as u16 {
            backslashes += 1;
        } else if character == b'"' as u16 {
            result.extend(std::iter::repeat_n(b'\\' as u16, backslashes + 1));
            backslashes = 0;
            result.push(character);
        } else {
            result.extend(std::iter::repeat_n(b'\\' as u16, backslashes));
            backslashes = 0;
            result.push(character);
        }
    }
    result.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2));
    result.push(b'"' as u16);
    OsString::from_wide(&result)
}

fn json_string(value: &[u8]) -> Option<Vec<u8>> {
    let mut result = Vec::with_capacity(value.len() + 2);
    result.push(b'"');
    for byte in value {
        match *byte {
            b'\\' => result.extend_from_slice(b"\\\\"),
            b'"' => result.extend_from_slice(br#"\""#),
            b'\x08' => result.extend_from_slice(br#"\b"#),
            b'\x0c' => result.extend_from_slice(br#"\f"#),
            b'\n' => result.extend_from_slice(br#"\n"#),
            b'\r' => result.extend_from_slice(br#"\r"#),
            b'\t' => result.extend_from_slice(br#"\t"#),
            0x00..=0x1f => return None,
            other => result.push(other),
        }
    }
    result.push(b'"');
    Some(result)
}

fn boxed_utf8_result(value: Vec<u8>, out_ptr: *mut *mut u8, out_len: *mut usize) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    let mut bytes = value.into_boxed_slice();
    let ptr = bytes.as_mut_ptr();
    let len = bytes.len();
    std::mem::forget(bytes);
    unsafe {
        *out_ptr = ptr;
        *out_len = len;
    }
    0
}

fn startup_command(executable_directory: OsString) -> Vec<u16> {
    let launcher = PathBuf::from(executable_directory).join("fcitx5-launcher.exe");
    let mut command = quote(launcher.as_os_str());
    command.push(" --background");
    wide_z(&command)
}

fn query_startup(executable_directory: OsString, registry_value: OsString) -> Result<bool, ()> {
    let expected = startup_command(executable_directory);
    let value_name = wide_z(&registry_value);
    let mut raw_key = null_mut();
    let open_result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut raw_key,
        )
    };
    if open_result != ERROR_SUCCESS {
        return Ok(false);
    }
    let key = RegistryKey(raw_key);
    let mut value_type = 0_u32;
    let mut bytes = 0_u32;
    let size_result = unsafe {
        RegQueryValueExW(
            key.get(),
            value_name.as_ptr(),
            null_mut(),
            &mut value_type,
            null_mut(),
            &mut bytes,
        )
    };
    if size_result == ERROR_FILE_NOT_FOUND {
        return Ok(false);
    }
    if size_result != ERROR_SUCCESS || value_type != REG_SZ || !(2..=64 * 1024).contains(&bytes) {
        return Err(());
    }
    let mut value = vec![0_u16; (bytes as usize).div_ceil(2)];
    let read_result = unsafe {
        RegQueryValueExW(
            key.get(),
            value_name.as_ptr(),
            null_mut(),
            &mut value_type,
            value.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    while value.last().copied() == Some(0) {
        value.pop();
    }
    let mut expected_trimmed = expected;
    while expected_trimmed.last().copied() == Some(0) {
        expected_trimmed.pop();
    }
    if read_result != ERROR_SUCCESS {
        return Err(());
    }
    Ok(value == expected_trimmed)
}

fn set_startup(
    executable_directory: OsString,
    registry_value: OsString,
    enabled: bool,
) -> Result<(), ()> {
    let value_name = wide_z(&registry_value);
    let mut raw_key = null_mut();
    let create_result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY.as_ptr(),
            0,
            null_mut(),
            0,
            KEY_SET_VALUE,
            null_mut(),
            &mut raw_key,
            null_mut(),
        )
    };
    if create_result != ERROR_SUCCESS {
        return Err(());
    }
    let key = RegistryKey(raw_key);
    let result = if enabled {
        let command = startup_command(executable_directory);
        unsafe {
            RegSetValueExW(
                key.get(),
                value_name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as Dword,
            )
        }
    } else {
        let delete_result = unsafe { RegDeleteValueW(key.get(), value_name.as_ptr()) };
        if delete_result == ERROR_FILE_NOT_FOUND {
            ERROR_SUCCESS
        } else {
            delete_result
        }
    };
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(())
    }
}

/// # Safety
///
/// UTF-16 slices must remain valid for the duration of the call. `out_enabled`
/// must point to writable storage. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_startup_query_utf16(
    executable_directory: Fcitx5ControlUtf16,
    registry_value: Fcitx5ControlUtf16,
    out_enabled: *mut u8,
) -> i32 {
    if out_enabled.is_null() {
        return 1;
    }
    let Some(executable_directory) = string_from_utf16(executable_directory) else {
        return 1;
    };
    let Some(registry_value) = string_from_utf16(registry_value) else {
        return 1;
    };
    match query_startup(executable_directory, registry_value) {
        Ok(enabled) => {
            unsafe {
                *out_enabled = u8::from(enabled);
            }
            0
        }
        Err(()) => 1,
    }
}

/// # Safety
///
/// UTF-16 slices must remain valid for the duration of the call. No pointer is
/// retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_startup_set_utf16(
    executable_directory: Fcitx5ControlUtf16,
    registry_value: Fcitx5ControlUtf16,
    enabled: u8,
) -> i32 {
    let Some(executable_directory) = string_from_utf16(executable_directory) else {
        return 1;
    };
    let Some(registry_value) = string_from_utf16(registry_value) else {
        return 1;
    };
    match set_startup(executable_directory, registry_value, enabled != 0) {
        Ok(()) => 0,
        Err(()) => 1,
    }
}

/// # Safety
///
/// `out_ptr` and `out_len` must point to writable storage. The returned pointer
/// is process-static UTF-8 data and must not be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_schema_json_utf8(
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    unsafe {
        *out_ptr = CONTROL_SCHEMA_JSON.as_ptr();
        *out_len = CONTROL_SCHEMA_JSON.len();
    }
    0
}

/// # Safety
///
/// `id` must remain valid for the duration of the call. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_input_method_id_valid_utf16(id: Fcitx5ControlUtf16) -> u8 {
    if id.ptr.is_null() {
        return 0;
    }
    let value = unsafe { std::slice::from_raw_parts(id.ptr, id.len) };
    if value.is_empty() || value.len() > 64 {
        return 0;
    }
    u8::from(value.iter().all(|character| {
        (*character >= b'a' as u16 && *character <= b'z' as u16)
            || (*character >= b'0' as u16 && *character <= b'9' as u16)
            || *character == b'-' as u16
            || *character == b'_' as u16
    }))
}

/// # Safety
///
/// `value` must remain valid for the duration of the call. `out_ptr` and
/// `out_len` must point to writable storage. Any returned buffer must be freed
/// with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_json_string_utf8(
    value: Fcitx5ControlUtf8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if value.ptr.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let bytes = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    match json_string(bytes) {
        Some(escaped) => boxed_utf8_result(escaped, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `ptr` and `len` must be the exact buffer returned by a Control core UTF-8
/// allocation function, or `ptr` must be null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_utf8_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wide(value: &str) -> Vec<u16> {
        OsString::from(value).encode_wide().collect()
    }

    #[test]
    fn startup_command_quotes_launcher_path() {
        let command = startup_command(OsString::from(r"C:\Program Files\Fcitx5\bin"));
        let mut trimmed = command;
        while trimmed.last().copied() == Some(0) {
            trimmed.pop();
        }
        assert_eq!(
            trimmed,
            wide(r#""C:\Program Files\Fcitx5\bin\fcitx5-launcher.exe" --background"#)
        );
    }

    #[test]
    fn schema_documents_typed_control_commands() {
        assert!(CONTROL_SCHEMA_JSON.contains(r#""format_version":1"#));
        assert!(CONTROL_SCHEMA_JSON.contains(r#""set_presentation""#));
        assert!(CONTROL_SCHEMA_JSON.contains(r#""packages_repair""#));
        assert!(CONTROL_SCHEMA_JSON.contains(r#""package_network_owner":"fcitx5-downloader.exe""#));
        assert!(!CONTROL_SCHEMA_JSON.contains("sensitive_input\":true"));
    }

    #[test]
    fn input_method_ids_are_bounded_lowercase_ascii_tokens() {
        let valid = wide("rime-luna_pinyin");
        let upper = wide("Rime");
        let empty: Vec<u16> = Vec::new();
        let long = wide(&"a".repeat(65));
        unsafe {
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: valid.as_ptr(),
                    len: valid.len()
                }),
                1
            );
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: upper.as_ptr(),
                    len: upper.len()
                }),
                0
            );
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: empty.as_ptr(),
                    len: empty.len()
                }),
                0
            );
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: long.as_ptr(),
                    len: long.len()
                }),
                0
            );
        }
    }

    #[test]
    fn json_string_matches_control_output_contract() {
        assert_eq!(json_string(b"plain").as_deref(), Some(&b"\"plain\""[..]));
        assert_eq!(
            json_string(b"quote\"slash\\\n\t").as_deref(),
            Some(&br#""quote\"slash\\\n\t""#[..])
        );
        assert_eq!(
            json_string("企鹅".as_bytes()).as_deref(),
            Some(&b"\"\xe4\xbc\x81\xe9\xb9\x85\""[..])
        );
        assert_eq!(json_string(&[0x01]), None);
    }
}
