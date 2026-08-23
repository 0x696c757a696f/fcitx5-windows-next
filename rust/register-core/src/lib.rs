#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

type Dword = u32;
type Lstatus = i32;
type Hkey = *mut std::ffi::c_void;

const ERROR_SUCCESS: Lstatus = 0;
const ERROR_FILE_NOT_FOUND: Lstatus = 2;
const KEY_QUERY_VALUE: Dword = 0x0001;
const REG_SZ: Dword = 1;
const HKEY_LOCAL_MACHINE: Hkey = 0x8000_0002_usize as Hkey;
const TSF_TEXT_SERVICE_CLSID: &str = "{3A21B9E2-4F47-4C36-8BFA-91D7D3B3E901}";

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Fcitx5RegisterUtf16 {
    ptr: *const u16,
    len: usize,
}

pub const REGISTER_ARTIFACT_OK: u32 = 0;
pub const REGISTER_ARTIFACT_INVALID_ARGUMENT: u32 = 1;
pub const REGISTER_ARTIFACT_HELPER_LOCATION: u32 = 2;
pub const REGISTER_ARTIFACT_CURRENT_DLL_MISSING: u32 = 3;
pub const REGISTER_ARTIFACT_PAIRED_DLL_MISSING: u32 = 4;
pub const REGISTER_ARTIFACT_DLL_OUTSIDE_PRODUCT: u32 = 5;

pub const REGISTER_OPERATION_UNKNOWN: u32 = 0;
pub const REGISTER_OPERATION_REGISTER: u32 = 1;
pub const REGISTER_OPERATION_REPAIR: u32 = 2;
pub const REGISTER_OPERATION_UNREGISTER: u32 = 3;
pub const REGISTER_OPERATION_STATUS: u32 = 4;
pub const REGISTER_OPERATION_VALIDATE_ARTIFACT: u32 = 5;

pub const REGISTER_DLL_ARGUMENT_OK: u32 = 0;
pub const REGISTER_DLL_ARGUMENT_INVALID: u32 = 1;

pub const REGISTER_EXPORT_NONE: u32 = 0;
pub const REGISTER_EXPORT_REGISTER_SERVER: u32 = 1;
pub const REGISTER_EXPORT_UNREGISTER_SERVER: u32 = 2;

pub const REGISTER_STATUS_REGISTERED: u32 = 0;
pub const REGISTER_STATUS_NOT_REGISTERED: u32 = 1;
pub const REGISTER_STATUS_PATH_MISMATCH: u32 = 2;
pub const REGISTER_STATUS_INVALID_ARGUMENT: u32 = 3;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        h_key: Hkey,
        sub_key: *const u16,
        options: Dword,
        sam_desired: Dword,
        result: *mut Hkey,
    ) -> Lstatus;
    fn RegQueryValueExW(
        h_key: Hkey,
        value_name: *const u16,
        reserved: *mut Dword,
        value_type: *mut Dword,
        data: *mut u8,
        data_size: *mut Dword,
    ) -> Lstatus;
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

fn path_from_utf16(value: Fcitx5RegisterUtf16) -> Option<PathBuf> {
    if value.ptr.is_null() {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    Some(PathBuf::from(OsString::from_wide(slice)))
}

fn wide_z(value: &OsStr) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_wide().collect();
    wide.push(0);
    wide
}

fn architecture_names(bits: u32) -> Option<(&'static str, &'static str)> {
    match bits {
        64 => Some(("x64", "x86")),
        32 => Some(("x86", "x64")),
        _ => None,
    }
}

fn same_path(first: &Path, second: &Path) -> bool {
    let Ok(first) = std::fs::canonicalize(first) else {
        return false;
    };
    let Ok(second) = std::fs::canonicalize(second) else {
        return false;
    };
    first
        .as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&second.as_os_str().to_string_lossy())
}

