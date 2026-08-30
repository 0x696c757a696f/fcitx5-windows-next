#![deny(unsafe_op_in_unsafe_fn)]

use std::env;
use std::ffi::c_void;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

type Bool = i32;
type Dword = u32;
type Hbitmap = *mut c_void;
type Hdc = *mut c_void;
type Hgdobj = *mut c_void;
type Hwnd = *mut c_void;
type Lparam = isize;
type Lresult = isize;
type Uint = u32;
type Wparam = usize;

const BI_RGB: Dword = 0;
const DIB_RGB_COLORS: Uint = 0;
const GWL_STYLE: i32 = -16;
const PW_RENDERFULLCONTENT: Uint = 0x0000_0002;
const CAPTUREBLT: Dword = 0x4000_0000;
const SRCCOPY: Dword = 0x00CC_0020;
const WM_COMMAND: Uint = 0x0111;
const WM_CLOSE: Uint = 0x0010;
const WM_SETTEXT: Uint = 0x000C;
const WM_GETTEXT: Uint = 0x000D;
const WM_GETTEXTLENGTH: Uint = 0x000E;
const WM_PRINT: Uint = 0x0317;
const WM_PRINTCLIENT: Uint = 0x0318;
const CB_GETCOUNT: Uint = 0x0146;
const CB_GETCURSEL: Uint = 0x0147;
const CB_GETLBTEXT: Uint = 0x0148;
const CB_GETLBTEXTLEN: Uint = 0x0149;
const CB_SETCURSEL: Uint = 0x014E;
const LB_GETCOUNT: Uint = 0x018B;
const LB_SETCURSEL: Uint = 0x0186;
const PRF_CHECKVISIBLE: Lparam = 0x0000_0001;
const PRF_NONCLIENT: Lparam = 0x0000_0002;
const PRF_CLIENT: Lparam = 0x0000_0004;
const PRF_ERASEBKGND: Lparam = 0x0000_0008;
const PRF_CHILDREN: Lparam = 0x0000_0010;
const WS_TABSTOP: u32 = 0x0001_0000;

const K_NAV_GENERAL: i32 = 130;
const K_NAV_APPEARANCE: i32 = 131;
const K_NAV_SHORTCUTS: i32 = 132;
const K_NAV_UPDATES: i32 = 133;
const K_NAV_REPAIR: i32 = 134;
const K_NAV_PACKAGES: i32 = 135;
const K_PAGE_TITLE: i32 = 140;
const K_STATUS: i32 = 110;
const K_PREVIEW: i32 = 112;
const K_PACKAGES: i32 = 113;
const K_PACKAGE_DETAIL: i32 = 127;
const K_APPEARANCE_FONT_SIZE: i32 = 150;
const K_APPEARANCE_OPACITY: i32 = 151;
const K_APPEARANCE_FONT_FAMILY: i32 = 152;
const K_APPEARANCE_SPACING: i32 = 153;
const K_APPEARANCE_CORNER_RADIUS: i32 = 154;
const K_APPEARANCE_CANDIDATE_WIDTH: i32 = 155;
const K_INPUT_METHOD_LIST: i32 = 156;
const K_LANGUAGE_SELECTOR: i32 = 157;
const K_LABEL_FONT_SIZE: i32 = 160;
const K_LABEL_OPACITY: i32 = 161;
const K_LABEL_CANDIDATE_FONT: i32 = 162;
const K_LABEL_SPACING: i32 = 163;
const K_LABEL_CORNER_RADIUS: i32 = 164;
const K_LABEL_CANDIDATE_WIDTH: i32 = 165;
const K_LABEL_INPUT_METHODS: i32 = 166;
const K_LABEL_LANGUAGE: i32 = 167;
const K_PACKAGE_INSTALL: i32 = 170;
const K_PACKAGE_UPDATE: i32 = 171;
const K_PACKAGE_REMOVE: i32 = 172;
const K_PACKAGE_CONFIGURE: i32 = 173;
const K_PACKAGE_REFRESH: i32 = 174;
const K_PACKAGE_DETAILS: i32 = 175;
const K_PACKAGE_ENABLE_DISABLE: i32 = 176;
const K_PACKAGE_REPAIR: i32 = 177;
const K_SAVE_STATUS: i32 = 206;
const LBN_SELCHANGE: Wparam = 1;
const CBN_SELCHANGE: Wparam = 1;
const EN_CHANGE: Wparam = 0x0300;
const SWP_NOMOVE: Uint = 0x0002;
const SWP_NOZORDER: Uint = 0x0004;
const SWP_NOACTIVATE: Uint = 0x0010;
const PREVIEW_STATE_ENV: &str = "FCITX5_CONFIG_RUST_PREVIEW_STATE";
const COLOR_SETTINGS_BACKGROUND: (u8, u8, u8) = (243, 243, 243);
const COLOR_SETTINGS_SIDEBAR: (u8, u8, u8) = (246, 248, 250);
const COLOR_SETTINGS_CONTENT: (u8, u8, u8) = (255, 255, 255);

const PAGES: &[Page] = &[
    Page {
        id: K_NAV_GENERAL,
        slug: "input-methods",
        controls: &[
            K_PAGE_TITLE,
            K_LABEL_INPUT_METHODS,
            K_INPUT_METHOD_LIST,
            K_LABEL_LANGUAGE,
            K_LANGUAGE_SELECTOR,
            K_SAVE_STATUS,
        ],
    },
    Page {
        id: K_NAV_APPEARANCE,
        slug: "appearance",
        controls: &[
            K_PAGE_TITLE,
            K_PREVIEW,
            K_LABEL_FONT_SIZE,
            K_APPEARANCE_FONT_SIZE,
            K_LABEL_OPACITY,
            K_APPEARANCE_OPACITY,
            K_LABEL_CANDIDATE_FONT,
            K_APPEARANCE_FONT_FAMILY,
            K_LABEL_SPACING,
            K_APPEARANCE_SPACING,
            K_LABEL_CORNER_RADIUS,
            K_APPEARANCE_CORNER_RADIUS,
            K_LABEL_CANDIDATE_WIDTH,
            K_APPEARANCE_CANDIDATE_WIDTH,
            K_SAVE_STATUS,
        ],
    },
    Page {
        id: K_NAV_SHORTCUTS,
        slug: "shortcuts",
        controls: &[K_PAGE_TITLE, K_STATUS],
    },
    Page {
        id: K_NAV_UPDATES,
        slug: "updates",
        controls: &[K_PAGE_TITLE, K_STATUS],
    },
    Page {
        id: K_NAV_REPAIR,
        slug: "diagnostics-repair",
        controls: &[K_PAGE_TITLE, K_STATUS],
    },
    Page {
        id: K_NAV_PACKAGES,
        slug: "addons",
        controls: &[
            K_PAGE_TITLE,
            K_PACKAGES,
            K_PACKAGE_DETAIL,
            K_PACKAGE_INSTALL,
            K_PACKAGE_UPDATE,
            K_PACKAGE_REMOVE,
            K_PACKAGE_CONFIGURE,
            K_PACKAGE_REFRESH,
            K_PACKAGE_DETAILS,
            K_PACKAGE_ENABLE_DISABLE,
            K_PACKAGE_REPAIR,
            K_STATUS,
        ],
    },
];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
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

