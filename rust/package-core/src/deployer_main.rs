#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use fcitx5_package_core::{
    activate_staged_payload_tree, is_safe_relative_package_path, parse_trusted_keys, sha256_digest,
    stage_validated_archive_zip, TrustedKey,
};

const MAXIMUM_ARTIFACT_BYTES: u64 = 128 * 1024 * 1024;

fn version() -> &'static str {
    option_env!("FCITX_WINDOWS_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn main() {
    let exit_code = match run(std::env::args_os().collect()) {
        Ok(code) => code,
        Err(DeployError::Package { code, message }) => {
            eprintln!("{code}: {message}");
            2
        }
        Err(DeployError::Internal(message)) => {
            eprintln!("internal_error: {message}");
            3
        }
    };
    std::process::exit(exit_code);
}

fn run(args: Vec<OsString>) -> Result<i32, DeployError> {
    if args.len() == 2 && args[1] == "--version" {
        println!("fcitx5-deployer {}", version());
        return Ok(0);
    }
    if args.len() == 2 && args[1] == "--self-test" {
        return match protected_install_root() {
            Ok(_) => Ok(1),
            Err(DeployError::Package { .. }) => Ok(0),
            Err(error) => Err(error),
        };
    }
    if args.len() == 5 && args[1] == "--activate" {
        activate(
            &PathBuf::from(&args[2]),
            &unicode_arg(&args[3])?,
            &unicode_arg(&args[4])?,
        )?;
        return Ok(0);
    }
    eprintln!("Usage: fcitx5-deployer --activate LOCAL_ARCHIVE SHA256 TRANSACTION_ID");
    Ok(1)
}

fn activate(source: &Path, expected_hash: &str, transaction_id: &str) -> Result<(), DeployError> {
    if !win32::is_elevated()
        || !is_hex_digest(expected_hash)
        || !is_safe_relative_package_path(transaction_id)
        || transaction_id.contains('/')
        || transaction_id.contains('\\')
    {
        return Err(package_error(
            "privilege_boundary",
            "deployer request is invalid",
        ));
    }
    let root = protected_install_root()?;
    let keys = read_trusted_keys(root.join("security").join("trusted-keys.json"))?;
    let transactions = root.join(".transactions").join(transaction_id);
    if transactions.exists() {
        return Err(package_error(
            "transaction_exists",
            "protected transaction exists",
        ));
    }
    std::fs::create_dir_all(&transactions).map_err(|error| {
        internal_error(format!("transaction directory creation failed: {error}"))
    })?;
    let protected_archive = transactions.join("artifact.fcpkg");
    let activated = (|| {
        win32::copy_exclusive_artifact(source, &protected_archive)?;
        let bytes = std::fs::read(&protected_archive)
            .map_err(|error| package_error("io_error", format!("artifact read failed: {error}")))?;
        if sha256_digest(&bytes).as_str() != expected_hash.to_ascii_lowercase() {
            return Err(package_error(
                "hash_mismatch",
                "artifact changed across elevation boundary",
            ));
        }
        let staged = stage_validated_archive_zip(&protected_archive, &root, transaction_id, &keys)
            .map_err(|error| package_error(error.code(), error.to_string()))?;
        activate_staged_payload_tree(&staged, &root, &keys)
            .map_err(|error| package_error(error.code(), error.to_string()))?;
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&transactions);
    activated
}

fn protected_install_root() -> Result<PathBuf, DeployError> {
    let executable = win32::module_path()?;
    let root = executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| package_error("privilege_boundary", "module path is unavailable"))?
        .to_path_buf();
    let program_files = win32::program_files_path()?;
    let executable_name = executable
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let parent_name = executable
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !win32::ordinal_path_prefix(&root, &program_files)?
        || !executable_name.eq_ignore_ascii_case("fcitx5-deployer.exe")
        || !parent_name.eq_ignore_ascii_case("bin")
    {
        return Err(package_error(
            "privilege_boundary",
            "deployer must run from Program Files/Fcitx5/bin",
        ));
    }
    Ok(root)
}

fn read_trusted_keys(path: impl AsRef<Path>) -> Result<Vec<TrustedKey>, DeployError> {
    let bytes = std::fs::read(path).map_err(|error| {
        package_error(
            "invalid_keyring",
            format!("trusted keyring read failed: {error}"),
        )
    })?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| package_error("invalid_keyring", "trusted keyring is not UTF-8"))?;
    let keys =
        parse_trusted_keys(text).map_err(|error| package_error(error.code(), error.to_string()))?;
    if keys.is_empty() {
        return Err(package_error("invalid_keyring", "trusted keyring is empty"));
    }
    Ok(keys)
}

