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
const PW_RENDERFULLCONTENT: Uint = 0x0000_0002;
const CAPTUREBLT: Dword = 0x4000_0000;
const SRCCOPY: Dword = 0x00CC_0020;
const WM_COMMAND: Uint = 0x0111;
const WM_CLOSE: Uint = 0x0010;
const WM_PRINT: Uint = 0x0317;
const WM_PRINTCLIENT: Uint = 0x0318;
const PRF_CHECKVISIBLE: Lparam = 0x0000_0001;
const PRF_NONCLIENT: Lparam = 0x0000_0002;
const PRF_CLIENT: Lparam = 0x0000_0004;
const PRF_ERASEBKGND: Lparam = 0x0000_0008;
const PRF_CHILDREN: Lparam = 0x0000_0010;

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
const K_SAVE_STATUS: i32 = 206;

const PAGES: &[Page] = &[
    Page {
        id: K_NAV_GENERAL,
        slug: "input-methods",
        controls: &[K_PAGE_TITLE, K_SAVE_STATUS],
    },
    Page {
        id: K_NAV_APPEARANCE,
        slug: "appearance",
        controls: &[K_PAGE_TITLE, K_PREVIEW, K_SAVE_STATUS],
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
        controls: &[K_PAGE_TITLE, K_PACKAGES, K_PACKAGE_DETAIL, K_STATUS],
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
    fn GetWindowDC(hwnd: Hwnd) -> Hdc;
    fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> Bool;
    fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
    fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: i32) -> i32;
    fn GetWindowThreadProcessId(hwnd: Hwnd, process_id: *mut Dword) -> Dword;
    fn IsWindow(hwnd: Hwnd) -> Bool;
    fn IsWindowVisible(hwnd: Hwnd) -> Bool;
    fn PrintWindow(hwnd: Hwnd, hdc: Hdc, flags: Uint) -> Bool;
    fn ReleaseDC(hwnd: Hwnd, hdc: Hdc) -> i32;
    fn SendMessageW(hwnd: Hwnd, msg: Uint, wparam: Wparam, lparam: Lparam) -> Lresult;
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
    let child = Command::new(&args.config_exe)
        .spawn()
        .map_err(|error| format!("launch {}: {error}", args.config_exe.display()))?;
    let guard = ProcessGuard { child };
    let hwnd = wait_for_window(guard.child.id(), Duration::from_secs(10))?;
    let mut report = String::new();
    report.push_str("# Config UI QA\n\n");
    report.push_str(&format!("- exe: `{}`\n", args.config_exe.display()));
    report.push_str(&format!("- pid: `{}`\n\n", guard.child.id()));
    report.push_str("| Page | Result | Screenshot |\n|---|---|---|\n");
    for page in PAGES {
        navigate(hwnd, page.id)?;
        thread::sleep(Duration::from_millis(200));
        verify_page(hwnd, page)?;
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
        }
        report.push_str(&format!("| {} | ok | `{}` |\n", page.slug, file_name));
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

fn child_text(hwnd: Hwnd, id: i32) -> Result<String, String> {
    let child = unsafe { GetDlgItem(hwnd, id) };
    if child.is_null() {
        return Ok(String::new());
    }
    let len = unsafe { GetWindowTextLengthW(child) };
    if len <= 0 {
        return Ok(String::new());
    }
    let mut buffer = vec![0u16; len as usize + 1];
    let copied = unsafe { GetWindowTextW(child, buffer.as_mut_ptr(), buffer.len() as i32) };
    if copied <= 0 {
        return Ok(String::new());
    }
    buffer.truncate(copied as usize);
    Ok(String::from_utf16_lossy(&buffer))
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
    }
    stats.shared_theme_pixels = stats.selected_accent_pixels
        + stats.white_surface_pixels
        + stats.dark_surface_pixels
        + stats.dark_text_pixels;
    stats
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