#[link(name = "user32")]
extern "system" {
    fn EnumChildWindows(
        hwnd: Hwnd,
        callback: extern "system" fn(Hwnd, Lparam) -> Bool,
        lparam: Lparam,
    ) -> Bool;
    fn EnumWindows(callback: extern "system" fn(Hwnd, Lparam) -> Bool, lparam: Lparam) -> Bool;
    fn GetDlgItem(hwnd: Hwnd, id: i32) -> Hwnd;
    fn GetDC(hwnd: Hwnd) -> Hdc;
    fn GetWindowLongW(hwnd: Hwnd, index: i32) -> i32;
    fn GetWindowDC(hwnd: Hwnd) -> Hdc;
    fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> Bool;
    fn GetWindowThreadProcessId(hwnd: Hwnd, process_id: *mut Dword) -> Dword;
    fn IsWindow(hwnd: Hwnd) -> Bool;
    fn IsWindowVisible(hwnd: Hwnd) -> Bool;
    fn PrintWindow(hwnd: Hwnd, hdc: Hdc, flags: Uint) -> Bool;
    fn ReleaseDC(hwnd: Hwnd, hdc: Hdc) -> i32;
    fn SendMessageW(hwnd: Hwnd, msg: Uint, wparam: Wparam, lparam: Lparam) -> Lresult;
    fn SetWindowPos(
        hwnd: Hwnd,
        hwnd_insert_after: Hwnd,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: Uint,
    ) -> Bool;
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
    fn DeleteDC(hdc: Hdc) -> Bool;
    fn DeleteObject(object: Hgdobj) -> Bool;
    fn GetDIBits(
        hdc: Hdc,
        bitmap: Hbitmap,
        start: Uint,
        lines: Uint,
        bits: *mut c_void,
        info: *mut BitmapInfo,
        usage: Uint,
    ) -> i32;
    fn RestoreDC(hdc: Hdc, saved_dc: i32) -> Bool;
    fn SaveDC(hdc: Hdc) -> i32;
    fn SelectObject(hdc: Hdc, object: Hgdobj) -> Hgdobj;
    fn SetViewportOrgEx(hdc: Hdc, x: i32, y: i32, point: *mut c_void) -> Bool;
}

#[derive(Clone, Copy)]
struct Page {
    id: i32,
    slug: &'static str,
    controls: &'static [i32],
}

struct Args {
    config_exe: PathBuf,
    out_dir: PathBuf,
    candidate_ui_exe: Option<PathBuf>,
}

struct BitmapCapture {
    width: i32,
    height: i32,
    pixels: Vec<u8>,
}

struct ChildPrintContext {
    memory_dc: Hdc,
    parent_rect: Rect,
}

#[derive(Clone, Copy, Debug, Default)]
struct ImageStats {
    bytes: usize,
    non_background_pixels: usize,
    checksum: u64,
    selected_green_pixels: usize,
    selected_blue_pixels: usize,
    selected_accent_pixels: usize,
    white_surface_pixels: usize,
    dark_surface_pixels: usize,
    dark_text_pixels: usize,
    shared_theme_pixels: usize,
    settings_background_pixels: usize,
    settings_sidebar_pixels: usize,
    settings_content_pixels: usize,
}

