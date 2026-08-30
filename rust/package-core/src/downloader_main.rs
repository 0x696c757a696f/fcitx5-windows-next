#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_void, OsStr, OsString};
use std::fs;
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use fcitx5_package_core::{
    sha256_digest, validate_https_repository_url, HexDigest32, PackageFacadeError,
};

type Bool = i32;
type Dword = u32;
type Word = u16;
type Handle = *mut c_void;
type Hinternet = *mut c_void;
type InternetPort = Word;

const TOKEN_QUERY: Dword = 0x0008;
const TOKEN_ELEVATION_CLASS: Dword = 20;
const INTERNET_SCHEME_HTTPS: Dword = 2;
const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: Dword = 0;
const WINHTTP_FLAG_SECURE: Dword = 0x0080_0000;
const WINHTTP_OPTION_REDIRECT_POLICY: Dword = 88;
const WINHTTP_OPTION_REDIRECT_POLICY_NEVER: Dword = 0;
const WINHTTP_QUERY_STATUS_CODE: Dword = 19;
const WINHTTP_QUERY_FLAG_NUMBER: Dword = 0x2000_0000;
const MOVEFILE_WRITE_THROUGH: Dword = 0x0000_0008;
const MAXIMUM_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAXIMUM_DOWNLOAD_BYTES: u64 = 128 * 1024 * 1024;

#[repr(C)]
struct TokenElevation {
    token_is_elevated: Dword,
}

#[repr(C)]
struct UrlComponents {
    struct_size: Dword,
    scheme: *mut u16,
    scheme_length: Dword,
    scheme_id: Dword,
    host_name: *mut u16,
    host_name_length: Dword,
    port: InternetPort,
    user_name: *mut u16,
    user_name_length: Dword,
    password: *mut u16,
    password_length: Dword,
    url_path: *mut u16,
    url_path_length: Dword,
    extra_info: *mut u16,
    extra_info_length: Dword,
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(
        process_handle: Handle,
        desired_access: Dword,
        token_handle: *mut Handle,
    ) -> Bool;
    fn GetTokenInformation(
        token_handle: Handle,
        token_information_class: Dword,
        token_information: *mut c_void,
        token_information_length: Dword,
        return_length: *mut Dword,
    ) -> Bool;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> Handle;
    fn CloseHandle(object: Handle) -> Bool;
    fn MoveFileExW(existing_file_name: *const u16, new_file_name: *const u16, flags: Dword)
        -> Bool;
}

#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpCloseHandle(handle: Hinternet) -> Bool;
    fn WinHttpCrackUrl(
        url: *const u16,
        url_length: Dword,
        flags: Dword,
        components: *mut UrlComponents,
    ) -> Bool;
    fn WinHttpOpen(
        user_agent: *const u16,
        access_type: Dword,
        proxy_name: *const u16,
        proxy_bypass: *const u16,
        flags: Dword,
    ) -> Hinternet;
    fn WinHttpConnect(
        session: Hinternet,
        server_name: *const u16,
        server_port: InternetPort,
        reserved: Dword,
    ) -> Hinternet;
    fn WinHttpOpenRequest(
        connect: Hinternet,
        verb: *const u16,
        object_name: *const u16,
        version: *const u16,
        referrer: *const u16,
        accept_types: *const *const u16,
        flags: Dword,
    ) -> Hinternet;
    fn WinHttpSetOption(
        internet: Hinternet,
        option: Dword,
        buffer: *mut c_void,
        buffer_length: Dword,
    ) -> Bool;
    fn WinHttpSendRequest(
        request: Hinternet,
        headers: *const u16,
        headers_length: Dword,
        optional: *mut c_void,
        optional_length: Dword,
        total_length: Dword,
        context: usize,
    ) -> Bool;
    fn WinHttpReceiveResponse(request: Hinternet, reserved: *mut c_void) -> Bool;
    fn WinHttpQueryHeaders(
        request: Hinternet,
        info_level: Dword,
        name: *const u16,
        buffer: *mut c_void,
        buffer_length: *mut Dword,
        index: *mut Dword,
    ) -> Bool;
    fn WinHttpReadData(
        request: Hinternet,
        buffer: *mut c_void,
        bytes_to_read: Dword,
        bytes_read: *mut Dword,
    ) -> Bool;
}

