#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_void, OsString};
use std::io;
use std::mem::size_of;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::thread;

type Bool = i32;
type Dword = u32;
type Handle = *mut c_void;
type Lpvoid = *mut c_void;

const FALSE: Bool = 0;
const TRUE: Bool = 1;
const INVALID_HANDLE_VALUE: Handle = !0_usize as Handle;
const HANDLE_FLAG_INHERIT: Dword = 0x0000_0001;
const STARTF_USESTDHANDLES: Dword = 0x0000_0100;
const CREATE_NO_WINDOW: Dword = 0x0800_0000;
const CREATE_SUSPENDED: Dword = 0x0000_0004;
const EXTENDED_STARTUPINFO_PRESENT: Dword = 0x0008_0000;
const WAIT_OBJECT_0: Dword = 0x0000_0000;
const WAIT_TIMEOUT: Dword = 0x0000_0102;
#[cfg(test)]
const ERROR_INVALID_PARAMETER: Dword = 87;
const ERROR_ACCESS_DENIED: Dword = 5;
const ERROR_TIMEOUT: Dword = 1460;
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: Dword = 0x0000_2000;
const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: Dword = 9;
const PROC_THREAD_ATTRIBUTE_HANDLE_LIST: usize = 0x0002_0002;
#[cfg(test)]
const SYNCHRONIZE: Dword = 0x0010_0000;

#[repr(C)]
pub struct Fcitx5ProcessUtf16 {
    ptr: *const u16,
    len: usize,
}

#[repr(C)]
pub struct Fcitx5ProcessRunResult {
    success: u8,
    reserved: [u8; 7],
    output_ptr: *mut u16,
    output_len: usize,
}

#[repr(C)]
struct SecurityAttributes {
    n_length: Dword,
    lp_security_descriptor: Lpvoid,
    b_inherit_handle: Bool,
}

#[repr(C)]
struct StartupInfoW {
    cb: Dword,
    lp_reserved: *mut u16,
    lp_desktop: *mut u16,
    lp_title: *mut u16,
    dw_x: Dword,
    dw_y: Dword,
    dw_x_size: Dword,
    dw_y_size: Dword,
    dw_x_count_chars: Dword,
    dw_y_count_chars: Dword,
    dw_fill_attribute: Dword,
    dw_flags: Dword,
    w_show_window: u16,
    cb_reserved2: u16,
    lp_reserved2: *mut u8,
    h_std_input: Handle,
    h_std_output: Handle,
    h_std_error: Handle,
}

#[repr(C)]
struct StartupInfoExW {
    startup_info: StartupInfoW,
    lp_attribute_list: Lpvoid,
}