struct ProcessGuard {
    child: Child,
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    fs::create_dir_all(&args.out_dir).map_err(|error| format!("create output dir: {error}"))?;
    let candidate_reference = match args.candidate_ui_exe.as_ref() {
        Some(candidate_ui) => Some(capture_candidate_reference(candidate_ui, &args.out_dir)?),
        None => None,
    };
    let rust_config = rust_config_exe(&args.config_exe);
    let preview_state_path = args.out_dir.join("config-rust-preview-state.txt");
    let mut command = Command::new(&args.config_exe);
    if rust_config {
        command.env(PREVIEW_STATE_ENV, &preview_state_path);
    }
    let child = command
        .spawn()
        .map_err(|error| format!("launch {}: {error}", args.config_exe.display()))?;
    let guard = ProcessGuard { child };
    let hwnd = wait_for_window(guard.child.id(), Duration::from_secs(10))?;
    if rust_config {
        verify_resize_repaint(hwnd)?;
    }
    let mut report = String::new();
    report.push_str("# Config UI QA\n\n");
    report.push_str(&format!("- exe: `{}`\n", args.config_exe.display()));
    report.push_str(&format!("- pid: `{}`\n\n", guard.child.id()));
    report.push_str("| Page | Result | Screenshot |\n|---|---|---|\n");
    let mut persisted_font_family = None;
    for page in PAGES {
        navigate(hwnd, page.id)?;
        thread::sleep(Duration::from_millis(200));
        verify_page(hwnd, page)?;
        if page.id == K_NAV_GENERAL && (rust_config || has_child(hwnd, K_INPUT_METHOD_LIST)) {
            verify_enabled_input_method_list(hwnd)?;
            report.push_str("| input-methods-enabled-list | ok | non-empty Rust UI list |\n");
            verify_language_selector(hwnd)?;
            report.push_str("| language-selector | ok | localized Rust UI selector |\n");
        }
        if page.id == K_NAV_PACKAGES && (rust_config || has_child(hwnd, K_PACKAGE_INSTALL)) {
            verify_package_page(hwnd)?;
            report.push_str(
                "| packages-plugin-config | ok | package actions and config page load |\n",
            );
        }
        let file_name = format!("config-{}.bmp", page.slug);
        let mut capture = capture_window(hwnd)?;
        if page.id == K_NAV_APPEARANCE && selected_theme_accent_bbox(&capture).is_none() {
            if let Ok(screen_capture) = capture_window_from_screen(hwnd) {
                let screen_name = "config-appearance-screen.bmp";
                write_bitmap(&screen_capture, &args.out_dir.join(screen_name))?;
                if selected_theme_accent_bbox(&screen_capture).is_some() {
                    capture = screen_capture;
                }
            }
        }
        write_bitmap(&capture, &args.out_dir.join(&file_name))?;
        let stats = image_stats(&capture);
        if rust_config {
            verify_modern_settings_surface(page, stats)?;
            if page.id == K_NAV_APPEARANCE {
                verify_no_legacy_static_ghosting(&capture)?;
            }
        }
        if page.id == K_NAV_APPEARANCE {
            let preview = crop_config_preview(hwnd, &capture)?;
            let preview_name = "config-appearance-candidate-preview.bmp";
            write_bitmap(&preview, &args.out_dir.join(preview_name))?;
            let preview_stats = image_stats(&preview);
            if preview_stats.non_background_pixels < 128 {
                return Err(
                    "Config candidate preview crop did not contain visible rendering".to_string(),
                );
            }
            if let Some(reference) = candidate_reference {
                assert_preview_matches_candidate_theme(preview_stats, reference)?;
                report.push_str(&format!(
                    "| appearance-preview | ok: shared theme pixels {} | `{}` |\n",
                    preview_stats.shared_theme_pixels, preview_name
                ));
            }
            if rust_config_exe(&args.config_exe) || has_child(hwnd, K_APPEARANCE_FONT_SIZE) {
                verify_appearance_numeric_inputs(hwnd)?;
                report.push_str("| appearance-numeric-inputs | ok | Rust schema validation |\n");
            }
            if rust_config_exe(&args.config_exe) || has_child(hwnd, K_APPEARANCE_FONT_FAMILY) {
                let font_family = verify_system_font_picker(hwnd)?;
                persisted_font_family = Some(font_family);
                report.push_str(
                    "| appearance-system-font-picker | ok | Rust system font inventory |\n",
                );
            }
        } else if preview_region_has_candidate_accent(hwnd, page, &capture)? {
            return Err(format!(
                "{} page retained stale candidate preview accent pixels after navigation",
                page.slug
            ));
        }
        report.push_str(&format!("| {} | ok | `{}` |\n", page.slug, file_name));
    }
    if rust_config {
        let font_family = persisted_font_family
            .as_deref()
            .ok_or("Rust Config font picker did not run before persistence check")?;
        verify_system_font_picker_persistence(&args.config_exe, &preview_state_path, font_family)?;
        report
            .push_str("| appearance-system-font-picker-persistence | ok | restart round-trip |\n");
    }
    fs::write(args.out_dir.join("config-ui-qa.md"), report)
        .map_err(|error| format!("write report: {error}"))?;
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut values = env::args_os().skip(1);
    let mut config_exe = None;
    let mut out_dir = None;
    let mut candidate_ui_exe = None;
    while let Some(arg) = values.next() {
        if arg == "--config-exe" {
            config_exe = values.next().map(PathBuf::from);
        } else if arg == "--out" {
            out_dir = values.next().map(PathBuf::from);
        } else if arg == "--candidate-ui-exe" {
            candidate_ui_exe = values.next().map(PathBuf::from);
        } else {
            return Err(format!("unknown argument: {}", arg.to_string_lossy()));
        }
    }
    Ok(Args {
        config_exe: config_exe.ok_or("missing --config-exe")?,
        out_dir: out_dir.ok_or("missing --out")?,
        candidate_ui_exe,
    })
}

fn wait_for_window(process_id: u32, timeout: Duration) -> Result<Hwnd, String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        let mut query = WindowQuery {
            process_id,
            hwnd: std::ptr::null_mut(),
        };
        unsafe {
            EnumWindows(
                enum_window,
                (&mut query as *mut WindowQuery).cast::<c_void>() as Lparam,
            );
        }
        if !query.hwnd.is_null() {
            return Ok(query.hwnd);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err("timed out waiting for Config window".to_string())
}

struct WindowQuery {
    process_id: u32,
    hwnd: Hwnd,
}

extern "system" fn enum_window(hwnd: Hwnd, lparam: Lparam) -> Bool {
    let query = unsafe { &mut *(lparam as *mut WindowQuery) };
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == query.process_id && IsWindowVisible(hwnd) != 0 {
            query.hwnd = hwnd;
            return 0;
        }
    }
    1
}

extern "system" fn print_child_window(hwnd: Hwnd, lparam: Lparam) -> Bool {
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }
    let context = unsafe { &mut *(lparam as *mut ChildPrintContext) };
    let mut child_rect = Rect::default();
    if unsafe { GetWindowRect(hwnd, &mut child_rect) } == 0 {
        return 1;
    }
    let saved = unsafe { SaveDC(context.memory_dc) };
    if saved != 0 {
        unsafe {
            SetViewportOrgEx(
                context.memory_dc,
                child_rect.left - context.parent_rect.left,
                child_rect.top - context.parent_rect.top,
                std::ptr::null_mut(),
            );
            SendMessageW(
                hwnd,
                WM_PRINT,
                context.memory_dc as Wparam,
                PRF_CHECKVISIBLE | PRF_NONCLIENT | PRF_CLIENT | PRF_ERASEBKGND | PRF_CHILDREN,
            );
            RestoreDC(context.memory_dc, saved);
        }
    }
    1
}

fn navigate(hwnd: Hwnd, control_id: i32) -> Result<(), String> {
    if unsafe { IsWindow(hwnd) } == 0 {
        return Err("Config window disappeared".to_string());
    }
    unsafe {
        SendMessageW(hwnd, WM_COMMAND, control_id as Wparam, 0);
    }
    Ok(())
}

fn verify_page(hwnd: Hwnd, page: &Page) -> Result<(), String> {
    let title = child_text(hwnd, K_PAGE_TITLE)?;
    if title.trim().is_empty() {
        return Err(format!("{} page title is empty", page.slug));
    }
    let mut rects = Vec::new();
    for &control in page.controls {
        let child = unsafe { GetDlgItem(hwnd, control) };
        if child.is_null() || unsafe { IsWindowVisible(child) } == 0 {
            continue;
        }
        let mut rect = Rect::default();
        if unsafe { GetWindowRect(child, &mut rect) } == 0 {
            return Err(format!("{} control {control} has no rect", page.slug));
        }
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return Err(format!("{} control {control} has empty rect", page.slug));
        }
        if is_keyboard_focus_control(control) && !has_tabstop(child) {
            return Err(format!(
                "{} control {control} is missing WS_TABSTOP keyboard focus style",
                page.slug
            ));
        }
        rects.push((control, rect));
    }
    for outer in 0..rects.len() {
        for inner in (outer + 1)..rects.len() {
            if intersects(rects[outer].1, rects[inner].1) {
                return Err(format!(
                    "{} controls {} and {} overlap",
                    page.slug, rects[outer].0, rects[inner].0
                ));
            }
        }
    }
    Ok(())
}

