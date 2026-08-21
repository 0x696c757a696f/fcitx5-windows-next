#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

const MANIFEST_FORMAT_VERSION_V1: u64 = 1;
const MANIFEST_FORMAT_VERSION_V2: u64 = 2;
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_PACKAGE_ID_BYTES: usize = 64;
const MAX_VERSION_BYTES: usize = 64;
const MAX_METADATA_BYTES: usize = 256;
const MAX_PACKAGE_PATH_BYTES: usize = 512;
const MAX_DEPENDENCY_COUNT: usize = 256;
const MAX_PERMISSION_COUNT: usize = 32;
const MAX_FILE_COUNT: usize = 4096;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PAYLOAD_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SIGNATURE_BYTES: u64 = 16 * 1024;
const SUPPORTED_CORE_API: &str = "1";
const SUPPORTED_ADDON_ABI: &str = "1";
const SIGNATURE_ENVELOPE_CANONICALIZATION: &str = "fcitx5-windows-next-json-v1";
const MLDSA65_PUBLIC_KEY_BYTES: usize = 1952;
const MLDSA65_SIGNATURE_BYTES: usize = 3309;
const SLHDSA_SHA2_128S_PUBLIC_KEY_BYTES: usize = 32;
const RSA_PUBLIC_MAGIC: u32 = 0x3141_5352;

#[cfg(windows)]
mod mldsa_verify_adapter {
    #![allow(unsafe_code)]

    use std::ffi::c_int;

    unsafe extern "C" {
        fn fcitx5_mldsa65_verify(
            signature: *const u8,
            message: *const u8,
            message_len: usize,
            context: *const u8,
            context_len: usize,
            public_key: *const u8,
        ) -> c_int;
    }

    pub fn verify(signature: &[u8], message: &[u8], public_key: &[u8]) -> bool {
        if signature.is_empty() || message.is_empty() || public_key.is_empty() {
            return false;
        }
        let status = unsafe {
            fcitx5_mldsa65_verify(
                signature.as_ptr(),
                message.as_ptr(),
                message.len(),
                std::ptr::null(),
                0,
                public_key.as_ptr(),
            )
        };
        status == 0
    }
}

#[cfg(windows)]
mod rsa_verify_adapter {
    #![allow(unsafe_code)]

    use std::ffi::{c_int, c_void};

    unsafe extern "system" {
        fn BCryptOpenAlgorithmProvider(
            ph_algorithm: *mut *mut c_void,
            psz_alg_id: *const u16,
            psz_implementation: *const u16,
            dw_flags: u32,
        ) -> c_int;
        fn BCryptCloseAlgorithmProvider(h_algorithm: *mut c_void, dw_flags: u32) -> c_int;
        fn BCryptImportKeyPair(
            h_algorithm: *mut c_void,
            h_import_key: *mut c_void,
            psz_blob_type: *const u16,
            ph_key: *mut *mut c_void,
            pb_input: *mut u8,
            cb_input: u32,
            dw_flags: u32,
        ) -> c_int;
        fn BCryptDestroyKey(h_key: *mut c_void) -> c_int;
        fn BCryptVerifySignature(
            h_key: *mut c_void,
            p_padding_info: *const c_void,
            pb_hash: *const u8,
            cb_hash: u32,
            pb_signature: *const u8,
            cb_signature: u32,
            dw_flags: u32,
        ) -> c_int;
    }

    const BCRYPT_RSA_ALGORITHM: &[u16] = &[b'R' as u16, b'S' as u16, b'A' as u16, 0];
    const BCRYPT_RSAPUBLIC_BLOB: &[u16] = &[
        b'R' as u16,
        b'S' as u16,
        b'A' as u16,
        b'P' as u16,
        b'U' as u16,
        b'B' as u16,
        b'L' as u16,
        b'I' as u16,
        b'C' as u16,
        b'B' as u16,
        b'L' as u16,
        b'O' as u16,
        b'B' as u16,
        0,
    ];
    const BCRYPT_SHA256_ALGORITHM: &[u16] = &[
        b'S' as u16,
        b'H' as u16,
        b'A' as u16,
        b'2' as u16,
        b'5' as u16,
        b'6' as u16,
        0,
    ];
    const BCRYPT_PAD_PKCS1: u32 = 0x0000_0002;

    #[repr(C)]
    struct BcryptPkcs1PaddingInfo {
        psz_alg_id: *const u16,
    }

