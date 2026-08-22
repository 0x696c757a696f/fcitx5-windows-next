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
const SRCCOPY: Dword = 0x00CC_0020;
const WM_COMMAND: Uint = 0x0111;

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
    fn EnumWindows(callback: extern "system" fn(Hwnd, Lparam) -> Bool, lparam: Lparam) -> Bool;
    fn GetDlgItem(hwnd: Hwnd, id: i32) -> Hwnd;
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
    fn SelectObject(hdc: Hdc, object: Hgdobj) -> Hgdobj;
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
    let child = Command::new(&args.config_exe)
        .spawn()
        .map_err(|error| format!("launch {}: {error}", args.config_exe.display()))?;
    let hwnd = wait_for_window(child.id(), Duration::from_secs(10))?;
    let mut report = String::new();
    report.push_str("# Config UI QA\n\n");
    report.push_str(&format!("- exe: `{}`\n", args.config_exe.display()));
    report.push_str(&format!("- pid: `{}`\n\n", child.id()));
    report.push_str("| Page | Result | Screenshot |\n|---|---|---|\n");
    for page in PAGES {
        navigate(hwnd, page.id)?;
        thread::sleep(Duration::from_millis(200));
        verify_page(hwnd, page)?;
        let file_name = format!("config-{}.bmp", page.slug);
        capture_window(hwnd, &args.out_dir.join(&file_name))?;
        report.push_str(&format!("| {} | ok | `{}` |\n", page.slug, file_name));
    }
    fs::write(args.out_dir.join("config-ui-qa.md"), report)
        .map_err(|error| format!("write report: {error}"))?;
    let _guard = ProcessGuard { child };
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut values = env::args_os().skip(1);
    let mut config_exe = None;
    let mut out_dir = None;
    while let Some(arg) = values.next() {
        if arg == "--config-exe" {
            config_exe = values.next().map(PathBuf::from);
        } else if arg == "--out" {
            out_dir = values.next().map(PathBuf::from);
        } else {
            return Err(format!("unknown argument: {}", arg.to_string_lossy()));
        }
    }
    Ok(Args {
        config_exe: config_exe.ok_or("missing --config-exe")?,
        out_dir: out_dir.ok_or("missing --out")?,
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

fn capture_window(hwnd: Hwnd, path: &Path) -> Result<(), String> {
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
    let printed = unsafe { PrintWindow(hwnd, memory_dc, PW_RENDERFULLCONTENT) };
    if printed == 0 {
        unsafe {
            BitBlt(memory_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY);
        }
    }
    let result = write_bitmap(memory_dc, bitmap, width, height, path);
    unsafe {
        SelectObject(memory_dc, previous);
        DeleteObject(bitmap.cast::<c_void>());
        DeleteDC(memory_dc);
        ReleaseDC(hwnd, window_dc);
    }
    result
}

fn write_bitmap(
    hdc: Hdc,
    bitmap: Hbitmap,
    width: i32,
    height: i32,
    path: &Path,
) -> Result<(), String> {
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
    let pixel_offset = 14u32 + 40u32;
    let file_size = pixel_offset + pixels.len() as u32;
    let mut file = fs::File::create(path).map_err(|error| format!("create bitmap: {error}"))?;
    file.write_all(b"BM").map_err(|error| error.to_string())?;
    file.write_all(&file_size.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&[0; 4]).map_err(|error| error.to_string())?;
    file.write_all(&pixel_offset.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(40u32).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&width.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(-height).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(1u16).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(32u16).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&BI_RGB.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(pixels.len() as u32).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&[0; 16])
        .map_err(|error| error.to_string())?;
    file.write_all(&pixels).map_err(|error| error.to_string())?;
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