fn verify_resize_repaint(hwnd: Hwnd) -> Result<(), String> {
    // SAFETY: `hwnd` is the live top-level Settings window. NOZORDER/NOACTIVATE keep this QA
    // resize local to the test process and avoid stealing activation.
    if unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            980,
            560,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    } == 0
    {
        return Err("Config window resize for repaint QA failed".to_string());
    }
    thread::sleep(Duration::from_millis(150));
    let capture = capture_window(hwnd)?;
    verify_modern_settings_surface(
        &Page {
            id: K_NAV_GENERAL,
            slug: "resize-repaint",
            controls: &[],
        },
        image_stats(&capture),
    )
}

fn verify_modern_settings_surface(page: &Page, stats: ImageStats) -> Result<(), String> {
    if stats.settings_background_pixels < 512 {
        return Err(format!(
            "{} page did not repaint the modern Settings background surface",
            page.slug
        ));
    }
    if stats.settings_sidebar_pixels < 512 {
        return Err(format!(
            "{} page did not repaint the modern Settings sidebar surface",
            page.slug
        ));
    }
    if stats.settings_content_pixels < 512 {
        return Err(format!(
            "{} page did not repaint the modern Settings content card surface",
            page.slug
        ));
    }
    Ok(())
}

fn verify_no_legacy_static_ghosting(capture: &BitmapCapture) -> Result<(), String> {
    let stale_left_gutter = Rect {
        left: 220,
        top: 24,
        right: 247,
        bottom: 190,
    };
    let dark_pixels = dark_text_pixels_in_rect(capture, stale_left_gutter);
    if dark_pixels > 8 {
        return Err(format!(
            "appearance page left stale-text gutter contains {dark_pixels} dark pixels; likely transparent STATIC ghosting from the previous page"
        ));
    }
    Ok(())
}

fn dark_text_pixels_in_rect(capture: &BitmapCapture, rect: Rect) -> usize {
    let left = rect.left.clamp(0, capture.width);
    let top = rect.top.clamp(0, capture.height);
    let right = rect.right.clamp(left, capture.width);
    let bottom = rect.bottom.clamp(top, capture.height);
    let mut count = 0usize;
    for y in top..bottom {
        for x in left..right {
            let offset = ((y as usize) * (capture.width as usize) + x as usize) * 4;
            let b = capture.pixels[offset];
            let g = capture.pixels[offset + 1];
            let r = capture.pixels[offset + 2];
            if (28..=60).contains(&r) && (28..=62).contains(&g) && (30..=66).contains(&b) {
                count += 1;
            }
        }
    }
    count
}

fn is_keyboard_focus_control(control: i32) -> bool {
    matches!(
        control,
        K_INPUT_METHOD_LIST
            | K_LANGUAGE_SELECTOR
            | K_APPEARANCE_FONT_SIZE
            | K_APPEARANCE_OPACITY
            | K_APPEARANCE_FONT_FAMILY
            | K_APPEARANCE_SPACING
            | K_APPEARANCE_CORNER_RADIUS
            | K_APPEARANCE_CANDIDATE_WIDTH
            | K_PACKAGES
            | K_PACKAGE_INSTALL
            | K_PACKAGE_UPDATE
            | K_PACKAGE_REMOVE
            | K_PACKAGE_CONFIGURE
            | K_PACKAGE_REFRESH
            | K_PACKAGE_DETAILS
            | K_PACKAGE_ENABLE_DISABLE
            | K_PACKAGE_REPAIR
    )
}

fn has_tabstop(hwnd: Hwnd) -> bool {
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    style & WS_TABSTOP == WS_TABSTOP
}

fn child_text(hwnd: Hwnd, id: i32) -> Result<String, String> {
    let child = unsafe { GetDlgItem(hwnd, id) };
    if child.is_null() {
        return Ok(String::new());
    }
    let len = unsafe { SendMessageW(child, WM_GETTEXTLENGTH, 0, 0) as i32 };
    if len <= 0 {
        return Ok(String::new());
    }
    let mut buffer = vec![0u16; len as usize + 1];
    let copied = unsafe {
        SendMessageW(
            child,
            WM_GETTEXT,
            buffer.len() as Wparam,
            buffer.as_mut_ptr() as Lparam,
        ) as i32
    };
    if copied <= 0 {
        return Ok(String::new());
    }
    buffer.truncate(copied as usize);
    Ok(String::from_utf16_lossy(&buffer))
}

fn verify_appearance_numeric_inputs(hwnd: Hwnd) -> Result<(), String> {
    set_child_text(hwnd, K_APPEARANCE_FONT_SIZE, "20")?;
    notify_control_change(hwnd, K_APPEARANCE_FONT_SIZE);
    require_status_contains(hwnd, "font_size_dip accepted")?;

    set_child_text(hwnd, K_APPEARANCE_FONT_SIZE, "9999")?;
    notify_control_change(hwnd, K_APPEARANCE_FONT_SIZE);
    require_status_contains(hwnd, "appearance.numeric.out_of_range")?;

    set_child_text(hwnd, K_APPEARANCE_FONT_SIZE, "")?;
    notify_control_change(hwnd, K_APPEARANCE_FONT_SIZE);
    require_status_contains(hwnd, "appearance.numeric.incomplete")?;

    set_child_text(hwnd, K_APPEARANCE_OPACITY, "0.20")?;
    notify_control_change(hwnd, K_APPEARANCE_OPACITY);
    require_status_contains(hwnd, "opacity accepted")?;

    set_child_text(hwnd, K_APPEARANCE_OPACITY, "not-a-number")?;
    notify_control_change(hwnd, K_APPEARANCE_OPACITY);
    require_status_contains(hwnd, "appearance.numeric.invalid")?;

    set_child_text(hwnd, K_APPEARANCE_SPACING, "16")?;
    notify_control_change(hwnd, K_APPEARANCE_SPACING);
    require_status_contains(hwnd, "spacing_dip accepted")?;

    set_child_text(hwnd, K_APPEARANCE_CORNER_RADIUS, "49")?;
    notify_control_change(hwnd, K_APPEARANCE_CORNER_RADIUS);
    require_status_contains(hwnd, "appearance.numeric.out_of_range")?;

    set_child_text(hwnd, K_APPEARANCE_CANDIDATE_WIDTH, "160")?;
    notify_control_change(hwnd, K_APPEARANCE_CANDIDATE_WIDTH);
    require_status_contains(hwnd, "candidate_width_dip accepted")?;

    set_child_text(hwnd, K_APPEARANCE_FONT_SIZE, "18")?;
    notify_control_change(hwnd, K_APPEARANCE_FONT_SIZE);
    Ok(())
}