fn unicode_arg(value: &OsString) -> Result<String, DeployError> {
    value
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| internal_error("argument is not valid Unicode"))
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug)]
enum DeployError {
    Package { code: &'static str, message: String },
    Internal(String),
}

fn package_error(code: &'static str, message: impl Into<String>) -> DeployError {
    DeployError::Package {
        code,
        message: message.into(),
    }
}

fn internal_error(message: impl Into<String>) -> DeployError {
    DeployError::Internal(message.into())
}

#[cfg(windows)]
mod win32 {
    #![allow(unsafe_code)]

    use super::{internal_error, package_error, DeployError, MAXIMUM_ARTIFACT_BYTES};
    use std::ffi::c_void;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    type Handle = *mut c_void;

    const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_ELEVATION_CLASS: u32 = 20;
    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const OPEN_EXISTING: u32 = 3;
    const CREATE_NEW: u32 = 1;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_WRITE_THROUGH: u32 = 0x8000_0000;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    const FILE_ATTRIBUTE_TAG_INFO_CLASS: u32 = 9;
    const CSIDL_PROGRAM_FILES: i32 = 0x0026;
    const SHGFP_TYPE_CURRENT: u32 = 0;
    const CSTR_EQUAL: i32 = 2;

    #[repr(C)]
    struct TokenElevation {
        token_is_elevated: u32,
    }

    #[repr(C)]
    struct FileAttributeTagInfo {
        file_attributes: u32,
        reparse_tag: u32,
    }

    #[repr(C)]
    struct LargeInteger {
        quad_part: i64,
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn CloseHandle(object: Handle) -> i32;
        fn GetCurrentProcess() -> Handle;
        fn OpenProcessToken(process: Handle, desired_access: u32, token: *mut Handle) -> i32;
        fn GetTokenInformation(
            token: Handle,
            token_information_class: u32,
            token_information: *mut c_void,
            token_information_length: u32,
            return_length: *mut u32,
        ) -> i32;
        fn GetModuleFileNameW(module: Handle, filename: *mut u16, size: u32) -> u32;
        fn SHGetFolderPathW(
            hwnd: Handle,
            csidl: i32,
            token: Handle,
            flags: u32,
            path: *mut u16,
        ) -> i32;
        fn CompareStringOrdinal(
            string1: *const u16,
            count1: i32,
            string2: *const u16,
            count2: i32,
            ignore_case: i32,
        ) -> i32;
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: Handle,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: Handle,
        ) -> Handle;
        fn GetFileInformationByHandleEx(
            file: Handle,
            file_information_class: u32,
            file_information: *mut c_void,
            buffer_size: u32,
        ) -> i32;
        fn GetFileSizeEx(file: Handle, file_size: *mut LargeInteger) -> i32;
        fn ReadFile(
            file: Handle,
            buffer: *mut c_void,
            bytes_to_read: u32,
            bytes_read: *mut u32,
            overlapped: Handle,
        ) -> i32;
        fn WriteFile(
            file: Handle,
            buffer: *const c_void,
            bytes_to_write: u32,
            bytes_written: *mut u32,
            overlapped: Handle,
        ) -> i32;
        fn FlushFileBuffers(file: Handle) -> i32;
    }

    struct OwnedHandle(Handle);

    impl OwnedHandle {
        fn new(handle: Handle) -> Result<Self, DeployError> {
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                Err(package_error(
                    "artifact_changed",
                    "artifact cannot be opened exclusively",
                ))
            } else {
                Ok(Self(handle))
            }
        }

