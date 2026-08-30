#![deny(unsafe_op_in_unsafe_fn)]
//! Narrow Win32 window, GDI, and Control-ABI adapter for the Rust Settings shell.

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicI32, AtomicPtr, AtomicUsize, Ordering};

use super::{
    candidate_preview_paint_plan, design_tokens, settings_surface_paint_plan,
    validate_appearance_numeric_input, AppearanceNumericField, PageId, Rect as LayoutRect, Size,
    WindowSmokeEvidence,
};

type Hinstance = *mut c_void;
type Hwnd = *mut c_void;
type Hicon = *mut c_void;
type Hcursor = *mut c_void;
type Hbrush = *mut c_void;
type Hfont = *mut c_void;
type HgdiObj = *mut c_void;
type Lpcwstr = *const u16;
type Lparam = isize;
type Lresult = isize;
type Wparam = usize;
type Hdc = *mut c_void;

const CS_HREDRAW: u32 = 0x0002;
const CS_VREDRAW: u32 = 0x0001;
const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
const DT_LEFT: u32 = 0x0000;
const DT_SINGLELINE: u32 = 0x0020;
const DT_VCENTER: u32 = 0x0004;
const CBN_SELCHANGE: u16 = 1;
const CB_ADDSTRING: u32 = 0x0143;
const CB_GETCURSEL: u32 = 0x0147;
const CB_GETLBTEXT: u32 = 0x0148;
const CB_GETLBTEXTLEN: u32 = 0x0149;
const CB_SETCURSEL: u32 = 0x014E;
const CBS_DROPDOWNLIST: u32 = 0x0003;
const CBS_HASSTRINGS: u32 = 0x0200;
const EN_CHANGE: u16 = 0x0300;
const ES_AUTOHSCROLL: u32 = 0x0080;
const FALSE: i32 = 0;
const TRUE: i32 = 1;
const LBN_SELCHANGE: u16 = 1;
const LB_ADDSTRING: u32 = 0x0180;
const LB_SETCURSEL: u32 = 0x0186;
const LB_GETCURSEL: u32 = 0x0188;
const LB_GETTEXT: u32 = 0x0189;
const LB_GETTEXTLEN: u32 = 0x018A;
const TRANSPARENT: i32 = 1;
const OPAQUE: i32 = 2;
const WM_CTLCOLORSTATIC: u32 = 0x0138;
const WM_CLOSE: u32 = 0x0010;
const WM_COMMAND: u32 = 0x0111;
const WM_DESTROY: u32 = 0x0002;
const WM_DRAWITEM: u32 = 0x002B;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_PAINT: u32 = 0x000F;
const WM_PRINTCLIENT: u32 = 0x0318;
const WM_SETFONT: u32 = 0x0030;
const WM_SIZE: u32 = 0x0005;
const WS_BORDER: u32 = 0x0080_0000;
const BS_FLAT: u32 = 0x8000;
const BS_OWNERDRAW: u32 = 0x000B;
const WS_CHILD: u32 = 0x4000_0000;
const WS_CLIPCHILDREN: u32 = 0x0200_0000;
const WS_OVERLAPPEDWINDOW: u32 = 0x00cf_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WS_VSCROLL: u32 = 0x0020_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const SW_HIDE: i32 = 0;
const SW_SHOWNORMAL: i32 = 1;
const SW_SHOW: i32 = 5;
const GET_PIXEL_ERROR: u32 = 0xffff_ffff;
const K_STATUS: i32 = 110;
const K_PREVIEW: i32 = 112;
const K_PACKAGES: i32 = 113;
const K_PACKAGE_DETAIL: i32 = 127;
const K_NAV_GENERAL: i32 = 130;
const K_NAV_APPEARANCE: i32 = 131;
const K_NAV_SHORTCUTS: i32 = 132;
const K_NAV_UPDATES: i32 = 133;
const K_NAV_REPAIR: i32 = 134;
const K_NAV_PACKAGES: i32 = 135;
const K_PAGE_TITLE: i32 = 140;
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
const PREVIEW_STATE_ENV: &str = super::CONFIG_QA_PREVIEW_STATE_ENV;
static PREVIEW_PAINT_COUNT: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_NAV_PAGE: AtomicI32 = AtomicI32::new(K_NAV_GENERAL);
static SETTINGS_UI_FONT: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static SETTINGS_TITLE_FONT: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static SETTINGS_HEADER_BRUSH: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static SETTINGS_CONTENT_BRUSH: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

#[repr(C)]
struct WndClassW {
    style: u32,
    lpfn_wnd_proc: Option<unsafe extern "system" fn(Hwnd, u32, Wparam, Lparam) -> Lresult>,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: Hinstance,
    h_icon: Hicon,
    h_cursor: Hcursor,
    hbr_background: Hbrush,
    lpsz_menu_name: Lpcwstr,
    lpsz_class_name: Lpcwstr,
}

#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Rect {
    fn width(&self) -> i32 {
        self.right - self.left
    }

    fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[repr(C)]
struct PaintStruct {
    hdc: Hdc,
    f_erase: i32,
    rc_paint: Rect,
    f_restore: i32,
    f_inc_update: i32,
    rgb_reserved: [u8; 32],
}

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Msg {
    hwnd: Hwnd,
    message: u32,
    w_param: Wparam,
    l_param: Lparam,
    time: u32,
    pt: Point,
}

#[repr(C)]
struct DrawItemStruct {
    ctl_type: u32,
    ctl_id: u32,
    item_id: u32,
    item_action: u32,
    item_state: u32,
    hwnd_item: Hwnd,
    hdc: Hdc,
    rc_item: Rect,
    item_data: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ControlUtf16 {
    ptr: *const u16,
    len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ControlUtf8 {
    ptr: *const u8,
    len: usize,
}

// SAFETY: These declarations bind synchronous User32 entry points. Every call below proves
// its handle, pointer, and buffer-lifetime preconditions immediately at the call site.
#[link(name = "user32")]
unsafe extern "system" {
    fn BeginPaint(hwnd: Hwnd, paint: *mut PaintStruct) -> Hdc;
    fn RegisterClassW(window_class: *const WndClassW) -> u16;
    fn CreateWindowExW(
        ex_style: u32,
        class_name: Lpcwstr,
        window_name: Lpcwstr,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: *mut c_void,
        instance: Hinstance,
        param: *mut c_void,
    ) -> Hwnd;
    fn DefWindowProcW(hwnd: Hwnd, message: u32, wparam: Wparam, lparam: Lparam) -> Lresult;
    fn DestroyWindow(hwnd: Hwnd) -> i32;
    fn DispatchMessageW(message: *const Msg) -> Lresult;
    fn DrawTextW(hdc: Hdc, text: *const u16, count: i32, rect: *mut Rect, format: u32) -> i32;
    fn EndPaint(hwnd: Hwnd, paint: *const PaintStruct) -> i32;
    fn GetClientRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
    fn GetDC(hwnd: Hwnd) -> Hdc;
    fn GetDlgCtrlID(hwnd: Hwnd) -> i32;
    fn GetDlgItem(hwnd: Hwnd, item_id: i32) -> Hwnd;
    fn GetMessageW(message: *mut Msg, hwnd: Hwnd, min_filter: u32, max_filter: u32) -> i32;
    fn GetParent(hwnd: Hwnd) -> Hwnd;
    fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
    fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
    fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: i32) -> i32;
    fn InvalidateRect(hwnd: Hwnd, rect: *const Rect, erase: i32) -> i32;
    fn IsWindowVisible(hwnd: Hwnd) -> i32;
    fn PostQuitMessage(exit_code: i32);
    fn ReleaseDC(hwnd: Hwnd, dc: Hdc) -> i32;
    fn SendMessageW(hwnd: Hwnd, message: u32, wparam: Wparam, lparam: Lparam) -> Lresult;
    fn SetWindowTextW(hwnd: Hwnd, text: Lpcwstr) -> i32;
    fn ShowWindow(hwnd: Hwnd, command_show: i32) -> i32;
    fn TranslateMessage(message: *const Msg) -> i32;
    fn UpdateWindow(hwnd: Hwnd) -> i32;
}

// SAFETY: These declarations bind process-local GDI operations. Calls retain and release GDI
// objects according to the documented paint-cycle ownership rules.
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike_out: u32,
        char_set: u32,
        output_precision: u32,
        clip_precision: u32,
        quality: u32,
        pitch_and_family: u32,
        face_name: Lpcwstr,
    ) -> Hfont;
    fn CreateSolidBrush(color: u32) -> Hbrush;
    fn DeleteObject(object: *mut c_void) -> i32;
    fn FillRect(hdc: Hdc, rect: *const Rect, brush: Hbrush) -> i32;
    fn GetPixel(hdc: Hdc, x: i32, y: i32) -> u32;
    fn SelectObject(hdc: Hdc, object: HgdiObj) -> HgdiObj;
    fn SetBkColor(hdc: Hdc, color: u32) -> u32;
    fn SetBkMode(hdc: Hdc, mode: i32) -> i32;
    fn SetTextColor(hdc: Hdc, color: u32) -> u32;
}

