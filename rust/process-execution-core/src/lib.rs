#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_void, OsString};
use std::io;
use std::mem::size_of;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
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
const ERROR_ACCESS_DENIED: Dword = 5;
const ERROR_TIMEOUT: Dword = 1460;
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: Dword = 0x0000_2000;
const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: Dword = 9;
const PROC_THREAD_ATTRIBUTE_HANDLE_LIST: usize = 0x0002_0002;

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

impl Drop for UniqueHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[derive(Clone, Copy)]
struct SendHandle(usize);

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

fn drain_pipe(read_pipe: SendHandle, max_output_bytes: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 2048];
    loop {
        let mut read = 0_u32;
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
        if bytes.len() < max_output_bytes {
            let remaining = max_output_bytes - bytes.len();
            bytes.extend_from_slice(&buffer[..remaining.min(read as usize)]);
        }
    }
    bytes
}

fn run_process(
    executable: &Path,
    arguments: &[OsString],
    timeout_ms: u32,
    max_output_bytes: usize,
) -> io::Result<(bool, Vec<u16>)> {
    if !executable.is_file() {
        return Ok((false, Vec::new()));
    }
    let mut job_limits = JobObjectExtendedLimitInformation {
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
    let job = UniqueHandle::new(unsafe { CreateJobObjectW(null_mut(), null()) })
        .ok_or_else(io::Error::last_os_error)?;
    if unsafe {
        SetInformationJobObject(
            job.get(),
            JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
            (&mut job_limits as *mut JobObjectExtendedLimitInformation).cast(),
            size_of::<JobObjectExtendedLimitInformation>() as Dword,
        )
    } == FALSE
    {
        return Err(io::Error::last_os_error());
    }

    let mut attributes = SecurityAttributes {
        n_length: size_of::<SecurityAttributes>() as Dword,
        lp_security_descriptor: null_mut(),
        b_inherit_handle: TRUE,
    };
    let mut raw_read_pipe = null_mut();
    let mut raw_write_pipe = null_mut();
    if unsafe { CreatePipe(&mut raw_read_pipe, &mut raw_write_pipe, &mut attributes, 0) } == FALSE {
        return Err(io::Error::last_os_error());
    }
    let read_pipe = UniqueHandle::new(raw_read_pipe).ok_or_else(io::Error::last_os_error)?;
    let write_pipe = UniqueHandle::new(raw_write_pipe).ok_or_else(io::Error::last_os_error)?;
    if unsafe { SetHandleInformation(read_pipe.get(), HANDLE_FLAG_INHERIT, 0) } == FALSE {
        return Err(io::Error::last_os_error());
    }

    let mut attribute_list_size = 0_usize;
    unsafe {
        let _ = InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut attribute_list_size);
    }
    let mut attribute_storage = vec![0_u8; attribute_list_size];
    let attribute_list = attribute_storage.as_mut_ptr().cast::<c_void>();
    if unsafe { InitializeProcThreadAttributeList(attribute_list, 1, 0, &mut attribute_list_size) }
        == FALSE
    {
        return Err(io::Error::last_os_error());
    }
    struct AttributeList(Lpvoid);
    impl Drop for AttributeList {
        fn drop(&mut self) {
            unsafe {
                DeleteProcThreadAttributeList(self.0);
            }
        }
    }
    let attribute_list_guard = AttributeList(attribute_list);
    let mut inherited_handles = [write_pipe.get()];
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
        return Ok((false, Vec::new()));
    }
    let process_handle =
        UniqueHandle::new(process_info.h_process).ok_or_else(io::Error::last_os_error)?;
    let thread_handle =
        UniqueHandle::new(process_info.h_thread).ok_or_else(io::Error::last_os_error)?;

    if unsafe { AssignProcessToJobObject(job.get(), process_handle.get()) } == FALSE {
        unsafe {
            let _ = TerminateProcess(process_handle.get(), ERROR_ACCESS_DENIED);
        }
        return Ok((false, Vec::new()));
    }
    if unsafe { ResumeThread(thread_handle.get()) } == u32::MAX {
        unsafe {
            let _ = TerminateJobObject(job.get(), ERROR_ACCESS_DENIED);
            let _ = WaitForSingleObject(process_handle.get(), 5000);
        }
        return Ok((false, Vec::new()));
    }

    let reader_handle = SendHandle(read_pipe.get() as usize);
    let reader = thread::spawn(move || drain_pipe(reader_handle, max_output_bytes));
    let wait = unsafe { WaitForSingleObject(process_handle.get(), timeout_ms) };
    let mut final_wait = wait;
    if wait == WAIT_TIMEOUT {
        unsafe {
            let _ = TerminateJobObject(job.get(), ERROR_TIMEOUT);
        }
        final_wait = unsafe { WaitForSingleObject(process_handle.get(), 5000) };
    }
    let bytes = reader.join().unwrap_or_default();
    let mut exit_code = 1_u32;
    unsafe {
        let _ = GetExitCodeProcess(process_handle.get(), &mut exit_code);
    }
    let output: Vec<u16> = String::from_utf8_lossy(&bytes).encode_utf16().collect();
    Ok((
        wait == WAIT_OBJECT_0 && final_wait == WAIT_OBJECT_0 && exit_code == 0,
        output,
    ))
}

fn output_to_result(success: bool, output: Vec<u16>, result: *mut Fcitx5ProcessRunResult) -> i32 {
    if result.is_null() {
        return 1;
    }
    let mut boxed = output.into_boxed_slice();
    let output_ptr = boxed.as_mut_ptr();
    let output_len = boxed.len();
    std::mem::forget(boxed);
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
            Ok((success, output)) => output_to_result(success, output, result),
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
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
