#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

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

fn path_from_utf16(value: Fcitx5RegisterUtf16) -> Option<PathBuf> {
    if value.ptr.is_null() {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    Some(PathBuf::from(OsString::from_wide(slice)))
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