// SAFETY: This declaration has no retained Rust pointers; the null module argument is checked.
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(module_name: Lpcwstr) -> Hinstance;
}

// SAFETY: This narrow Control ABI accepts borrowed UTF-16/UTF-8 slices only for the synchronous
// call. The caller keeps both buffers alive and checks the status before returning.
unsafe extern "C" {
    fn fcitx5_control_atomic_write_utf8_file_utf16(
        destination: ControlUtf16,
        content: ControlUtf8,
    ) -> i32;
}

pub fn create(
    title: &str,
    minimum_window_dip: Size,
    candidate_preview_rect: LayoutRect,
) -> Result<WindowSmokeEvidence, String> {
    let class_name = to_wide("Fcitx5ConfigPocWindow");
    let preview_class_name = to_wide("Fcitx5ConfigPocCandidatePreviewHost");
    let title = to_wide(title);
    let preview_title = to_wide("Candidate Preview");
    // SAFETY: A null module name asks Windows for the current process module handle.
    let instance = unsafe { GetModuleHandleW(null()) };
    if instance.is_null() {
        return Err("GetModuleHandleW failed for Rust Config PoC".to_owned());
    }
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
    let preview_window_class = WndClassW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfn_wnd_proc: Some(candidate_preview_window_proc),
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: instance,
        h_icon: null_mut(),
        h_cursor: null_mut(),
        hbr_background: null_mut(),
        lpsz_menu_name: null(),
        lpsz_class_name: preview_class_name.as_ptr(),
    };
    // SAFETY: The class descriptors reference live UTF-16 buffers for this call and use a
    // window procedure with the expected system ABI.
    let atom = unsafe { RegisterClassW(&window_class) };
    if atom == 0 {
        return Err("RegisterClassW failed for Rust Config PoC".to_owned());
    }
    // SAFETY: The class descriptor references live UTF-16 buffers for this call and uses a
    // window procedure with the expected system ABI.
    let preview_atom = unsafe { RegisterClassW(&preview_window_class) };
    if preview_atom == 0 {
        return Err("RegisterClassW failed for Rust Config PoC candidate preview host".to_owned());
    }
    // SAFETY: All UTF-16 class/title pointers stay alive for the duration of this call. Parent,
    // menu, and parameter handles are null because this creates the top-level smoke window.
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            minimum_window_dip.width,
            minimum_window_dip.height,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        )
    };
    if hwnd.is_null() {
        return Err("CreateWindowExW failed for Rust Config PoC".to_owned());
    }
    // SAFETY: The preview class/title pointers stay alive for the call, and `hwnd` is a live
    // top-level window handle created above. The child coordinates come from the validated
    // layout model.
    let preview_hwnd = unsafe {
        CreateWindowExW(
            0,
            preview_class_name.as_ptr(),
            preview_title.as_ptr(),
            WS_CHILD | WS_VISIBLE,
            candidate_preview_rect.x,
            candidate_preview_rect.y,
            candidate_preview_rect.width,
            candidate_preview_rect.height,
            hwnd,
            control_id_handle(K_PREVIEW),
            instance,
            null_mut(),
        )
    };
    if preview_hwnd.is_null() {
        // SAFETY: `hwnd` is a live window handle created above and is being cleaned up on the
        // failure path.
        unsafe {
            DestroyWindow(hwnd);
        }
        return Err("CreateWindowExW failed for Rust Config PoC candidate preview host".to_owned());
    }
    PREVIEW_PAINT_COUNT.store(0, Ordering::SeqCst);
    // SAFETY: Both handles were created successfully and can be shown/painted immediately.
    unsafe {
        ShowWindow(hwnd, SW_SHOWNORMAL);
        ShowWindow(preview_hwnd, SW_SHOWNORMAL);
        InvalidateRect(preview_hwnd, null(), FALSE);
        UpdateWindow(hwnd);
        UpdateWindow(preview_hwnd);
    }
    let mut rect = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut preview_rect = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: `hwnd` is a live window handle and `rect` points to writable memory.
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        // SAFETY: Handles were created above and are being destroyed on this failure path.
        unsafe {
            DestroyWindow(preview_hwnd);
            DestroyWindow(hwnd);
        }
        return Err("GetWindowRect failed for Rust Config PoC".to_owned());
    }
    // SAFETY: `preview_hwnd` is a live child window handle and `preview_rect` is writable.
    if unsafe { GetWindowRect(preview_hwnd, &mut preview_rect) } == 0 {
        // SAFETY: Handles were created above and are being destroyed on this failure path.
        unsafe {
            DestroyWindow(preview_hwnd);
            DestroyWindow(hwnd);
        }
        return Err("GetWindowRect failed for Rust Config PoC candidate preview host".to_owned());
    }
    let candidate_preview_child_inside_window = preview_rect.left >= rect.left
        && preview_rect.top >= rect.top
        && preview_rect.right <= rect.right
        && preview_rect.bottom <= rect.bottom;
    // SAFETY: `preview_hwnd` is a live child window handle.
    let candidate_preview_child_parented = unsafe { GetParent(preview_hwnd) } == hwnd;
    let (candidate_preview_child_selected_pixel, candidate_preview_child_selected_pixel_visible) =
        sample_selected_candidate_pixel(preview_hwnd);
    let candidate_preview_child_paint_count = PREVIEW_PAINT_COUNT.load(Ordering::SeqCst);
    let candidate_preview_child_painted = candidate_preview_child_paint_count > 0;
    // SAFETY: Window handles are live until the explicit cleanup below.
    let visible = unsafe { IsWindowVisible(hwnd) } != 0;
    // SAFETY: Window handles are live until the explicit cleanup below.
    let candidate_preview_child_visible = unsafe { IsWindowVisible(preview_hwnd) } != 0;
    // SAFETY: `hwnd` is live until the explicit cleanup below.
    let title_readable = unsafe { GetWindowTextLengthW(hwnd) } > 0;
    // SAFETY: Handles were created above and are destroyed before returning.
    unsafe {
        DestroyWindow(preview_hwnd);
        DestroyWindow(hwnd);
    }
    Ok(WindowSmokeEvidence {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
        visible,
        title_readable,
        candidate_preview_child_hwnd_created: true,
        candidate_preview_child_visible,
        candidate_preview_child_parented,
        candidate_preview_child_inside_window,
        candidate_preview_child_painted,
        candidate_preview_child_selected_pixel_visible,
        candidate_preview_child_paint_count,
        candidate_preview_child_selected_pixel,
        candidate_preview_child_left: preview_rect.left,
        candidate_preview_child_top: preview_rect.top,
        candidate_preview_child_right: preview_rect.right,
        candidate_preview_child_bottom: preview_rect.bottom,
        candidate_preview_child_width: preview_rect.right - preview_rect.left,
        candidate_preview_child_height: preview_rect.bottom - preview_rect.top,
    })
}

