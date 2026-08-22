#![deny(unsafe_op_in_unsafe_fn)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let mut args = env::args_os().skip(1);
    let mut self_check = false;
    let mut window_smoke = false;
    let mut demo_snapshot = false;
    let mut scroll_demo_snapshot = false;
    let mut host_snapshot: Option<String> = None;
    let mut dpi_scale = 1.0_f32;
    let mut report: Option<PathBuf> = None;
    let mut screenshot: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        if arg == "--self-check" {
            self_check = true;
        } else if arg == "--window-smoke" {
            window_smoke = true;
        } else if arg == "--demo-snapshot" {
            demo_snapshot = true;
        } else if arg == "--scroll-demo-snapshot" {
            scroll_demo_snapshot = true;
        } else if arg == "--host-snapshot" {
            let Some(host) = args.next() else {
                eprintln!("--host-snapshot requires a mock host name");
                std::process::exit(2);
            };
            host_snapshot = Some(host.to_string_lossy().into_owned());
        } else if arg == "--dpi-scale" {
            let Some(value) = args.next() else {
                eprintln!("--dpi-scale requires a value");
                std::process::exit(2);
            };
            let parsed = value.to_string_lossy().parse::<f32>().unwrap_or(0.0);
            if !(0.5..=4.0).contains(&parsed) || !parsed.is_finite() {
                eprintln!("--dpi-scale must be a finite value from 0.5 through 4.0");
                std::process::exit(2);
            }
            dpi_scale = parsed;
        } else if arg == "--report" {
            let Some(path) = args.next() else {
                eprintln!("--report requires a path");
                std::process::exit(2);
            };
            report = Some(PathBuf::from(path));
        } else if arg == "--screenshot" {
            let Some(path) = args.next() else {
                eprintln!("--screenshot requires a path");
                std::process::exit(2);
            };
            screenshot = Some(PathBuf::from(path));
        } else {
            eprintln!("unknown argument: {}", arg.to_string_lossy());
            std::process::exit(2);
        }
    }

    if self_check == window_smoke {
        eprintln!(
            "usage: fcitx5-candidate-poc (--self-check | --window-smoke) [--demo-snapshot | --scroll-demo-snapshot | --host-snapshot HOST] [--dpi-scale VALUE] [--report PATH] [--screenshot PATH]"
        );
        std::process::exit(2);
    }
    let mode_count = usize::from(demo_snapshot)
        + usize::from(scroll_demo_snapshot)
        + usize::from(host_snapshot.is_some());
    if mode_count > 1 {
        eprintln!("snapshot modes are mutually exclusive");
        std::process::exit(2);
    }

    let result = if self_check {
        fcitx5_candidate_core::run_candidate_poc_self_check()
    } else {
        run_window_smoke(
            screenshot.as_deref(),
            demo_snapshot,
            scroll_demo_snapshot,
            host_snapshot.as_deref(),
            dpi_scale,
        )
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
fn run_window_smoke(
    screenshot: Option<&Path>,
    demo_snapshot: bool,
    scroll_demo_snapshot: bool,
    host_snapshot: Option<&str>,
    dpi_scale: f32,
) -> Result<String, String> {
    window_smoke::run(
        screenshot,
        demo_snapshot,
        scroll_demo_snapshot,
        host_snapshot,
        dpi_scale,
    )
}

#[cfg(not(windows))]
fn run_window_smoke(
    _screenshot: Option<&Path>,
    _demo_snapshot: bool,
    _scroll_demo_snapshot: bool,
    _host_snapshot: Option<&str>,
    _dpi_scale: f32,
) -> Result<String, String> {
    Err("window smoke is only available on Windows".to_owned())
}

#[cfg(windows)]
mod window_smoke {
    use fcitx5_candidate_core::{
        candidate_poc_scenarios, layout, LayoutInput, Orientation, PocCandidate, PocScenario,
        Point, Rect as CoreRect, Size,
    };
    use std::ffi::c_void;
    use std::fs;
    use std::path::Path;
    use std::ptr::{null, null_mut};
    use std::sync::OnceLock;

    type Bool = i32;
    type Dword = u32;
    type Hbrush = *mut c_void;
    type Hbitmap = *mut c_void;
    type Hcursor = *mut c_void;
    type Hdc = *mut c_void;
    type Hgdobj = *mut c_void;
    type Hicon = *mut c_void;
    type Hinstance = *mut c_void;
    type Hmenu = *mut c_void;
    type Hwnd = *mut c_void;
    type Hresult = i32;
    type Lparam = isize;
    type Lresult = isize;
    type Uint = u32;
    type Wparam = usize;

    const CHILDID_SELF: i32 = 0;
    const BI_RGB: Dword = 0;
    const COINIT_APARTMENTTHREADED: Dword = 0x2;
    const COLORREF_BACKGROUND: Dword = 0x00F8_F6F2;
    const COLORREF_SELECTED_BACKGROUND: Dword = 0x00D9_F2E4;
    const COLORREF_TEXT: Dword = 0x0022_2222;
    const CS_HREDRAW: Uint = 0x0002;
    const CS_VREDRAW: Uint = 0x0001;
    const DT_LEFT: Uint = 0x0000;
    const DT_SINGLELINE: Uint = 0x0020;
    const DT_VCENTER: Uint = 0x0004;
    const DIB_RGB_COLORS: Uint = 0;
    const OBJID_WINDOW: Dword = 0;
    const SRCCOPY: Dword = 0x00CC_0020;
    const SW_SHOWNOACTIVATE: i32 = 4;
    const TRANSPARENT: i32 = 1;
    const CLSCTX_INPROC_SERVER: Dword = 0x1;
    const UIA_CONTROL_TYPE_PROPERTY_ID: i32 = 30003;
    const UIA_NAME_PROPERTY_ID: i32 = 30005;
    const UIA_WINDOW_CONTROL_TYPE_ID: i32 = 50032;
    const UIA_PANE_CONTROL_TYPE_ID: i32 = 50033;
    const VT_BSTR: u16 = 8;
    const VT_I4: u16 = 3;
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
    #[derive(Clone, Copy)]
    struct BitmapInfoHeader {
        bi_size: Dword,
        bi_width: i32,
        bi_height: i32,
        bi_planes: u16,
        bi_bit_count: u16,
        bi_compression: Dword,
        bi_size_image: Dword,
        bi_x_pels_per_meter: i32,
        bi_y_pels_per_meter: i32,
        bi_clr_used: Dword,
        bi_clr_important: Dword,
    }

    #[repr(C)]
    struct BitmapInfo {
        bmi_header: BitmapInfoHeader,
        bmi_colors: [Dword; 1],
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

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Variant {
        vt: u16,
        reserved1: u16,
        reserved2: u16,
        reserved3: u16,
        data1: isize,
        data2: isize,
    }

    #[repr(C)]
    struct IAccessible {
        vtable: *const IAccessibleVtable,
    }

    #[repr(C)]
    struct IAccessibleVtable {
        query_interface:
            unsafe extern "system" fn(*mut IAccessible, *const Guid, *mut *mut c_void) -> Hresult,
        add_ref: unsafe extern "system" fn(*mut IAccessible) -> u32,
        release: unsafe extern "system" fn(*mut IAccessible) -> u32,
        get_type_info_count: usize,
        get_type_info: usize,
        get_ids_of_names: usize,
        invoke: usize,
        get_acc_parent: usize,
        get_acc_child_count: usize,
        get_acc_child: usize,
        get_acc_name:
            unsafe extern "system" fn(*mut IAccessible, Variant, *mut *mut u16) -> Hresult,
    }

    #[repr(C)]
    struct IUIAutomation {
        vtable: *const IUIAutomationVtable,
    }

    #[repr(C)]
    struct IUIAutomationVtable {
        query_interface: usize,
        add_ref: usize,
        release: unsafe extern "system" fn(*mut IUIAutomation) -> u32,
        compare_elements: usize,
        compare_runtime_ids: usize,
        get_root_element: usize,
        element_from_handle: unsafe extern "system" fn(
            *mut IUIAutomation,
            Hwnd,
            *mut *mut IUIAutomationElement,
        ) -> Hresult,
    }

    #[repr(C)]
    struct IUIAutomationElement {
        vtable: *const IUIAutomationElementVtable,
    }

    #[repr(C)]
    struct IUIAutomationElementVtable {
        query_interface: usize,
        add_ref: usize,
        release: unsafe extern "system" fn(*mut IUIAutomationElement) -> u32,
        set_focus: usize,
        get_runtime_id: usize,
        find_first: usize,
        find_all: usize,
        find_first_build_cache: usize,
        find_all_build_cache: usize,
        build_updated_cache: usize,
        get_current_property_value:
            unsafe extern "system" fn(*mut IUIAutomationElement, i32, *mut Variant) -> Hresult,
    }

    const CLSID_CUIAUTOMATION: Guid = Guid {
        data1: 0xff48dba4,
        data2: 0x60ef,
        data3: 0x4201,
        data4: [0xaa, 0x87, 0x54, 0x10, 0x3e, 0xef, 0x59, 0x4e],
    };

    const IID_IUIAUTOMATION: Guid = Guid {
        data1: 0x30cbe57d,
        data2: 0xd9d0,
        data3: 0x452a,
        data4: [0xab, 0x13, 0x7a, 0xc5, 0xac, 0x48, 0x25, 0xee],
    };

    const IID_IACCESSIBLE: Guid = Guid {
        data1: 0x618736e0,
        data2: 0x3c3d,
        data3: 0x11cf,
        data4: [0x81, 0x0c, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71],
    };

    static WINDOW_TEXT: OnceLock<Vec<Vec<u16>>> = OnceLock::new();
    static WINDOW_LAYOUT_RECTS: OnceLock<Vec<Rect>> = OnceLock::new();
    static WINDOW_SELECTED_VISIBLE: OnceLock<Option<usize>> = OnceLock::new();

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
        fn GetWindowDC(hwnd: Hwnd) -> Hdc;
        fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> Bool;
        fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: i32) -> i32;
        fn IsWindowVisible(hwnd: Hwnd) -> Bool;
        fn ReleaseDC(hwnd: Hwnd, hdc: Hdc) -> i32;
        fn RegisterClassW(class: *const WndClassW) -> u16;
        fn SetBkMode(hdc: Hdc, mode: i32) -> i32;
        fn SetTextColor(hdc: Hdc, color: Dword) -> Dword;
        fn ShowWindow(hwnd: Hwnd, command: i32) -> Bool;
        fn UpdateWindow(hwnd: Hwnd) -> Bool;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn BitBlt(
            dest: Hdc,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            src: Hdc,
            x1: i32,
            y1: i32,
            rop: Dword,
        ) -> Bool;
        fn CreateCompatibleBitmap(hdc: Hdc, cx: i32, cy: i32) -> Hbitmap;
        fn CreateCompatibleDC(hdc: Hdc) -> Hdc;
        fn CreateSolidBrush(color: Dword) -> Hbrush;
        fn DeleteDC(hdc: Hdc) -> Bool;
        fn DeleteObject(object: *mut c_void) -> Bool;
        fn GetDIBits(
            hdc: Hdc,
            bitmap: Hbitmap,
            start: Uint,
            lines: Uint,
            bits: *mut c_void,
            info: *mut BitmapInfo,
            usage: Uint,
        ) -> i32;
        fn SelectObject(hdc: Hdc, object: Hgdobj) -> Hgdobj;
    }

    #[link(name = "ole32")]
    extern "system" {
        fn CoCreateInstance(
            clsid: *const Guid,
            outer: *mut c_void,
            context: Dword,
            iid: *const Guid,
            object: *mut *mut c_void,
        ) -> Hresult;
        fn CoInitializeEx(reserved: *mut c_void, coinit: Dword) -> Hresult;
        fn CoUninitialize();
    }

    #[link(name = "oleacc")]
    extern "system" {
        fn AccessibleObjectFromWindow(
            hwnd: Hwnd,
            object_id: Dword,
            iid: *const Guid,
            object: *mut *mut c_void,
        ) -> Hresult;
    }

    #[link(name = "oleaut32")]
    extern "system" {
        fn SysFreeString(value: *mut u16);
        fn SysStringLen(value: *const u16) -> u32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
    }

    struct CaptureEvidence {
        bytes: usize,
        non_background_pixels: usize,
        checksum: u64,
        path: String,
    }

    struct LayoutEvidence {
        visible_candidate_rects: usize,
        painted_candidate_rects: usize,
        rects_inside_window: bool,
        rects_non_overlapping: bool,
        layout_driven_paint: bool,
    }

    struct InspectionSpec<'a> {
        expected_width: i32,
        expected_height: i32,
        title: &'a [u16],
        screenshot: Option<&'a Path>,
        snapshot_name: &'a str,
        orientation_name: &'a str,
        host_name: &'a str,
        locale_name: &'a str,
        popup_allowed: bool,
        candidate_count: usize,
        layout_evidence: LayoutEvidence,
        dpi_scale: f32,
        scroll_mode: bool,
        expects_emoji: bool,
    }

    pub fn run(
        screenshot: Option<&Path>,
        demo_snapshot: bool,
        scroll_demo_snapshot: bool,
        host_snapshot: Option<&str>,
        dpi_scale: f32,
    ) -> Result<String, String> {
        if !(0.5..=4.0).contains(&dpi_scale) || !dpi_scale.is_finite() {
            return Err(
                "Rust Candidate PoC DPI scale must be finite and within 0.5..=4.0".to_owned(),
            );
        }
        let (
            layout,
            title,
            text_lines,
            snapshot_name,
            orientation_name,
            host_name,
            locale_name,
            popup_allowed,
            effective_dpi_scale,
            scroll_mode,
            selected_candidate,
            emoji_candidate_render_path,
        ) = if scroll_demo_snapshot {
            let items = (0..60)
                .map(|index| Size {
                    width: if index < 42 {
                        56.0 * dpi_scale
                    } else {
                        96.0 * dpi_scale
                    },
                    height: 34.0 * dpi_scale,
                })
                .collect();
            let text_lines = scroll_demo_text();
            (
                layout(&LayoutInput {
                    orientation: Orientation::Horizontal,
                    items,
                    caret: Point { x: 100.0, y: 100.0 },
                    caret_height: 24.0 * dpi_scale,
                    work_area: CoreRect {
                        left: 0.0,
                        top: 0.0,
                        right: 1920.0,
                        bottom: 1080.0,
                    },
                    max_width: 720.0 * dpi_scale,
                    padding_x: 8.0 * dpi_scale,
                    padding_y: 6.0 * dpi_scale,
                    row_gap: 2.0 * dpi_scale,
                    column_gap: 8.0 * dpi_scale,
                    scroll_mode: true,
                    scroll_columns: 6,
                    scroll_visible_rows: 6,
                    selected: 18,
                    scroll_cell_width: 96.0 * dpi_scale,
                    ..LayoutInput::default()
                }),
                wide("Fcitx5 Candidate Scroll Demo"),
                text_lines,
                "scroll-demo-snapshot",
                "horizontal",
                "demo-scroll",
                "zh-CN",
                true,
                dpi_scale,
                true,
                18,
                false,
            )
        } else if let Some(host) = host_snapshot {
            let scenario = host_snapshot_scenario(host)?;
            let orientation = scenario.expected_orientation;
            let item_sizes = scenario
                .candidates
                .iter()
                .map(|candidate| host_candidate_size(candidate, scenario.dpi_scale))
                .collect();
            let text_lines = scenario
                .candidates
                .iter()
                .map(|candidate| {
                    wide(
                        format!(
                            "{} {} {}",
                            candidate.label, candidate.text, candidate.comment
                        )
                        .trim(),
                    )
                })
                .collect();
            (
                layout(&LayoutInput {
                    orientation,
                    items: item_sizes,
                    caret: scenario.caret,
                    caret_height: 24.0 * scenario.dpi_scale,
                    work_area: scenario.work_area,
                    max_width: 720.0 * scenario.dpi_scale,
                    padding_x: 8.0 * scenario.dpi_scale,
                    padding_y: 6.0 * scenario.dpi_scale,
                    row_gap: 2.0 * scenario.dpi_scale,
                    column_gap: 8.0 * scenario.dpi_scale,
                    selected: scenario.selected,
                    ..LayoutInput::default()
                }),
                wide(&format!("Fcitx5 Candidate Host - {}", scenario.host)),
                text_lines,
                "host-snapshot",
                orientation_to_name(orientation),
                scenario.host,
                scenario.locale,
                scenario.popup_allowed,
                scenario.dpi_scale,
                false,
                scenario.selected,
                scenario
                    .candidates
                    .iter()
                    .any(|candidate| contains_non_bmp_or_zwj(&candidate.text)),
            )
        } else if demo_snapshot {
            (
                layout(&LayoutInput {
                    orientation: Orientation::Vertical,
                    items: vec![
                        Size {
                            width: 110.0 * dpi_scale,
                            height: 34.0 * dpi_scale,
                        },
                        Size {
                            width: 86.0 * dpi_scale,
                            height: 34.0 * dpi_scale,
                        },
                        Size {
                            width: 110.0 * dpi_scale,
                            height: 34.0 * dpi_scale,
                        },
                    ],
                    caret: Point { x: 100.0, y: 100.0 },
                    caret_height: 24.0 * dpi_scale,
                    work_area: CoreRect {
                        left: 0.0,
                        top: 0.0,
                        right: 1920.0,
                        bottom: 1080.0,
                    },
                    max_width: 720.0 * dpi_scale,
                    padding_x: 8.0 * dpi_scale,
                    padding_y: 6.0 * dpi_scale,
                    row_gap: 2.0 * dpi_scale,
                    column_gap: 8.0 * dpi_scale,
                    selected: 0,
                    ..LayoutInput::default()
                }),
                wide("Fcitx5 Candidate Demo"),
                vec![wide("1. 输入法"), wide("2. 输入"), wide("3. 中文")],
                "demo-snapshot",
                "vertical",
                "demo",
                "zh-CN",
                true,
                dpi_scale,
                false,
                0,
                false,
            )
        } else {
            (
                layout(&LayoutInput {
                    orientation: Orientation::Horizontal,
                    items: vec![
                        Size {
                            width: 92.0 * dpi_scale,
                            height: 34.0 * dpi_scale,
                        },
                        Size {
                            width: 164.0 * dpi_scale,
                            height: 34.0 * dpi_scale,
                        },
                        Size {
                            width: 130.0 * dpi_scale,
                            height: 34.0 * dpi_scale,
                        },
                    ],
                    caret: Point { x: 180.0, y: 360.0 },
                    caret_height: 24.0 * dpi_scale,
                    work_area: CoreRect {
                        left: 0.0,
                        top: 0.0,
                        right: 1920.0,
                        bottom: 1080.0,
                    },
                    max_width: 720.0 * dpi_scale,
                    padding_x: 8.0 * dpi_scale,
                    padding_y: 6.0 * dpi_scale,
                    row_gap: 2.0 * dpi_scale,
                    column_gap: 8.0 * dpi_scale,
                    selected: 0,
                    ..LayoutInput::default()
                }),
                wide("Fcitx5 Candidate PoC - 1 😀 emoji"),
                vec![wide("1  😀  emoji"), wide("2  候选  text fallback")],
                "emoji-window",
                "horizontal",
                "emoji-smoke",
                "zh-CN",
                true,
                dpi_scale,
                false,
                0,
                true,
            )
        };
        let total_candidate_count = text_lines.len();
        let visible_rects = visible_window_rects(&layout.items, layout.window)?;
        let selected_visible = layout
            .item_indices
            .iter()
            .position(|index| *index == selected_candidate);
        let layout_evidence = inspect_layout_rectangles(&layout.items, layout.window)?;
        let visible_text_lines = visible_text_lines(&text_lines, &layout.item_indices);
        let width = ((layout.window.right - layout.window.left).ceil() as i32).max(1);
        let height = ((layout.window.bottom - layout.window.top).ceil() as i32).max(1);
        let class_name = wide("Fcitx5CandidateRustPoc");
        let _ = WINDOW_TEXT.set(visible_text_lines);
        let _ = WINDOW_LAYOUT_RECTS.set(visible_rects);
        let _ = WINDOW_SELECTED_VISIBLE.set(selected_visible);

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
        let result = inspect_window(
            hwnd,
            InspectionSpec {
                expected_width: width,
                expected_height: height,
                title: &title,
                screenshot,
                snapshot_name,
                orientation_name,
                host_name,
                locale_name,
                popup_allowed,
                candidate_count: total_candidate_count,
                layout_evidence,
                dpi_scale: effective_dpi_scale,
                scroll_mode,
                expects_emoji: emoji_candidate_render_path,
            },
        );
        unsafe {
            DestroyWindow(hwnd);
        }
        result
    }

    fn inspect_window(hwnd: Hwnd, spec: InspectionSpec<'_>) -> Result<String, String> {
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
        if (actual_width - spec.expected_width).abs() > 2
            || (actual_height - spec.expected_height).abs() > 2
        {
            return Err(format!(
                "Rust Candidate PoC window size mismatch: got {actual_width}x{actual_height}, expected {}x{}",
                spec.expected_width, spec.expected_height
            ));
        }

        let mut text = [0u16; 128];
        let title_length = unsafe { GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32) };
        if title_length <= 0 || !text.starts_with(&spec.title[..spec.title.len().saturating_sub(1)])
        {
            return Err("Rust Candidate PoC accessibility title was not readable".to_owned());
        }
        let accessible_name = accessible_name(hwnd)?;
        if !accessible_name.contains("Fcitx5 Candidate") {
            return Err(format!(
                "Rust Candidate PoC MSAA accessible name mismatch: {accessible_name}"
            ));
        }
        if spec.expects_emoji && !accessible_name.contains("emoji") {
            return Err(format!(
                "Rust Candidate PoC MSAA accessible name missing emoji path: {accessible_name}"
            ));
        }
        let uia = uia_window_evidence(hwnd)?;
        if !uia.name.contains("Fcitx5 Candidate") {
            return Err(format!(
                "Rust Candidate PoC UIA name mismatch: {}",
                uia.name
            ));
        }
        if uia.control_type != UIA_WINDOW_CONTROL_TYPE_ID
            && uia.control_type != UIA_PANE_CONTROL_TYPE_ID
        {
            return Err(format!(
                "Rust Candidate PoC UIA control type mismatch: {}",
                uia.control_type
            ));
        }
        let capture = if let Some(path) = spec.screenshot {
            Some(capture_window(hwnd, actual_width, actual_height, path)?)
        } else {
            None
        };
        let capture_json = capture.as_ref().map_or_else(
            || {
                String::from(
                    "  \"screenshot_written\":false,\n  \"visual_non_background_pixels\":0,\n  \"visual_checksum\":0,\n",
                )
            },
            |capture| {
                format!(
                    "  \"screenshot_written\":true,\n  \"screenshot_path\":\"{}\",\n  \"screenshot_bytes\":{},\n  \"visual_non_background_pixels\":{},\n  \"visual_checksum\":{},\n",
                    json_escape(&capture.path),
                    capture.bytes,
                    capture.non_background_pixels,
                    capture.checksum
                )
            },
        );
        Ok(format!(
            "{{\n  \"component\":\"fcitx5-candidate-poc\",\n  \"kind\":\"rust-window-smoke\",\n  \"snapshot_name\":\"{}\",\n  \"orientation\":\"{}\",\n  \"host\":\"{}\",\n  \"locale\":\"{}\",\n  \"popup_allowed\":{},\n  \"candidate_count\":{},\n  \"visible_candidate_rects\":{},\n  \"painted_candidate_rects\":{},\n  \"layout_driven_paint\":{},\n  \"layout_rects_inside_window\":{},\n  \"layout_rects_non_overlapping\":{},\n  \"dpi_scale\":{:.2},\n  \"scroll_mode\":{},\n  \"hwnd_created\":true,\n  \"no_activate\":true,\n  \"cpp_ffi\":false,\n  \"send_input\":false,\n  \"global_hooks\":false,\n  \"process_injection\":false,\n  \"window_left\":{},\n  \"window_top\":{},\n  \"window_right\":{},\n  \"window_bottom\":{},\n  \"visible\":true,\n  \"accessibility_title_readable\":true,\n  \"msaa_accessible_name_readable\":true,\n  \"uia_name_readable\":true,\n  \"uia_control_type\":{},\n{}  \"emoji_candidate_render_path\":{},\n  \"result\":\"PASS\"\n}}",
            json_escape(spec.snapshot_name),
            json_escape(spec.orientation_name),
            json_escape(spec.host_name),
            json_escape(spec.locale_name),
            if spec.popup_allowed { "true" } else { "false" },
            spec.candidate_count,
            spec.layout_evidence.visible_candidate_rects,
            spec.layout_evidence.painted_candidate_rects,
            if spec.layout_evidence.layout_driven_paint {
                "true"
            } else {
                "false"
            },
            if spec.layout_evidence.rects_inside_window {
                "true"
            } else {
                "false"
            },
            if spec.layout_evidence.rects_non_overlapping {
                "true"
            } else {
                "false"
            },
            spec.dpi_scale,
            if spec.scroll_mode { "true" } else { "false" },
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            uia.control_type,
            capture_json,
            if spec.expects_emoji { "true" } else { "false" }
        ))
    }

    fn capture_window(
        hwnd: Hwnd,
        width: i32,
        height: i32,
        path: &Path,
    ) -> Result<CaptureEvidence, String> {
        if width <= 0 || height <= 0 {
            return Err("cannot capture an empty Rust Candidate PoC window".to_owned());
        }
        let window_dc = unsafe { GetWindowDC(hwnd) };
        if window_dc.is_null() {
            return Err("GetWindowDC failed for Rust Candidate PoC".to_owned());
        }
        let memory_dc = unsafe { CreateCompatibleDC(window_dc) };
        if memory_dc.is_null() {
            unsafe {
                ReleaseDC(hwnd, window_dc);
            }
            return Err("CreateCompatibleDC failed for Rust Candidate PoC".to_owned());
        }
        let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width, height) };
        if bitmap.is_null() {
            unsafe {
                DeleteDC(memory_dc);
                ReleaseDC(hwnd, window_dc);
            }
            return Err("CreateCompatibleBitmap failed for Rust Candidate PoC".to_owned());
        }
        let old_object = unsafe { SelectObject(memory_dc, bitmap.cast()) };
        let copied = unsafe { BitBlt(memory_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY) };
        if copied == 0 {
            unsafe {
                SelectObject(memory_dc, old_object);
                DeleteObject(bitmap);
                DeleteDC(memory_dc);
                ReleaseDC(hwnd, window_dc);
            }
            return Err("BitBlt failed for Rust Candidate PoC".to_owned());
        }

        let bytes_per_pixel = 4usize;
        let pixel_bytes = width as usize * height as usize * bytes_per_pixel;
        let mut pixels = vec![0u8; pixel_bytes];
        let mut info = BitmapInfo {
            bmi_header: BitmapInfoHeader {
                bi_size: std::mem::size_of::<BitmapInfoHeader>() as Dword,
                bi_width: width,
                bi_height: -height,
                bi_planes: 1,
                bi_bit_count: 32,
                bi_compression: BI_RGB,
                bi_size_image: pixel_bytes as Dword,
                bi_x_pels_per_meter: 0,
                bi_y_pels_per_meter: 0,
                bi_clr_used: 0,
                bi_clr_important: 0,
            },
            bmi_colors: [0],
        };
        let lines = unsafe {
            GetDIBits(
                memory_dc,
                bitmap,
                0,
                height as Uint,
                pixels.as_mut_ptr().cast(),
                &mut info,
                DIB_RGB_COLORS,
            )
        };
        unsafe {
            SelectObject(memory_dc, old_object);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(hwnd, window_dc);
        }
        if lines != height {
            return Err("GetDIBits failed for Rust Candidate PoC".to_owned());
        }

        let non_background_pixels = pixels
            .chunks_exact(bytes_per_pixel)
            .filter(|pixel| {
                let blue = pixel[0] as i16;
                let green = pixel[1] as i16;
                let red = pixel[2] as i16;
                (red - 0xF2).abs() > 8 || (green - 0xF6).abs() > 8 || (blue - 0xF8).abs() > 8
            })
            .count();
        if non_background_pixels < 8 {
            return Err("Rust Candidate PoC screenshot did not contain visible text".to_owned());
        }
        let checksum = fnv1a64(&pixels);
        write_bmp(path, width, height, &pixels)?;
        let metadata = fs::metadata(path)
            .map_err(|error| format!("Rust Candidate PoC screenshot metadata failed: {error}"))?;
        Ok(CaptureEvidence {
            bytes: metadata.len() as usize,
            non_background_pixels,
            checksum,
            path: path.display().to_string(),
        })
    }

    fn write_bmp(path: &Path, width: i32, height: i32, pixels: &[u8]) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create screenshot directory: {error}"))?;
        }
        let header_size = 14usize + 40usize;
        let file_size = header_size + pixels.len();
        let mut bytes = Vec::with_capacity(file_size);
        bytes.extend_from_slice(b"BM");
        bytes.extend_from_slice(&(file_size as u32).to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&(header_size as u32).to_le_bytes());
        bytes.extend_from_slice(&40u32.to_le_bytes());
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&(-height).to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&32u16.to_le_bytes());
        bytes.extend_from_slice(&BI_RGB.to_le_bytes());
        bytes.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(pixels);
        fs::write(path, bytes)
            .map_err(|error| format!("failed to write Rust Candidate PoC screenshot: {error}"))
    }

    fn fnv1a64(bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        hash
    }

    fn json_escape(value: &str) -> String {
        let mut escaped = String::new();
        for character in value.chars() {
            match character {
                '"' => escaped.push_str("\\\""),
                '\\' => escaped.push_str("\\\\"),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                character if character < ' ' => {
                    escaped.push_str(&format!("\\u{:04x}", character as u32));
                }
                character => escaped.push(character),
            }
        }
        escaped
    }

    fn accessible_name(hwnd: Hwnd) -> Result<String, String> {
        let init_result = unsafe { CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED) };
        let should_uninitialize = init_result >= 0;
        let mut object: *mut c_void = null_mut();
        let result = unsafe {
            AccessibleObjectFromWindow(hwnd, OBJID_WINDOW, &IID_IACCESSIBLE, &mut object)
        };
        if result < 0 || object.is_null() {
            if should_uninitialize {
                unsafe {
                    CoUninitialize();
                }
            }
            return Err(format!(
                "AccessibleObjectFromWindow failed for Rust Candidate PoC: HRESULT 0x{:08x}",
                result as u32
            ));
        }

        let accessible = object.cast::<IAccessible>();
        let variant = Variant {
            vt: VT_I4,
            reserved1: 0,
            reserved2: 0,
            reserved3: 0,
            data1: CHILDID_SELF as isize,
            data2: 0,
        };
        let mut name: *mut u16 = null_mut();
        let name_result =
            unsafe { ((*(*accessible).vtable).get_acc_name)(accessible, variant, &mut name) };
        let release = unsafe { (*(*accessible).vtable).release };
        unsafe {
            release(accessible);
        }
        if should_uninitialize {
            unsafe {
                CoUninitialize();
            }
        }
        if name_result < 0 || name.is_null() {
            return Err(format!(
                "IAccessible::get_accName failed for Rust Candidate PoC: HRESULT 0x{:08x}",
                name_result as u32
            ));
        }
        let length = unsafe { SysStringLen(name) } as usize;
        let value = unsafe { std::slice::from_raw_parts(name, length) };
        let string = String::from_utf16_lossy(value);
        unsafe {
            SysFreeString(name);
        }
        Ok(string)
    }

    struct UiaEvidence {
        name: String,
        control_type: i32,
    }

    fn uia_window_evidence(hwnd: Hwnd) -> Result<UiaEvidence, String> {
        let init_result = unsafe { CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED) };
        let should_uninitialize = init_result >= 0;
        if init_result < 0 && init_result as u32 != 0x80010106 {
            return Err(format!(
                "CoInitializeEx failed before Rust Candidate PoC UIA check: HRESULT 0x{:08x}",
                init_result as u32
            ));
        }

        let mut automation_object: *mut c_void = null_mut();
        let create_result = unsafe {
            CoCreateInstance(
                &CLSID_CUIAUTOMATION,
                null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_IUIAUTOMATION,
                &mut automation_object,
            )
        };
        if create_result < 0 || automation_object.is_null() {
            if should_uninitialize {
                unsafe {
                    CoUninitialize();
                }
            }
            return Err(format!(
                "CoCreateInstance(CUIAutomation) failed for Rust Candidate PoC: HRESULT 0x{:08x}",
                create_result as u32
            ));
        }

        let automation = automation_object.cast::<IUIAutomation>();
        let mut element: *mut IUIAutomationElement = null_mut();
        let element_result = unsafe {
            ((*(*automation).vtable).element_from_handle)(automation, hwnd, &mut element)
        };
        unsafe {
            ((*(*automation).vtable).release)(automation);
        }
        if element_result < 0 || element.is_null() {
            if should_uninitialize {
                unsafe {
                    CoUninitialize();
                }
            }
            return Err(format!(
                "IUIAutomation::ElementFromHandle failed for Rust Candidate PoC: HRESULT 0x{:08x}",
                element_result as u32
            ));
        }

        let name_result = uia_bstr_property(element, UIA_NAME_PROPERTY_ID);
        let control_type_result = uia_i4_property(element, UIA_CONTROL_TYPE_PROPERTY_ID);
        unsafe {
            ((*(*element).vtable).release)(element);
        }
        if should_uninitialize {
            unsafe {
                CoUninitialize();
            }
        }

        Ok(UiaEvidence {
            name: name_result?,
            control_type: control_type_result?,
        })
    }

    fn empty_variant() -> Variant {
        Variant {
            vt: 0,
            reserved1: 0,
            reserved2: 0,
            reserved3: 0,
            data1: 0,
            data2: 0,
        }
    }

    fn uia_bstr_property(
        element: *mut IUIAutomationElement,
        property: i32,
    ) -> Result<String, String> {
        let mut value = empty_variant();
        let result = unsafe {
            ((*(*element).vtable).get_current_property_value)(element, property, &mut value)
        };
        if result < 0 || value.vt != VT_BSTR || value.data1 == 0 {
            return Err(format!(
                "IUIAutomationElement::GetCurrentPropertyValue({property}) failed for Rust Candidate PoC: HRESULT 0x{:08x}, vt {}",
                result as u32, value.vt
            ));
        }
        let bstr = value.data1 as *mut u16;
        let length = unsafe { SysStringLen(bstr) } as usize;
        let slice = unsafe { std::slice::from_raw_parts(bstr, length) };
        let string = String::from_utf16_lossy(slice);
        unsafe {
            SysFreeString(bstr);
        }
        Ok(string)
    }

    fn uia_i4_property(element: *mut IUIAutomationElement, property: i32) -> Result<i32, String> {
        let mut value = empty_variant();
        let result = unsafe {
            ((*(*element).vtable).get_current_property_value)(element, property, &mut value)
        };
        if result < 0 || value.vt != VT_I4 {
            return Err(format!(
                "IUIAutomationElement::GetCurrentPropertyValue({property}) failed for Rust Candidate PoC: HRESULT 0x{:08x}, vt {}",
                result as u32, value.vt
            ));
        }
        Ok(value.data1 as i32)
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
                    if let (Some(lines), Some(rects)) =
                        (WINDOW_TEXT.get(), WINDOW_LAYOUT_RECTS.get())
                    {
                        let selected = WINDOW_SELECTED_VISIBLE.get().copied().flatten();
                        for (index, (text, layout_rect)) in
                            lines.iter().zip(rects.iter()).enumerate()
                        {
                            if selected == Some(index) {
                                let brush =
                                    unsafe { CreateSolidBrush(COLORREF_SELECTED_BACKGROUND) };
                                if !brush.is_null() {
                                    unsafe {
                                        FillRect(hdc, layout_rect, brush);
                                        DeleteObject(brush);
                                    }
                                }
                            }
                            let mut text_rect = *layout_rect;
                            text_rect.left += 6;
                            text_rect.right -= 6;
                            unsafe {
                                DrawTextW(
                                    hdc,
                                    text.as_ptr(),
                                    (text.len() - 1) as i32,
                                    &mut text_rect,
                                    DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                                );
                            }
                        }
                    }
                    unsafe {
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

    fn visible_text_lines(text_lines: &[Vec<u16>], item_indices: &[usize]) -> Vec<Vec<u16>> {
        if item_indices.is_empty() {
            return text_lines.to_vec();
        }
        item_indices
            .iter()
            .filter_map(|index| text_lines.get(*index).cloned())
            .collect()
    }

    fn visible_window_rects(items: &[CoreRect], window: CoreRect) -> Result<Vec<Rect>, String> {
        if items.is_empty() {
            return Err("Rust Candidate PoC produced no visible paint rectangles".to_owned());
        }
        items
            .iter()
            .map(|item| {
                if !core_rect_inside(*item, window) {
                    return Err(
                        "Rust Candidate PoC cannot paint an item outside its window".to_owned()
                    );
                }
                Ok(Rect {
                    left: (item.left - window.left).round() as i32,
                    top: (item.top - window.top).round() as i32,
                    right: (item.right - window.left).round() as i32,
                    bottom: (item.bottom - window.top).round() as i32,
                })
            })
            .collect()
    }

    fn inspect_layout_rectangles(
        items: &[CoreRect],
        window: CoreRect,
    ) -> Result<LayoutEvidence, String> {
        if items.is_empty() {
            return Err("Rust Candidate PoC produced no visible candidate rectangles".to_owned());
        }
        for (index, item) in items.iter().enumerate() {
            if !core_rect_inside(*item, window) {
                return Err(format!(
                    "Rust Candidate PoC visible candidate {index} is outside the window"
                ));
            }
            for (other_index, other) in items.iter().enumerate().skip(index + 1) {
                if core_rects_overlap(*item, *other) {
                    return Err(format!(
                        "Rust Candidate PoC visible candidate rectangles overlap: {index} and {other_index}"
                    ));
                }
            }
        }
        Ok(LayoutEvidence {
            visible_candidate_rects: items.len(),
            painted_candidate_rects: items.len(),
            rects_inside_window: true,
            rects_non_overlapping: true,
            layout_driven_paint: true,
        })
    }

    fn core_rect_inside(inner: CoreRect, outer: CoreRect) -> bool {
        const EPSILON: f32 = 0.5;
        inner.left + EPSILON >= outer.left
            && inner.top + EPSILON >= outer.top
            && inner.right <= outer.right + EPSILON
            && inner.bottom <= outer.bottom + EPSILON
            && inner.right > inner.left
            && inner.bottom > inner.top
    }

    fn core_rects_overlap(left: CoreRect, right: CoreRect) -> bool {
        left.left < right.right
            && left.right > right.left
            && left.top < right.bottom
            && left.bottom > right.top
    }

    fn host_snapshot_scenario(host: &str) -> Result<PocScenario, String> {
        let scenario = candidate_poc_scenarios()
            .into_iter()
            .find(|scenario| scenario.host == host)
            .ok_or_else(|| format!("unknown Candidate PoC mock host: {host}"))?;
        if !scenario.popup_allowed {
            return Err(format!(
                "{host} is a UILess mock host; use --self-check for popup-suppressed host evidence"
            ));
        }
        Ok(scenario)
    }

    fn host_candidate_size(candidate: &PocCandidate, scale: f32) -> Size {
        let text_units = candidate.text.chars().count().max(1) as f32;
        let comment_units = candidate.comment.chars().count() as f32;
        let label_units = candidate.label.chars().count() as f32;
        Size {
            width: ((label_units * 14.0) + (text_units * 26.0) + (comment_units * 7.0) + 20.0)
                .clamp(56.0, 260.0)
                * scale,
            height: 34.0 * scale,
        }
    }

    fn orientation_to_name(orientation: Orientation) -> &'static str {
        match orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        }
    }

    fn contains_non_bmp_or_zwj(value: &str) -> bool {
        value.chars().any(|character| {
            character == '\u{200d}' || character == '\u{fe0f}' || character as u32 > 0xffff
        })
    }

    fn scroll_demo_text() -> Vec<Vec<u16>> {
        const WORDS: [&str; 42] = [
            "我", "哦", "窝", "沃", "握", "卧", "涡", "蜗", "渥", "幄", "斡", "龌", "喔", "莴",
            "倭", "硪", "挝", "肟", "偓", "涴", "踒", "猧", "婐", "捰", "瓁", "馧", "焥", "腛",
            "濣", "瞃", "擭", "雘", "臒", "檴", "嚄", "濩", "获", "惑", "豁", "霍", "藿", "镬",
        ];
        (0..60)
            .map(|index| {
                let text = if index < WORDS.len() {
                    WORDS[index].to_owned()
                } else {
                    format!("候选{}", index + 1)
                };
                let label = if (18..24).contains(&index) {
                    format!("{} ", index - 17)
                } else {
                    String::new()
                };
                wide(&format!("{label}{text}"))
            })
            .collect()
    }
}