pub fn validate_product_artifact(helper: &Path, dll: &Path, architecture_bits: u32) -> u32 {
    let Some((current_architecture, paired_architecture)) = architecture_names(architecture_bits)
    else {
        return REGISTER_ARTIFACT_INVALID_ARGUMENT;
    };
    if helper.parent().and_then(Path::file_name) != Some(OsStr::new("bin")) {
        return REGISTER_ARTIFACT_HELPER_LOCATION;
    }
    let Some(root) = helper.parent().and_then(Path::parent) else {
        return REGISTER_ARTIFACT_HELPER_LOCATION;
    };

    let expected = root
        .join("tsf")
        .join(current_architecture)
        .join("fcitx5-tsf.dll");
    let paired = root
        .join("tsf")
        .join(paired_architecture)
        .join("fcitx5-tsf.dll");
    if !expected.is_file() {
        return REGISTER_ARTIFACT_CURRENT_DLL_MISSING;
    }
    if !paired.is_file() {
        return REGISTER_ARTIFACT_PAIRED_DLL_MISSING;
    }
    if !same_path(dll, &expected) {
        return REGISTER_ARTIFACT_DLL_OUTSIDE_PRODUCT;
    }
    REGISTER_ARTIFACT_OK
}

pub fn parse_operation(operation: &OsStr) -> u32 {
    let text = operation.to_string_lossy();
    match text.as_ref() {
        "--register" => REGISTER_OPERATION_REGISTER,
        "--repair" => REGISTER_OPERATION_REPAIR,
        "--unregister" => REGISTER_OPERATION_UNREGISTER,
        "--status" => REGISTER_OPERATION_STATUS,
        "--validate-artifact" => REGISTER_OPERATION_VALIDATE_ARTIFACT,
        _ => REGISTER_OPERATION_UNKNOWN,
    }
}

pub fn validate_dll_argument(dll: &Path) -> u32 {
    if dll.is_absolute() && dll.file_name() == Some(OsStr::new("fcitx5-tsf.dll")) {
        REGISTER_DLL_ARGUMENT_OK
    } else {
        REGISTER_DLL_ARGUMENT_INVALID
    }
}

pub fn operation_requires_admin(operation: u32) -> u32 {
    matches!(
        operation,
        REGISTER_OPERATION_REGISTER | REGISTER_OPERATION_REPAIR | REGISTER_OPERATION_UNREGISTER
    ) as u32
}

pub fn operation_export(operation: u32) -> u32 {
    match operation {
        REGISTER_OPERATION_REGISTER | REGISTER_OPERATION_REPAIR => REGISTER_EXPORT_REGISTER_SERVER,
        REGISTER_OPERATION_UNREGISTER => REGISTER_EXPORT_UNREGISTER_SERVER,
        _ => REGISTER_EXPORT_NONE,
    }
}

fn registered_path() -> Option<PathBuf> {
    let key_path = OsString::from(format!(
        r"Software\Classes\CLSID\{}\InprocServer32",
        TSF_TEXT_SERVICE_CLSID
    ));
    let key_path = wide_z(&key_path);
    let mut raw_key: Hkey = std::ptr::null_mut();
    let open = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            key_path.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut raw_key,
        )
    };
    if open == ERROR_FILE_NOT_FOUND {
        return None;
    }
    if open != ERROR_SUCCESS || raw_key.is_null() {
        return None;
    }
    let key = RegistryKey(raw_key);
    let mut value_type: Dword = 0;
    let mut bytes: Dword = 0;
    let query_size = unsafe {
        RegQueryValueExW(
            key.get(),
            std::ptr::null(),
            std::ptr::null_mut(),
            &mut value_type,
            std::ptr::null_mut(),
            &mut bytes,
        )
    };
    if query_size != ERROR_SUCCESS || value_type != REG_SZ || bytes < 2 {
        return None;
    }
    let mut value = vec![0_u16; (bytes as usize).div_ceil(2)];
    let query_value = unsafe {
        RegQueryValueExW(
            key.get(),
            std::ptr::null(),
            std::ptr::null_mut(),
            &mut value_type,
            value.as_mut_ptr().cast::<u8>(),
            &mut bytes,
        )
    };
    if query_value != ERROR_SUCCESS || value_type != REG_SZ {
        return None;
    }
    while value.last() == Some(&0) {
        value.pop();
    }
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(OsString::from_wide(&value)))
}