pub fn run_interactive(
    title: &str,
    minimum_window_dip: Size,
    candidate_preview_rect: LayoutRect,
) -> Result<(), String> {
    let class_name = to_wide("Fcitx5ConfigPocWindow");
    let preview_class_name = to_wide("Fcitx5ConfigPocCandidatePreviewHost");
    let title = to_wide(title);
    // SAFETY: A null module name asks Windows for the current process module handle.
    let instance = unsafe { GetModuleHandleW(null()) };
    if instance.is_null() {
        return Err("GetModuleHandleW failed for Rust Settings UI Preview".to_owned());
    }
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
    let preview_window_class = WndClassW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfn_wnd_proc: Some(candidate_preview_window_proc),
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: instance,
        h_icon: null_mut(),
        h_cursor: null_mut(),
        hbr_background: null_mut(),
        lpsz_menu_name: null(),
        lpsz_class_name: preview_class_name.as_ptr(),
    };
    // SAFETY: The class descriptors reference live UTF-16 buffers for this call and use
    // window procedures with the expected system ABI.
    if unsafe { RegisterClassW(&window_class) } == 0 {
        return Err("RegisterClassW failed for Rust Settings UI Preview".to_owned());
    }
    // SAFETY: Same registration contract as the top-level class above.
    if unsafe { RegisterClassW(&preview_window_class) } == 0 {
        return Err(
            "RegisterClassW failed for Rust Settings UI Preview candidate preview host".to_owned(),
        );
    }
    // SAFETY: The class/title pointers stay alive for the duration of this call. Parent,
    // menu, and parameter handles are null because this creates the top-level settings window.
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            minimum_window_dip.width,
            minimum_window_dip.height,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        )
    };
    if hwnd.is_null() {
        return Err("CreateWindowExW failed for Rust Settings UI Preview".to_owned());
    }
    create_settings_controls(hwnd, instance, &preview_class_name, candidate_preview_rect)?;
    // SAFETY: `hwnd` and its children were created successfully and can be shown/painted.
    unsafe {
        ShowWindow(hwnd, SW_SHOWNORMAL);
        UpdateWindow(hwnd);
    }
    message_loop(hwnd)
}

unsafe extern "system" fn window_proc(
    hwnd: Hwnd,
    message: u32,
    wparam: Wparam,
    lparam: Lparam,
) -> Lresult {
    if message == WM_COMMAND {
        let command_id = loword(wparam);
        if let Some(title) = page_title_for_command(command_id) {
            ACTIVE_NAV_PAGE.store(i32::from(command_id), Ordering::SeqCst);
            update_page_title(hwnd, title);
            apply_page_visibility(hwnd, i32::from(command_id));
            repaint_settings_window(hwnd);
            invalidate_preview(hwnd);
            return 0;
        }
        if hiword(wparam) == EN_CHANGE && handle_numeric_edit_change(hwnd, command_id) {
            return 0;
        }
        if hiword(wparam) == CBN_SELCHANGE && handle_font_family_change(hwnd, command_id) {
            return 0;
        }
        if hiword(wparam) == CBN_SELCHANGE && handle_language_change(hwnd, command_id) {
            return 0;
        }
        if hiword(wparam) == LBN_SELCHANGE && handle_package_selection_change(hwnd, command_id) {
            return 0;
        }
        if handle_package_action(hwnd, command_id) {
            return 0;
        }
    }
    if message == WM_DRAWITEM && draw_modern_nav_item(lparam as *const DrawItemStruct) {
        return 1;
    }
    if message == WM_CLOSE {
        // SAFETY: Windows delivered WM_CLOSE for this live HWND; DestroyWindow starts normal
        // teardown and leads to WM_DESTROY.
        unsafe {
            DestroyWindow(hwnd);
        }
        return 0;
    }
    if message == WM_DESTROY {
        // SAFETY: The top-level Settings preview is being destroyed; posting quit exits only
        // this process-local message loop.
        unsafe {
            PostQuitMessage(0);
        }
        return 0;
    }
    if message == WM_SIZE {
        repaint_settings_window(hwnd);
        invalidate_preview(hwnd);
    }
    if message == WM_ERASEBKGND {
        paint_settings_background(hwnd, wparam as Hdc);
        return 1;
    }
    if message == WM_CTLCOLORSTATIC {
        let hdc = wparam as Hdc;
        let child = lparam as Hwnd;
        let control_id = if child.is_null() {
            0
        } else {
            // SAFETY: `child` is the control HWND provided by WM_CTLCOLORSTATIC.
            unsafe { GetDlgCtrlID(child) }
        };
        let (background_color, brush) = static_control_background(control_id);
        // SAFETY: The HDC is provided by Windows for child static-control painting and these
        // calls only affect drawing attributes for this paint cycle.
        unsafe {
            SetBkMode(hdc, OPAQUE);
            SetBkColor(hdc, background_color);
            SetTextColor(hdc, design_tokens().palette.text_primary);
            return brush as Lresult;
        }
    }
    if message == WM_PRINTCLIENT {
        paint_settings_background(hwnd, wparam as Hdc);
        return 0;
    }
    if message == WM_PAINT {
        let mut paint = PaintStruct {
            hdc: null_mut(),
            f_erase: 0,
            rc_paint: Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            f_restore: 0,
            f_inc_update: 0,
            rgb_reserved: [0; 32],
        };
        // SAFETY: Windows calls this procedure for a valid top-level Settings HWND.
        let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
        if !hdc.is_null() {
            paint_settings_background(hwnd, hdc);
            // SAFETY: `paint` was initialized by BeginPaint for this HWND and must be closed.
            unsafe {
                EndPaint(hwnd, &paint);
            }
        }
        return 0;
    }
    // SAFETY: Delegates unhandled messages to the system default window procedure.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn paint_settings_background(hwnd: Hwnd, hdc: Hdc) {
    if hdc.is_null() {
        return;
    }
    let mut client = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: `hwnd` is a live top-level Settings HWND and `client` is writable.
    if unsafe { GetClientRect(hwnd, &mut client) } == 0 {
        return;
    }
    let page = active_page_id();
    let window = Size {
        width: client.width(),
        height: client.height(),
    };
    let Ok(plan) = settings_surface_paint_plan(page, window) else {
        fill_rect(hdc, &client, design_tokens().palette.background);
        return;
    };
    for component in plan.components {
        let rect = rect_from_layout(component.rect);
        fill_rect(hdc, &rect, component.fill_color);
    }
    let tokens = design_tokens();
    let accent = Rect {
        left: tokens.spacing_16,
        top: tokens.spacing_24 - tokens.spacing_8 + tokens.spacing_4 / 2,
        right: tokens.spacing_16 + tokens.nav_accent_width,
        bottom: 278,
    };
    fill_rect(hdc, &accent, tokens.palette.accent);
}

fn active_page_id() -> PageId {
    match ACTIVE_NAV_PAGE.load(Ordering::SeqCst) {
        K_NAV_APPEARANCE => PageId::Appearance,
        K_NAV_SHORTCUTS => PageId::Shortcuts,
        K_NAV_UPDATES => PageId::Updates,
        K_NAV_REPAIR => PageId::Diagnostics,
        K_NAV_PACKAGES => PageId::Addons,
        _ => PageId::InputMethods,
    }
}

fn rect_from_layout(rect: LayoutRect) -> Rect {
    Rect {
        left: rect.x,
        top: rect.y,
        right: rect.x + rect.width,
        bottom: rect.y + rect.height,
    }
}

fn fill_rect(hdc: Hdc, rect: &Rect, color: u32) {
    // SAFETY: Creates a process-local GDI brush for immediate FillRect use.
    let brush = unsafe { CreateSolidBrush(color) };
    if brush.is_null() {
        return;
    }
    // SAFETY: The HDC is valid for the current paint/print cycle, `rect` is initialized, and
    // the brush is deleted immediately after use.
    unsafe {
        FillRect(hdc, rect, brush);
        DeleteObject(brush);
    }
}