#[repr(C)]
struct ProcessInformation {
    h_process: Handle,
    h_thread: Handle,
    dw_process_id: Dword,
    dw_thread_id: Dword,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct JobObjectBasicLimitInformation {
    per_process_user_time_limit: i64,
    per_job_user_time_limit: i64,
    limit_flags: Dword,
    minimum_working_set_size: usize,
    maximum_working_set_size: usize,
    active_process_limit: Dword,
    affinity: usize,
    priority_class: Dword,
    scheduling_class: Dword,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct IoCounters {
    read_operation_count: u64,
    write_operation_count: u64,
    other_operation_count: u64,
    read_transfer_count: u64,
    write_transfer_count: u64,
    other_transfer_count: u64,
}

#[repr(C)]
struct JobObjectExtendedLimitInformation {
    basic_limit_information: JobObjectBasicLimitInformation,
    io_info: IoCounters,
    process_memory_limit: usize,
    job_memory_limit: usize,
    peak_process_memory_used: usize,
    peak_job_memory_used: usize,
}

// This is the narrow Win32 ABI boundary used by the process-execution owner.
unsafe extern "system" {
    fn CloseHandle(handle: Handle) -> Bool;
    fn CreateJobObjectW(attributes: *mut SecurityAttributes, name: *const u16) -> Handle;
    fn SetInformationJobObject(
        job: Handle,
        info_class: Dword,
        info: Lpvoid,
        info_len: Dword,
    ) -> Bool;
    fn CreatePipe(
        read_pipe: *mut Handle,
        write_pipe: *mut Handle,
        attributes: *mut SecurityAttributes,
        size: Dword,
    ) -> Bool;
    fn SetHandleInformation(handle: Handle, mask: Dword, flags: Dword) -> Bool;
    fn InitializeProcThreadAttributeList(
        list: Lpvoid,
        attribute_count: Dword,
        flags: Dword,
        size: *mut usize,
    ) -> Bool;
    fn UpdateProcThreadAttribute(
        list: Lpvoid,
        flags: Dword,
        attribute: usize,
        value: Lpvoid,
        size: usize,
        previous_value: Lpvoid,
        return_size: *mut usize,
    ) -> Bool;
    fn DeleteProcThreadAttributeList(list: Lpvoid);
    fn CreateProcessW(
        application_name: *const u16,
        command_line: *mut u16,
        process_attributes: *mut SecurityAttributes,
        thread_attributes: *mut SecurityAttributes,
        inherit_handles: Bool,
        creation_flags: Dword,
        environment: Lpvoid,
        current_directory: *const u16,
        startup_info: *mut StartupInfoW,
        process_information: *mut ProcessInformation,
    ) -> Bool;
    fn AssignProcessToJobObject(job: Handle, process: Handle) -> Bool;
    #[cfg(test)]
    fn OpenProcess(desired_access: Dword, inherit_handle: Bool, process_id: Dword) -> Handle;
    fn ResumeThread(thread: Handle) -> Dword;
    fn TerminateProcess(process: Handle, exit_code: Dword) -> Bool;
    fn TerminateJobObject(job: Handle, exit_code: Dword) -> Bool;
    fn WaitForSingleObject(handle: Handle, milliseconds: Dword) -> Dword;
    fn ReadFile(
        file: Handle,
        buffer: *mut c_void,
        bytes_to_read: Dword,
        bytes_read: *mut Dword,
        overlapped: Lpvoid,
    ) -> Bool;
    fn GetExitCodeProcess(process: Handle, exit_code: *mut Dword) -> Bool;
}

/// An owning Windows Job Object configured to terminate its assigned process
/// tree when the last owner closes it.
///
/// The standard [`OwnedHandle`] makes this owner [`Send`] and [`Sync`]. It has
/// no mutable Rust state, and Windows synchronizes access to the underlying
/// kernel object.
#[must_use = "dropping the Job Object terminates its assigned process tree"]
#[derive(Debug)]
pub struct JobObject {
    handle: OwnedHandle,
}

impl JobObject {
    /// Creates a Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
    ///
    /// The Job Object is never returned unless configuration succeeds.
    ///
    /// # Errors
    ///
    /// Returns the Windows error from creating or configuring the Job Object.
    pub fn new_kill_on_close() -> io::Result<Self> {
        // SAFETY: Null security attributes and name request a new unnamed Job
        // Object. The call returns a handle owned exclusively by this function.
        let raw_handle = unsafe { CreateJobObjectW(null_mut(), null()) };
        if raw_handle.is_null() || raw_handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: CreateJobObjectW returned a non-null, non-invalid handle that
        // this function exclusively owns, so OwnedHandle may close it exactly once.
        let handle = unsafe { OwnedHandle::from_raw_handle(raw_handle) };
        let job = Self { handle };
        let mut limits = JobObjectExtendedLimitInformation {
            basic_limit_information: JobObjectBasicLimitInformation {
                per_process_user_time_limit: 0,
                per_job_user_time_limit: 0,
                limit_flags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                minimum_working_set_size: 0,
                maximum_working_set_size: 0,
                active_process_limit: 0,
                affinity: 0,
                priority_class: 0,
                scheduling_class: 0,
            },
            io_info: IoCounters {
                read_operation_count: 0,
                write_operation_count: 0,
                other_operation_count: 0,
                read_transfer_count: 0,
                write_transfer_count: 0,
                other_transfer_count: 0,
            },
            process_memory_limit: 0,
            job_memory_limit: 0,
            peak_process_memory_used: 0,
            peak_job_memory_used: 0,
        };
        // SAFETY: job owns a valid Job Object handle. limits is a correctly
        // sized, initialized JOB_OBJECT_EXTENDED_LIMIT_INFORMATION for the call.
        let configured = unsafe {
            SetInformationJobObject(
                job.handle.as_raw_handle(),
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                (&mut limits as *mut JobObjectExtendedLimitInformation).cast(),
                size_of::<JobObjectExtendedLimitInformation>() as Dword,
            )
        };
        if configured == FALSE {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }

    /// Assigns a borrowed process handle to this Job Object.
    ///
    /// The caller retains ownership of `process` whether assignment succeeds or
    /// fails. A successful assignment causes Windows to terminate the process
    /// tree when this owner is dropped.
    ///
    /// # Errors
    ///
    /// Returns the Windows error when `process` cannot be assigned. The
    /// borrowed handle is never closed by this method.
    pub fn assign_process(&self, process: BorrowedHandle<'_>) -> io::Result<()> {
        // SAFETY: self owns a valid Job Object and process is a valid borrowed
        // handle for the duration of this call. The API only observes both handles.
        if unsafe { AssignProcessToJobObject(self.handle.as_raw_handle(), process.as_raw_handle()) }
            == FALSE
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    fn raw_handle(&self) -> Handle {
        self.handle.as_raw_handle()
    }
}

struct UniqueHandle(Handle);

impl UniqueHandle {
    fn new(handle: Handle) -> Option<Self> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(handle))
        }
    }

    fn get(&self) -> Handle {
        self.0
    }
}

impl AsHandle for UniqueHandle {
    fn as_handle(&self) -> BorrowedHandle<'_> {
        // SAFETY: UniqueHandle is constructed only from valid handles and owns
        // the handle for the returned borrow's lifetime.
        unsafe { BorrowedHandle::borrow_raw(self.0) }
    }
}