pub fn registration_status_for_dll(dll: &Path) -> u32 {
    let Some(actual) = registered_path() else {
        return REGISTER_STATUS_NOT_REGISTERED;
    };
    if same_path(&actual, dll) {
        REGISTER_STATUS_REGISTERED
    } else {
        REGISTER_STATUS_PATH_MISMATCH
    }
}

#[no_mangle]
pub extern "C" fn fcitx5_register_validate_artifact(
    helper: Fcitx5RegisterUtf16,
    dll: Fcitx5RegisterUtf16,
    architecture_bits: u32,
) -> u32 {
    let Some(helper) = path_from_utf16(helper) else {
        return REGISTER_ARTIFACT_INVALID_ARGUMENT;
    };
    let Some(dll) = path_from_utf16(dll) else {
        return REGISTER_ARTIFACT_INVALID_ARGUMENT;
    };
    validate_product_artifact(&helper, &dll, architecture_bits)
}

#[no_mangle]
pub extern "C" fn fcitx5_register_parse_operation(operation: Fcitx5RegisterUtf16) -> u32 {
    let Some(operation) = path_from_utf16(operation) else {
        return REGISTER_OPERATION_UNKNOWN;
    };
    parse_operation(operation.as_os_str())
}

#[no_mangle]
pub extern "C" fn fcitx5_register_validate_dll_argument(dll: Fcitx5RegisterUtf16) -> u32 {
    let Some(dll) = path_from_utf16(dll) else {
        return REGISTER_DLL_ARGUMENT_INVALID;
    };
    validate_dll_argument(&dll)
}

#[no_mangle]
pub extern "C" fn fcitx5_register_operation_requires_admin(operation: u32) -> u32 {
    operation_requires_admin(operation)
}

#[no_mangle]
pub extern "C" fn fcitx5_register_operation_export(operation: u32) -> u32 {
    operation_export(operation)
}

