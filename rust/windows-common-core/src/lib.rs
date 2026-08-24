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
const MAX_DWORD_MINUS_ONE: u64 = u32::MAX as u64 - 1;

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

#[repr(C)]
struct SecurityAttributes {
    n_length: u32,
    security_descriptor: *mut c_void,
    inherit_handle: i32,
}

struct PipeSecurityState {
    descriptor: *mut c_void,
    attributes: SecurityAttributes,
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
) -> Option<Box<PipeSecurityState>> {
    let descriptor = pipe_security_descriptor(service_account, session_id, user_sid)?;
    Some(Box::new(PipeSecurityState {
        descriptor,
        attributes: SecurityAttributes {
            n_length: std::mem::size_of::<SecurityAttributes>() as u32,
            security_descriptor: descriptor,
            inherit_handle: 0,
        },
    }))
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
    fn CreateEventW(
        event_attributes: *mut c_void,
        manual_reset: i32,
        initial_state: i32,
        name: *const u16,
    ) -> *mut c_void;
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
}

#[link(name = "user32")]
unsafe extern "system" {
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

fn pipe_transfer(
    pipe: *mut c_void,
    write: bool,
    data: *mut u8,
    size: usize,
    deadline: u64,
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
                // SAFETY: Waits on the event owned by this operation.
                let wait_result = unsafe { WaitForSingleObject(event, wait) };
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
    let Some(state) = pipe_security_state(service_account != 0, session_id, &user_sid) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(state).cast::<c_void>()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_pipe_security_attributes(
    state: *mut c_void,
) -> *mut c_void {
    if state.is_null() {
        return std::ptr::null_mut();
    }
    let state = state.cast::<PipeSecurityState>();
    // SAFETY: `state` is an opaque handle returned by
    // fcitx5_windows_common_pipe_security_create_utf16 and remains owned by
    // the caller until destroy.
    unsafe { std::ptr::addr_of_mut!((*state).attributes).cast::<c_void>() }
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_pipe_security_destroy(state: *mut c_void) {
    if state.is_null() {
        return;
    }
    // SAFETY: `state` must be a handle returned by
    // fcitx5_windows_common_pipe_security_create_utf16 that has not yet been
    // destroyed. Dropping the Box releases the LocalAlloc descriptor.
    unsafe {
        drop(Box::from_raw(state.cast::<PipeSecurityState>()));
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