fn draw_modern_nav_item(draw_item: *const DrawItemStruct) -> bool {
    if draw_item.is_null() {
        return false;
    }
    // SAFETY: WM_DRAWITEM provides a live DRAWITEMSTRUCT pointer for the duration of message
    // dispatch. We copy only POD fields and do not retain borrowed handles.
    let item = unsafe { &*draw_item };
    let control_id = item.ctl_id as i32;
    if !is_nav_control(control_id) || item.hdc.is_null() {
        return false;
    }

    let selected = ACTIVE_NAV_PAGE.load(Ordering::SeqCst) == control_id;
    let tokens = design_tokens();
    let background = if selected {
        tokens.palette.nav_selected
    } else {
        tokens.palette.sidebar
    };
    fill_rect(item.hdc, &item.rc_item, background);

    if selected {
        let accent = Rect {
            left: item.rc_item.left,
            top: item.rc_item.top + tokens.spacing_8 - 1,
            right: item.rc_item.left + tokens.nav_accent_width + 1,
            bottom: item.rc_item.bottom - tokens.spacing_8 + 1,
        };
        fill_rect(item.hdc, &accent, tokens.palette.accent);
    }

    let font = settings_ui_font();
    let old_font = if font.is_null() {
        null_mut()
    } else {
        // SAFETY: The process-owned font is valid for this draw cycle and restored below.
        unsafe { SelectObject(item.hdc, font.cast::<c_void>()) }
    };

    let text = child_text(item.hwnd_item);
    let text = to_wide(&text);
    let mut text_rect = Rect {
        left: item.rc_item.left + 18,
        top: item.rc_item.top,
        right: item.rc_item.right - 12,
        bottom: item.rc_item.bottom,
    };
    // SAFETY: The HDC belongs to this owner-draw callback. The UTF-16 buffer is
    // NUL-terminated and lives through DrawTextW.
    unsafe {
        SetBkMode(item.hdc, TRANSPARENT);
        SetTextColor(item.hdc, tokens.palette.text_primary);
        DrawTextW(
            item.hdc,
            text.as_ptr(),
            -1,
            &mut text_rect,
            DT_LEFT | DT_SINGLELINE | DT_VCENTER,
        );
        if !old_font.is_null() {
            SelectObject(item.hdc, old_font);
        }
    }
    true
}

fn is_nav_control(control_id: i32) -> bool {
    matches!(
        control_id,
        K_NAV_GENERAL
            | K_NAV_APPEARANCE
            | K_NAV_SHORTCUTS
            | K_NAV_UPDATES
            | K_NAV_REPAIR
            | K_NAV_PACKAGES
    )
}

fn static_control_background(control_id: i32) -> (u32, Hbrush) {
    let tokens = design_tokens();
    if control_id == K_PAGE_TITLE {
        (
            tokens.palette.header,
            cached_solid_brush(&SETTINGS_HEADER_BRUSH, tokens.palette.header),
        )
    } else {
        (
            tokens.palette.content,
            cached_solid_brush(&SETTINGS_CONTENT_BRUSH, tokens.palette.content),
        )
    }
}

fn cached_solid_brush(slot: &AtomicPtr<c_void>, color: u32) -> Hbrush {
    let existing = slot.load(Ordering::SeqCst);
    if !existing.is_null() {
        return existing.cast();
    }
    // SAFETY: Creates a process-local solid brush retained until process exit. Returning a
    // stable brush handle is required by WM_CTLCOLORSTATIC; a temporary brush would be invalid
    // after the message returns.
    let created = unsafe { CreateSolidBrush(color) };
    if created.is_null() {
        return null_mut();
    }
    slot.store(created.cast::<c_void>(), Ordering::SeqCst);
    created
}

fn settings_ui_font() -> Hfont {
    let existing = SETTINGS_UI_FONT.load(Ordering::SeqCst);
    if !existing.is_null() {
        return existing;
    }
    let tokens = design_tokens();
    let face_name = to_wide("Segoe UI");
    // SAFETY: The face name buffer is NUL-terminated and lives for the duration of the call.
    // The created HFONT intentionally lives until process exit so all child HWNDs can keep
    // using it without a dangling GDI handle.
    let created = unsafe {
        CreateFontW(
            -tokens.body_font_height,
            0,
            0,
            0,
            tokens.body_weight,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            face_name.as_ptr(),
        )
    };
    if created.is_null() {
        return null_mut();
    }
    SETTINGS_UI_FONT.store(created.cast::<c_void>(), Ordering::SeqCst);
    created
}

fn settings_title_font() -> Hfont {
    let existing = SETTINGS_TITLE_FONT.load(Ordering::SeqCst);
    if !existing.is_null() {
        return existing;
    }
    let tokens = design_tokens();
    let face_name = to_wide("Segoe UI");
    // SAFETY: Same lifetime contract as settings_ui_font; this title font is process-owned.
    let created = unsafe {
        CreateFontW(
            -tokens.title_font_height,
            0,
            0,
            0,
            tokens.title_weight,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            face_name.as_ptr(),
        )
    };
    if created.is_null() {
        return null_mut();
    }
    SETTINGS_TITLE_FONT.store(created.cast::<c_void>(), Ordering::SeqCst);
    created
}

unsafe extern "system" fn candidate_preview_window_proc(
    hwnd: Hwnd,
    message: u32,
    wparam: Wparam,
    lparam: Lparam,
) -> Lresult {
    if message == WM_PAINT {
        let mut paint = PaintStruct {
            hdc: null_mut(),
            f_erase: 0,
            rc_paint: Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            f_restore: 0,
            f_inc_update: 0,
            rgb_reserved: [0; 32],
        };
        // SAFETY: Windows calls this window procedure for a valid preview HWND during paint.
        let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
        if !hdc.is_null() {
            paint_candidate_preview(hwnd, hdc);
            // SAFETY: `paint` was initialized by BeginPaint for this HWND and must be closed.
            unsafe {
                EndPaint(hwnd, &paint);
            }
            PREVIEW_PAINT_COUNT.fetch_add(1, Ordering::SeqCst);
        }
        return 0;
    }
    // SAFETY: Delegates unhandled messages to the system default window procedure.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn paint_candidate_preview(hwnd: Hwnd, hdc: Hdc) {
    let mut client = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: `hwnd` is the preview HWND currently being painted and `client` is writable.
    if unsafe { GetClientRect(hwnd, &mut client) } == 0 {
        return;
    }
    let Ok(plan) = candidate_preview_paint_plan(1.0, client.width() as f32, client.height() as f32)
    else {
        return;
    };
    // SAFETY: Creates a process-local GDI brush for immediate FillRect use.
    let background_brush = unsafe { CreateSolidBrush(plan.background_color) };
    if !background_brush.is_null() {
        // SAFETY: `hdc` is valid for this paint cycle, `client` is initialized, and the brush
        // is deleted immediately after use.
        unsafe {
            FillRect(hdc, &client, background_brush);
            DeleteObject(background_brush);
        }
    }
    // SAFETY: The HDC is valid for the paint cycle and this setter does not retain pointers.
    unsafe {
        SetBkMode(hdc, TRANSPARENT);
    }
    let font = settings_ui_font();
    let old_font = if font.is_null() {
        null_mut()
    } else {
        // SAFETY: The font is process-owned and valid for this paint cycle; the old font is
        // restored after drawing the preview text.
        unsafe { SelectObject(hdc, font.cast::<c_void>()) }
    };
    for item in plan.items {
        let rect = rect_from_candidate_core(item.bounds);
        if item.selected {
            // SAFETY: Creates a process-local GDI brush for immediate FillRect use.
            let selected_brush = unsafe { CreateSolidBrush(plan.selected_background_color) };
            if !selected_brush.is_null() {
                // SAFETY: `rect` is bounded by candidate-core's preview plan and the brush is
                // deleted after use.
                unsafe {
                    FillRect(hdc, &rect, selected_brush);
                    DeleteObject(selected_brush);
                }
            }
        }
        let color = if item.selected {
            plan.selected_text_color
        } else {
            plan.text_color
        };
        draw_preview_line(hdc, rect, color, &item.text);
    }
    if !old_font.is_null() {
        // SAFETY: Restores the GDI object returned by SelectObject for this HDC.
        unsafe {
            SelectObject(hdc, old_font);
        }
    }
}

