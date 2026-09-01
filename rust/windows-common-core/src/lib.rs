#![deny(unsafe_op_in_unsafe_fn)]

use std::env;
use std::ffi::{c_void, OsStr, OsString};
use std::fs;
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const VERSION_FALLBACK: &str = env!("CARGO_PKG_VERSION");
const RELEASE_CHANNEL_FALLBACK: &str = "stable";
const ENDPOINT_MAX_WIDE_UNITS: usize = 32_768;
const SMALL_TEXT_FILE_MAX_BYTES: u64 = 64 * 1024;
const MAX_DWORD_MINUS_ONE: u64 = u32::MAX as u64 - 1;
const IPC_MAGIC: u32 = 0x3457_4346;
const IPC_VERSION: u16 = 14;
const IPC_HEADER_SIZE: usize = 64;
const IPC_MAX_HOT_FRAME_SIZE: usize = 256 * 1024;
const ERROR_INVALID_DATA: u32 = 13;
const ERROR_ALREADY_EXISTS: u32 = 183;
const ERROR_SUCCESS: i32 = 0;
const ERROR_TIMEOUT: u32 = 1460;
const KF_FLAG_CREATE: u32 = 0x0000_8000;
const RRF_RT_REG_DWORD: u32 = 0x0000_0010;
const HKEY_CURRENT_USER: *mut c_void = 0x8000_0001usize as *mut c_void;
const DEFAULT_CHARSET: u8 = 1;
const LF_FACESIZE: usize = 32;
const PIPE_ACCESS_DUPLEX: u32 = 0x0000_0003;
const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;
const PIPE_TYPE_BYTE: u32 = 0x0000_0000;
const PIPE_READMODE_BYTE: u32 = 0x0000_0000;
const PIPE_WAIT: u32 = 0x0000_0000;
const PIPE_REJECT_REMOTE_CLIENTS: u32 = 0x0000_0008;
const WAIT_OBJECT_0: u32 = 0;
static NEXT_LAUNCHER_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_PIPE_CLIENT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct LogFontW {
    height: i32,
    width: i32,
    escapement: i32,
    orientation: i32,
    weight: i32,
    italic: u8,
    underline: u8,
    strike_out: u8,
    char_set: u8,
    out_precision: u8,
    clip_precision: u8,
    quality: u8,
    pitch_and_family: u8,
    face_name: [u16; LF_FACESIZE],
}

impl Default for LogFontW {
    fn default() -> Self {
        Self {
            height: 0,
            width: 0,
            escapement: 0,
            orientation: 0,
            weight: 0,
            italic: 0,
            underline: 0,
            strike_out: 0,
            char_set: DEFAULT_CHARSET,
            out_precision: 0,
            clip_precision: 0,
            quality: 0,
            pitch_and_family: 0,
            face_name: [0; LF_FACESIZE],
        }
    }
}

const FOLDERID_LOCAL_APP_DATA: Guid = Guid {
    data1: 0xf1b3_2785,
    data2: 0x6fba,
    data3: 0x4fcf,
    data4: [0x9d, 0x55, 0x7b, 0x8e, 0x7f, 0x15, 0x70, 0x91],
};

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

fn write_bytes(bytes: &[u8], out: *mut u8, capacity: usize) -> usize {
    if !out.is_null() && capacity != 0 {
        let count = bytes.len().min(capacity);
        if count != 0 {
            // SAFETY: The caller supplied writable storage for `capacity` bytes.
            // We copy at most that many initialized bytes.
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, count) };
        }
    }
    bytes.len()
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

fn release_data_directory_from_raw(path: *const u16, len: usize) -> Option<PathBuf> {
    let directory = path_from_raw(path, len)?;
    let mut components = directory.components();
    let first = components.next()?;
    if !matches!(first, Component::Normal(_))
        || !components.all(|component| matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(directory)
}

fn local_app_data_root() -> Option<PathBuf> {
    let mut local_app_data: *mut u16 = std::ptr::null_mut();
    // SAFETY: FOLDERID_LOCAL_APP_DATA is a stable KNOWNFOLDERID, the token is
    // null for the current user, and `local_app_data` is an out pointer freed
    // with CoTaskMemFree on success.
    let status = unsafe {
        SHGetKnownFolderPath(
            &FOLDERID_LOCAL_APP_DATA,
            KF_FLAG_CREATE,
            std::ptr::null_mut(),
            &mut local_app_data,
        )
    };
    if status < 0 || local_app_data.is_null() {
        return None;
    }
    let mut len = 0usize;
    // SAFETY: SHGetKnownFolderPath returns a NUL-terminated UTF-16 string.
    unsafe {
        while *local_app_data.add(len) != 0 {
            len += 1;
        }
    }
    // SAFETY: `len` was measured up to the terminating NUL.
    let path = PathBuf::from(OsString::from_wide(unsafe {
        std::slice::from_raw_parts(local_app_data, len)
    }));
    // SAFETY: Buffer ownership belongs to the caller and is released with
    // CoTaskMemFree according to SHGetKnownFolderPath.
    unsafe {
        CoTaskMemFree(local_app_data.cast());
    }
    Some(path)
}

fn default_data_root_for_module_with_local<F>(
    module_path: &Path,
    data_directory: &Path,
    local_app_data: F,
) -> Option<PathBuf>
where
    F: FnOnce() -> Option<PathBuf>,
{
    if let Some(root) = portable_data_root_for_module(module_path) {
        return Some(root);
    }
    local_app_data().map(|root| root.join(data_directory))
}

fn default_data_root_for_module(module_path: &Path, data_directory: &Path) -> Option<PathBuf> {
    default_data_root_for_module_with_local(module_path, data_directory, local_app_data_root)
}

/// Returns the Fcitx5 user-data root for the current executable.
///
/// This follows the shared portable-install and LocalAppData policy used by the
/// native adapters, so Rust product frontends do not derive a competing path.
#[must_use]
pub fn default_fcitx5_data_root_for_current_process() -> Option<PathBuf> {
    let module_path = env::current_exe().ok()?;
    default_data_root_for_module(&module_path, Path::new("Fcitx5"))
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

#[repr(C)]
/// Windows security attributes borrowed from a live descriptor owner.
///
/// The fields are intentionally private: callers can pass this borrowed value
/// to a native API during the borrow, but cannot safely extract or retain its
/// descriptor pointer.
pub struct SecurityAttributes {
    n_length: u32,
    security_descriptor: *mut c_void,
    inherit_handle: i32,
}

struct PipeSecurityState {
    descriptor: *mut c_void,
    attributes: SecurityAttributes,
}

/// Owns the current user's pipe security descriptor and its native attributes.
///
/// This type intentionally does not implement `Send` or `Sync`: its raw
/// LocalAlloc-backed descriptor is only exposed as an immutable borrow for a
/// synchronous native call on the constructing thread.
pub struct CurrentUserSecurityAttributes {
    state: PipeSecurityState,
}

impl CurrentUserSecurityAttributes {
    /// Resolves the authoritative current process identity and creates its
    /// fail-closed pipe security attributes.
    #[must_use]
    pub fn new() -> Option<Self> {
        let identity = current_identity(std::ptr::null_mut(), 0, std::ptr::null_mut(), 0);
        if identity.status == 0 || identity.user_sid_len == 0 {
            return None;
        }

        let mut user_sid = vec![0_u16; identity.user_sid_len];
        let identity = current_identity(
            user_sid.as_mut_ptr(),
            user_sid.len(),
            std::ptr::null_mut(),
            0,
        );
        if identity.status == 0 || identity.user_sid_len != user_sid.len() {
            return None;
        }

        let user_sid = String::from_utf16(&user_sid).ok()?;
        Self::from_identity(
            identity.service_account != 0,
            identity.session_id,
            identity.secure_desktop != 0,
            &user_sid,
        )
    }

    fn from_identity(
        service_account: bool,
        session_id: u32,
        secure_desktop: bool,
        user_sid: &str,
    ) -> Option<Self> {
        may_launch_user_engine(service_account, session_id, secure_desktop, user_sid)
            .then(|| Self::from_pipe_identity(service_account, session_id, user_sid))?
    }

    fn from_pipe_identity(service_account: bool, session_id: u32, user_sid: &str) -> Option<Self> {
        Some(Self {
            state: pipe_security_state(service_account, session_id, user_sid)?,
        })
    }

    fn into_descriptor(mut self) -> *mut c_void {
        let descriptor = self.state.descriptor;
        self.state.descriptor = std::ptr::null_mut();
        self.state.attributes.security_descriptor = std::ptr::null_mut();
        descriptor
    }

    /// Borrows the native attributes for a synchronous native call.
    #[must_use]
    pub fn attributes(&self) -> &SecurityAttributes {
        &self.state.attributes
    }

    fn native_attributes(&self) -> *mut c_void {
        (&self.state.attributes as *const SecurityAttributes)
            .cast_mut()
            .cast::<c_void>()
    }
}

/// The current interactive user's process identity for namespaced IPC and
/// peer verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentUserRuntimeIdentity {
    process_id: u32,
    session_id: u32,
    user_sid: String,
    executable_path: PathBuf,
    service_account: bool,
    secure_desktop: bool,
}

impl CurrentUserRuntimeIdentity {
    /// Resolves the identity only when this process may run the per-user
    /// launcher and engine.
    #[must_use]
    pub fn current() -> Option<Self> {
        let query = current_identity(std::ptr::null_mut(), 0, std::ptr::null_mut(), 0);
        if query.status == 0 || query.user_sid_len == 0 || query.executable_path_len == 0 {
            return None;
        }
        let mut user_sid = vec![0_u16; query.user_sid_len];
        let mut executable_path = vec![0_u16; query.executable_path_len];
        let identity = current_identity(
            user_sid.as_mut_ptr(),
            user_sid.len(),
            executable_path.as_mut_ptr(),
            executable_path.len(),
        );
        if identity.status == 0
            || identity.user_sid_len != user_sid.len()
            || identity.executable_path_len != executable_path.len()
        {
            return None;
        }
        let user_sid = String::from_utf16(&user_sid).ok()?;
        let executable_path = PathBuf::from(OsString::from_wide(&executable_path));
        may_launch_user_engine(
            identity.service_account != 0,
            identity.session_id,
            identity.secure_desktop != 0,
            &user_sid,
        )
        .then_some(Self {
            process_id: identity.process_id,
            session_id: identity.session_id,
            user_sid,
            executable_path,
            service_account: identity.service_account != 0,
            secure_desktop: identity.secure_desktop != 0,
        })
    }

    /// Returns the process id associated with this identity.
    #[must_use]
    pub const fn process_id(&self) -> u32 {
        self.process_id
    }

    /// Returns the interactive logon session id.
    #[must_use]
    pub const fn session_id(&self) -> u32 {
        self.session_id
    }

    /// Returns the launcher executable path captured with the identity.
    #[must_use]
    pub fn executable_path(&self) -> &Path {
        &self.executable_path
    }

    /// Creates security attributes for named objects restricted to this exact
    /// interactive user and session.
    #[must_use]
    pub fn security_attributes(&self) -> Option<CurrentUserSecurityAttributes> {
        CurrentUserSecurityAttributes::from_identity(
            self.service_account,
            self.session_id,
            self.secure_desktop,
            &self.user_sid,
        )
    }

    /// Creates the generation-aware, current-user/session named-pipe path.
    #[must_use]
    pub fn local_endpoint_name(&self, generation: &str, channel: &str) -> Option<OsString> {
        local_name(
            true,
            &self.user_sid,
            self.session_id,
            generation,
            channel,
            &local_test_namespace().unwrap_or_default(),
        )
        .map(OsString::from)
    }

    /// Creates the generation-aware, current-user/session local kernel-object
    /// name.
    #[must_use]
    pub fn local_object_name(&self, generation: &str, channel: &str) -> Option<OsString> {
        local_name(
            false,
            &self.user_sid,
            self.session_id,
            generation,
            channel,
            &local_test_namespace().unwrap_or_default(),
        )
        .map(OsString::from)
    }

    /// Accepts only a named-pipe client from the same interactive principal
    /// and session.
    #[must_use]
    pub fn verifies_pipe_client(&self, pipe: BorrowedHandle<'_>) -> bool {
        self.verified_pipe_client_process_id(pipe).is_some()
    }

    /// Returns the verified named-pipe client's process ID when it belongs to
    /// this exact interactive principal and session.
    #[must_use]
    pub fn verified_pipe_client_process_id(&self, pipe: BorrowedHandle<'_>) -> Option<u32> {
        let peer = verified_pipe_client_peer(
            pipe.as_raw_handle(),
            self.service_account,
            self.session_id,
            self.secure_desktop,
            &self.user_sid,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
        );
        (peer.status != 0 && peer.process_id != 0).then_some(peer.process_id)
    }
}

/// Returns the shared monotonic deadline clock in milliseconds.
#[must_use]
pub fn monotonic_milliseconds() -> u64 {
    tick_milliseconds()
}

/// Returns the shared absolute deadline `milliseconds` from now.
#[must_use]
pub fn deadline_after(milliseconds: u32) -> u64 {
    deadline_after_milliseconds(milliseconds)
}

/// Reports whether an absolute deadline has not expired.
#[must_use]
pub fn deadline_has_time_remaining(deadline: u64) -> bool {
    deadline_has_time(deadline)
}

/// Resolves the deployment generation for the current executable.
#[must_use]
pub fn current_runtime_generation_for_current_process() -> String {
    current_runtime_generation()
}

fn wide_nul(value: &OsStr) -> Vec<u16> {
    let mut wide = value.encode_wide().collect::<Vec<_>>();
    wide.push(0);
    wide
}

fn owned_kernel_handle(raw: *mut c_void) -> io::Result<OwnedHandle> {
    if raw.is_null() || raw == invalid_handle_value() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: The successful Win32 creation call transfers sole ownership of
    // this non-null, non-invalid handle to the returned owner.
    Ok(unsafe { OwnedHandle::from_raw_handle(raw) })
}

/// Owns a per-user/session named mutex used to elect one launcher instance.
#[must_use = "keep this value alive while the launcher owns the singleton"]
pub struct SingleInstance {
    _handle: OwnedHandle,
    primary: bool,
}

impl SingleInstance {
    /// Acquires a named mutex using the supplied current-user security policy.
    ///
    /// A successful call always owns one mutex handle. `is_primary` distinguishes
    /// the launcher elected to host the command pipe from an existing instance.
    ///
    /// # Errors
    ///
    /// Returns the Win32 creation error when the named mutex cannot be opened
    /// or created.
    pub fn acquire(name: &OsStr, security: &CurrentUserSecurityAttributes) -> io::Result<Self> {
        let name = wide_nul(name);
        if name.len() == 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "empty mutex name",
            ));
        }
        // SAFETY: `name` is NUL-terminated UTF-16 and the security attributes
        // are borrowed for this synchronous call only.
        let raw = unsafe { CreateMutexW(security.native_attributes(), 0, name.as_ptr()) };
        let handle = owned_kernel_handle(raw)?;
        // SAFETY: Reads the thread-local result of CreateMutexW immediately.
        let primary = unsafe { GetLastError() } != ERROR_ALREADY_EXISTS;
        Ok(Self {
            _handle: handle,
            primary,
        })
    }

    /// Returns whether this process created the singleton mutex.
    #[must_use]
    pub const fn is_primary(&self) -> bool {
        self.primary
    }
}

/// Owns a manual-reset named event restricted to the current user/session.
#[must_use = "keep this value alive while another process must observe the event"]
pub struct NamedEvent {
    handle: OwnedHandle,
}