fn verify_enabled_input_method_list(hwnd: Hwnd) -> Result<(), String> {
    let listbox = unsafe { GetDlgItem(hwnd, K_INPUT_METHOD_LIST) };
    if listbox.is_null() {
        return Err("missing enabled input method list".to_string());
    }
    let count = unsafe { SendMessageW(listbox, LB_GETCOUNT, 0, 0) };
    if count < 2 {
        return Err(format!(
            "enabled input method list had {count} entries, expected multiple visible methods"
        ));
    }
    Ok(())
}

fn verify_language_selector(hwnd: Hwnd) -> Result<(), String> {
    let combo = unsafe { GetDlgItem(hwnd, K_LANGUAGE_SELECTOR) };
    if combo.is_null() {
        return Err("missing language selector".to_string());
    }
    let count = unsafe { SendMessageW(combo, CB_GETCOUNT, 0, 0) };
    if count < 3 {
        return Err(format!(
            "language selector had {count} entries, expected system/en-US/zh-CN"
        ));
    }
    let selected_result = unsafe { SendMessageW(combo, CB_SETCURSEL, 2, 0) };
    if selected_result < 0 {
        return Err("language selector rejected Simplified Chinese selection".to_string());
    }
    notify_combo_selection(hwnd, K_LANGUAGE_SELECTOR);
    require_status_contains(hwnd, "language accepted")?;
    Ok(())
}

fn verify_package_page(hwnd: Hwnd) -> Result<(), String> {
    let packages = unsafe { GetDlgItem(hwnd, K_PACKAGES) };
    if packages.is_null() {
        return Err("missing packages list".to_string());
    }
    let count = unsafe { SendMessageW(packages, LB_GETCOUNT, 0, 0) };
    if count < 3 {
        return Err(format!(
            "packages list had {count} entries, expected official package inventory"
        ));
    }
    for (button, expected) in [
        (K_PACKAGE_REFRESH, "package.refresh planned"),
        (K_PACKAGE_DETAILS, "package.details loaded"),
        (K_PACKAGE_INSTALL, "signed repository metadata required"),
        (K_PACKAGE_UPDATE, "package.update planned"),
        (K_PACKAGE_ENABLE_DISABLE, "package.enable_disable planned"),
        (K_PACKAGE_REMOVE, "package.remove planned"),
        (K_PACKAGE_CONFIGURE, "plugin_config loaded"),
        (K_PACKAGE_REPAIR, "package.repair planned"),
    ] {
        unsafe {
            SendMessageW(hwnd, WM_COMMAND, button as Wparam, 0);
        }
        require_status_contains(hwnd, expected)?;
    }
    if count > 1 {
        let selected_result = unsafe { SendMessageW(packages, LB_SETCURSEL, 1, 0) };
        if selected_result < 0 {
            return Err("packages list rejected selection change".to_string());
        }
        notify_listbox_selection(hwnd, K_PACKAGES);
        require_status_contains(hwnd, "package.selection changed")?;
        let detail = child_text(hwnd, K_PACKAGE_DETAIL)?;
        if !detail.contains("fcitx5-chinese-addons") || !detail.contains("refresh/details") {
            return Err(format!(
                "package details did not refresh from selected package: `{detail}`"
            ));
        }
    }
    Ok(())
}

fn verify_system_font_picker(hwnd: Hwnd) -> Result<String, String> {
    let combo = unsafe { GetDlgItem(hwnd, K_APPEARANCE_FONT_FAMILY) };
    if combo.is_null() {
        return Err("missing system font picker combobox".to_string());
    }
    let count = unsafe { SendMessageW(combo, CB_GETCOUNT, 0, 0) };
    if count <= 0 {
        return Err("system font picker did not expose current system fonts".to_string());
    }
    let selected = if count > 1 { 1 } else { 0 };
    let selected_result = unsafe { SendMessageW(combo, CB_SETCURSEL, selected as Wparam, 0) };
    if selected_result < 0 {
        return Err("system font picker rejected selection".to_string());
    }
    let selected_family = combo_selected_text(combo)?;
    notify_combo_selection(hwnd, K_APPEARANCE_FONT_FAMILY);
    require_status_contains(hwnd, "font_family accepted")?;
    Ok(selected_family)
}

fn verify_system_font_picker_persistence(
    config_exe: &Path,
    preview_state_path: &Path,
    expected_family: &str,
) -> Result<(), String> {
    let state = fs::read_to_string(preview_state_path).map_err(|error| {
        format!(
            "read persisted Rust Config font state {}: {error}",
            preview_state_path.display()
        )
    })?;
    if !state.contains(&format!(
        "font_family={}",
        expected_family.replace('\\', "\\\\").replace('\n', "\\n")
    )) {
        return Err(format!(
            "persisted font state `{state}` did not contain selected family `{expected_family}`"
        ));
    }
    let child = Command::new(config_exe)
        .env(PREVIEW_STATE_ENV, preview_state_path)
        .spawn()
        .map_err(|error| format!("relaunch {}: {error}", config_exe.display()))?;
    let guard = ProcessGuard { child };
    let hwnd = wait_for_window(guard.child.id(), Duration::from_secs(10))?;
    navigate(hwnd, K_NAV_APPEARANCE)?;
    thread::sleep(Duration::from_millis(200));
    let combo = unsafe { GetDlgItem(hwnd, K_APPEARANCE_FONT_FAMILY) };
    if combo.is_null() {
        return Err("reopened Rust Config is missing system font picker".to_string());
    }
    let reopened_family = combo_selected_text(combo)?;
    if !reopened_family.eq_ignore_ascii_case(expected_family) {
        return Err(format!(
            "reopened Rust Config selected `{reopened_family}` instead of persisted `{expected_family}`"
        ));
    }
    Ok(())
}

fn combo_selected_text(combo: Hwnd) -> Result<String, String> {
    let selected = unsafe { SendMessageW(combo, CB_GETCURSEL, 0, 0) };
    if selected < 0 {
        return Err("system font picker has no selected item".to_string());
    }
    let len = unsafe { SendMessageW(combo, CB_GETLBTEXTLEN, selected as Wparam, 0) };
    if len <= 0 {
        return Err("system font picker selected item has no text".to_string());
    }
    let mut buffer = vec![0u16; len as usize + 1];
    let copied = unsafe {
        SendMessageW(
            combo,
            CB_GETLBTEXT,
            selected as Wparam,
            buffer.as_mut_ptr() as Lparam,
        )
    };
    if copied <= 0 {
        return Err("system font picker selected text could not be read".to_string());
    }
    buffer.truncate(copied as usize);
    Ok(String::from_utf16_lossy(&buffer))
}

fn has_child(hwnd: Hwnd, id: i32) -> bool {
    !unsafe { GetDlgItem(hwnd, id) }.is_null()
}

fn rust_config_exe(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.eq_ignore_ascii_case("fcitx5-config"))
        .unwrap_or(false)
}