fn draw_preview_line(hdc: Hdc, mut rect: Rect, color: u32, text: &str) {
    let text = to_wide(text);
    // SAFETY: The UTF-16 buffer is NUL-terminated and lives for the duration of DrawTextW.
    unsafe {
        SetTextColor(hdc, color);
        DrawTextW(
            hdc,
            text.as_ptr(),
            (text.len() - 1) as i32,
            &mut rect,
            DT_LEFT | DT_SINGLELINE | DT_VCENTER,
        );
    }
}

fn rect_from_candidate_core(rect: fcitx5_candidate_core::Rect) -> Rect {
    Rect {
        left: rect.left.round() as i32,
        top: rect.top.round() as i32,
        right: rect.right.round() as i32,
        bottom: rect.bottom.round() as i32,
    }
}

fn sample_selected_candidate_pixel(hwnd: Hwnd) -> (u32, bool) {
    // SAFETY: `hwnd` is a live preview child window while this function is called.
    let hdc = unsafe { GetDC(hwnd) };
    if hdc.is_null() {
        return (GET_PIXEL_ERROR, false);
    }
    // SAFETY: The HDC is a client DC for the preview HWND; (12,12) is inside the selected row.
    let pixel = unsafe { GetPixel(hdc, 12, 12) };
    // SAFETY: Releases the client DC acquired above for the same HWND.
    unsafe {
        ReleaseDC(hwnd, hdc);
    }
    let Ok(plan) = candidate_preview_paint_plan(1.0, 596.0, 166.0) else {
        return (pixel, false);
    };
    (pixel, pixel == plan.selected_background_color)
}

fn create_settings_controls(
    hwnd: Hwnd,
    instance: Hinstance,
    preview_class_name: &[u16],
    candidate_preview_rect: LayoutRect,
) -> Result<(), String> {
    let preview_left = candidate_preview_rect.x;
    let preview_width = candidate_preview_rect.width;
    let static_class = to_wide("STATIC");
    let button_class = to_wide("BUTTON");
    let edit_class = to_wide("EDIT");
    let combo_class = to_wide("COMBOBOX");
    let listbox_class = to_wide("LISTBOX");
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_PAGE_TITLE,
        "Input methods",
        248,
        28,
        596,
        48,
        0,
    )?;
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_STATUS,
        "Ready. Rust Settings UI Preview is running inside the Config process.",
        248,
        596,
        596,
        36,
        0,
    )?;
    let packages = create_child_control(
        hwnd,
        instance,
        &listbox_class,
        K_PACKAGES,
        "",
        248,
        128,
        360,
        126,
        WS_BORDER | WS_VSCROLL | WS_TABSTOP,
    )?;
    populate_available_packages(packages);
    create_child_control(
            hwnd,
            instance,
            &static_class,
            K_PACKAGE_DETAIL,
            "Rime: trusted signed add-on package. Configure opens through Rust package/control boundaries.",
            626,
            128,
            218,
            126,
            WS_BORDER,
        )?;
    create_child_control(
        hwnd,
        instance,
        &button_class,
        K_PACKAGE_INSTALL,
        "Install",
        248,
        276,
        112,
        38,
        WS_TABSTOP,
    )?;
    create_child_control(
        hwnd,
        instance,
        &button_class,
        K_PACKAGE_UPDATE,
        "Update",
        372,
        276,
        112,
        38,
        WS_TABSTOP,
    )?;
    create_child_control(
        hwnd,
        instance,
        &button_class,
        K_PACKAGE_REMOVE,
        "Remove",
        496,
        276,
        112,
        38,
        WS_TABSTOP,
    )?;
    create_child_control(
        hwnd,
        instance,
        &button_class,
        K_PACKAGE_CONFIGURE,
        "Configure",
        626,
        276,
        128,
        38,
        WS_TABSTOP,
    )?;
    create_child_control(
        hwnd,
        instance,
        &button_class,
        K_PACKAGE_REFRESH,
        "Refresh",
        248,
        328,
        112,
        38,
        WS_TABSTOP,
    )?;
    create_child_control(
        hwnd,
        instance,
        &button_class,
        K_PACKAGE_DETAILS,
        "Details",
        372,
        328,
        112,
        38,
        WS_TABSTOP,
    )?;
    create_child_control(
        hwnd,
        instance,
        &button_class,
        K_PACKAGE_ENABLE_DISABLE,
        "Enable / Disable",
        496,
        328,
        128,
        38,
        WS_TABSTOP,
    )?;
    create_child_control(
        hwnd,
        instance,
        &button_class,
        K_PACKAGE_REPAIR,
        "Repair",
        636,
        328,
        112,
        38,
        WS_TABSTOP,
    )?;
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_SAVE_STATUS,
        "No pending changes",
        248,
        640,
        596,
        36,
        0,
    )?;
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_LABEL_INPUT_METHODS,
        "Enabled input methods",
        248,
        112,
        596,
        30,
        0,
    )?;
    let input_methods = create_child_control(
        hwnd,
        instance,
        &listbox_class,
        K_INPUT_METHOD_LIST,
        "",
        248,
        154,
        596,
        132,
        WS_BORDER | WS_VSCROLL | WS_TABSTOP,
    )?;
    populate_enabled_input_methods(input_methods);
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_LABEL_LANGUAGE,
        "Language / 语言",
        248,
        314,
        180,
        30,
        0,
    )?;
    let language_selector = create_child_control(
        hwnd,
        instance,
        &combo_class,
        K_LANGUAGE_SELECTOR,
        "",
        248,
        350,
        280,
        96,
        WS_BORDER | WS_VSCROLL | WS_TABSTOP | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
    )?;
    populate_language_selector(language_selector);
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_LABEL_FONT_SIZE,
        "Text size",
        248,
        312,
        116,
        28,
        0,
    )?;
    create_child_control(
        hwnd,
        instance,
        &edit_class,
        K_APPEARANCE_FONT_SIZE,
        "18",
        248,
        342,
        92,
        34,
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
    )?;
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_LABEL_OPACITY,
        "Opacity",
        374,
        312,
        92,
        28,
        0,
    )?;
    create_child_control(
        hwnd,
        instance,
        &edit_class,
        K_APPEARANCE_OPACITY,
        "1.00",
        374,
        342,
        92,
        34,
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
    )?;
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_LABEL_SPACING,
        "Gap",
        500,
        312,
        112,
        28,
        0,
    )?;
    create_child_control(
        hwnd,
        instance,
        &edit_class,
        K_APPEARANCE_SPACING,
        "8",
        500,
        342,
        92,
        34,
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
    )?;
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_LABEL_CORNER_RADIUS,
        "Corners",
        626,
        312,
        112,
        28,
        0,
    )?;
    create_child_control(
        hwnd,
        instance,
        &edit_class,
        K_APPEARANCE_CORNER_RADIUS,
        "12",
        626,
        342,
        92,
        34,
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
    )?;
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_LABEL_CANDIDATE_WIDTH,
        "Width",
        752,
        312,
        84,
        28,
        0,
    )?;
    create_child_control(
        hwnd,
        instance,
        &edit_class,
        K_APPEARANCE_CANDIDATE_WIDTH,
        "420",
        752,
        342,
        92,
        34,
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
    )?;
    create_child_control(
        hwnd,
        instance,
        &static_class,
        K_LABEL_CANDIDATE_FONT,
        "Candidate font",
        248,
        396,
        180,
        28,
        0,
    )?;
    let font_combo = create_child_control(
        hwnd,
        instance,
        &combo_class,
        K_APPEARANCE_FONT_FAMILY,
        "",
        428,
        392,
        280,
        128,
        WS_BORDER | WS_VSCROLL | WS_TABSTOP | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
    )?;
    populate_system_font_picker(font_combo, preview_state_font_family().as_deref())?;
    for (index, (id, label)) in [
        (K_NAV_GENERAL, "Input methods"),
        (K_NAV_APPEARANCE, "Appearance"),
        (K_NAV_SHORTCUTS, "Shortcuts"),
        (K_NAV_UPDATES, "Updates"),
        (K_NAV_REPAIR, "Diagnostics"),
        (K_NAV_PACKAGES, "Packages"),
    ]
    .iter()
    .enumerate()
    {
        create_child_control(
            hwnd,
            instance,
            &button_class,
            *id,
            label,
            24,
            84 + (index as i32 * 54),
            176,
            42,
            WS_TABSTOP,
        )?;
    }
    let preview_title = to_wide("Candidate Preview");
    // SAFETY: The preview class/title pointers stay alive for the call, and `hwnd` is a live
    // top-level window handle created above. The child coordinates come from the validated
    // layout model and the child id is the QA-visible K_PREVIEW control id.
    let preview_hwnd = unsafe {
        CreateWindowExW(
            0,
            preview_class_name.as_ptr(),
            preview_title.as_ptr(),
            WS_CHILD | WS_VISIBLE,
            preview_left,
            candidate_preview_rect.y,
            preview_width,
            candidate_preview_rect.height,
            hwnd,
            control_id_handle(K_PREVIEW),
            instance,
            null_mut(),
        )
    };
    if preview_hwnd.is_null() {
        return Err("CreateWindowExW failed for Rust Settings UI Preview K_PREVIEW".to_owned());
    }
    // SAFETY: `preview_hwnd` is a live child handle and can be explicitly repainted.
    unsafe {
        InvalidateRect(preview_hwnd, null(), FALSE);
        UpdateWindow(preview_hwnd);
    }
    apply_page_visibility(hwnd, K_NAV_GENERAL);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_child_control(
    parent: Hwnd,
    instance: Hinstance,
    class_name: &[u16],
    id: i32,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    mut extra_style: u32,
) -> Result<Hwnd, String> {
    if is_nav_control(id) {
        extra_style |= BS_OWNERDRAW;
    } else if matches!(
        id,
        K_PACKAGE_INSTALL
            | K_PACKAGE_UPDATE
            | K_PACKAGE_REMOVE
            | K_PACKAGE_CONFIGURE
            | K_PACKAGE_REFRESH
            | K_PACKAGE_DETAILS
            | K_PACKAGE_ENABLE_DISABLE
            | K_PACKAGE_REPAIR
    ) {
        extra_style |= BS_FLAT;
    }
    let text = to_wide(text);
    // SAFETY: The class/text UTF-16 buffers live for this call, `parent` is the live top-level
    // window, and the positive child id is passed through Win32's HMENU/id slot.
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            text.as_ptr(),
            WS_CHILD | WS_VISIBLE | extra_style,
            x,
            y,
            width,
            height,
            parent,
            control_id_handle(id),
            instance,
            null_mut(),
        )
    };
    if hwnd.is_null() {
        return Err(format!(
            "CreateWindowExW failed for Rust Settings UI Preview child control {id}"
        ));
    }
    let font = if id == K_PAGE_TITLE {
        settings_title_font()
    } else {
        settings_ui_font()
    };
    if !font.is_null() {
        // SAFETY: `hwnd` is a live child control. WM_SETFONT stores the HFONT handle but does
        // not take ownership; settings_ui_font keeps the process-wide font alive until exit.
        unsafe {
            SendMessageW(hwnd, WM_SETFONT, font as Wparam, TRUE as Lparam);
        }
    }
    Ok(hwnd)
}