impl NamedEvent {
    /// Creates or opens a manual-reset event under the supplied security policy.
    ///
    /// # Errors
    ///
    /// Returns the Win32 creation error when the event cannot be opened or
    /// created.
    pub fn create(name: &OsStr, security: &CurrentUserSecurityAttributes) -> io::Result<Self> {
        let name = wide_nul(name);
        if name.len() == 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "empty event name",
            ));
        }
        // SAFETY: `name` is NUL-terminated UTF-16 and the security attributes
        // are borrowed for this synchronous call only.
        let raw = unsafe { CreateEventW(security.native_attributes(), 1, 0, name.as_ptr()) };
        Ok(Self {
            handle: owned_kernel_handle(raw)?,
        })
    }

    /// Signals the event.
    ///
    /// # Errors
    ///
    /// Returns the Win32 signaling error.
    pub fn signal(&self) -> io::Result<()> {
        // SAFETY: `handle` owns a live event handle for this synchronous call.
        if unsafe { SetEvent(self.handle.as_raw_handle()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Resets the event to its non-signaled state.
    ///
    /// # Errors
    ///
    /// Returns the Win32 reset error.
    pub fn reset(&self) -> io::Result<()> {
        // SAFETY: `handle` owns a live event handle for this synchronous call.
        if unsafe { ResetEvent(self.handle.as_raw_handle()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Returns whether the event is currently signaled without blocking.
    #[must_use]
    pub fn is_signaled(&self) -> bool {
        // SAFETY: `handle` owns a live waitable handle for this zero-time wait.
        (unsafe { WaitForSingleObject(self.handle.as_raw_handle(), 0) }) == WAIT_OBJECT_0
    }
}

/// Waits for a borrowed waitable handle until `timeout` expires.
#[must_use]
pub fn wait_for_handle(handle: BorrowedHandle<'_>, timeout: Duration) -> bool {
    let Ok(milliseconds) = u32::try_from(timeout.as_millis().min(u128::from(MAX_DWORD_MINUS_ONE)))
    else {
        return false;
    };
    // SAFETY: `handle` remains borrowed and valid for this synchronous wait.
    (unsafe { WaitForSingleObject(handle.as_raw_handle(), milliseconds) }) == WAIT_OBJECT_0
}

/// Checks whether a primary launcher pipe is available without connecting to it.
#[must_use]
pub fn named_pipe_is_available(name: &OsStr, timeout: Duration) -> bool {
    let Ok(milliseconds) = u32::try_from(timeout.as_millis().min(u128::from(MAX_DWORD_MINUS_ONE)))
    else {
        return false;
    };
    let name = wide_nul(name);
    if name.len() == 1 {
        return false;
    }
    // SAFETY: `name` is a live NUL-terminated UTF-16 pipe path.
    (unsafe { WaitNamedPipeW(name.as_ptr(), milliseconds) }) != 0
}

/// Owns one overlapped local named-pipe server instance.
#[must_use = "dropping the server closes its pipe instance"]
pub struct NamedPipeServer {
    handle: OwnedHandle,
}

impl NamedPipeServer {
    /// Creates a same-user/session, remote-client-rejecting duplex pipe.
    ///
    /// # Errors
    ///
    /// Returns a Win32 error for invalid buffer bounds or pipe creation failure.
    pub fn create(
        name: &OsStr,
        security: &CurrentUserSecurityAttributes,
        buffer_bytes: usize,
    ) -> io::Result<Self> {
        let Ok(buffer_bytes) = u32::try_from(buffer_bytes) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "pipe buffer too large",
            ));
        };
        if buffer_bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "empty pipe buffer",
            ));
        }
        let name = wide_nul(name);
        if name.len() == 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "empty pipe name",
            ));
        }
        // SAFETY: `name` is NUL-terminated UTF-16 and the security attributes
        // are borrowed for this synchronous creation call only.
        let raw = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                // Let bounded server worker pools create every listener they
                // need; Windows still enforces the per-process pipe limit.
                255, // PIPE_UNLIMITED_INSTANCES
                buffer_bytes,
                buffer_bytes,
                100,
                security.native_attributes(),
            )
        };
        Ok(Self {
            handle: owned_kernel_handle(raw)?,
        })
    }

    /// Waits for one client until `deadline` or the supplied stop event.
    #[must_use]
    pub fn connect_until(&self, deadline: u64, stop: &NamedEvent) -> bool {
        pipe_connect_client(
            self.handle.as_raw_handle(),
            deadline,
            stop.handle.as_raw_handle(),
        )
    }

    /// Verifies the connected pipe client against this user/session identity.
    #[must_use]
    pub fn verifies_client(&self, identity: &CurrentUserRuntimeIdentity) -> bool {
        identity.verifies_pipe_client(self.handle.as_handle())
    }

    /// Returns the connected client's process ID after enforcing the same
    /// user/session peer policy as [`Self::verifies_client`].
    #[must_use]
    pub fn verified_client_process_id(&self, identity: &CurrentUserRuntimeIdentity) -> Option<u32> {
        identity.verified_pipe_client_process_id(self.handle.as_handle())
    }

    /// Reads exactly `bytes.len()` bytes before `deadline`.
    #[must_use]
    pub fn read_exact(&self, bytes: &mut [u8], deadline: u64) -> bool {
        read_named_pipe_exact(self.handle.as_handle(), bytes, deadline)
    }

    /// Writes all bytes before `deadline`.
    #[must_use]
    pub fn write_all(&self, bytes: &[u8], deadline: u64) -> bool {
        write_named_pipe_all(self.handle.as_handle(), bytes, deadline)
    }

    /// Waits for the connected client to close after consuming a final response.
    ///
    /// This is bounded by `deadline`; it is intended only for process-shutdown
    /// responses where dropping the server immediately could discard unread bytes.
    #[must_use]
    pub fn wait_for_client_disconnect(&self, deadline: u64) -> bool {
        loop {
            // SAFETY: the pipe handle is live and all optional output buffers are
            // null because this call only probes whether the client remains connected.
            if unsafe {
                PeekNamedPipe(
                    self.handle.as_raw_handle(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            } == 0
            {
                return true;
            }
            if monotonic_milliseconds() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

impl Drop for NamedPipeServer {
    fn drop(&mut self) {
        // SAFETY: `handle` remains live until its OwnedHandle field drops. A
        // best-effort disconnect releases a connected client before closure.
        unsafe {
            let _ = DisconnectNamedPipe(self.handle.as_raw_handle());
        }
    }
}

/// Waits for an overlapped named-pipe client connection until `deadline` or
/// the optional stop handle is signaled.
#[must_use]
pub fn connect_named_pipe_client(
    pipe: BorrowedHandle<'_>,
    deadline: u64,
    stop_handle: Option<BorrowedHandle<'_>>,
) -> bool {
    pipe_connect_client(
        pipe.as_raw_handle(),
        deadline,
        stop_handle.map_or(std::ptr::null_mut(), |handle| handle.as_raw_handle()),
    )
}

/// Reads exactly `bytes.len()` bytes from an overlapped named pipe before the
/// absolute deadline expires.
#[must_use]
pub fn read_named_pipe_exact(pipe: BorrowedHandle<'_>, bytes: &mut [u8], deadline: u64) -> bool {
    pipe_transfer(
        pipe.as_raw_handle(),
        false,
        bytes.as_mut_ptr(),
        bytes.len(),
        deadline,
    )
}

/// Writes all bytes to an overlapped named pipe before the absolute deadline
/// expires.
#[must_use]
pub fn write_named_pipe_all(pipe: BorrowedHandle<'_>, bytes: &[u8], deadline: u64) -> bool {
    pipe_transfer(
        pipe.as_raw_handle(),
        true,
        bytes.as_ptr().cast_mut(),
        bytes.len(),
        deadline,
    )
}

impl Drop for PipeSecurityState {
    fn drop(&mut self) {
        if !self.descriptor.is_null() {
            // SAFETY: `descriptor` is a LocalAlloc-owned security descriptor
            // returned by ConvertStringSecurityDescriptorToSecurityDescriptorW.
            unsafe {
                LocalFree(self.descriptor);
            }
            self.descriptor = std::ptr::null_mut();
            self.attributes.security_descriptor = std::ptr::null_mut();
        }
    }
}

fn pipe_security_state(
    service_account: bool,
    session_id: u32,
    user_sid: &str,
) -> Option<PipeSecurityState> {
    let descriptor = pipe_security_descriptor(service_account, session_id, user_sid)?;
    Some(PipeSecurityState {
        descriptor,
        attributes: SecurityAttributes {
            n_length: std::mem::size_of::<SecurityAttributes>() as u32,
            security_descriptor: descriptor,
            inherit_handle: 0,
        },
    })
}

fn system_uses_dark_appearance() -> bool {
    let mut sub_key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
        .encode_utf16()
        .collect();
    sub_key.push(0);
    let mut value_name: Vec<u16> = "AppsUseLightTheme".encode_utf16().collect();
    value_name.push(0);
    let mut light = 1_u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: The registry key and value are NUL-terminated UTF-16 strings,
    // and `light`/`size` are valid output buffers for a REG_DWORD query.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            sub_key.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut light as *mut u32).cast::<c_void>(),
            &mut size,
        )
    };
    status == ERROR_SUCCESS && size == std::mem::size_of::<u32>() as u32 && light == 0
}

fn add_unique_font_family(fonts: &mut Vec<String>, family: &str) {
    let candidate = family.trim();
    if candidate.is_empty() || candidate.starts_with('@') {
        return;
    }
    if fonts
        .iter()
        .any(|existing| existing.to_lowercase() == candidate.to_lowercase())
    {
        return;
    }
    fonts.push(candidate.to_owned());
}

fn font_face_name(log_font: &LogFontW) -> String {
    let len = log_font
        .face_name
        .iter()
        .position(|code_unit| *code_unit == 0)
        .unwrap_or(log_font.face_name.len());
    String::from_utf16_lossy(&log_font.face_name[..len])
}

unsafe extern "system" fn collect_font_family(
    log_font: *const LogFontW,
    _text_metric: *const c_void,
    _font_type: u32,
    data: isize,
) -> i32 {
    if log_font.is_null() || data == 0 {
        return 0;
    }
    // SAFETY: `data` is the Vec<String> pointer supplied to EnumFontFamiliesExW
    // by `discover_system_font_families`, and it remains live for the duration
    // of the synchronous enumeration callback.
    let fonts = unsafe { &mut *(data as *mut Vec<String>) };
    // SAFETY: Windows invokes the callback with a valid LOGFONTW pointer for
    // the current font family entry.
    let family = font_face_name(unsafe { &*log_font });
    add_unique_font_family(fonts, &family);
    i32::from(fonts.len() < 512)
}

fn discover_system_font_families() -> Vec<String> {
    let mut discovered = Vec::new();
    // SAFETY: Passing a null HWND requests a screen DC. The returned handle is
    // released with the same null HWND below.
    let dc = unsafe { GetDC(std::ptr::null_mut()) };
    if !dc.is_null() {
        let mut query = LogFontW::default();
        // SAFETY: `query` and `discovered` are live for the synchronous GDI
        // enumeration. The callback does not retain pointers.
        unsafe {
            EnumFontFamiliesExW(
                dc,
                &mut query,
                Some(collect_font_family),
                (&mut discovered as *mut Vec<String>) as isize,
                0,
            );
        }
        // SAFETY: Releases the DC acquired from GetDC(null).
        unsafe {
            ReleaseDC(std::ptr::null_mut(), dc);
        }
    }
    discovered
}

fn ordered_system_font_families(discovered: &[String]) -> Vec<String> {
    let mut discovered = discovered.to_owned();
    discovered.sort_by_key(|family| family.to_lowercase());

    let mut ordered = Vec::new();
    for preset in [
        "Microsoft YaHei",
        "Segoe UI",
        "Segoe UI Emoji",
        "Noto Sans CJK SC",
        "Cascadia Mono",
        "Consolas",
    ] {
        if let Some(font) = discovered
            .iter()
            .find(|family| family.to_lowercase() == preset.to_lowercase())
        {
            add_unique_font_family(&mut ordered, font);
        }
    }
    for font in &discovered {
        add_unique_font_family(&mut ordered, font);
    }
    if ordered.is_empty() {
        add_unique_font_family(&mut ordered, "Segoe UI");
    }
    ordered
}

fn system_font_family_payload() -> Vec<u16> {
    let ordered = ordered_system_font_families(&discover_system_font_families());
    let mut payload = Vec::new();
    for family in ordered {
        payload.extend(family.encode_utf16());
        payload.push(0);
    }
    payload
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
    fn OpenProcessToken(process: *mut c_void, desired_access: u32, token: *mut *mut c_void) -> i32;
    fn GetTokenInformation(
        token: *mut c_void,
        token_information_class: u32,
        token_information: *mut c_void,
        token_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
    fn ConvertSidToStringSidW(sid: *mut c_void, string_sid: *mut *mut u16) -> i32;
    fn IsWellKnownSid(sid: *mut c_void, well_known_sid_type: i32) -> i32;
    fn RegGetValueW(
        key: *mut c_void,
        sub_key: *const u16,
        value: *const u16,
        flags: u32,
        value_type: *mut u32,
        data: *mut c_void,
        data_size: *mut u32,
    ) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> *mut c_void;
    fn QueryFullProcessImageNameW(
        process: *mut c_void,
        flags: u32,
        exe_name: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
    fn CreateFileW(
        file_name: *const u16,
        desired_access: u32,
        share_mode: u32,
        security_attributes: *mut c_void,
        creation_disposition: u32,
        flags_and_attributes: u32,
        template_file: *mut c_void,
    ) -> *mut c_void;
    fn GetFileInformationByHandle(
        file: *mut c_void,
        file_information: *mut ByHandleFileInformation,
    ) -> i32;
    fn GetFinalPathNameByHandleW(
        file: *mut c_void,
        file_path: *mut u16,
        file_path_size: u32,
        flags: u32,
    ) -> u32;
    fn GetNamedPipeClientProcessId(pipe: *mut c_void, client_process_id: *mut u32) -> i32;
    fn GetNamedPipeServerProcessId(pipe: *mut c_void, server_process_id: *mut u32) -> i32;
    fn GetModuleFileNameW(module: *mut c_void, filename: *mut u16, size: u32) -> u32;
    fn GetCurrentProcessId() -> u32;
    fn GetTickCount64() -> u64;
    fn CreateMutexW(
        mutex_attributes: *mut c_void,
        initial_owner: i32,
        name: *const u16,
    ) -> *mut c_void;
    fn CreateEventW(
        event_attributes: *mut c_void,
        manual_reset: i32,
        initial_state: i32,
        name: *const u16,
    ) -> *mut c_void;
    fn SetEvent(event: *mut c_void) -> i32;
    fn ResetEvent(event: *mut c_void) -> i32;
    fn CreateNamedPipeW(
        name: *const u16,
        open_mode: u32,
        pipe_mode: u32,
        max_instances: u32,
        out_buffer_size: u32,
        in_buffer_size: u32,
        default_timeout: u32,
        security_attributes: *mut c_void,
    ) -> *mut c_void;
    fn DisconnectNamedPipe(pipe: *mut c_void) -> i32;
    fn PeekNamedPipe(
        pipe: *mut c_void,
        buffer: *mut c_void,
        buffer_size: u32,
        bytes_read: *mut u32,
        total_bytes_available: *mut u32,
        bytes_left_this_message: *mut u32,
    ) -> i32;
    fn WriteFile(
        file: *mut c_void,
        buffer: *const c_void,
        number_of_bytes_to_write: u32,
        number_of_bytes_written: *mut u32,
        overlapped: *mut Overlapped,
    ) -> i32;
    fn ReadFile(
        file: *mut c_void,
        buffer: *mut c_void,
        number_of_bytes_to_read: u32,
        number_of_bytes_read: *mut u32,
        overlapped: *mut Overlapped,
    ) -> i32;
    fn WaitNamedPipeW(name: *const u16, timeout: u32) -> i32;
    fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
    fn GetOverlappedResult(
        file: *mut c_void,
        overlapped: *mut Overlapped,
        number_of_bytes_transferred: *mut u32,
        wait: i32,
    ) -> i32;
    fn CancelIoEx(file: *mut c_void, overlapped: *mut Overlapped) -> i32;
    fn GetLastError() -> u32;
    fn SetLastError(error: u32);
    fn LocalFree(memory: *mut c_void) -> *mut c_void;
    fn CloseHandle(object: *mut c_void) -> i32;
    fn ConnectNamedPipe(pipe: *mut c_void, overlapped: *mut Overlapped) -> i32;
    fn WaitForMultipleObjects(
        count: u32,
        handles: *const *mut c_void,
        wait_all: i32,
        milliseconds: u32,
    ) -> u32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetDC(window: *mut c_void) -> *mut c_void;
    fn ReleaseDC(window: *mut c_void, dc: *mut c_void) -> i32;
    fn RegisterWindowMessageW(string: *const u16) -> u32;
    fn PostMessageW(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> i32;
    fn OpenInputDesktop(flags: u32, inherit: i32, desired_access: u32) -> *mut c_void;
    fn GetUserObjectInformationW(
        object: *mut c_void,
        index: i32,
        information: *mut c_void,
        length: u32,
        length_needed: *mut u32,
    ) -> i32;
    fn CloseDesktop(desktop: *mut c_void) -> i32;
}

/// Notifies running Candidate windows that their resolved visual configuration changed.
pub fn broadcast_visual_config_changed() {
    let message_name: Vec<u16> = OsStr::new("Fcitx5WindowsNext.VisualConfigChanged.v1")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `message_name` is a live, NUL-terminated UTF-16 buffer for the call.
    let message = unsafe { RegisterWindowMessageW(message_name.as_ptr()) };
    if message != 0 {
        // SAFETY: HWND_BROADCAST is the documented pseudo-handle 0xffff; this posts a
        // registered message with no borrowed pointer payload.
        let _ = unsafe { PostMessageW(0xffffusize as *mut c_void, message, 0, 0) };
    }
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn EnumFontFamiliesExW(
        dc: *mut c_void,
        log_font: *mut LogFontW,
        callback: Option<
            unsafe extern "system" fn(*const LogFontW, *const c_void, u32, isize) -> i32,
        >,
        data: isize,
        flags: u32,
    ) -> i32;
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHGetKnownFolderPath(
        known_folder_id: *const Guid,
        flags: u32,
        token: *mut c_void,
        path: *mut *mut u16,
    ) -> i32;
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoTaskMemFree(memory: *mut c_void);
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

fn remaining_milliseconds(deadline: u64) -> Option<u32> {
    // SAFETY: Monotonic Windows tick query with no preconditions.
    let now = unsafe { GetTickCount64() };
    if now >= deadline {
        return None;
    }
    Some((deadline - now).min(MAX_DWORD_MINUS_ONE) as u32)
}

fn deadline_after_milliseconds(milliseconds: u32) -> u64 {
    // SAFETY: Monotonic Windows tick query with no preconditions.
    unsafe { GetTickCount64() }.saturating_add(milliseconds as u64)
}

fn tick_milliseconds() -> u64 {
    // SAFETY: Monotonic Windows tick query with no preconditions.
    unsafe { GetTickCount64() }
}

fn deadline_has_time(deadline: u64) -> bool {
    remaining_milliseconds(deadline).is_some()
}

fn pipe_transfer(
    pipe: *mut c_void,
    write: bool,
    data: *mut u8,
    size: usize,
    deadline: u64,
) -> bool {
    pipe_transfer_with_stop(pipe, write, data, size, deadline, std::ptr::null_mut())
}

fn pipe_transfer_with_stop(
    pipe: *mut c_void,
    write: bool,
    data: *mut u8,
    size: usize,
    deadline: u64,
    stop_handle: *mut c_void,
) -> bool {
    const ERROR_IO_PENDING: u32 = 997;
    const WAIT_OBJECT_0: u32 = 0;

    if pipe.is_null() || pipe == invalid_handle_value() || (data.is_null() && size != 0) {
        return false;
    }
    let mut completed = 0_usize;
    while completed < size {
        let Some(wait) = remaining_milliseconds(deadline) else {
            return false;
        };
        let remaining = size - completed;
        if remaining > u32::MAX as usize {
            return false;
        }
        // SAFETY: Creates an unnamed manual-reset event for one overlapped I/O.
        let event = unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null()) };
        if event.is_null() {
            return false;
        }
        let mut operation = Overlapped {
            internal: 0,
            internal_high: 0,
            offset: 0,
            offset_high: 0,
            event,
        };
        let mut transferred = 0_u32;
        // SAFETY: The caller supplies a valid pipe handle and a buffer covering
        // `size` bytes for this operation. Windows validates the handle. The
        // event and OVERLAPPED live until the operation is completed/cancelled.
        let immediate = unsafe {
            if write {
                WriteFile(
                    pipe,
                    data.add(completed).cast::<c_void>(),
                    remaining as u32,
                    &mut transferred,
                    &mut operation,
                )
            } else {
                ReadFile(
                    pipe,
                    data.add(completed).cast::<c_void>(),
                    remaining as u32,
                    &mut transferred,
                    &mut operation,
                )
            }
        };
        let mut success = immediate != 0;
        if !success {
            // SAFETY: Reads the thread-local Win32 error for the I/O call above.
            let error = unsafe { GetLastError() };
            if error == ERROR_IO_PENDING {
                let wait_result = if stop_handle.is_null() {
                    // SAFETY: Waits on the event owned by this operation.
                    unsafe { WaitForSingleObject(event, wait) }
                } else {
                    let handles = [event, stop_handle];
                    // SAFETY: Waits on the operation event and the stop handle;
                    // both are live handles for the duration of the call.
                    unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, wait) }
                };
                if wait_result == WAIT_OBJECT_0 {
                    // SAFETY: The event signaled that this overlapped operation completed.
                    success = unsafe {
                        GetOverlappedResult(pipe, &mut operation, &mut transferred, 0) != 0
                    };
                } else {
                    // SAFETY: Cancels and drains this specific outstanding operation before
                    // closing the event handle.
                    unsafe {
                        CancelIoEx(pipe, &mut operation);
                        GetOverlappedResult(pipe, &mut operation, &mut transferred, 1);
                    }
                    success = false;
                }
            }
        }
        // SAFETY: `event` is a live handle from CreateEventW above.
        unsafe { CloseHandle(event) };
        if !success || transferred == 0 {
            return false;
        }
        completed += transferred as usize;
    }
    true
}

fn pipe_connect_client(pipe: *mut c_void, deadline: u64, stop_handle: *mut c_void) -> bool {
    const ERROR_PIPE_CONNECTED: u32 = 535;
    const ERROR_IO_PENDING: u32 = 997;
    const WAIT_OBJECT_0: u32 = 0;

    if pipe.is_null() || pipe == invalid_handle_value() {
        return false;
    }
    // SAFETY: Creates an unnamed manual-reset event for one overlapped connect.
    let event = unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null()) };
    if event.is_null() {
        return false;
    }
    let mut operation = Overlapped {
        internal: 0,
        internal_high: 0,
        offset: 0,
        offset_high: 0,
        event,
    };
    // SAFETY: The pipe is a live named-pipe server handle created for
    // overlapped I/O; the event/OVERLAPPED live until the connect completes.
    let immediate = unsafe { ConnectNamedPipe(pipe, &mut operation) };
    let mut connected = immediate != 0;
    if !connected {
        // SAFETY: Reads the thread-local Win32 error for the connect call above.
        let error = unsafe { GetLastError() };
        if error == ERROR_PIPE_CONNECTED {
            connected = true;
        } else if error == ERROR_IO_PENDING {
            let Some(wait) = remaining_milliseconds(deadline) else {
                // SAFETY: Cancels and drains the outstanding connect before
                // closing the event handle.
                let mut transferred = 0_u32;
                unsafe {
                    CancelIoEx(pipe, &mut operation);
                    GetOverlappedResult(pipe, &mut operation, &mut transferred, 1);
                    CloseHandle(event);
                }
                return false;
            };
            let wait_result = if stop_handle.is_null() {
                // SAFETY: Waits on the event owned by this operation.
                unsafe { WaitForSingleObject(event, wait) }
            } else {
                let handles = [event, stop_handle];
                // SAFETY: Waits on the operation event and the stop handle;
                // both are live handles for the duration of the call.
                unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, wait) }
            };
            if wait_result == WAIT_OBJECT_0 {
                let mut transferred = 0_u32;
                // SAFETY: The event signaled that the connect completed.
                connected =
                    unsafe { GetOverlappedResult(pipe, &mut operation, &mut transferred, 0) != 0 };
            } else {
                // SAFETY: Cancels and drains this specific outstanding operation
                // before closing the event handle.
                let mut transferred = 0_u32;
                unsafe {
                    CancelIoEx(pipe, &mut operation);
                    GetOverlappedResult(pipe, &mut operation, &mut transferred, 1);
                }
            }
        }
    }
    // SAFETY: `event` is a live handle from CreateEventW above.
    unsafe { CloseHandle(event) };
    connected
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    let chunk = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([chunk[0], chunk[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let chunk = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
}

fn read_u64_le(bytes: &[u8], offset: usize) -> Option<u64> {
    let chunk = bytes.get(offset..offset + 8)?;
    Some(u64::from_le_bytes([
        chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
    ]))
}

fn ipc_response_header_body_size(header: &[u8]) -> Option<usize> {
    if header.len() != IPC_HEADER_SIZE
        || read_u32_le(header, 0)? != IPC_MAGIC
        || read_u16_le(header, 4)? != IPC_VERSION
    {
        return None;
    }
    let message_type = read_u16_le(header, 6)?;
    if !(2..=11).contains(&message_type) || message_type == 3 || message_type == 5 {
        return None;
    }
    let body_size = read_u32_le(header, 8)? as usize;
    let request_id = read_u64_le(header, 12)?;
    let response_to = read_u64_le(header, 20)?;
    if request_id == 0 || response_to == 0 {
        return None;
    }
    (body_size <= IPC_MAX_HOT_FRAME_SIZE - IPC_HEADER_SIZE).then_some(body_size)
}

fn pipe_transact(
    pipe: *mut c_void,
    request: &[u8],
    response_output: *mut u8,
    response_capacity: usize,
    deadline: u64,
) -> Fcitx5WindowsCommonPipeTransact {
    if request.is_empty()
        || request.len() > IPC_MAX_HOT_FRAME_SIZE
        || response_output.is_null()
        || response_capacity < IPC_HEADER_SIZE
        || !pipe_transfer(
            pipe,
            true,
            request.as_ptr() as *mut u8,
            request.len(),
            deadline,
        )
    {
        return Fcitx5WindowsCommonPipeTransact::default();
    }
    let mut header = [0_u8; IPC_HEADER_SIZE];
    if !pipe_transfer(pipe, false, header.as_mut_ptr(), header.len(), deadline) {
        return Fcitx5WindowsCommonPipeTransact::default();
    }
    let Some(body_size) = ipc_response_header_body_size(&header) else {
        return Fcitx5WindowsCommonPipeTransact::default();
    };
    let response_len = IPC_HEADER_SIZE + body_size;
    if response_len > response_capacity {
        return Fcitx5WindowsCommonPipeTransact::default();
    }
    // SAFETY: The caller supplied writable response storage for
    // `response_capacity` bytes. The checked response length is within it.
    unsafe { std::ptr::copy_nonoverlapping(header.as_ptr(), response_output, header.len()) };
    if body_size != 0
        && !pipe_transfer(
            pipe,
            false,
            // SAFETY: `response_len <= response_capacity`, so the body starts
            // inside the same writable allocation.
            unsafe { response_output.add(IPC_HEADER_SIZE) },
            body_size,
            deadline,
        )
    {
        return Fcitx5WindowsCommonPipeTransact::default();
    }
    Fcitx5WindowsCommonPipeTransact {
        status: 1,
        response_len,
    }
}

fn pipe_transact_with_error(
    pipe: *mut c_void,
    request: &[u8],
    response_output: *mut u8,
    response_capacity: usize,
    deadline: u64,
) -> Fcitx5WindowsCommonPipeTransactWithError {
    if request.is_empty()
        || request.len() > IPC_MAX_HOT_FRAME_SIZE
        || response_output.is_null()
        || response_capacity < IPC_HEADER_SIZE
    {
        return Fcitx5WindowsCommonPipeTransactWithError {
            failure_error: ERROR_INVALID_DATA,
            ..Default::default()
        };
    }
    if !pipe_transfer(
        pipe,
        true,
        request.as_ptr() as *mut u8,
        request.len(),
        deadline,
    ) {
        return Fcitx5WindowsCommonPipeTransactWithError {
            failure_error: ERROR_TIMEOUT,
            ..Default::default()
        };
    }
    let mut header = [0_u8; IPC_HEADER_SIZE];
    if !pipe_transfer(pipe, false, header.as_mut_ptr(), header.len(), deadline) {
        return Fcitx5WindowsCommonPipeTransactWithError {
            failure_error: ERROR_TIMEOUT,
            ..Default::default()
        };
    }
    let Some(body_size) = ipc_response_header_body_size(&header) else {
        return Fcitx5WindowsCommonPipeTransactWithError {
            failure_error: ERROR_INVALID_DATA,
            ..Default::default()
        };
    };
    let response_len = IPC_HEADER_SIZE + body_size;
    if response_len > response_capacity {
        return Fcitx5WindowsCommonPipeTransactWithError {
            failure_error: ERROR_INVALID_DATA,
            ..Default::default()
        };
    }
    // SAFETY: The caller supplied writable response storage for
    // `response_capacity` bytes. The checked response length is within it.
    unsafe { std::ptr::copy_nonoverlapping(header.as_ptr(), response_output, header.len()) };
    if body_size != 0
        && !pipe_transfer(
            pipe,
            false,
            // SAFETY: `response_len <= response_capacity`, so the body starts
            // inside the same writable allocation.
            unsafe { response_output.add(IPC_HEADER_SIZE) },
            body_size,
            deadline,
        )
    {
        return Fcitx5WindowsCommonPipeTransactWithError {
            failure_error: ERROR_TIMEOUT,
            ..Default::default()
        };
    }
    Fcitx5WindowsCommonPipeTransactWithError {
        status: 1,
        failure_error: 0,
        response_len,
    }
}

fn open_pipe_client(pipe_name: &[u16], deadline: u64, wait_when_busy: bool) -> *mut c_void {
    const ERROR_PIPE_BUSY: u32 = 231;
    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;

    if pipe_name.is_empty() || remaining_milliseconds(deadline).is_none() {
        return invalid_handle_value();
    }
    let mut name = pipe_name.to_vec();
    name.push(0);
    loop {
        // SAFETY: `name` is an owned null-terminated UTF-16 string. The pipe is
        // opened for overlapped client I/O, matching the previous C++ clients.
        let pipe = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            )
        };
        if !pipe.is_null() && pipe != invalid_handle_value() {
            return pipe;
        }
        // SAFETY: Reads the thread-local Win32 error for CreateFileW above.
        let error = unsafe { GetLastError() };
        let Some(wait) = remaining_milliseconds(deadline) else {
            // SAFETY: Restores the opening failure as the externally visible error.
            unsafe { SetLastError(error) };
            return invalid_handle_value();
        };
        if !wait_when_busy || error != ERROR_PIPE_BUSY || wait == 0 {
            // SAFETY: Restores the opening failure as the externally visible error.
            unsafe { SetLastError(error) };
            return invalid_handle_value();
        }
        // SAFETY: `name` remains alive and null-terminated for the wait call.
        if unsafe { WaitNamedPipeW(name.as_ptr(), wait) } == 0 {
            // SAFETY: Preserve the original busy/open failure, matching the C++
            // retry loop's externally visible error behavior.
            unsafe { SetLastError(error) };
            return invalid_handle_value();
        }
    }
}