#[no_mangle]
pub extern "C" fn fcitx5_register_registration_status_for_dll(dll: Fcitx5RegisterUtf16) -> u32 {
    let Some(dll) = path_from_utf16(dll) else {
        return REGISTER_STATUS_INVALID_ARGUMENT;
    };
    registration_status_for_dll(&dll)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-register-core-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        root
    }

    fn write_fixture(path: &Path) {
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
        fs::write(path, b"fixture").expect("write fixture");
    }

    #[test]
    fn parses_register_operations_and_validates_tsf_dll_argument() {
        assert_eq!(
            parse_operation(OsStr::new("--register")),
            REGISTER_OPERATION_REGISTER
        );
        assert_eq!(
            parse_operation(OsStr::new("--repair")),
            REGISTER_OPERATION_REPAIR
        );
        assert_eq!(
            parse_operation(OsStr::new("--unregister")),
            REGISTER_OPERATION_UNREGISTER
        );
        assert_eq!(
            parse_operation(OsStr::new("--status")),
            REGISTER_OPERATION_STATUS
        );
        assert_eq!(
            parse_operation(OsStr::new("--validate-artifact")),
            REGISTER_OPERATION_VALIDATE_ARTIFACT
        );
        assert_eq!(
            parse_operation(OsStr::new("--bad")),
            REGISTER_OPERATION_UNKNOWN
        );
        assert_eq!(
            validate_dll_argument(Path::new("C:/Fcitx5/tsf/x64/fcitx5-tsf.dll")),
            REGISTER_DLL_ARGUMENT_OK
        );
        assert_eq!(
            validate_dll_argument(Path::new("fcitx5-tsf.dll")),
            REGISTER_DLL_ARGUMENT_INVALID
        );
        assert_eq!(
            validate_dll_argument(Path::new("C:/Fcitx5/tsf/x64/other.dll")),
            REGISTER_DLL_ARGUMENT_INVALID
        );
    }

    #[test]
    fn classifies_registration_privilege_and_export_policy() {
        assert_eq!(operation_requires_admin(REGISTER_OPERATION_REGISTER), 1);
        assert_eq!(operation_requires_admin(REGISTER_OPERATION_REPAIR), 1);
        assert_eq!(operation_requires_admin(REGISTER_OPERATION_UNREGISTER), 1);
        assert_eq!(operation_requires_admin(REGISTER_OPERATION_STATUS), 0);
        assert_eq!(
            operation_requires_admin(REGISTER_OPERATION_VALIDATE_ARTIFACT),
            0
        );
        assert_eq!(
            operation_export(REGISTER_OPERATION_REGISTER),
            REGISTER_EXPORT_REGISTER_SERVER
        );
        assert_eq!(
            operation_export(REGISTER_OPERATION_REPAIR),
            REGISTER_EXPORT_REGISTER_SERVER
        );
        assert_eq!(
            operation_export(REGISTER_OPERATION_UNREGISTER),
            REGISTER_EXPORT_UNREGISTER_SERVER
        );
        assert_eq!(
            operation_export(REGISTER_OPERATION_STATUS),
            REGISTER_EXPORT_NONE
        );
    }

    #[test]
    fn status_constants_preserve_register_cli_contract() {
        assert_eq!(REGISTER_STATUS_REGISTERED, 0);
        assert_eq!(REGISTER_STATUS_NOT_REGISTERED, 1);
        assert_eq!(REGISTER_STATUS_PATH_MISMATCH, 2);
        assert_eq!(REGISTER_STATUS_INVALID_ARGUMENT, 3);
    }

    #[test]
    fn validates_current_and_paired_tsf_artifact_paths() {
        let root = unique_root("valid");
        let helper = root.join("Fcitx5").join("bin").join("fcitx5-register.exe");
        write_fixture(&helper);
        let x64 = root
            .join("Fcitx5")
            .join("tsf")
            .join("x64")
            .join("fcitx5-tsf.dll");
        let x86 = root
            .join("Fcitx5")
            .join("tsf")
            .join("x86")
            .join("fcitx5-tsf.dll");
        write_fixture(&x64);
        write_fixture(&x86);

        assert_eq!(
            validate_product_artifact(&helper, &x64, 64),
            REGISTER_ARTIFACT_OK
        );
        assert_eq!(
            validate_product_artifact(&helper, &x86, 32),
            REGISTER_ARTIFACT_OK
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_outside_or_incomplete_tsf_artifacts() {
        let root = unique_root("invalid");
        let helper = root.join("Fcitx5").join("bin").join("fcitx5-register.exe");
        write_fixture(&helper);
        let x64 = root
            .join("Fcitx5")
            .join("tsf")
            .join("x64")
            .join("fcitx5-tsf.dll");
        let x86 = root
            .join("Fcitx5")
            .join("tsf")
            .join("x86")
            .join("fcitx5-tsf.dll");
        let outside = root.join("outside").join("fcitx5-tsf.dll");
        write_fixture(&x64);
        write_fixture(&outside);

        assert_eq!(
            validate_product_artifact(&helper, &x64, 64),
            REGISTER_ARTIFACT_PAIRED_DLL_MISSING
        );
        write_fixture(&x86);
        assert_eq!(
            validate_product_artifact(&helper, &outside, 64),
            REGISTER_ARTIFACT_DLL_OUTSIDE_PRODUCT
        );
        assert_eq!(
            validate_product_artifact(&root.join("fcitx5-register.exe"), &x64, 64),
            REGISTER_ARTIFACT_HELPER_LOCATION
        );
        assert_eq!(
            validate_product_artifact(&helper, &x64, 128),
            REGISTER_ARTIFACT_INVALID_ARGUMENT
        );
        let _ = fs::remove_dir_all(root);
    }
}