        fn raw(&self) -> Handle {
            self.0
        }
    }

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
                unsafe {
                    let _ = CloseHandle(self.0);
                }
            }
        }
    }

    pub fn is_elevated() -> bool {
        let mut token = std::ptr::null_mut();
        let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
        let Ok(token) = OwnedHandle::new(token) else {
            return false;
        };
        if opened == 0 {
            return false;
        }
        let mut elevation = TokenElevation {
            token_is_elevated: 0,
        };
        let mut returned = 0;
        let ok = unsafe {
            GetTokenInformation(
                token.raw(),
                TOKEN_ELEVATION_CLASS,
                &mut elevation as *mut _ as *mut c_void,
                std::mem::size_of::<TokenElevation>() as u32,
                &mut returned,
            )
        };
        ok != 0 && elevation.token_is_elevated != 0
    }

    pub fn module_path() -> Result<PathBuf, DeployError> {
        let mut buffer = vec![0u16; 32768];
        let length = unsafe {
            GetModuleFileNameW(
                std::ptr::null_mut(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
            )
        };
        if length == 0 || length as usize >= buffer.len() {
            return Err(package_error(
                "privilege_boundary",
                "module path is unavailable",
            ));
        }
        buffer.truncate(length as usize);
        Ok(PathBuf::from(std::ffi::OsString::from_wide(&buffer)))
    }

    pub fn program_files_path() -> Result<PathBuf, DeployError> {
        let mut buffer = vec![0u16; 260];
        let status = unsafe {
            SHGetFolderPathW(
                std::ptr::null_mut(),
                CSIDL_PROGRAM_FILES,
                std::ptr::null_mut(),
                SHGFP_TYPE_CURRENT,
                buffer.as_mut_ptr(),
            )
        };
        if status != 0 {
            return Err(package_error(
                "privilege_boundary",
                "Program Files path is unavailable",
            ));
        }
        let len = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        buffer.truncate(len);
        Ok(PathBuf::from(std::ffi::OsString::from_wide(&buffer)))
    }

    pub fn ordinal_path_prefix(candidate: &Path, parent: &Path) -> Result<bool, DeployError> {
        let candidate = std::fs::canonicalize(candidate).map_err(|error| {
            internal_error(format!("candidate path canonicalization failed: {error}"))
        })?;
        let mut parent = std::fs::canonicalize(parent).map_err(|error| {
            internal_error(format!("parent path canonicalization failed: {error}"))
        })?;
        parent.push("");
        let candidate_wide = wide_path(&candidate);
        let parent_wide = wide_path(&parent);
        if candidate_wide.len() <= parent_wide.len() {
            return Ok(false);
        }
        let result = unsafe {
            CompareStringOrdinal(
                candidate_wide.as_ptr(),
                parent_wide.len() as i32,
                parent_wide.as_ptr(),
                parent_wide.len() as i32,
                1,
            )
        };
        Ok(result == CSTR_EQUAL)
    }

    pub fn copy_exclusive_artifact(source: &Path, destination: &Path) -> Result<(), DeployError> {
        let input = OwnedHandle::new(unsafe {
            CreateFileW(
                wide_nul(source).as_ptr(),
                GENERIC_READ,
                0,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
                std::ptr::null_mut(),
            )
        })?;
        let mut tag = FileAttributeTagInfo {
            file_attributes: 0,
            reparse_tag: 0,
        };
        let mut size = LargeInteger { quad_part: 0 };
        let tag_ok = unsafe {
            GetFileInformationByHandleEx(
                input.raw(),
                FILE_ATTRIBUTE_TAG_INFO_CLASS,
                &mut tag as *mut _ as *mut c_void,
                std::mem::size_of::<FileAttributeTagInfo>() as u32,
            )
        };
        let size_ok = unsafe { GetFileSizeEx(input.raw(), &mut size) };
        if tag_ok == 0
            || size_ok == 0
            || (tag.file_attributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0
            || size.quad_part <= 0
            || size.quad_part as u64 > MAXIMUM_ARTIFACT_BYTES
        {
            return Err(package_error(
                "artifact_changed",
                "artifact identity or size is unsafe",
            ));
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                internal_error(format!("destination directory creation failed: {error}"))
            })?;
        }
        let output = OwnedHandle::new(unsafe {
            CreateFileW(
                wide_nul(destination).as_ptr(),
                GENERIC_WRITE,
                0,
                std::ptr::null_mut(),
                CREATE_NEW,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH,
                std::ptr::null_mut(),
            )
        })
        .map_err(|_| package_error("io_error", "protected artifact copy cannot be created"))?;
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let mut read = 0;
            let read_ok = unsafe {
                ReadFile(
                    input.raw(),
                    buffer.as_mut_ptr() as *mut c_void,
                    buffer.len() as u32,
                    &mut read,
                    std::ptr::null_mut(),
                )
            };
            if read_ok == 0 {
                return Err(package_error("io_error", "artifact read failed"));
            }
            if read == 0 {
                break;
            }
            let mut written = 0;
            let write_ok = unsafe {
                WriteFile(
                    output.raw(),
                    buffer.as_ptr() as *const c_void,
                    read,
                    &mut written,
                    std::ptr::null_mut(),
                )
            };
            if write_ok == 0 || written != read {
                return Err(package_error("io_error", "protected artifact copy failed"));
            }
        }
        let flushed = unsafe { FlushFileBuffers(output.raw()) };
        if flushed == 0 {
            return Err(package_error("io_error", "protected artifact flush failed"));
        }
        Ok(())
    }

    fn wide_path(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().collect()
    }

    fn wide_nul(path: &Path) -> Vec<u16> {
        let mut value = wide_path(path);
        value.push(0);
        value
    }
}