fn set_child_text(hwnd: Hwnd, id: i32, value: &str) -> Result<(), String> {
    let child = unsafe { GetDlgItem(hwnd, id) };
    if child.is_null() {
        return Err(format!("missing editable appearance control {id}"));
    }
    let text = to_wide(value);
    if unsafe { SendMessageW(child, WM_SETTEXT, 0, text.as_ptr() as Lparam) } == 0 {
        return Err(format!("set editable appearance control {id}"));
    }
    let updated = child_text(hwnd, id)?;
    if updated != value {
        return Err(format!(
            "editable appearance control {id} kept `{updated}` after setting `{value}`"
        ));
    }
    Ok(())
}

fn notify_control_change(hwnd: Hwnd, id: i32) {
    let wparam = ((EN_CHANGE & 0xffff) << 16) | ((id as Wparam) & 0xffff);
    unsafe {
        SendMessageW(hwnd, WM_COMMAND, wparam, 0);
    }
}

fn notify_combo_selection(hwnd: Hwnd, id: i32) {
    let wparam = ((CBN_SELCHANGE & 0xffff) << 16) | ((id as Wparam) & 0xffff);
    unsafe {
        SendMessageW(hwnd, WM_COMMAND, wparam, 0);
    }
}

fn notify_listbox_selection(hwnd: Hwnd, id: i32) {
    let wparam = ((LBN_SELCHANGE & 0xffff) << 16) | ((id as Wparam) & 0xffff);
    unsafe {
        SendMessageW(hwnd, WM_COMMAND, wparam, 0);
    }
}

fn require_status_contains(hwnd: Hwnd, expected: &str) -> Result<(), String> {
    let status = child_text(hwnd, K_SAVE_STATUS)?;
    if !status.contains(expected) {
        return Err(format!(
            "appearance numeric status `{status}` did not contain `{expected}`"
        ));
    }
    Ok(())
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn intersects(left: Rect, right: Rect) -> bool {
    left.left < right.right
        && left.right > right.left
        && left.top < right.bottom
        && left.bottom > right.top
}

fn capture_window(hwnd: Hwnd) -> Result<BitmapCapture, String> {
    capture_window_with_client_print(hwnd, false)
}

fn capture_window_from_screen(hwnd: Hwnd) -> Result<BitmapCapture, String> {
    let mut rect = Rect::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return Err("GetWindowRect failed".to_string());
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return Err("window has empty screen capture rect".to_string());
    }
    let screen_dc = unsafe { GetDC(std::ptr::null_mut()) };
    if screen_dc.is_null() {
        return Err("GetDC screen failed".to_string());
    }
    let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width, height) };
    if memory_dc.is_null() || bitmap.is_null() {
        unsafe {
            if !bitmap.is_null() {
                DeleteObject(bitmap.cast::<c_void>());
            }
            if !memory_dc.is_null() {
                DeleteDC(memory_dc);
            }
            ReleaseDC(std::ptr::null_mut(), screen_dc);
        }
        return Err("screen capture allocation failed".to_string());
    }
    let previous = unsafe { SelectObject(memory_dc, bitmap.cast::<c_void>()) };
    unsafe {
        BitBlt(
            memory_dc,
            0,
            0,
            width,
            height,
            screen_dc,
            rect.left,
            rect.top,
            SRCCOPY | CAPTUREBLT,
        );
        ReleaseDC(std::ptr::null_mut(), screen_dc);
    }
    let result = capture_bitmap(memory_dc, bitmap, width, height);
    unsafe {
        SelectObject(memory_dc, previous);
        DeleteObject(bitmap.cast::<c_void>());
        DeleteDC(memory_dc);
    }
    result
}

fn capture_window_with_client_print(
    hwnd: Hwnd,
    prefer_client_print: bool,
) -> Result<BitmapCapture, String> {
    let mut rect = Rect::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return Err("GetWindowRect failed".to_string());
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return Err("window has empty capture rect".to_string());
    }
    let window_dc = unsafe { GetWindowDC(hwnd) };
    if window_dc.is_null() {
        return Err("GetWindowDC failed".to_string());
    }
    let memory_dc = unsafe { CreateCompatibleDC(window_dc) };
    let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width, height) };
    if memory_dc.is_null() || bitmap.is_null() {
        unsafe {
            if !bitmap.is_null() {
                DeleteObject(bitmap.cast::<c_void>());
            }
            if !memory_dc.is_null() {
                DeleteDC(memory_dc);
            }
            ReleaseDC(hwnd, window_dc);
        }
        return Err("GDI capture allocation failed".to_string());
    }
    let previous = unsafe { SelectObject(memory_dc, bitmap.cast::<c_void>()) };
    let mut result = if prefer_client_print {
        unsafe {
            SendMessageW(hwnd, WM_PRINTCLIENT, memory_dc as Wparam, 0);
        }
        capture_bitmap(memory_dc, bitmap, width, height)
    } else {
        unsafe {
            SendMessageW(
                hwnd,
                WM_PRINT,
                memory_dc as Wparam,
                PRF_CHECKVISIBLE | PRF_NONCLIENT | PRF_CLIENT | PRF_ERASEBKGND | PRF_CHILDREN,
            );
            let mut child_context = ChildPrintContext {
                memory_dc,
                parent_rect: rect,
            };
            EnumChildWindows(
                hwnd,
                print_child_window,
                (&mut child_context as *mut ChildPrintContext) as Lparam,
            );
        }
        capture_bitmap(memory_dc, bitmap, width, height)
    };
    if result
        .as_ref()
        .map(|capture| image_stats(capture).non_background_pixels < 64)
        .unwrap_or(true)
    {
        let printed = unsafe { PrintWindow(hwnd, memory_dc, PW_RENDERFULLCONTENT) };
        if printed == 0 {
            unsafe {
                BitBlt(memory_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY);
            }
        }
        result = capture_bitmap(memory_dc, bitmap, width, height);
    }
    if result
        .as_ref()
        .map(|capture| image_stats(capture).non_background_pixels < 64)
        .unwrap_or(false)
    {
        unsafe {
            BitBlt(memory_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY);
        }
        result = capture_bitmap(memory_dc, bitmap, width, height);
    }
    let needs_screen_fallback = prefer_client_print
        && result
            .as_ref()
            .map(|capture| image_stats(capture).non_background_pixels < 64)
            .unwrap_or(true);
    let screen_dc = if needs_screen_fallback {
        unsafe { GetDC(std::ptr::null_mut()) }
    } else {
        std::ptr::null_mut()
    };
    if !screen_dc.is_null() {
        unsafe {
            BitBlt(
                memory_dc,
                0,
                0,
                width,
                height,
                screen_dc,
                rect.left,
                rect.top,
                SRCCOPY | CAPTUREBLT,
            );
            ReleaseDC(std::ptr::null_mut(), screen_dc);
        }
        result = capture_bitmap(memory_dc, bitmap, width, height);
    }
    unsafe {
        SelectObject(memory_dc, previous);
        DeleteObject(bitmap.cast::<c_void>());
        DeleteDC(memory_dc);
        ReleaseDC(hwnd, window_dc);
    }
    result
}

