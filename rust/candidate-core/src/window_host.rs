//! Minimal Win32 candidate-window host primitives.
//!
//! Rust owns class registration, HWND lifetime, WndProc storage, and the
//! message pump. The C++ callback remains a narrow adapter for direct Fcitx
//! objects and the already-frozen candidate interaction state.

#![deny(unsafe_op_in_unsafe_fn)]

use core::ffi::c_void;

const CS_DROPSHADOW: u32 = 0x0002_0000;
const GWL_USERDATA: i32 = -21;
const SW_SHOWNOACTIVATE: i32 = 4;
const HTCLIENT: isize = 1;
const MA_NOACTIVATE: isize = 3;
const SWP_NOACTIVATE: u32 = 0x0010;
const SWP_NOZORDER: u32 = 0x0004;
const WM_DESTROY: u32 = 0x0002;
const WM_DPICHANGED: u32 = 0x02E0;
const WM_MOUSEACTIVATE: u32 = 0x0021;
const WM_NCCREATE: u32 = 0x0081;
const WM_NCDESTROY: u32 = 0x0082;
const WM_NCHITTEST: u32 = 0x0084;
const WM_QUIT_ERROR: i32 = 1;
const WS_EX_LAYERED: u32 = 0x0008_0000;
const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
const WS_EX_TOPMOST: u32 = 0x0000_0008;
const WS_POPUP: u32 = 0x8000_0000;

type Hwnd = *mut c_void;
type Hdc = *mut c_void;
type WndProc = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize;
pub type CandidateWindowCallback =
    unsafe extern "system" fn(*mut c_void, Hwnd, u32, usize, isize) -> isize;