fn close_pipe_client(pipe: *mut c_void) {
    if pipe.is_null() || pipe == invalid_handle_value() {
        return;
    }
    // SAFETY: `pipe` is expected to be a live overlapped pipe client handle.
    // CancelIoEx is best-effort cleanup before CloseHandle, matching the
    // previous C++ client shutdown behavior.
    unsafe {
        CancelIoEx(pipe, std::ptr::null_mut());
        CloseHandle(pipe);
    }
}

fn pipe_server_process_id(pipe: *mut c_void) -> Option<u32> {
    if pipe.is_null() || pipe == invalid_handle_value() {
        return None;
    }
    let mut process_id = 0_u32;
    // SAFETY: `pipe` is a caller-supplied named-pipe handle. Windows validates
    // the handle and writes a process id only on success.
    let ok = unsafe { GetNamedPipeServerProcessId(pipe, &mut process_id) };
    (ok != 0 && process_id != 0).then_some(process_id)
}

fn pipe_client_process_id(pipe: *mut c_void) -> Option<u32> {
    if pipe.is_null() || pipe == invalid_handle_value() {
        return None;
    }
    let mut process_id = 0_u32;
    // SAFETY: `pipe` is a caller-supplied named-pipe handle. Windows validates
    // the handle and writes a process id only on success.
    let ok = unsafe { GetNamedPipeClientProcessId(pipe, &mut process_id) };
    (ok != 0 && process_id != 0).then_some(process_id)
}

#[allow(clippy::too_many_arguments)]
fn verify_pipe_server_peer(
    pipe: *mut c_void,
    current_service_account: bool,
    current_session_id: u32,
    current_secure_desktop: bool,
    current_user_sid: &str,
    policy_mode: u32,
    expected_executable_path: &[u16],
    development_exception_enabled: bool,
) -> bool {
    const POLICY_EXACT_EXECUTABLE: u32 = 0;
    const POLICY_DEVELOPMENT_SAME_USER_SESSION: u32 = 1;

    if !may_launch_user_engine(
        current_service_account,
        current_session_id,
        current_secure_desktop,
        current_user_sid,
    ) {
        return false;
    }
    let Some(server_process_id) = pipe_server_process_id(pipe) else {
        return false;
    };
    let query = process_identity_with_executable_file(
        server_process_id,
        std::ptr::null_mut(),
        0,
        std::ptr::null_mut(),
        0,
        std::ptr::null_mut(),
        0,
    );
    if query.status == 0 || query.user_sid_len == 0 || query.executable_path_len == 0 {
        return false;
    }
    let mut server_user_sid = vec![0_u16; query.user_sid_len];
    let mut server_executable_path = vec![0_u16; query.executable_path_len];
    let mut server_final_path = vec![0_u16; query.executable_final_path_len];
    let filled = process_identity_with_executable_file(
        server_process_id,
        server_user_sid.as_mut_ptr(),
        server_user_sid.len(),
        server_executable_path.as_mut_ptr(),
        server_executable_path.len(),
        if server_final_path.is_empty() {
            std::ptr::null_mut()
        } else {
            server_final_path.as_mut_ptr()
        },
        server_final_path.len(),
    );
    if filled.status == 0
        || filled.user_sid_len != server_user_sid.len()
        || filled.executable_path_len != server_executable_path.len()
    {
        return false;
    }
    let Ok(server_user_sid) = String::from_utf16(&server_user_sid) else {
        return false;
    };
    if !same_principal_and_session(
        filled.session_id,
        filled.service_account != 0,
        &server_user_sid,
        current_session_id,
        current_user_sid,
    ) {
        return false;
    }
    if policy_mode == POLICY_DEVELOPMENT_SAME_USER_SESSION {
        return peer_development_policy_allowed(development_exception_enabled);
    }
    if policy_mode != POLICY_EXACT_EXECUTABLE
        || expected_executable_path.is_empty()
        || filled.executable_file_status == 0
        || filled.executable_final_path_len == 0
        || filled.executable_final_path_len > server_final_path.len()
    {
        return false;
    }
    server_final_path.truncate(filled.executable_final_path_len);
    let Ok(server_final_path) = String::from_utf16(&server_final_path) else {
        return false;
    };
    let Some((expected, expected_final_path)) =
        executable_file_identity_owned(expected_executable_path)
    else {
        return false;
    };
    let Ok(expected_final_path) = String::from_utf16(&expected_final_path) else {
        return false;
    };
    executable_files_match(
        filled.executable_file_volume_serial_number,
        filled.executable_file_index_high,
        filled.executable_file_index_low,
        filled.executable_file_number_of_links,
        filled.executable_file_contains_reparse_point != 0,
        &server_final_path,
        expected.volume_serial_number,
        expected.file_index_high,
        expected.file_index_low,
        expected.number_of_links,
        expected.contains_reparse_point != 0,
        &expected_final_path,
    )
}

#[allow(clippy::too_many_arguments)]
fn verified_pipe_client_peer(
    pipe: *mut c_void,
    current_service_account: bool,
    current_session_id: u32,
    current_secure_desktop: bool,
    current_user_sid: &str,
    user_sid_output: *mut u16,
    user_sid_capacity: usize,
    executable_path_output: *mut u16,
    executable_path_capacity: usize,
    executable_final_path_output: *mut u16,
    executable_final_path_capacity: usize,
) -> Fcitx5WindowsCommonVerifiedPipeClient {
    if !may_launch_user_engine(
        current_service_account,
        current_session_id,
        current_secure_desktop,
        current_user_sid,
    ) {
        return Fcitx5WindowsCommonVerifiedPipeClient::default();
    }
    let Some(client_process_id) = pipe_client_process_id(pipe) else {
        return Fcitx5WindowsCommonVerifiedPipeClient::default();
    };
    let query = process_identity_with_executable_file(
        client_process_id,
        std::ptr::null_mut(),
        0,
        std::ptr::null_mut(),
        0,
        std::ptr::null_mut(),
        0,
    );
    if query.status == 0 || query.user_sid_len == 0 || query.executable_path_len == 0 {
        return Fcitx5WindowsCommonVerifiedPipeClient::default();
    }
    let mut client_user_sid = vec![0_u16; query.user_sid_len];
    let mut client_executable_path = vec![0_u16; query.executable_path_len];
    let mut client_final_path = vec![0_u16; query.executable_final_path_len];
    let filled = process_identity_with_executable_file(
        client_process_id,
        client_user_sid.as_mut_ptr(),
        client_user_sid.len(),
        client_executable_path.as_mut_ptr(),
        client_executable_path.len(),
        if client_final_path.is_empty() {
            std::ptr::null_mut()
        } else {
            client_final_path.as_mut_ptr()
        },
        client_final_path.len(),
    );
    if filled.status == 0
        || filled.user_sid_len != client_user_sid.len()
        || filled.executable_path_len != client_executable_path.len()
    {
        return Fcitx5WindowsCommonVerifiedPipeClient::default();
    }
    let Ok(client_user_sid_string) = String::from_utf16(&client_user_sid) else {
        return Fcitx5WindowsCommonVerifiedPipeClient::default();
    };
    if !same_principal_and_session(
        filled.session_id,
        filled.service_account != 0,
        &client_user_sid_string,
        current_session_id,
        current_user_sid,
    ) {
        return Fcitx5WindowsCommonVerifiedPipeClient::default();
    }
    if filled.executable_file_status != 0 {
        if filled.executable_final_path_len == 0
            || filled.executable_final_path_len > client_final_path.len()
        {
            return Fcitx5WindowsCommonVerifiedPipeClient::default();
        }
        client_final_path.truncate(filled.executable_final_path_len);
    } else {
        client_final_path.clear();
    }
    write_wide_units(&client_user_sid, user_sid_output, user_sid_capacity);
    write_wide_units(
        &client_executable_path,
        executable_path_output,
        executable_path_capacity,
    );
    write_wide_units(
        &client_final_path,
        executable_final_path_output,
        executable_final_path_capacity,
    );
    Fcitx5WindowsCommonVerifiedPipeClient {
        status: 1,
        service_account: filled.service_account,
        executable_file_status: filled.executable_file_status,
        executable_file_contains_reparse_point: filled.executable_file_contains_reparse_point,
        process_id: client_process_id,
        session_id: filled.session_id,
        executable_file_volume_serial_number: filled.executable_file_volume_serial_number,
        executable_file_index_high: filled.executable_file_index_high,
        executable_file_index_low: filled.executable_file_index_low,
        executable_file_number_of_links: filled.executable_file_number_of_links,
        user_sid_len: client_user_sid.len(),
        executable_path_len: client_executable_path.len(),
        executable_final_path_len: client_final_path.len(),
    }
}

fn local_test_namespace() -> Option<String> {
    env::var("FCITX5_TEST_NAMESPACE")
        .ok()
        .filter(|value| valid_channel(value))
}

fn current_runtime_generation() -> String {
    let mut path = vec![0_u16; ENDPOINT_MAX_WIDE_UNITS];
    // SAFETY: Passing a null module handle asks Windows for the current process
    // image path. `path` is writable storage for `path.len()` UTF-16 units.
    let size =
        unsafe { GetModuleFileNameW(std::ptr::null_mut(), path.as_mut_ptr(), path.len() as u32) }
            as usize;
    if size > 0 && size < path.len() {
        path.truncate(size);
        let module_path = PathBuf::from(String::from_utf16_lossy(&path));
        if let Some(generation) = current_runtime_generation_for_module(&module_path) {
            return generation;
        }
    }
    "current".to_owned()
}

fn process_image_path(process_id: u32) -> Option<String> {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    if process_id == 0 {
        return None;
    }
    // SAFETY: Opening by process id with query-only rights. Any successful
    // handle is closed before returning.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if handle.is_null() || handle == invalid_handle_value() {
        return None;
    }
    let mut path = vec![0_u16; ENDPOINT_MAX_WIDE_UNITS];
    let mut length = path.len() as u32;
    // SAFETY: `handle` is valid and `path` is writable storage for `length`
    // UTF-16 code units. Windows updates `length` with the written count.
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, path.as_mut_ptr(), &mut length) };
    // SAFETY: `handle` was returned by OpenProcess above.
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 || length == 0 || length as usize > path.len() {
        return None;
    }
    path.truncate(length as usize);
    String::from_utf16(&path)
        .ok()
        .filter(|value| !value.is_empty())
}

fn process_session_id(process_id: u32) -> Fcitx5WindowsCommonProcessSession {
    if process_id == 0 {
        return Fcitx5WindowsCommonProcessSession::default();
    }
    let mut session_id = 0_u32;
    // SAFETY: `session_id` is a valid out pointer for the duration of the call.
    let ok = unsafe { ProcessIdToSessionId(process_id, &mut session_id) };
    if ok == 0 {
        return Fcitx5WindowsCommonProcessSession::default();
    }
    Fcitx5WindowsCommonProcessSession {
        status: 1,
        session_id,
    }
}

fn process_user_sid(process_id: u32) -> Option<(Vec<u16>, bool)> {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_USER_CLASS: u32 = 1;
    const TOKEN_INFORMATION_MAX_BYTES: u32 = 64 * 1024;
    const WIN_LOCAL_SYSTEM_SID: i32 = 22;
    const WIN_LOCAL_SERVICE_SID: i32 = 23;
    const WIN_NETWORK_SERVICE_SID: i32 = 24;
    const SID_STRING_MAX_WIDE_UNITS: usize = 1024;

    if process_id == 0 {
        return None;
    }
    // SAFETY: Opening by process id with query-only rights. Any successful
    // handle is closed before returning.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() || process == invalid_handle_value() {
        return None;
    }
    let mut token = std::ptr::null_mut();
    // SAFETY: `process` is valid and `token` is a valid out pointer.
    let token_ok = unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) };
    // SAFETY: `process` was returned by OpenProcess above.
    unsafe {
        CloseHandle(process);
    }
    if token_ok == 0 || token.is_null() {
        return None;
    }
    let mut required = 0_u32;
    // SAFETY: The initial zero-length query asks Windows for the required
    // buffer size. `required` is writable.
    unsafe {
        GetTokenInformation(
            token,
            TOKEN_USER_CLASS,
            std::ptr::null_mut(),
            0,
            &mut required,
        );
    }
    if required == 0 || required > TOKEN_INFORMATION_MAX_BYTES {
        // SAFETY: `token` was returned by OpenProcessToken above.
        unsafe {
            CloseHandle(token);
        }
        return None;
    }
    let mut buffer = vec![0_u8; required as usize];
    // SAFETY: `buffer` provides `required` writable bytes for the TOKEN_USER
    // payload and `required` remains a valid in/out length pointer.
    let info_ok = unsafe {
        GetTokenInformation(
            token,
            TOKEN_USER_CLASS,
            buffer.as_mut_ptr().cast::<c_void>(),
            required,
            &mut required,
        )
    };
    // SAFETY: `token` was returned by OpenProcessToken above.
    unsafe {
        CloseHandle(token);
    }
    if info_ok == 0 {
        return None;
    }
    let token_user = buffer.as_ptr().cast::<TokenUser>();
    // SAFETY: A successful TokenUser query returns a TOKEN_USER-compatible
    // buffer whose first field is SID_AND_ATTRIBUTES.
    let sid = unsafe { (*token_user).user.sid };
    if sid.is_null() {
        return None;
    }
    let mut raw_sid = std::ptr::null_mut();
    // SAFETY: `sid` comes from a successful TOKEN_USER query and `raw_sid` is
    // an out pointer for a LocalAlloc-owned UTF-16 SID string.
    if unsafe { ConvertSidToStringSidW(sid, &mut raw_sid) } == 0 || raw_sid.is_null() {
        return None;
    }
    let mut len = 0_usize;
    while len < SID_STRING_MAX_WIDE_UNITS {
        // SAFETY: `raw_sid` points to a NUL-terminated Windows-allocated UTF-16
        // string. The loop is bounded to avoid untrusted unbounded scanning.
        if unsafe { *raw_sid.add(len) } == 0 {
            break;
        }
        len += 1;
    }
    if len == SID_STRING_MAX_WIDE_UNITS {
        // SAFETY: `raw_sid` was allocated by ConvertSidToStringSidW.
        unsafe {
            LocalFree(raw_sid.cast::<c_void>());
        }
        return None;
    }
    // SAFETY: `raw_sid` is readable for `len` initialized UTF-16 code units.
    let sid_units = unsafe { std::slice::from_raw_parts(raw_sid, len) }.to_vec();
    // SAFETY: `raw_sid` was allocated by ConvertSidToStringSidW.
    unsafe {
        LocalFree(raw_sid.cast::<c_void>());
    }
    let service_account = {
        // SAFETY: `sid` is valid for the lifetime of `buffer`, which is still
        // alive through this block.
        unsafe {
            IsWellKnownSid(sid, WIN_LOCAL_SYSTEM_SID) != 0
                || IsWellKnownSid(sid, WIN_LOCAL_SERVICE_SID) != 0
                || IsWellKnownSid(sid, WIN_NETWORK_SERVICE_SID) != 0
        }
    };
    Some((sid_units, service_account))
}