struct OwnedHandle(Handle);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

struct InternetHandle(Hinternet);

impl InternetHandle {
    fn new(handle: Hinternet) -> Result<Self, DownloadError> {
        if handle.is_null() {
            Err(DownloadError::new("network_error", "WinHTTP handle failed"))
        } else {
            Ok(Self(handle))
        }
    }

    fn get(&self) -> Hinternet {
        self.0
    }
}

impl Drop for InternetHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
}

#[derive(Debug)]
struct DownloadError {
    code: &'static str,
    message: &'static str,
}

impl DownloadError {
    fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

fn wide_z(value: &OsStr) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_wide().collect();
    wide.push(0);
    wide
}

fn wide_string_z(value: &str) -> Vec<u16> {
    wide_z(OsStr::new(value))
}

fn is_elevated() -> bool {
    let mut token: Handle = std::ptr::null_mut();
    let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if opened == 0 || token.is_null() {
        return true;
    }
    let token = OwnedHandle(token);
    let mut elevation = TokenElevation {
        token_is_elevated: 0,
    };
    let mut returned = 0;
    let queried = unsafe {
        GetTokenInformation(
            token.0,
            TOKEN_ELEVATION_CLASS,
            (&mut elevation as *mut TokenElevation).cast::<c_void>(),
            std::mem::size_of::<TokenElevation>() as Dword,
            &mut returned,
        )
    };
    queried != 0 && elevation.token_is_elevated != 0
}

fn crack_https_url(url: &OsStr) -> Option<(Vec<u16>, InternetPort, Vec<u16>)> {
    let text = url.to_string_lossy();
    if text.len() > 4096 || text.contains('#') {
        return None;
    }
    let wide = wide_z(url);
    let mut components = UrlComponents {
        struct_size: std::mem::size_of::<UrlComponents>() as Dword,
        scheme: std::ptr::null_mut(),
        scheme_length: Dword::MAX,
        scheme_id: 0,
        host_name: std::ptr::null_mut(),
        host_name_length: Dword::MAX,
        port: 0,
        user_name: std::ptr::null_mut(),
        user_name_length: Dword::MAX,
        password: std::ptr::null_mut(),
        password_length: Dword::MAX,
        url_path: std::ptr::null_mut(),
        url_path_length: Dword::MAX,
        extra_info: std::ptr::null_mut(),
        extra_info_length: Dword::MAX,
    };
    let cracked =
        unsafe { WinHttpCrackUrl(wide.as_ptr(), (wide.len() - 1) as Dword, 0, &mut components) };
    if cracked == 0
        || components.scheme_id != INTERNET_SCHEME_HTTPS
        || components.host_name.is_null()
        || components.host_name_length == 0
        || !components.user_name.is_null()
        || !components.password.is_null()
    {
        return None;
    }
    let host = unsafe {
        std::slice::from_raw_parts(components.host_name, components.host_name_length as usize)
    }
    .to_vec();
    let path = if components.url_path.is_null() {
        Vec::new()
    } else {
        unsafe {
            std::slice::from_raw_parts(components.url_path, components.url_path_length as usize)
        }
        .to_vec()
    };
    if path.is_empty() {
        return None;
    }
    let mut target = path;
    if !components.extra_info.is_null() && components.extra_info_length > 0 {
        target.extend_from_slice(unsafe {
            std::slice::from_raw_parts(components.extra_info, components.extra_info_length as usize)
        });
    }
    target.push(0);
    let mut host_z = host;
    host_z.push(0);
    Some((host_z, components.port, target))
}

fn partial_path(destination: &Path) -> PathBuf {
    let mut value = OsString::from(destination.as_os_str());
    value.push(".download");
    PathBuf::from(value)
}

