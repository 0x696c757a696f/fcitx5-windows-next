#![windows_subsystem = "windows"]
#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::OsString;
use std::path::Path;

fn version() -> &'static str {
    option_env!("FCITX_WINDOWS_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn main() {
    std::process::exit(run(std::env::args_os().collect()));
}

fn run(args: Vec<OsString>) -> i32 {
    let Some(executable) = win32::module_path() else {
        return 2;
    };
    let Some(root) = executable.parent().map(Path::to_path_buf) else {
        return 2;
    };

    if args.len() == 2 && args[1] == "--version" {
        win32::message_box_a(
            version(),
            "Fcitx5 for Windows Next",
            win32::MB_ICONINFORMATION,
        );
        return 0;
    }
    if args.len() == 2 && args[1] == "--self-test" {
        return if root.join("bin").join("fcitx5-launcher.exe").is_file()
            && root.join("bin").join("fcitx5-config.exe").is_file()
            && root
                .join("tsf")
                .join("x64")
                .join("fcitx5-tsf.dll")
                .is_file()
            && root
                .join("tsf")
                .join("x86")
                .join("fcitx5-tsf.dll")
                .is_file()
        {
            0
        } else {
            3
        };
    }
    if args.len() == 2 && args[1] == "--elevated-register" {
        if !clear_user_registration_shadows(&root) {
            return 13;
        }
        return run_registration(&root, false);
    }
    if args.len() == 2 && args[1] == "--elevated-unregister" {
        if !clear_user_registration_shadows(&root) {
            return 13;
        }
        return run_registration(&root, true);
    }
    if args.len() == 2 && args[1] == "--repair-only" {
        if !clear_user_registration_shadows(&root) {
            error_box("The stale current-user TSF registration could not be removed.");
            return 13;
        }
        let result = elevate_registration(&executable, false);
        if result != 0 {
            error_box("Administrator approval is required to repair both TSF architectures.");
            return result;
        }
        if !win32::launch_detached(
            &root.join("bin").join("fcitx5-launcher.exe"),
            "--background",
        ) {
            error_box("Registration was repaired, but the input method service could not start.");
            return 5;
        }
        win32::message_box_w(
            "Fcitx5 registration and background service were repaired successfully.",
            "Fcitx5 for Windows Next",
            win32::MB_ICONINFORMATION,
        );
        return 0;
    }

    let mut action = inferred_action(&executable);
    if args.len() == 2 && args[1] == "--settings" {
        action = Action::Settings;
    } else if args.len() == 2 && args[1] == "--unregister" {
        action = Action::Unregister;
    } else if args.len() > 1 && !(args.len() == 2 && args[1] == "--start") {
        return 2;
    }

    match action {
        Action::Settings => {
            if !win32::launch_detached(&root.join("bin").join("fcitx5-config.exe"), "") {
                error_box("The settings program is missing or could not be started.");
                return 4;
            }
            0
        }
        Action::Unregister => {
            if !clear_user_registration_shadows(&root) {
                error_box("The stale current-user TSF registration could not be removed.");
                return 13;
            }
            let _ = win32::launch_and_wait(
                &root.join("bin").join("fcitx5-control.exe"),
                "--shutdown",
                60_000,
            );
            let result = elevate_registration(&executable, true);
            if result != 0 {
                error_box("Administrator approval is required to unregister the TSF components.");
                return result;
            }
            win32::message_box_w(
                "Fcitx5 has been unregistered. The portable files can now be removed.",
                "Fcitx5 for Windows Next",
                win32::MB_ICONINFORMATION,
            );
            0
        }
        Action::Start => {
            if !clear_user_registration_shadows(&root) {
                error_box("The stale current-user TSF registration could not be removed.");
                return 13;
            }
            if !registration_healthy(&root) {
                let registration = elevate_registration(&executable, false);
                if registration != 0 {
                    error_box(
                        "Administrator approval is required once to register the TSF components.",
                    );
                    return registration;
                }
            }
            if !win32::launch_detached(
                &root.join("bin").join("fcitx5-launcher.exe"),
                "--background",
            ) {
                error_box("The input method service could not be started.");
                return 5;
            }
            win32::message_box_w(
                "Fcitx5 is running. Select it from the Windows input indicator (Win+Space).\n\nUse 'Fcitx5 Settings.exe' in this folder to configure it.",
                "Fcitx5 for Windows Next",
                win32::MB_ICONINFORMATION,
            );
            0
        }
    }
}