fn capture_candidate_reference(candidate_ui: &Path, out_dir: &Path) -> Result<ImageStats, String> {
    let child = Command::new(candidate_ui)
        .arg("--demo")
        .spawn()
        .map_err(|error| format!("launch {} --demo: {error}", candidate_ui.display()))?;
    let guard = ProcessGuard { child };
    let hwnd = wait_for_window(guard.child.id(), Duration::from_secs(10))?;
    thread::sleep(Duration::from_millis(300));
    let capture = capture_window_with_client_print(hwnd, true)?;
    write_bitmap(&capture, &out_dir.join("candidate-ui-demo-reference.bmp"))?;
    unsafe {
        SendMessageW(hwnd, WM_CLOSE, 0, 0);
    }
    let stats = image_stats(&capture);
    if stats.non_background_pixels < 64 {
        return Err(
            "candidate UI reference screenshot did not contain visible content".to_string(),
        );
    }
    Ok(stats)
}

fn crop_config_preview(hwnd: Hwnd, capture: &BitmapCapture) -> Result<BitmapCapture, String> {
    // SAFETY: hwnd is the live Settings top-level window discovered from the
    // launched process; K_PREVIEW is an in-process child control id.
    let preview_child = unsafe { GetDlgItem(hwnd, K_PREVIEW) };
    // SAFETY: IsWindowVisible only reads the HWND state. Null is checked first.
    let selected = if !preview_child.is_null() && unsafe { IsWindowVisible(preview_child) } != 0 {
        let mut window_rect = Rect::default();
        // SAFETY: window_rect points to valid writable stack storage.
        if unsafe { GetWindowRect(hwnd, &mut window_rect) } == 0 {
            return Err("Config window rect unavailable for preview crop".to_string());
        }
        let mut preview_rect = Rect::default();
        // SAFETY: preview_rect points to valid writable stack storage and
        // preview_child was validated above.
        if unsafe { GetWindowRect(preview_child, &mut preview_rect) } == 0 {
            return Err("Config preview control rect unavailable".to_string());
        }
        selected_theme_accent_bbox(capture).unwrap_or(Rect {
            left: preview_rect.left - window_rect.left,
            top: preview_rect.top - window_rect.top,
            right: preview_rect.right - window_rect.left,
            bottom: preview_rect.bottom - window_rect.top,
        })
    } else {
        selected_theme_accent_bbox(capture)
            .ok_or("Config appearance screenshot did not contain candidate preview accent")?
    };
    let preview = Rect {
        left: selected.left - 64,
        top: selected.top - 48,
        right: selected.right + 520,
        bottom: selected.bottom + 128,
    };
    crop_bitmap(capture, preview)
}

fn preview_region_has_candidate_accent(
    hwnd: Hwnd,
    page: &Page,
    capture: &BitmapCapture,
) -> Result<bool, String> {
    let preview_child = unsafe { GetDlgItem(hwnd, K_PREVIEW) };
    if preview_child.is_null() {
        return Ok(false);
    }
    let mut window_rect = Rect::default();
    // SAFETY: `window_rect` points to valid writable stack storage and `hwnd` is a live top-level
    // Settings window discovered from the launched process.
    if unsafe { GetWindowRect(hwnd, &mut window_rect) } == 0 {
        return Err("Config window rect unavailable for stale preview check".to_string());
    }
    let mut preview_rect = Rect::default();
    // SAFETY: `preview_rect` points to valid writable stack storage and `preview_child` is a child
    // HWND returned by GetDlgItem.
    if unsafe { GetWindowRect(preview_child, &mut preview_rect) } == 0 {
        return Err("Config preview control rect unavailable for stale preview check".to_string());
    }
    let preview = Rect {
        left: preview_rect.left - window_rect.left,
        top: preview_rect.top - window_rect.top,
        right: preview_rect.right - window_rect.left,
        bottom: preview_rect.bottom - window_rect.top,
    };
    for &control in page.controls {
        if control == K_PREVIEW {
            continue;
        }
        let child = unsafe { GetDlgItem(hwnd, control) };
        if child.is_null() || unsafe { IsWindowVisible(child) } == 0 {
            continue;
        }
        let mut child_rect = Rect::default();
        // SAFETY: `child_rect` points to valid writable storage and `child` is a visible child
        // handle returned by GetDlgItem.
        if unsafe { GetWindowRect(child, &mut child_rect) } == 0 {
            continue;
        }
        let child_capture_rect = Rect {
            left: child_rect.left - window_rect.left,
            top: child_rect.top - window_rect.top,
            right: child_rect.right - window_rect.left,
            bottom: child_rect.bottom - window_rect.top,
        };
        if intersects(preview, child_capture_rect) {
            return Ok(false);
        }
    }
    let crop = crop_bitmap(capture, preview)?;
    Ok(image_stats(&crop).selected_accent_pixels >= 8)
}

fn selected_theme_accent_bbox(capture: &BitmapCapture) -> Option<Rect> {
    let mut bbox: Option<Rect> = None;
    for y in 0..capture.height {
        for x in 0..capture.width {
            // Ignore the left navigation accent and the lower Appearance page controls; the
            // embedded Candidate preview lives in the upper main content surface.
            if x < 400 || y > 520 {
                continue;
            }
            let offset = ((y as usize) * (capture.width as usize) + x as usize) * 4;
            let b = capture.pixels[offset];
            let g = capture.pixels[offset + 1];
            let r = capture.pixels[offset + 2];
            let green_selected = r <= 32 && (145..=190).contains(&g) && (85..=135).contains(&b);
            let blue_selected =
                (170..=230).contains(&r) && (190..=230).contains(&g) && (220..=255).contains(&b);
            if green_selected || blue_selected {
                bbox = Some(match bbox {
                    Some(rect) => Rect {
                        left: rect.left.min(x),
                        top: rect.top.min(y),
                        right: rect.right.max(x + 1),
                        bottom: rect.bottom.max(y + 1),
                    },
                    None => Rect {
                        left: x,
                        top: y,
                        right: x + 1,
                        bottom: y + 1,
                    },
                });
            }
        }
    }
    bbox
}