fn publish_file(partial: &Path, destination: &Path) -> Result<(), DownloadError> {
    let partial = wide_z(partial.as_os_str());
    let destination = wide_z(destination.as_os_str());
    let moved = unsafe {
        MoveFileExW(
            partial.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(DownloadError::new(
            "io_error",
            "download publication failed",
        ))
    } else {
        Ok(())
    }
}

fn download(
    url: &OsStr,
    expected_hash: Option<&str>,
    destination: &Path,
) -> Result<(), DownloadError> {
    if is_elevated() {
        return Err(DownloadError::new(
            "privilege_boundary",
            "downloader refuses to run elevated",
        ));
    }
    let expected = match expected_hash {
        Some(value) => Some(HexDigest32::parse(value).map_err(|_| {
            DownloadError::new("invalid_download", "destination or hash is invalid")
        })?),
        None => None,
    };
    if !destination.is_absolute() || destination.exists() {
        return Err(DownloadError::new(
            "invalid_download",
            "destination or hash is invalid",
        ));
    }
    let url_text = url.to_str().ok_or_else(|| {
        DownloadError::new("invalid_download", "download URL is not valid Unicode")
    })?;
    validate_https_repository_url(url_text).map_err(facade_error)?;
    let Some((host, port, target)) = crack_https_url(url) else {
        return Err(DownloadError::new(
            "invalid_download",
            "only credential-free HTTPS is allowed",
        ));
    };

    let user_agent = wide_string_z("Fcitx5-Package/1");
    let get = wide_string_z("GET");
    let session = InternetHandle::new(unsafe {
        WinHttpOpen(
            user_agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    })?;
    let connection =
        InternetHandle::new(unsafe { WinHttpConnect(session.get(), host.as_ptr(), port, 0) })?;
    let request = InternetHandle::new(unsafe {
        WinHttpOpenRequest(
            connection.get(),
            get.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        )
    })?;
    let mut redirect_policy: Dword = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
    let sent = unsafe {
        WinHttpSetOption(
            request.get(),
            WINHTTP_OPTION_REDIRECT_POLICY,
            (&mut redirect_policy as *mut Dword).cast::<c_void>(),
            std::mem::size_of::<Dword>() as Dword,
        ) != 0
            && WinHttpSendRequest(
                request.get(),
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
                0,
                0,
            ) != 0
            && WinHttpReceiveResponse(request.get(), std::ptr::null_mut()) != 0
    };
    if !sent {
        return Err(DownloadError::new("network_error", "HTTPS request failed"));
    }
    let mut status: Dword = 0;
    let mut status_size = std::mem::size_of::<Dword>() as Dword;
    let queried = unsafe {
        WinHttpQueryHeaders(
            request.get(),
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            std::ptr::null(),
            (&mut status as *mut Dword).cast::<c_void>(),
            &mut status_size,
            std::ptr::null_mut(),
        )
    };
    if queried == 0 || status != 200 {
        return Err(DownloadError::new(
            "network_error",
            "repository returned a non-200 response",
        ));
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|_| DownloadError::new("io_error", "download write failed"))?;
    }
    let partial = partial_path(destination);
    if partial.exists() {
        return Err(DownloadError::new(
            "invalid_download",
            "partial destination already exists",
        ));
    }

    let result = (|| {
        let mut output = fs::File::create(&partial)
            .map_err(|_| DownloadError::new("io_error", "download write failed"))?;
        let mut bytes = Vec::new();
        let maximum = if expected.is_some() {
            MAXIMUM_DOWNLOAD_BYTES
        } else {
            MAXIMUM_MANIFEST_BYTES
        };
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            let mut read = 0;
            let ok = unsafe {
                WinHttpReadData(
                    request.get(),
                    buffer.as_mut_ptr().cast::<c_void>(),
                    buffer.len() as Dword,
                    &mut read,
                )
            };
            if ok == 0 {
                return Err(DownloadError::new("network_error", "response read failed"));
            }
            if read == 0 {
                break;
            }
            let read = read as usize;
            if bytes.len() as u64 > maximum.saturating_sub(read as u64) {
                return Err(DownloadError::new(
                    "invalid_download",
                    "download exceeds size budget",
                ));
            }
            output
                .write_all(&buffer[..read])
                .map_err(|_| DownloadError::new("io_error", "download write failed"))?;
            bytes.extend_from_slice(&buffer[..read]);
        }
        output
            .flush()
            .map_err(|_| DownloadError::new("io_error", "download write failed"))?;
        if let Some(expected) = expected {
            let actual = sha256_digest(&bytes);
            if !actual.as_str().eq_ignore_ascii_case(expected.as_str()) {
                return Err(DownloadError::new(
                    "hash_mismatch",
                    "download SHA-256 differs from metadata",
                ));
            }
        }
        publish_file(&partial, destination)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

fn facade_error(error: PackageFacadeError) -> DownloadError {
    DownloadError::new(error.code(), "download URL violates package HTTPS policy")
}

fn version() -> &'static str {
    option_env!("FCITX_WINDOWS_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn usage() -> i32 {
    eprintln!(
        "Usage:\n  fcitx5-downloader --download HTTPS_URL SHA256 ABSOLUTE_DESTINATION\n  fcitx5-downloader --download-signed-metadata HTTPS_URL ABSOLUTE_DESTINATION"
    );
    1
}

fn run(args: &[OsString]) -> i32 {
    if args.len() == 2 && args[1] == "--version" {
        println!("fcitx5-downloader {}", version());
        return 0;
    }
    if args.len() == 2 && args[1] == "--self-test" {
        return if crack_https_url(OsStr::new("http://example.invalid/file")).is_none()
            && crack_https_url(OsStr::new("https://example.invalid/file")).is_some()
        {
            0
        } else {
            1
        };
    }
    let result = if args.len() == 5 && args[1] == "--download" {
        download(
            &args[2],
            Some(&args[3].to_string_lossy()),
            Path::new(&args[4]),
        )
    } else if args.len() == 4 && args[1] == "--download-signed-metadata" {
        download(&args[2], None, Path::new(&args[3]))
    } else {
        return usage();
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{}: {}", error.code, error.message);
            2
        }
    }
}

fn main() {
    let args: Vec<OsString> = std::env::args_os().collect();
    std::process::exit(run(&args));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::ffi::OsStringExt;

    #[test]
    fn self_test_accepts_only_https_without_credentials() {
        assert!(crack_https_url(OsStr::new("http://example.invalid/file")).is_none());
        assert!(crack_https_url(OsStr::new("https://example.invalid/file")).is_some());
        assert!(crack_https_url(OsStr::new("https://user@example.invalid/file")).is_none());
        assert!(crack_https_url(OsStr::new("https://example.invalid/file#frag")).is_none());
    }

    #[test]
    fn invalid_usage_and_version_are_stable() {
        assert_eq!(run(&[OsString::from("fcitx5-downloader")]), 1);
        assert_eq!(
            run(&[
                OsString::from("fcitx5-downloader"),
                OsString::from("--version")
            ]),
            0
        );
    }

    #[test]
    fn rejects_invalid_download_shape_before_network() {
        assert_eq!(
            run(&[
                OsString::from("fcitx5-downloader"),
                OsString::from("--download"),
                OsString::from("https://example.invalid/file"),
                OsString::from("bad-hash"),
                OsString::from("relative.fcpkg"),
            ]),
            2
        );
    }

    #[test]
    fn partial_path_appends_download_suffix() {
        assert_eq!(
            partial_path(Path::new("C:\\tmp\\a.fcpkg")),
            PathBuf::from(OsString::from_wide(
                &"C:\\tmp\\a.fcpkg.download"
                    .encode_utf16()
                    .collect::<Vec<_>>()
            ))
        );
    }

    #[test]
    fn win32_constants_preserve_downloader_contract() {
        assert_eq!(WINHTTP_OPTION_REDIRECT_POLICY, 88);
        assert_eq!(WINHTTP_OPTION_REDIRECT_POLICY_NEVER, 0);
        assert_eq!(MAXIMUM_MANIFEST_BYTES, 1024 * 1024);
        assert_eq!(MAXIMUM_DOWNLOAD_BYTES, 128 * 1024 * 1024);
    }
}