    pub fn verify(signature: &[u8], message: &[u8], public_key: &[u8]) -> bool {
        if signature.is_empty() || message.is_empty() || public_key.is_empty() {
            return false;
        }
        let mut algorithm = std::ptr::null_mut();
        let opened = unsafe {
            BCryptOpenAlgorithmProvider(
                &mut algorithm,
                BCRYPT_RSA_ALGORITHM.as_ptr(),
                std::ptr::null(),
                0,
            )
        };
        if opened != 0 || algorithm.is_null() {
            return false;
        }
        let mut key = std::ptr::null_mut();
        let imported = unsafe {
            BCryptImportKeyPair(
                algorithm,
                std::ptr::null_mut(),
                BCRYPT_RSAPUBLIC_BLOB.as_ptr(),
                &mut key,
                public_key.as_ptr() as *mut u8,
                public_key.len() as u32,
                0,
            )
        };
        if imported != 0 || key.is_null() {
            let _ = unsafe { BCryptCloseAlgorithmProvider(algorithm, 0) };
            return false;
        }
        let hash = super::sha256_bytes(message);
        let padding = BcryptPkcs1PaddingInfo {
            psz_alg_id: BCRYPT_SHA256_ALGORITHM.as_ptr(),
        };
        let status = unsafe {
            BCryptVerifySignature(
                key,
                &padding as *const _ as *const c_void,
                hash.as_ptr(),
                hash.len() as u32,
                signature.as_ptr(),
                signature.len() as u32,
                BCRYPT_PAD_PKCS1,
            )
        };
        let _ = unsafe { BCryptDestroyKey(key) };
        let _ = unsafe { BCryptCloseAlgorithmProvider(algorithm, 0) };
        status == 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryDependency {
    id: String,
    version: String,
}

impl RepositoryDependency {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryEntry {
    id: String,
    title: String,
    summary: String,
    version: String,
    release_sequence: u64,
    package_type: PackageType,
    architecture: String,
    download_url: String,
    sha256: HexDigest32,
    dependencies: Vec<RepositoryDependency>,
}

impl RepositoryEntry {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn release_sequence(&self) -> u64 {
        self.release_sequence
    }

    pub fn package_type(&self) -> &PackageType {
        &self.package_type
    }

    pub fn architecture(&self) -> &str {
        &self.architecture
    }

    pub fn download_url(&self) -> &str {
        &self.download_url
    }

    pub fn sha256(&self) -> &HexDigest32 {
        &self.sha256
    }

    pub fn dependencies(&self) -> &[RepositoryDependency] {
        &self.dependencies
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryIndex {
    format_version: u64,
    channel: String,
    generated_at: String,
    key_id: String,
    packages: Vec<RepositoryEntry>,
}

impl RepositoryIndex {
    pub fn format_version(&self) -> u64 {
        self.format_version
    }

    pub fn channel(&self) -> &str {
        &self.channel
    }

    pub fn generated_at(&self) -> &str {
        &self.generated_at
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn packages(&self) -> &[RepositoryEntry] {
        &self.packages
    }
}

#[cfg(windows)]
mod miniz_archive_adapter {
    #![allow(unsafe_code)]

    use super::{archive_error, ArchiveEntry, ArchiveError, MAX_ARCHIVE_BYTES};
    use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    #[repr(C)]
    struct Fcitx5MinizEntry {
        name: [c_char; 512],
        uncompressed_size: u64,
        directory: c_uint,
        encrypted: c_uint,
        supported: c_uint,
        unix_symlink: c_uint,
    }

    unsafe extern "C" {
        fn fcitx5_miniz_open_utf16(
            path: *const u16,
            maximum_archive_bytes: u64,
            out_archive: *mut *mut c_void,
        ) -> c_int;
        fn fcitx5_miniz_close(archive: *mut c_void);
        fn fcitx5_miniz_num_files(archive: *mut c_void) -> c_uint;
        fn fcitx5_miniz_stat(
            archive: *mut c_void,
            index: c_uint,
            out_entry: *mut Fcitx5MinizEntry,
        ) -> c_int;
        fn fcitx5_miniz_locate(
            archive: *mut c_void,
            name: *const c_char,
            out_index: *mut c_uint,
        ) -> c_int;
        fn fcitx5_miniz_validate(archive: *mut c_void, index: c_uint) -> c_int;
        fn fcitx5_miniz_extract(
            archive: *mut c_void,
            index: c_uint,
            output: *mut u8,
            output_size: usize,
        ) -> c_int;
    }

    pub struct ZipArchive {
        handle: *mut c_void,
    }

    impl ZipArchive {
        pub fn open(path: &Path) -> Result<Self, ArchiveError> {
            let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
            if wide_path.is_empty() {
                return Err(archive_error(
                    "invalid_archive",
                    "unable to open package archive",
                ));
            }
            wide_path.push(0);
            let mut handle = std::ptr::null_mut();
            let opened = unsafe {
                fcitx5_miniz_open_utf16(wide_path.as_ptr(), MAX_ARCHIVE_BYTES, &mut handle)
            };
            if opened == 0 || handle.is_null() {
                return Err(archive_error(
                    "invalid_archive",
                    "ZIP central directory is invalid",
                ));
            }
            Ok(Self { handle })
        }

        pub fn len(&self) -> usize {
            unsafe { fcitx5_miniz_num_files(self.handle) as usize }
        }

        pub fn stat(&self, index: usize) -> Result<ArchiveEntry, ArchiveError> {
            let mut raw = Fcitx5MinizEntry {
                name: [0; 512],
                uncompressed_size: 0,
                directory: 0,
                encrypted: 0,
                supported: 0,
                unix_symlink: 0,
            };
            let ok = unsafe { fcitx5_miniz_stat(self.handle, index as c_uint, &mut raw) };
            if ok == 0 {
                return Err(archive_error(
                    "invalid_archive",
                    "archive entry metadata is invalid",
                ));
            }
            let name = unsafe { CStr::from_ptr(raw.name.as_ptr()) }
                .to_str()
                .map_err(|_| {
                    archive_error("unsafe_archive_path", "archive path is not valid UTF-8")
                })?
                .to_owned();
            Ok(ArchiveEntry {
                name,
                uncompressed_size: raw.uncompressed_size,
                directory: raw.directory != 0,
                encrypted: raw.encrypted != 0,
                supported: raw.supported != 0,
                unix_symlink: raw.unix_symlink != 0,
            })
        }

        pub fn locate(&self, name: &str) -> Result<usize, ArchiveError> {
            let name = CString::new(name).map_err(|_| {
                archive_error("unsafe_archive_path", "archive path contains a NUL byte")
            })?;
            let mut index = 0;
            let ok = unsafe { fcitx5_miniz_locate(self.handle, name.as_ptr(), &mut index) };
            if ok == 0 {
                return Err(archive_error(
                    "invalid_archive",
                    "required archive entry is missing",
                ));
            }
            Ok(index as usize)
        }

        pub fn validate(&self, index: usize) -> Result<(), ArchiveError> {
            let ok = unsafe { fcitx5_miniz_validate(self.handle, index as c_uint) };
            if ok == 0 {
                return Err(archive_error(
                    "invalid_archive",
                    "archive entry integrity validation failed",
                ));
            }
            Ok(())
        }

        pub fn extract(&self, index: usize, maximum_size: u64) -> Result<Vec<u8>, ArchiveError> {
            let entry = self.stat(index)?;
            if entry.directory
                || entry.encrypted
                || !entry.supported
                || entry.unix_symlink
                || entry.uncompressed_size > maximum_size
                || entry.uncompressed_size > usize::MAX as u64
            {
                return Err(archive_error(
                    "invalid_archive",
                    "archive entry violates type or size constraints",
                ));
            }
            let mut output = vec![0_u8; entry.uncompressed_size as usize];
            let ok = unsafe {
                fcitx5_miniz_extract(
                    self.handle,
                    index as c_uint,
                    output.as_mut_ptr(),
                    output.len(),
                )
            };
            if ok == 0 {
                return Err(archive_error(
                    "invalid_archive",
                    "archive entry failed integrity validation",
                ));
            }
            Ok(output)
        }
    }

    impl Drop for ZipArchive {
        fn drop(&mut self) {
            unsafe { fcitx5_miniz_close(self.handle) };
        }
    }
}

#[cfg(windows)]
mod win32_fs_adapter {
    #![allow(unsafe_code)]

    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    unsafe extern "system" {
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    pub fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
        let mut source_wide: Vec<u16> = source.as_os_str().encode_wide().collect();
        let mut destination_wide: Vec<u16> = destination.as_os_str().encode_wide().collect();
        source_wide.push(0);
        destination_wide.push(0);
        let ok = unsafe {
            MoveFileExW(
                source_wide.as_ptr(),
                destination_wide.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn publish_directory(source: &Path, destination: &Path) -> io::Result<()> {
        let mut source_wide: Vec<u16> = source.as_os_str().encode_wide().collect();
        let mut destination_wide: Vec<u16> = destination.as_os_str().encode_wide().collect();
        source_wide.push(0);
        destination_wide.push(0);
        let ok = unsafe {
            MoveFileExW(
                source_wide.as_ptr(),
                destination_wide.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PackageId(String);

impl PackageId {
    pub fn parse(value: &str) -> Result<Self, PackageIdError> {
        let bytes = value.as_bytes();
        if bytes.is_empty()
            || bytes.len() > MAX_PACKAGE_ID_BYTES
            || !bytes[0].is_ascii_lowercase()
            || !bytes.iter().all(|&byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'-' | b'_')
            })
        {
            return Err(PackageIdError);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageIdError;

impl fmt::Display for PackageIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid package id")
    }
}

impl std::error::Error for PackageIdError {}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SafeRelativePackagePath(String);

impl SafeRelativePackagePath {
    pub fn parse(value: &str) -> Result<Self, PackagePathError> {
        validate_package_path(value)?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackagePathError {
    Empty,
    TooLong,
    AbsoluteOrRooted,
    BackslashSeparator,
    AlternateDataStream,
    EmptyComponent,
    DotComponent,
    ParentComponent,
    TrailingDotOrSpace,
    ControlCharacter,
    DosDeviceComponent,
}

impl fmt::Display for PackagePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "empty path",
            Self::TooLong => "path exceeds package path budget",
            Self::AbsoluteOrRooted => "absolute or rooted path",
            Self::BackslashSeparator => "backslash separator",
            Self::AlternateDataStream => "alternate data stream marker",
            Self::EmptyComponent => "empty path component",
            Self::DotComponent => "dot path component",
            Self::ParentComponent => "parent path component",
            Self::TrailingDotOrSpace => "component ends with dot or space",
            Self::ControlCharacter => "control character in path component",
            Self::DosDeviceComponent => "DOS device path component",
        })
    }
}

impl std::error::Error for PackagePathError {}

pub fn is_safe_relative_package_path(value: &str) -> bool {
    validate_package_path(value).is_ok()
}

fn validate_package_path(value: &str) -> Result<(), PackagePathError> {
    if value.is_empty() {
        return Err(PackagePathError::Empty);
    }
    if value.len() > MAX_PACKAGE_PATH_BYTES {
        return Err(PackagePathError::TooLong);
    }
    if value.starts_with('/')
        || value.starts_with('\\')
        || value.ends_with('/')
        || value.ends_with('\\')
    {
        return Err(PackagePathError::AbsoluteOrRooted);
    }
    if value.contains('\\') {
        return Err(PackagePathError::BackslashSeparator);
    }
    if value.contains(':') {
        return Err(PackagePathError::AlternateDataStream);
    }
    if value.as_bytes().contains(&0) {
        return Err(PackagePathError::ControlCharacter);
    }

    for component in value.split('/') {
        validate_component(component)?;
    }
    Ok(())
}

fn validate_component(component: &str) -> Result<(), PackagePathError> {
    if component.is_empty() {
        return Err(PackagePathError::EmptyComponent);
    }
    if component == "." {
        return Err(PackagePathError::DotComponent);
    }
    if component == ".." {
        return Err(PackagePathError::ParentComponent);
    }
    if component.ends_with('.') || component.ends_with(' ') {
        return Err(PackagePathError::TrailingDotOrSpace);
    }
    if component.bytes().any(|byte| byte < 0x20) {
        return Err(PackagePathError::ControlCharacter);
    }

    let stem = component
        .split_once('.')
        .map_or(component, |(stem, _)| stem);
    let lowered = stem.to_ascii_lowercase();
    if matches!(lowered.as_str(), "con" | "prn" | "aux" | "nul")
        || (lowered.len() == 4
            && ((lowered.starts_with("com") && matches!(lowered.as_bytes()[3], b'1'..=b'9'))
                || (lowered.starts_with("lpt") && matches!(lowered.as_bytes()[3], b'1'..=b'9'))))
    {
        return Err(PackagePathError::DosDeviceComponent);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HexDigest32(String);

impl HexDigest32 {
    pub fn parse(value: &str) -> Result<Self, HexDigestError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(HexDigestError);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HexDigestError;

impl fmt::Display for HexDigestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid 32-byte hex digest")
    }
}

impl std::error::Error for HexDigestError {}

pub fn sha256_digest(bytes: &[u8]) -> HexDigest32 {
    HexDigest32(hex_lower(&sha256_bytes(bytes)))
}

pub fn blake3_digest(bytes: &[u8]) -> HexDigest32 {
    HexDigest32(blake3::hash(bytes).to_hex().to_string())
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    const INITIAL_STATE: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const ROUND_CONSTANTS: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut state = INITIAL_STATE;
    let mut block = [0_u8; 64];
    let block_count = bytes.len() / 64;
    for index in 0..block_count {
        block.copy_from_slice(&bytes[index * 64..index * 64 + 64]);
        sha256_compress(&mut state, &block, &ROUND_CONSTANTS);
    }

    let remainder = &bytes[block_count * 64..];
    block = [0_u8; 64];
    block[..remainder.len()].copy_from_slice(remainder);
    block[remainder.len()] = 0x80;
    if remainder.len() >= 56 {
        sha256_compress(&mut state, &block, &ROUND_CONSTANTS);
        block = [0_u8; 64];
    }
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    block[56..64].copy_from_slice(&bit_len.to_be_bytes());
    sha256_compress(&mut state, &block, &ROUND_CONSTANTS);

    let mut digest = [0_u8; 32];
    for (index, word) in state.iter().enumerate() {
        digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
}

fn sha256_compress(state: &mut [u32; 8], block: &[u8; 64], round_constants: &[u32; 64]) {
    let mut schedule = [0_u32; 64];
    for index in 0..16 {
        schedule[index] = u32::from_be_bytes([
            block[index * 4],
            block[index * 4 + 1],
            block[index * 4 + 2],
            block[index * 4 + 3],
        ]);
    }
    for index in 16..64 {
        let s0 = schedule[index - 15].rotate_right(7)
            ^ schedule[index - 15].rotate_right(18)
            ^ (schedule[index - 15] >> 3);
        let s1 = schedule[index - 2].rotate_right(17)
            ^ schedule[index - 2].rotate_right(19)
            ^ (schedule[index - 2] >> 10);
        schedule[index] = schedule[index - 16]
            .wrapping_add(s0)
            .wrapping_add(schedule[index - 7])
            .wrapping_add(s1);
    }

    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    let mut f = state[5];
    let mut g = state[6];
    let mut h = state[7];

    for index in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choice = (e & f) ^ ((!e) & g);
        let temp1 = h
            .wrapping_add(s1)
            .wrapping_add(choice)
            .wrapping_add(round_constants[index])
            .wrapping_add(schedule[index]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0.wrapping_add(majority);

        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

fn hex_lower(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadHashes {
    blake3: Option<HexDigest32>,
    sha256: Option<HexDigest32>,
}

impl PayloadHashes {
    pub fn v1_sha256(sha256: HexDigest32) -> Self {
        Self {
            blake3: None,
            sha256: Some(sha256),
        }
    }

    pub fn v2_blake3(blake3: HexDigest32, sha256: Option<HexDigest32>) -> Self {
        Self {
            blake3: Some(blake3),
            sha256,
        }
    }

    pub fn blake3(&self) -> Option<&HexDigest32> {
        self.blake3.as_ref()
    }

    pub fn sha256(&self) -> Option<&HexDigest32> {
        self.sha256.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedArtifact {
    package_id: PackageId,
    path: SafeRelativePackagePath,
    size: u64,
    hashes: PayloadHashes,
}

impl VerifiedArtifact {
    pub fn new(
        package_id: PackageId,
        path: SafeRelativePackagePath,
        size: u64,
        hashes: PayloadHashes,
    ) -> Self {
        Self {
            package_id,
            path,
            size,
            hashes,
        }
    }

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    pub fn path(&self) -> &SafeRelativePackagePath {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn hashes(&self) -> &PayloadHashes {
        &self.hashes
    }
}

pub fn has_windows_ordinal_case_collision<'a>(
    paths: impl IntoIterator<Item = &'a SafeRelativePackagePath>,
) -> bool {
    let mut seen = BTreeSet::new();
    for path in paths {
        let key = path.as_str().to_lowercase();
        if !seen.insert(key) {
            return true;
        }
    }
    false
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageType {
    Core,
    Addon,
    InputMethodData,
    Theme,
    Translation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dependency {
    id: PackageId,
    version: String,
}

impl Dependency {
    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manifest {
    format_version: u64,
    id: PackageId,
    version: String,
    package_type: PackageType,
    architecture: String,
    min_os: String,
    core_api: String,
    addon_abi: String,
    dependencies: Vec<Dependency>,
    license: String,
    source_commit: String,
    permissions: Vec<String>,
    files: Vec<VerifiedArtifact>,
    key_id: PackageId,
}

impl Manifest {
    pub fn format_version(&self) -> u64 {
        self.format_version
    }

    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn package_type(&self) -> &PackageType {
        &self.package_type
    }

    pub fn architecture(&self) -> &str {
        &self.architecture
    }

    pub fn min_os(&self) -> &str {
        &self.min_os
    }

    pub fn core_api(&self) -> &str {
        &self.core_api
    }

    pub fn addon_abi(&self) -> &str {
        &self.addon_abi
    }

    pub fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }

    pub fn license(&self) -> &str {
        &self.license
    }

    pub fn source_commit(&self) -> &str {
        &self.source_commit
    }

    pub fn permissions(&self) -> &[String] {
        &self.permissions
    }

    pub fn files(&self) -> &[VerifiedArtifact] {
        &self.files
    }

    pub fn key_id(&self) -> &PackageId {
        &self.key_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestError {
    code: &'static str,
    message: String,
}

impl ManifestError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ManifestError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityError {
    code: &'static str,
    message: String,
}

impl CompatibilityError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for CompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CompatibilityError {}

pub fn parse_manifest(bytes: &str) -> Result<Manifest, ManifestError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(manifest_error(
            "invalid_manifest",
            "manifest size is outside the accepted range",
        ));
    }

    let document = JsonParser::new(bytes)
        .parse()
        .map_err(|message| manifest_error("invalid_manifest", message))?;
    let object = document
        .as_object()
        .ok_or_else(|| manifest_error("invalid_manifest", "expected a JSON object"))?;
    require_object_keys(
        object,
        &[
            "format_version",
            "id",
            "version",
            "type",
            "architecture",
            "min_os",
            "core_api",
            "addon_abi",
            "dependencies",
            "license",
            "source_commit",
            "permissions",
            "key_id",
        ],
        &["files", "payload"],
    )?;

    let format_version = require_unsigned(object, "format_version", "unsupported_manifest")?;
    if format_version != MANIFEST_FORMAT_VERSION_V1 && format_version != MANIFEST_FORMAT_VERSION_V2
    {
        return Err(manifest_error(
            "unsupported_manifest",
            "format_version is unsupported",
        ));
    }
    if (format_version == MANIFEST_FORMAT_VERSION_V1
        && (!object_contains(object, "files") || object_contains(object, "payload")))
        || (format_version == MANIFEST_FORMAT_VERSION_V2
            && (!object_contains(object, "payload") || object_contains(object, "files")))
    {
        return Err(manifest_error(
            "invalid_manifest",
            "manifest payload schema does not match format version",
        ));
    }

    let id = PackageId::parse(&require_string(object, "id", MAX_PACKAGE_ID_BYTES, false)?)
        .map_err(|_| manifest_error("invalid_manifest", "package identity is invalid"))?;
    let version = require_ascii_token_string(object, "version", MAX_VERSION_BYTES, ".+-_")?;
    let package_type = parse_package_type(&require_string(object, "type", 32, false)?)?;
    let architecture = require_string(object, "architecture", 8, false)?;
    if !matches!(architecture.as_str(), "any" | "x86" | "x64") {
        return Err(manifest_error(
            "invalid_manifest",
            "package architecture is invalid",
        ));
    }
    let min_os = require_string(object, "min_os", 32, false)?;
    let core_api = require_string(object, "core_api", MAX_VERSION_BYTES, false)?;
    let addon_abi = require_string(object, "addon_abi", MAX_VERSION_BYTES, true)?;
    let license = require_string(object, "license", MAX_METADATA_BYTES, false)?;
    let source_commit = require_string(object, "source_commit", 128, false)?;
    let key_id = PackageId::parse(&require_string(
        object,
        "key_id",
        MAX_PACKAGE_ID_BYTES,
        false,
    )?)
    .map_err(|_| manifest_error("invalid_manifest", "package key id is invalid"))?;

    let dependencies = parse_dependencies(require_array(object, "dependencies")?)?;
    let permissions = parse_permissions(require_array(object, "permissions")?)?;
    let files = parse_manifest_files(format_version, &id, object)?;

    Ok(Manifest {
        format_version,
        id,
        version,
        package_type,
        architecture,
        min_os,
        core_api,
        addon_abi,
        dependencies,
        license,
        source_commit,
        permissions,
        files,
        key_id,
    })
}

pub fn validate_manifest_compatibility(
    manifest: &Manifest,
    architecture: &str,
) -> Result<(), CompatibilityError> {
    if !matches!(architecture, "x64" | "x86") {
        return Err(compatibility_error("runtime architecture is invalid"));
    }
    if manifest.architecture() != "any" && manifest.architecture() != architecture {
        return Err(compatibility_error(
            "package architecture does not match this runtime",
        ));
    }
    if manifest.core_api() != SUPPORTED_CORE_API {
        return Err(compatibility_error(
            "package requires an unsupported Core API",
        ));
    }
    if manifest.package_type() == &PackageType::Addon && manifest.addon_abi() != SUPPORTED_ADDON_ABI
    {
        return Err(compatibility_error("addon ABI does not match this engine"));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrustAlgorithm {
    Rsa2048Sha256,
    Mldsa65,
    SlhdsaSha2_128s,
}

impl TrustAlgorithm {
    fn parse_for_keyring(value: &str, format_version: u64) -> Result<Self, KeyringError> {
        match value {
            "rsa-2048-sha256" => Ok(Self::Rsa2048Sha256),
            "mldsa65" if format_version == 2 => Ok(Self::Mldsa65),
            "slhdsa-sha2-128s" if format_version == 2 => Ok(Self::SlhdsaSha2_128s),
            _ => Err(keyring_error("trusted key algorithm is unsupported")),
        }
    }

    fn parse_for_signature(value: &str) -> Result<Self, SignatureEnvelopeError> {
        match value {
            "mldsa65" => Ok(Self::Mldsa65),
            "slhdsa-sha2-128s" => Ok(Self::SlhdsaSha2_128s),
            _ => Err(signature_error(
                "signature envelope requires an unsupported algorithm",
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rsa2048Sha256 => "rsa-2048-sha256",
            Self::Mldsa65 => "mldsa65",
            Self::SlhdsaSha2_128s => "slhdsa-sha2-128s",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedKey {
    id: PackageId,
    algorithm: TrustAlgorithm,
    public_key: Vec<u8>,
    revoked: bool,
}

impl TrustedKey {
    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn algorithm(&self) -> &TrustAlgorithm {
        &self.algorithm
    }

    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    pub fn revoked(&self) -> bool {
        self.revoked
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyringError {
    code: &'static str,
    message: String,
}

impl KeyringError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for KeyringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for KeyringError {}

pub fn parse_trusted_keys(bytes: &str) -> Result<Vec<TrustedKey>, KeyringError> {
    let document = JsonParser::new(bytes)
        .parse()
        .map_err(|_| keyring_error("trusted key file is not strict JSON"))?;
    let object = document
        .as_object()
        .ok_or_else(|| keyring_error("trusted key file schema is invalid"))?;
    if !object_contains(object, "format_version") {
        return Err(keyring_error("trusted key file schema is invalid"));
    }
    let format_version = object_get(object, "format_version")
        .and_then(JsonValue::as_number)
        .ok_or_else(|| keyring_error("trusted key file schema is invalid"))?;

    match format_version {
        1 => require_object_keys_with_code(
            object,
            &["format_version", "keys"],
            &[],
            "invalid_keyring",
        )?,
        2 => {
            require_object_keys_with_code(
                object,
                &["format_version", "policy", "keys"],
                &[],
                "invalid_keyring",
            )?;
            validate_keyring_policy(require_object_for_code(
                object,
                "policy",
                "invalid_keyring",
            )?)?;
        }
        _ => return Err(keyring_error("trusted key format version is unsupported")),
    }

    let keys = object_get(object, "keys")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| keyring_error("trusted key file schema is invalid"))?;
    if keys.len() > 64 {
        return Err(keyring_error("trusted key file schema is invalid"));
    }

    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for value in keys {
        let key = value
            .as_object()
            .ok_or_else(|| keyring_error("trusted key file schema is invalid"))?;
        if format_version == 1 {
            require_object_keys_with_code(
                key,
                &["key_id", "algorithm", "status", "public_key_base64"],
                &[],
                "invalid_keyring",
            )?;
        } else {
            require_object_keys_with_code(
                key,
                &[
                    "key_id",
                    "algorithm",
                    "status",
                    "public_key_base64",
                    "scope",
                    "channels",
                ],
                &[],
                "invalid_keyring",
            )?;
            validate_non_empty_bounded_array(key, "scope", 8)?;
            validate_non_empty_bounded_array(key, "channels", 16)?;
        }

        let id = PackageId::parse(&require_string_for_code(
            key,
            "key_id",
            MAX_PACKAGE_ID_BYTES,
            false,
            "invalid_keyring",
        )?)
        .map_err(|_| keyring_error("trusted key record is invalid"))?;
        let algorithm = TrustAlgorithm::parse_for_keyring(
            &require_string_for_code(key, "algorithm", 32, false, "invalid_keyring")?,
            format_version,
        )?;
        let status = require_string_for_code(key, "status", 16, false, "invalid_keyring")?;
        let public_key = decode_base64(&require_string_for_code(
            key,
            "public_key_base64",
            16384,
            false,
            "invalid_keyring",
        )?)
        .map_err(|_| keyring_error("base64 decoding failed"))?;

        if !matches!(status.as_str(), "trusted" | "revoked") || !ids.insert(id.as_str().to_owned())
        {
            return Err(keyring_error("trusted key record is invalid"));
        }
        validate_public_key(&algorithm, &public_key)?;
        result.push(TrustedKey {
            id,
            algorithm,
            public_key,
            revoked: status == "revoked",
        });
    }

    Ok(result)
}

fn validate_keyring_policy(policy: &[(String, JsonValue)]) -> Result<(), KeyringError> {
    require_object_keys_with_code(
        policy,
        &[
            "official_required_signatures",
            "compatibility_hashes",
            "default_payload_hash",
        ],
        &[],
        "invalid_keyring",
    )?;
    let official = object_get(policy, "official_required_signatures")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| keyring_error("trusted key policy is invalid"))?;
    let compatibility = object_get(policy, "compatibility_hashes")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| keyring_error("trusted key policy is invalid"))?;
    if official.len() > 8
        || compatibility.len() > 8
        || require_string_for_code(policy, "default_payload_hash", 32, false, "invalid_keyring")?
            != "blake3"
    {
        return Err(keyring_error("trusted key policy is invalid"));
    }
    for algorithm in official {
        let Some(value) = algorithm.as_string() else {
            return Err(keyring_error("trusted key policy algorithm is invalid"));
        };
        if !matches!(value, "mldsa65" | "slhdsa-sha2-128s") {
            return Err(keyring_error(
                "trusted key policy requires unsupported algorithm",
            ));
        }
    }
    Ok(())
}

fn validate_non_empty_bounded_array(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
) -> Result<(), KeyringError> {
    let array = object_get(object, key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| keyring_error("trusted key scope/channel policy is invalid"))?;
    if array.is_empty() || array.len() > maximum {
        return Err(keyring_error("trusted key scope/channel policy is invalid"));
    }
    Ok(())
}

fn validate_public_key(algorithm: &TrustAlgorithm, public_key: &[u8]) -> Result<(), KeyringError> {
    match algorithm {
        TrustAlgorithm::Rsa2048Sha256 => {
            if public_key.len() < 8 {
                return Err(keyring_error("RSA public key blob is truncated"));
            }
            let magic =
                u32::from_le_bytes([public_key[0], public_key[1], public_key[2], public_key[3]]);
            let bit_length =
                u32::from_le_bytes([public_key[4], public_key[5], public_key[6], public_key[7]]);
            if magic != RSA_PUBLIC_MAGIC || !(2048..=4096).contains(&bit_length) {
                return Err(keyring_error(
                    "RSA public key strength or representation is invalid",
                ));
            }
            Ok(())
        }
        TrustAlgorithm::Mldsa65 if public_key.len() == MLDSA65_PUBLIC_KEY_BYTES => Ok(()),
        TrustAlgorithm::Mldsa65 => Err(keyring_error("ML-DSA-65 public key length is invalid")),
        TrustAlgorithm::SlhdsaSha2_128s
            if public_key.len() == SLHDSA_SHA2_128S_PUBLIC_KEY_BYTES =>
        {
            Ok(())
        }
        TrustAlgorithm::SlhdsaSha2_128s => {
            Err(keyring_error("SLH-DSA public key length is invalid"))
        }
    }
}

fn keyring_error(message: impl Into<String>) -> KeyringError {
    KeyringError {
        code: "invalid_keyring",
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SignedObject {
    RepositoryIndex,
    PackageManifest,
}

impl SignedObject {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "repository-index" => Some(Self::RepositoryIndex),
            "package-manifest" => Some(Self::PackageManifest),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RepositoryIndex => "repository-index",
            Self::PackageManifest => "package-manifest",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureEnvelopeEntry {
    key_id: PackageId,
    algorithm: TrustAlgorithm,
    signature: Vec<u8>,
}

impl SignatureEnvelopeEntry {
    pub fn key_id(&self) -> &PackageId {
        &self.key_id
    }

    pub fn algorithm(&self) -> &TrustAlgorithm {
        &self.algorithm
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureEnvelope {
    format_version: u64,
    signed_object: SignedObject,
    canonicalization: String,
    signatures: Vec<SignatureEnvelopeEntry>,
}

impl SignatureEnvelope {
    pub fn format_version(&self) -> u64 {
        self.format_version
    }

    pub fn signed_object(&self) -> &SignedObject {
        &self.signed_object
    }

    pub fn canonicalization(&self) -> &str {
        &self.canonicalization
    }

    pub fn signatures(&self) -> &[SignatureEnvelopeEntry] {
        &self.signatures
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureEnvelopeError {
    code: &'static str,
    message: String,
}

impl SignatureEnvelopeError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for SignatureEnvelopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SignatureEnvelopeError {}

pub fn parse_signature_envelope(
    bytes: &str,
    expected_object: SignedObject,
) -> Result<SignatureEnvelope, SignatureEnvelopeError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(signature_error("signature envelope identity is invalid"));
    }
    let document = JsonParser::new(bytes)
        .parse()
        .map_err(|_| signature_error("signature envelope is not strict JSON"))?;
    let object = document
        .as_object()
        .ok_or_else(|| signature_error("signature envelope entry must be a JSON object"))?;
    require_object_keys_with_code(
        object,
        &[
            "format_version",
            "signed_object",
            "canonicalization",
            "signatures",
        ],
        &[],
        "invalid_signature",
    )
    .map_err(|error| signature_error(error.message))?;

    let format_version = object_get(object, "format_version")
        .and_then(JsonValue::as_number)
        .ok_or_else(|| signature_error("signature envelope format version is unsupported"))?;
    if format_version != 2 {
        return Err(signature_error(
            "signature envelope format version is unsupported",
        ));
    }

    let signed_object_name =
        require_signature_string(object, "signed_object", 64).map_err(signature_error)?;
    let signed_object = SignedObject::parse(&signed_object_name)
        .ok_or_else(|| signature_error("signature envelope object binding is invalid"))?;
    let canonicalization =
        require_signature_string(object, "canonicalization", 64).map_err(signature_error)?;
    if signed_object != expected_object || canonicalization != SIGNATURE_ENVELOPE_CANONICALIZATION {
        return Err(signature_error(
            "signature envelope object binding is invalid",
        ));
    }

    let signatures = object_get(object, "signatures")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| signature_error("signature envelope signatures array is invalid"))?;
    if signatures.is_empty() || signatures.len() > 16 {
        return Err(signature_error(
            "signature envelope signatures array is invalid",
        ));
    }

    let mut key_ids = BTreeSet::new();
    let mut has_required_mldsa65 = false;
    let mut parsed = Vec::new();
    for value in signatures {
        let entry = value
            .as_object()
            .ok_or_else(|| signature_error("signature envelope entry must be a JSON object"))?;
        require_object_keys_with_code(
            entry,
            &["key_id", "algorithm", "signature_base64"],
            &[],
            "invalid_signature",
        )
        .map_err(|error| signature_error(error.message))?;
        let key_id = PackageId::parse(
            &require_signature_string(entry, "key_id", MAX_PACKAGE_ID_BYTES)
                .map_err(signature_error)?,
        )
        .map_err(|_| signature_error("signature envelope key id is invalid or duplicated"))?;
        if !key_ids.insert(key_id.as_str().to_owned()) {
            return Err(signature_error(
                "signature envelope key id is invalid or duplicated",
            ));
        }
        let algorithm = TrustAlgorithm::parse_for_signature(
            &require_signature_string(entry, "algorithm", 32).map_err(signature_error)?,
        )?;
        if algorithm == TrustAlgorithm::Mldsa65 {
            has_required_mldsa65 = true;
        }
        let signature = decode_base64(
            &require_signature_string(entry, "signature_base64", 16384).map_err(signature_error)?,
        )
        .map_err(|_| signature_error("base64 decoding failed"))?;
        parsed.push(SignatureEnvelopeEntry {
            key_id,
            algorithm,
            signature,
        });
    }
    if !has_required_mldsa65 {
        return Err(signature_error(
            "signature envelope is missing required ML-DSA-65 signature",
        ));
    }

    Ok(SignatureEnvelope {
        format_version,
        signed_object,
        canonicalization,
        signatures: parsed,
    })
}

fn require_signature_string(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
) -> Result<String, String> {
    let value = object_get(object, key)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| format!("{key} must be a string"))?;
    if value.is_empty() || value.len() > maximum || value.contains('\0') {
        return Err(format!("{key} has an invalid length"));
    }
    Ok(value.to_owned())
}

fn signature_error(message: impl Into<String>) -> SignatureEnvelopeError {
    SignatureEnvelopeError {
        code: "invalid_signature",
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureVerificationError {
    code: &'static str,
    message: String,
}

impl SignatureVerificationError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for SignatureVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SignatureVerificationError {}

#[cfg(windows)]
pub fn verify_mldsa65_signature(
    object_bytes: &[u8],
    signature: &[u8],
    key: &TrustedKey,
) -> Result<(), SignatureVerificationError> {
    if key.revoked() {
        return Err(signature_verification_error(
            "revoked_key",
            "ML-DSA key is revoked",
        ));
    }
    if key.algorithm() != &TrustAlgorithm::Mldsa65
        || key.public_key().len() != MLDSA65_PUBLIC_KEY_BYTES
        || signature.len() != MLDSA65_SIGNATURE_BYTES
        || object_bytes.is_empty()
    {
        return Err(signature_verification_error(
            "invalid_signature",
            "ML-DSA signature identity is incomplete",
        ));
    }
    if !mldsa_verify_adapter::verify(signature, object_bytes, key.public_key()) {
        return Err(signature_verification_error(
            "invalid_signature",
            "ML-DSA-65 signature verification failed",
        ));
    }
    Ok(())
}

#[cfg(windows)]
pub fn verify_manifest_signature(
    object_bytes: &[u8],
    signature: &[u8],
    key: &TrustedKey,
) -> Result<(), SignatureVerificationError> {
    if key.revoked() {
        return Err(signature_verification_error(
            "revoked_key",
            "manifest key is revoked",
        ));
    }
    if key.algorithm() != &TrustAlgorithm::Rsa2048Sha256
        || key.public_key().is_empty()
        || signature.is_empty()
    {
        return Err(signature_verification_error(
            "invalid_signature",
            "signature identity is incomplete",
        ));
    }
    if !rsa_verify_adapter::verify(signature, object_bytes, key.public_key()) {
        return Err(signature_verification_error(
            "invalid_signature",
            "manifest signature verification failed",
        ));
    }
    Ok(())
}

#[cfg(windows)]
pub fn verify_signature_envelope(
    object_bytes: &[u8],
    envelope: &SignatureEnvelope,
    trusted_keys: &[TrustedKey],
    expected_object: SignedObject,
    expected_key_id: &PackageId,
) -> Result<(), SignatureVerificationError> {
    if envelope.format_version() != 2
        || envelope.signed_object() != &expected_object
        || envelope.canonicalization() != SIGNATURE_ENVELOPE_CANONICALIZATION
    {
        return Err(signature_verification_error(
            "invalid_signature",
            "signature envelope binding is invalid",
        ));
    }
    for entry in envelope.signatures() {
        if entry.algorithm() != &TrustAlgorithm::Mldsa65 {
            continue;
        }
        if entry.key_id() != expected_key_id {
            return Err(signature_verification_error(
                "untrusted_key",
                "ML-DSA signature key id does not match signed metadata",
            ));
        }
        let trusted_key = trusted_keys
            .iter()
            .find(|candidate| candidate.id() == entry.key_id())
            .ok_or_else(|| {
                signature_verification_error("untrusted_key", "ML-DSA signature key is not trusted")
            })?;
        verify_mldsa65_signature(object_bytes, entry.signature(), trusted_key)?;
        return Ok(());
    }
    Err(signature_verification_error(
        "invalid_signature",
        "signature envelope has no required ML-DSA signature",
    ))
}

fn signature_verification_error(
    code: &'static str,
    message: impl Into<String>,
) -> SignatureVerificationError {
    SignatureVerificationError {
        code,
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryError {
    code: &'static str,
    message: String,
}

impl RepositoryError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RepositoryError {}

fn repository_error(message: impl Into<String>) -> RepositoryError {
    RepositoryError {
        code: "invalid_repository",
        message: message.into(),
    }
}

fn repository_verify_error(code: &'static str, message: impl Into<String>) -> RepositoryError {
    RepositoryError {
        code,
        message: message.into(),
    }
}

fn is_https_repository_url(value: &str) -> bool {
    value.starts_with("https://")
        && value.len() <= 2048
        && !value.contains('@')
        && !value.contains('#')
        && !value.contains('\\')
        && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}

fn parse_repository_package_type(value: &str) -> Result<PackageType, RepositoryError> {
    match value {
        "core" => Ok(PackageType::Core),
        "addon" => Ok(PackageType::Addon),
        "inputmethod-data" => Ok(PackageType::InputMethodData),
        "theme" => Ok(PackageType::Theme),
        "translation" => Ok(PackageType::Translation),
        _ => Err(repository_error("repository package type is unsupported")),
    }
}

fn repository_require_object_keys(
    object: &[(String, JsonValue)],
    required: &[&str],
) -> Result<(), RepositoryError> {
    for key in required {
        if !object_contains(object, key) {
            return Err(repository_error(format!("missing required key: {key}")));
        }
    }
    for (key, _) in object {
        if !required.iter().any(|allowed| *allowed == key) {
            return Err(repository_error(format!("unknown key: {key}")));
        }
    }
    Ok(())
}

fn repository_require_string(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
    allow_empty: bool,
) -> Result<String, RepositoryError> {
    let value = object_get(object, key)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| repository_error(format!("{key} must be a string")))?;
    if (!allow_empty && value.is_empty()) || value.len() > maximum || value.contains('\0') {
        return Err(repository_error(format!("{key} has an invalid length")));
    }
    Ok(value.to_owned())
}

fn repository_require_unsigned(
    object: &[(String, JsonValue)],
    key: &str,
) -> Result<u64, RepositoryError> {
    object_get(object, key)
        .and_then(JsonValue::as_number)
        .ok_or_else(|| repository_error(format!("{key} must be an unsigned integer")))
}

fn repository_require_array<'a>(
    object: &'a [(String, JsonValue)],
    key: &str,
) -> Result<&'a [JsonValue], RepositoryError> {
    object_get(object, key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| repository_error(format!("{key} must be an array")))
}

fn parse_repository_dependencies(
    values: &[JsonValue],
) -> Result<Vec<RepositoryDependency>, RepositoryError> {
    if values.len() > MAX_DEPENDENCY_COUNT {
        return Err(repository_error("repository dependency list is invalid"));
    }
    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for value in values {
        let object = value
            .as_object()
            .ok_or_else(|| repository_error("expected a JSON object"))?;
        repository_require_object_keys(object, &["id", "version"])?;
        let id = repository_require_string(object, "id", MAX_PACKAGE_ID_BYTES, false)?;
        let version = repository_require_string(object, "version", MAX_VERSION_BYTES, false)?;
        if !is_ascii_token(&id, "-_.")
            || !is_ascii_token(&version, ".+-_")
            || !ids.insert(id.clone())
        {
            return Err(repository_error(
                "repository dependency is invalid or duplicated",
            ));
        }
        result.push(RepositoryDependency { id, version });
    }
    Ok(result)
}

pub fn parse_repository_index(
    bytes: &str,
    expected_channel: &str,
) -> Result<RepositoryIndex, RepositoryError> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(repository_error(
            "repository index exceeds its resource budget",
        ));
    }
    let document = JsonParser::new(bytes)
        .parse()
        .map_err(|_| repository_error("repository index is not strict JSON"))?;
    let object = document
        .as_object()
        .ok_or_else(|| repository_error("expected a JSON object"))?;
    repository_require_object_keys(
        object,
        &[
            "format_version",
            "channel",
            "generated_at",
            "key_id",
            "packages",
        ],
    )?;
    let format_version = repository_require_unsigned(object, "format_version")?;
    if format_version != 1 {
        return Err(repository_error(
            "repository format_version must be exactly 1",
        ));
    }
    let channel = repository_require_string(object, "channel", 16, false)?;
    let generated_at = repository_require_string(object, "generated_at", 64, false)?;
    let key_id = repository_require_string(object, "key_id", MAX_PACKAGE_ID_BYTES, false)?;
    if channel != expected_channel || !is_ascii_token(&key_id, "-_.") {
        return Err(repository_error(
            "repository identity is invalid or channel mismatch",
        ));
    }

    let packages = repository_require_array(object, "packages")?;
    if packages.len() > MAX_FILE_COUNT {
        return Err(repository_error("package catalog is invalid"));
    }
    let mut identities = BTreeSet::new();
    let mut parsed = Vec::new();
    for value in packages {
        let item = value
            .as_object()
            .ok_or_else(|| repository_error("expected a JSON object"))?;
        repository_require_object_keys(
            item,
            &[
                "id",
                "title",
                "summary",
                "version",
                "release_sequence",
                "type",
                "architecture",
                "download_url",
                "sha256",
                "dependencies",
            ],
        )?;
        let id = repository_require_string(item, "id", MAX_PACKAGE_ID_BYTES, false)?;
        let title = repository_require_string(item, "title", 128, false)?;
        let summary = repository_require_string(item, "summary", 512, true)?;
        let version = repository_require_string(item, "version", MAX_VERSION_BYTES, false)?;
        let release_sequence = repository_require_unsigned(item, "release_sequence")?;
        let package_type =
            parse_repository_package_type(&repository_require_string(item, "type", 32, false)?)?;
        let architecture = repository_require_string(item, "architecture", 8, false)?;
        let download_url = repository_require_string(item, "download_url", 2048, false)?;
        let sha256 = HexDigest32::parse(&repository_require_string(item, "sha256", 64, false)?)
            .map_err(|_| repository_error("repository sha256 digest is invalid"))?;
        let dependencies =
            parse_repository_dependencies(repository_require_array(item, "dependencies")?)?;
        if !is_ascii_token(&id, "-_.")
            || !is_ascii_token(&version, ".+-_")
            || !matches!(architecture.as_str(), "any" | "x86" | "x64")
            || !is_https_repository_url(&download_url)
            || release_sequence == 0
            || !identities.insert((id.clone(), architecture.clone()))
        {
            return Err(repository_error(
                "repository package record is invalid or duplicated",
            ));
        }
        parsed.push(RepositoryEntry {
            id,
            title,
            summary,
            version,
            release_sequence,
            package_type,
            architecture,
            download_url,
            sha256,
            dependencies,
        });
    }
    parsed.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(RepositoryIndex {
        format_version,
        channel,
        generated_at,
        key_id,
        packages: parsed,
    })
}

pub fn verify_repository_index(
    index_bytes: &[u8],
    signature: &[u8],
    trusted_keys: &[TrustedKey],
    expected_channel: &str,
) -> Result<RepositoryIndex, RepositoryError> {
    let index_text = std::str::from_utf8(index_bytes)
        .map_err(|_| repository_error("repository index is not strict JSON"))?;
    let result = parse_repository_index(index_text, expected_channel)?;
    let trusted_key = trusted_keys
        .iter()
        .find(|candidate| candidate.id().as_str() == result.key_id())
        .ok_or_else(|| repository_verify_error("untrusted_key", "repository key is not trusted"))?;
    verify_manifest_signature(index_bytes, signature, trusted_key)
        .map_err(|error| repository_verify_error(error.code(), error.to_string()))?;
    Ok(result)
}

pub fn verify_repository_index_envelope(
    index_bytes: &[u8],
    envelope: &SignatureEnvelope,
    trusted_keys: &[TrustedKey],
    expected_channel: &str,
) -> Result<RepositoryIndex, RepositoryError> {
    let index_text = std::str::from_utf8(index_bytes)
        .map_err(|_| repository_error("repository index is not strict JSON"))?;
    let result = parse_repository_index(index_text, expected_channel)?;
    let key_id = PackageId::parse(result.key_id()).map_err(|_| {
        repository_verify_error("invalid_signature", "signature envelope binding is invalid")
    })?;
    verify_signature_envelope(
        index_bytes,
        envelope,
        trusted_keys,
        SignedObject::RepositoryIndex,
        &key_id,
    )
    .map_err(|error| repository_verify_error(error.code(), error.to_string()))?;
    Ok(result)
}

pub fn find_repository_package<'a>(
    index: &'a RepositoryIndex,
    package_id: &str,
    architecture: &str,
) -> Option<&'a RepositoryEntry> {
    index.packages.iter().find(|entry| {
        entry.id == package_id
            && (entry.architecture == "any" || entry.architecture == architecture)
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionError {
    code: &'static str,
    message: String,
}

impl ResolutionError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ResolutionError {}

pub fn resolve_exact_dependencies(
    available: &[Manifest],
    requested_ids: &[&str],
) -> Result<Vec<String>, ResolutionError> {
    let mut packages = Vec::<(&str, &Manifest)>::new();
    for package in available {
        if packages.iter().any(|(id, _)| *id == package.id().as_str()) {
            return Err(resolution_error("repository contains duplicate package id"));
        }
        packages.push((package.id().as_str(), package));
    }

    let mut visits = Vec::<(&str, Visit)>::new();
    let mut result = Vec::new();
    for id in requested_ids {
        visit_dependency(id, &packages, &mut visits, &mut result)?;
    }
    Ok(result)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Visit {
    Visiting,
    Complete,
}

fn visit_dependency<'a>(
    id: &'a str,
    packages: &[(&'a str, &'a Manifest)],
    visits: &mut Vec<(&'a str, Visit)>,
    result: &mut Vec<String>,
) -> Result<(), ResolutionError> {
    if let Some((_, state)) = visits.iter().find(|(visited_id, _)| *visited_id == id) {
        return match state {
            Visit::Visiting => Err(resolution_error("dependency cycle detected")),
            Visit::Complete => Ok(()),
        };
    }
    let manifest = packages
        .iter()
        .find_map(|(package_id, package)| (*package_id == id).then_some(*package))
        .ok_or_else(|| resolution_error(format!("required package is unavailable: {id}")))?;

    visits.push((id, Visit::Visiting));
    for dependency in manifest.dependencies() {
        let target = packages
            .iter()
            .find_map(|(package_id, package)| {
                (*package_id == dependency.id().as_str()).then_some(*package)
            })
            .ok_or_else(|| resolution_error("exact dependency version is unavailable"))?;
        if target.version() != dependency.version() {
            return Err(resolution_error("exact dependency version is unavailable"));
        }
        visit_dependency(dependency.id().as_str(), packages, visits, result)?;
    }
    if let Some((_, state)) = visits.iter_mut().find(|(visited_id, _)| *visited_id == id) {
        *state = Visit::Complete;
    }
    result.push(id.to_owned());
    Ok(())
}

fn resolution_error(message: impl Into<String>) -> ResolutionError {
    ResolutionError {
        code: "resolution_failed",
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadEntry {
    path: SafeRelativePackagePath,
    size: u64,
}

impl PayloadEntry {
    pub fn new(path: SafeRelativePackagePath, size: u64) -> Self {
        Self { path, size }
    }

    pub fn path(&self) -> &SafeRelativePackagePath {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadError {
    code: &'static str,
    message: String,
}

impl PayloadError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for PayloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PayloadError {}

pub fn verify_payload_inventory(
    manifest: &Manifest,
    observed: &[PayloadEntry],
) -> Result<(), PayloadError> {
    let mut declared = Vec::<(&str, u64, bool)>::new();
    for file in manifest.files() {
        declared.push((file.path().as_str(), file.size(), false));
    }
    let mut observed_exact = BTreeSet::new();
    let mut observed_windows = BTreeSet::new();
    for entry in observed {
        if !observed_exact.insert(entry.path().as_str().to_owned())
            || !observed_windows.insert(entry.path().as_str().to_lowercase())
        {
            return Err(payload_error(
                "payload contains duplicate or colliding files",
            ));
        }
        let Some((_, declared_size, seen)) = declared
            .iter_mut()
            .find(|(path, _, _)| *path == entry.path().as_str())
        else {
            return Err(payload_error("payload contains an undeclared file"));
        };
        if *declared_size != entry.size() {
            return Err(payload_error("payload file does not match manifest"));
        }
        *seen = true;
    }
    if declared.iter().any(|(_, _, seen)| !*seen) {
        return Err(payload_error("payload file does not match manifest"));
    }
    Ok(())
}

fn payload_error(message: impl Into<String>) -> PayloadError {
    PayloadError {
        code: "payload_mismatch",
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingError {
    code: &'static str,
    message: String,
}

impl StagingError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for StagingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for StagingError {}

#[cfg(windows)]
pub fn verify_payload_root(
    manifest: &Manifest,
    payload_root: impl AsRef<Path>,
) -> Result<(), StagingError> {
    let payload_root = payload_root.as_ref();
    let observed = read_payload_bytes_from_root(manifest, payload_root)?;
    verify_payload_bytes(manifest, &observed).map_err(staging_from_payload_error)?;
    reject_undeclared_payload_files(manifest, payload_root)?;
    Ok(())
}

#[cfg(windows)]
pub fn stage_verified_payload_tree(
    manifest: &Manifest,
    manifest_bytes: &[u8],
    payload_root: impl AsRef<Path>,
    install_root: impl AsRef<Path>,
    transaction_id: &str,
    signature: &[u8],
) -> Result<PathBuf, StagingError> {
    PackageId::parse(transaction_id)
        .map_err(|_| staging_error("unsafe_path", "transaction id or install root is unsafe"))?;
    let payload_root = payload_root.as_ref();
    let install_root = install_root.as_ref();
    if path_contains_reparse_component(install_root).map_err(staging_io_error)? {
        return Err(staging_error(
            "unsafe_path",
            "transaction id or install root is unsafe",
        ));
    }
    verify_payload_root(manifest, payload_root)?;

    let staging_root = install_root.join("staging");
    fs::create_dir_all(&staging_root).map_err(staging_io_error)?;
    let staged = staging_root.join(transaction_id);
    if staged.exists() {
        return Err(staging_error(
            "transaction_exists",
            "staging transaction already exists",
        ));
    }

    let result = (|| {
        fs::create_dir_all(staged.join("payload")).map_err(staging_io_error)?;
        for file in manifest.files() {
            let source = join_package_path(payload_root, file.path());
            let destination = join_package_path(&staged.join("payload"), file.path());
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(staging_io_error)?;
            }
            fs::copy(source, destination).map_err(staging_io_error)?;
        }
        fs::write(staged.join("manifest.json"), manifest_bytes).map_err(staging_io_error)?;
        fs::write(staged.join("manifest.sig"), signature).map_err(staging_io_error)?;
        verify_payload_root(manifest, staged.join("payload"))?;
        Ok(staged.clone())
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&staged);
    }
    result
}

#[cfg(windows)]
fn read_payload_bytes_from_root(
    manifest: &Manifest,
    payload_root: &Path,
) -> Result<Vec<PayloadBytesEntry>, StagingError> {
    if !payload_root.is_dir()
        || path_contains_reparse_component(payload_root).map_err(staging_io_error)?
    {
        return Err(staging_error(
            "unsafe_payload",
            "payload root is missing or contains a reparse point",
        ));
    }

    let mut observed = Vec::with_capacity(manifest.files().len());
    for file in manifest.files() {
        let path = join_package_path(payload_root, file.path());
        if path_contains_reparse_component(&path).map_err(staging_io_error)? {
            return Err(staging_error(
                "unsafe_payload",
                "payload contains a reparse point",
            ));
        }
        let metadata = fs::metadata(&path).map_err(|_| {
            staging_error("payload_mismatch", "payload file does not match manifest")
        })?;
        if !metadata.is_file() || metadata.len() != file.size() {
            return Err(staging_error(
                "payload_mismatch",
                "payload file does not match manifest",
            ));
        }
        let bytes = fs::read(&path).map_err(staging_io_error)?;
        observed.push(PayloadBytesEntry::new(file.path().clone(), bytes));
    }
    Ok(observed)
}

#[cfg(windows)]
fn reject_undeclared_payload_files(
    manifest: &Manifest,
    payload_root: &Path,
) -> Result<(), StagingError> {
    let expected = manifest
        .files()
        .iter()
        .map(|file| file.path().as_str().to_owned())
        .collect::<BTreeSet<_>>();
    reject_undeclared_payload_files_in(payload_root, payload_root, &expected)
}

#[cfg(windows)]
fn reject_undeclared_payload_files_in(
    payload_root: &Path,
    directory: &Path,
    expected: &BTreeSet<String>,
) -> Result<(), StagingError> {
    for entry in fs::read_dir(directory).map_err(|_| {
        staging_error(
            "unsafe_payload",
            "payload directory cannot be enumerated safely",
        )
    })? {
        let entry = entry.map_err(|_| {
            staging_error(
                "unsafe_payload",
                "payload directory cannot be enumerated safely",
            )
        })?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(staging_io_error)?;
        if metadata_has_reparse_point(&metadata) {
            return Err(staging_error(
                "unsafe_payload",
                "payload contains a reparse point",
            ));
        }
        if metadata.is_dir() {
            reject_undeclared_payload_files_in(payload_root, &path, expected)?;
        } else if metadata.is_file() {
            let relative = relative_package_path(payload_root, &path).ok_or_else(|| {
                staging_error("payload_mismatch", "payload contains an undeclared file")
            })?;
            if !expected.contains(&relative) {
                return Err(staging_error(
                    "payload_mismatch",
                    "payload contains an undeclared file",
                ));
            }
        }
    }
    Ok(())
}

fn join_package_path(root: &Path, relative: &SafeRelativePackagePath) -> PathBuf {
    let mut path = root.to_path_buf();
    for component in relative.as_str().split('/') {
        path.push(component);
    }
    path
}

#[cfg(windows)]
fn relative_package_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut components = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => components.push(value.to_str()?),
            _ => return None,
        }
    }
    let joined = components.join("/");
    is_safe_relative_package_path(&joined).then_some(joined)
}

#[cfg(windows)]
fn path_contains_reparse_component(path: &Path) -> io::Result<bool> {
    for ancestor in path.ancestors() {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata_has_reparse_point(&metadata) => return Ok(true),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(false)
}

#[cfg(windows)]
fn metadata_has_reparse_point(metadata: &fs::Metadata) -> bool {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

fn staging_error(code: &'static str, message: impl Into<String>) -> StagingError {
    StagingError {
        code,
        message: message.into(),
    }
}

fn staging_io_error(error: io::Error) -> StagingError {
    staging_error("io_error", error.to_string())
}

fn staging_from_payload_error(error: PayloadError) -> StagingError {
    StagingError {
        code: error.code,
        message: error.message,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadDigestEntry {
    path: SafeRelativePackagePath,
    size: u64,
    blake3: Option<HexDigest32>,
    sha256: Option<HexDigest32>,
}

impl PayloadDigestEntry {
    pub fn new(
        path: SafeRelativePackagePath,
        size: u64,
        blake3: Option<HexDigest32>,
        sha256: Option<HexDigest32>,
    ) -> Self {
        Self {
            path,
            size,
            blake3,
            sha256,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadBytesEntry {
    path: SafeRelativePackagePath,
    bytes: Vec<u8>,
}

impl PayloadBytesEntry {
    pub fn new(path: SafeRelativePackagePath, bytes: Vec<u8>) -> Self {
        Self { path, bytes }
    }
}

pub fn verify_payload_bytes(
    manifest: &Manifest,
    observed: &[PayloadBytesEntry],
) -> Result<(), PayloadError> {
    let mut digests = Vec::with_capacity(observed.len());
    for entry in observed {
        digests.push(PayloadDigestEntry::new(
            entry.path.clone(),
            u64::try_from(entry.bytes.len())
                .map_err(|_| payload_error("payload file does not match manifest"))?,
            Some(blake3_digest(&entry.bytes)),
            Some(sha256_digest(&entry.bytes)),
        ));
    }
    verify_payload_digests(manifest, &digests)
}

pub fn verify_payload_digests(
    manifest: &Manifest,
    observed: &[PayloadDigestEntry],
) -> Result<(), PayloadError> {
    let inventory: Vec<_> = observed
        .iter()
        .map(|entry| PayloadEntry::new(entry.path.clone(), entry.size))
        .collect();
    verify_payload_inventory(manifest, &inventory)?;

    for file in manifest.files() {
        let observed_file = observed
            .iter()
            .find(|entry| entry.path.as_str() == file.path().as_str())
            .ok_or_else(|| payload_error("payload file does not match manifest"))?;
        if manifest.format_version() == MANIFEST_FORMAT_VERSION_V1 {
            if file.hashes().sha256() != observed_file.sha256.as_ref() {
                return Err(payload_error("payload file does not match manifest"));
            }
        } else if manifest.format_version() == MANIFEST_FORMAT_VERSION_V2 {
            if file.hashes().blake3() != observed_file.blake3.as_ref()
                || (file.hashes().sha256().is_some()
                    && file.hashes().sha256() != observed_file.sha256.as_ref())
            {
                return Err(payload_error("payload file does not match manifest"));
            }
        } else {
            return Err(payload_error(
                "payload verifier does not support manifest version",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageLifecycleState {
    Installed,
    Enabled,
    Disabled,
    PendingUpdate,
    PendingRemove,
    Broken,
    Quarantined,
}

impl PackageLifecycleState {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "installed" => Some(Self::Installed),
            "enabled" => Some(Self::Enabled),
            "disabled" => Some(Self::Disabled),
            "pending_update" => Some(Self::PendingUpdate),
            "pending_remove" => Some(Self::PendingRemove),
            "broken" => Some(Self::Broken),
            "quarantined" => Some(Self::Quarantined),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::PendingUpdate => "pending_update",
            Self::PendingRemove => "pending_remove",
            Self::Broken => "broken",
            Self::Quarantined => "quarantined",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockEntry {
    id: PackageId,
    version: String,
    manifest_sha256: HexDigest32,
    state: PackageLifecycleState,
}

impl LockEntry {
    pub fn new(
        id: PackageId,
        version: String,
        manifest_sha256: HexDigest32,
        state: PackageLifecycleState,
    ) -> Self {
        Self {
            id,
            version,
            manifest_sha256,
            state,
        }
    }

    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn manifest_sha256(&self) -> &HexDigest32 {
        &self.manifest_sha256
    }

    pub fn state(&self) -> &PackageLifecycleState {
        &self.state
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockfileError {
    code: &'static str,
    message: String,
}

impl LockfileError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for LockfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LockfileError {}

pub fn parse_lockfile(bytes: &str) -> Result<Vec<LockEntry>, LockfileError> {
    let document = JsonParser::new(bytes)
        .parse()
        .map_err(|_| lockfile_error("packages.lock is not strict JSON"))?;
    let object = document
        .as_object()
        .ok_or_else(|| lockfile_error("packages.lock schema is invalid"))?;
    require_object_keys_for_lockfile(object, &["format_version", "packages"], &[])?;
    let format_version = object_get(object, "format_version")
        .and_then(JsonValue::as_number)
        .ok_or_else(|| lockfile_error("packages.lock schema is invalid"))?;
    let packages = object_get(object, "packages")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| lockfile_error("packages.lock schema is invalid"))?;
    if format_version != 1 || packages.len() > MAX_FILE_COUNT {
        return Err(lockfile_error("packages.lock schema is invalid"));
    }

    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for value in packages {
        let entry = value
            .as_object()
            .ok_or_else(|| lockfile_error("packages.lock entry is invalid"))?;
        require_object_keys_for_lockfile(
            entry,
            &["id", "version", "manifest_sha256", "state"],
            &[],
        )?;
        let id = PackageId::parse(&require_lockfile_string(
            entry,
            "id",
            MAX_PACKAGE_ID_BYTES,
            false,
        )?)
        .map_err(|_| lockfile_error("packages.lock entry is invalid"))?;
        if !ids.insert(id.as_str().to_owned()) {
            return Err(lockfile_error("packages.lock entry is invalid"));
        }
        let version = require_lockfile_string(entry, "version", MAX_VERSION_BYTES, false)?;
        let manifest_sha256 = HexDigest32::parse(&require_lockfile_string(
            entry,
            "manifest_sha256",
            64,
            false,
        )?)
        .map_err(|_| lockfile_error("packages.lock entry is invalid"))?;
        let state =
            PackageLifecycleState::parse(&require_lockfile_string(entry, "state", 32, false)?)
                .ok_or_else(|| lockfile_error("packages.lock entry is invalid"))?;
        if !is_ascii_token(&version, ".+-_") {
            return Err(lockfile_error("packages.lock entry is invalid"));
        }
        result.push(LockEntry {
            id,
            version,
            manifest_sha256,
            state,
        });
    }
    Ok(result)
}

fn require_object_keys_for_lockfile(
    object: &[(String, JsonValue)],
    required: &[&str],
    optional: &[&str],
) -> Result<(), LockfileError> {
    for key in required {
        if !object_contains(object, key) {
            return Err(lockfile_error(format!("missing required key: {key}")));
        }
    }
    for (key, _) in object {
        if !required
            .iter()
            .chain(optional.iter())
            .any(|allowed| *allowed == key)
        {
            return Err(lockfile_error(format!("unknown key: {key}")));
        }
    }
    Ok(())
}

fn require_lockfile_string(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
    allow_empty: bool,
) -> Result<String, LockfileError> {
    let value = object_get(object, key)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| lockfile_error(format!("{key} must be a string")))?;
    if (!allow_empty && value.is_empty()) || value.len() > maximum || value.contains('\0') {
        return Err(lockfile_error(format!("{key} has an invalid length")));
    }
    Ok(value.to_owned())
}

fn lockfile_error(message: impl Into<String>) -> LockfileError {
    LockfileError {
        code: "invalid_lockfile",
        message: message.into(),
    }
}

#[cfg(windows)]
pub fn read_installed_lockfile(
    install_root: impl AsRef<Path>,
) -> Result<Vec<LockEntry>, LockfileError> {
    let lock_path = install_root.as_ref().join("packages.lock");
    if !lock_path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read_to_string(lock_path)
        .map_err(|_| lockfile_error("packages.lock is not strict JSON"))?;
    parse_lockfile(&bytes)
}

#[cfg(windows)]
pub fn write_installed_lockfile_atomic(
    install_root: impl AsRef<Path>,
    entries: &[LockEntry],
) -> Result<(), StagingError> {
    let install_root = install_root.as_ref();
    fs::create_dir_all(install_root).map_err(staging_io_error)?;
    let temporary = install_root.join("packages.lock.new");
    let lock_path = install_root.join("packages.lock");
    fs::write(&temporary, lockfile_to_json(entries)).map_err(staging_io_error)?;
    win32_fs_adapter::replace_file(&temporary, &lock_path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        staging_error(
            "activation_failed",
            format!("unable to atomically publish packages.lock: {error}"),
        )
    })
}

#[cfg(windows)]
pub fn activate_staged_payload_tree(
    staged_root: impl AsRef<Path>,
    install_root: impl AsRef<Path>,
    trusted_keys: &[TrustedKey],
) -> Result<(), StagingError> {
    let staged_root = staged_root.as_ref();
    let install_root = install_root.as_ref();
    if path_contains_reparse_component(staged_root).map_err(staging_io_error)?
        || path_contains_reparse_component(install_root).map_err(staging_io_error)?
    {
        return Err(staging_error(
            "unsafe_path",
            "activation path contains a reparse point",
        ));
    }
    let manifest_bytes = fs::read(staged_root.join("manifest.json")).map_err(staging_io_error)?;
    if manifest_bytes.len() > MAX_MANIFEST_BYTES {
        return Err(staging_error(
            "invalid_manifest",
            "manifest size is outside the accepted range",
        ));
    }
    let manifest_text = std::str::from_utf8(&manifest_bytes)
        .map_err(|_| staging_error("invalid_manifest", "manifest is not valid UTF-8"))?;
    let manifest = parse_manifest(manifest_text).map_err(staging_from_manifest_error)?;
    let trusted_key = trusted_keys
        .iter()
        .find(|candidate| candidate.id() == manifest.key_id())
        .ok_or_else(|| staging_error("untrusted_key", "manifest key is not trusted"))?;
    if fs::metadata(staged_root.join("manifest.sig"))
        .map_err(staging_io_error)?
        .len()
        > MAX_SIGNATURE_BYTES
    {
        return Err(staging_error(
            "invalid_signature",
            "manifest signature is outside its resource budget",
        ));
    }
    let signature = fs::read(staged_root.join("manifest.sig")).map_err(staging_io_error)?;
    verify_manifest_signature(&manifest_bytes, &signature, trusted_key)
        .map_err(|error| staging_error(error.code, error.message))?;
    verify_payload_root(&manifest, staged_root.join("payload"))?;
    let active_before = read_installed_lockfile(install_root)
        .map_err(|error| staging_error(error.code, error.message))?;
    for dependency in manifest.dependencies() {
        let installed = active_before.iter().any(|entry| {
            entry.id() == dependency.id()
                && entry.version() == dependency.version()
                && !matches!(
                    entry.state(),
                    PackageLifecycleState::Disabled
                        | PackageLifecycleState::PendingRemove
                        | PackageLifecycleState::Broken
                        | PackageLifecycleState::Quarantined
                )
        });
        if !installed {
            return Err(staging_error(
                "resolution_failed",
                "an exact active dependency is unavailable at activation time",
            ));
        }
    }

    let versions = install_root.join("versions").join(manifest.id().as_str());
    fs::create_dir_all(&versions).map_err(staging_io_error)?;
    let destination = versions.join(manifest.version());
    if destination.exists() {
        verify_payload_root(&manifest, &destination)?;
    } else {
        win32_fs_adapter::publish_directory(&staged_root.join("payload"), &destination).map_err(
            |_| staging_error("activation_failed", "unable to publish version directory"),
        )?;
    }

    let metadata = install_root.join("manifests").join(manifest.id().as_str());
    fs::create_dir_all(&metadata).map_err(staging_io_error)?;
    fs::write(
        metadata.join(format!("{}.json", manifest.version())),
        &manifest_bytes,
    )
    .map_err(staging_io_error)?;
    fs::write(
        metadata.join(format!("{}.sig", manifest.version())),
        &signature,
    )
    .map_err(staging_io_error)?;

    let mut lock = active_before;
    upsert_installed_lock_entry(
        &mut lock,
        manifest.id().clone(),
        manifest.version().to_owned(),
        sha256_digest(&manifest_bytes),
    )
    .map_err(|error| staging_error(error.code, error.message))?;
    write_installed_lockfile_atomic(install_root, &lock)?;
    let _ = fs::remove_dir_all(staged_root);
    Ok(())
}

#[cfg(windows)]
pub fn set_installed_package_state(
    install_root: impl AsRef<Path>,
    package_id: &str,
    state: PackageLifecycleState,
) -> Result<(), LifecycleError> {
    let install_root = install_root.as_ref();
    let mut lock = read_installed_lockfile(install_root)
        .map_err(|error| lifecycle_error(error.code, error.message))?;
    set_package_state_entries(&mut lock, package_id, state)?;
    write_installed_lockfile_atomic(install_root, &lock)
        .map_err(|error| lifecycle_error(error.code, error.message))
}

#[cfg(windows)]
pub fn mark_installed_package_for_removal(
    install_root: impl AsRef<Path>,
    package_id: &str,
) -> Result<(), LifecycleError> {
    let install_root = install_root.as_ref();
    let mut lock = read_installed_lockfile(install_root)
        .map_err(|error| lifecycle_error(error.code, error.message))?;
    let manifests = read_installed_manifests(install_root, &lock)?;
    mark_package_for_removal_entries(&mut lock, &manifests, package_id)?;
    write_installed_lockfile_atomic(install_root, &lock)
        .map_err(|error| lifecycle_error(error.code, error.message))
}

#[cfg(windows)]
pub fn finalize_installed_package_removal(
    install_root: impl AsRef<Path>,
    package_id: &str,
) -> Result<(), LifecycleError> {
    let install_root = install_root.as_ref();
    let mut lock = read_installed_lockfile(install_root)
        .map_err(|error| lifecycle_error(error.code, error.message))?;
    finalize_package_removal_entries(&mut lock, package_id)?;
    write_installed_lockfile_atomic(install_root, &lock)
        .map_err(|error| lifecycle_error(error.code, error.message))?;
    let package_id = PackageId::parse(package_id).map_err(|_| {
        lifecycle_error("invalid_state", "package id or lifecycle state is invalid")
    })?;
    fs::remove_dir_all(install_root.join("versions").join(package_id.as_str())).map_err(|_| {
        lifecycle_error(
            "remove_pending",
            "package deactivated but payload cleanup must be retried",
        )
    })?;
    fs::remove_dir_all(install_root.join("manifests").join(package_id.as_str())).map_err(|_| {
        lifecycle_error(
            "remove_pending",
            "package deactivated but metadata cleanup must be retried",
        )
    })?;
    Ok(())
}

#[cfg(windows)]
fn read_installed_manifests(
    install_root: &Path,
    lock: &[LockEntry],
) -> Result<Vec<Manifest>, LifecycleError> {
    let mut manifests = Vec::new();
    for entry in lock {
        let path = install_root
            .join("manifests")
            .join(entry.id().as_str())
            .join(format!("{}.json", entry.version()));
        let bytes = fs::read_to_string(path)
            .map_err(|_| lifecycle_error("package_not_found", "package manifest is unavailable"))?;
        let manifest = parse_manifest(&bytes)
            .map_err(|_| lifecycle_error("package_not_found", "package manifest is unavailable"))?;
        if manifest.id() != entry.id() || manifest.version() != entry.version() {
            return Err(lifecycle_error(
                "package_not_found",
                "package manifest is unavailable",
            ));
        }
        manifests.push(manifest);
    }
    Ok(manifests)
}

fn lockfile_to_json(entries: &[LockEntry]) -> String {
    let mut output = String::from("{\n  \"format_version\": 1,\n  \"packages\": [");
    for (index, entry) in entries.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("\n    {\"id\": \"");
        output.push_str(entry.id().as_str());
        output.push_str("\", \"version\": \"");
        output.push_str(&json_escape(entry.version()));
        output.push_str("\", \"manifest_sha256\": \"");
        output.push_str(entry.manifest_sha256().as_str());
        output.push_str("\", \"state\": \"");
        output.push_str(entry.state().as_str());
        output.push_str("\"}");
    }
    output.push_str("\n  ]\n}\n");
    output
}

fn json_escape(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if c < ' ' => output.push_str(&format!("\\u{:04x}", c as u32)),
            c => output.push(c),
        }
    }
    output
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleError {
    code: &'static str,
    message: String,
}

impl LifecycleError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LifecycleError {}

pub fn set_package_state_entries(
    lock: &mut [LockEntry],
    package_id: &str,
    state: PackageLifecycleState,
) -> Result<(), LifecycleError> {
    let package_id = PackageId::parse(package_id).map_err(|_| {
        lifecycle_error("invalid_state", "package id or lifecycle state is invalid")
    })?;
    let entry = lock
        .iter_mut()
        .find(|entry| entry.id() == &package_id)
        .ok_or_else(|| lifecycle_error("package_not_found", "package is not installed"))?;
    entry.state = state;
    Ok(())
}

pub fn mark_package_for_removal_entries(
    lock: &mut [LockEntry],
    installed_manifests: &[Manifest],
    package_id: &str,
) -> Result<(), LifecycleError> {
    let package_id = PackageId::parse(package_id).map_err(|_| {
        lifecycle_error("invalid_state", "package id or lifecycle state is invalid")
    })?;
    let target = lock
        .iter()
        .find(|entry| entry.id() == &package_id)
        .ok_or_else(|| lifecycle_error("package_not_found", "package is not installed"))?;
    let target_manifest = installed_manifests
        .iter()
        .find(|manifest| manifest.id() == target.id() && manifest.version() == target.version())
        .ok_or_else(|| lifecycle_error("package_not_found", "package manifest is unavailable"))?;
    if target_manifest.package_type() == &PackageType::Core {
        return Err(lifecycle_error(
            "protected_package",
            "core packages cannot be removed",
        ));
    }
    for entry in lock.iter() {
        if entry.id() == &package_id || entry.state() == &PackageLifecycleState::PendingRemove {
            continue;
        }
        let Some(dependent) = installed_manifests
            .iter()
            .find(|manifest| manifest.id() == entry.id() && manifest.version() == entry.version())
        else {
            continue;
        };
        if dependent
            .dependencies()
            .iter()
            .any(|dependency| dependency.id() == &package_id)
        {
            return Err(lifecycle_error(
                "package_in_use",
                "another installed package depends on this package",
            ));
        }
    }
    let entry = lock
        .iter_mut()
        .find(|entry| entry.id() == &package_id)
        .ok_or_else(|| lifecycle_error("package_not_found", "package is not installed"))?;
    entry.state = PackageLifecycleState::PendingRemove;
    Ok(())
}

pub fn finalize_package_removal_entries(
    lock: &mut Vec<LockEntry>,
    package_id: &str,
) -> Result<(), LifecycleError> {
    let package_id = PackageId::parse(package_id).map_err(|_| {
        lifecycle_error("invalid_state", "package id or lifecycle state is invalid")
    })?;
    let Some(index) = lock.iter().position(|entry| entry.id() == &package_id) else {
        return Err(lifecycle_error(
            "invalid_state",
            "package is not pending removal",
        ));
    };
    if lock[index].state() != &PackageLifecycleState::PendingRemove {
        return Err(lifecycle_error(
            "invalid_state",
            "package is not pending removal",
        ));
    }
    lock.remove(index);
    Ok(())
}

pub fn upsert_installed_lock_entry(
    lock: &mut Vec<LockEntry>,
    package_id: PackageId,
    version: String,
    manifest_sha256: HexDigest32,
) -> Result<(), LifecycleError> {
    if version.is_empty() || version.len() > MAX_VERSION_BYTES || !is_ascii_token(&version, ".+-_")
    {
        return Err(lifecycle_error(
            "invalid_state",
            "package id or lifecycle state is invalid",
        ));
    }
    let updated = LockEntry {
        id: package_id,
        version,
        manifest_sha256,
        state: PackageLifecycleState::Installed,
    };
    if let Some(entry) = lock.iter_mut().find(|entry| entry.id() == updated.id()) {
        *entry = updated;
    } else {
        lock.push(updated);
    }
    lock.sort_by(|left, right| left.id().cmp(right.id()));
    Ok(())
}

fn lifecycle_error(code: &'static str, message: impl Into<String>) -> LifecycleError {
    LifecycleError {
        code,
        message: message.into(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveEntry {
    name: String,
    uncompressed_size: u64,
    directory: bool,
    encrypted: bool,
    supported: bool,
    unix_symlink: bool,
}

impl ArchiveEntry {
    pub fn file(name: &str, uncompressed_size: u64) -> Self {
        Self {
            name: name.to_owned(),
            uncompressed_size,
            directory: false,
            encrypted: false,
            supported: true,
            unix_symlink: false,
        }
    }

    pub fn directory(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            uncompressed_size: 0,
            directory: true,
            encrypted: false,
            supported: true,
            unix_symlink: false,
        }
    }

    pub fn with_unix_symlink(mut self) -> Self {
        self.unix_symlink = true;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveError {
    code: &'static str,
    message: String,
}

impl ArchiveError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ArchiveError {}

pub fn validate_archive_inventory(
    manifest: &Manifest,
    entries: &[ArchiveEntry],
) -> Result<(), ArchiveError> {
    if entries.len() < 3 || entries.len() > MAX_FILE_COUNT + 2 {
        return Err(archive_error(
            "invalid_archive",
            "archive entry count is outside its budget",
        ));
    }
    let mut expected_casefold =
        BTreeSet::from(["manifest.json".to_owned(), "manifest.sig".to_owned()]);
    let mut expected_files = Vec::<(String, u64)>::new();
    for file in manifest.files() {
        let archive_name = format!("payload/{}", file.path().as_str());
        if !expected_casefold.insert(archive_name.to_lowercase()) {
            return Err(archive_error(
                "unsafe_archive_path",
                "manifest paths collide on Windows",
            ));
        }
        expected_files.push((archive_name, file.size()));
    }

    let mut seen = BTreeSet::new();
    let mut total_uncompressed = 0_u64;
    for entry in entries {
        if entry.name.is_empty() || entry.encrypted || !entry.supported || entry.unix_symlink {
            return Err(archive_error(
                "invalid_archive",
                "archive contains an unsupported or executable link entry",
            ));
        }
        let mut logical_name = entry.name.as_str();
        let trimmed;
        if entry.directory && logical_name.ends_with('/') {
            trimmed = logical_name.trim_end_matches('/').to_owned();
            logical_name = &trimmed;
        }
        if logical_name != "manifest.json" && logical_name != "manifest.sig" {
            let Some(payload_path) = logical_name.strip_prefix("payload/") else {
                return Err(archive_error(
                    "unsafe_archive_path",
                    "archive path is outside payload/",
                ));
            };
            if !is_safe_relative_package_path(payload_path) {
                return Err(archive_error(
                    "unsafe_archive_path",
                    "archive path is outside payload/",
                ));
            }
        }
        if !seen.insert(logical_name.to_lowercase()) {
            return Err(archive_error(
                "unsafe_archive_path",
                "archive contains a case-insensitive duplicate path",
            ));
        }
        if entry.directory {
            continue;
        }
        if !expected_casefold.contains(&logical_name.to_lowercase()) {
            return Err(archive_error(
                "invalid_archive",
                "archive contains an undeclared file",
            ));
        }
        if let Some((_, expected_size)) =
            expected_files.iter().find(|(name, _)| name == logical_name)
        {
            if *expected_size != entry.uncompressed_size {
                return Err(archive_error(
                    "payload_mismatch",
                    "archive file size differs from manifest",
                ));
            }
        }
        if entry.uncompressed_size > MAX_FILE_BYTES
            || total_uncompressed > MAX_PAYLOAD_BYTES.saturating_sub(entry.uncompressed_size)
        {
            return Err(archive_error(
                "invalid_archive",
                "archive expands beyond its resource budget",
            ));
        }
        total_uncompressed += entry.uncompressed_size;
    }
    if seen.len() < expected_casefold.len() {
        return Err(archive_error(
            "invalid_archive",
            "archive is missing a declared payload file",
        ));
    }
    Ok(())
}

#[cfg(windows)]
pub fn stage_validated_archive_zip(
    archive_path: impl AsRef<Path>,
    install_root: impl AsRef<Path>,
    transaction_id: &str,
    trusted_keys: &[TrustedKey],
) -> Result<PathBuf, StagingError> {
    PackageId::parse(transaction_id)
        .map_err(|_| staging_error("unsafe_path", "transaction id or install root is unsafe"))?;
    let archive_path = archive_path.as_ref();
    let install_root = install_root.as_ref();
    if path_contains_reparse_component(archive_path).map_err(staging_io_error)?
        || path_contains_reparse_component(install_root).map_err(staging_io_error)?
    {
        return Err(staging_error(
            "unsafe_path",
            "transaction id or install root is unsafe",
        ));
    }

    let archive = miniz_archive_adapter::ZipArchive::open(archive_path)
        .map_err(staging_from_archive_error)?;
    let entries = read_zip_inventory(&archive).map_err(staging_from_archive_error)?;
    let manifest_bytes = archive
        .extract(
            archive
                .locate("manifest.json")
                .map_err(staging_from_archive_error)?,
            MAX_MANIFEST_BYTES as u64,
        )
        .map_err(staging_from_archive_error)?;
    let signature = archive
        .extract(
            archive
                .locate("manifest.sig")
                .map_err(staging_from_archive_error)?,
            MAX_SIGNATURE_BYTES,
        )
        .map_err(staging_from_archive_error)?;
    let manifest_text = std::str::from_utf8(&manifest_bytes)
        .map_err(|_| staging_error("invalid_manifest", "manifest is not valid UTF-8"))?;
    let manifest = parse_manifest(manifest_text).map_err(staging_from_manifest_error)?;
    let trusted_key = trusted_keys
        .iter()
        .find(|candidate| candidate.id() == manifest.key_id())
        .ok_or_else(|| staging_error("untrusted_key", "manifest key is not trusted"))?;
    verify_manifest_signature(&manifest_bytes, &signature, trusted_key)
        .map_err(|error| staging_error(error.code, error.message))?;
    validate_manifest_compatibility(&manifest, current_runtime_architecture())
        .map_err(staging_from_compatibility_error)?;
    validate_archive_inventory(&manifest, &entries).map_err(staging_from_archive_error)?;
    for index in 0..archive.len() {
        archive
            .validate(index)
            .map_err(staging_from_archive_error)?;
    }

    let staging_root = install_root.join("staging");
    fs::create_dir_all(&staging_root).map_err(staging_io_error)?;
    let extraction = staging_root.join(format!("{transaction_id}.extract"));
    let staged = staging_root.join(transaction_id);
    if extraction.exists() || staged.exists() {
        return Err(staging_error(
            "transaction_exists",
            "staging transaction already exists",
        ));
    }

    let result = (|| {
        fs::create_dir_all(extraction.join("payload")).map_err(staging_io_error)?;
        fs::write(extraction.join("manifest.json"), &manifest_bytes).map_err(staging_io_error)?;
        fs::write(extraction.join("manifest.sig"), &signature).map_err(staging_io_error)?;
        for file in manifest.files() {
            let archive_name = format!("payload/{}", file.path().as_str());
            let contents = archive
                .extract(
                    archive
                        .locate(&archive_name)
                        .map_err(staging_from_archive_error)?,
                    file.size(),
                )
                .map_err(staging_from_archive_error)?;
            let destination = join_package_path(&extraction.join("payload"), file.path());
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(staging_io_error)?;
            }
            fs::write(destination, contents).map_err(staging_io_error)?;
        }
        verify_payload_root(&manifest, extraction.join("payload"))?;
        fs::rename(&extraction, &staged).map_err(staging_io_error)?;
        Ok(staged.clone())
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&extraction);
        let _ = fs::remove_dir_all(&staged);
    }
    result
}

#[cfg(windows)]
fn read_zip_inventory(
    archive: &miniz_archive_adapter::ZipArchive,
) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        entries.push(archive.stat(index)?);
    }
    Ok(entries)
}

#[cfg(all(windows, target_pointer_width = "64"))]
fn current_runtime_architecture() -> &'static str {
    "x64"
}

#[cfg(all(windows, not(target_pointer_width = "64")))]
fn current_runtime_architecture() -> &'static str {
    "x86"
}

fn archive_error(code: &'static str, message: impl Into<String>) -> ArchiveError {
    ArchiveError {
        code,
        message: message.into(),
    }
}

fn staging_from_archive_error(error: ArchiveError) -> StagingError {
    staging_error(error.code, error.message)
}

fn staging_from_manifest_error(error: ManifestError) -> StagingError {
    staging_error(error.code, error.message)
}

fn staging_from_compatibility_error(error: CompatibilityError) -> StagingError {
    staging_error(error.code, error.message)
}

fn manifest_error(code: &'static str, message: impl Into<String>) -> ManifestError {
    ManifestError {
        code,
        message: message.into(),
    }
}

fn compatibility_error(message: impl Into<String>) -> CompatibilityError {
    CompatibilityError {
        code: "incompatible_package",
        message: message.into(),
    }
}

fn parse_package_type(value: &str) -> Result<PackageType, ManifestError> {
    match value {
        "core" => Ok(PackageType::Core),
        "addon" => Ok(PackageType::Addon),
        "inputmethod-data" => Ok(PackageType::InputMethodData),
        "theme" => Ok(PackageType::Theme),
        "translation" => Ok(PackageType::Translation),
        _ => Err(manifest_error(
            "invalid_manifest",
            "unsupported package type",
        )),
    }
}

fn parse_dependencies(values: &[JsonValue]) -> Result<Vec<Dependency>, ManifestError> {
    if values.len() > MAX_DEPENDENCY_COUNT {
        return Err(manifest_error(
            "invalid_manifest",
            "dependencies must be a bounded array",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for value in values {
        let object = value
            .as_object()
            .ok_or_else(|| manifest_error("invalid_manifest", "expected a JSON object"))?;
        require_object_keys(object, &["id", "version"], &[])?;
        let id = PackageId::parse(&require_string(object, "id", MAX_PACKAGE_ID_BYTES, false)?)
            .map_err(|_| manifest_error("invalid_manifest", "dependency identity is invalid"))?;
        if !ids.insert(id.as_str().to_owned()) {
            return Err(manifest_error(
                "invalid_manifest",
                "dependency identity is invalid or duplicated",
            ));
        }
        let version = require_ascii_token_string(object, "version", MAX_VERSION_BYTES, ".+-_")?;
        result.push(Dependency { id, version });
    }
    Ok(result)
}

fn parse_permissions(values: &[JsonValue]) -> Result<Vec<String>, ManifestError> {
    if values.len() > MAX_PERMISSION_COUNT {
        return Err(manifest_error(
            "invalid_manifest",
            "permissions must be a bounded array",
        ));
    }
    let mut names = BTreeSet::new();
    let mut result = Vec::new();
    for value in values {
        let permission = value
            .as_string()
            .ok_or_else(|| manifest_error("invalid_manifest", "permission must be a string"))?;
        if permission.len() > 64
            || !is_ascii_token(permission, "-_")
            || !names.insert(permission.to_owned())
        {
            return Err(manifest_error(
                "invalid_manifest",
                "permission is invalid or duplicated",
            ));
        }
        result.push(permission.to_owned());
    }
    Ok(result)
}

fn parse_manifest_files(
    format_version: u64,
    package_id: &PackageId,
    object: &[(String, JsonValue)],
) -> Result<Vec<VerifiedArtifact>, ManifestError> {
    let files_key = if format_version == MANIFEST_FORMAT_VERSION_V1 {
        "files"
    } else {
        "payload"
    };
    let values = require_array(object, files_key)?;
    if values.is_empty() || values.len() > MAX_FILE_COUNT {
        return Err(manifest_error(
            "invalid_manifest",
            "files must be a non-empty bounded array",
        ));
    }

    let mut exact_paths = BTreeSet::new();
    let mut windows_paths = BTreeSet::new();
    let mut total_size = 0_u64;
    let mut result = Vec::new();
    for value in values {
        let file = value
            .as_object()
            .ok_or_else(|| manifest_error("invalid_manifest", "expected a JSON object"))?;
        if format_version == MANIFEST_FORMAT_VERSION_V1 {
            require_object_keys(file, &["path", "size", "sha256"], &[])?;
        } else {
            require_object_keys(file, &["path", "size", "hashes"], &[])?;
        }

        let path = SafeRelativePackagePath::parse(&require_string(
            file,
            "path",
            MAX_PACKAGE_PATH_BYTES,
            false,
        )?)
        .map_err(|_| manifest_error("invalid_manifest", "file path is invalid"))?;
        let size = require_unsigned(file, "size", "invalid_manifest")?;
        if size > MAX_FILE_BYTES || total_size > MAX_PAYLOAD_BYTES.saturating_sub(size) {
            return Err(manifest_error(
                "invalid_manifest",
                "file entry violates path, hash or resource limits",
            ));
        }
        total_size += size;
        if !exact_paths.insert(path.as_str().to_owned())
            || !windows_paths.insert(path.as_str().to_lowercase())
        {
            return Err(manifest_error(
                "invalid_manifest",
                "file entry violates path, hash or resource limits",
            ));
        }

        let hashes = if format_version == MANIFEST_FORMAT_VERSION_V1 {
            PayloadHashes::v1_sha256(
                HexDigest32::parse(&require_string(file, "sha256", 64, false)?)
                    .map_err(|_| manifest_error("invalid_manifest", "file hash is invalid"))?,
            )
        } else {
            let hashes = require_object(file, "hashes")?;
            require_object_keys(hashes, &["blake3"], &["sha256"])?;
            let blake3 = HexDigest32::parse(&require_string(hashes, "blake3", 64, false)?)
                .map_err(|_| manifest_error("invalid_manifest", "file hash is invalid"))?;
            let sha256 = if object_contains(hashes, "sha256") {
                Some(
                    HexDigest32::parse(&require_string(hashes, "sha256", 64, false)?)
                        .map_err(|_| manifest_error("invalid_manifest", "file hash is invalid"))?,
                )
            } else {
                None
            };
            PayloadHashes::v2_blake3(blake3, sha256)
        };
        result.push(VerifiedArtifact::new(
            package_id.clone(),
            path,
            size,
            hashes,
        ));
    }
    Ok(result)
}

fn require_object_keys(
    object: &[(String, JsonValue)],
    required: &[&str],
    optional: &[&str],
) -> Result<(), ManifestError> {
    for key in required {
        if !object_contains(object, key) {
            return Err(manifest_error(
                "invalid_manifest",
                format!("missing required key: {key}"),
            ));
        }
    }
    for (key, _) in object {
        if !required
            .iter()
            .chain(optional.iter())
            .any(|allowed| *allowed == key)
        {
            return Err(manifest_error(
                "invalid_manifest",
                format!("unknown key: {key}"),
            ));
        }
    }
    Ok(())
}

fn require_string(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
    allow_empty: bool,
) -> Result<String, ManifestError> {
    let value = object_get(object, key)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| manifest_error("invalid_manifest", format!("{key} must be a string")))?;
    if (!allow_empty && value.is_empty()) || value.len() > maximum || value.contains('\0') {
        return Err(manifest_error(
            "invalid_manifest",
            format!("{key} has an invalid length"),
        ));
    }
    Ok(value.to_owned())
}

fn require_ascii_token_string(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
    extra: &str,
) -> Result<String, ManifestError> {
    let value = require_string(object, key, maximum, false)?;
    if !is_ascii_token(&value, extra) {
        return Err(manifest_error(
            "invalid_manifest",
            format!("{key} is not an accepted token"),
        ));
    }
    Ok(value)
}

fn require_unsigned(
    object: &[(String, JsonValue)],
    key: &str,
    error_code: &'static str,
) -> Result<u64, ManifestError> {
    object_get(object, key)
        .and_then(JsonValue::as_number)
        .ok_or_else(|| manifest_error(error_code, format!("{key} must be an unsigned integer")))
}

fn require_array<'a>(
    object: &'a [(String, JsonValue)],
    key: &str,
) -> Result<&'a [JsonValue], ManifestError> {
    object_get(object, key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| manifest_error("invalid_manifest", format!("{key} must be an array")))
}

fn require_object<'a>(
    object: &'a [(String, JsonValue)],
    key: &str,
) -> Result<&'a [(String, JsonValue)], ManifestError> {
    object_get(object, key)
        .and_then(JsonValue::as_object)
        .ok_or_else(|| manifest_error("invalid_manifest", format!("{key} must be an object")))
}

fn require_object_keys_with_code(
    object: &[(String, JsonValue)],
    required: &[&str],
    optional: &[&str],
    _code: &'static str,
) -> Result<(), KeyringError> {
    for key in required {
        if !object_contains(object, key) {
            return Err(keyring_error(format!("missing required key: {key}")));
        }
    }
    for (key, _) in object {
        if !required
            .iter()
            .chain(optional.iter())
            .any(|allowed| *allowed == key)
        {
            return Err(keyring_error(format!("unknown key: {key}")));
        }
    }
    Ok(())
}

fn require_object_for_code<'a>(
    object: &'a [(String, JsonValue)],
    key: &str,
    _code: &'static str,
) -> Result<&'a [(String, JsonValue)], KeyringError> {
    object_get(object, key)
        .and_then(JsonValue::as_object)
        .ok_or_else(|| keyring_error(format!("{key} must be an object")))
}

fn require_string_for_code(
    object: &[(String, JsonValue)],
    key: &str,
    maximum: usize,
    allow_empty: bool,
    _code: &'static str,
) -> Result<String, KeyringError> {
    let value = object_get(object, key)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| keyring_error(format!("{key} must be a string")))?;
    if (!allow_empty && value.is_empty()) || value.len() > maximum || value.contains('\0') {
        return Err(keyring_error(format!("{key} has an invalid length")));
    }
    Ok(value.to_owned())
}

fn object_contains(object: &[(String, JsonValue)], key: &str) -> bool {
    object.iter().any(|(candidate, _)| candidate == key)
}

fn object_get<'a>(object: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    object
        .iter()
        .rev()
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
}

fn is_ascii_token(value: &str, extra: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || extra.as_bytes().contains(&byte))
}

fn decode_base64(value: &str) -> Result<Vec<u8>, ()> {
    if value.is_empty() || !value.len().is_multiple_of(4) {
        return Err(());
    }
    let mut output = Vec::with_capacity(value.len() / 4 * 3);
    let chunk_count = value.len() / 4;
    for index in 0..chunk_count {
        let chunk = &value.as_bytes()[index * 4..index * 4 + 4];
        let last = index + 1 == chunk_count;
        let a = base64_value(chunk[0]).ok_or(())?;
        let b = base64_value(chunk[1]).ok_or(())?;
        let c = if chunk[2] == b'=' {
            if !last || chunk[3] != b'=' {
                return Err(());
            }
            None
        } else {
            Some(base64_value(chunk[2]).ok_or(())?)
        };
        let d = if chunk[3] == b'=' {
            if !last {
                return Err(());
            }
            None
        } else {
            Some(base64_value(chunk[3]).ok_or(())?)
        };
        output.push((a << 2) | (b >> 4));
        if let Some(c) = c {
            output.push((b << 4) | (c >> 2));
            if let Some(d) = d {
                output.push((c << 6) | d);
            }
        }
    }
    Ok(output)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum JsonValue {
    Object(Vec<(String, JsonValue)>),
    Array(Vec<JsonValue>),
    String(String),
    Number(u64),
    Bool,
    Null,
}

impl JsonValue {
    fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<u64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    input: &'a str,
    index: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, index: 0 }
    }

    fn parse(mut self) -> Result<JsonValue, String> {
        let value = self.parse_value()?;
        self.skip_whitespace();
        if self.index != self.input.len() {
            return Err("manifest has trailing bytes".to_owned());
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.peek_byte() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            Some(b't') => self.consume_literal("true").map(|()| JsonValue::Bool),
            Some(b'f') => self.consume_literal("false").map(|()| JsonValue::Bool),
            Some(b'n') => self.consume_literal("null").map(|()| JsonValue::Null),
            _ => Err("manifest is not strict JSON".to_owned()),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.expect_byte(b'{')?;
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.consume_byte(b'}') {
            return Ok(JsonValue::Object(values));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            let value = self.parse_value()?;
            values.push((key, value));
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                break;
            }
            self.expect_byte(b',')?;
        }
        Ok(JsonValue::Object(values))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.expect_byte(b'[')?;
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.consume_byte(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.consume_byte(b']') {
                break;
            }
            self.expect_byte(b',')?;
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect_byte(b'"')?;
        let mut output = String::new();
        while let Some(byte) = self.next_byte() {
            match byte {
                b'"' => return Ok(output),
                b'\\' => output.push(self.parse_escape()?),
                0x00..=0x1f => return Err("manifest string contains a control byte".to_owned()),
                _ => {
                    let start = self.index - 1;
                    let character = self.input[start..]
                        .chars()
                        .next()
                        .ok_or_else(|| "manifest string is truncated".to_owned())?;
                    self.index = start + character.len_utf8();
                    output.push(character);
                }
            }
        }
        Err("manifest string is unterminated".to_owned())
    }

    fn parse_escape(&mut self) -> Result<char, String> {
        match self.next_byte() {
            Some(b'"') => Ok('"'),
            Some(b'\\') => Ok('\\'),
            Some(b'/') => Ok('/'),
            Some(b'b') => Ok('\u{0008}'),
            Some(b'f') => Ok('\u{000c}'),
            Some(b'n') => Ok('\n'),
            Some(b'r') => Ok('\r'),
            Some(b't') => Ok('\t'),
            Some(b'u') => self.parse_unicode_escape(),
            _ => Err("manifest string has an invalid escape".to_owned()),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, String> {
        let high = self.parse_hex_quad()?;
        if (0xd800..=0xdbff).contains(&high) {
            let checkpoint = self.index;
            if self.next_byte() != Some(b'\\') || self.next_byte() != Some(b'u') {
                return Err("manifest string has an unpaired surrogate".to_owned());
            }
            let low = self.parse_hex_quad()?;
            if !(0xdc00..=0xdfff).contains(&low) {
                self.index = checkpoint;
                return Err("manifest string has an invalid surrogate pair".to_owned());
            }
            let codepoint = 0x10000 + (((high - 0xd800) << 10) | (low - 0xdc00));
            char::from_u32(codepoint).ok_or_else(|| "manifest unicode escape is invalid".to_owned())
        } else if (0xdc00..=0xdfff).contains(&high) {
            Err("manifest string has an unpaired surrogate".to_owned())
        } else {
            char::from_u32(high).ok_or_else(|| "manifest unicode escape is invalid".to_owned())
        }
    }

    fn parse_hex_quad(&mut self) -> Result<u32, String> {
        let mut value = 0_u32;
        for _ in 0..4 {
            let byte = self
                .next_byte()
                .ok_or_else(|| "manifest unicode escape is truncated".to_owned())?;
            value = value
                .checked_mul(16)
                .and_then(|prefix| byte_to_hex(byte).map(|digit| prefix + digit))
                .ok_or_else(|| "manifest unicode escape is invalid".to_owned())?;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<u64, String> {
        let start = self.index;
        if self.consume_byte(b'0') {
            if matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err("manifest number has a leading zero".to_owned());
            }
        } else {
            self.consume_digits()?;
        }
        if matches!(self.peek_byte(), Some(b'.' | b'e' | b'E')) {
            return Err("manifest number must be an unsigned integer".to_owned());
        }
        self.input[start..self.index]
            .parse::<u64>()
            .map_err(|_| "manifest number is out of range".to_owned())
    }

    fn consume_digits(&mut self) -> Result<(), String> {
        let start = self.index;
        while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
            self.index += 1;
        }
        if self.index == start {
            return Err("manifest number expected a digit".to_owned());
        }
        Ok(())
    }

    fn consume_literal(&mut self, literal: &str) -> Result<(), String> {
        if self.input[self.index..].starts_with(literal) {
            self.index += literal.len();
            Ok(())
        } else {
            Err("manifest literal is invalid".to_owned())
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), String> {
        if self.consume_byte(expected) {
            Ok(())
        } else {
            Err("manifest is not strict JSON".to_owned())
        }
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.index += 1;
        Some(byte)
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.index).copied()
    }
}

fn byte_to_hex(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = include_str!("../../../tests/fixtures/package_path_corpus.json");

    #[cfg(windows)]
    mod rsa_signing_fixture {
        #![allow(unsafe_code)]

        use std::ffi::{c_int, c_void};

        unsafe extern "system" {
            fn BCryptOpenAlgorithmProvider(
                ph_algorithm: *mut *mut c_void,
                psz_alg_id: *const u16,
                psz_implementation: *const u16,
                dw_flags: u32,
            ) -> c_int;
            fn BCryptCloseAlgorithmProvider(h_algorithm: *mut c_void, dw_flags: u32) -> c_int;
            fn BCryptGenerateKeyPair(
                h_algorithm: *mut c_void,
                ph_key: *mut *mut c_void,
                dw_length: u32,
                dw_flags: u32,
            ) -> c_int;
            fn BCryptFinalizeKeyPair(h_key: *mut c_void, dw_flags: u32) -> c_int;
            fn BCryptDestroyKey(h_key: *mut c_void) -> c_int;
            fn BCryptExportKey(
                h_key: *mut c_void,
                h_export_key: *mut c_void,
                psz_blob_type: *const u16,
                pb_output: *mut u8,
                cb_output: u32,
                pcb_result: *mut u32,
                dw_flags: u32,
            ) -> c_int;
            fn BCryptSignHash(
                h_key: *mut c_void,
                p_padding_info: *const c_void,
                pb_input: *const u8,
                cb_input: u32,
                pb_output: *mut u8,
                cb_output: u32,
                pcb_result: *mut u32,
                dw_flags: u32,
            ) -> c_int;
        }

        const BCRYPT_RSA_ALGORITHM: &[u16] = &[b'R' as u16, b'S' as u16, b'A' as u16, 0];
        const BCRYPT_RSAPUBLIC_BLOB: &[u16] = &[
            b'R' as u16,
            b'S' as u16,
            b'A' as u16,
            b'P' as u16,
            b'U' as u16,
            b'B' as u16,
            b'L' as u16,
            b'I' as u16,
            b'C' as u16,
            b'B' as u16,
            b'L' as u16,
            b'O' as u16,
            b'B' as u16,
            0,
        ];
        const BCRYPT_SHA256_ALGORITHM: &[u16] = &[
            b'S' as u16,
            b'H' as u16,
            b'A' as u16,
            b'2' as u16,
            b'5' as u16,
            b'6' as u16,
            0,
        ];
        const BCRYPT_PAD_PKCS1: u32 = 0x0000_0002;

        #[repr(C)]
        struct BcryptPkcs1PaddingInfo {
            psz_alg_id: *const u16,
        }

        pub struct RsaSigningFixture {
            algorithm: *mut c_void,
            key: *mut c_void,
        }

        impl RsaSigningFixture {
            pub fn new() -> Self {
                let mut algorithm = std::ptr::null_mut();
                let opened = unsafe {
                    BCryptOpenAlgorithmProvider(
                        &mut algorithm,
                        BCRYPT_RSA_ALGORITHM.as_ptr(),
                        std::ptr::null(),
                        0,
                    )
                };
                assert_eq!(opened, 0, "RSA algorithm provider should open");
                let mut key = std::ptr::null_mut();
                let generated = unsafe { BCryptGenerateKeyPair(algorithm, &mut key, 2048, 0) };
                assert_eq!(generated, 0, "RSA key pair should generate");
                let finalized = unsafe { BCryptFinalizeKeyPair(key, 0) };
                assert_eq!(finalized, 0, "RSA key pair should finalize");
                Self { algorithm, key }
            }

            pub fn public_blob(&self) -> Vec<u8> {
                let mut size = 0_u32;
                let exported_size = unsafe {
                    BCryptExportKey(
                        self.key,
                        std::ptr::null_mut(),
                        BCRYPT_RSAPUBLIC_BLOB.as_ptr(),
                        std::ptr::null_mut(),
                        0,
                        &mut size,
                        0,
                    )
                };
                assert_eq!(exported_size, 0, "RSA public key sizing should succeed");
                let mut blob = vec![0_u8; size as usize];
                let exported = unsafe {
                    BCryptExportKey(
                        self.key,
                        std::ptr::null_mut(),
                        BCRYPT_RSAPUBLIC_BLOB.as_ptr(),
                        blob.as_mut_ptr(),
                        blob.len() as u32,
                        &mut size,
                        0,
                    )
                };
                assert_eq!(exported, 0, "RSA public key export should succeed");
                blob.truncate(size as usize);
                blob
            }

            pub fn sign(&self, message: &[u8]) -> Vec<u8> {
                let hash = super::sha256_bytes(message);
                let padding = BcryptPkcs1PaddingInfo {
                    psz_alg_id: BCRYPT_SHA256_ALGORITHM.as_ptr(),
                };
                let mut size = 0_u32;
                let sized = unsafe {
                    BCryptSignHash(
                        self.key,
                        &padding as *const _ as *const c_void,
                        hash.as_ptr(),
                        hash.len() as u32,
                        std::ptr::null_mut(),
                        0,
                        &mut size,
                        BCRYPT_PAD_PKCS1,
                    )
                };
                assert_eq!(sized, 0, "RSA signature sizing should succeed");
                let mut signature = vec![0_u8; size as usize];
                let signed = unsafe {
                    BCryptSignHash(
                        self.key,
                        &padding as *const _ as *const c_void,
                        hash.as_ptr(),
                        hash.len() as u32,
                        signature.as_mut_ptr(),
                        signature.len() as u32,
                        &mut size,
                        BCRYPT_PAD_PKCS1,
                    )
                };
                assert_eq!(signed, 0, "RSA signature generation should succeed");
                signature.truncate(size as usize);
                signature
            }
        }

        impl Drop for RsaSigningFixture {
            fn drop(&mut self) {
                unsafe {
                    if !self.key.is_null() {
                        let _ = BCryptDestroyKey(self.key);
                    }
                    if !self.algorithm.is_null() {
                        let _ = BCryptCloseAlgorithmProvider(self.algorithm, 0);
                    }
                }
            }
        }
    }

    #[test]
    fn package_path_corpus_matches_frozen_cpp_policy() {
        let cases = parse_path_cases(CORPUS);
        assert!(cases.len() >= 20, "path corpus is unexpectedly small");
        for (path, accepted) in cases {
            assert_eq!(
                is_safe_relative_package_path(&path),
                accepted,
                "package path corpus mismatch for {path:?}"
            );
        }
    }

    #[test]
    fn case_collision_corpus_matches_frozen_cpp_policy() {
        let sets = parse_collision_sets(CORPUS);
        assert!(
            sets.len() >= 2,
            "case collision corpus is unexpectedly small"
        );
        for paths in sets {
            let parsed: Vec<_> = paths
                .iter()
                .map(|path| {
                    SafeRelativePackagePath::parse(path).expect("collision fixture path is valid")
                })
                .collect();
            assert!(
                has_windows_ordinal_case_collision(parsed.iter()),
                "case collision fixture did not collide: {paths:?}"
            );
        }
    }

    #[test]
    fn strong_types_reject_invalid_identity_and_hashes() {
        assert!(PackageId::parse("fcitx5-rime").is_ok());
        assert!(PackageId::parse("Fcitx5-Rime").is_err());
        assert!(SafeRelativePackagePath::parse("bin/addon.dll").is_ok());
        assert!(SafeRelativePackagePath::parse("bin/CON").is_err());
        assert!(HexDigest32::parse(&"a".repeat(64)).is_ok());
        assert!(HexDigest32::parse("not-a-digest").is_err());
    }

    #[test]
    fn sha256_digest_matches_known_answers() {
        assert_eq!(
            sha256_digest(b"").as_str(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_digest(b"abc").as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn blake3_digest_matches_known_answers() {
        assert_eq!(
            blake3_digest(b"").as_str(),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
        assert_eq!(
            blake3_digest(b"abc").as_str(),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
    }

    #[test]
    fn manifest_v1_matches_cpp_golden_shape() {
        let manifest = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("valid v1 manifest should parse");

        assert_eq!(manifest.format_version(), MANIFEST_FORMAT_VERSION_V1);
        assert_eq!(manifest.id().as_str(), "fcitx5-rime");
        assert_eq!(manifest.version(), "1.0.0");
        assert_eq!(manifest.package_type(), &PackageType::Addon);
        assert_eq!(manifest.architecture(), ARCHITECTURE);
        assert_eq!(manifest.core_api(), "1");
        assert_eq!(manifest.addon_abi(), "1");
        assert_eq!(manifest.files().len(), 1);
        assert_eq!(manifest.files()[0].path().as_str(), "bin/addon.dll");
        assert_eq!(manifest.files()[0].size(), 12);
        assert!(manifest.files()[0].hashes().blake3().is_none());
        assert_eq!(
            manifest.files()[0].hashes().sha256().unwrap().as_str(),
            &"a".repeat(64)
        );
    }

    #[test]
    fn manifest_compatibility_matches_cpp_runtime_policy() {
        let manifest = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("valid manifest should parse");
        validate_manifest_compatibility(&manifest, ARCHITECTURE)
            .expect("matching architecture should be compatible");

        let any_arch = parse_manifest(
            &manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12).replace(
                &format!("\"architecture\":\"{ARCHITECTURE}\""),
                "\"architecture\":\"any\"",
            ),
        )
        .expect("any architecture manifest should parse");
        validate_manifest_compatibility(&any_arch, ARCHITECTURE)
            .expect("any architecture should be compatible");

        let opposite_architecture = if ARCHITECTURE == "x64" { "x86" } else { "x64" };
        assert_compatibility_error(&manifest, opposite_architecture);
        assert_compatibility_error(&manifest, "arm64");

        let unsupported_core_api = parse_manifest(
            &manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12)
                .replace("\"core_api\":\"1\"", "\"core_api\":\"2\""),
        )
        .expect("manifest with unsupported API should still parse");
        assert_compatibility_error(&unsupported_core_api, ARCHITECTURE);

        let unsupported_addon_abi = parse_manifest(
            &manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12)
                .replace("\"addon_abi\":\"1\"", "\"addon_abi\":\"2\""),
        )
        .expect("manifest with unsupported ABI should still parse");
        assert_compatibility_error(&unsupported_addon_abi, ARCHITECTURE);

        let core_without_addon_abi = parse_manifest(
            &manifest_v1("fcitx5-core", "1.0.0", &"c".repeat(64), 12)
                .replace("\"type\":\"addon\"", "\"type\":\"core\"")
                .replace("\"addon_abi\":\"1\"", "\"addon_abi\":\"\""),
        )
        .expect("core manifest should parse");
        validate_manifest_compatibility(&core_without_addon_abi, ARCHITECTURE)
            .expect("core package should not require addon ABI");
    }

    #[test]
    fn manifest_v2_requires_blake3_and_accepts_optional_sha256() {
        let manifest = parse_manifest(&manifest_v2(
            "fcitx5-rime",
            "1.0.0",
            &"b".repeat(64),
            12,
            "official-2026-mldsa65",
            Some(&"a".repeat(64)),
        ))
        .expect("valid v2 manifest should parse");

        assert_eq!(manifest.format_version(), MANIFEST_FORMAT_VERSION_V2);
        assert_eq!(manifest.key_id().as_str(), "official-2026-mldsa65");
        assert_eq!(
            manifest.files()[0].hashes().blake3().unwrap().as_str(),
            &"b".repeat(64)
        );
        assert_eq!(
            manifest.files()[0].hashes().sha256().unwrap().as_str(),
            &"a".repeat(64)
        );

        let missing_blake3 = manifest_v2(
            "fcitx5-rime",
            "1.0.0",
            &"b".repeat(64),
            12,
            "official-2026-mldsa65",
            None,
        )
        .replace("\"blake3\":\"", "\"not_blake3\":\"");
        assert_manifest_error("invalid_manifest", &missing_blake3);
    }

    #[test]
    fn manifest_rejects_cpp_invalid_cases() {
        let unsupported = manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12)
            .replace("\"format_version\": 1", "\"format_version\": 99");
        assert_manifest_error("unsupported_manifest", &unsupported);

        let bad_hash = manifest_v1("fcitx5-rime", "1.0.0", "not-a-digest", 12);
        assert_manifest_error("invalid_manifest", &bad_hash);

        let traversal = manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12)
            .replace("bin/addon.dll", "../escape.dll");
        assert_manifest_error("invalid_manifest", &traversal);

        let duplicate_dependency =
            manifest_v1_with_dependencies("[{\"id\":\"fcitx5-rime\",\"version\":\"1\"},{\"id\":\"fcitx5-rime\",\"version\":\"1\"}]");
        assert_manifest_error("invalid_manifest", &duplicate_dependency);

        let case_collision = manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12).replace(
            "{\"path\":\"bin/addon.dll\",\"size\":12,\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}",
            "{\"path\":\"bin/addon.dll\",\"size\":12,\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"},{\"path\":\"BIN/ADDON.DLL\",\"size\":12,\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}",
        );
        assert_manifest_error("invalid_manifest", &case_collision);
    }

    #[test]
    fn trusted_keyring_v2_matches_cpp_pqc_schema() {
        let mldsa = vec![0x41; MLDSA65_PUBLIC_KEY_BYTES];
        let slhdsa = vec![0x42; SLHDSA_SHA2_128S_PUBLIC_KEY_BYTES];
        let keyring = keyring_v2(&mldsa, &slhdsa);
        let keys = parse_trusted_keys(&keyring).expect("valid PQC keyring should parse");

        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].id().as_str(), "official-2026-mldsa65");
        assert_eq!(keys[0].algorithm(), &TrustAlgorithm::Mldsa65);
        assert_eq!(keys[0].public_key().len(), MLDSA65_PUBLIC_KEY_BYTES);
        assert!(!keys[0].revoked());
        assert!(keys[1].revoked());
        assert_eq!(keys[2].algorithm(), &TrustAlgorithm::SlhdsaSha2_128s);

        let bad_mldsa_length =
            keyring.replacen(&base64_for_test(&mldsa), &base64_for_test(&slhdsa), 1);
        assert_keyring_error(&bad_mldsa_length);

        let duplicate = keyring.replace("official-2026-mldsa65-revoked", "official-2026-mldsa65");
        assert_keyring_error(&duplicate);

        let unsupported_required = keyring.replacen("\"mldsa65\"", "\"ed25519\"", 1);
        assert_keyring_error(&unsupported_required);
    }

    #[test]
    fn signature_envelope_v2_matches_cpp_schema() {
        let mldsa_signature = vec![0x43; 3309];
        let slhdsa_signature = vec![0x44; 7856];
        let index = signature_envelope(
            "repository-index",
            &[
                (
                    "official-2026-mldsa65",
                    "mldsa65",
                    mldsa_signature.as_slice(),
                ),
                (
                    "official-2026-slh-dsa-recovery",
                    "slhdsa-sha2-128s",
                    slhdsa_signature.as_slice(),
                ),
            ],
        );
        let parsed = parse_signature_envelope(&index, SignedObject::RepositoryIndex)
            .expect("repository index envelope should parse");
        assert_eq!(parsed.format_version(), 2);
        assert_eq!(parsed.signed_object(), &SignedObject::RepositoryIndex);
        assert_eq!(
            parsed.canonicalization(),
            SIGNATURE_ENVELOPE_CANONICALIZATION
        );
        assert_eq!(parsed.signatures().len(), 2);
        assert_eq!(parsed.signatures()[0].algorithm(), &TrustAlgorithm::Mldsa65);
        assert_eq!(parsed.signatures()[0].signature().len(), 3309);

        assert_signature_error(&index, SignedObject::PackageManifest);

        let manifest = signature_envelope(
            "package-manifest",
            &[(
                "official-2026-mldsa65",
                "mldsa65",
                mldsa_signature.as_slice(),
            )],
        );
        assert!(
            parse_signature_envelope(&manifest, SignedObject::PackageManifest).is_ok(),
            "package manifest envelope should parse"
        );

        let missing_mldsa = signature_envelope(
            "package-manifest",
            &[(
                "official-2026-slh-dsa-recovery",
                "slhdsa-sha2-128s",
                slhdsa_signature.as_slice(),
            )],
        );
        assert_signature_error(&missing_mldsa, SignedObject::PackageManifest);

        let unsupported = manifest.replacen("\"mldsa65\"", "\"ed25519\"", 1);
        assert_signature_error(&unsupported, SignedObject::PackageManifest);

        let duplicate = index.replace("official-2026-slh-dsa-recovery", "official-2026-mldsa65");
        assert_signature_error(&duplicate, SignedObject::RepositoryIndex);

        let malformed = manifest.replace(&base64_for_test(&mldsa_signature), "not base64!");
        assert_signature_error(&malformed, SignedObject::PackageManifest);
    }

    #[cfg(windows)]
    #[test]
    fn repository_index_v1_matches_cpp_schema() {
        let signer = rsa_signing_fixture::RsaSigningFixture::new();
        let trusted = rsa_trusted_key("release-2026", signer.public_blob());
        let repository = format!(
            "{{\"format_version\":1,\"channel\":\"stable\",\"generated_at\":\"2026-08-17T00:00:00Z\",\
             \"key_id\":\"release-2026\",\"packages\":[{{\"id\":\"fcitx5-rime\",\
             \"title\":\"Rime\",\"summary\":\"Rime input engine\",\"version\":\"1.0.0\",\
             \"release_sequence\":1,\"type\":\"addon\",\"architecture\":\"x64\",\
             \"download_url\":\"https://packages.example.invalid/fcitx5-rime.fcpkg\",\
             \"sha256\":\"{}\",\"dependencies\":[]}}]}}",
            "a".repeat(64)
        );
        let signature = signer.sign(repository.as_bytes());
        let parsed = verify_repository_index(
            repository.as_bytes(),
            &signature,
            std::slice::from_ref(&trusted),
            "stable",
        )
        .expect("repository index should verify");
        assert_eq!(parsed.format_version(), 1);
        assert_eq!(parsed.channel(), "stable");
        assert_eq!(
            find_repository_package(&parsed, "fcitx5-rime", "x64")
                .expect("repository package should resolve")
                .version(),
            "1.0.0"
        );

        let tampered = repository.replacen("Rime input", "Fake input", 1);
        assert_eq!(
            verify_repository_index(
                tampered.as_bytes(),
                &signature,
                std::slice::from_ref(&trusted),
                "stable",
            )
            .expect_err("tampered repository should fail")
            .code(),
            "invalid_signature"
        );

        let wrong_key = rsa_signing_fixture::RsaSigningFixture::new();
        let wrong_trusted = rsa_trusted_key("release-2026-other", wrong_key.public_blob());
        let wrong_signature = wrong_key.sign(repository.as_bytes());
        assert_eq!(
            verify_repository_index(
                repository.as_bytes(),
                &wrong_signature,
                std::slice::from_ref(&wrong_trusted),
                "stable",
            )
            .expect_err("wrong trusted key should fail")
            .code(),
            "untrusted_key"
        );

        let mut revoked = trusted.clone();
        revoked.revoked = true;
        assert_eq!(
            verify_repository_index(
                repository.as_bytes(),
                &signature,
                std::slice::from_ref(&revoked),
                "stable",
            )
            .expect_err("revoked key should fail")
            .code(),
            "revoked_key"
        );

        let beta_repository = repository.replace("\"channel\":\"stable\"", "\"channel\":\"beta\"");
        let beta_signature = signer.sign(beta_repository.as_bytes());
        assert_eq!(
            verify_repository_index(
                beta_repository.as_bytes(),
                &beta_signature,
                std::slice::from_ref(&trusted),
                "stable",
            )
            .expect_err("wrong channel should fail")
            .code(),
            "invalid_repository"
        );
    }

    #[cfg(windows)]
    #[test]
    fn repository_index_v2_matches_cpp_schema() {
        let Some(signer) = pqc_fixture_signer_path() else {
            eprintln!("skipping repository v2 Rust verification fixture: signer binary not built");
            return;
        };
        let temp = std::env::temp_dir().join(format!(
            "fcitx5-package-core-repository-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).expect("fixture temp should create");
        let repository = br#"{"format_version":1,"channel":"stable","generated_at":"2026-08-17T00:00:00Z","key_id":"official-2026-mldsa65","packages":[{"id":"fcitx5-rime","title":"Rime","summary":"Rime input engine","version":"1.0.0","release_sequence":1,"type":"addon","architecture":"x64","download_url":"https://packages.example.invalid/fcitx5-rime.fcpkg","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","dependencies":[]}]}"#;
        let repository_path = temp.join("repository.json");
        let signature_path = temp.join("repository.sig.json");
        let keyring_path = temp.join("trusted-keys.json");
        std::fs::write(&repository_path, repository).expect("repository should write");
        let output = std::process::Command::new(&signer)
            .arg("--sign")
            .arg("repository-index")
            .arg(&repository_path)
            .arg(&signature_path)
            .arg(&keyring_path)
            .arg("official-2026-mldsa65")
            .output()
            .expect("fixture signer should run");
        assert!(
            output.status.success(),
            "fixture signer failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let keyring = std::fs::read_to_string(&keyring_path).expect("keyring should read");
        let keys = parse_trusted_keys(&keyring).expect("fixture keyring should parse");
        let envelope_text =
            std::fs::read_to_string(&signature_path).expect("signature envelope should read");
        let envelope = parse_signature_envelope(&envelope_text, SignedObject::RepositoryIndex)
            .expect("fixture repository envelope should parse");
        let parsed = verify_repository_index_envelope(repository, &envelope, &keys, "stable")
            .expect("repository v2 should verify");
        assert_eq!(parsed.key_id(), "official-2026-mldsa65");
        assert!(find_repository_package(&parsed, "fcitx5-rime", "x64").is_some());

        assert_eq!(
            verify_repository_index_envelope(b"tampered", &envelope, &keys, "stable")
                .expect_err("tampered repository should fail")
                .code(),
            "invalid_signature"
        );
        let mut revoked_keys = keys.clone();
        revoked_keys[0].revoked = true;
        assert_eq!(
            verify_repository_index_envelope(repository, &envelope, &revoked_keys, "stable")
                .expect_err("revoked repository key should fail")
                .code(),
            "revoked_key"
        );
        assert_eq!(
            verify_repository_index_envelope(repository, &envelope, &[], "stable")
                .expect_err("untrusted repository key should fail")
                .code(),
            "untrusted_key"
        );
        let beta_repository = repository
            .iter()
            .copied()
            .collect::<Vec<u8>>()
            .into_iter()
            .collect::<Vec<u8>>();
        let mut beta_text = String::from_utf8(beta_repository).expect("repository should be UTF-8");
        beta_text = beta_text.replace("\"channel\":\"stable\"", "\"channel\":\"beta\"");
        let beta_path = temp.join("repository-beta.json");
        std::fs::write(&beta_path, beta_text.as_bytes()).expect("beta repository should write");
        let beta_signature = std::process::Command::new(&signer)
            .arg("--sign")
            .arg("repository-index")
            .arg(&beta_path)
            .arg(&signature_path)
            .arg(&keyring_path)
            .arg("official-2026-mldsa65")
            .output()
            .expect("fixture signer should run");
        assert!(beta_signature.status.success(), "fixture signer failed");
        let beta_envelope_text =
            std::fs::read_to_string(&signature_path).expect("signature envelope should read");
        let beta_envelope =
            parse_signature_envelope(&beta_envelope_text, SignedObject::RepositoryIndex)
                .expect("fixture repository envelope should parse");
        assert_eq!(
            verify_repository_index_envelope(beta_text.as_bytes(), &beta_envelope, &keys, "stable")
                .expect_err("wrong channel should fail")
                .code(),
            "invalid_repository"
        );
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[cfg(windows)]
    #[test]
    fn mldsa65_signature_verification_matches_cpp_fixture() {
        let Some(signer) = pqc_fixture_signer_path() else {
            eprintln!("skipping ML-DSA Rust verification fixture: signer binary not built");
            return;
        };
        let key_id =
            PackageId::parse("official-test-2026-mldsa65").expect("fixture key id should parse");
        let temp =
            std::env::temp_dir().join(format!("fcitx5-package-core-mldsa-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).expect("fixture temp should create");
        let object_path = temp.join("manifest.json");
        let signature_path = temp.join("manifest.sig.json");
        let keyring_path = temp.join("trusted-keys.json");
        let object =
            br#"{"format_version":2,"id":"fcitx5-rime","key_id":"official-test-2026-mldsa65"}"#;
        std::fs::write(&object_path, object).expect("fixture object should write");
        let output = std::process::Command::new(&signer)
            .arg("--sign")
            .arg("package-manifest")
            .arg(&object_path)
            .arg(&signature_path)
            .arg(&keyring_path)
            .arg(key_id.as_str())
            .output()
            .expect("fixture signer should run");
        assert!(
            output.status.success(),
            "fixture signer failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let keyring = std::fs::read_to_string(&keyring_path).expect("keyring should read");
        let mut keys = parse_trusted_keys(&keyring).expect("fixture keyring should parse");
        let envelope_text =
            std::fs::read_to_string(&signature_path).expect("signature envelope should read");
        let envelope = parse_signature_envelope(&envelope_text, SignedObject::PackageManifest)
            .expect("fixture envelope should parse");
        verify_signature_envelope(
            object,
            &envelope,
            &keys,
            SignedObject::PackageManifest,
            &key_id,
        )
        .expect("fixture ML-DSA signature should verify");
        assert_eq!(
            verify_signature_envelope(
                b"tampered",
                &envelope,
                &keys,
                SignedObject::PackageManifest,
                &key_id,
            )
            .expect_err("tampered object should fail")
            .code(),
            "invalid_signature"
        );
        keys[0].revoked = true;
        assert_eq!(
            verify_signature_envelope(
                object,
                &envelope,
                &keys,
                SignedObject::PackageManifest,
                &key_id,
            )
            .expect_err("revoked key should fail")
            .code(),
            "revoked_key"
        );
        assert_eq!(
            verify_signature_envelope(
                object,
                &envelope,
                &[],
                SignedObject::PackageManifest,
                &key_id,
            )
            .expect_err("untrusted key should fail")
            .code(),
            "untrusted_key"
        );
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn dependency_resolution_matches_cpp_exact_ordering() {
        let rime = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("rime manifest should parse");
        let schema = parse_manifest(&manifest_v1_with_dependencies_for(
            "rime-schema-luna",
            "1.0.0",
            &"b".repeat(64),
            12,
            "[{\"id\":\"fcitx5-rime\",\"version\":\"1.0.0\"}]",
        ))
        .expect("schema manifest should parse");
        let order =
            resolve_exact_dependencies(&[schema.clone(), rime.clone()], &["rime-schema-luna"])
                .expect("dependency resolution should succeed");
        assert_eq!(order, vec!["fcitx5-rime", "rime-schema-luna"]);

        let missing_version = parse_manifest(&manifest_v1_with_dependencies_for(
            "rime-schema-luna",
            "1.0.0",
            &"b".repeat(64),
            12,
            "[{\"id\":\"fcitx5-rime\",\"version\":\"2.0.0\"}]",
        ))
        .expect("schema manifest should parse");
        assert_resolution_error(&[missing_version, rime.clone()], &["rime-schema-luna"]);
        assert_resolution_error(&[rime.clone(), rime.clone()], &["fcitx5-rime"]);

        let cycle_a = parse_manifest(&manifest_v1_with_dependencies_for(
            "cycle-a",
            "1.0.0",
            &"c".repeat(64),
            12,
            "[{\"id\":\"cycle-b\",\"version\":\"1.0.0\"}]",
        ))
        .expect("cycle manifest should parse");
        let cycle_b = parse_manifest(&manifest_v1_with_dependencies_for(
            "cycle-b",
            "1.0.0",
            &"d".repeat(64),
            12,
            "[{\"id\":\"cycle-a\",\"version\":\"1.0.0\"}]",
        ))
        .expect("cycle manifest should parse");
        assert_resolution_error(&[cycle_a, cycle_b], &["cycle-a"]);
    }

    #[test]
    fn payload_inventory_matches_cpp_declared_file_contract() {
        let manifest = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("manifest should parse");
        let declared = PayloadEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
        );
        assert!(verify_payload_inventory(&manifest, &[declared.clone()]).is_ok());

        let missing: [PayloadEntry; 0] = [];
        assert_payload_error(&manifest, &missing);
        let wrong_size = PayloadEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            13,
        );
        assert_payload_error(&manifest, &[wrong_size]);
        let extra = PayloadEntry::new(
            SafeRelativePackagePath::parse("bin/extra.dll").expect("path should parse"),
            12,
        );
        assert_payload_error(&manifest, &[declared.clone(), extra]);
        let case_variant = PayloadEntry::new(
            SafeRelativePackagePath::parse("BIN/ADDON.DLL").expect("path should parse"),
            12,
        );
        assert_payload_error(&manifest, &[declared, case_variant]);
    }

    #[test]
    fn payload_digest_matching_keeps_cpp_v1_v2_hash_semantics() {
        let manifest_v1 = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("manifest should parse");
        let v1_observed = PayloadDigestEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
            None,
            Some(HexDigest32::parse(&"a".repeat(64)).expect("hash should parse")),
        );
        assert!(verify_payload_digests(&manifest_v1, &[v1_observed]).is_ok());
        let v1_bad = PayloadDigestEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
            None,
            Some(HexDigest32::parse(&"b".repeat(64)).expect("hash should parse")),
        );
        assert_payload_digest_error(&manifest_v1, &[v1_bad]);

        let manifest_v2 = parse_manifest(&manifest_v2(
            "fcitx5-rime",
            "2.0.0",
            &"c".repeat(64),
            12,
            "official-2026-mldsa65",
            Some(&"d".repeat(64)),
        ))
        .expect("manifest should parse");
        let v2_observed = PayloadDigestEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
            Some(HexDigest32::parse(&"c".repeat(64)).expect("hash should parse")),
            Some(HexDigest32::parse(&"d".repeat(64)).expect("hash should parse")),
        );
        assert!(verify_payload_digests(&manifest_v2, &[v2_observed]).is_ok());
        let v2_bad_blake3 = PayloadDigestEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
            Some(HexDigest32::parse(&"e".repeat(64)).expect("hash should parse")),
            Some(HexDigest32::parse(&"d".repeat(64)).expect("hash should parse")),
        );
        assert_payload_digest_error(&manifest_v2, &[v2_bad_blake3]);
        let v2_bad_sha = PayloadDigestEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            12,
            Some(HexDigest32::parse(&"c".repeat(64)).expect("hash should parse")),
            Some(HexDigest32::parse(&"e".repeat(64)).expect("hash should parse")),
        );
        assert_payload_digest_error(&manifest_v2, &[v2_bad_sha]);
    }

    #[test]
    fn payload_sha256_bytes_are_hashed_before_matching_manifest() {
        let hello_sha256 = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        let manifest_v1 = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", hello_sha256, 5))
            .expect("manifest should parse");
        let observed = PayloadBytesEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            b"hello".to_vec(),
        );
        verify_payload_bytes(&manifest_v1, &[observed])
            .expect("v1 payload bytes should hash to manifest SHA-256");

        let hello_blake3 = blake3_digest(b"hello").as_str().to_owned();
        let manifest_v2_with_sha = parse_manifest(&manifest_v2(
            "fcitx5-rime",
            "1.0.0",
            &hello_blake3,
            5,
            "official-2026-mldsa65",
            Some(hello_sha256),
        ))
        .expect("manifest should parse");
        let observed = PayloadBytesEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            b"hello".to_vec(),
        );
        verify_payload_bytes(&manifest_v2_with_sha, &[observed.clone()])
            .expect("v2 payload bytes should hash to manifest BLAKE3 and SHA-256");
        let manifest_v2_without_sha = parse_manifest(&manifest_v2(
            "fcitx5-rime",
            "1.0.0",
            &hello_blake3,
            5,
            "official-2026-mldsa65",
            None,
        ))
        .expect("manifest should parse");
        verify_payload_bytes(&manifest_v2_without_sha, &[observed])
            .expect("v2 payload bytes should accept required BLAKE3 without optional SHA-256");

        let wrong = PayloadBytesEntry::new(
            SafeRelativePackagePath::parse("bin/addon.dll").expect("path should parse"),
            b"HELLO".to_vec(),
        );
        assert_payload_digest_error_for_bytes(&manifest_v1, &[wrong]);
    }

    #[test]
    fn lockfile_parser_matches_cpp_lifecycle_schema() {
        let lockfile = format!(
            "{{\"format_version\":1,\"packages\":[\
             {{\"id\":\"fcitx5-rime\",\"version\":\"1.0.0\",\
             \"manifest_sha256\":\"{}\",\"state\":\"installed\"}},\
             {{\"id\":\"rime-schema-luna\",\"version\":\"1.0.0\",\
             \"manifest_sha256\":\"{}\",\"state\":\"pending_remove\"}}]}}",
            "a".repeat(64),
            "b".repeat(64)
        );
        let entries = parse_lockfile(&lockfile).expect("valid lockfile should parse");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id().as_str(), "fcitx5-rime");
        assert_eq!(entries[0].version(), "1.0.0");
        assert_eq!(entries[0].manifest_sha256().as_str(), &"a".repeat(64));
        assert_eq!(entries[0].state(), &PackageLifecycleState::Installed);
        assert_eq!(entries[1].state().as_str(), "pending_remove");

        assert_lockfile_error(&lockfile.replace("\"format_version\":1", "\"format_version\":2"));
        assert_lockfile_error(&lockfile.replace("\"installed\"", "\"unknown\""));
        assert_lockfile_error(&lockfile.replace(&"a".repeat(64), "not-a-digest"));
        assert_lockfile_error(&lockfile.replace("rime-schema-luna", "fcitx5-rime"));
        assert_lockfile_error(
            &lockfile.replace("\"version\":\"1.0.0\"", "\"version\":\"bad version\""),
        );
    }

    #[test]
    fn archive_inventory_matches_cpp_payload_entry_policy() {
        let manifest = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("manifest should parse");
        let valid = [
            ArchiveEntry::file("manifest.json", 100),
            ArchiveEntry::file("manifest.sig", 64),
            ArchiveEntry::file("payload/bin/addon.dll", 12),
        ];
        assert!(validate_archive_inventory(&manifest, &valid).is_ok());

        assert_archive_error(
            "invalid_archive",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
            ],
        );
        assert_archive_error(
            "unsafe_archive_path",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
                ArchiveEntry::file("payload/bin/addon.dll", 12),
                ArchiveEntry::file("payload/../escape.dll", 6),
            ],
        );
        assert_archive_error(
            "unsafe_archive_path",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
                ArchiveEntry::file("payload/bin/addon.dll", 12),
                ArchiveEntry::file("payload/BIN/ADDON.DLL", 12),
            ],
        );
        assert_archive_error(
            "invalid_archive",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
                ArchiveEntry::file("payload/bin/addon.dll", 12),
                ArchiveEntry::file("payload/bin/extra.dll", 12),
            ],
        );
        assert_archive_error(
            "payload_mismatch",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
                ArchiveEntry::file("payload/bin/addon.dll", 13),
            ],
        );
        assert_archive_error(
            "invalid_archive",
            &manifest,
            &[
                ArchiveEntry::file("manifest.json", 100),
                ArchiveEntry::file("manifest.sig", 64),
                ArchiveEntry::file("payload/bin/addon.dll", 12).with_unix_symlink(),
            ],
        );
    }

    #[cfg(windows)]
    #[test]
    fn payload_staging_matches_cpp_filesystem_adapter_policy() {
        let hello_blake3 = blake3_digest(b"hello");
        let hello_sha256 = sha256_digest(b"hello");
        let manifest = parse_manifest(&manifest_v2(
            "fcitx5-rime",
            "1.0.0",
            hello_blake3.as_str(),
            5,
            "official-2026-mldsa65",
            Some(hello_sha256.as_str()),
        ))
        .expect("manifest should parse");
        let temp =
            std::env::temp_dir().join(format!("fcitx5-package-core-stage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("payload/bin")).expect("payload root should create");
        std::fs::write(temp.join("payload/bin/addon.dll"), b"hello")
            .expect("payload file should write");
        verify_payload_root(&manifest, temp.join("payload")).expect("payload root should verify");
        let install_root = temp.join("install");
        let staged = stage_verified_payload_tree(
            &manifest,
            br#"{"format_version":2,"id":"fcitx5-rime"}"#,
            temp.join("payload"),
            &install_root,
            "tx-one",
            b"signature",
        )
        .expect("payload should stage");
        assert!(staged.join("payload/bin/addon.dll").exists());
        assert!(staged.join("manifest.json").exists());
        assert!(staged.join("manifest.sig").exists());
        std::fs::write(temp.join("payload/bin/undeclared.dll"), b"oops")
            .expect("undeclared file should write");
        assert_eq!(
            verify_payload_root(&manifest, temp.join("payload"))
                .expect_err("undeclared file should fail")
                .code(),
            "payload_mismatch"
        );
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[cfg(windows)]
    #[test]
    fn archive_zip_staging_matches_cpp_extraction_policy() {
        let hello_blake3 = blake3_digest(b"hello");
        let hello_sha256 = sha256_digest(b"hello");
        let signer = rsa_signing_fixture::RsaSigningFixture::new();
        let trusted = rsa_trusted_key("release-2026", signer.public_blob());
        let manifest_text = manifest_v2(
            "fcitx5-rime",
            "1.0.0",
            hello_blake3.as_str(),
            5,
            "release-2026",
            Some(hello_sha256.as_str()),
        );
        let temp =
            std::env::temp_dir().join(format!("fcitx5-package-core-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).expect("temp should create");
        let valid_archive = temp.join("valid.fcpkg");
        write_store_zip_for_test(
            &valid_archive,
            &[
                ("manifest.json", manifest_text.as_bytes()),
                ("manifest.sig", &signer.sign(manifest_text.as_bytes())),
                ("payload/bin/addon.dll", b"hello"),
            ],
        );
        let install_root = temp.join("install");
        let staged = stage_validated_archive_zip(
            &valid_archive,
            &install_root,
            "tx-zip",
            std::slice::from_ref(&trusted),
        )
        .expect("ZIP archive should stage");
        assert_eq!(
            std::fs::read(staged.join("payload/bin/addon.dll")).expect("payload should read"),
            b"hello"
        );
        assert!(staged.join("manifest.json").exists());
        assert!(staged.join("manifest.sig").exists());

        let traversal_archive = temp.join("traversal.fcpkg");
        write_store_zip_for_test(
            &traversal_archive,
            &[
                ("manifest.json", manifest_text.as_bytes()),
                ("manifest.sig", &signer.sign(manifest_text.as_bytes())),
                ("payload/bin/addon.dll", b"hello"),
                ("payload/../escape.dll", b"escape"),
            ],
        );
        assert_eq!(
            stage_validated_archive_zip(
                &traversal_archive,
                &install_root,
                "tx-traversal",
                std::slice::from_ref(&trusted),
            )
            .expect_err("traversal archive should fail")
            .code(),
            "unsafe_archive_path"
        );

        let collision_archive = temp.join("collision.fcpkg");
        write_store_zip_for_test(
            &collision_archive,
            &[
                ("manifest.json", manifest_text.as_bytes()),
                ("manifest.sig", &signer.sign(manifest_text.as_bytes())),
                ("payload/bin/addon.dll", b"hello"),
                ("payload/BIN/ADDON.DLL", b"hello"),
            ],
        );
        assert_eq!(
            stage_validated_archive_zip(
                &collision_archive,
                &install_root,
                "tx-collision",
                std::slice::from_ref(&trusted),
            )
            .expect_err("case-collision archive should fail")
            .code(),
            "unsafe_archive_path"
        );

        let missing_archive = temp.join("missing.fcpkg");
        write_store_zip_for_test(
            &missing_archive,
            &[
                ("manifest.json", manifest_text.as_bytes()),
                ("manifest.sig", &signer.sign(manifest_text.as_bytes())),
            ],
        );
        assert_eq!(
            stage_validated_archive_zip(
                &missing_archive,
                &install_root,
                "tx-missing",
                std::slice::from_ref(&trusted),
            )
            .expect_err("missing payload archive should fail")
            .code(),
            "invalid_archive"
        );
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[cfg(windows)]
    #[test]
    fn activation_publishes_payload_metadata_and_lockfile_like_cpp() {
        let hello_blake3 = blake3_digest(b"hello");
        let hello_sha256 = sha256_digest(b"hello");
        let signer = rsa_signing_fixture::RsaSigningFixture::new();
        let trusted = rsa_trusted_key("release-2026", signer.public_blob());
        let manifest_text = manifest_v2(
            "fcitx5-rime",
            "1.0.0",
            hello_blake3.as_str(),
            5,
            "release-2026",
            Some(hello_sha256.as_str()),
        );
        let temp = std::env::temp_dir().join(format!(
            "fcitx5-package-core-activate-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).expect("temp should create");
        let archive = temp.join("valid.fcpkg");
        write_store_zip_for_test(
            &archive,
            &[
                ("manifest.json", manifest_text.as_bytes()),
                ("manifest.sig", &signer.sign(manifest_text.as_bytes())),
                ("payload/bin/addon.dll", b"hello"),
            ],
        );
        let install_root = temp.join("install");
        let staged = stage_validated_archive_zip(
            &archive,
            &install_root,
            "tx-activate",
            std::slice::from_ref(&trusted),
        )
        .expect("archive should stage");
        activate_staged_payload_tree(&staged, &install_root, std::slice::from_ref(&trusted))
            .expect("staged payload should activate");
        assert!(install_root
            .join("versions/fcitx5-rime/1.0.0/bin/addon.dll")
            .exists());
        assert!(install_root
            .join("manifests/fcitx5-rime/1.0.0.json")
            .exists());
        assert!(install_root
            .join("manifests/fcitx5-rime/1.0.0.sig")
            .exists());
        let lock = read_installed_lockfile(&install_root).expect("lockfile should parse");
        assert_eq!(lock.len(), 1);
        assert_eq!(lock[0].id().as_str(), "fcitx5-rime");
        assert_eq!(lock[0].version(), "1.0.0");
        assert_eq!(lock[0].state(), &PackageLifecycleState::Installed);

        let bad_manifest = manifest_v2(
            "fcitx5-rime",
            "1.1.0",
            hello_blake3.as_str(),
            5,
            "release-2026",
            Some(hello_sha256.as_str()),
        );
        let bad_archive = temp.join("bad.fcpkg");
        write_store_zip_for_test(
            &bad_archive,
            &[
                ("manifest.json", bad_manifest.as_bytes()),
                ("manifest.sig", &signer.sign(bad_manifest.as_bytes())),
                ("payload/bin/addon.dll", b"hello"),
            ],
        );
        let bad_staged = stage_validated_archive_zip(
            &bad_archive,
            &install_root,
            "tx-bad",
            std::slice::from_ref(&trusted),
        )
        .expect("bad archive should stage before tamper");
        std::fs::write(bad_staged.join("payload/bin/addon.dll"), b"tampered")
            .expect("tamper should write");
        assert_eq!(
            activate_staged_payload_tree(
                &bad_staged,
                &install_root,
                std::slice::from_ref(&trusted),
            )
            .expect_err("tampered staged payload should fail")
            .code(),
            "payload_mismatch"
        );
        let lock_after_failure =
            read_installed_lockfile(&install_root).expect("lockfile should still parse");
        assert_eq!(lock_after_failure.len(), 1);
        assert_eq!(lock_after_failure[0].version(), "1.0.0");
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn lifecycle_state_machine_matches_cpp_lock_rules() {
        let rime = parse_manifest(&manifest_v1("fcitx5-rime", "1.0.0", &"a".repeat(64), 12))
            .expect("manifest should parse");
        let schema = parse_manifest(&manifest_v1_with_dependencies_for(
            "rime-schema-luna",
            "1.0.0",
            &"b".repeat(64),
            12,
            "[{\"id\":\"fcitx5-rime\",\"version\":\"1.0.0\"}]",
        ))
        .expect("manifest should parse");
        let mut lock = vec![
            lock_entry("fcitx5-rime", PackageLifecycleState::Installed),
            lock_entry("rime-schema-luna", PackageLifecycleState::Installed),
        ];

        set_package_state_entries(&mut lock, "fcitx5-rime", PackageLifecycleState::Disabled)
            .expect("state update should succeed");
        assert_eq!(lock[0].state(), &PackageLifecycleState::Disabled);
        assert_lifecycle_error(
            "package_not_found",
            set_package_state_entries(
                &mut lock,
                "missing-package",
                PackageLifecycleState::Disabled,
            ),
        );

        assert_lifecycle_error(
            "package_in_use",
            mark_package_for_removal_entries(
                &mut lock,
                &[rime.clone(), schema.clone()],
                "fcitx5-rime",
            ),
        );
        mark_package_for_removal_entries(&mut lock, &[rime, schema], "rime-schema-luna")
            .expect("leaf package removal should be marked");
        assert_eq!(lock[1].state(), &PackageLifecycleState::PendingRemove);
        assert_lifecycle_error(
            "invalid_state",
            finalize_package_removal_entries(&mut lock.clone(), "fcitx5-rime"),
        );
        finalize_package_removal_entries(&mut lock, "rime-schema-luna")
            .expect("pending removal should finalize");
        assert_eq!(lock.len(), 1);

        let core = parse_manifest(
            &manifest_v1("fcitx5-core", "1.0.0", &"c".repeat(64), 12)
                .replace("\"type\":\"addon\"", "\"type\":\"core\""),
        )
        .expect("core manifest should parse");
        let mut core_lock = vec![lock_entry("fcitx5-core", PackageLifecycleState::Installed)];
        assert_lifecycle_error(
            "protected_package",
            mark_package_for_removal_entries(&mut core_lock, &[core], "fcitx5-core"),
        );
    }

    #[test]
    fn installed_lock_upsert_matches_cpp_activation_sorting() {
        let mut lock = vec![
            lock_entry("fcitx5-rime", PackageLifecycleState::Disabled),
            lock_entry("rime-schema-luna", PackageLifecycleState::Installed),
        ];
        upsert_installed_lock_entry(
            &mut lock,
            PackageId::parse("fcitx5-rime").expect("package id should parse"),
            "2.0.0".to_owned(),
            HexDigest32::parse(&"b".repeat(64)).expect("digest should parse"),
        )
        .expect("existing package should be replaced");

        assert_eq!(lock.len(), 2);
        assert_eq!(lock[0].id().as_str(), "fcitx5-rime");
        assert_eq!(lock[0].version(), "2.0.0");
        assert_eq!(lock[0].state(), &PackageLifecycleState::Installed);
        assert_eq!(lock[0].manifest_sha256().as_str(), &"b".repeat(64));

        upsert_installed_lock_entry(
            &mut lock,
            PackageId::parse("anthy").expect("package id should parse"),
            "1.0.0".to_owned(),
            HexDigest32::parse(&"c".repeat(64)).expect("digest should parse"),
        )
        .expect("new package should be inserted");
        assert_eq!(
            lock.iter()
                .map(|entry| entry.id().as_str())
                .collect::<Vec<_>>(),
            vec!["anthy", "fcitx5-rime", "rime-schema-luna"]
        );

        assert_lifecycle_error(
            "invalid_state",
            upsert_installed_lock_entry(
                &mut lock,
                PackageId::parse("bad-version").expect("package id should parse"),
                "bad version".to_owned(),
                HexDigest32::parse(&"d".repeat(64)).expect("digest should parse"),
            ),
        );
    }

    #[cfg(target_pointer_width = "64")]
    const ARCHITECTURE: &str = "x64";
    #[cfg(not(target_pointer_width = "64"))]
    const ARCHITECTURE: &str = "x86";

    fn manifest_v1(id: &str, version: &str, sha256: &str, size: u64) -> String {
        manifest_v1_with_dependencies_for(id, version, sha256, size, "[]")
    }

    fn manifest_v1_with_dependencies(dependencies: &str) -> String {
        manifest_v1_with_dependencies_for("fcitx5-rime", "1.0.0", &"a".repeat(64), 12, dependencies)
    }

    fn manifest_v1_with_dependencies_for(
        id: &str,
        version: &str,
        sha256: &str,
        size: u64,
        dependencies: &str,
    ) -> String {
        format!(
            "{{\"format_version\": 1,\"id\":\"{id}\",\"version\":\"{version}\",\"type\":\"addon\",\
             \"architecture\":\"{ARCHITECTURE}\",\"min_os\":\"6.1-sp1\",\"core_api\":\"1\",\
             \"addon_abi\":\"1\",\"dependencies\":{dependencies},\"license\":\"MIT\",\
             \"source_commit\":\"0123456789abcdef\",\"permissions\":[\"native-code\",\"input-data\"],\
             \"files\":[{{\"path\":\"bin/addon.dll\",\"size\":{size},\"sha256\":\"{sha256}\"}}],\
             \"key_id\":\"release-2026\"}}"
        )
    }

    #[cfg(windows)]
    fn rsa_trusted_key(id: &str, public_key: Vec<u8>) -> TrustedKey {
        TrustedKey {
            id: PackageId::parse(id).expect("RSA fixture key id should be valid"),
            algorithm: TrustAlgorithm::Rsa2048Sha256,
            public_key,
            revoked: false,
        }
    }

    fn manifest_v2(
        id: &str,
        version: &str,
        blake3: &str,
        size: u64,
        key_id: &str,
        sha256: Option<&str>,
    ) -> String {
        let sha256 = sha256.map_or(String::new(), |value| format!(",\"sha256\":\"{value}\""));
        format!(
            "{{\"format_version\": 2,\"id\":\"{id}\",\"version\":\"{version}\",\"type\":\"addon\",\
             \"architecture\":\"{ARCHITECTURE}\",\"min_os\":\"6.1-sp1\",\"core_api\":\"1\",\
             \"addon_abi\":\"1\",\"dependencies\":[],\"license\":\"MIT\",\
             \"source_commit\":\"0123456789abcdef\",\"permissions\":[\"native-code\",\"input-data\"],\
             \"payload\":[{{\"path\":\"bin/addon.dll\",\"size\":{size},\
             \"hashes\":{{\"blake3\":\"{blake3}\"{sha256}}}}}],\"key_id\":\"{key_id}\"}}"
        )
    }

    fn assert_manifest_error(code: &str, manifest: &str) {
        let error = parse_manifest(manifest).expect_err("manifest should be rejected");
        assert_eq!(error.code(), code);
    }

    fn assert_compatibility_error(manifest: &Manifest, architecture: &str) {
        assert_eq!(
            validate_manifest_compatibility(manifest, architecture)
                .expect_err("manifest should be incompatible")
                .code(),
            "incompatible_package"
        );
    }

    fn keyring_v2(mldsa: &[u8], slhdsa: &[u8]) -> String {
        format!(
            "{{\"format_version\":2,\
             \"policy\":{{\"official_required_signatures\":[\"mldsa65\"],\
             \"compatibility_hashes\":[\"sha256\"],\"default_payload_hash\":\"blake3\"}},\
             \"keys\":[\
             {{\"key_id\":\"official-2026-mldsa65\",\"algorithm\":\"mldsa65\",\
             \"status\":\"trusted\",\"public_key_base64\":\"{}\",\
             \"scope\":[\"repository\",\"package\"],\"channels\":[\"stable\"]}},\
             {{\"key_id\":\"official-2026-mldsa65-revoked\",\"algorithm\":\"mldsa65\",\
             \"status\":\"revoked\",\"public_key_base64\":\"{}\",\
             \"scope\":[\"repository\",\"package\"],\"channels\":[\"stable\"]}},\
             {{\"key_id\":\"official-2026-slh-dsa-recovery\",\
             \"algorithm\":\"slhdsa-sha2-128s\",\"status\":\"trusted\",\
             \"public_key_base64\":\"{}\",\"scope\":[\"repository\"],\
             \"channels\":[\"stable\"]}}]}}",
            base64_for_test(mldsa),
            base64_for_test(mldsa),
            base64_for_test(slhdsa)
        )
    }

    fn signature_envelope(signed_object: &str, signatures: &[(&str, &str, &[u8])]) -> String {
        let entries: Vec<_> = signatures
            .iter()
            .map(|(key_id, algorithm, signature)| {
                format!(
                    "{{\"key_id\":\"{key_id}\",\"algorithm\":\"{algorithm}\",\
                     \"signature_base64\":\"{}\"}}",
                    base64_for_test(signature)
                )
            })
            .collect();
        format!(
            "{{\"format_version\":2,\"signed_object\":\"{signed_object}\",\
             \"canonicalization\":\"{SIGNATURE_ENVELOPE_CANONICALIZATION}\",\
             \"signatures\":[{}]}}",
            entries.join(",")
        )
    }

    #[cfg(windows)]
    fn pqc_fixture_signer_path() -> Option<std::path::PathBuf> {
        [
            "out/build/windows-x64-dev/Debug/fcitx5-pqc-fixture-signer.exe",
            "out/build/windows-x86-dev/Debug/fcitx5-pqc-fixture-signer.exe",
            "out/build/windows-x64-release/Release/fcitx5-pqc-fixture-signer.exe",
            "out/build/windows-x86-release/Release/fcitx5-pqc-fixture-signer.exe",
        ]
        .iter()
        .map(std::path::PathBuf::from)
        .find(|path| path.is_file())
    }

    fn assert_keyring_error(keyring: &str) {
        assert_eq!(
            parse_trusted_keys(keyring)
                .expect_err("keyring should be rejected")
                .code(),
            "invalid_keyring"
        );
    }

    fn assert_signature_error(envelope: &str, expected: SignedObject) {
        assert_eq!(
            parse_signature_envelope(envelope, expected)
                .expect_err("signature envelope should be rejected")
                .code(),
            "invalid_signature"
        );
    }

    fn assert_resolution_error(available: &[Manifest], requested_ids: &[&str]) {
        assert_eq!(
            resolve_exact_dependencies(available, requested_ids)
                .expect_err("resolution should fail")
                .code(),
            "resolution_failed"
        );
    }

    fn assert_payload_error(manifest: &Manifest, observed: &[PayloadEntry]) {
        assert_eq!(
            verify_payload_inventory(manifest, observed)
                .expect_err("payload inventory should be rejected")
                .code(),
            "payload_mismatch"
        );
    }

    fn assert_payload_digest_error(manifest: &Manifest, observed: &[PayloadDigestEntry]) {
        assert_eq!(
            verify_payload_digests(manifest, observed)
                .expect_err("payload digest should be rejected")
                .code(),
            "payload_mismatch"
        );
    }

    fn assert_payload_digest_error_for_bytes(manifest: &Manifest, observed: &[PayloadBytesEntry]) {
        assert_eq!(
            verify_payload_bytes(manifest, observed)
                .expect_err("payload bytes should be rejected")
                .code(),
            "payload_mismatch"
        );
    }

    fn assert_lockfile_error(lockfile: &str) {
        assert_eq!(
            parse_lockfile(lockfile)
                .expect_err("lockfile should be rejected")
                .code(),
            "invalid_lockfile"
        );
    }

    fn assert_archive_error(code: &str, manifest: &Manifest, entries: &[ArchiveEntry]) {
        assert_eq!(
            validate_archive_inventory(manifest, entries)
                .expect_err("archive inventory should be rejected")
                .code(),
            code
        );
    }

    fn assert_lifecycle_error(result_code: &str, result: Result<(), LifecycleError>) {
        assert_eq!(
            result.expect_err("lifecycle transition should fail").code(),
            result_code
        );
    }

    fn lock_entry(id: &str, state: PackageLifecycleState) -> LockEntry {
        LockEntry::new(
            PackageId::parse(id).expect("id should parse"),
            "1.0.0".to_owned(),
            HexDigest32::parse(&"a".repeat(64)).expect("hash should parse"),
            state,
        )
    }

    fn base64_for_test(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = *chunk.get(1).unwrap_or(&0);
            let b2 = *chunk.get(2).unwrap_or(&0);
            output.push(ALPHABET[(b0 >> 2) as usize] as char);
            output.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
            if chunk.len() > 1 {
                output.push(ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                output.push('=');
            }
            if chunk.len() > 2 {
                output.push(ALPHABET[(b2 & 0x3f) as usize] as char);
            } else {
                output.push('=');
            }
        }
        output
    }

    #[cfg(windows)]
    fn write_store_zip_for_test(path: &std::path::Path, entries: &[(&str, &[u8])]) {
        let mut archive = Vec::new();
        let mut central_directory = Vec::new();
        for (name, contents) in entries {
            let offset = archive.len() as u32;
            let name_bytes = name.as_bytes();
            let crc = crc32_for_test(contents);
            push_u32_le(&mut archive, 0x0403_4b50);
            push_u16_le(&mut archive, 20);
            push_u16_le(&mut archive, 0);
            push_u16_le(&mut archive, 0);
            push_u16_le(&mut archive, 0);
            push_u16_le(&mut archive, 0);
            push_u32_le(&mut archive, crc);
            push_u32_le(&mut archive, contents.len() as u32);
            push_u32_le(&mut archive, contents.len() as u32);
            push_u16_le(&mut archive, name_bytes.len() as u16);
            push_u16_le(&mut archive, 0);
            archive.extend_from_slice(name_bytes);
            archive.extend_from_slice(contents);

            push_u32_le(&mut central_directory, 0x0201_4b50);
            push_u16_le(&mut central_directory, 20);
            push_u16_le(&mut central_directory, 20);
            push_u16_le(&mut central_directory, 0);
            push_u16_le(&mut central_directory, 0);
            push_u16_le(&mut central_directory, 0);
            push_u16_le(&mut central_directory, 0);
            push_u32_le(&mut central_directory, crc);
            push_u32_le(&mut central_directory, contents.len() as u32);
            push_u32_le(&mut central_directory, contents.len() as u32);
            push_u16_le(&mut central_directory, name_bytes.len() as u16);
            push_u16_le(&mut central_directory, 0);
            push_u16_le(&mut central_directory, 0);
            push_u16_le(&mut central_directory, 0);
            push_u16_le(&mut central_directory, 0);
            push_u32_le(&mut central_directory, 0);
            push_u32_le(&mut central_directory, offset);
            central_directory.extend_from_slice(name_bytes);
        }
        let central_offset = archive.len() as u32;
        let central_size = central_directory.len() as u32;
        archive.extend_from_slice(&central_directory);
        push_u32_le(&mut archive, 0x0605_4b50);
        push_u16_le(&mut archive, 0);
        push_u16_le(&mut archive, 0);
        push_u16_le(&mut archive, entries.len() as u16);
        push_u16_le(&mut archive, entries.len() as u16);
        push_u32_le(&mut archive, central_size);
        push_u32_le(&mut archive, central_offset);
        push_u16_le(&mut archive, 0);
        std::fs::write(path, archive).expect("ZIP fixture should write");
    }

    #[cfg(windows)]
    fn crc32_for_test(bytes: &[u8]) -> u32 {
        let mut crc = 0xffff_ffff_u32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                let mask = 0_u32.wrapping_sub(crc & 1);
                crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            }
        }
        !crc
    }

    #[cfg(windows)]
    fn push_u16_le(output: &mut Vec<u8>, value: u16) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    #[cfg(windows)]
    fn push_u32_le(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn parse_path_cases(corpus: &str) -> Vec<(String, bool)> {
        corpus
            .lines()
            .filter_map(|line| {
                let path_start = line.find("\"path\": \"")? + "\"path\": \"".len();
                let path_end = find_json_string_end(line, path_start);
                let accepted_start = line.find("\"accepted\": ")? + "\"accepted\": ".len();
                let accepted = line[accepted_start..].starts_with("true");
                Some((unescape_json_string(&line[path_start..path_end]), accepted))
            })
            .collect()
    }

    fn parse_collision_sets(corpus: &str) -> Vec<Vec<String>> {
        corpus
            .lines()
            .filter_map(|line| {
                let paths_start = line.find("\"paths\": [")? + "\"paths\": [".len();
                let paths_end = line[paths_start..].find(']')? + paths_start;
                let inner = &line[paths_start..paths_end];
                let mut paths = Vec::new();
                let mut rest = inner;
                while let Some(start) = rest.find('"') {
                    let value_start = start + 1;
                    let value_end = find_json_string_end(rest, value_start);
                    paths.push(unescape_json_string(&rest[value_start..value_end]));
                    rest = &rest[value_end + 1..];
                }
                Some(paths)
            })
            .collect()
    }

    fn find_json_string_end(text: &str, start: usize) -> usize {
        let bytes = text.as_bytes();
        let mut index = start;
        let mut escaped = false;
        while index < bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                return index;
            }
            index += 1;
        }
        panic!("unterminated JSON string in test corpus");
    }

    fn unescape_json_string(value: &str) -> String {
        let mut output = String::new();
        let mut chars = value.chars();
        while let Some(character) = chars.next() {
            if character != '\\' {
                output.push(character);
                continue;
            }
            match chars.next().expect("dangling JSON escape") {
                '"' => output.push('"'),
                '\\' => output.push('\\'),
                '/' => output.push('/'),
                'b' => output.push('\u{0008}'),
                'f' => output.push('\u{000c}'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                'u' => {
                    let mut hex = String::new();
                    for _ in 0..4 {
                        hex.push(chars.next().expect("short JSON unicode escape"));
                    }
                    let codepoint =
                        u32::from_str_radix(&hex, 16).expect("invalid JSON unicode escape");
                    output.push(char::from_u32(codepoint).expect("invalid JSON unicode codepoint"));
                }
                other => panic!("unsupported JSON escape: {other}"),
            }
        }
        output
    }
}