const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
const SRCCOPY: u32 = 0x00CC_0020;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct BitmapInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[repr(C)]
struct BitmapInfo {
    bmi_header: BitmapInfoHeader,
    bmi_colors: [u32; 1],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Msg {
    hwnd: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    point: Point,
}

#[repr(C)]
struct WndClassW {
    style: u32,
    window_proc: Option<WndProc>,
    class_extra: i32,
    window_extra: i32,
    instance: *mut c_void,
    icon: *mut c_void,
    cursor: *mut c_void,
    background: *mut c_void,
    menu_name: *const u16,
    class_name: *const u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct CreateStructW {
    create_params: *mut c_void,
    instance: *mut c_void,
    menu: *mut c_void,
    parent: Hwnd,
    height: i32,
    width: i32,
    y: i32,
    x: i32,
    style: i32,
    name: *const u16,
    class: *const u16,
    extended_style: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Fcitx5CandidateWindowCreateInput {
    pub instance: *mut c_void,
    pub owner: *mut c_void,
    pub callback: Option<CandidateWindowCallback>,
    pub class_name: *const u16,
    pub class_name_len: usize,
    pub visible: u8,
    pub interaction_test: u8,
}

struct WindowHost {
    owner: *mut c_void,
    callback: CandidateWindowCallback,
    creation_complete: bool,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn CreateWindowExW(
        extended_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: *mut c_void,
        instance: *mut c_void,
        parameter: *mut c_void,
    ) -> Hwnd;
    fn DefWindowProcW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> isize;
    fn DestroyWindow(window: Hwnd) -> i32;
    fn DispatchMessageW(message: *const Msg) -> isize;
    fn GetMessageW(message: *mut Msg, window: Hwnd, min: u32, max: u32) -> i32;
    fn GetDC(window: Hwnd) -> Hdc;
    fn ReleaseDC(window: Hwnd, dc: Hdc) -> i32;
    fn GetClientRect(window: Hwnd, rect: *mut Rect) -> i32;
    #[cfg(target_pointer_width = "64")]
    fn GetWindowLongPtrW(window: Hwnd, index: i32) -> isize;
    #[cfg(target_pointer_width = "32")]
    fn GetWindowLongW(window: Hwnd, index: i32) -> i32;
    fn LoadCursorW(instance: *mut c_void, cursor: *const u16) -> *mut c_void;
    fn RegisterClassW(class: *const WndClassW) -> u16;
    #[cfg(target_pointer_width = "64")]
    fn SetWindowLongPtrW(window: Hwnd, index: i32, value: isize) -> isize;
    #[cfg(target_pointer_width = "32")]
    fn SetWindowLongW(window: Hwnd, index: i32, value: i32) -> i32;
    fn SetWindowPos(
        window: Hwnd,
        insert_after: Hwnd,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn ShowWindow(window: Hwnd, command: i32) -> i32;
    fn PostQuitMessage(exit_code: i32);
    fn TranslateMessage(message: *const Msg) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateDIBSection(
        dc: Hdc,
        bmi: *const BitmapInfo,
        usage: u32,
        bits: *mut *mut c_void,
        section: *mut c_void,
        offset: u32,
    ) -> *mut c_void;
    fn CreateCompatibleDC(dc: Hdc) -> Hdc;
    #[allow(dead_code)]
    fn DeleteDC(dc: Hdc) -> i32;
    fn DeleteObject(object: *mut c_void) -> i32;
    fn SelectObject(dc: Hdc, object: *mut c_void) -> *mut c_void;
    fn StretchBlt(
        dest: Hdc,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        src: Hdc,
        x1: i32,
        y1: i32,
        w1: i32,
        h1: i32,
        rop: u32,
    ) -> i32;
}

fn quit_exit_code(wparam: usize) -> i32 {
    i32::try_from(wparam).unwrap_or(WM_QUIT_ERROR)
}

// SAFETY: `window` is a valid HWND whose user-data slot may be queried.
unsafe fn window_user_data(window: Hwnd) -> isize {
    #[cfg(target_pointer_width = "64")]
    {
        // SAFETY: window and GWL_USERDATA are passed directly to user32.
        return unsafe { GetWindowLongPtrW(window, GWL_USERDATA) };
    }
    #[cfg(target_pointer_width = "32")]
    {
        // SAFETY: x86 Win32 exposes the documented GetWindowLongW macro target.
        return unsafe { GetWindowLongW(window, GWL_USERDATA) as isize };
    }
}

// SAFETY: `window` is valid and `value` is the caller's pointer-sized user-data value.
unsafe fn set_window_user_data(window: Hwnd, value: isize) {
    #[cfg(target_pointer_width = "64")]
    {
        // SAFETY: window and GWL_USERDATA are passed directly to user32.
        unsafe { SetWindowLongPtrW(window, GWL_USERDATA, value) };
    }
    #[cfg(target_pointer_width = "32")]
    {
        // SAFETY: x86 pointer-sized values fit i32 and use the Win32 macro target.
        unsafe { SetWindowLongW(window, GWL_USERDATA, value as i32) };
    }
}

unsafe extern "system" fn candidate_window_procedure(
    window: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if message == WM_NCCREATE {
        if lparam == 0 {
            return 0;
        }
        // SAFETY: Windows supplies CREATESTRUCTW for WM_NCCREATE.
        let host = unsafe {
            (*(lparam as *const CreateStructW))
                .create_params
                .cast::<WindowHost>()
        };
        if host.is_null() {
            return 0;
        }
        // SAFETY: `host` comes from Box::into_raw in the create export and
        // remains owned by this HWND until WM_NCDESTROY.
        // SAFETY: this valid HWND receives the Box handle created for this exact WM_NCCREATE.
        unsafe { set_window_user_data(window, host.cast::<()>() as isize) };
        return 1;
    }

    // SAFETY: GWLP_USERDATA is either zero or the WindowHost stored above.
    let host = unsafe { window_user_data(window) as *mut WindowHost };
    if host.is_null() {
        // SAFETY: forwards an unowned message to the default window procedure.
        return unsafe { DefWindowProcW(window, message, wparam, lparam) };
    }

    match message {
        WM_MOUSEACTIVATE => return MA_NOACTIVATE,
        WM_NCHITTEST => return HTCLIENT,
        WM_DPICHANGED => {
            if lparam != 0 {
                // SAFETY: WM_DPICHANGED supplies a valid suggested RECT.
                let suggested = unsafe { &*(lparam as *const Rect) };
                // SAFETY: `window` is the active HWND and the suggested values
                // come from Windows for this exact message.
                unsafe {
                    SetWindowPos(
                        window,
                        core::ptr::null_mut(),
                        suggested.left,
                        suggested.top,
                        suggested.right - suggested.left,
                        suggested.bottom - suggested.top,
                        SWP_NOACTIVATE | SWP_NOZORDER,
                    )
                };
            }
            return 0;
        }
        WM_DESTROY => {
            // SAFETY: posts a thread-local quit message without dereferencing pointers.
            unsafe { PostQuitMessage(0) };
            return 0;
        }
        _ => {}
    }

    // SAFETY: `host` is valid until this WM_NCDESTROY handler releases it.
    let result = unsafe { ((*host).callback)((*host).owner, window, message, wparam, lparam) };
    if message == WM_NCDESTROY {
        // SAFETY: stops future lookups before releasing the one Box allocation
        // associated with this HWND.
        unsafe { set_window_user_data(window, 0) };
        // If CreateWindowExW later reports failure, its caller still owns the
        // Box. Successful creation transfers ownership to WM_NCDESTROY.
        // SAFETY: `host` remains valid until this WM_NCDESTROY handler releases it.
        if unsafe { (*host).creation_complete } {
            // SAFETY: WindowHost was allocated with Box::into_raw exactly once.
            unsafe { drop(Box::from_raw(host)) };
        }
    }
    result
}

/// Creates the shipping Candidate HWND with Rust-owned registration and WndProc.
///
/// # Safety
///
/// `input` and `out_window` must point to writable C-compatible structures.
/// `input.class_name` must identify `class_name_len` valid UTF-16 code units;
/// the callback and owner must remain valid until the window is destroyed.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_window_create(
    input: *const Fcitx5CandidateWindowCreateInput,
    out_window: *mut Hwnd,
) -> u8 {
    if input.is_null() || out_window.is_null() {
        return 0;
    }
    // SAFETY: non-null checked above; callers provide one initialized input.
    let input = unsafe { *input };
    let Some(callback) = input.callback else {
        return 0;
    };
    if input.owner.is_null()
        || input.class_name.is_null()
        || input.class_name_len == 0
        || input.class_name_len > 128
    {
        return 0;
    }
    // SAFETY: the C ABI contract requires this UTF-16 slice to be valid.
    let class_name = unsafe { core::slice::from_raw_parts(input.class_name, input.class_name_len) };
    if class_name.contains(&0) {
        return 0;
    }
    let mut registered_name = Vec::with_capacity(class_name.len() + 1);
    registered_name.extend_from_slice(class_name);
    registered_name.push(0);
    let class = WndClassW {
        style: CS_DROPSHADOW,
        window_proc: Some(candidate_window_procedure),
        class_extra: 0,
        window_extra: 0,
        instance: input.instance,
        icon: core::ptr::null_mut(),
        // SAFETY: IDC_ARROW is a documented MAKEINTRESOURCE value.
        cursor: unsafe { LoadCursorW(core::ptr::null_mut(), 32512usize as *const u16) },
        background: core::ptr::null_mut(),
        menu_name: core::ptr::null(),
        class_name: registered_name.as_ptr(),
    };
    // SAFETY: WndClassW and its class-name storage remain valid for the call.
    unsafe { RegisterClassW(&class) };

    let mut extended_style = WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST;
    if input.interaction_test == 0 {
        extended_style |= WS_EX_LAYERED;
    }
    let host = Box::new(WindowHost {
        owner: input.owner,
        callback,
        creation_complete: false,
    });
    let raw_host = Box::into_raw(host);
    const EMPTY_WINDOW_NAME: [u16; 1] = [0];
    // SAFETY: arguments are valid Win32 values; `raw_host` is retained by the
    // WndProc and released at WM_NCDESTROY after successful creation.
    let window = unsafe {
        CreateWindowExW(
            extended_style,
            registered_name.as_ptr(),
            EMPTY_WINDOW_NAME.as_ptr(),
            WS_POPUP,
            100,
            100,
            360,
            120,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            input.instance,
            raw_host.cast(),
        )
    };
    if window.is_null() {
        // The WndProc leaves the Box owned by this caller until CreateWindowExW
        // reports success, so this is correct both before and after WM_NCCREATE.
        // SAFETY: `raw_host` has not transferred ownership on a failed create.
        unsafe { drop(Box::from_raw(raw_host)) };
        return 0;
    }
    // SAFETY: successful creation transfers ownership to WM_NCDESTROY.
    unsafe { (*raw_host).creation_complete = true };
    if input.visible != 0 {
        // SAFETY: `window` is a valid HWND returned above.
        unsafe { ShowWindow(window, SW_SHOWNOACTIVATE) };
    }
    // SAFETY: caller provided writable output storage.
    unsafe { *out_window = window };
    1
}

/// Destroys a Candidate HWND created by [`fcitx5_candidate_window_create`].
#[no_mangle]
pub extern "C" fn fcitx5_candidate_window_destroy(window: Hwnd) {
    if !window.is_null() {
        // SAFETY: caller supplies an HWND created by this host; Windows runs
        // WM_NCDESTROY, which releases the associated WindowHost.
        // SAFETY: this host owns the valid HWND and its WM_NCDESTROY releases associated state.
        unsafe { DestroyWindow(window) };
    }
}

/// Runs the calling thread's Win32 message pump.
///
/// Returns `WM_QUIT`'s exit code, or `1` when `GetMessageW` fails.
#[no_mangle]
pub extern "C" fn fcitx5_candidate_window_run_message_loop() -> i32 {
    let mut message = Msg::default();
    loop {
        // SAFETY: `message` is valid writable storage and the null HWND with
        // zero message bounds requests this thread's complete queue.
        match unsafe { GetMessageW(&mut message, core::ptr::null_mut(), 0, 0) } {
            1 => {
                // SAFETY: GetMessageW initialized `message` before returning 1.
                unsafe { TranslateMessage(&message) };
                // SAFETY: GetMessageW initialized `message` before returning 1.
                unsafe { DispatchMessageW(&message) };
            }
            0 => return quit_exit_code(message.wparam),
            _ => return WM_QUIT_ERROR,
        }
    }
}

/// Blits a top-down BGRA buffer onto a window DC (081C paint slice).
///
/// The C++ `paintOnce` DIB path moves here verbatim: allocate a top-down DIB
/// from the buffer's stride/byte count, memcpy, and `StretchBlt` to the client
/// rect. Any invalid descriptor is a silent no-op, exactly like the C++
/// early-return.
///
/// # Safety
///
/// `window` must be a valid HWND, `pixels` must reference `pixel_byte_len`
/// readable bytes, and `out_client` must point to writable storage for the
/// client-rect query result when `query_client` is non-zero.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_window_blit_bgra(
    window: Hwnd,
    pixels: *const u8,
    pixel_byte_len: usize,
    pixel_stride: usize,
    out_client: *mut i32,
) -> u8 {
    if window.is_null() || pixels.is_null() || out_client.is_null() {
        return 0;
    }
    if pixel_stride == 0
        || pixel_byte_len == 0
        || pixel_stride % 4 != 0
        || pixel_byte_len % pixel_stride != 0
    {
        return 0;
    }
    // SAFETY: `window` is a valid HWND per the caller contract.
    let mut client = Rect::default();
    // SAFETY: `window` is a live HWND and `client` is writable RECT storage.
    if unsafe { GetClientRect(window, &mut client) } == 0 {
        return 0;
    }
    let pix_w = (pixel_stride / 4) as i32;
    let pix_h = (pixel_byte_len / pixel_stride) as i32;
    if pix_w <= 0 || pix_h <= 0 {
        return 0;
    }
    // SAFETY: `window` is a valid HWND per the caller contract.
    let dc = unsafe { GetDC(window) };
    if dc.is_null() {
        return 0;
    }
    let bmi = BitmapInfo {
        bmi_header: BitmapInfoHeader {
            bi_size: core::mem::size_of::<BitmapInfoHeader>() as u32,
            bi_width: pix_w,
            bi_height: -pix_h,
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: BI_RGB,
            ..BitmapInfoHeader::default()
        },
        bmi_colors: [0; 1],
    };
    let mut bits: *mut c_void = core::ptr::null_mut();
    // SAFETY: `dc` is valid and `bits` receives the DIB allocation pointer.
    let bitmap = unsafe {
        CreateDIBSection(
            dc,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            core::ptr::null_mut(),
            0,
        )
    };
    let blitted = if !bitmap.is_null() && !bits.is_null() {
        // SAFETY: `bits` covers pixel_byte_len bytes and pixels has that length.
        unsafe {
            core::ptr::copy_nonoverlapping(pixels.cast::<u8>(), bits.cast::<u8>(), pixel_byte_len);
        }
        // SAFETY: `dc` is a valid HDC.
        let mem_dc = unsafe { CreateCompatibleDC(dc) };
        if mem_dc.is_null() {
            false
        } else {
            // SAFETY: both DC and bitmap are valid.
            let old = unsafe { SelectObject(mem_dc, bitmap) };
            // SAFETY: both DCs are valid with matching pixel formats.
            let drawn = unsafe {
                StretchBlt(
                    dc,
                    0,
                    0,
                    client.right,
                    client.bottom,
                    mem_dc,
                    0,
                    0,
                    pix_w,
                    pix_h,
                    SRCCOPY,
                )
            };
            // SAFETY: restore the original bitmap selection.
            unsafe { SelectObject(mem_dc, old) };
            drawn != 0
        }
    } else {
        false
    };
    if !bitmap.is_null() {
        // SAFETY: `bitmap` is owned by this call and not selected anywhere else.
        unsafe { DeleteObject(bitmap) };
    }
    // SAFETY: `dc` came from GetDC above.
    unsafe { ReleaseDC(window, dc) };
    // SAFETY: non-null checked above; caller provides writable output storage.
    unsafe {
        *out_client = client.right;
    }
    u8::from(blitted)
}

/// Blits a top-down BGRA buffer onto a caller-supplied DC (WM_PRINT path).
///
/// Same DIB semantics as [`fcitx5_candidate_window_blit_bgra`] but the
/// destination DC is provided by the caller (e.g. the WM_PRINT wparam).
///
/// # Safety
///
/// `dc` must be a valid HDC, `pixels` must reference `pixel_byte_len`
/// readable bytes, and `client_width`/`client_height` must be the target
/// client-rect extent.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_window_blit_bgra_to_dc(
    dc: Hdc,
    pixels: *const u8,
    pixel_byte_len: usize,
    pixel_stride: usize,
    client_width: i32,
    client_height: i32,
) -> u8 {
    // SAFETY: callers uphold the documented pointer/DC contracts.
    unsafe {
        blit_bgra_to_dc(
            dc,
            pixels,
            pixel_byte_len,
            pixel_stride,
            client_width,
            client_height,
        )
    }
}

/// Shared implementation for the exported blit entry points.
///
/// # Safety
///
/// Same contracts as [`fcitx5_candidate_window_blit_bgra_to_dc`].
// SAFETY: `dc` is a live device context and `pixels` names the validated BGRA span below.
pub(crate) unsafe fn blit_bgra_to_dc(
    dc: Hdc,
    pixels: *const u8,
    pixel_byte_len: usize,
    pixel_stride: usize,
    client_width: i32,
    client_height: i32,
) -> u8 {
    if dc.is_null() || pixels.is_null() {
        return 0;
    }
    if pixel_stride == 0
        || pixel_byte_len == 0
        || pixel_stride % 4 != 0
        || pixel_byte_len % pixel_stride != 0
    {
        return 0;
    }
    let pix_w = (pixel_stride / 4) as i32;
    let pix_h = (pixel_byte_len / pixel_stride) as i32;
    if pix_w <= 0 || pix_h <= 0 {
        return 0;
    }
    let bmi = BitmapInfo {
        bmi_header: BitmapInfoHeader {
            bi_size: core::mem::size_of::<BitmapInfoHeader>() as u32,
            bi_width: pix_w,
            bi_height: -pix_h,
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: BI_RGB,
            ..BitmapInfoHeader::default()
        },
        bmi_colors: [0; 1],
    };
    let mut bits: *mut c_void = core::ptr::null_mut();
    // SAFETY: `dc` is valid and `bits` receives the DIB allocation pointer.
    let bitmap = unsafe {
        CreateDIBSection(
            dc,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            core::ptr::null_mut(),
            0,
        )
    };
    if bitmap.is_null() || bits.is_null() {
        if !bitmap.is_null() {
            // SAFETY: `bitmap` is owned by this call.
            unsafe { DeleteObject(bitmap) };
        }
        return 0;
    }
    // SAFETY: `bits` covers pixel_byte_len bytes and pixels has that length.
    unsafe {
        core::ptr::copy_nonoverlapping(pixels.cast::<u8>(), bits.cast::<u8>(), pixel_byte_len);
    }
    // SAFETY: `dc` is a valid HDC.
    let mem_dc = unsafe { CreateCompatibleDC(dc) };
    if mem_dc.is_null() {
        // SAFETY: `bitmap` is owned by this call.
        unsafe { DeleteObject(bitmap) };
        return 0;
    }
    // SAFETY: both DC and bitmap are valid.
    let old = unsafe { SelectObject(mem_dc, bitmap) };
    // SAFETY: both DCs are valid with matching pixel formats.
    let drawn = unsafe {
        StretchBlt(
            dc,
            0,
            0,
            client_width,
            client_height,
            mem_dc,
            0,
            0,
            pix_w,
            pix_h,
            SRCCOPY,
        )
    };
    // SAFETY: restore the original bitmap selection.
    unsafe { SelectObject(mem_dc, old) };
    // SAFETY: `mem_dc` came from CreateCompatibleDC above.
    unsafe { DeleteDC(mem_dc) };
    // SAFETY: `bitmap` is owned by this call.
    unsafe { DeleteObject(bitmap) };
    u8::from(drawn != 0)
}

#[cfg(test)]
mod tests {
    use super::quit_exit_code;

    #[test]
    fn quit_exit_code_rejects_values_outside_i32() {
        assert_eq!(quit_exit_code(42), 42);
        assert_eq!(quit_exit_code(usize::MAX), 1);
    }
}