fn process_identity(
    process_id: u32,
    user_sid_output: *mut u16,
    user_sid_capacity: usize,
    executable_path_output: *mut u16,
    executable_path_capacity: usize,
) -> Fcitx5WindowsCommonProcessIdentity {
    let session = process_session_id(process_id);
    if session.status == 0 {
        return Fcitx5WindowsCommonProcessIdentity::default();
    }
    let Some((user_sid, service_account)) = process_user_sid(process_id) else {
        return Fcitx5WindowsCommonProcessIdentity::default();
    };
    let Some(executable_path) = process_image_path(process_id) else {
        return Fcitx5WindowsCommonProcessIdentity::default();
    };
    let executable_path: Vec<u16> = executable_path.encode_utf16().collect();
    write_wide_units(&user_sid, user_sid_output, user_sid_capacity);
    write_wide_units(
        &executable_path,
        executable_path_output,
        executable_path_capacity,
    );
    Fcitx5WindowsCommonProcessIdentity {
        status: 1,
        service_account: service_account as u8,
        session_id: session.session_id,
        user_sid_len: user_sid.len(),
        executable_path_len: executable_path.len(),
    }
}

fn current_identity(
    user_sid_output: *mut u16,
    user_sid_capacity: usize,
    executable_path_output: *mut u16,
    executable_path_capacity: usize,
) -> Fcitx5WindowsCommonCurrentIdentity {
    // SAFETY: Retrieves the current process id and has no preconditions.
    let process_id = unsafe { GetCurrentProcessId() };
    let process = process_identity(
        process_id,
        user_sid_output,
        user_sid_capacity,
        executable_path_output,
        executable_path_capacity,
    );
    if process.status == 0 {
        return Fcitx5WindowsCommonCurrentIdentity::default();
    }
    Fcitx5WindowsCommonCurrentIdentity {
        status: 1,
        service_account: process.service_account,
        secure_desktop: secure_input_desktop() as u8,
        process_id,
        session_id: process.session_id,
        user_sid_len: process.user_sid_len,
        executable_path_len: process.executable_path_len,
    }
}

fn process_identity_with_executable_file(
    process_id: u32,
    user_sid_output: *mut u16,
    user_sid_capacity: usize,
    executable_path_output: *mut u16,
    executable_path_capacity: usize,
    executable_final_path_output: *mut u16,
    executable_final_path_capacity: usize,
) -> Fcitx5WindowsCommonProcessExecutableIdentity {
    let session = process_session_id(process_id);
    if session.status == 0 {
        return Fcitx5WindowsCommonProcessExecutableIdentity::default();
    }
    let Some((user_sid, service_account)) = process_user_sid(process_id) else {
        return Fcitx5WindowsCommonProcessExecutableIdentity::default();
    };
    let Some(executable_path) = process_image_path(process_id) else {
        return Fcitx5WindowsCommonProcessExecutableIdentity::default();
    };
    let executable_path: Vec<u16> = executable_path.encode_utf16().collect();
    write_wide_units(&user_sid, user_sid_output, user_sid_capacity);
    write_wide_units(
        &executable_path,
        executable_path_output,
        executable_path_capacity,
    );

    if let Some((identity, final_path)) = executable_file_identity_owned(&executable_path) {
        write_wide_units(
            &final_path,
            executable_final_path_output,
            executable_final_path_capacity,
        );
        return Fcitx5WindowsCommonProcessExecutableIdentity {
            status: 1,
            service_account: service_account as u8,
            executable_file_status: 1,
            executable_file_contains_reparse_point: identity.contains_reparse_point,
            session_id: session.session_id,
            executable_file_volume_serial_number: identity.volume_serial_number,
            executable_file_index_high: identity.file_index_high,
            executable_file_index_low: identity.file_index_low,
            executable_file_number_of_links: identity.number_of_links,
            user_sid_len: user_sid.len(),
            executable_path_len: executable_path.len(),
            executable_final_path_len: final_path.len(),
        };
    }

    Fcitx5WindowsCommonProcessExecutableIdentity {
        status: 1,
        service_account: service_account as u8,
        session_id: session.session_id,
        user_sid_len: user_sid.len(),
        executable_path_len: executable_path.len(),
        ..Default::default()
    }
}

fn current_identity_with_executable_file(
    user_sid_output: *mut u16,
    user_sid_capacity: usize,
    executable_path_output: *mut u16,
    executable_path_capacity: usize,
    executable_final_path_output: *mut u16,
    executable_final_path_capacity: usize,
) -> Fcitx5WindowsCommonCurrentExecutableIdentity {
    // SAFETY: Retrieves the current process id and has no preconditions.
    let process_id = unsafe { GetCurrentProcessId() };
    let process = process_identity_with_executable_file(
        process_id,
        user_sid_output,
        user_sid_capacity,
        executable_path_output,
        executable_path_capacity,
        executable_final_path_output,
        executable_final_path_capacity,
    );
    if process.status == 0 {
        return Fcitx5WindowsCommonCurrentExecutableIdentity::default();
    }
    Fcitx5WindowsCommonCurrentExecutableIdentity {
        status: 1,
        service_account: process.service_account,
        secure_desktop: secure_input_desktop() as u8,
        executable_file_status: process.executable_file_status,
        executable_file_contains_reparse_point: process.executable_file_contains_reparse_point,
        process_id,
        session_id: process.session_id,
        executable_file_volume_serial_number: process.executable_file_volume_serial_number,
        executable_file_index_high: process.executable_file_index_high,
        executable_file_index_low: process.executable_file_index_low,
        executable_file_number_of_links: process.executable_file_number_of_links,
        user_sid_len: process.user_sid_len,
        executable_path_len: process.executable_path_len,
        executable_final_path_len: process.executable_final_path_len,
    }
}

/// A named-pipe client whose server was verified as the exact expected
/// executable running under the current principal and session.
///
/// The pipe handle is owned privately and is closed when this value is
/// dropped.
#[must_use = "dropping the verified client closes its pipe handle"]
pub struct VerifiedPipeClient {
    pipe: *mut c_void,
}