impl Drop for UniqueHandle {
    fn drop(&mut self) {
        // SAFETY: UniqueHandle owns a valid handle and Drop runs once, so this
        // is the sole CloseHandle call for it.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[derive(Clone, Copy)]
struct SendHandle(usize);

// SAFETY: SendHandle is only created from the read pipe held by run_process,
// which remains open until the reader thread joins. The handle has no Rust aliasing state.
unsafe impl Send for SendHandle {}

impl SendHandle {
    fn get(self) -> Handle {
        self.0 as Handle
    }
}

fn wide_from_slice(slice: Fcitx5ProcessUtf16) -> Option<OsString> {
    if slice.ptr.is_null() {
        return None;
    }
    // SAFETY: the FFI entry point documents that ptr is valid for len UTF-16
    // values for this call, and the slice is only read before that call returns.
    let value = unsafe { std::slice::from_raw_parts(slice.ptr, slice.len) };
    Some(OsString::from_wide(value))
}

fn wide_z(value: &std::ffi::OsStr) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_wide().collect();
    wide.push(0);
    wide
}

fn quote(value: &std::ffi::OsStr) -> OsString {
    let wide: Vec<u16> = value.encode_wide().collect();
    let mut result: Vec<u16> = Vec::with_capacity(wide.len() + 2);
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

fn command_line(executable: &Path, arguments: &[OsString]) -> Vec<u16> {
    let mut command = quote(executable.as_os_str());
    for argument in arguments {
        command.push(" ");
        command.push(quote(argument));
    }
    wide_z(&command)
}

fn drain_pipe(read_pipe: SendHandle, max_output_bytes: usize) -> (Vec<u8>, bool) {
    let mut bytes = Vec::new();
    let mut truncated = false;
    let mut buffer = [0_u8; 2048];
    loop {
        let mut read = 0_u32;
        // SAFETY: read_pipe remains open until this drain finishes, and buffer
        // and read are writable for the sizes passed to ReadFile.
        let ok = unsafe {
            ReadFile(
                read_pipe.get(),
                buffer.as_mut_ptr().cast(),
                buffer.len() as Dword,
                &mut read,
                null_mut(),
            )
        };
        if ok == FALSE || read == 0 {
            break;
        }
        let remaining = max_output_bytes.saturating_sub(bytes.len());
        if remaining != 0 {
            bytes.extend_from_slice(&buffer[..remaining.min(read as usize)]);
        }
        truncated |= read as usize > remaining;
    }
    (bytes, truncated)
}

fn run_process(
    executable: &Path,
    arguments: &[OsString],
    timeout_ms: u32,
    max_output_bytes: usize,
) -> io::Result<(bool, Vec<u16>, bool)> {
    if !executable.is_file() {
        return Ok((false, Vec::new(), false));
    }
    let job = JobObject::new_kill_on_close()?;

    let mut attributes = SecurityAttributes {
        n_length: size_of::<SecurityAttributes>() as Dword,
        lp_security_descriptor: null_mut(),
        b_inherit_handle: TRUE,
    };
    let mut raw_read_pipe = null_mut();
    let mut raw_write_pipe = null_mut();
    // SAFETY: attributes, raw_read_pipe, and raw_write_pipe are initialized
    // writable storage for CreatePipe, which creates two owned handles on success.
    if unsafe { CreatePipe(&mut raw_read_pipe, &mut raw_write_pipe, &mut attributes, 0) } == FALSE {
        return Err(io::Error::last_os_error());
    }
    let read_pipe = UniqueHandle::new(raw_read_pipe).ok_or_else(io::Error::last_os_error)?;
    let write_pipe = UniqueHandle::new(raw_write_pipe).ok_or_else(io::Error::last_os_error)?;
    // SAFETY: read_pipe owns a valid pipe handle, and the call only changes its
    // inherit flag before the child process can inherit it.
    if unsafe { SetHandleInformation(read_pipe.get(), HANDLE_FLAG_INHERIT, 0) } == FALSE {
        return Err(io::Error::last_os_error());
    }

    let mut attribute_list_size = 0_usize;
    // SAFETY: A null list deliberately asks Windows for the required allocation
    // size. The initialized size pointer is writable for the duration of the call.
    unsafe {
        let _ = InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut attribute_list_size);
    }
    let mut attribute_storage = vec![0_u8; attribute_list_size];
    let attribute_list = attribute_storage.as_mut_ptr().cast::<c_void>();
    // SAFETY: attribute_storage provides the size Windows requested, and its
    // address remains stable until AttributeList deletes the initialized list.
    if unsafe { InitializeProcThreadAttributeList(attribute_list, 1, 0, &mut attribute_list_size) }
        == FALSE
    {
        return Err(io::Error::last_os_error());
    }
    struct AttributeList(Lpvoid);
    impl Drop for AttributeList {
        fn drop(&mut self) {
            // SAFETY: AttributeList is created only after successful
            // InitializeProcThreadAttributeList and deletes the list once.
            unsafe {
                DeleteProcThreadAttributeList(self.0);
            }
        }
    }
    let attribute_list_guard = AttributeList(attribute_list);
    let mut inherited_handles = [write_pipe.get()];
    // SAFETY: attribute_list_guard owns an initialized attribute list, and
    // inherited_handles stays live until CreateProcessW consumes the list.
    if unsafe {
        UpdateProcThreadAttribute(
            attribute_list_guard.0,
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            inherited_handles.as_mut_ptr().cast(),
            size_of::<Handle>(),
            null_mut(),
            null_mut(),
        )
    } == FALSE
    {
        return Err(io::Error::last_os_error());
    }

    let mut startup = StartupInfoExW {
        startup_info: StartupInfoW {
            cb: size_of::<StartupInfoExW>() as Dword,
            lp_reserved: null_mut(),
            lp_desktop: null_mut(),
            lp_title: null_mut(),
            dw_x: 0,
            dw_y: 0,
            dw_x_size: 0,
            dw_y_size: 0,
            dw_x_count_chars: 0,
            dw_y_count_chars: 0,
            dw_fill_attribute: 0,
            dw_flags: STARTF_USESTDHANDLES,
            w_show_window: 0,
            cb_reserved2: 0,
            lp_reserved2: null_mut(),
            h_std_input: null_mut(),
            h_std_output: write_pipe.get(),
            h_std_error: write_pipe.get(),
        },
        lp_attribute_list: attribute_list_guard.0,
    };
    let mut process_info = ProcessInformation {
        h_process: null_mut(),
        h_thread: null_mut(),
        dw_process_id: 0,
        dw_thread_id: 0,
    };
    let executable_wide = wide_z(executable.as_os_str());
    let mut command = command_line(executable, arguments);
    let current_directory = executable.parent().unwrap_or_else(|| Path::new(""));
    let current_directory_wide = wide_z(current_directory.as_os_str());
    // SAFETY: all UTF-16 buffers and startup/process structures remain valid and
    // writable through the call; only write_pipe is listed for inheritance.
    let created = unsafe {
        CreateProcessW(
            executable_wide.as_ptr(),
            command.as_mut_ptr(),
            null_mut(),
            null_mut(),
            TRUE,
            CREATE_NO_WINDOW | CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT,
            null_mut(),
            current_directory_wide.as_ptr(),
            (&mut startup.startup_info as *mut StartupInfoW).cast(),
            &mut process_info,
        )
    };
    drop(attribute_list_guard);
    drop(write_pipe);
    if created == FALSE {
        return Ok((false, Vec::new(), false));
    }
    let process_handle =
        UniqueHandle::new(process_info.h_process).ok_or_else(io::Error::last_os_error)?;
    let thread_handle =
        UniqueHandle::new(process_info.h_thread).ok_or_else(io::Error::last_os_error)?;

    if job.assign_process(process_handle.as_handle()).is_err() {
        // SAFETY: process_handle owns the suspended child process. Terminating it
        // prevents the unassigned process from running after assignment failure.
        unsafe {
            let _ = TerminateProcess(process_handle.get(), ERROR_ACCESS_DENIED);
        }
        return Ok((false, Vec::new(), false));
    }
    // SAFETY: thread_handle owns the primary thread returned by CreateProcessW.
    if unsafe { ResumeThread(thread_handle.get()) } == u32::MAX {
        // SAFETY: job owns the configured Job Object and process_handle remains
        // valid until the bounded wait completes.
        unsafe {
            let _ = TerminateJobObject(job.raw_handle(), ERROR_ACCESS_DENIED);
            let _ = WaitForSingleObject(process_handle.get(), 5000);
        }
        return Ok((false, Vec::new(), false));
    }

    let reader_handle = SendHandle(read_pipe.get() as usize);
    let reader = thread::spawn(move || drain_pipe(reader_handle, max_output_bytes));
    // SAFETY: process_handle owns a valid process handle for this synchronous wait.
    let wait = unsafe { WaitForSingleObject(process_handle.get(), timeout_ms) };
    let mut final_wait = wait;
    if wait == WAIT_TIMEOUT {
        // SAFETY: job owns the configured Job Object; terminating it also
        // terminates descendants before the final process wait.
        unsafe {
            let _ = TerminateJobObject(job.raw_handle(), ERROR_TIMEOUT);
        }
        // SAFETY: process_handle remains valid for this final bounded wait.
        final_wait = unsafe { WaitForSingleObject(process_handle.get(), 5000) };
    }
    let (bytes, truncated) = reader.join().unwrap_or_default();
    let mut exit_code = 1_u32;
    // SAFETY: process_handle owns a valid process handle and exit_code is valid
    // writable storage for GetExitCodeProcess.
    unsafe {
        let _ = GetExitCodeProcess(process_handle.get(), &mut exit_code);
    }
    let output: Vec<u16> = String::from_utf8_lossy(&bytes).encode_utf16().collect();
    Ok((
        wait == WAIT_OBJECT_0 && final_wait == WAIT_OBJECT_0 && exit_code == 0,
        output,
        truncated,
    ))
}

#[derive(Debug)]
pub struct ProcessOutput {
    pub success: bool,
    pub output: String,
}

/// Runs a fixed executable with bounded combined stdout/stderr capture.
///
/// # Errors
///
/// Returns an I/O error when process setup fails or the captured output exceeds
/// `max_output_bytes`.
pub fn run_process_bounded(
    executable: &Path,
    arguments: &[OsString],
    timeout_ms: u32,
    max_output_bytes: usize,
) -> io::Result<ProcessOutput> {
    let (success, output, truncated) =
        run_process(executable, arguments, timeout_ms, max_output_bytes)?;
    if truncated {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "process output exceeded limit",
        ));
    }
    Ok(ProcessOutput {
        success,
        output: String::from_utf16_lossy(&output),
    })
}