fn message_loop(hwnd: Hwnd) -> Result<(), String> {
    let mut message = Msg {
        hwnd: null_mut(),
        message: 0,
        w_param: 0,
        l_param: 0,
        time: 0,
        pt: Point { x: 0, y: 0 },
    };
    loop {
        // SAFETY: `message` points to writable stack storage; null HWND receives thread
        // messages for this single Settings UI process.
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result == -1 {
            // SAFETY: `hwnd` is the top-level window created by `run_interactive`; destroy it
            // on the error path so the process does not leave a stray Config window.
            unsafe {
                DestroyWindow(hwnd);
            }
            return Err("GetMessageW failed for Rust Settings UI Preview".to_owned());
        }
        if result == 0 {
            return Ok(());
        }
        // SAFETY: `message` was populated by GetMessageW and can be translated/dispatched.
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn update_page_title(hwnd: Hwnd, title: &str) {
    // SAFETY: Reads the child handle for the QA-visible K_PAGE_TITLE control.
    let title_hwnd = unsafe { GetDlgItem(hwnd, K_PAGE_TITLE) };
    if title_hwnd.is_null() {
        return;
    }
    let title = to_wide(title);
    // SAFETY: `title_hwnd` is a live child control and the UTF-16 buffer lives for this call.
    unsafe {
        SetWindowTextW(title_hwnd, title.as_ptr());
    }
}

fn apply_page_visibility(hwnd: Hwnd, active_page: i32) {
    for control in [
        K_LABEL_INPUT_METHODS,
        K_INPUT_METHOD_LIST,
        K_LABEL_LANGUAGE,
        K_LANGUAGE_SELECTOR,
        K_LABEL_FONT_SIZE,
        K_APPEARANCE_FONT_SIZE,
        K_LABEL_OPACITY,
        K_APPEARANCE_OPACITY,
        K_LABEL_SPACING,
        K_APPEARANCE_SPACING,
        K_LABEL_CORNER_RADIUS,
        K_APPEARANCE_CORNER_RADIUS,
        K_LABEL_CANDIDATE_WIDTH,
        K_APPEARANCE_CANDIDATE_WIDTH,
        K_LABEL_CANDIDATE_FONT,
        K_APPEARANCE_FONT_FAMILY,
        K_PREVIEW,
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
        K_SAVE_STATUS,
    ] {
        show_child_control(
            hwnd,
            control,
            controls_for_page(active_page).contains(&control),
        );
    }
    repaint_navigation(hwnd);
    repaint_settings_window(hwnd);
}

fn controls_for_page(active_page: i32) -> &'static [i32] {
    match active_page {
        K_NAV_GENERAL => &[
            K_LABEL_INPUT_METHODS,
            K_INPUT_METHOD_LIST,
            K_LABEL_LANGUAGE,
            K_LANGUAGE_SELECTOR,
            K_SAVE_STATUS,
        ],
        K_NAV_APPEARANCE => &[
            K_LABEL_FONT_SIZE,
            K_APPEARANCE_FONT_SIZE,
            K_LABEL_OPACITY,
            K_APPEARANCE_OPACITY,
            K_LABEL_SPACING,
            K_APPEARANCE_SPACING,
            K_LABEL_CORNER_RADIUS,
            K_APPEARANCE_CORNER_RADIUS,
            K_LABEL_CANDIDATE_WIDTH,
            K_APPEARANCE_CANDIDATE_WIDTH,
            K_LABEL_CANDIDATE_FONT,
            K_APPEARANCE_FONT_FAMILY,
            K_PREVIEW,
            K_SAVE_STATUS,
        ],
        K_NAV_PACKAGES => &[
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
        K_NAV_SHORTCUTS | K_NAV_UPDATES | K_NAV_REPAIR => &[K_STATUS],
        _ => &[K_SAVE_STATUS],
    }
}

fn show_child_control(parent: Hwnd, id: i32, visible: bool) {
    // SAFETY: Reads the child handle for a QA-visible control id.
    let child = unsafe { GetDlgItem(parent, id) };
    if child.is_null() {
        return;
    }
    // SAFETY: `child` is a live HWND and ShowWindow only changes its visibility.
    unsafe {
        ShowWindow(child, if visible { SW_SHOW } else { SW_HIDE });
    }
}

fn invalidate_preview(hwnd: Hwnd) {
    // SAFETY: Reads the child handle for the QA-visible K_PREVIEW control.
    let preview_hwnd = unsafe { GetDlgItem(hwnd, K_PREVIEW) };
    if preview_hwnd.is_null() {
        return;
    }
    // SAFETY: `preview_hwnd` is a live child control if GetDlgItem returned non-null.
    unsafe {
        InvalidateRect(preview_hwnd, null(), FALSE);
        UpdateWindow(preview_hwnd);
    }
}

fn repaint_navigation(hwnd: Hwnd) {
    for control in [
        K_NAV_GENERAL,
        K_NAV_APPEARANCE,
        K_NAV_SHORTCUTS,
        K_NAV_UPDATES,
        K_NAV_REPAIR,
        K_NAV_PACKAGES,
    ] {
        // SAFETY: Reads known child HWNDs and invalidates only those that exist.
        let child = unsafe { GetDlgItem(hwnd, control) };
        if child.is_null() {
            continue;
        }
        // SAFETY: The child HWND is live and can be repainted immediately.
        unsafe {
            InvalidateRect(child, null(), TRUE);
            UpdateWindow(child);
        }
    }
}

fn repaint_settings_window(hwnd: Hwnd) {
    // SAFETY: Invalidates the entire top-level Settings client area with erase so hidden child
    // controls cannot leave stale pixels during navigation or resize.
    unsafe {
        InvalidateRect(hwnd, null(), TRUE);
        UpdateWindow(hwnd);
    }
}

fn page_title_for_command(command_id: u16) -> Option<&'static str> {
    match i32::from(command_id) {
        K_NAV_GENERAL => Some("Input methods"),
        K_NAV_APPEARANCE => Some("Appearance"),
        K_NAV_SHORTCUTS => Some("Shortcuts"),
        K_NAV_UPDATES => Some("Updates"),
        K_NAV_REPAIR => Some("Diagnostics and repair"),
        K_NAV_PACKAGES => Some("Packages"),
        _ => None,
    }
}

fn handle_numeric_edit_change(hwnd: Hwnd, command_id: u16) -> bool {
    let Some(field) = numeric_field_for_command(command_id) else {
        return false;
    };
    let edit = unsafe { GetDlgItem(hwnd, i32::from(command_id)) };
    if edit.is_null() {
        return true;
    }
    let text = child_text(edit);
    let status = match validate_appearance_numeric_input(field, &text) {
        Ok(value) => format!("{} accepted: {value:.2}", field.spec().key),
        Err("appearance.numeric.incomplete") => {
            "appearance.numeric.incomplete: keeping last valid value".to_owned()
        }
        Err(error) => format!("{error}: keeping last valid value"),
    };
    set_child_text(hwnd, K_SAVE_STATUS, &status);
    invalidate_preview(hwnd);
    true
}

fn numeric_field_for_command(command_id: u16) -> Option<AppearanceNumericField> {
    match i32::from(command_id) {
        K_APPEARANCE_FONT_SIZE => Some(AppearanceNumericField::FontSizeDip),
        K_APPEARANCE_OPACITY => Some(AppearanceNumericField::Opacity),
        K_APPEARANCE_SPACING => Some(AppearanceNumericField::SpacingDip),
        K_APPEARANCE_CORNER_RADIUS => Some(AppearanceNumericField::CornerRadiusDip),
        K_APPEARANCE_CANDIDATE_WIDTH => Some(AppearanceNumericField::CandidateWidthDip),
        _ => None,
    }
}

fn handle_font_family_change(hwnd: Hwnd, command_id: u16) -> bool {
    if i32::from(command_id) != K_APPEARANCE_FONT_FAMILY {
        return false;
    }
    let font_family = unsafe { GetDlgItem(hwnd, K_APPEARANCE_FONT_FAMILY) };
    let selected = selected_combo_text(font_family).unwrap_or_else(|| "unknown".to_owned());
    let status = match persist_preview_font_family(&selected) {
        Ok(()) => format!("font_family accepted: {selected}"),
        Err(error) => format!("font_family persistence failed: {error}"),
    };
    set_child_text(hwnd, K_SAVE_STATUS, &status);
    invalidate_preview(hwnd);
    true
}

fn handle_language_change(hwnd: Hwnd, command_id: u16) -> bool {
    if i32::from(command_id) != K_LANGUAGE_SELECTOR {
        return false;
    }
    let language_selector = unsafe { GetDlgItem(hwnd, K_LANGUAGE_SELECTOR) };
    let selected =
        selected_combo_text(language_selector).unwrap_or_else(|| "System default".to_owned());
    set_child_text(
        hwnd,
        K_SAVE_STATUS,
        &format!("language accepted: {selected}"),
    );
    true
}

fn handle_package_action(hwnd: Hwnd, command_id: u16) -> bool {
    let status = match i32::from(command_id) {
        K_PACKAGE_REFRESH => Some("package.refresh planned: trusted repository metadata required"),
        K_PACKAGE_DETAILS => {
            update_package_detail_from_selection(hwnd);
            Some("package.details loaded: selected component metadata")
        }
        K_PACKAGE_INSTALL => {
            Some("package.install planned: signed repository metadata required before download")
        }
        K_PACKAGE_UPDATE => Some("package.update planned: Rust package-core transaction"),
        K_PACKAGE_ENABLE_DISABLE => Some("package.enable_disable planned: Rust package-core state"),
        K_PACKAGE_REMOVE => Some("package.remove planned: rollback-safe Rust package-core state"),
        K_PACKAGE_CONFIGURE => Some("plugin_config loaded: fcitx5-rime settings surface"),
        K_PACKAGE_REPAIR => Some("package.repair planned: verify and restore installed payloads"),
        _ => None,
    };
    let Some(status) = status else {
        return false;
    };
    set_child_text(hwnd, K_STATUS, status);
    set_child_text(hwnd, K_SAVE_STATUS, status);
    true
}

fn handle_package_selection_change(hwnd: Hwnd, command_id: u16) -> bool {
    if i32::from(command_id) != K_PACKAGES {
        return false;
    }
    update_package_detail_from_selection(hwnd);
    set_child_text(
        hwnd,
        K_STATUS,
        "package.selection changed: details refreshed",
    );
    set_child_text(
        hwnd,
        K_SAVE_STATUS,
        "package.selection changed: details refreshed",
    );
    true
}

fn update_package_detail_from_selection(hwnd: Hwnd) {
    let packages = unsafe { GetDlgItem(hwnd, K_PACKAGES) };
    let selected =
        selected_listbox_text(packages).unwrap_or_else(|| "fcitx5-rime — installed".to_owned());
    set_child_text(
            hwnd,
            K_PACKAGE_DETAIL,
            &format!(
                "{selected}: type=addon, source=official signed fixture, actions=refresh/details/install/update/enable-disable/remove/repair"
            ),
        );
}

fn populate_system_font_picker(combo: Hwnd, persisted_font: Option<&str>) -> Result<(), String> {
    let mut fonts = system_font_families_for_picker();
    if fonts.is_empty() {
        fonts.push("Segoe UI".to_owned());
    }
    let mut selected_index = 0usize;
    for family in &fonts {
        if let Some(persisted_font) = persisted_font {
            if family.eq_ignore_ascii_case(persisted_font) {
                selected_index = fonts
                    .iter()
                    .position(|candidate| candidate.eq_ignore_ascii_case(persisted_font))
                    .unwrap_or(0);
            }
        }
        let family = to_wide(family);
        // SAFETY: `combo` is a live combobox HWND and the UTF-16 string buffer lives for the
        // synchronous CB_ADDSTRING message.
        unsafe {
            SendMessageW(combo, CB_ADDSTRING, 0, family.as_ptr() as Lparam);
        }
    }
    // SAFETY: `combo` is a live combobox HWND; selecting the first item initializes the
    // visible current system-font choice for QA and users.
    unsafe {
        SendMessageW(combo, CB_SETCURSEL, selected_index, 0);
    }
    Ok(())
}

fn system_font_families_for_picker() -> Vec<String> {
    let required =
        fcitx5_windows_common_core::fcitx5_windows_common_system_font_families_utf16(null_mut(), 0);
    if required == 0 {
        return Vec::new();
    }
    let mut payload = vec![0u16; required];
    let written = fcitx5_windows_common_core::fcitx5_windows_common_system_font_families_utf16(
        payload.as_mut_ptr(),
        payload.len(),
    )
    .min(payload.len());
    let mut fonts = Vec::new();
    let mut start = 0usize;
    for index in 0..written {
        if payload[index] == 0 {
            if index > start {
                fonts.push(String::from_utf16_lossy(&payload[start..index]));
            }
            start = index + 1;
        }
    }
    fonts
}

fn populate_enabled_input_methods(listbox: Hwnd) {
    for input_method in ["Pinyin - 中文", "Rime - 中州韵", "Keyboard - English (US)"] {
        let input_method = to_wide(input_method);
        // SAFETY: `listbox` is a live LISTBOX HWND and the UTF-16 buffer lives for this
        // synchronous LB_ADDSTRING message.
        unsafe {
            SendMessageW(listbox, LB_ADDSTRING, 0, input_method.as_ptr() as Lparam);
        }
    }
}

fn populate_available_packages(listbox: Hwnd) {
    for package in [
        "fcitx5-rime - Rime",
        "fcitx5-chinese-addons - Chinese Addons",
        "fcitx5-mozc - Mozc",
    ] {
        let package = to_wide(package);
        // SAFETY: `listbox` is a live LISTBOX HWND and the UTF-16 buffer lives for this
        // synchronous LB_ADDSTRING message.
        unsafe {
            SendMessageW(listbox, LB_ADDSTRING, 0, package.as_ptr() as Lparam);
        }
    }
    // SAFETY: `listbox` is a live LISTBOX HWND. Selecting the first item gives details and
    // selection-change QA a deterministic starting point without running package operations.
    unsafe {
        SendMessageW(listbox, LB_SETCURSEL, 0, 0);
    }
}

fn populate_language_selector(combo: Hwnd) {
    for language in ["System default", "English (United States)", "简体中文"] {
        let language = to_wide(language);
        // SAFETY: `combo` is a live combobox HWND and the UTF-16 string buffer lives for the
        // synchronous CB_ADDSTRING message.
        unsafe {
            SendMessageW(combo, CB_ADDSTRING, 0, language.as_ptr() as Lparam);
        }
    }
    // SAFETY: `combo` is a live combobox HWND; selecting index 0 initializes the system
    // language policy.
    unsafe {
        SendMessageW(combo, CB_SETCURSEL, 0, 0);
    }
}

fn preview_state_path() -> Option<PathBuf> {
    std::env::var_os(PREVIEW_STATE_ENV).map(PathBuf::from)
}

fn preview_state_font_family() -> Option<String> {
    let path = preview_state_path()?;
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("font_family="))
        .map(unescape_state_value)
}