impl VerifiedPipeClient {
    /// Connects to `pipe_name` and accepts it only when its server is exactly
    /// `expected_server` and passes the current peer-identity policy.
    ///
    /// Returns `None` if opening or verifying the pipe cannot complete within
    /// `timeout`.
    pub fn connect_exact(
        pipe_name: &OsStr,
        expected_server: &Path,
        timeout: Duration,
    ) -> Option<Self> {
        let timeout_milliseconds =
            u32::try_from(timeout.as_millis().min(u128::from(MAX_DWORD_MINUS_ONE))).ok()?;
        if timeout_milliseconds == 0 {
            return None;
        }
        let deadline = deadline_after_milliseconds(timeout_milliseconds);
        let pipe_name = pipe_name.encode_wide().collect::<Vec<_>>();
        let expected_server = expected_server
            .as_os_str()
            .encode_wide()
            .collect::<Vec<_>>();
        if pipe_name.is_empty() || expected_server.is_empty() {
            return None;
        }

        let identity = current_identity_with_executable_file(
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
        );
        if identity.status == 0 || identity.user_sid_len == 0 {
            return None;
        }
        let mut user_sid = vec![0_u16; identity.user_sid_len];
        let identity = current_identity_with_executable_file(
            user_sid.as_mut_ptr(),
            user_sid.len(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
        );
        if identity.status == 0 || identity.user_sid_len != user_sid.len() {
            return None;
        }
        let Ok(user_sid) = String::from_utf16(&user_sid) else {
            return None;
        };

        let pipe = open_pipe_client(&pipe_name, deadline, true);
        if pipe.is_null() || pipe == invalid_handle_value() {
            return None;
        }
        if !verify_pipe_server_peer(
            pipe,
            identity.service_account != 0,
            identity.session_id,
            identity.secure_desktop != 0,
            &user_sid,
            0,
            &expected_server,
            false,
        ) {
            close_pipe_client(pipe);
            return None;
        }
        Some(Self { pipe })
    }

    /// Writes the complete `frame` before `timeout` expires.
    ///
    /// Returns `false` if the pipe disconnects, makes no progress, or the
    /// complete frame cannot be written before the deadline.
    pub fn write_all(&mut self, frame: &[u8], timeout: Duration) -> bool {
        if frame.is_empty() {
            return true;
        }
        let Ok(timeout_milliseconds) =
            u32::try_from(timeout.as_millis().min(u128::from(MAX_DWORD_MINUS_ONE)))
        else {
            return false;
        };
        if timeout_milliseconds == 0 {
            return false;
        }
        pipe_transfer(
            self.pipe,
            true,
            frame.as_ptr().cast_mut(),
            frame.len(),
            deadline_after_milliseconds(timeout_milliseconds),
        )
    }

    /// Exchanges one complete request frame for one complete response frame
    /// before `timeout` expires.
    ///
    /// Returns the response length only when it fits in `response` and the
    /// underlying pipe transfer completes successfully.
    #[must_use]
    pub fn transact(
        &mut self,
        request: &[u8],
        response: &mut [u8],
        timeout: Duration,
    ) -> Option<usize> {
        let timeout_milliseconds =
            u32::try_from(timeout.as_millis().min(u128::from(MAX_DWORD_MINUS_ONE))).ok()?;
        if timeout_milliseconds == 0 || request.is_empty() || response.is_empty() {
            return None;
        }
        let result = pipe_transact(
            self.pipe,
            request,
            response.as_mut_ptr(),
            response.len(),
            deadline_after_milliseconds(timeout_milliseconds),
        );
        (result.status != 0 && result.response_len <= response.len()).then_some(result.response_len)
    }
}

/// Returns a non-zero request id for a launcher-originated IPC request.
#[must_use]
pub fn next_launcher_request_id() -> u64 {
    NEXT_LAUNCHER_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

impl Drop for VerifiedPipeClient {
    fn drop(&mut self) {
        close_pipe_client(self.pipe);
    }
}

fn utf8_to_wide(bytes: &[u8], output: *mut u16, capacity: usize) -> Fcitx5WindowsCommonUtf8ToWide {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Fcitx5WindowsCommonUtf8ToWide::default();
    };
    let wide: Vec<u16> = text.encode_utf16().collect();
    write_wide_units(&wide, output, capacity);
    Fcitx5WindowsCommonUtf8ToWide {
        status: 1,
        utf16_len: wide.len(),
    }
}

fn wide_to_utf8(wide: &[u16], output: *mut u8, capacity: usize) -> Fcitx5WindowsCommonWideToUtf8 {
    let Ok(text) = String::from_utf16(wide) else {
        return Fcitx5WindowsCommonWideToUtf8::default();
    };
    let bytes = text.as_bytes();
    write_bytes(bytes, output, capacity);
    Fcitx5WindowsCommonWideToUtf8 {
        status: 1,
        utf8_len: bytes.len(),
    }
}

fn utf8_offset_to_wide(bytes: &[u8], offset: u32) -> Fcitx5WindowsCommonUtf8OffsetToWide {
    let offset = offset as usize;
    if offset > bytes.len() {
        return Fcitx5WindowsCommonUtf8OffsetToWide::default();
    }
    let Ok(prefix) = std::str::from_utf8(&bytes[..offset]) else {
        return Fcitx5WindowsCommonUtf8OffsetToWide::default();
    };
    let Ok(utf16_offset) = u32::try_from(prefix.encode_utf16().count()) else {
        return Fcitx5WindowsCommonUtf8OffsetToWide::default();
    };
    Fcitx5WindowsCommonUtf8OffsetToWide {
        status: 1,
        utf16_offset,
    }
}

fn ipc_status_ok(status: u32) -> bool {
    status == 0
}

fn accept_hello_response(
    response_to: u64,
    session_id: u32,
    status: u32,
    expected_request_id: u64,
    expected_session_id: u32,
) -> bool {
    response_to == expected_request_id && session_id == expected_session_id && ipc_status_ok(status)
}

fn apply_hello_response_scalars(
    response_to: u64,
    engine_epoch: u64,
    session_id: u32,
    status: u32,
    expected_request_id: u64,
    expected_session_id: u32,
) -> Fcitx5WindowsCommonHelloResponseScalars {
    if !accept_hello_response(
        response_to,
        session_id,
        status,
        expected_request_id,
        expected_session_id,
    ) {
        return Fcitx5WindowsCommonHelloResponseScalars::default();
    }
    Fcitx5WindowsCommonHelloResponseScalars {
        status: 1,
        handshake_complete: 1,
        engine_epoch,
    }
}

#[allow(clippy::too_many_arguments)]
fn accept_key_response(
    response_to: u64,
    engine_epoch: u64,
    session_id: u32,
    context_id: u64,
    revision: u64,
    status: u32,
    expected_request_id: u64,
    expected_engine_epoch: u64,
    expected_session_id: u32,
    expected_context_id: u64,
    previous_revision: u64,
) -> bool {
    response_to == expected_request_id
        && engine_epoch == expected_engine_epoch
        && session_id == expected_session_id
        && context_id == expected_context_id
        && revision > previous_revision
        && ipc_status_ok(status)
}

#[allow(clippy::too_many_arguments)]
fn accept_candidate_select_response(
    response_to: u64,
    engine_epoch: u64,
    session_id: u32,
    context_id: u64,
    revision: u64,
    status: u32,
    expected_request_id: u64,
    expected_engine_epoch: u64,
    expected_session_id: u32,
    expected_context_id: u64,
    previous_revision: u64,
) -> bool {
    accept_key_response(
        response_to,
        engine_epoch,
        session_id,
        context_id,
        revision,
        status,
        expected_request_id,
        expected_engine_epoch,
        expected_session_id,
        expected_context_id,
        previous_revision,
    )
}

fn accept_candidate_select_request(
    current_engine_epoch: u64,
    expected_engine_epoch: u64,
    target_process_id: u32,
    context_id: u64,
    composition_id: u64,
    revision: u64,
    candidate_id: u64,
) -> bool {
    target_process_id != 0
        && expected_engine_epoch != 0
        && current_engine_epoch == expected_engine_epoch
        && context_id != 0
        && composition_id != 0
        && revision != 0
        && candidate_id != 0
}

fn accept_engine_status_response(
    response_to: u64,
    engine_epoch: u64,
    session_id: u32,
    status: u32,
    expected_request_id: u64,
    expected_engine_epoch: u64,
    expected_session_id: u32,
) -> bool {
    response_to == expected_request_id
        && engine_epoch == expected_engine_epoch
        && session_id == expected_session_id
        && ipc_status_ok(status)
}

fn accept_launcher_response(
    response_to: u64,
    session_id: u32,
    expected_request_id: u64,
    expected_session_id: u32,
) -> bool {
    response_to == expected_request_id && session_id == expected_session_id
}

fn next_pipe_client_request_id() -> u64 {
    NEXT_PIPE_CLIENT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

fn apply_key_response_scalars(
    input: Fcitx5WindowsCommonKeyResponseScalarInput,
) -> Fcitx5WindowsCommonKeyResponseScalars {
    if !accept_key_response(
        input.response_to,
        input.engine_epoch,
        input.session_id,
        input.context_id,
        input.revision,
        input.status,
        input.expected_request_id,
        input.expected_engine_epoch,
        input.expected_session_id,
        input.expected_context_id,
        input.previous_revision,
    ) {
        return Fcitx5WindowsCommonKeyResponseScalars::default();
    }
    Fcitx5WindowsCommonKeyResponseScalars {
        status: 1,
        handled: input.handled,
        delete_surrounding_text: input.delete_surrounding_text,
        forward_key: input.forward_key,
        forward_key_release: input.forward_key_release,
        caret_valid: input.caret_valid,
        engine_epoch: input.engine_epoch,
        context_composition_id: input.composition_id,
        context_revision: input.revision,
        result_composition_id: input.composition_id,
        result_revision: input.revision,
        selected_candidate: input.selected_candidate,
        candidate_page: input.candidate_page,
        candidate_total: input.candidate_total,
        delete_surrounding_offset: input.delete_surrounding_offset,
        delete_surrounding_size: input.delete_surrounding_size,
        forward_key_sym: input.forward_key_sym,
        forward_key_states: input.forward_key_states,
        forward_key_code: input.forward_key_code,
        caret_left: input.caret_left,
        caret_top: input.caret_top,
        caret_right: input.caret_right,
        caret_bottom: input.caret_bottom,
        caret_dpi: input.caret_dpi,
        candidate_visibility: input.candidate_visibility,
    }
}

fn apply_engine_status_response_scalars(
    input: Fcitx5WindowsCommonEngineStatusResponseScalarInput,
) -> Fcitx5WindowsCommonEngineStatusResponseScalars {
    if !accept_engine_status_response(
        input.response_to,
        input.engine_epoch,
        input.session_id,
        input.status,
        input.expected_request_id,
        input.expected_engine_epoch,
        input.expected_session_id,
    ) {
        return Fcitx5WindowsCommonEngineStatusResponseScalars::default();
    }
    Fcitx5WindowsCommonEngineStatusResponseScalars {
        status: 1,
        response_status: input.status,
        request_id: input.request_id,
        response_to: input.response_to,
        engine_epoch: input.engine_epoch,
        session_id: input.session_id,
        context_id: input.context_id,
        composition_id: input.composition_id,
        revision: input.revision,
    }
}

fn apply_launcher_response_scalars(
    input: Fcitx5WindowsCommonLauncherResponseScalarInput,
) -> Fcitx5WindowsCommonLauncherResponseScalars {
    if !accept_launcher_response(
        input.response_to,
        input.session_id,
        input.expected_request_id,
        input.expected_session_id,
    ) {
        return Fcitx5WindowsCommonLauncherResponseScalars::default();
    }
    Fcitx5WindowsCommonLauncherResponseScalars {
        status: 1,
        response_status: input.status,
        launcher_state: input.launcher_state,
        engine_state: input.engine_state,
        start_disposition: input.start_disposition,
        safe_mode: input.safe_mode,
        request_id: input.request_id,
        response_to: input.response_to,
        engine_epoch: input.engine_epoch,
        session_id: input.session_id,
        context_id: input.context_id,
        composition_id: input.composition_id,
        revision: input.revision,
        retry_after_milliseconds: input.retry_after_milliseconds,
    }
}

fn secure_input_desktop() -> bool {
    const DESKTOP_READOBJECTS: u32 = 0x0001;
    const UOI_NAME: i32 = 2;

    // SAFETY: Opens the current input desktop for read-only object metadata.
    // Failure is treated as secure/fail-closed, matching the C++ contract.
    let desktop = unsafe { OpenInputDesktop(0, 0, DESKTOP_READOBJECTS) };
    if desktop.is_null() {
        return true;
    }
    let mut name = [0_u16; 256];
    let mut required = 0_u32;
    // SAFETY: `desktop` is valid and `name` is writable storage. The handle is
    // closed immediately after the metadata query.
    let read = unsafe {
        GetUserObjectInformationW(
            desktop,
            UOI_NAME,
            name.as_mut_ptr().cast::<c_void>(),
            (name.len() * std::mem::size_of::<u16>()) as u32,
            &mut required,
        )
    };
    // SAFETY: `desktop` was returned by OpenInputDesktop above.
    unsafe {
        CloseDesktop(desktop);
    }
    if read == 0 {
        return true;
    }
    let end = name
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(name.len());
    let lowered = String::from_utf16_lossy(&name[..end]).to_lowercase();
    lowered == "winlogon" || lowered == "disconnect"
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

fn paths_refer_to_same_file(left: &[u16], right: &[u16]) -> bool {
    let left = basic_file_identity(left);
    if left.status == 0 {
        return false;
    }
    let right = basic_file_identity(right);
    right.status != 0
        && basic_file_identities_match(
            left.volume_serial_number,
            left.file_index_high,
            left.file_index_low,
            right.volume_serial_number,
            right.file_index_high,
            right.file_index_low,
        )
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonBasicFileIdentity {
    pub status: u8,
    pub volume_serial_number: u32,
    pub file_index_high: u32,
    pub file_index_low: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonExecutableFileIdentity {
    pub status: u8,
    pub contains_reparse_point: u8,
    pub volume_serial_number: u32,
    pub file_index_high: u32,
    pub file_index_low: u32,
    pub number_of_links: u32,
    pub final_path_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonProcessSession {
    pub status: u8,
    pub session_id: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonProcessIdentity {
    pub status: u8,
    pub service_account: u8,
    pub session_id: u32,
    pub user_sid_len: usize,
    pub executable_path_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonCurrentIdentity {
    pub status: u8,
    pub service_account: u8,
    pub secure_desktop: u8,
    pub process_id: u32,
    pub session_id: u32,
    pub user_sid_len: usize,
    pub executable_path_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonProcessExecutableIdentity {
    pub status: u8,
    pub service_account: u8,
    pub executable_file_status: u8,
    pub executable_file_contains_reparse_point: u8,
    pub session_id: u32,
    pub executable_file_volume_serial_number: u32,
    pub executable_file_index_high: u32,
    pub executable_file_index_low: u32,
    pub executable_file_number_of_links: u32,
    pub user_sid_len: usize,
    pub executable_path_len: usize,
    pub executable_final_path_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonVerifiedPipeClient {
    pub status: u8,
    pub service_account: u8,
    pub executable_file_status: u8,
    pub executable_file_contains_reparse_point: u8,
    pub process_id: u32,
    pub session_id: u32,
    pub executable_file_volume_serial_number: u32,
    pub executable_file_index_high: u32,
    pub executable_file_index_low: u32,
    pub executable_file_number_of_links: u32,
    pub user_sid_len: usize,
    pub executable_path_len: usize,
    pub executable_final_path_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonUtf8ToWide {
    pub status: u8,
    pub utf16_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonWideToUtf8 {
    pub status: u8,
    pub utf8_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonUtf8OffsetToWide {
    pub status: u8,
    pub utf16_offset: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonPipeTransact {
    pub status: u8,
    pub response_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonPipeTransactWithError {
    pub status: u8,
    pub failure_error: u32,
    pub response_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonHelloResponseScalars {
    pub status: u8,
    pub handshake_complete: u8,
    pub engine_epoch: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonKeyResponseScalarInput {
    pub response_to: u64,
    pub engine_epoch: u64,
    pub session_id: u32,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub status: u32,
    pub expected_request_id: u64,
    pub expected_engine_epoch: u64,
    pub expected_session_id: u32,
    pub expected_context_id: u64,
    pub previous_revision: u64,
    pub handled: u8,
    pub selected_candidate: u32,
    pub candidate_page: u32,
    pub candidate_total: u32,
    pub candidate_visibility: u8,
    pub delete_surrounding_text: u8,
    pub delete_surrounding_offset: i32,
    pub delete_surrounding_size: u32,
    pub forward_key: u8,
    pub forward_key_sym: u32,
    pub forward_key_states: u32,
    pub forward_key_code: i32,
    pub forward_key_release: u8,
    pub caret_valid: u8,
    pub caret_left: i32,
    pub caret_top: i32,
    pub caret_right: i32,
    pub caret_bottom: i32,
    pub caret_dpi: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonKeyResponseScalars {
    pub status: u8,
    pub handled: u8,
    pub delete_surrounding_text: u8,
    pub forward_key: u8,
    pub forward_key_release: u8,
    pub caret_valid: u8,
    pub engine_epoch: u64,
    pub context_composition_id: u64,
    pub context_revision: u64,
    pub result_composition_id: u64,
    pub result_revision: u64,
    pub selected_candidate: u32,
    pub candidate_page: u32,
    pub candidate_total: u32,
    pub delete_surrounding_offset: i32,
    pub delete_surrounding_size: u32,
    pub forward_key_sym: u32,
    pub forward_key_states: u32,
    pub forward_key_code: i32,
    pub caret_left: i32,
    pub caret_top: i32,
    pub caret_right: i32,
    pub caret_bottom: i32,
    pub caret_dpi: u32,
    pub candidate_visibility: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonEngineStatusResponseScalarInput {
    pub request_id: u64,
    pub response_to: u64,
    pub engine_epoch: u64,
    pub session_id: u32,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub status: u32,
    pub expected_request_id: u64,
    pub expected_engine_epoch: u64,
    pub expected_session_id: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonEngineStatusResponseScalars {
    pub status: u8,
    pub response_status: u32,
    pub request_id: u64,
    pub response_to: u64,
    pub engine_epoch: u64,
    pub session_id: u32,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonLauncherResponseScalarInput {
    pub request_id: u64,
    pub response_to: u64,
    pub engine_epoch: u64,
    pub session_id: u32,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub status: u32,
    pub launcher_state: u32,
    pub engine_state: u32,
    pub start_disposition: u32,
    pub safe_mode: u8,
    pub retry_after_milliseconds: u64,
    pub expected_request_id: u64,
    pub expected_session_id: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonLauncherResponseScalars {
    pub status: u8,
    pub response_status: u32,
    pub launcher_state: u32,
    pub engine_state: u32,
    pub start_disposition: u32,
    pub safe_mode: u8,
    pub request_id: u64,
    pub response_to: u64,
    pub engine_epoch: u64,
    pub session_id: u32,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub retry_after_milliseconds: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5WindowsCommonCurrentExecutableIdentity {
    pub status: u8,
    pub service_account: u8,
    pub secure_desktop: u8,
    pub executable_file_status: u8,
    pub executable_file_contains_reparse_point: u8,
    pub process_id: u32,
    pub session_id: u32,
    pub executable_file_volume_serial_number: u32,
    pub executable_file_index_high: u32,
    pub executable_file_index_low: u32,
    pub executable_file_number_of_links: u32,
    pub user_sid_len: usize,
    pub executable_path_len: usize,
    pub executable_final_path_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SidAndAttributes {
    sid: *mut c_void,
    attributes: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct TokenUser {
    user: SidAndAttributes,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FileTime {
    low_date_time: u32,
    high_date_time: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ByHandleFileInformation {
    file_attributes: u32,
    creation_time: FileTime,
    last_access_time: FileTime,
    last_write_time: FileTime,
    volume_serial_number: u32,
    file_size_high: u32,
    file_size_low: u32,
    number_of_links: u32,
    file_index_high: u32,
    file_index_low: u32,
}

fn invalid_handle_value() -> *mut c_void {
    (-1_isize) as *mut c_void
}

fn basic_file_identity(path: &[u16]) -> Fcitx5WindowsCommonBasicFileIdentity {
    const FILE_READ_ATTRIBUTES: u32 = 0x80;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_WRITE: u32 = 0x2;
    const FILE_SHARE_DELETE: u32 = 0x4;
    const OPEN_EXISTING: u32 = 3;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    if path.is_empty() || path.len() >= 32_768 {
        return Fcitx5WindowsCommonBasicFileIdentity::default();
    }
    let mut nul_terminated = path.to_vec();
    nul_terminated.push(0);
    // SAFETY: `nul_terminated` is a NUL-terminated UTF-16 path. We request
    // metadata-only access and close the returned handle on every success path.
    let handle = unsafe {
        CreateFileW(
            nul_terminated.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle.is_null() || handle == invalid_handle_value() {
        return Fcitx5WindowsCommonBasicFileIdentity::default();
    }
    let mut information = ByHandleFileInformation {
        file_attributes: 0,
        creation_time: FileTime {
            low_date_time: 0,
            high_date_time: 0,
        },
        last_access_time: FileTime {
            low_date_time: 0,
            high_date_time: 0,
        },
        last_write_time: FileTime {
            low_date_time: 0,
            high_date_time: 0,
        },
        volume_serial_number: 0,
        file_size_high: 0,
        file_size_low: 0,
        number_of_links: 0,
        file_index_high: 0,
        file_index_low: 0,
    };
    // SAFETY: `handle` is valid and `information` points to initialized writable
    // storage matching BY_HANDLE_FILE_INFORMATION layout.
    let ok = unsafe { GetFileInformationByHandle(handle, &mut information) };
    // SAFETY: `handle` was returned by CreateFileW above.
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 {
        return Fcitx5WindowsCommonBasicFileIdentity::default();
    }
    Fcitx5WindowsCommonBasicFileIdentity {
        status: 1,
        volume_serial_number: information.volume_serial_number,
        file_index_high: information.file_index_high,
        file_index_low: information.file_index_low,
    }
}

fn executable_file_identity(
    path: &[u16],
    final_path_output: *mut u16,
    final_path_capacity: usize,
) -> Fcitx5WindowsCommonExecutableFileIdentity {
    const FILE_READ_ATTRIBUTES: u32 = 0x80;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_WRITE: u32 = 0x2;
    const FILE_SHARE_DELETE: u32 = 0x4;
    const OPEN_EXISTING: u32 = 3;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
    const NORMALIZED_DOS_PATH: u32 = 0;

    if path.is_empty()
        || path.len() >= 32_768
        || final_path_output.is_null()
        || final_path_capacity == 0
    {
        return Fcitx5WindowsCommonExecutableFileIdentity::default();
    }
    let mut nul_terminated = path.to_vec();
    nul_terminated.push(0);
    // SAFETY: `nul_terminated` is a NUL-terminated UTF-16 path. We request
    // metadata-only access and close the returned handle before returning.
    let handle = unsafe {
        CreateFileW(
            nul_terminated.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if handle.is_null() || handle == invalid_handle_value() {
        return Fcitx5WindowsCommonExecutableFileIdentity::default();
    }
    let mut information = ByHandleFileInformation {
        file_attributes: 0,
        creation_time: FileTime {
            low_date_time: 0,
            high_date_time: 0,
        },
        last_access_time: FileTime {
            low_date_time: 0,
            high_date_time: 0,
        },
        last_write_time: FileTime {
            low_date_time: 0,
            high_date_time: 0,
        },
        volume_serial_number: 0,
        file_size_high: 0,
        file_size_low: 0,
        number_of_links: 0,
        file_index_high: 0,
        file_index_low: 0,
    };
    // SAFETY: `handle` is valid and `information` points to initialized writable
    // storage matching BY_HANDLE_FILE_INFORMATION layout.
    let info_ok = unsafe { GetFileInformationByHandle(handle, &mut information) };
    // SAFETY: `final_path_output` points to writable storage for
    // `final_path_capacity` UTF-16 code units supplied by the caller.
    let final_path_len = unsafe {
        GetFinalPathNameByHandleW(
            handle,
            final_path_output,
            final_path_capacity as u32,
            NORMALIZED_DOS_PATH,
        )
    };
    // SAFETY: `handle` was returned by CreateFileW above.
    unsafe {
        CloseHandle(handle);
    }
    if info_ok == 0 || final_path_len == 0 || final_path_len as usize >= final_path_capacity {
        return Fcitx5WindowsCommonExecutableFileIdentity::default();
    }
    let path_buf = PathBuf::from(String::from_utf16_lossy(path));
    Fcitx5WindowsCommonExecutableFileIdentity {
        status: 1,
        contains_reparse_point: path_is_reparse_point_or_untrusted(&path_buf) as u8,
        volume_serial_number: information.volume_serial_number,
        file_index_high: information.file_index_high,
        file_index_low: information.file_index_low,
        number_of_links: information.number_of_links,
        final_path_len: final_path_len as usize,
    }
}

fn executable_file_identity_owned(
    path: &[u16],
) -> Option<(Fcitx5WindowsCommonExecutableFileIdentity, Vec<u16>)> {
    let mut final_path = vec![0_u16; ENDPOINT_MAX_WIDE_UNITS];
    let identity = executable_file_identity(path, final_path.as_mut_ptr(), final_path.len());
    if identity.status == 0
        || identity.final_path_len == 0
        || identity.final_path_len > final_path.len()
    {
        return None;
    }
    final_path.truncate(identity.final_path_len);
    Some((identity, final_path))
}

fn executable_paths_match(left: &[u16], right: &[u16]) -> bool {
    let Some((left_identity, left_final_path)) = executable_file_identity_owned(left) else {
        return false;
    };
    let Ok(left_final_path) = String::from_utf16(&left_final_path) else {
        return false;
    };

    let Some((right_identity, right_final_path)) = executable_file_identity_owned(right) else {
        return false;
    };
    let Ok(right_final_path) = String::from_utf16(&right_final_path) else {
        return false;
    };

    executable_files_match(
        left_identity.volume_serial_number,
        left_identity.file_index_high,
        left_identity.file_index_low,
        left_identity.number_of_links,
        left_identity.contains_reparse_point != 0,
        &left_final_path,
        right_identity.volume_serial_number,
        right_identity.file_index_high,
        right_identity.file_index_low,
        right_identity.number_of_links,
        right_identity.contains_reparse_point != 0,
        &right_final_path,
    )
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
/// `output` may be null for size queries or writable UTF-16 storage for
/// `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_local_test_namespace_utf16(
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(namespace) = local_test_namespace() else {
        return 0;
    };
    write_wide_string(&namespace, output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `output` may be null for size queries or writable UTF-16 storage for
/// `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_current_generation_utf16(
    output: *mut u16,
    capacity: usize,
) -> usize {
    write_wide_string(&current_runtime_generation(), output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `output` may be null for size queries or writable UTF-16 storage for
/// `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_process_image_path_utf16(
    process_id: u32,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(path) = process_image_path(process_id) else {
        return 0;
    };
    write_wide_string(&path, output, capacity)
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_process_session_id(
    process_id: u32,
) -> Fcitx5WindowsCommonProcessSession {
    process_session_id(process_id)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `service_account` may be null or point to one writable byte. `output` may
/// be null for size queries or writable UTF-16 storage for `capacity` code
/// units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_process_user_sid_utf16(
    process_id: u32,
    service_account: *mut u8,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some((sid, is_service_account)) = process_user_sid(process_id) else {
        return 0;
    };
    if !service_account.is_null() {
        // SAFETY: The caller supplied a writable one-byte out pointer.
        unsafe {
            *service_account = is_service_account as u8;
        }
    }
    write_wide_units(&sid, output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// Output pointers may be null for size queries or point to writable UTF-16
/// storage for their paired capacities. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_process_identity_utf16(
    process_id: u32,
    user_sid_output: *mut u16,
    user_sid_capacity: usize,
    executable_path_output: *mut u16,
    executable_path_capacity: usize,
) -> Fcitx5WindowsCommonProcessIdentity {
    process_identity(
        process_id,
        user_sid_output,
        user_sid_capacity,
        executable_path_output,
        executable_path_capacity,
    )
}

#[unsafe(no_mangle)]
/// # Safety
///
/// Output pointers may be null for size queries or point to writable UTF-16
/// storage for their paired capacities. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_current_identity_utf16(
    user_sid_output: *mut u16,
    user_sid_capacity: usize,
    executable_path_output: *mut u16,
    executable_path_capacity: usize,
) -> Fcitx5WindowsCommonCurrentIdentity {
    current_identity(
        user_sid_output,
        user_sid_capacity,
        executable_path_output,
        executable_path_capacity,
    )
}

#[unsafe(no_mangle)]
/// # Safety
///
/// Output pointers may be null for size queries or point to writable UTF-16
/// storage for their paired capacities. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_process_identity_with_executable_file_utf16(
    process_id: u32,
    user_sid_output: *mut u16,
    user_sid_capacity: usize,
    executable_path_output: *mut u16,
    executable_path_capacity: usize,
    executable_final_path_output: *mut u16,
    executable_final_path_capacity: usize,
) -> Fcitx5WindowsCommonProcessExecutableIdentity {
    process_identity_with_executable_file(
        process_id,
        user_sid_output,
        user_sid_capacity,
        executable_path_output,
        executable_path_capacity,
        executable_final_path_output,
        executable_final_path_capacity,
    )
}

#[unsafe(no_mangle)]
/// # Safety
///
/// Output pointers may be null for size queries or point to writable UTF-16
/// storage for their paired capacities. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_current_identity_with_executable_file_utf16(
    user_sid_output: *mut u16,
    user_sid_capacity: usize,
    executable_path_output: *mut u16,
    executable_path_capacity: usize,
    executable_final_path_output: *mut u16,
    executable_final_path_capacity: usize,
) -> Fcitx5WindowsCommonCurrentExecutableIdentity {
    current_identity_with_executable_file(
        user_sid_output,
        user_sid_capacity,
        executable_path_output,
        executable_path_capacity,
        executable_final_path_output,
        executable_final_path_capacity,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_secure_input_desktop() -> u8 {
    secure_input_desktop() as u8
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
/// `module_path` and `data_directory` must point to exactly their paired
/// readable UTF-16 code unit lengths. `data_directory` must be a non-empty
/// relative product data directory. `output` may be null for size queries or
/// writable UTF-16 storage for `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_default_data_root_for_module_utf16(
    module_path: *const u16,
    module_path_len: usize,
    data_directory: *const u16,
    data_directory_len: usize,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(module_path) = path_from_raw(module_path, module_path_len) else {
        return 0;
    };
    let Some(data_directory) = release_data_directory_from_raw(data_directory, data_directory_len)
    else {
        return 0;
    };
    let Some(root) = default_data_root_for_module(&module_path, &data_directory) else {
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
    let Some(security) = CurrentUserSecurityAttributes::from_pipe_identity(
        service_account != 0,
        session_id,
        &user_sid,
    ) else {
        return std::ptr::null_mut();
    };
    security.into_descriptor()
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `user_sid` must be null only when `user_sid_len` is zero, or point to a valid
/// UTF-16 buffer with exactly `user_sid_len` code units. The returned pointer is
/// a Rust-owned opaque handle that must be released with
/// `fcitx5_windows_common_pipe_security_destroy`.
pub unsafe extern "C" fn fcitx5_windows_common_pipe_security_create_utf16(
    service_account: u8,
    session_id: u32,
    user_sid: *const u16,
    user_sid_len: usize,
) -> *mut c_void {
    let Some(user_sid) = wide_string_from_raw(user_sid, user_sid_len) else {
        return std::ptr::null_mut();
    };
    let Some(security) = CurrentUserSecurityAttributes::from_pipe_identity(
        service_account != 0,
        session_id,
        &user_sid,
    ) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(security)).cast::<c_void>()
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `state` must be a live opaque handle returned by
/// `fcitx5_windows_common_pipe_security_create_utf16`. The returned pointer is
/// borrowed from that handle and is valid only until it is destroyed.
pub unsafe extern "C" fn fcitx5_windows_common_pipe_security_attributes(
    state: *mut c_void,
) -> *mut c_void {
    if state.is_null() {
        return std::ptr::null_mut();
    }
    let state = state.cast::<CurrentUserSecurityAttributes>();
    // SAFETY: `state` is an opaque handle returned by
    // fcitx5_windows_common_pipe_security_create_utf16 and remains owned by
    // the caller until destroy.
    unsafe { std::ptr::addr_of_mut!((*state).state.attributes).cast::<c_void>() }
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `state` must be null or an opaque handle returned by
/// `fcitx5_windows_common_pipe_security_create_utf16` that has not yet been
/// destroyed.
pub unsafe extern "C" fn fcitx5_windows_common_pipe_security_destroy(state: *mut c_void) {
    if state.is_null() {
        return;
    }
    // SAFETY: `state` must be a handle returned by
    // fcitx5_windows_common_pipe_security_create_utf16 that has not yet been
    // destroyed. Dropping the Box releases the LocalAlloc descriptor.
    unsafe {
        drop(Box::from_raw(state.cast::<CurrentUserSecurityAttributes>()));
    }
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
/// `pipe` must be a live overlapped pipe handle. `data` must be null only when
/// `size` is zero, or point to a buffer covering exactly `size` bytes. For
/// write operations the buffer is read; for read operations the buffer is
/// written. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_pipe_transfer(
    pipe: *mut c_void,
    write: u8,
    data: *mut u8,
    size: usize,
    deadline: u64,
) -> u8 {
    pipe_transfer(pipe, write != 0, data, size, deadline) as u8
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `pipe` must be a live overlapped pipe handle. `data` must be null only when
/// `size` is zero, or point to a buffer covering exactly `size` bytes. For
/// write operations the buffer is read; for read operations the buffer is
/// written. `stop_handle` may be null, or must be a live waitable handle that
/// remains valid for this call. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_pipe_transfer_with_stop(
    pipe: *mut c_void,
    write: u8,
    data: *mut u8,
    size: usize,
    deadline: u64,
    stop_handle: *mut c_void,
) -> u8 {
    pipe_transfer_with_stop(pipe, write != 0, data, size, deadline, stop_handle) as u8
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `pipe` must be a live named-pipe server handle created for overlapped I/O.
/// `stop_handle` may be null, or must be a live waitable handle that remains
/// valid for this call. No handle is closed or retained.
pub unsafe extern "C" fn fcitx5_windows_common_pipe_connect_client(
    pipe: *mut c_void,
    deadline: u64,
    stop_handle: *mut c_void,
) -> u8 {
    pipe_connect_client(pipe, deadline, stop_handle) as u8
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `pipe` must be a live overlapped pipe handle. `request` must be null only
/// when `request_len` is zero, or point to a readable request buffer.
/// `response_output` must point to writable storage for `response_capacity`
/// bytes. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_pipe_transact(
    pipe: *mut c_void,
    request: *const u8,
    request_len: usize,
    response_output: *mut u8,
    response_capacity: usize,
    deadline: u64,
) -> Fcitx5WindowsCommonPipeTransact {
    let request = if request.is_null() {
        if request_len != 0 {
            return Fcitx5WindowsCommonPipeTransact::default();
        }
        &[]
    } else {
        // SAFETY: The caller supplies exactly `request_len` readable bytes.
        unsafe { std::slice::from_raw_parts(request, request_len) }
    };
    pipe_transact(pipe, request, response_output, response_capacity, deadline)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `pipe` must be a live named-pipe handle. `request` must point to
/// `request_len` readable bytes. `response_output` must point to writable
/// storage for `response_capacity` bytes. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_pipe_transact_with_error(
    pipe: *mut c_void,
    request: *const u8,
    request_len: usize,
    response_output: *mut u8,
    response_capacity: usize,
    deadline: u64,
) -> Fcitx5WindowsCommonPipeTransactWithError {
    if request.is_null() {
        return Fcitx5WindowsCommonPipeTransactWithError {
            failure_error: ERROR_INVALID_DATA,
            ..Default::default()
        };
    }
    // SAFETY: The caller provides exactly `request_len` readable bytes. The
    // slice is used only during this call.
    let request = unsafe { std::slice::from_raw_parts(request, request_len) };
    pipe_transact_with_error(pipe, request, response_output, response_capacity, deadline)
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_deadline_after_milliseconds(milliseconds: u32) -> u64 {
    deadline_after_milliseconds(milliseconds)
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_tick_milliseconds() -> u64 {
    tick_milliseconds()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_deadline_has_time(deadline: u64) -> u8 {
    deadline_has_time(deadline) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_current_process_id() -> u32 {
    // SAFETY: Retrieves the current process id and has no preconditions.
    unsafe { GetCurrentProcessId() }
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_system_uses_dark_appearance() -> u8 {
    system_uses_dark_appearance() as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_system_font_families_utf16(
    output: *mut u16,
    capacity: usize,
) -> usize {
    write_wide_units(&system_font_family_payload(), output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `pipe_name` must be null only when `pipe_name_len` is zero, or point to a
/// readable UTF-16 buffer with exactly the provided length. No pointer is
/// retained. The returned handle is owned by the caller and must be closed with
/// `CloseHandle`.
pub unsafe extern "C" fn fcitx5_windows_common_open_pipe_client_utf16(
    pipe_name: *const u16,
    pipe_name_len: usize,
    deadline: u64,
    wait_when_busy: u8,
) -> *mut c_void {
    let pipe_name = if pipe_name.is_null() {
        if pipe_name_len != 0 {
            return invalid_handle_value();
        }
        &[]
    } else {
        // SAFETY: The caller supplies exactly `pipe_name_len` readable UTF-16
        // code units. The slice is copied before any Win32 call retains nothing.
        unsafe { std::slice::from_raw_parts(pipe_name, pipe_name_len) }
    };
    open_pipe_client(pipe_name, deadline, wait_when_busy != 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_close_pipe_client(pipe: *mut c_void) {
    close_pipe_client(pipe);
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `input` must be null only when `input_len` is zero, or point to a readable
/// byte buffer with exactly the provided length. `output` may be null for size
/// queries or point to writable UTF-16 storage for `capacity` code units. No
/// pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_utf8_to_wide_utf16(
    input: *const u8,
    input_len: usize,
    output: *mut u16,
    capacity: usize,
) -> Fcitx5WindowsCommonUtf8ToWide {
    let input = if input.is_null() {
        if input_len != 0 {
            return Fcitx5WindowsCommonUtf8ToWide::default();
        }
        &[]
    } else {
        // SAFETY: The caller supplies exactly `input_len` readable bytes. The
        // slice is only decoded/copied during this call.
        unsafe { std::slice::from_raw_parts(input, input_len) }
    };
    utf8_to_wide(input, output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `input` must be null only when `input_len` is zero, or point to a readable
/// UTF-16 buffer with exactly the provided length. `output` may be null for
/// size queries or point to writable byte storage for `capacity` bytes. No
/// pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_wide_utf16_to_utf8(
    input: *const u16,
    input_len: usize,
    output: *mut u8,
    capacity: usize,
) -> Fcitx5WindowsCommonWideToUtf8 {
    let input = if input.is_null() {
        if input_len != 0 {
            return Fcitx5WindowsCommonWideToUtf8::default();
        }
        &[]
    } else {
        // SAFETY: The caller supplies exactly `input_len` readable UTF-16 code
        // units. The slice is only decoded/copied during this call.
        unsafe { std::slice::from_raw_parts(input, input_len) }
    };
    wide_to_utf8(input, output, capacity)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `input` must be null only when `input_len` is zero, or point to a readable
/// byte buffer with exactly the provided length. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_utf8_offset_to_wide(
    input: *const u8,
    input_len: usize,
    offset: u32,
) -> Fcitx5WindowsCommonUtf8OffsetToWide {
    let input = if input.is_null() {
        if input_len != 0 {
            return Fcitx5WindowsCommonUtf8OffsetToWide::default();
        }
        &[]
    } else {
        // SAFETY: The caller supplies exactly `input_len` readable bytes. The
        // slice is only decoded during this call.
        unsafe { std::slice::from_raw_parts(input, input_len) }
    };
    utf8_offset_to_wide(input, offset)
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_accept_hello_response(
    response_to: u64,
    session_id: u32,
    status: u32,
    expected_request_id: u64,
    expected_session_id: u32,
) -> u8 {
    accept_hello_response(
        response_to,
        session_id,
        status,
        expected_request_id,
        expected_session_id,
    ) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_apply_hello_response_scalars(
    response_to: u64,
    engine_epoch: u64,
    session_id: u32,
    status: u32,
    expected_request_id: u64,
    expected_session_id: u32,
) -> Fcitx5WindowsCommonHelloResponseScalars {
    apply_hello_response_scalars(
        response_to,
        engine_epoch,
        session_id,
        status,
        expected_request_id,
        expected_session_id,
    )
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn fcitx5_windows_common_accept_key_response(
    response_to: u64,
    engine_epoch: u64,
    session_id: u32,
    context_id: u64,
    revision: u64,
    status: u32,
    expected_request_id: u64,
    expected_engine_epoch: u64,
    expected_session_id: u32,
    expected_context_id: u64,
    previous_revision: u64,
) -> u8 {
    accept_key_response(
        response_to,
        engine_epoch,
        session_id,
        context_id,
        revision,
        status,
        expected_request_id,
        expected_engine_epoch,
        expected_session_id,
        expected_context_id,
        previous_revision,
    ) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_apply_key_response_scalars(
    input: Fcitx5WindowsCommonKeyResponseScalarInput,
) -> Fcitx5WindowsCommonKeyResponseScalars {
    apply_key_response_scalars(input)
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn fcitx5_windows_common_accept_candidate_select_response(
    response_to: u64,
    engine_epoch: u64,
    session_id: u32,
    context_id: u64,
    revision: u64,
    status: u32,
    expected_request_id: u64,
    expected_engine_epoch: u64,
    expected_session_id: u32,
    expected_context_id: u64,
    previous_revision: u64,
) -> u8 {
    accept_candidate_select_response(
        response_to,
        engine_epoch,
        session_id,
        context_id,
        revision,
        status,
        expected_request_id,
        expected_engine_epoch,
        expected_session_id,
        expected_context_id,
        previous_revision,
    ) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_accept_candidate_select_request(
    current_engine_epoch: u64,
    expected_engine_epoch: u64,
    target_process_id: u32,
    context_id: u64,
    composition_id: u64,
    revision: u64,
    candidate_id: u64,
) -> u8 {
    accept_candidate_select_request(
        current_engine_epoch,
        expected_engine_epoch,
        target_process_id,
        context_id,
        composition_id,
        revision,
        candidate_id,
    ) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_accept_engine_status_response(
    response_to: u64,
    engine_epoch: u64,
    session_id: u32,
    status: u32,
    expected_request_id: u64,
    expected_engine_epoch: u64,
    expected_session_id: u32,
) -> u8 {
    accept_engine_status_response(
        response_to,
        engine_epoch,
        session_id,
        status,
        expected_request_id,
        expected_engine_epoch,
        expected_session_id,
    ) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_apply_engine_status_response_scalars(
    input: Fcitx5WindowsCommonEngineStatusResponseScalarInput,
) -> Fcitx5WindowsCommonEngineStatusResponseScalars {
    apply_engine_status_response_scalars(input)
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_accept_launcher_response(
    response_to: u64,
    session_id: u32,
    expected_request_id: u64,
    expected_session_id: u32,
) -> u8 {
    accept_launcher_response(
        response_to,
        session_id,
        expected_request_id,
        expected_session_id,
    ) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_apply_launcher_response_scalars(
    input: Fcitx5WindowsCommonLauncherResponseScalarInput,
) -> Fcitx5WindowsCommonLauncherResponseScalars {
    apply_launcher_response_scalars(input)
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_next_launcher_request_id() -> u64 {
    next_launcher_request_id()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_next_pipe_client_request_id() -> u64 {
    next_pipe_client_request_id()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_ipc_status_ok(status: u32) -> u8 {
    ipc_status_ok(status) as u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_set_last_error(error: u32) {
    // SAFETY: Sets the thread-local Win32 last-error value for the caller.
    unsafe { SetLastError(error) };
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `pipe` must be a live named-pipe handle. `current_user_sid` and
/// `expected_executable_path` must be null only when their corresponding length
/// is zero, or point to valid UTF-16 buffers with exactly the provided lengths.
/// No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_verify_pipe_server_peer_utf16(
    pipe: *mut c_void,
    current_service_account: u8,
    current_session_id: u32,
    current_secure_desktop: u8,
    current_user_sid: *const u16,
    current_user_sid_len: usize,
    policy_mode: u32,
    expected_executable_path: *const u16,
    expected_executable_path_len: usize,
    development_exception_enabled: u8,
) -> u8 {
    let Some(current_user_sid) = wide_string_from_raw(current_user_sid, current_user_sid_len)
    else {
        return 0;
    };
    let expected_executable_path = if expected_executable_path.is_null() {
        if expected_executable_path_len != 0 {
            return 0;
        }
        &[]
    } else {
        // SAFETY: The caller supplies exactly `expected_executable_path_len`
        // readable UTF-16 code units. The slice is only used during this call.
        unsafe {
            std::slice::from_raw_parts(expected_executable_path, expected_executable_path_len)
        }
    };
    verify_pipe_server_peer(
        pipe,
        current_service_account != 0,
        current_session_id,
        current_secure_desktop != 0,
        &current_user_sid,
        policy_mode,
        expected_executable_path,
        development_exception_enabled != 0,
    ) as u8
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `pipe` must be a live named-pipe handle. `current_user_sid` must be null
/// only when `current_user_sid_len` is zero, or point to a valid UTF-16 buffer
/// with exactly the provided length. Output pointers may be null for size
/// queries or point to writable UTF-16 storage for their paired capacities. No
/// pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_verify_pipe_client_peer_utf16(
    pipe: *mut c_void,
    current_service_account: u8,
    current_session_id: u32,
    current_secure_desktop: u8,
    current_user_sid: *const u16,
    current_user_sid_len: usize,
    user_sid_output: *mut u16,
    user_sid_capacity: usize,
    executable_path_output: *mut u16,
    executable_path_capacity: usize,
    executable_final_path_output: *mut u16,
    executable_final_path_capacity: usize,
) -> Fcitx5WindowsCommonVerifiedPipeClient {
    let Some(current_user_sid) = wide_string_from_raw(current_user_sid, current_user_sid_len)
    else {
        return Fcitx5WindowsCommonVerifiedPipeClient::default();
    };
    verified_pipe_client_peer(
        pipe,
        current_service_account != 0,
        current_session_id,
        current_secure_desktop != 0,
        &current_user_sid,
        user_sid_output,
        user_sid_capacity,
        executable_path_output,
        executable_path_capacity,
        executable_final_path_output,
        executable_final_path_capacity,
    )
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
/// `path` must point to exactly `path_len` readable UTF-16 code units.
pub unsafe extern "C" fn fcitx5_windows_common_basic_file_identity_utf16(
    path: *const u16,
    path_len: usize,
) -> Fcitx5WindowsCommonBasicFileIdentity {
    if path.is_null() {
        return Fcitx5WindowsCommonBasicFileIdentity::default();
    }
    // SAFETY: The caller supplies exactly `path_len` readable UTF-16 code units.
    // The path is copied and not retained.
    basic_file_identity(unsafe { std::slice::from_raw_parts(path, path_len) })
}

#[unsafe(no_mangle)]
/// # Safety
///
/// Both path pointers must point to exactly their corresponding readable UTF-16
/// code-unit lengths. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_paths_refer_to_same_file_utf16(
    left_path: *const u16,
    left_path_len: usize,
    right_path: *const u16,
    right_path_len: usize,
) -> u8 {
    if left_path.is_null() || right_path.is_null() {
        return 0;
    }
    let left = unsafe { std::slice::from_raw_parts(left_path, left_path_len) };
    let right = unsafe { std::slice::from_raw_parts(right_path, right_path_len) };
    paths_refer_to_same_file(left, right) as u8
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `path` must point to exactly `path_len` readable UTF-16 code units.
/// `final_path_output` must point to writable storage for `final_path_capacity`
/// UTF-16 code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_executable_file_identity_utf16(
    path: *const u16,
    path_len: usize,
    final_path_output: *mut u16,
    final_path_capacity: usize,
) -> Fcitx5WindowsCommonExecutableFileIdentity {
    if path.is_null() {
        return Fcitx5WindowsCommonExecutableFileIdentity::default();
    }
    // SAFETY: The caller supplies exactly `path_len` readable UTF-16 code units.
    // The path is copied and not retained.
    executable_file_identity(
        unsafe { std::slice::from_raw_parts(path, path_len) },
        final_path_output,
        final_path_capacity,
    )
}

#[unsafe(no_mangle)]
/// # Safety
///
/// Both path pointers must point to exactly their corresponding readable UTF-16
/// code-unit lengths. No pointer is retained.
pub unsafe extern "C" fn fcitx5_windows_common_executable_paths_match_utf16(
    left_path: *const u16,
    left_path_len: usize,
    right_path: *const u16,
    right_path_len: usize,
) -> u8 {
    if left_path.is_null() || right_path.is_null() {
        return 0;
    }
    let left = unsafe { std::slice::from_raw_parts(left_path, left_path_len) };
    let right = unsafe { std::slice::from_raw_parts(right_path, right_path_len) };
    executable_paths_match(left, right) as u8
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
    use std::sync::atomic::{AtomicU64, Ordering as TestOrdering};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateNamedPipeW(
            name: *const u16,
            open_mode: u32,
            pipe_mode: u32,
            max_instances: u32,
            out_buffer_size: u32,
            in_buffer_size: u32,
            default_timeout: u32,
            security_attributes: *mut c_void,
        ) -> *mut c_void;
        fn DisconnectNamedPipe(pipe: *mut c_void) -> i32;
    }

    const PIPE_ACCESS_DUPLEX: u32 = 0x0000_0003;
    const PIPE_TYPE_BYTE: u32 = 0x0000_0000;
    const PIPE_READMODE_BYTE: u32 = 0x0000_0000;
    const PIPE_WAIT: u32 = 0x0000_0000;
    static NEXT_TEST_PIPE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    #[derive(Debug)]
    enum PipeServerEvent {
        Connected,
        Received(Vec<u8>),
        Disconnected,
        Failed,
    }

    fn next_test_pipe_name() -> OsString {
        let sequence = NEXT_TEST_PIPE_SEQUENCE.fetch_add(1, TestOrdering::Relaxed);
        let process_id = std::process::id();
        OsString::from(format!(
            r"\\.\pipe\fcitx5-windows-common-core-verified-client-{process_id}-{sequence}"
        ))
    }

    fn spawn_local_pipe_server<F>(body: F) -> (OsString, Receiver<PipeServerEvent>, JoinHandle<()>)
    where
        F: FnOnce(*mut c_void, &Sender<PipeServerEvent>) + Send + 'static,
    {
        let pipe_name = next_test_pipe_name();
        let mut wide_name: Vec<u16> = pipe_name.encode_wide().collect();
        wide_name.push(0);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let (event_sender, event_receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            // SAFETY: `wide_name` is an owned null-terminated pipe name. The
            // server handle is closed on every path below.
            let pipe = unsafe {
                CreateNamedPipeW(
                    wide_name.as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    1,
                    64,
                    64,
                    0,
                    std::ptr::null_mut(),
                )
            };
            if pipe.is_null() || pipe == invalid_handle_value() {
                let _ = ready_sender.send(false);
                return;
            }
            let _ = ready_sender.send(true);
            // SAFETY: `pipe` is a synchronous server handle. A client connects
            // after the ready barrier is released.
            let connected = unsafe { ConnectNamedPipe(pipe, std::ptr::null_mut()) } != 0
                || unsafe {
                    // SAFETY: Reads the thread-local error from ConnectNamedPipe.
                    GetLastError() == 535
                };
            if connected {
                body(pipe, &event_sender);
            } else {
                let _ = event_sender.send(PipeServerEvent::Failed);
            }
            // SAFETY: `pipe` is the live server handle created above. Disconnect
            // is best-effort before exactly one CloseHandle call.
            unsafe {
                DisconnectNamedPipe(pipe);
                CloseHandle(pipe);
            }
        });
        assert!(
            ready_receiver
                .recv_timeout(Duration::from_secs(2))
                .expect("server fixture ready"),
            "server fixture created its pipe"
        );
        (pipe_name, event_receiver, server)
    }

    fn next_pipe_server_event(events: &Receiver<PipeServerEvent>) -> PipeServerEvent {
        events
            .recv_timeout(Duration::from_secs(2))
            .expect("server fixture event")
    }

    fn read_partial_frame(pipe: *mut c_void, len: usize) -> Option<Vec<u8>> {
        let mut received = Vec::with_capacity(len);
        while received.len() < len {
            let mut chunk = [0_u8; 3];
            let mut transferred = 0_u32;
            let count = (len - received.len()).min(chunk.len());
            // SAFETY: `pipe` is the live fixture server handle and `chunk`
            // provides `count` writable bytes for this synchronous read.
            let ok = unsafe {
                ReadFile(
                    pipe,
                    chunk.as_mut_ptr().cast::<c_void>(),
                    count as u32,
                    &mut transferred,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 || transferred == 0 || transferred as usize > count {
                return None;
            }
            received.extend_from_slice(&chunk[..transferred as usize]);
        }
        Some(received)
    }

    fn peer_disconnected(pipe: *mut c_void) -> bool {
        let mut byte = 0_u8;
        let mut transferred = 0_u32;
        // SAFETY: `pipe` is the live fixture server handle and `byte` is
        // writable storage for this synchronous read.
        let ok = unsafe {
            ReadFile(
                pipe,
                (&mut byte as *mut u8).cast::<c_void>(),
                1,
                &mut transferred,
                std::ptr::null_mut(),
            )
        };
        ok == 0 || transferred == 0
    }

    #[test]
    fn verified_pipe_client_connects_to_exact_current_executable_and_writes_full_frame() {
        let expected = env::current_exe().expect("current test executable");
        let frame = (0_u8..=255).cycle().take(4096).collect::<Vec<_>>();
        let expected_frame = frame.clone();
        let (pipe_name, events, server) = spawn_local_pipe_server(move |pipe, events| {
            let _ = events.send(PipeServerEvent::Connected);
            let event = read_partial_frame(pipe, expected_frame.len())
                .map(PipeServerEvent::Received)
                .unwrap_or(PipeServerEvent::Failed);
            let _ = events.send(event);
        });

        let mut client = VerifiedPipeClient::connect_exact(
            pipe_name.as_os_str(),
            &expected,
            Duration::from_millis(500),
        )
        .expect("verified client");
        assert!(matches!(
            next_pipe_server_event(&events),
            PipeServerEvent::Connected
        ));
        assert!(client.write_all(&frame, Duration::from_secs(1)));
        assert!(matches!(
            next_pipe_server_event(&events),
            PipeServerEvent::Received(received) if received == frame
        ));
        drop(client);
        server.join().expect("server thread");
    }

    #[test]
    fn verified_pipe_client_rejects_wrong_server_executable() {
        let wrong_executable = PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot"))
            .join("System32")
            .join("cmd.exe");
        assert!(wrong_executable.is_file(), "wrong executable fixture");
        let (pipe_name, events, server) = spawn_local_pipe_server(|pipe, events| {
            let _ = events.send(PipeServerEvent::Connected);
            let event = if peer_disconnected(pipe) {
                PipeServerEvent::Disconnected
            } else {
                PipeServerEvent::Failed
            };
            let _ = events.send(event);
        });

        assert!(VerifiedPipeClient::connect_exact(
            pipe_name.as_os_str(),
            &wrong_executable,
            Duration::from_millis(500),
        )
        .is_none());
        assert!(matches!(
            next_pipe_server_event(&events),
            PipeServerEvent::Connected
        ));
        assert!(matches!(
            next_pipe_server_event(&events),
            PipeServerEvent::Disconnected
        ));
        server.join().expect("server thread");
    }

    #[test]
    fn verified_pipe_client_write_fails_after_server_disconnects() {
        let expected = env::current_exe().expect("current test executable");
        let (release_sender, release_receiver) = mpsc::sync_channel(1);
        let (pipe_name, events, server) = spawn_local_pipe_server(move |pipe, events| {
            let _ = events.send(PipeServerEvent::Connected);
            let _ = release_receiver.recv_timeout(Duration::from_secs(2));
            // SAFETY: `pipe` is the live fixture server handle. Disconnecting
            // it makes the peer's next write fail deterministically.
            unsafe {
                DisconnectNamedPipe(pipe);
            }
            let _ = events.send(PipeServerEvent::Disconnected);
        });
        let mut client = VerifiedPipeClient::connect_exact(
            pipe_name.as_os_str(),
            &expected,
            Duration::from_millis(500),
        )
        .expect("verified client");
        assert!(matches!(
            next_pipe_server_event(&events),
            PipeServerEvent::Connected
        ));
        release_sender.send(()).expect("release server disconnect");
        assert!(matches!(
            next_pipe_server_event(&events),
            PipeServerEvent::Disconnected
        ));
        assert!(!client.write_all(&[1], Duration::from_millis(100)));
        drop(client);
        server.join().expect("server thread");
    }

    #[test]
    fn verified_pipe_client_busy_wait_is_bounded() {
        let expected = env::current_exe().expect("current test executable");
        let (release_sender, release_receiver) = mpsc::sync_channel(1);
        let (pipe_name, events, server) = spawn_local_pipe_server(move |_pipe, events| {
            let _ = events.send(PipeServerEvent::Connected);
            let _ = release_receiver.recv_timeout(Duration::from_secs(2));
        });
        let client = VerifiedPipeClient::connect_exact(
            pipe_name.as_os_str(),
            &expected,
            Duration::from_millis(500),
        )
        .expect("first verified client");
        assert!(matches!(
            next_pipe_server_event(&events),
            PipeServerEvent::Connected
        ));

        let started = Instant::now();
        assert!(VerifiedPipeClient::connect_exact(
            pipe_name.as_os_str(),
            &expected,
            Duration::from_millis(50),
        )
        .is_none());
        assert!(started.elapsed() < Duration::from_secs(1));
        drop(client);
        release_sender.send(()).expect("release server");
        server.join().expect("server thread");
    }

    #[test]
    fn verified_pipe_client_drop_disconnects_the_server_once() {
        let expected = env::current_exe().expect("current test executable");
        let (pipe_name, events, server) = spawn_local_pipe_server(|pipe, events| {
            let _ = events.send(PipeServerEvent::Connected);
            let event = if peer_disconnected(pipe) {
                PipeServerEvent::Disconnected
            } else {
                PipeServerEvent::Failed
            };
            let _ = events.send(event);
        });
        let client = VerifiedPipeClient::connect_exact(
            pipe_name.as_os_str(),
            &expected,
            Duration::from_millis(500),
        )
        .expect("verified client");
        assert!(matches!(
            next_pipe_server_event(&events),
            PipeServerEvent::Connected
        ));
        drop(client);
        assert!(matches!(
            next_pipe_server_event(&events),
            PipeServerEvent::Disconnected
        ));
        server.join().expect("server thread");
    }

    fn wide_units(value: &str) -> Vec<u16> {
        value.encode_utf16().collect()
    }

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
    fn local_test_namespace_matches_cpp_contract() {
        unsafe {
            env::set_var("FCITX5_TEST_NAMESPACE", "contract-42");
        }
        assert_eq!(local_test_namespace().as_deref(), Some("contract-42"));
        unsafe {
            env::set_var("FCITX5_TEST_NAMESPACE", "../bad");
        }
        assert_eq!(local_test_namespace(), None);
        unsafe {
            env::remove_var("FCITX5_TEST_NAMESPACE");
        }
    }

    #[test]
    fn current_runtime_generation_abi_matches_cpp_contract() {
        let generation = current_runtime_generation();
        assert!(!generation.is_empty());
        assert!(valid_generation(&generation));
    }

    #[test]
    fn process_image_path_query_matches_cpp_contract() {
        let current_process_id = unsafe { GetCurrentProcessId() };
        let path = process_image_path(current_process_id).expect("current process image path");
        assert!(path.to_ascii_lowercase().ends_with(".exe"));
        assert!(process_image_path(0).is_none());
    }

    #[test]
    fn process_session_id_query_matches_cpp_contract() {
        let current_process_id = unsafe { GetCurrentProcessId() };
        let session = process_session_id(current_process_id);
        assert_eq!(session.status, 1);
        assert_eq!(process_session_id(0).status, 0);
    }

    #[test]
    fn process_user_sid_query_matches_cpp_contract() {
        let current_process_id = unsafe { GetCurrentProcessId() };
        let (sid, _service_account) =
            process_user_sid(current_process_id).expect("current process user sid");
        let sid = String::from_utf16(&sid).expect("sid is utf16");
        assert!(sid.starts_with("S-1-"));
        assert!(process_user_sid(0).is_none());
    }

    #[test]
    fn process_identity_query_matches_cpp_contract() {
        let current_process_id = unsafe { GetCurrentProcessId() };
        let query = process_identity(
            current_process_id,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
        );
        assert_eq!(query.status, 1);
        assert!(query.user_sid_len > 0);
        assert!(query.executable_path_len > 0);
        let mut sid = vec![0_u16; query.user_sid_len];
        let mut path = vec![0_u16; query.executable_path_len];
        let filled = process_identity(
            current_process_id,
            sid.as_mut_ptr(),
            sid.len(),
            path.as_mut_ptr(),
            path.len(),
        );
        assert_eq!(filled.status, 1);
        assert_eq!(filled.user_sid_len, sid.len());
        assert_eq!(filled.executable_path_len, path.len());
        assert!(String::from_utf16(&sid).expect("sid").starts_with("S-1-"));
        assert!(String::from_utf16(&path)
            .expect("path")
            .to_ascii_lowercase()
            .ends_with(".exe"));
        assert_eq!(
            process_identity(0, std::ptr::null_mut(), 0, std::ptr::null_mut(), 0).status,
            0
        );
    }

    #[test]
    fn current_identity_query_matches_cpp_contract() {
        let query = current_identity(std::ptr::null_mut(), 0, std::ptr::null_mut(), 0);
        assert_eq!(query.status, 1);
        assert_eq!(query.process_id, unsafe { GetCurrentProcessId() });
        assert!(query.user_sid_len > 0);
        assert!(query.executable_path_len > 0);
        let mut sid = vec![0_u16; query.user_sid_len];
        let mut path = vec![0_u16; query.executable_path_len];
        let filled = current_identity(sid.as_mut_ptr(), sid.len(), path.as_mut_ptr(), path.len());
        assert_eq!(filled.status, 1);
        assert_eq!(filled.user_sid_len, sid.len());
        assert_eq!(filled.executable_path_len, path.len());
        assert!(String::from_utf16(&sid).expect("sid").starts_with("S-1-"));
        assert!(String::from_utf16(&path)
            .expect("path")
            .to_ascii_lowercase()
            .ends_with(".exe"));
    }

    #[test]
    fn process_identity_with_executable_file_matches_cpp_contract() {
        let current_process_id = unsafe { GetCurrentProcessId() };
        let query = process_identity_with_executable_file(
            current_process_id,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
        );
        assert_eq!(query.status, 1);
        assert!(query.user_sid_len > 0);
        assert!(query.executable_path_len > 0);
        assert_eq!(query.executable_file_status, 1);
        assert!(query.executable_final_path_len > 0);
        let mut sid = vec![0_u16; query.user_sid_len];
        let mut path = vec![0_u16; query.executable_path_len];
        let mut final_path = vec![0_u16; query.executable_final_path_len];
        let filled = process_identity_with_executable_file(
            current_process_id,
            sid.as_mut_ptr(),
            sid.len(),
            path.as_mut_ptr(),
            path.len(),
            final_path.as_mut_ptr(),
            final_path.len(),
        );
        assert_eq!(filled.status, 1);
        assert_eq!(filled.user_sid_len, sid.len());
        assert_eq!(filled.executable_path_len, path.len());
        assert_eq!(filled.executable_file_status, 1);
        assert_eq!(filled.executable_final_path_len, final_path.len());
        assert!(String::from_utf16(&sid).expect("sid").starts_with("S-1-"));
        assert!(String::from_utf16(&path)
            .expect("path")
            .to_ascii_lowercase()
            .ends_with(".exe"));
        assert!(String::from_utf16(&final_path)
            .expect("final path")
            .to_ascii_lowercase()
            .ends_with(".exe"));
        assert_eq!(
            process_identity_with_executable_file(
                0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                0
            )
            .status,
            0
        );
    }

    #[test]
    fn current_identity_with_executable_file_matches_cpp_contract() {
        let query = current_identity_with_executable_file(
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
        );
        assert_eq!(query.status, 1);
        assert_eq!(query.process_id, unsafe { GetCurrentProcessId() });
        assert_eq!(query.executable_file_status, 1);
        assert!(query.user_sid_len > 0);
        assert!(query.executable_path_len > 0);
        assert!(query.executable_final_path_len > 0);
        let mut sid = vec![0_u16; query.user_sid_len];
        let mut path = vec![0_u16; query.executable_path_len];
        let mut final_path = vec![0_u16; query.executable_final_path_len];
        let filled = current_identity_with_executable_file(
            sid.as_mut_ptr(),
            sid.len(),
            path.as_mut_ptr(),
            path.len(),
            final_path.as_mut_ptr(),
            final_path.len(),
        );
        assert_eq!(filled.status, 1);
        assert_eq!(filled.user_sid_len, sid.len());
        assert_eq!(filled.executable_path_len, path.len());
        assert_eq!(filled.executable_file_status, 1);
        assert_eq!(filled.executable_final_path_len, final_path.len());
        assert!(String::from_utf16(&sid).expect("sid").starts_with("S-1-"));
        assert!(String::from_utf16(&path)
            .expect("path")
            .to_ascii_lowercase()
            .ends_with(".exe"));
        assert!(String::from_utf16(&final_path)
            .expect("final path")
            .to_ascii_lowercase()
            .ends_with(".exe"));
    }

    #[test]
    fn secure_input_desktop_query_matches_cpp_contract() {
        let _ = secure_input_desktop();
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
        assert_eq!(
            default_data_root_for_module_with_local(
                &runtime_bin_module,
                Path::new("Fcitx5"),
                || Some(root.join("local"))
            )
            .as_deref(),
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
    fn default_data_root_falls_back_to_local_app_data_contract() {
        let root = env::temp_dir().join(format!(
            "fcitx5-windows-common-local-data-root-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let module = root
            .join("runtime")
            .join("00000041")
            .join("fcitx5-control.exe");
        assert_eq!(
            default_data_root_for_module_with_local(&module, Path::new("Fcitx5"), || {
                Some(root.join("local-app-data"))
            })
            .as_deref(),
            Some(root.join("local-app-data").join("Fcitx5").as_path())
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn release_data_directory_rejects_absolute_or_traversal_contract() {
        let empty = wide_units("");
        let absolute = wide_units(r"C:\Users\test\AppData");
        let traversal = wide_units(r"..\Fcitx5");
        let nested = wide_units(r"Fcitx5\stable");
        assert!(release_data_directory_from_raw(empty.as_ptr(), empty.len()).is_none());
        assert!(release_data_directory_from_raw(absolute.as_ptr(), absolute.len()).is_none());
        assert!(release_data_directory_from_raw(traversal.as_ptr(), traversal.len()).is_none());
        assert_eq!(
            release_data_directory_from_raw(nested.as_ptr(), nested.len()).as_deref(),
            Some(Path::new(r"Fcitx5\stable"))
        );
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
    fn pipe_security_state_matches_cpp_attributes_contract() {
        let current_process_id = unsafe { GetCurrentProcessId() };
        let (sid, _service_account) =
            process_user_sid(current_process_id).expect("current process user sid");
        let sid = String::from_utf16(&sid).expect("sid is utf16");
        let state = pipe_security_state(false, 7, &sid).expect("valid pipe security state");
        assert!(!state.descriptor.is_null());
        assert_eq!(
            state.attributes.n_length,
            std::mem::size_of::<SecurityAttributes>() as u32
        );
        assert_eq!(state.attributes.security_descriptor, state.descriptor);
        assert_eq!(state.attributes.inherit_handle, 0);
        assert!(pipe_security_state(true, 7, &sid).is_none());
        assert!(pipe_security_state(false, 0, &sid).is_none());
        assert!(pipe_security_state(false, 7, "").is_none());
    }

    #[test]
    fn current_user_security_attributes_use_current_runtime_identity() {
        let security =
            CurrentUserSecurityAttributes::new().expect("current user pipe security attributes");

        let attributes = security.attributes();

        assert_eq!(
            attributes.n_length,
            std::mem::size_of::<SecurityAttributes>() as u32
        );
        assert!(!attributes.security_descriptor.is_null());
        assert_eq!(attributes.inherit_handle, 0);
    }

    #[test]
    fn current_user_security_attributes_fail_closed_for_invalid_identity() {
        const USER_SID: &str = "S-1-5-21-100";

        assert!(CurrentUserSecurityAttributes::from_identity(true, 7, false, USER_SID).is_none());
        assert!(CurrentUserSecurityAttributes::from_identity(false, 0, false, USER_SID).is_none());
        assert!(CurrentUserSecurityAttributes::from_identity(false, 7, false, "").is_none());
    }

    #[test]
    fn pipe_security_abi_preserves_owned_descriptor_contract() {
        let current_process_id = unsafe { GetCurrentProcessId() };
        let (user_sid, _) = process_user_sid(current_process_id).expect("current process user sid");

        // SAFETY: `user_sid` remains live for this length-delimited ABI call.
        let state = unsafe {
            fcitx5_windows_common_pipe_security_create_utf16(
                0,
                7,
                user_sid.as_ptr(),
                user_sid.len(),
            )
        };
        assert!(!state.is_null());
        // SAFETY: `state` is the still-live handle returned just above.
        let attributes = unsafe {
            fcitx5_windows_common_pipe_security_attributes(state).cast::<SecurityAttributes>()
        };
        assert!(!attributes.is_null());
        // SAFETY: the returned attributes are borrowed from the live `state`.
        let attributes = unsafe { &*attributes };
        assert_eq!(
            attributes.n_length,
            std::mem::size_of::<SecurityAttributes>() as u32
        );
        assert!(!attributes.security_descriptor.is_null());
        assert_eq!(attributes.inherit_handle, 0);
        // SAFETY: `state` has not yet been destroyed and is released once here.
        unsafe {
            fcitx5_windows_common_pipe_security_destroy(state);
        }

        // SAFETY: `user_sid` remains live for this length-delimited ABI call.
        let descriptor = unsafe {
            fcitx5_windows_common_pipe_security_descriptor_utf16(
                0,
                7,
                user_sid.as_ptr(),
                user_sid.len(),
            )
        };
        assert!(!descriptor.is_null());
        // SAFETY: The descriptor ABI transfers the LocalAlloc allocation to
        // the caller, which releases it exactly once here.
        unsafe {
            LocalFree(descriptor);
        }

        // SAFETY: the UTF-16 input remains live for the duration of each ABI call.
        assert!(unsafe {
            fcitx5_windows_common_pipe_security_create_utf16(
                1,
                7,
                user_sid.as_ptr(),
                user_sid.len(),
            )
        }
        .is_null());
        // SAFETY: the UTF-16 input remains live for the duration of each ABI call.
        assert!(unsafe {
            fcitx5_windows_common_pipe_security_create_utf16(
                0,
                0,
                user_sid.as_ptr(),
                user_sid.len(),
            )
        }
        .is_null());
        // SAFETY: a null UTF-16 pointer with zero length is permitted by this ABI.
        assert!(unsafe {
            fcitx5_windows_common_pipe_security_create_utf16(0, 7, std::ptr::null(), 0)
        }
        .is_null());
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
    fn pipe_server_peer_policy_rejects_invalid_pipe_like_cpp_contract() {
        assert!(pipe_server_process_id(std::ptr::null_mut()).is_none());
        assert!(pipe_server_process_id(invalid_handle_value()).is_none());
        assert!(!verify_pipe_server_peer(
            std::ptr::null_mut(),
            false,
            7,
            false,
            "S-1-5-21-test",
            0,
            &[],
            false
        ));
    }

    #[test]
    fn pipe_transfer_rejects_invalid_pipe_like_cpp_contract() {
        assert!(!pipe_transfer(
            std::ptr::null_mut(),
            true,
            std::ptr::null_mut(),
            1,
            unsafe { GetTickCount64() } + 100
        ));
        let mut byte = 0_u8;
        assert!(!pipe_transfer(
            invalid_handle_value(),
            false,
            &mut byte,
            1,
            unsafe { GetTickCount64() } + 100
        ));
    }

    #[test]
    fn pipe_transfer_with_stop_rejects_invalid_pipe_like_cpp_contract() {
        // SAFETY: Monotonic Windows tick query with no preconditions.
        let deadline = unsafe { GetTickCount64() } + 100;
        assert!(!pipe_transfer_with_stop(
            std::ptr::null_mut(),
            true,
            std::ptr::null_mut(),
            1,
            deadline,
            std::ptr::null_mut(),
        ));
        let mut byte = 0_u8;
        assert!(!pipe_transfer_with_stop(
            invalid_handle_value(),
            false,
            &mut byte,
            1,
            deadline,
            std::ptr::null_mut(),
        ));
        // SAFETY: The invalid pipe/null-buffer inputs are deliberately passed
        // to verify the C ABI's fail-closed validation path.
        assert_eq!(
            unsafe {
                fcitx5_windows_common_pipe_transfer_with_stop(
                    std::ptr::null_mut(),
                    1,
                    std::ptr::null_mut(),
                    1,
                    deadline,
                    std::ptr::null_mut(),
                )
            },
            0
        );
    }

    #[test]
    fn pipe_connect_client_rejects_invalid_pipe_like_cpp_contract() {
        // SAFETY: Monotonic Windows tick query with no preconditions.
        let deadline = unsafe { GetTickCount64() } + 100;
        assert!(!pipe_connect_client(
            std::ptr::null_mut(),
            deadline,
            std::ptr::null_mut(),
        ));
        assert!(!pipe_connect_client(
            invalid_handle_value(),
            deadline,
            std::ptr::null_mut(),
        ));
        // SAFETY: The invalid pipe is deliberately passed to verify the C ABI's
        // fail-closed validation path.
        assert_eq!(
            unsafe {
                fcitx5_windows_common_pipe_connect_client(
                    std::ptr::null_mut(),
                    deadline,
                    std::ptr::null_mut(),
                )
            },
            0
        );
    }

    #[test]
    fn pipe_transact_rejects_invalid_pipe_like_cpp_contract() {
        let request = [0_u8; IPC_HEADER_SIZE];
        let mut response = [0_u8; IPC_HEADER_SIZE];
        assert_eq!(
            pipe_transact(
                std::ptr::null_mut(),
                &request,
                response.as_mut_ptr(),
                response.len(),
                unsafe { GetTickCount64() } + 100
            )
            .status,
            0
        );
        assert_eq!(
            ipc_response_header_body_size(&[0_u8; IPC_HEADER_SIZE]),
            None
        );
    }

    #[test]
    fn pipe_transact_with_error_preserves_launcher_failure_contract() {
        let mut response = [0_u8; IPC_HEADER_SIZE];
        let invalid_request = pipe_transact_with_error(
            std::ptr::null_mut(),
            &[],
            response.as_mut_ptr(),
            response.len(),
            deadline_after_milliseconds(100),
        );
        assert_eq!(invalid_request.status, 0);
        assert_eq!(invalid_request.failure_error, ERROR_INVALID_DATA);

        let request = [0_u8; IPC_HEADER_SIZE];
        let invalid_pipe = pipe_transact_with_error(
            std::ptr::null_mut(),
            &request,
            response.as_mut_ptr(),
            response.len(),
            deadline_after_milliseconds(100),
        );
        assert_eq!(invalid_pipe.status, 0);
        assert_eq!(invalid_pipe.failure_error, ERROR_TIMEOUT);
    }

    #[test]
    fn deadline_and_current_process_id_match_cpp_contract() {
        assert_ne!(deadline_after_milliseconds(1), 0);
        assert_ne!(tick_milliseconds(), 0);
        assert_ne!(fcitx5_windows_common_tick_milliseconds(), 0);
        assert!(deadline_has_time(deadline_after_milliseconds(100)));
        assert!(!deadline_has_time(0));
        assert_ne!(unsafe { GetCurrentProcessId() }, 0);
        assert_eq!(fcitx5_windows_common_current_process_id(), unsafe {
            GetCurrentProcessId()
        });
    }

    #[test]
    fn system_dark_appearance_registry_policy_matches_cpp_contract() {
        assert!(matches!(
            fcitx5_windows_common_system_uses_dark_appearance(),
            0 | 1
        ));
        assert!(matches!(system_uses_dark_appearance(), false | true));
    }

    #[test]
    fn system_font_family_ordering_matches_cpp_contract() {
        let fonts = ordered_system_font_families(&[
            "Consolas".to_owned(),
            "@Arial".to_owned(),
            "Segoe UI Emoji".to_owned(),
            "Arial".to_owned(),
            "arial".to_owned(),
            "Microsoft YaHei".to_owned(),
        ]);
        assert_eq!(fonts[0], "Microsoft YaHei");
        assert_eq!(fonts[1], "Segoe UI Emoji");
        assert_eq!(fonts[2], "Consolas");
        assert!(fonts.iter().any(|family| family == "Arial"));
        assert_eq!(
            fonts
                .iter()
                .filter(|family| family.eq_ignore_ascii_case("Arial"))
                .count(),
            1
        );
        assert!(!fonts.iter().any(|family| family.starts_with('@')));
    }

    #[test]
    fn system_font_families_utf16_returns_picker_payload() {
        let required = fcitx5_windows_common_system_font_families_utf16(std::ptr::null_mut(), 0);
        assert!(required > 0);

        let mut payload = vec![0_u16; required];
        let filled =
            fcitx5_windows_common_system_font_families_utf16(payload.as_mut_ptr(), payload.len());
        assert_eq!(filled, required);
        assert_eq!(payload.last(), Some(&0));

        let families = payload
            .split(|code_unit| *code_unit == 0)
            .filter(|slice| !slice.is_empty())
            .map(String::from_utf16_lossy)
            .collect::<Vec<_>>();
        assert!(!families.is_empty());
        assert!(!families.iter().any(|family| family.starts_with('@')));
    }

    #[test]
    fn open_pipe_client_rejects_empty_name_like_cpp_contract() {
        assert_eq!(
            open_pipe_client(&[], unsafe { GetTickCount64() } + 100, true),
            invalid_handle_value()
        );
        assert_eq!(
            open_pipe_client(&[b'x' as u16], unsafe { GetTickCount64() }, true),
            invalid_handle_value()
        );
    }

    #[test]
    fn close_pipe_client_ignores_invalid_handle_like_cpp_contract() {
        close_pipe_client(std::ptr::null_mut());
        close_pipe_client(invalid_handle_value());
        fcitx5_windows_common_close_pipe_client(std::ptr::null_mut());
        fcitx5_windows_common_close_pipe_client(invalid_handle_value());
    }

    #[test]
    fn utf8_to_wide_matches_cpp_contract() {
        let source = "abc\u{1f600}";
        let query = utf8_to_wide(source.as_bytes(), std::ptr::null_mut(), 0);
        assert_eq!(query.status, 1);
        assert_eq!(query.utf16_len, 5);
        let mut wide = vec![0_u16; query.utf16_len];
        let filled = utf8_to_wide(source.as_bytes(), wide.as_mut_ptr(), wide.len());
        assert_eq!(filled.status, 1);
        assert_eq!(filled.utf16_len, wide.len());
        assert_eq!(String::from_utf16(&wide).expect("wide"), source);
        assert_eq!(utf8_to_wide(&[0xff], std::ptr::null_mut(), 0).status, 0);
    }

    #[test]
    fn wide_to_utf8_matches_cpp_contract() {
        let source = "abc\u{1f600}";
        let wide: Vec<u16> = source.encode_utf16().collect();
        let query = wide_to_utf8(&wide, std::ptr::null_mut(), 0);
        assert_eq!(query.status, 1);
        assert_eq!(query.utf8_len, source.len());
        let mut bytes = vec![0_u8; query.utf8_len];
        let filled = wide_to_utf8(&wide, bytes.as_mut_ptr(), bytes.len());
        assert_eq!(filled.status, 1);
        assert_eq!(filled.utf8_len, bytes.len());
        assert_eq!(String::from_utf8(bytes).expect("utf8"), source);
        assert_eq!(wide_to_utf8(&[0xd800], std::ptr::null_mut(), 0).status, 0);
        let empty = wide_to_utf8(&[], std::ptr::null_mut(), 0);
        assert_eq!(empty.status, 1);
        assert_eq!(empty.utf8_len, 0);
    }

    #[test]
    fn utf8_offset_to_wide_matches_cpp_contract() {
        let source = "a\u{1f600}b";
        assert_eq!(utf8_offset_to_wide(source.as_bytes(), 0).utf16_offset, 0);
        assert_eq!(utf8_offset_to_wide(source.as_bytes(), 1).utf16_offset, 1);
        assert_eq!(utf8_offset_to_wide(source.as_bytes(), 5).utf16_offset, 3);
        assert_eq!(
            utf8_offset_to_wide(source.as_bytes(), source.len() as u32).utf16_offset,
            4
        );
        assert_eq!(utf8_offset_to_wide(source.as_bytes(), 2).status, 0);
        assert_eq!(utf8_offset_to_wide(source.as_bytes(), 99).status, 0);
    }

    #[test]
    fn ipc_response_acceptance_matches_cpp_contract() {
        assert!(accept_hello_response(11, 7, 0, 11, 7));
        assert!(!accept_hello_response(12, 7, 0, 11, 7));
        assert!(!accept_hello_response(11, 8, 0, 11, 7));
        assert!(!accept_hello_response(11, 7, 1, 11, 7));

        assert!(accept_key_response(11, 99, 7, 42, 5, 0, 11, 99, 7, 42, 4));
        assert!(!accept_key_response(11, 98, 7, 42, 5, 0, 11, 99, 7, 42, 4));
        assert!(!accept_key_response(11, 99, 7, 42, 4, 0, 11, 99, 7, 42, 4));
        assert!(!accept_key_response(11, 99, 7, 43, 5, 0, 11, 99, 7, 42, 4));
        assert!(!accept_key_response(11, 99, 7, 42, 5, 3, 11, 99, 7, 42, 4));

        assert!(accept_candidate_select_response(
            12, 99, 7, 42, 6, 0, 12, 99, 7, 42, 5
        ));
        assert!(!accept_candidate_select_response(
            12, 99, 7, 42, 5, 0, 12, 99, 7, 42, 5
        ));

        assert!(accept_candidate_select_request(99, 99, 1234, 42, 6, 5, 77));
        assert!(!accept_candidate_select_request(98, 99, 1234, 42, 6, 5, 77));
        assert!(!accept_candidate_select_request(99, 0, 1234, 42, 6, 5, 77));
        assert!(!accept_candidate_select_request(99, 99, 0, 42, 6, 5, 77));
        assert!(!accept_candidate_select_request(99, 99, 1234, 0, 6, 5, 77));
        assert!(!accept_candidate_select_request(99, 99, 1234, 42, 0, 5, 77));
        assert!(!accept_candidate_select_request(99, 99, 1234, 42, 6, 0, 77));
        assert!(!accept_candidate_select_request(99, 99, 1234, 42, 6, 5, 0));

        assert!(accept_engine_status_response(13, 99, 7, 0, 13, 99, 7));
        assert!(!accept_engine_status_response(13, 0, 7, 0, 13, 99, 7));
        assert!(!accept_engine_status_response(13, 99, 7, 5, 13, 99, 7));

        assert!(accept_launcher_response(14, 7, 14, 7));
        assert!(!accept_launcher_response(15, 7, 14, 7));
        assert!(!accept_launcher_response(14, 8, 14, 7));
    }

    #[test]
    fn hello_response_scalar_application_matches_cpp_contract() {
        let accepted = apply_hello_response_scalars(11, 99, 7, 0, 11, 7);
        assert_eq!(accepted.status, 1);
        assert_eq!(accepted.handshake_complete, 1);
        assert_eq!(accepted.engine_epoch, 99);

        let rejected = apply_hello_response_scalars(12, 99, 7, 0, 11, 7);
        assert_eq!(rejected.status, 0);
        assert_eq!(rejected.handshake_complete, 0);
        assert_eq!(rejected.engine_epoch, 0);
    }

    #[test]
    fn launcher_request_id_and_status_policy_match_cpp_contract() {
        let first = fcitx5_windows_common_next_launcher_request_id();
        let second = fcitx5_windows_common_next_launcher_request_id();
        assert_eq!(second, first + 1);
        let pipe_first = fcitx5_windows_common_next_pipe_client_request_id();
        let pipe_second = fcitx5_windows_common_next_pipe_client_request_id();
        assert_ne!(pipe_first, 0);
        assert_eq!(pipe_second, pipe_first + 1);
        assert_eq!(fcitx5_windows_common_ipc_status_ok(0), 1);
        assert_eq!(fcitx5_windows_common_ipc_status_ok(1), 0);
    }

    #[test]
    fn engine_status_response_scalar_application_matches_cpp_contract() {
        let input = Fcitx5WindowsCommonEngineStatusResponseScalarInput {
            request_id: 21,
            response_to: 13,
            engine_epoch: 99,
            session_id: 7,
            context_id: 0,
            composition_id: 0,
            revision: 0,
            status: 0,
            expected_request_id: 13,
            expected_engine_epoch: 99,
            expected_session_id: 7,
        };
        let accepted = apply_engine_status_response_scalars(input);
        assert_eq!(accepted.status, 1);
        assert_eq!(accepted.response_status, 0);
        assert_eq!(accepted.request_id, 21);
        assert_eq!(accepted.response_to, 13);
        assert_eq!(accepted.engine_epoch, 99);
        assert_eq!(accepted.session_id, 7);
        assert_eq!(accepted.context_id, 0);
        assert_eq!(accepted.composition_id, 0);
        assert_eq!(accepted.revision, 0);

        let rejected = apply_engine_status_response_scalars(
            Fcitx5WindowsCommonEngineStatusResponseScalarInput {
                response_to: 14,
                ..input
            },
        );
        assert_eq!(rejected.status, 0);
        assert_eq!(rejected.engine_epoch, 0);
    }

    #[test]
    fn launcher_response_scalar_application_matches_cpp_contract() {
        let input = Fcitx5WindowsCommonLauncherResponseScalarInput {
            request_id: 0,
            response_to: 14,
            engine_epoch: 0,
            session_id: 7,
            context_id: 0,
            composition_id: 0,
            revision: 0,
            status: 0,
            launcher_state: 2,
            engine_state: 3,
            start_disposition: 4,
            safe_mode: 1,
            retry_after_milliseconds: 1500,
            expected_request_id: 14,
            expected_session_id: 7,
        };
        let accepted = apply_launcher_response_scalars(input);
        assert_eq!(accepted.status, 1);
        assert_eq!(accepted.response_status, 0);
        assert_eq!(accepted.launcher_state, 2);
        assert_eq!(accepted.engine_state, 3);
        assert_eq!(accepted.start_disposition, 4);
        assert_eq!(accepted.safe_mode, 1);
        assert_eq!(accepted.response_to, 14);
        assert_eq!(accepted.session_id, 7);
        assert_eq!(accepted.retry_after_milliseconds, 1500);

        let rejected =
            apply_launcher_response_scalars(Fcitx5WindowsCommonLauncherResponseScalarInput {
                session_id: 8,
                ..input
            });
        assert_eq!(rejected.status, 0);
        assert_eq!(rejected.launcher_state, 0);
    }

    #[test]
    fn set_last_error_matches_cpp_contract() {
        fcitx5_windows_common_set_last_error(1234);
        assert_eq!(unsafe { GetLastError() }, 1234);
        fcitx5_windows_common_set_last_error(0);
        assert_eq!(unsafe { GetLastError() }, 0);
    }

    #[test]
    fn key_response_scalar_application_matches_cpp_contract() {
        let input = Fcitx5WindowsCommonKeyResponseScalarInput {
            response_to: 11,
            engine_epoch: 99,
            session_id: 7,
            context_id: 42,
            composition_id: 123,
            revision: 5,
            status: 0,
            expected_request_id: 11,
            expected_engine_epoch: 99,
            expected_session_id: 7,
            expected_context_id: 42,
            previous_revision: 4,
            handled: 1,
            selected_candidate: 2,
            candidate_page: 3,
            candidate_total: 9,
            candidate_visibility: 1,
            delete_surrounding_text: 1,
            delete_surrounding_offset: -1,
            delete_surrounding_size: 2,
            forward_key: 1,
            forward_key_sym: 0xff0d,
            forward_key_states: 4,
            forward_key_code: 28,
            forward_key_release: 1,
            caret_valid: 1,
            caret_left: 10,
            caret_top: 20,
            caret_right: 30,
            caret_bottom: 40,
            caret_dpi: 144,
        };
        let applied = apply_key_response_scalars(input);
        assert_eq!(applied.status, 1);
        assert_eq!(applied.handled, 1);
        assert_eq!(applied.engine_epoch, 99);
        assert_eq!(applied.context_composition_id, 123);
        assert_eq!(applied.context_revision, 5);
        assert_eq!(applied.result_composition_id, 123);
        assert_eq!(applied.result_revision, 5);
        assert_eq!(applied.selected_candidate, 2);
        assert_eq!(applied.candidate_page, 3);
        assert_eq!(applied.candidate_total, 9);
        assert_eq!(applied.candidate_visibility, 1);
        assert_eq!(applied.delete_surrounding_text, 1);
        assert_eq!(applied.delete_surrounding_offset, -1);
        assert_eq!(applied.delete_surrounding_size, 2);
        assert_eq!(applied.forward_key, 1);
        assert_eq!(applied.forward_key_sym, 0xff0d);
        assert_eq!(applied.forward_key_states, 4);
        assert_eq!(applied.forward_key_code, 28);
        assert_eq!(applied.forward_key_release, 1);
        assert_eq!(applied.caret_valid, 1);
        assert_eq!(applied.caret_left, 10);
        assert_eq!(applied.caret_top, 20);
        assert_eq!(applied.caret_right, 30);
        assert_eq!(applied.caret_bottom, 40);
        assert_eq!(applied.caret_dpi, 144);

        let stale = Fcitx5WindowsCommonKeyResponseScalarInput {
            revision: 4,
            ..input
        };
        assert_eq!(apply_key_response_scalars(stale).status, 0);
    }

    #[test]
    fn pipe_client_peer_policy_rejects_invalid_pipe_like_cpp_contract() {
        assert!(pipe_client_process_id(std::ptr::null_mut()).is_none());
        assert!(pipe_client_process_id(invalid_handle_value()).is_none());
        assert_eq!(
            verified_pipe_client_peer(
                std::ptr::null_mut(),
                false,
                7,
                false,
                "S-1-5-21-test",
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                0
            )
            .status,
            0
        );
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
    fn paths_refer_to_same_file_query_matches_cpp_contract() {
        let current = env::current_exe().expect("current exe");
        let current_wide: Vec<u16> = current.as_os_str().encode_wide().collect();
        let missing = env::temp_dir().join(format!(
            "fcitx5-windows-common-missing-basic-file-{}",
            std::process::id()
        ));
        let missing_wide: Vec<u16> = missing.as_os_str().encode_wide().collect();
        assert!(paths_refer_to_same_file(&current_wide, &current_wide));
        assert!(!paths_refer_to_same_file(&current_wide, &missing_wide));
        assert!(!paths_refer_to_same_file(&[], &current_wide));
    }

    #[test]
    fn basic_file_identity_query_rejects_empty_path_like_cpp_contract() {
        let empty = basic_file_identity(&[]);
        assert_eq!(empty.status, 0);
    }

    #[test]
    fn executable_file_identity_query_rejects_empty_path_like_cpp_contract() {
        let mut final_path = [0_u16; 16];
        let empty = executable_file_identity(&[], final_path.as_mut_ptr(), final_path.len());
        assert_eq!(empty.status, 0);
    }

    #[test]
    fn executable_paths_match_query_matches_cpp_contract() {
        let current = env::current_exe().expect("current exe");
        let current_wide: Vec<u16> = current.as_os_str().encode_wide().collect();
        let missing = env::temp_dir().join(format!(
            "fcitx5-windows-common-missing-executable-file-{}",
            std::process::id()
        ));
        let missing_wide: Vec<u16> = missing.as_os_str().encode_wide().collect();
        assert!(executable_paths_match(&current_wide, &current_wide));
        assert!(!executable_paths_match(&current_wide, &missing_wide));
        assert!(!executable_paths_match(&[], &current_wide));
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