fn output_to_result(success: bool, output: Vec<u16>, result: *mut Fcitx5ProcessRunResult) -> i32 {
    if result.is_null() {
        return 1;
    }
    let mut boxed = output.into_boxed_slice();
    let output_ptr = boxed.as_mut_ptr();
    let output_len = boxed.len();
    std::mem::forget(boxed);
    // SAFETY: the FFI contract requires result to be writable. The leaked boxed
    // slice is returned intact and is later reclaimed only by the paired free API.
    unsafe {
        *result = Fcitx5ProcessRunResult {
            success: u8::from(success),
            reserved: [0; 7],
            output_ptr,
            output_len,
        };
    }
    0
}

/// # Safety
///
/// All UTF-16 slices must remain valid for the duration of the call. `arguments`
/// must reference `argument_count` slices when `argument_count` is non-zero.
/// `result` must point to writable storage. Any returned `output_ptr` must be
/// released with `fcitx5_process_output_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_process_run_utf16(
    executable: Fcitx5ProcessUtf16,
    arguments: *const Fcitx5ProcessUtf16,
    argument_count: usize,
    timeout_ms: u32,
    max_output_bytes: usize,
    result: *mut Fcitx5ProcessRunResult,
) -> i32 {
    let run = std::panic::catch_unwind(|| {
        let Some(executable) = wide_from_slice(executable) else {
            return output_to_result(false, Vec::new(), result);
        };
        let argument_slices = if argument_count == 0 {
            &[][..]
        } else if arguments.is_null() {
            return output_to_result(false, Vec::new(), result);
        } else {
            // SAFETY: the FFI contract guarantees argument_count valid
            // Fcitx5ProcessUtf16 values at arguments for this call.
            unsafe { std::slice::from_raw_parts(arguments, argument_count) }
        };
        let mut parsed_arguments = Vec::with_capacity(argument_slices.len());
        for argument in argument_slices {
            let Some(argument) = wide_from_slice(Fcitx5ProcessUtf16 {
                ptr: argument.ptr,
                len: argument.len,
            }) else {
                return output_to_result(false, Vec::new(), result);
            };
            parsed_arguments.push(argument);
        }
        match run_process(
            &PathBuf::from(executable),
            &parsed_arguments,
            timeout_ms,
            max_output_bytes,
        ) {
            Ok((success, output, _)) => output_to_result(success, output, result),
            Err(_) => output_to_result(false, Vec::new(), result),
        }
    });
    run.unwrap_or(1)
}