fn persist_preview_font_family(font_family: &str) -> Result<(), String> {
    let Some(path) = preview_state_path() else {
        return Ok(());
    };
    let content = format!("font_family={}\n", escape_state_value(font_family));
    atomic_write_utf8_file(&path, &content)
}

fn atomic_write_utf8_file(path: &Path, content: &str) -> Result<(), String> {
    let wide_path = path.as_os_str().encode_wide().collect::<Vec<_>>();
    let destination = ControlUtf16 {
        ptr: wide_path.as_ptr(),
        len: wide_path.len(),
    };
    let content = ControlUtf8 {
        ptr: content.as_bytes().as_ptr(),
        len: content.len(),
    };
    // SAFETY: The UTF-16 path and UTF-8 content buffers live for this synchronous Rust Control
    // ABI call. The callee does not retain pointers and performs the atomic file replacement.
    let status = unsafe { fcitx5_control_atomic_write_utf8_file_utf16(destination, content) };
    if status == 0 {
        Ok(())
    } else {
        Err("atomic_write_utf8_file".to_owned())
    }
}

fn selected_combo_text(combo: Hwnd) -> Option<String> {
    if combo.is_null() {
        return None;
    }
    // SAFETY: `combo` is a live combobox HWND.
    let selected = unsafe { SendMessageW(combo, CB_GETCURSEL, 0, 0) };
    if selected < 0 {
        return None;
    }
    // SAFETY: `combo` is a live combobox HWND and `selected` is the current selection index.
    let len = unsafe { SendMessageW(combo, CB_GETLBTEXTLEN, selected as Wparam, 0) };
    if len <= 0 {
        return None;
    }
    let mut buffer = vec![0u16; len as usize + 1];
    // SAFETY: `buffer` is writable and large enough for the selected list item plus NUL.
    let copied = unsafe {
        SendMessageW(
            combo,
            CB_GETLBTEXT,
            selected as Wparam,
            buffer.as_mut_ptr() as Lparam,
        )
    };
    if copied <= 0 {
        return None;
    }
    buffer.truncate(copied as usize);
    Some(String::from_utf16_lossy(&buffer))
}