fn crop_bitmap(capture: &BitmapCapture, rect: Rect) -> Result<BitmapCapture, String> {
    let left = rect.left.clamp(0, capture.width);
    let top = rect.top.clamp(0, capture.height);
    let right = rect.right.clamp(left, capture.width);
    let bottom = rect.bottom.clamp(top, capture.height);
    let width = right - left;
    let height = bottom - top;
    if width <= 0 || height <= 0 {
        return Err("Config preview crop has empty bounds".to_string());
    }
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    for row in 0..height {
        let source = (((top + row) as usize) * (capture.width as usize) + left as usize) * 4;
        let destination = (row as usize) * (width as usize) * 4;
        let bytes = (width as usize) * 4;
        pixels[destination..destination + bytes]
            .copy_from_slice(&capture.pixels[source..source + bytes]);
    }
    Ok(BitmapCapture {
        width,
        height,
        pixels,
    })
}

fn image_stats(capture: &BitmapCapture) -> ImageStats {
    let mut stats = ImageStats {
        bytes: 14 + 40 + capture.pixels.len(),
        checksum: 1469598103934665603,
        ..ImageStats::default()
    };
    let (pixels, remainder) = capture.pixels.as_chunks::<4>();
    debug_assert!(remainder.is_empty());
    let background = pixels.first().unwrap_or(&[0, 0, 0, 0]);
    for pixel in pixels {
        let b = pixel[0];
        let g = pixel[1];
        let r = pixel[2];
        if pixel != background {
            stats.non_background_pixels += 1;
        }
        let value = u32::from_le_bytes([pixel[0], pixel[1], pixel[2], pixel[3]]);
        stats.checksum ^= u64::from(value);
        stats.checksum = stats.checksum.wrapping_mul(1099511628211);
        let selected_green = r <= 32 && (145..=190).contains(&g) && (85..=135).contains(&b);
        let selected_blue =
            (170..=230).contains(&r) && (190..=230).contains(&g) && (220..=255).contains(&b);
        if selected_green {
            stats.selected_green_pixels += 1;
        }
        if selected_blue {
            stats.selected_blue_pixels += 1;
        }
        if selected_green || selected_blue {
            stats.selected_accent_pixels += 1;
        }
        if r >= 245 && g >= 245 && b >= 245 {
            stats.white_surface_pixels += 1;
        }
        if (30..=46).contains(&r) && (30..=48).contains(&g) && (34..=54).contains(&b) {
            stats.dark_surface_pixels += 1;
        }
        if (28..=60).contains(&r) && (28..=62).contains(&g) && (30..=66).contains(&b) {
            stats.dark_text_pixels += 1;
        }
        if close_rgb(r, g, b, COLOR_SETTINGS_BACKGROUND, 2) {
            stats.settings_background_pixels += 1;
        }
        if close_rgb(r, g, b, COLOR_SETTINGS_SIDEBAR, 2) {
            stats.settings_sidebar_pixels += 1;
        }
        if close_rgb(r, g, b, COLOR_SETTINGS_CONTENT, 0) {
            stats.settings_content_pixels += 1;
        }
    }
    stats.shared_theme_pixels = stats.selected_accent_pixels
        + stats.white_surface_pixels
        + stats.dark_surface_pixels
        + stats.dark_text_pixels;
    stats
}

fn close_rgb(r: u8, g: u8, b: u8, expected: (u8, u8, u8), tolerance: u8) -> bool {
    r.abs_diff(expected.0) <= tolerance
        && g.abs_diff(expected.1) <= tolerance
        && b.abs_diff(expected.2) <= tolerance
}

fn assert_preview_matches_candidate_theme(
    preview: ImageStats,
    reference: ImageStats,
) -> Result<(), String> {
    if reference.selected_green_pixels >= 20 && preview.selected_accent_pixels < 10 {
        return Err(format!(
            "Config preview crop did not contain a candidate selected-background accent: reference={}, preview_green={}, preview_blue={}",
            reference.selected_green_pixels, preview.selected_green_pixels, preview.selected_blue_pixels
        ));
    }
    if reference.white_surface_pixels >= 20 && preview.white_surface_pixels < 20 {
        return Err(format!(
            "Config preview crop did not contain the candidate light surface theme color: reference={}, preview={}",
            reference.white_surface_pixels, preview.white_surface_pixels
        ));
    }
    if reference.dark_surface_pixels >= 20 && preview.dark_surface_pixels < 20 {
        return Err(format!(
            "Config preview crop did not contain the candidate dark surface theme color: reference={}, preview={}",
            reference.dark_surface_pixels, preview.dark_surface_pixels
        ));
    }
    if preview.shared_theme_pixels < 64 {
        return Err(format!(
            "Config preview crop did not share enough theme-colored pixels with candidate UI: reference={}, preview={}",
            reference.shared_theme_pixels, preview.shared_theme_pixels
        ));
    }
    if preview.bytes == 0 || preview.checksum == 0 {
        return Err("Config preview crop evidence is empty".to_string());
    }
    Ok(())
}

fn capture_bitmap(
    hdc: Hdc,
    bitmap: Hbitmap,
    width: i32,
    height: i32,
) -> Result<BitmapCapture, String> {
    let stride = (width as usize) * 4;
    let mut pixels = vec![0u8; stride * (height as usize)];
    let mut info = BitmapInfo {
        bmi_header: BitmapInfoHeader {
            bi_size: std::mem::size_of::<BitmapInfoHeader>() as Dword,
            bi_width: width,
            bi_height: -height,
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: BI_RGB,
            bi_size_image: pixels.len() as Dword,
            bi_x_pels_per_meter: 0,
            bi_y_pels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        },
        bmi_colors: [0],
    };
    let lines = unsafe {
        GetDIBits(
            hdc,
            bitmap,
            0,
            height as Uint,
            pixels.as_mut_ptr().cast::<c_void>(),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    if lines != height {
        return Err("GetDIBits failed".to_string());
    }
    Ok(BitmapCapture {
        width,
        height,
        pixels,
    })
}

fn write_bitmap(capture: &BitmapCapture, path: &Path) -> Result<(), String> {
    let pixel_offset = 14u32 + 40u32;
    let file_size = pixel_offset + capture.pixels.len() as u32;
    let mut file = fs::File::create(path).map_err(|error| format!("create bitmap: {error}"))?;
    file.write_all(b"BM").map_err(|error| error.to_string())?;
    file.write_all(&file_size.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&[0; 4]).map_err(|error| error.to_string())?;
    file.write_all(&pixel_offset.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(40u32).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&capture.width.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(-capture.height).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(1u16).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(32u16).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&BI_RGB.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(capture.pixels.len() as u32).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&[0; 16])
        .map_err(|error| error.to_string())?;
    file.write_all(&capture.pixels)
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_are_unique() {
        let mut ids = PAGES.iter().map(|page| page.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), PAGES.len());
    }
}