/// # Safety
///
/// `ptr` and `len` must be the exact buffer returned by
/// `fcitx5_process_run_utf16`, or `ptr` must be null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_process_output_free(ptr: *mut u16, len: usize) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: the FFI contract guarantees ptr and len came from the paired run
    // API, so this reconstructs the exact allocation once for Drop.
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::os::windows::io::{AsHandle, AsRawHandle};
    use std::process::Command;

    fn wide_string(value: &str) -> Vec<u16> {
        OsString::from(value).encode_wide().collect()
    }

    fn quote_wide(value: &str) -> Vec<u16> {
        quote(&OsString::from(value)).encode_wide().collect()
    }

    #[test]
    fn quote_matches_windows_argument_rules() {
        assert_eq!(wide_string("\"simple\""), quote_wide("simple"));
        assert_eq!(wide_string("\"a b\\\\\\\\\""), quote_wide(r"a b\\"));
        assert_eq!(wide_string("\"say \\\"hi\\\"\""), quote_wide("say \"hi\""));
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn job_object_is_send_and_sync() {
        assert_send_sync::<JobObject>();
    }

    fn wait_for_handle(handle: BorrowedHandle<'_>, timeout_ms: Dword) -> Dword {
        // SAFETY: handle is a valid borrow for this synchronous wait call.
        unsafe { WaitForSingleObject(handle.as_raw_handle(), timeout_ms) }
    }

    fn open_process_for_wait(process_id: Dword) -> io::Result<Option<UniqueHandle>> {
        // SAFETY: process_id was reported by the fixture process. The returned
        // handle, when any, is immediately owned by UniqueHandle.
        let handle = unsafe { OpenProcess(SYNCHRONIZE, FALSE, process_id) };
        if let Some(handle) = UniqueHandle::new(handle) {
            return Ok(Some(handle));
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            return Ok(None);
        }
        Err(error)
    }

    fn assert_process_exited(process_id: Dword, message: &str) {
        if let Some(process) = open_process_for_wait(process_id).expect("open fixture process") {
            assert_eq!(
                wait_for_handle(process.as_handle(), 5_000),
                WAIT_OBJECT_0,
                "{message}"
            );
        }
    }

    fn spawn_fixture(role: &str) -> (std::process::Child, BufReader<std::process::ChildStdout>) {
        let mut child = Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", "tests::fixture_process_entry", "--nocapture"])
            .env("FCITX_PROCESS_FIXTURE_ROLE", role)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("start Rust process fixture");
        let output = BufReader::new(child.stdout.take().expect("fixture stdout"));
        (child, output)
    }

    fn fixture_ready(reader: &mut BufReader<std::process::ChildStdout>) -> (Dword, u16) {
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).expect("fixture readiness");
            if line.starts_with("READY ") {
                break;
            }
        }
        let mut fields = line.split_whitespace();
        assert_eq!(fields.next(), Some("READY"), "fixture readiness line");
        let pid = fields
            .next()
            .expect("fixture child pid")
            .parse()
            .expect("numeric fixture pid");
        let port = fields
            .next()
            .expect("fixture port")
            .parse()
            .expect("numeric fixture port");
        (pid, port)
    }

    fn release_fixture(port: u16) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("fixture barrier");
        stream.write_all(&[1]).expect("fixture release");
    }

    fn assert_fixture_tree_reaches_readiness() -> Dword {
        let (mut root, mut output) = spawn_fixture("root");
        let (child_pid, port) = fixture_ready(&mut output);
        release_fixture(port);
        assert_eq!(root.wait().expect("fixture root exit").code(), Some(0));
        assert_process_exited(child_pid, "fixture child did not exit after release");
        child_pid
    }

    #[test]
    #[forbid(unsafe_code)]
    fn fixture_process_entry() {
        let Some(role) = std::env::var_os("FCITX_PROCESS_FIXTURE_ROLE") else {
            return;
        };
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("fixture listener");
        let port = listener.local_addr().expect("fixture address").port();
        if role == "single" {
            println!("READY 0 {port}");
            std::io::stdout().flush().expect("fixture readiness flush");
            let _ = listener.accept().expect("fixture release connection");
            return;
        }
        if role == "child" {
            let root_port: u16 = std::env::var("FCITX_PROCESS_FIXTURE_PORT")
                .expect("fixture root port")
                .parse()
                .expect("numeric fixture root port");
            let mut stream = TcpStream::connect(("127.0.0.1", root_port)).expect("fixture root");
            writeln!(stream, "{}", std::process::id()).expect("fixture child pid");
            let mut release = [0_u8; 1];
            let _ = std::io::Read::read(&mut stream, &mut release);
            return;
        }
        let child = Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", "tests::fixture_process_entry", "--nocapture"])
            .env("FCITX_PROCESS_FIXTURE_ROLE", "child")
            .env("FCITX_PROCESS_FIXTURE_PORT", port.to_string())
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("start Rust fixture child");
        let (mut child_stream, _) = listener.accept().expect("fixture child connection");
        let mut child_pid = String::new();
        BufReader::new(&mut child_stream)
            .read_line(&mut child_pid)
            .expect("fixture child readiness");
        let child_pid = child_pid.trim();
        println!("READY {child_pid} {port}");
        std::io::stdout().flush().expect("fixture readiness flush");
        let (mut release_stream, _) = listener.accept().expect("fixture release connection");
        let mut release = [0_u8; 1];
        let _ = std::io::Read::read(&mut release_stream, &mut release);
        child_stream.write_all(&[1]).expect("fixture child release");
        let mut child = child;
        assert_eq!(child.wait().expect("fixture child exit").code(), Some(0));
    }

    #[test]
    fn job_object_creation_configures_kill_on_close() {
        let (mut child, mut output) = spawn_fixture("single");
        let _ = fixture_ready(&mut output);

        {
            let job = JobObject::new_kill_on_close().expect("configured job object");
            job.assign_process(child.as_handle())
                .expect("assign fixture process");
        }

        let wait = wait_for_handle(child.as_handle(), 5_000);
        if wait != WAIT_OBJECT_0 {
            let _ = child.kill();
            panic!("dropping the job object did not terminate its process");
        }
        let _ = child.wait().expect("fixture exit status");
    }

    #[test]
    fn failed_assignment_does_not_take_the_borrowed_handle() {
        let job = JobObject::new_kill_on_close().expect("configured job object");
        let file = File::open(std::env::current_exe().expect("current test executable"))
            .expect("open non-process handle");

        assert!(job.assign_process(file.as_handle()).is_err());
        assert!(file.metadata().is_ok());
    }

    fn powershell() -> PathBuf {
        let system_root = std::env::var_os("SystemRoot").expect("SystemRoot");
        PathBuf::from(system_root).join("System32/WindowsPowerShell/v1.0/powershell.exe")
    }

    fn run_powershell(command: &str, timeout_ms: u32, max_output_bytes: usize) -> (bool, String) {
        let (ok, output, _) = run_process(
            &powershell(),
            &[
                OsString::from("-NoProfile"),
                OsString::from("-Command"),
                OsString::from(command),
            ],
            timeout_ms,
            max_output_bytes,
        )
        .expect("powershell run");
        (ok, String::from_utf16_lossy(&output))
    }

    #[test]
    fn process_output_is_drained_bounded_and_failure_visible() {
        let (ok, output) = run_powershell("Write-Output hello", 30_000, 2 * 1024 * 1024);
        assert!(ok);
        assert!(output.contains("hello"));

        let (ok, output) = run_powershell(
            "1..2000 | ForEach-Object { 'x' * 60 }",
            30_000,
            2 * 1024 * 1024,
        );
        assert!(ok);
        assert!(output.len() >= 64 * 1024);

        let (ok, output) = run_powershell("1..4000 | ForEach-Object { 'y' * 60 }", 30_000, 4096);
        assert!(ok);
        assert!(output.len() <= 4096);

        let (ok, output) = run_powershell("Write-Output nope; exit 7", 30_000, 2 * 1024 * 1024);
        assert!(!ok);
        assert!(output.contains("nope"));

        let (ok, output) = run_powershell(
            "$s=[Console]::OpenStandardOutput();$b=[byte[]](0xff,0xfe,0x61);$s.Write($b,0,$b.Length)",
            30_000,
            2 * 1024 * 1024,
        );
        assert!(ok);
        assert!(!output.is_empty());
    }

    #[test]
    fn safe_api_rejects_oversized_output() {
        let result = run_process_bounded(
            &powershell(),
            &[
                OsString::from("-NoProfile"),
                OsString::from("-Command"),
                OsString::from("'x' * 100"),
            ],
            30_000,
            16,
        );
        assert_eq!(
            result.expect_err("oversized output should fail").kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn rust_fixture_root_reaches_child_readiness_without_run_process_timeout() {
        assert_fixture_tree_reaches_readiness();
    }

    #[test]
    fn timeout_kills_child_process_tree() {
        let fixture = std::env::current_exe().expect("test executable");
        std::env::set_var("FCITX_PROCESS_FIXTURE_ROLE", "root");
        let (ok, output, _) = run_process(
            &fixture,
            &[
                OsString::from("--exact"),
                OsString::from("tests::fixture_process_entry"),
                OsString::from("--nocapture"),
            ],
            5_000,
            2 * 1024 * 1024,
        )
        .expect("Rust fixture run");
        std::env::remove_var("FCITX_PROCESS_FIXTURE_ROLE");
        let output = String::from_utf16_lossy(&output);
        let child_process_id = output
            .lines()
            .find_map(|line| line.strip_prefix("READY "))
            .and_then(|line| line.split_whitespace().next())
            .expect("fixture child readiness")
            .parse()
            .expect("numeric fixture child pid");

        assert!(!ok);
        assert_process_exited(
            child_process_id,
            "timed-out Job Object left its child fixture process running",
        );
    }
}