fn selected_listbox_text(listbox: Hwnd) -> Option<String> {
    if listbox.is_null() {
        return None;
    }
    // SAFETY: `listbox` is a live listbox HWND.
    let selected = unsafe { SendMessageW(listbox, LB_GETCURSEL, 0, 0) };
    if selected < 0 {
        return None;
    }
    // SAFETY: `listbox` is a live listbox HWND and `selected` is the current selection index.
    let len = unsafe { SendMessageW(listbox, LB_GETTEXTLEN, selected as Wparam, 0) };
    if len <= 0 {
        return None;
    }
    let mut buffer = vec![0u16; len as usize + 1];
    // SAFETY: `buffer` is writable and large enough for the selected list item plus NUL.
    let copied = unsafe {
        SendMessageW(
            listbox,
            LB_GETTEXT,
            selected as Wparam,
            buffer.as_mut_ptr() as Lparam,
        )
    };
    if copied <= 0 {
        return None;
    }
    buffer.truncate(copied as usize);
    Some(String::from_utf16_lossy(&buffer))
}

fn escape_state_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\n', "\\n")
}

fn unescape_state_value(value: &str) -> String {
    let mut result = String::new();
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            match character {
                'n' => result.push('\n'),
                other => result.push(other),
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            result.push(character);
        }
    }
    if escaped {
        result.push('\\');
    }
    result
}

fn child_text(hwnd: Hwnd) -> String {
    // SAFETY: Reads the current text length from a live child HWND.
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; len as usize + 1];
    // SAFETY: `buffer` is writable and large enough for `len + NUL` UTF-16 units.
    let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if copied <= 0 {
        return String::new();
    }
    buffer.truncate(copied as usize);
    String::from_utf16_lossy(&buffer)
}

fn set_child_text(parent: Hwnd, id: i32, text: &str) {
    // SAFETY: Reads the child handle for a QA-visible control id.
    let child = unsafe { GetDlgItem(parent, id) };
    if child.is_null() {
        return;
    }
    let text = to_wide(text);
    // SAFETY: `child` is a live HWND and the UTF-16 buffer lives for this call.
    unsafe {
        SetWindowTextW(child, text.as_ptr());
    }
}

fn loword(value: Wparam) -> u16 {
    (value & 0xffff) as u16
}

fn hiword(value: Wparam) -> u16 {
    ((value >> 16) & 0xffff) as u16
}

fn control_id_handle(id: i32) -> *mut c_void {
    if id <= 0 {
        return null_mut();
    }
    id as usize as *mut c_void
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