fn clear_user_registration_shadows(root: &Path) -> bool {
    let bin = root.join("bin");
    let helpers = [
        (
            bin.join("fcitx5-register.exe"),
            root.join("tsf").join("x64").join("fcitx5-tsf.dll"),
        ),
        (
            bin.join("fcitx5-register-x86.exe"),
            root.join("tsf").join("x86").join("fcitx5-tsf.dll"),
        ),
    ];
    helpers.into_iter().all(|(helper, dll)| {
        let arguments = format!("--remove-user-shadow --dll {}", quote(&dll));
        win32::launch_and_wait(&helper, &arguments, 60_000) == Some(0)
    })
}

fn run_registration(root: &Path, unregister: bool) -> i32 {
    let register64 = root.join("bin").join("fcitx5-register.exe");
    let register32 = root.join("bin").join("fcitx5-register-x86.exe");
    let dll64 = root.join("tsf").join("x64").join("fcitx5-tsf.dll");
    let dll32 = root.join("tsf").join("x86").join("fcitx5-tsf.dll");
    let operation = if unregister {
        "--unregister"
    } else {
        "--repair"
    };
    for (register, dll) in [(register64, dll64), (register32, dll32)] {
        if !register.is_file() || !dll.is_file() {
            return 10;
        }
        let arguments = format!("{operation} --dll {}", quote(&dll));
        let Some(exit_code) = win32::launch_and_wait(&register, &arguments, 60_000) else {
            return 11;
        };
        if exit_code != 0 {
            return 11;
        }
    }
    0
}

fn registration_healthy(root: &Path) -> bool {
    let register64 = root.join("bin").join("fcitx5-register.exe");
    let register32 = root.join("bin").join("fcitx5-register-x86.exe");
    let dll64 = root.join("tsf").join("x64").join("fcitx5-tsf.dll");
    let dll32 = root.join("tsf").join("x86").join("fcitx5-tsf.dll");
    for (register, dll) in [(register64, dll64), (register32, dll32)] {
        if !register.is_file() || !dll.is_file() {
            return false;
        }
        let arguments = format!("--status --dll {}", quote(&dll));
        if win32::launch_and_wait(&register, &arguments, 60_000) != Some(0) {
            return false;
        }
    }
    true
}

fn elevate_registration(executable: &Path, unregister: bool) -> i32 {
    let argument = if unregister {
        "--elevated-unregister"
    } else {
        "--elevated-register"
    };
    win32::shell_execute_runas_wait(executable, argument, 120_000).unwrap_or(12)
}

