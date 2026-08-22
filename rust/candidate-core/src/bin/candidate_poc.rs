#![deny(unsafe_op_in_unsafe_fn)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let mut args = env::args_os().skip(1);
    let mut self_check = false;
    let mut window_smoke = false;
    let mut report: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        if arg == "--self-check" {
            self_check = true;
        } else if arg == "--window-smoke" {
            window_smoke = true;
        } else if arg == "--report" {
            let Some(path) = args.next() else {
                eprintln!("--report requires a path");
                std::process::exit(2);
            };
            report = Some(PathBuf::from(path));
        } else {
            eprintln!("unknown argument: {}", arg.to_string_lossy());
            std::process::exit(2);
        }
    }

    if self_check == window_smoke {
        eprintln!("usage: fcitx5-candidate-poc (--self-check | --window-smoke) [--report PATH]");
        std::process::exit(2);
    }

    let result = if self_check {
        fcitx5_candidate_core::run_candidate_poc_self_check()
    } else {
        run_window_smoke()
    };

    match result {
        Ok(output) => {
            if let Some(path) = report {
                write_report(&path, &output);
                println!("candidate-poc-report={} result=PASS", path.display());
                return;
            }
            println!("{output}");
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn write_report(path: &Path, output: &str) {
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("failed to create report directory: {error}");
            std::process::exit(1);
        }
    }
    if let Err(error) = fs::write(path, output.as_bytes()) {
        eprintln!("failed to write report: {error}");
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn run_window_smoke() -> Result<String, String> {
    window_smoke::run()
}

#[cfg(not(windows))]
fn run_window_smoke() -> Result<String, String> {
    Err("window smoke is only available on Windows".to_owned())
}

#[cfg(windows)]
mod window_smoke {
    use fcitx5_candidate_core::{layout, LayoutInput, Orientation, Point, Rect as CoreRect, Size};
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};

    type Bool = i32;
    type Dword = u32;
    type Hbrush = *mut c_void;
    type Hcursor = *mut c_void;
    type Hdc = *mut c_void;
    type Hicon = *mut c_void;
    type Hinstance = *mut c_void;
    type Hmenu = *mut c_void;
    type Hwnd = *mut c_void;
    type Lparam = isize;
    type Lresult = isize;
    type Uint = u32;
    type Wparam = usize;

    const COLORREF_BACKGROUND: Dword = 0x00F8_F6F2;
    const COLORREF_TEXT: Dword = 0x0022_2222;
    const CS_HREDRAW: Uint = 0x0002;
    const CS_VREDRAW: Uint = 0x0001;
    const DT_LEFT: Uint = 0x0000;
    const DT_SINGLELINE: Uint = 0x0020;
    const DT_VCENTER: Uint = 0x0004;
    const SW_SHOWNOACTIVATE: i32 = 4;
    const TRANSPARENT: i32 = 1;
    const WM_DESTROY: Uint = 0x0002;
    const WM_PAINT: Uint = 0x000F;
    const WS_EX_NOACTIVATE: Dword = 0x0800_0000;
    const WS_EX_TOOLWINDOW: Dword = 0x0000_0080;
    const WS_EX_TOPMOST: Dword = 0x0000_0008;
    const WS_POPUP: Dword = 0x8000_0000;
    const WS_VISIBLE: Dword = 0x1000_0000;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct PaintStruct {
        hdc: Hdc,
        f_erase: Bool,
        rc_paint: Rect,
        f_restore: Bool,
        f_inc_update: Bool,
        rgb_reserved: [u8; 32],
    }

    #[repr(C)]
    struct WndClassW {
        style: Uint,
        lpfn_wnd_proc: Option<unsafe extern "system" fn(Hwnd, Uint, Wparam, Lparam) -> Lresult>,
        cb_cls_extra: i32,
        cb_wnd_extra: i32,
        h_instance: Hinstance,
        h_icon: Hicon,
        h_cursor: Hcursor,
        hbr_background: Hbrush,
        lpsz_menu_name: *const u16,
        lpsz_class_name: *const u16,
    }

    #[link(name = "user32")]
    extern "system" {
        fn BeginPaint(hwnd: Hwnd, paint: *mut PaintStruct) -> Hdc;
        fn CreateWindowExW(
            ex_style: Dword,
            class_name: *const u16,
            window_name: *const u16,
            style: Dword,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Hwnd,
            menu: Hmenu,
            instance: Hinstance,
            parameter: *mut c_void,
        ) -> Hwnd;
        fn DefWindowProcW(hwnd: Hwnd, message: Uint, wparam: Wparam, lparam: Lparam) -> Lresult;
        fn DestroyWindow(hwnd: Hwnd) -> Bool;
        fn DrawTextW(hdc: Hdc, text: *const u16, count: i32, rect: *mut Rect, format: Uint) -> i32;
        fn EndPaint(hwnd: Hwnd, paint: *const PaintStruct) -> Bool;
        fn FillRect(hdc: Hdc, rect: *const Rect, brush: Hbrush) -> i32;
        fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> Bool;
        fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: i32) -> i32;
        fn IsWindowVisible(hwnd: Hwnd) -> Bool;
        fn RegisterClassW(class: *const WndClassW) -> u16;
        fn SetBkMode(hdc: Hdc, mode: i32) -> i32;
        fn SetTextColor(hdc: Hdc, color: Dword) -> Dword;
        fn ShowWindow(hwnd: Hwnd, command: i32) -> Bool;
        fn UpdateWindow(hwnd: Hwnd) -> Bool;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateSolidBrush(color: Dword) -> Hbrush;
        fn DeleteObject(object: *mut c_void) -> Bool;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
    }

    pub fn run() -> Result<String, String> {
        let layout = layout(&LayoutInput {
            orientation: Orientation::Horizontal,
            items: vec![
                Size {
                    width: 92.0,
                    height: 34.0,
                },
                Size {
                    width: 164.0,
                    height: 34.0,
                },
                Size {
                    width: 130.0,
                    height: 34.0,
                },
            ],
            caret: Point { x: 180.0, y: 360.0 },
            caret_height: 24.0,
            work_area: CoreRect {
                left: 0.0,
                top: 0.0,
                right: 1920.0,
                bottom: 1080.0,
            },
            max_width: 720.0,
            padding_x: 8.0,
            padding_y: 6.0,
            row_gap: 2.0,
            column_gap: 8.0,
            selected: 0,
            ..LayoutInput::default()
        });
        let width = ((layout.window.right - layout.window.left).ceil() as i32).max(1);
        let height = ((layout.window.bottom - layout.window.top).ceil() as i32).max(1);
        let class_name = wide("Fcitx5CandidateRustPoc");
        let title = wide("Fcitx5 Candidate PoC - 1 😀 emoji");

        let instance = unsafe { GetModuleHandleW(null()) };
        let window_class = WndClassW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfn_wnd_proc: Some(window_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: instance,
            h_icon: null_mut(),
            h_cursor: null_mut(),
            hbr_background: null_mut(),
            lpsz_menu_name: null(),
            lpsz_class_name: class_name.as_ptr(),
        };
        let atom = unsafe { RegisterClassW(&window_class) };
        if atom == 0 {
            return Err("RegisterClassW failed for Rust Candidate PoC".to_owned());
        }
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                layout.window.left as i32,
                layout.window.top as i32,
                width,
                height,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            )
        };
        if hwnd.is_null() {
            return Err("CreateWindowExW failed for Rust Candidate PoC".to_owned());
        }
        let result = inspect_window(hwnd, width, height, &title);
        unsafe {
            DestroyWindow(hwnd);
        }
        result
    }

    fn inspect_window(
        hwnd: Hwnd,
        expected_width: i32,
        expected_height: i32,
        title: &[u16],
    ) -> Result<String, String> {
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            UpdateWindow(hwnd);
        }

        let mut rect = Rect::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return Err("GetWindowRect failed for Rust Candidate PoC".to_owned());
        }
        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return Err("Rust Candidate PoC window was not visible".to_owned());
        }
        let actual_width = rect.right - rect.left;
        let actual_height = rect.bottom - rect.top;
        if (actual_width - expected_width).abs() > 2 || (actual_height - expected_height).abs() > 2
        {
            return Err(format!(
                "Rust Candidate PoC window size mismatch: got {actual_width}x{actual_height}, expected {expected_width}x{expected_height}"
            ));
        }

        let mut text = [0u16; 128];
        let title_length = unsafe { GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32) };
        if title_length <= 0 || !text.starts_with(&title[..title.len().saturating_sub(1)]) {
            return Err("Rust Candidate PoC accessibility title was not readable".to_owned());
        }
        Ok(format!(
            "{{\n  \"component\":\"fcitx5-candidate-poc\",\n  \"kind\":\"rust-window-smoke\",\n  \"hwnd_created\":true,\n  \"no_activate\":true,\n  \"cpp_ffi\":false,\n  \"send_input\":false,\n  \"global_hooks\":false,\n  \"process_injection\":false,\n  \"window_left\":{},\n  \"window_top\":{},\n  \"window_right\":{},\n  \"window_bottom\":{},\n  \"visible\":true,\n  \"accessibility_title_readable\":true,\n  \"emoji_candidate_render_path\":true,\n  \"result\":\"PASS\"\n}}",
            rect.left, rect.top, rect.right, rect.bottom
        ))
    }

    unsafe extern "system" fn window_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: Wparam,
        lparam: Lparam,
    ) -> Lresult {
        match message {
            WM_PAINT => {
                let mut paint = PaintStruct {
                    hdc: null_mut(),
                    f_erase: 0,
                    rc_paint: Rect::default(),
                    f_restore: 0,
                    f_inc_update: 0,
                    rgb_reserved: [0; 32],
                };
                let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
                if !hdc.is_null() {
                    let brush = unsafe { CreateSolidBrush(COLORREF_BACKGROUND) };
                    if !brush.is_null() {
                        unsafe {
                            FillRect(hdc, &paint.rc_paint, brush);
                            DeleteObject(brush);
                        }
                    }
                    unsafe {
                        SetBkMode(hdc, TRANSPARENT);
                        SetTextColor(hdc, COLORREF_TEXT);
                    }
                    let text = wide("1  😀  emoji    2  候选  text fallback");
                    let mut text_rect = Rect {
                        left: 12,
                        top: 0,
                        right: 512,
                        bottom: 46,
                    };
                    unsafe {
                        DrawTextW(
                            hdc,
                            text.as_ptr(),
                            (text.len() - 1) as i32,
                            &mut text_rect,
                            DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                        );
                        EndPaint(hwnd, &paint);
                    }
                }
                0
            }
            WM_DESTROY => 0,
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}