fn error_box(detail: &str) {
    win32::message_box_w(
        &format!("Fcitx5 for Windows Next could not complete the operation.\n\n{detail}"),
        "Fcitx5 for Windows Next",
        win32::MB_ICONERROR,
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Start,
    Settings,
    Unregister,
}

fn inferred_action(executable: &Path) -> Action {
    let name = executable
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if name.contains("Settings") {
        Action::Settings
    } else if name.contains("Unregister") {
        Action::Unregister
    } else {
        Action::Start
    }
}

fn quote(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

#[cfg(windows)]
mod win32 {
    #![deny(unsafe_op_in_unsafe_fn)]

    use std::ffi::{c_void, OsString};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    type Handle = *mut c_void;

    pub const MB_ICONERROR: u32 = 0x10;
    pub const MB_ICONINFORMATION: u32 = 0x40;
    const MB_OK: u32 = 0;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 0x0000_0102;
    const ERROR_TIMEOUT: u32 = 1460;
    const SEE_MASK_NOCLOSEPROCESS: u32 = 0x0000_0040;
    const SEE_MASK_NOASYNC: u32 = 0x0000_0100;
    const SW_HIDE: i32 = 0;

    #[repr(C)]
    struct StartupInfoW {
        cb: u32,
        reserved: *mut u16,
        desktop: *mut u16,
        title: *mut u16,
        x: u32,
        y: u32,
        x_size: u32,
        y_size: u32,
        x_count_chars: u32,
        y_count_chars: u32,
        fill_attribute: u32,
        flags: u32,
        show_window: u16,
        reserved2: u16,
        reserved2_ptr: *mut u8,
        std_input: Handle,
        std_output: Handle,
        std_error: Handle,
    }

    #[repr(C)]
    struct ProcessInformation {
        process: Handle,
        thread: Handle,
        process_id: u32,
        thread_id: u32,
    }

    #[repr(C)]
    struct ShellExecuteInfoW {
        cb_size: u32,
        mask: u32,
        hwnd: Handle,
        verb: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
        instance: Handle,
        id_list: *mut c_void,
        class: *const u16,
        key_class: Handle,
        hot_key: u32,
        icon_or_monitor: Handle,
        process: Handle,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetModuleFileNameW(module: Handle, filename: *mut u16, size: u32) -> u32;
        fn CreateProcessW(
            application_name: *const u16,
            command_line: *mut u16,
            process_attributes: Handle,
            thread_attributes: Handle,
            inherit_handles: i32,
            creation_flags: u32,
            environment: Handle,
            current_directory: *const u16,
            startup_info: *mut StartupInfoW,
            process_information: *mut ProcessInformation,
        ) -> i32;
        fn CloseHandle(object: Handle) -> i32;
        fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
        fn GetExitCodeProcess(process: Handle, exit_code: *mut u32) -> i32;
        fn TerminateProcess(process: Handle, exit_code: u32) -> i32;
        fn MessageBoxW(hwnd: Handle, text: *const u16, caption: *const u16, kind: u32) -> i32;
        fn MessageBoxA(hwnd: Handle, text: *const u8, caption: *const u8, kind: u32) -> i32;
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteExW(execute_info: *mut ShellExecuteInfoW) -> i32;
    }

    pub fn module_path() -> Option<PathBuf> {
        let mut buffer = vec![0u16; 32768];
        let length = unsafe {
            GetModuleFileNameW(
                std::ptr::null_mut(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
            )
        };
        if length == 0 || length as usize >= buffer.len() {
            return None;
        }
        buffer.truncate(length as usize);
        Some(PathBuf::from(OsString::from_wide(&buffer)))
    }

    pub fn launch_and_wait(executable: &Path, arguments: &str, timeout_ms: u32) -> Option<u32> {
        let mut command = wide_nul(&format!("{} {arguments}", quote(executable)));
        let mut startup = StartupInfoW {
            cb: std::mem::size_of::<StartupInfoW>() as u32,
            reserved: std::ptr::null_mut(),
            desktop: std::ptr::null_mut(),
            title: std::ptr::null_mut(),
            x: 0,
            y: 0,
            x_size: 0,
            y_size: 0,
            x_count_chars: 0,
            y_count_chars: 0,
            fill_attribute: 0,
            flags: 0,
            show_window: 0,
            reserved2: 0,
            reserved2_ptr: std::ptr::null_mut(),
            std_input: std::ptr::null_mut(),
            std_output: std::ptr::null_mut(),
            std_error: std::ptr::null_mut(),
        };
        let mut process = ProcessInformation {
            process: std::ptr::null_mut(),
            thread: std::ptr::null_mut(),
            process_id: 0,
            thread_id: 0,
        };
        let created = unsafe {
            CreateProcessW(
                wide_path(executable).as_ptr(),
                command.as_mut_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                CREATE_NO_WINDOW,
                std::ptr::null_mut(),
                std::ptr::null(),
                &mut startup,
                &mut process,
            )
        };
        if created == 0 {
            return None;
        }
        unsafe {
            let _ = CloseHandle(process.thread);
        }
        let wait = unsafe { WaitForSingleObject(process.process, timeout_ms) };
        let mut exit_code = 0;
        let success = wait == WAIT_OBJECT_0
            && unsafe { GetExitCodeProcess(process.process, &mut exit_code) } != 0;
        if wait == WAIT_TIMEOUT {
            unsafe {
                let _ = TerminateProcess(process.process, ERROR_TIMEOUT);
                let _ = WaitForSingleObject(process.process, 5000);
            }
        }
        unsafe {
            let _ = CloseHandle(process.process);
        }
        success.then_some(exit_code)
    }

    pub fn launch_detached(executable: &Path, arguments: &str) -> bool {
        let mut command = wide_nul(&format!("{} {arguments}", quote(executable)));
        let mut startup = StartupInfoW {
            cb: std::mem::size_of::<StartupInfoW>() as u32,
            reserved: std::ptr::null_mut(),
            desktop: std::ptr::null_mut(),
            title: std::ptr::null_mut(),
            x: 0,
            y: 0,
            x_size: 0,
            y_size: 0,
            x_count_chars: 0,
            y_count_chars: 0,
            fill_attribute: 0,
            flags: 0,
            show_window: 0,
            reserved2: 0,
            reserved2_ptr: std::ptr::null_mut(),
            std_input: std::ptr::null_mut(),
            std_output: std::ptr::null_mut(),
            std_error: std::ptr::null_mut(),
        };
        let mut process = ProcessInformation {
            process: std::ptr::null_mut(),
            thread: std::ptr::null_mut(),
            process_id: 0,
            thread_id: 0,
        };
        let created = unsafe {
            CreateProcessW(
                wide_path(executable).as_ptr(),
                command.as_mut_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                CREATE_NEW_PROCESS_GROUP,
                std::ptr::null_mut(),
                std::ptr::null(),
                &mut startup,
                &mut process,
            )
        };
        if created == 0 {
            return false;
        }
        unsafe {
            let _ = CloseHandle(process.thread);
            let _ = CloseHandle(process.process);
        }
        true
    }

    pub fn shell_execute_runas_wait(
        executable: &Path,
        argument: &str,
        timeout_ms: u32,
    ) -> Option<i32> {
        let verb = wide_nul("runas");
        let file = wide_path_nul(executable);
        let parameters = wide_nul(argument);
        let mut info = ShellExecuteInfoW {
            cb_size: std::mem::size_of::<ShellExecuteInfoW>() as u32,
            mask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            hwnd: std::ptr::null_mut(),
            verb: verb.as_ptr(),
            file: file.as_ptr(),
            parameters: parameters.as_ptr(),
            directory: std::ptr::null(),
            show: SW_HIDE,
            instance: std::ptr::null_mut(),
            id_list: std::ptr::null_mut(),
            class: std::ptr::null(),
            key_class: std::ptr::null_mut(),
            hot_key: 0,
            icon_or_monitor: std::ptr::null_mut(),
            process: std::ptr::null_mut(),
        };
        if unsafe { ShellExecuteExW(&mut info) } == 0 || info.process.is_null() {
            return None;
        }
        let wait = unsafe { WaitForSingleObject(info.process, timeout_ms) };
        let mut exit_code = 13;
        if wait == WAIT_OBJECT_0 {
            let _ = unsafe { GetExitCodeProcess(info.process, &mut exit_code) };
        } else if wait == WAIT_TIMEOUT {
            unsafe {
                let _ = TerminateProcess(info.process, ERROR_TIMEOUT);
                let _ = WaitForSingleObject(info.process, 5000);
            }
            exit_code = ERROR_TIMEOUT;
        }
        unsafe {
            let _ = CloseHandle(info.process);
        }
        Some(exit_code as i32)
    }

    pub fn message_box_w(text: &str, caption: &str, icon: u32) {
        let text = wide_nul(text);
        let caption = wide_nul(caption);
        unsafe {
            let _ = MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                caption.as_ptr(),
                MB_OK | icon,
            );
        }
    }

    pub fn message_box_a(text: &str, caption: &str, icon: u32) {
        let text = nul_bytes(text);
        let caption = nul_bytes(caption);
        unsafe {
            let _ = MessageBoxA(
                std::ptr::null_mut(),
                text.as_ptr(),
                caption.as_ptr(),
                MB_OK | icon,
            );
        }
    }

    fn quote(path: &Path) -> String {
        format!("\"{}\"", path.display())
    }

    fn wide_path(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain([0]).collect()
    }

    fn wide_path_nul(path: &Path) -> Vec<u16> {
        wide_path(path)
    }

    fn wide_nul(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }

    fn nul_bytes(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .iter()
            .copied()
            .filter(|byte| *byte != 0)
            .chain([0])
            .collect()
    }
}
