//! 082: Rust shipping candidate-UI host binary.
//!
//! Owns the full candidate-window runtime: command line, visual config
//! consumption, window creation through the Rust window host, the frame
//! update orchestration, paint, interaction, the presentation pipe server,
//! and the mixed-binary self-test surface. Unsafe is limited to narrow
//! Win32 FFI adapters (the `ffi` module and window-entry shims), each with
//! a SAFETY comment.
#![deny(unsafe_op_in_unsafe_fn)]
// The shipping candidate window is a GUI-subsystem binary (no console), like
// the deleted C++ wWinMain host; self-test diagnostics use exit codes.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use std::collections::BTreeMap;
use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use fcitx5_candidate_core::frame_update::{
    frame_update, FrameCaret, FrameConfig, FrameOrientation, FrameOverflow, FrameState,
    FrameUpdateOutcome, FrameWriting,
};
use fcitx5_candidate_core::presentation_server::fcitx5_candidate_presentation_serve;
use fcitx5_candidate_core::window_host::{
    fcitx5_candidate_window_create, fcitx5_candidate_window_destroy,
    fcitx5_candidate_window_run_message_loop, Fcitx5CandidateWindowCreateInput,
};
use fcitx5_candidate_core::{
    content_locale_or_default, content_locale_valid, default_dwrite_locale,
    fcitx5_candidate_render_window_blit_to_dc, fcitx5_candidate_selection_intent,
    parse_candidate_command_line, resolve_paint_colors, selection_intent_valid,
    CandidateClickGuardState, CandidateFocusWatchState, CandidateModel, CandidatePointerState,
    CandidatePresentationState, CandidateScrollState, CandidateVisualArena,
    Fcitx5CandidateConfigColor, Fcitx5CandidateLayoutSize, Fcitx5CandidateRenderCandidateInput,
    Fcitx5CandidateRenderGeometryInput, Fcitx5CandidateRenderThemeInput,
    Fcitx5CandidateResolvedColors, Fcitx5CandidateSelectionIntent,
    Fcitx5CandidateVisualBuildOutput, Rect as FRect,
};
use fcitx5_config_core::{ConfigCore, FileStore, VisualSnapshotRequest};
use fcitx5_protocol_core::{CandidateRecord, CaretRect, KeyResponse, Status};
use fcitx5_windows_common_core::CandidateSelectClient;
use fcitx5_windows_common_core::{
    current_runtime_generation_for_current_process, local_data_directory,
    release_local_object_prefix, system_uses_dark_appearance, CurrentUserRuntimeIdentity,
};

// ---------------------------------------------------------------------------
// Win32 FFI (narrow adapters only)
// ---------------------------------------------------------------------------
mod ffi {
    use std::ffi::c_void;

    pub type Hwnd = *mut c_void;
    pub type Hdc = *mut c_void;
    pub type Hmonitor = *mut c_void;

    pub const WM_CLOSE: u32 = 0x0010;
    pub const WM_PAINT: u32 = 0x000F;
    pub const WM_TIMER: u32 = 0x0113;
    pub const WM_SETTINGCHANGE: u32 = 0x001A;
    pub const WM_SYSCOLORCHANGE: u32 = 0x0015;
    pub const WM_THEMECHANGED: u32 = 0x031A;
    pub const WM_PRINT: u32 = 0x0317;
    pub const WM_PRINTCLIENT: u32 = 0x0318;
    pub const WM_LBUTTONDOWN: u32 = 0x0201;
    pub const WM_LBUTTONUP: u32 = 0x0202;
    pub const WM_MOUSEWHEEL: u32 = 0x020A;
    pub const WM_CANCELMODE: u32 = 0x001F;
    pub const WM_CAPTURECHANGED: u32 = 0x0215;
    pub const WM_KEYDOWN: u32 = 0x0100;
    pub const WM_NCDESTROY: u32 = 0x0082;
    pub const WM_APP: u32 = 0x8000;
    pub const SW_HIDE: i32 = 0;
    pub const SWP_NOACTIVATE: u32 = 0x0010;
    pub const SWP_SHOWWINDOW: u32 = 0x0040;
    pub const HWND_TOPMOST: isize = -1;
    pub const LWA_ALPHA: u32 = 2;
    pub const SPI_GETHIGHCONTRAST: u32 = 0x0042;
    pub const HCF_HIGHCONTRASTON: u32 = 1;
    pub const MONITOR_DEFAULTTONEAREST: u32 = 2;
    pub const SM_CXSCREEN: i32 = 0;
    pub const RDW_INVALIDATE: u32 = 0x1;
    pub const RDW_UPDATENOW: u32 = 0x100;
    pub const MK_LBUTTON: usize = 1;
    pub const VK_PRIOR: usize = 0x21;
    pub const VK_NEXT: usize = 0x22;
    pub const SYNCHRONIZE: u32 = 0x0010_0000;
    pub const GWL_EXSTYLE: i32 = -20;
    pub const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
    pub const WS_EX_NOACTIVATE: isize = 0x0800_0000;
    pub const WS_EX_APPWINDOW: isize = 0x0004_0000;
    pub const CSTR_EQUAL: i32 = 2;
    pub const INFINITE: u32 = 0xFFFF_FFFF;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rect {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }

    #[repr(C)]
    pub struct MonitorInfo {
        pub cb_size: u32,
        pub monitor: Rect,
        pub work: Rect,
        pub flags: u32,
    }

    #[repr(C)]
    pub struct HighContrastW {
        pub cb_size: u32,
        pub dw_flags: u32,
        pub lpsz_default_scheme: *const u16,
    }

    #[repr(C)]
    pub struct PaintStruct {
        pub hdc: Hdc,
        pub f_erase: i32,
        pub rc_paint: Rect,
        pub f_restore: i32,
        pub f_inc_update: i32,
        pub rgb_reserved: [u8; 32],
    }

    extern "system" {
        pub fn BeginPaint(window: Hwnd, paint: *mut PaintStruct) -> Hdc;
        pub fn ClientToScreen(window: Hwnd, point: *mut Point) -> i32;
        pub fn CloseHandle(handle: *mut c_void) -> i32;
        pub fn CompareStringOrdinal(
            first: *const u16,
            first_len: i32,
            second: *const u16,
            second_len: i32,
            ignore_case: i32,
        ) -> i32;
        pub fn CreateSolidBrush(color: u32) -> *mut c_void;
        pub fn DeleteObject(object: *mut c_void) -> i32;
        pub fn EndPaint(window: Hwnd, paint: *const PaintStruct) -> i32;
        pub fn FillRect(dc: Hdc, rect: *const Rect, brush: *mut c_void) -> i32;
        pub fn GetCapture() -> Hwnd;
        pub fn GetClientRect(window: Hwnd, rect: *mut Rect) -> i32;
        pub fn FreeLibrary(module: *mut c_void) -> i32;
        pub fn GetDC(window: Hwnd) -> Hdc;
        pub fn GetForegroundWindow() -> Hwnd;
        pub fn GetModuleHandleW(name: *const u16) -> *mut c_void;
        pub fn GetMonitorInfoW(monitor: Hmonitor, info: *mut MonitorInfo) -> i32;
        pub fn GetSystemMetrics(index: i32) -> i32;
        // x86 has no GetWindowLongPtrW export (it is a Win32 macro for
        // GetWindowLongW); mirror the window_host cfg split.
        #[cfg(target_pointer_width = "64")]
        pub fn GetWindowLongPtrW(window: Hwnd, index: i32) -> isize;
        #[cfg(target_pointer_width = "32")]
        pub fn GetWindowLongW(window: Hwnd, index: i32) -> i32;
        pub fn GetWindowRect(window: Hwnd, rect: *mut Rect) -> i32;
        pub fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
        pub fn GetWindowThreadProcessId(window: Hwnd, process_id: *mut u32) -> u32;
        pub fn InvalidateRect(window: Hwnd, rect: *const Rect, erase: i32) -> i32;
        pub fn IsRectEmpty(rect: *const Rect) -> i32;
        pub fn IsWindowVisible(window: Hwnd) -> i32;
        pub fn KillTimer(window: Hwnd, event: usize) -> i32;
        pub fn LoadLibraryW(name: *const u16) -> *mut c_void;
        pub fn MonitorFromPoint(point: Point, flags: u32) -> Hmonitor;
        pub fn OpenProcess(access: u32, inherit: i32, process_id: u32) -> *mut c_void;
        pub fn PostMessageW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> i32;
        pub fn RedrawWindow(
            window: Hwnd,
            update: *const Rect,
            region: *mut c_void,
            flags: u32,
        ) -> i32;
        pub fn RegisterWindowMessageW(name: *const u16) -> u32;
        pub fn ReleaseCapture() -> i32;
        pub fn ReleaseDC(window: Hwnd, dc: Hdc) -> i32;
        pub fn SendMessageW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> isize;
        pub fn SetCapture(window: Hwnd) -> Hwnd;
        pub fn SetEnvironmentVariableW(name: *const u16, value: *const u16) -> i32;
        pub fn SetLayeredWindowAttributes(window: Hwnd, key: u32, alpha: u8, flags: u32) -> i32;
        pub fn SetProcessDPIAware() -> i32;
        pub fn SetTimer(
            window: Hwnd,
            event: usize,
            milliseconds: u32,
            callback: *mut c_void,
        ) -> usize;
        pub fn SetWindowPos(
            window: Hwnd,
            after: isize,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
        pub fn ShowWindow(window: Hwnd, command: i32) -> i32;
        pub fn SystemParametersInfoW(action: u32, param: u32, data: *mut c_void, init: u32) -> i32;
        pub fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
    }
}

use ffi::*;

const K_SNAPSHOT_MESSAGE: u32 = WM_APP + 1;
const K_FOCUS_WATCH_TIMER: usize = 1;
const K_CLICK_GUARD_TIMER: usize = 2;
const K_FOCUS_WATCH_INTERVAL_MS: u32 = 100;
const K_CLICK_GUARD_INTERVAL_MS: u32 = 750;

const VISUAL_CONFIG_CHANGED_NAME: &str = "Fcitx5WindowsNext.VisualConfigChanged.v1";
const CANDIDATE_DISMISS_NAME: &str = "Fcitx5WindowsNext.CandidateDismiss.v1";

fn candidate_dismiss_message() -> u32 {
    registered_message(CANDIDATE_DISMISS_NAME)
}

fn registered_message(name: &str) -> u32 {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: name is a NUL-terminated UTF-16 buffer.
    unsafe { RegisterWindowMessageW(wide.as_ptr()) }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn wide_nul(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn os_to_wide(value: &OsStr) -> Option<Vec<u16>> {
    let units: Vec<u16> = value.encode_wide().collect();
    if units.is_empty() || units.contains(&0) {
        return None;
    }
    Some(units)
}

fn strip_nul(units: &[u16]) -> &[u16] {
    let mut end = units.len();
    while end > 0 && units[end - 1] == 0 {
        end -= 1;
    }
    &units[..end]
}

fn wide_eq_ordinal(a: &[u16], b: &[u16]) -> bool {
    let a = strip_nul(a);
    let b = strip_nul(b);
    if a.is_empty() && b.is_empty() {
        return true;
    }
    if a.is_empty() || b.is_empty() {
        return false;
    }
    // SAFETY: both slices are valid UTF-16 code-unit buffers.
    unsafe {
        CompareStringOrdinal(a.as_ptr(), a.len() as i32, b.as_ptr(), b.len() as i32, 1)
            == CSTR_EQUAL
    }
}

// ---------------------------------------------------------------------------
// Visual config (Config Core owns parsing/validation; this only projects the
// resolved values the host consumes)
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Debug)]
struct Rgba {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

fn parse_color(value: &str) -> Option<Rgba> {
    let bytes = value.as_bytes();
    if (bytes.len() != 7 && bytes.len() != 9) || bytes[0] != b'#' {
        return None;
    }
    let nibble = |b: u8| -> Option<f32> {
        match b {
            b'0'..=b'9' => Some((b - b'0') as f32),
            b'a'..=b'f' => Some((b - b'a' + 10) as f32),
            b'A'..=b'F' => Some((b - b'A' + 10) as f32),
            _ => None,
        }
    };
    let byte = |i: usize| -> Option<f32> {
        let high = nibble(bytes[i])?;
        let low = nibble(bytes[i + 1])?;
        Some((high * 16.0 + low) / 255.0)
    };
    Some(Rgba {
        r: byte(1)?,
        g: byte(3)?,
        b: byte(5)?,
        a: if bytes.len() == 9 { byte(7)? } else { 1.0 },
    })
}

#[derive(Clone, Copy, Debug)]
struct PaintColors {
    background: Rgba,
    candidate_text: Rgba,
    selected_background: Rgba,
    selected_candidate_text: Rgba,
    comment_text: Rgba,
    border: Rgba,
    preedit_text: Rgba,
}

impl PaintColors {
    fn from_map(map: &BTreeMap<String, String>) -> Self {
        let default_text = Rgba {
            r: 0.13,
            g: 0.13,
            b: 0.14,
            a: 1.0,
        };
        let named = |name: &str| map.get(name).and_then(|value| parse_color(value));
        let candidate_text = named("candidate_text").unwrap_or(default_text);
        PaintColors {
            background: named("background").unwrap_or(Rgba {
                r: 0.97,
                g: 0.98,
                b: 0.98,
                a: 1.0,
            }),
            candidate_text,
            selected_background: named("selected_background").unwrap_or(Rgba {
                r: 0.027,
                g: 0.757,
                b: 0.376,
                a: 0.10,
            }),
            selected_candidate_text: named("selected_candidate_text").unwrap_or(Rgba {
                r: 0.027,
                g: 0.757,
                b: 0.376,
                a: 1.0,
            }),
            comment_text: named("comment_text").unwrap_or(candidate_text),
            border: named("border").unwrap_or(default_text),
            preedit_text: named("preedit_text").unwrap_or(candidate_text),
        }
    }

    fn to_config_color(color: Rgba) -> Fcitx5CandidateConfigColor {
        Fcitx5CandidateConfigColor {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
}

#[derive(Clone, Debug)]
struct VisualConfig {
    orientation: FrameOrientation,
    overflow: FrameOverflow,
    writing: FrameWriting,
    preedit_panel: bool,
    label_style: u32,
    label_visible: bool,
    scroll_mode: bool,
    max_width_dip: f32,
    padding_x_dip: f32,
    padding_y_dip: f32,
    row_gap_dip: f32,
    column_gap_dip: f32,
    item_padding_x_dip: f32,
    item_padding_y_dip: f32,
    label_gap_dip: f32,
    font_size_dip: f32,
    label_font_scale: f32,
    annotation_font_scale: f32,
    opacity: f32,
    corner_radius_dip: f32,
    colors: PaintColors,
    labels: Vec<String>,
}

fn load_visual_config(safe_mode: bool) -> Option<VisualConfig> {
    let data_root = local_data_directory()?;
    let identity = CurrentUserRuntimeIdentity::current()?;
    let installation = identity.executable_path().parent()?.to_path_buf();
    let current_path = data_root.join("config.toml");
    let store = FileStore::new();
    let request = VisualSnapshotRequest::new(
        &current_path,
        &installation,
        &data_root,
        safe_mode,
        system_uses_dark_appearance(),
    );
    let snapshot = ConfigCore::load_visual_snapshot(&store, request);
    let candidate = snapshot.snapshot().candidate();
    // Layout-type mapping mirrors the frozen C++ decodeNativeLayoutType:
    // `automatic` keeps the presentation-decided axis.
    let (orientation, overflow, writing) = match candidate.layout_type() {
        "stacked" => (
            FrameOrientation::Vertical,
            FrameOverflow::Paging,
            FrameWriting::Horizontal,
        ),
        "flow" => (
            FrameOrientation::Horizontal,
            FrameOverflow::Wrapping,
            FrameWriting::Horizontal,
        ),
        "scroll" => (
            FrameOrientation::Horizontal,
            FrameOverflow::Scrolling,
            FrameWriting::Horizontal,
        ),
        "vertical_text" => (
            FrameOrientation::Vertical,
            FrameOverflow::Paging,
            FrameWriting::VerticalRl,
        ),
        _ => (
            FrameOrientation::Automatic,
            FrameOverflow::Paging,
            FrameWriting::Horizontal,
        ),
    };
    let label_style = match candidate.label().style() {
        "plain" => 0,
        "dot" => 1,
        "paren" => 2,
        "bracket" => 3,
        "circled" => 4,
        _ => 1,
    };
    let geometry = candidate.geometry();
    Some(VisualConfig {
        orientation,
        overflow,
        writing,
        preedit_panel: candidate.preedit_mode() == "panel",
        label_style,
        label_visible: candidate.label().visible(),
        scroll_mode: candidate.scroll_mode(),
        max_width_dip: candidate.max_width_dip(),
        padding_x_dip: geometry.padding_x_dip(),
        padding_y_dip: geometry.padding_y_dip(),
        row_gap_dip: geometry.row_gap_dip(),
        column_gap_dip: geometry.column_gap_dip(),
        item_padding_x_dip: geometry.item_padding_x_dip(),
        item_padding_y_dip: geometry.item_padding_y_dip(),
        label_gap_dip: candidate.label().gap_dip(),
        font_size_dip: snapshot.snapshot().fonts().candidate().size_dip(),
        label_font_scale: candidate.label().font_scale(),
        annotation_font_scale: snapshot.snapshot().fonts().annotation().scale(),
        opacity: candidate.opacity(),
        corner_radius_dip: geometry.corner_radius_dip(),
        colors: PaintColors::from_map(candidate.colors()),
        labels: candidate.label().sequence().to_vec(),
    })
}

// ---------------------------------------------------------------------------
// Host state
// ---------------------------------------------------------------------------
struct Host {
    model: CandidateModel,
    presentation: CandidatePresentationState,
    scroll: CandidateScrollState,
    click_guard: CandidateClickGuardState,
    focus_watch: CandidateFocusWatchState,
    pointer: CandidatePointerState,
    arena: CandidateVisualArena,
    measure: fcitx5_candidate_core::renderer::MeasureEngine,
    window: *mut c_void,
    item_rects: Vec<FRect>,
    visible_indices: Vec<usize>,
    has_scrollbar: bool,
    font_dpi_scale: f32,
    selection_inflate_x: f32,
    selection_inflate_y: f32,
    resolved_horizontal: bool,
    preedit_panel_text: Vec<u8>,
    preedit_panel_rect: Option<(f32, f32, f32, f32)>,
    preedit_divider_y: f32,
    candidate_page_size: u32,
    last_caret: FrameCaret,
    content_locale_utf8: String,
    dwrite_locale: Vec<u16>,
    visual_config: VisualConfig,
    safe_mode: bool,
    interaction_test: bool,
    candidate_client: Option<CandidateSelectClient>,
    captured_test_intent: Option<Fcitx5CandidateSelectionIntent>,
    target_foreground: *mut c_void,
}

impl Host {
    fn new(safe_mode: bool, interaction_test: bool) -> Option<Box<Self>> {
        let visual_config = load_visual_config(safe_mode)?;
        let candidate_client = if interaction_test {
            None
        } else {
            let identity = CurrentUserRuntimeIdentity::current()?;
            let engine = identity
                .executable_path()
                .parent()?
                .join("fcitx5-engine.exe");
            let generation = current_runtime_generation_for_current_process();
            let pipe_name = identity.local_endpoint_name(&generation, "engine")?;
            let pipe_units = os_to_wide(pipe_name.as_os_str())?;
            let engine_units = wide(&engine.to_string_lossy());
            CandidateSelectClient::new(pipe_units, engine_units)
        };
        Some(Box::new(Self {
            model: CandidateModel::default(),
            presentation: CandidatePresentationState::default(),
            scroll: CandidateScrollState::default(),
            click_guard: CandidateClickGuardState::default(),
            focus_watch: CandidateFocusWatchState::default(),
            pointer: CandidatePointerState::default(),
            arena: CandidateVisualArena::new(),
            measure: fcitx5_candidate_core::renderer::MeasureEngine::new(),
            window: ptr::null_mut(),
            item_rects: Vec::new(),
            visible_indices: Vec::new(),
            has_scrollbar: false,
            font_dpi_scale: 1.0,
            selection_inflate_x: 0.0,
            selection_inflate_y: 0.0,
            resolved_horizontal: false,
            preedit_panel_text: Vec::new(),
            preedit_panel_rect: None,
            preedit_divider_y: 0.0,
            candidate_page_size: 0,
            last_caret: FrameCaret::default(),
            content_locale_utf8: String::new(),
            dwrite_locale: default_dwrite_locale(),
            visual_config,
            safe_mode,
            interaction_test,
            candidate_client,
            captured_test_intent: None,
            target_foreground: ptr::null_mut(),
        }))
    }

    fn apply_content_locale(&mut self, locale: &[u8]) {
        self.content_locale_utf8 = if content_locale_valid(locale) {
            String::from_utf8_lossy(locale).into_owned()
        } else {
            String::new()
        };
        let next = content_locale_or_default(self.content_locale_utf8.as_bytes());
        if wide_eq_ordinal(&self.dwrite_locale, &next) {
            return;
        }
        self.dwrite_locale = next;
    }

    fn update(&mut self, response: &KeyResponse) {
        self.apply_content_locale(&response.content_locale_utf8);
        if response.caret.valid {
            self.last_caret = FrameCaret {
                left: response.caret.left as f32,
                top: response.caret.top as f32,
                right: response.caret.right as f32,
                bottom: response.caret.bottom as f32,
                dpi: response.caret.dpi as f32,
                valid: true,
            };
        }
        let requested_font_scale = self.last_caret.dpi / 96.0;
        if requested_font_scale != self.font_dpi_scale {
            self.font_dpi_scale = requested_font_scale;
        }
        self.item_rects.clear();
        self.visible_indices.clear();
        self.has_scrollbar = false;
        self.preedit_panel_text.clear();
        self.preedit_panel_rect = None;
        self.preedit_divider_y = 0.0;

        let caret_point = Point {
            x: self.last_caret.left as i32,
            y: self.last_caret.top as i32,
        };
        let mut monitor_info = MonitorInfo {
            cb_size: std::mem::size_of::<MonitorInfo>() as u32,
            monitor: Rect::default(),
            work: Rect::default(),
            flags: 0,
        };
        // SAFETY: monitor_info is initialized with its size and writable.
        unsafe {
            let monitor = MonitorFromPoint(caret_point, MONITOR_DEFAULTTONEAREST);
            GetMonitorInfoW(monitor, &mut monitor_info);
        }
        let work_area = FRect {
            left: monitor_info.work.left as f32,
            top: monitor_info.work.top as f32,
            right: monitor_info.work.right as f32,
            bottom: monitor_info.work.bottom as f32,
        };
        let focus_pid = if self.interaction_test {
            std::process::id()
        } else {
            let mut process_id = 0_u32;
            // SAFETY: process_id is a valid writable DWORD.
            unsafe {
                let foreground = GetForegroundWindow();
                if !foreground.is_null() {
                    GetWindowThreadProcessId(foreground, &mut process_id);
                }
            }
            process_id
        };
        let config = FrameConfig {
            orientation: self.visual_config.orientation,
            overflow: self.visual_config.overflow,
            writing: self.visual_config.writing,
            preedit_panel: self.visual_config.preedit_panel,
            label_style: self.visual_config.label_style,
            label_visible: self.visual_config.label_visible,
            scroll_mode: self.visual_config.scroll_mode,
            max_width_dip: self.visual_config.max_width_dip,
            padding_x_dip: self.visual_config.padding_x_dip,
            padding_y_dip: self.visual_config.padding_y_dip,
            row_gap_dip: self.visual_config.row_gap_dip,
            column_gap_dip: self.visual_config.column_gap_dip,
            item_padding_x_dip: self.visual_config.item_padding_x_dip,
            item_padding_y_dip: self.visual_config.item_padding_y_dip,
            label_gap_dip: self.visual_config.label_gap_dip,
            font_size_dip: self.visual_config.font_size_dip,
            label_font_scale: self.visual_config.label_font_scale,
            annotation_font_scale: self.visual_config.annotation_font_scale,
        };
        let mut state = FrameState {
            model: &mut self.model,
            presentation: &mut self.presentation,
            scroll: &mut self.scroll,
            click_guard: &mut self.click_guard,
            focus_watch: &mut self.focus_watch,
            arena: &mut self.arena,
            measure: &mut self.measure,
            content_locale: &self.content_locale_utf8,
            configured_labels: &self.visual_config.labels,
        };
        let outcome = frame_update(
            &mut state,
            config,
            response,
            &mut self.last_caret,
            work_area,
            focus_pid,
        );
        let outputs = match outcome {
            FrameUpdateOutcome::Ignored => return,
            FrameUpdateOutcome::Dismiss => {
                self.dismiss_presentation();
                return;
            }
            FrameUpdateOutcome::HidePopup => {
                self.hide_popup();
                return;
            }
            FrameUpdateOutcome::Proceed(outputs) => outputs,
        };
        self.item_rects = outputs.item_rects;
        self.visible_indices = outputs.visible_indices;
        self.has_scrollbar = outputs.has_scrollbar;
        self.font_dpi_scale = outputs.font_scale;
        self.selection_inflate_x = outputs.selection_inflate_x;
        self.selection_inflate_y = outputs.selection_inflate_y;
        self.resolved_horizontal = outputs.horizontal;
        if let Some(rect) = outputs.preedit_panel {
            self.preedit_panel_rect = Some((rect.left, rect.top, rect.right, rect.bottom));
            self.preedit_divider_y = outputs.preedit_divider_y;
            self.preedit_panel_text = outputs.preedit_utf8;
        }
        self.candidate_page_size = response.candidate_page_size;
        self.target_foreground =
            // SAFETY: plain Win32 query.
            unsafe { GetForegroundWindow() };
        let window = self.window;
        // SAFETY: window is a valid HWND created by the Rust window host.
        unsafe {
            SetWindowPos(
                window,
                HWND_TOPMOST,
                outputs.window_left as i32,
                outputs.window_top as i32,
                outputs.window_width as i32,
                outputs.window_height as i32,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            InvalidateRect(window, ptr::null(), 0);
        }
    }

    fn hide_popup(&mut self) {
        let window = self.window;
        // SAFETY: window is a valid HWND.
        unsafe {
            ShowWindow(window, SW_HIDE);
            if GetCapture() == window {
                ReleaseCapture();
            }
            KillTimer(window, K_CLICK_GUARD_TIMER);
        }
        self.pointer.clear();
        self.click_guard.clear();
    }

    fn dismiss_presentation(&mut self) {
        self.hide_popup();
        self.presentation.reset();
        self.model.reset();
        self.item_rects.clear();
        self.visible_indices.clear();
        self.has_scrollbar = false;
        self.preedit_panel_text.clear();
        self.preedit_panel_rect = None;
        self.preedit_divider_y = 0.0;
        self.resolved_horizontal = false;
        self.target_foreground = ptr::null_mut();
        self.focus_watch.reset();
    }

    fn focus_target_process_id(&self) -> u32 {
        self.focus_watch.target()
    }

    fn foreground_target_is_valid(&self) -> bool {
        let mut process_id = 0_u32;
        // SAFETY: process_id is a valid writable DWORD.
        unsafe {
            let foreground = GetForegroundWindow();
            if !foreground.is_null() {
                GetWindowThreadProcessId(foreground, &mut process_id);
            }
        }
        self.focus_watch.is_valid(process_id, self.interaction_test)
    }

    fn dispatch_candidate(&mut self, local_index: usize) -> bool {
        if local_index >= self.visible_indices.len() || !self.foreground_target_is_valid() {
            return false;
        }
        let target_index = self.visible_indices[local_index];
        let Some(current) = self.model.semantic_snapshot() else {
            return false;
        };
        let Some(candidate) = current.candidates.get(target_index) else {
            return false;
        };
        let intent = fcitx5_candidate_selection_intent(
            self.focus_target_process_id(),
            current.identity.engine_epoch,
            current.identity.context_id,
            current.identity.composition_id,
            current.identity.revision,
            candidate.id,
        );
        if !selection_intent_valid(intent) {
            return false;
        }
        if !self.click_guard.begin() {
            return false;
        }
        let window = self.window;
        // SAFETY: window is a valid HWND.
        unsafe {
            SetTimer(
                window,
                K_CLICK_GUARD_TIMER,
                K_CLICK_GUARD_INTERVAL_MS,
                ptr::null_mut(),
            );
        }
        if self.interaction_test {
            self.captured_test_intent = Some(intent);
            return true;
        }
        let Some(client) = self.candidate_client.as_mut() else {
            self.click_guard.clear();
            // SAFETY: window is a valid HWND.
            unsafe { KillTimer(window, K_CLICK_GUARD_TIMER) };
            return false;
        };
        if !client.select(
            intent.target_process_id,
            intent.engine_epoch,
            intent.context_id,
            intent.composition_id,
            intent.revision,
            intent.candidate_id,
        ) {
            self.click_guard.clear();
            // SAFETY: window is a valid HWND.
            unsafe { KillTimer(window, K_CLICK_GUARD_TIMER) };
            return false;
        }
        true
    }

    // -- paint --------------------------------------------------------------
    fn paint_once(&mut self) -> bool {
        let window = self.window;
        let mut client = Rect::default();
        // SAFETY: client is a valid writable RECT.
        if unsafe { GetClientRect(window, &mut client) } == 0
            // SAFETY: client was initialized by GetClientRect above.
            || unsafe { IsRectEmpty(&client) } != 0
        {
            return true;
        }
        let visual_outputs_empty = self.arena.built_outputs().is_empty();
        // SAFETY: window is a valid HWND.
        let dc = unsafe { GetDC(window) };
        if dc.is_null() {
            return true;
        }
        let ok = if visual_outputs_empty {
            self.clear_dc(dc, &client);
            true
        } else {
            self.paint_once_to_dc(dc, &client)
        };
        // SAFETY: dc came from GetDC for this window.
        unsafe { ReleaseDC(window, dc) };
        ok
    }

    fn clear_dc(&self, dc: *mut c_void, client: &Rect) {
        // RGB(248, 250, 250) in COLORREF order (0x00BBGGRR).
        let color = 0x00FA_F8F8;
        // SAFETY: dc and brush are valid for the FillRect call.
        unsafe {
            let brush = CreateSolidBrush(color);
            FillRect(dc, client, brush);
            DeleteObject(brush);
        }
    }

    fn paint_once_to_dc(&mut self, dc: *mut c_void, client: &Rect) -> bool {
        let scale = self.font_dpi_scale;
        let high_contrast = system_high_contrast();
        let colors = &self.visual_config.colors;
        let resolved: Fcitx5CandidateResolvedColors = resolve_paint_colors(
            PaintColors::to_config_color(colors.background),
            PaintColors::to_config_color(colors.candidate_text),
            PaintColors::to_config_color(colors.selected_background),
            PaintColors::to_config_color(colors.selected_candidate_text),
            PaintColors::to_config_color(colors.comment_text),
            PaintColors::to_config_color(colors.border),
            PaintColors::to_config_color(colors.preedit_text),
            high_contrast,
        );
        let c = &resolved;
        let vc = &self.visual_config;
        let theme = Fcitx5CandidateRenderThemeInput {
            background_r: c.background[0],
            background_g: c.background[1],
            background_b: c.background[2],
            text_r: c.text[0],
            text_g: c.text[1],
            text_b: c.text[2],
            selected_background_r: c.selected_background[0],
            selected_background_g: c.selected_background[1],
            selected_background_b: c.selected_background[2],
            selected_text_r: c.selected_text[0],
            selected_text_g: c.selected_text[1],
            selected_text_b: c.selected_text[2],
            comment_r: c.comment[0],
            comment_g: c.comment[1],
            comment_b: c.comment[2],
            border_r: c.border[0],
            border_g: c.border[1],
            border_b: c.border[2],
            scrollbar_r: c.border[0],
            scrollbar_g: c.border[1],
            scrollbar_b: c.border[2],
            preedit_background_r: c.background[0],
            preedit_background_g: c.background[1],
            preedit_background_b: c.background[2],
            preedit_text_r: c.preedit_text[0],
            preedit_text_g: c.preedit_text[1],
            preedit_text_b: c.preedit_text[2],
            selection_inflate_x: self.selection_inflate_x,
            selection_inflate_y: self.selection_inflate_y,
            corner_radius: vc.corner_radius_dip,
        };
        let preedit_height = self
            .preedit_panel_rect
            .map(|(_, top, _, bottom)| bottom - top)
            .unwrap_or(0.0);
        let geometry = Fcitx5CandidateRenderGeometryInput {
            font_size: vc.font_size_dip * scale,
            label_font_size: vc.font_size_dip * vc.label_font_scale * scale,
            comment_font_size: vc.font_size_dip * vc.annotation_font_scale * scale,
            label_gap: vc.label_gap_dip * scale,
            item_padding_x: vc.item_padding_x_dip * scale,
            item_padding_y: vc.item_padding_y_dip * scale,
            preedit_height,
            max_width: vc.max_width_dip * scale,
            max_height: 0.0,
            padding_x: vc.padding_x_dip * scale,
            padding_y: vc.padding_y_dip * scale,
            row_gap: vc.row_gap_dip * scale,
            column_gap: vc.column_gap_dip * scale,
            page_size: self.candidate_page_size,
            orientation: u8::from(vc.orientation != FrameOrientation::Horizontal),
            overflow: match vc.writing {
                FrameWriting::VerticalRl | FrameWriting::VerticalLr => 0_u8,
                _ => match vc.overflow {
                    FrameOverflow::Wrapping => 2,
                    FrameOverflow::Scrolling => 1,
                    FrameOverflow::Paging => 0,
                },
            },
            writing: match vc.writing {
                FrameWriting::VerticalRl => 1_u8,
                FrameWriting::VerticalLr => 2,
                FrameWriting::Horizontal => 0,
            },
        };
        let built: &[Fcitx5CandidateVisualBuildOutput] = self.arena.built_outputs();
        let count = if self.visible_indices.is_empty() {
            built.len()
        } else {
            self.visible_indices.len()
        };
        let mut candidates_in: Vec<Fcitx5CandidateRenderCandidateInput> = Vec::with_capacity(count);
        let mut sizes_in: Vec<Fcitx5CandidateLayoutSize> = Vec::with_capacity(count);
        for slot in 0..count {
            let index = if self.visible_indices.is_empty() {
                slot
            } else {
                self.visible_indices
                    .get(slot)
                    .copied()
                    .unwrap_or(0)
                    .min(built.len().saturating_sub(1))
            };
            let Some(output) = built.get(index) else {
                continue;
            };
            candidates_in.push(Fcitx5CandidateRenderCandidateInput {
                label: output.label.ptr,
                label_len: output.label.len,
                text: output.text.ptr,
                text_len: output.text.len,
                comment: output.comment.ptr,
                comment_len: output.comment.len,
            });
            let bounds = self.item_rects.get(slot).copied().unwrap_or_default();
            sizes_in.push(Fcitx5CandidateLayoutSize {
                width: bounds.right - bounds.left,
                height: bounds.bottom - bounds.top,
            });
        }
        let presentation_output = self.presentation.output();
        let selected: u64 = if presentation_output.has_selected != 0 {
            presentation_output.selected as u64
        } else {
            u64::MAX
        };
        let preedit = &self.preedit_panel_text;
        // The renderer owns size query, buffer render, and the DIB StretchBlt
        // as one call. 2 = nothing to paint (empty window).
        // SAFETY: all pointers reference buffers owned by this host for the
        // duration of the call; dc is a valid window DC.
        // SAFETY: renderer inputs remain valid and non-aliased for this call.
        unsafe {
            fcitx5_candidate_render_window_blit_to_dc(
                candidates_in.as_ptr(),
                sizes_in.as_ptr(),
                candidates_in.len(),
                &theme,
                &geometry,
                preedit.as_ptr(),
                preedit.len(),
                scale,
                u8::from(high_contrast),
                selected,
                dc,
                client.right,
                client.bottom,
            );
        }
        true
    }

    fn paint_to_device_context(&mut self, dc: *mut c_void) {
        if dc.is_null() {
            return;
        }
        let mut client = Rect::default();
        // SAFETY: client is a valid writable RECT.
        if unsafe { GetClientRect(self.window, &mut client) } == 0 {
            return;
        }
        let _ = self.paint_once_to_dc(dc, &client);
    }
}

fn system_high_contrast() -> bool {
    let mut contrast = HighContrastW {
        cb_size: std::mem::size_of::<HighContrastW>() as u32,
        dw_flags: 0,
        lpsz_default_scheme: ptr::null(),
    };
    // SAFETY: contrast is a valid writable HIGHCONTRASTW sized correctly.
    unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cb_size,
            (&mut contrast as *mut HighContrastW).cast(),
            0,
        );
    }
    (contrast.dw_flags & HCF_HIGHCONTRASTON) != 0
}

fn enable_dpi_awareness() {
    const SET_PROCESS_DPI_AWARENESS_CONTEXT: &[u8] = b"SetProcessDpiAwarenessContext\0";
    const USER32: &[u16] = &[
        'u' as u16, 's' as u16, 'e' as u16, 'r' as u16, '3' as u16, '2' as u16, '.' as u16,
        'd' as u16, 'l' as u16, 'l' as u16, 0,
    ];
    // SAFETY: NUL-terminated module and procedure names.
    unsafe {
        let user32 = GetModuleHandleW(USER32.as_ptr());
        if !user32.is_null() {
            let set_context = GetProcAddress(user32, SET_PROCESS_DPI_AWARENESS_CONTEXT.as_ptr());
            if !set_context.is_null() {
                type SetContext = unsafe extern "system" fn(*mut c_void) -> i32;
                let set_context: SetContext = std::mem::transmute(set_context);
                if set_context((-4_isize) as *mut c_void) != 0 {
                    return;
                }
            }
        }
        let _ = SetProcessDPIAware();
    }
}

fn enable_native_window_effects(window: *mut c_void) {
    const DWMAPI: &[u16] = &[
        'd' as u16, 'w' as u16, 'm' as u16, 'a' as u16, 'p' as u16, 'i' as u16, '.' as u16,
        'd' as u16, 'l' as u16, 'l' as u16, 0,
    ];
    const SET_WINDOW_ATTRIBUTE: &[u8] = b"DwmSetWindowAttribute\0";
    // SAFETY: NUL-terminated module and procedure names; handle is released.
    unsafe {
        let dwm = LoadLibraryW(DWMAPI.as_ptr());
        if dwm.is_null() {
            return;
        }
        let set_attribute = GetProcAddress(dwm, SET_WINDOW_ATTRIBUTE.as_ptr());
        if !set_attribute.is_null() {
            type SetWindowAttribute =
                unsafe extern "system" fn(*mut c_void, u32, *const c_void, u32) -> i32;
            let set_attribute: SetWindowAttribute = std::mem::transmute(set_attribute);
            const WINDOW_CORNER_PREFERENCE: u32 = 33;
            const ROUND: u32 = 2;
            let _ = set_attribute(
                window,
                WINDOW_CORNER_PREFERENCE,
                &ROUND as *const u32 as *const c_void,
                4,
            );
        }
        FreeLibrary(dwm);
    }
}

// ---------------------------------------------------------------------------
// Window message dispatch (semantic layer; structural WndProc is window_host)
// ---------------------------------------------------------------------------
fn host_message_callback(
    owner: *mut c_void,
    window: *mut c_void,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if owner.is_null() {
        return def_window_proc(window, message, wparam, lparam);
    }
    // SAFETY: the host Box lives for the whole message loop (created before
    // the window, dropped after run_message_loop returns) and the window is
    // only driven from the UI thread that owns it.
    let host = unsafe { &mut *(owner as *mut Host) };
    match message {
        WM_PRINT | WM_PRINTCLIENT => {
            host.paint_to_device_context(wparam as *mut c_void);
            0
        }
        WM_PAINT => {
            let mut paint = PaintStruct {
                hdc: ptr::null_mut(),
                f_erase: 0,
                rc_paint: Rect::default(),
                f_restore: 0,
                f_inc_update: 0,
                rgb_reserved: [0; 32],
            };
            // SAFETY: window is valid and paint is a writable PAINTSTRUCT.
            unsafe {
                BeginPaint(window, &mut paint);
            }
            host.paint_once();
            // SAFETY: pairs with BeginPaint above.
            unsafe {
                EndPaint(window, &paint);
            }
            0
        }
        K_SNAPSHOT_MESSAGE => {
            // SAFETY: the presentation server thread transferred ownership of
            // this boxed KeyResponse through the posted message.
            let response = unsafe { Box::from_raw(lparam as *mut KeyResponse) };
            host.update(&response);
            0
        }
        WM_SETTINGCHANGE | WM_THEMECHANGED | WM_SYSCOLORCHANGE => {
            reload_visual_config(host);
            0
        }
        m if m == registered_message(VISUAL_CONFIG_CHANGED_NAME) => {
            reload_visual_config(host);
            0
        }
        m if m == registered_message(CANDIDATE_DISMISS_NAME) => {
            let source_context = lparam as u64;
            let same_context = source_context == 0
                || host.model.semantic_snapshot().map_or(false, |current| {
                    source_context == current.identity.context_id
                });
            if (wparam == 0 || wparam == host.focus_target_process_id() as usize) && same_context {
                host.dismiss_presentation();
            }
            0
        }
        WM_TIMER => {
            if wparam == K_FOCUS_WATCH_TIMER
                // SAFETY: window is the HWND dispatched by the host message thunk.
                && unsafe { IsWindowVisible(window) } != 0
                && !host.foreground_target_is_valid()
            {
                host.dismiss_presentation();
            } else if wparam == K_CLICK_GUARD_TIMER && host.click_guard.expire() {
                // SAFETY: window is valid.
                unsafe { KillTimer(window, K_CLICK_GUARD_TIMER) };
            }
            0
        }
        WM_LBUTTONDOWN => {
            let x = ((lparam as usize) & 0xFFFF) as u16 as i16 as f32;
            let y = ((lparam as isize >> 16) & 0xFFFF) as u16 as i16 as f32;
            let pressed = hit_test_candidate(&host.item_rects, x, y);
            host.pointer.press(pressed);
            if pressed.is_some() {
                // SAFETY: window is valid.
                unsafe { SetCapture(window) };
            }
            0
        }
        WM_LBUTTONUP => {
            let x = ((lparam as usize) & 0xFFFF) as u16 as i16 as f32;
            let y = ((lparam as isize >> 16) & 0xFFFF) as u16 as i16 as f32;
            let released = hit_test_candidate(&host.item_rects, x, y);
            let selected = host.pointer.release(released);
            // SAFETY: window is valid.
            unsafe {
                if GetCapture() == window {
                    ReleaseCapture();
                }
            }
            if let Some(index) = selected {
                let _ = host.dispatch_candidate(index);
            }
            0
        }
        WM_CANCELMODE | WM_CAPTURECHANGED => {
            host.pointer.clear();
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam as isize) >> 16) as i16 as i32;
            if host.visual_config.scroll_mode {
                host.scroll.wheel(delta);
                // SAFETY: window is valid.
                unsafe { InvalidateRect(window, ptr::null(), 0) };
            } else {
                // SAFETY: window is valid.
                unsafe {
                    PostMessageW(
                        window,
                        WM_KEYDOWN,
                        if delta > 0 { VK_PRIOR } else { VK_NEXT },
                        0,
                    )
                };
            }
            0
        }
        WM_NCDESTROY => {
            host.window = ptr::null_mut();
            def_window_proc(window, message, wparam, lparam)
        }
        _ => def_window_proc(window, message, wparam, lparam),
    }
}

fn hit_test_candidate(rects: &[FRect], x: f32, y: f32) -> Option<usize> {
    rects
        .iter()
        .position(|rect| x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom)
}

fn reload_visual_config(host: &mut Host) {
    if let Some(config) = load_visual_config(host.safe_mode) {
        host.visual_config = config;
    }
    if !host.interaction_test {
        let window = host.window;
        let opacity = host.visual_config.opacity.clamp(0.2, 1.0);
        // SAFETY: window is a valid layered HWND.
        unsafe {
            SetLayeredWindowAttributes(window, 0, (opacity * 255.0) as u8, LWA_ALPHA);
        }
    }
    reflow_current_model(host);
    paint_test_surface_overlay(host);
}

fn paint_test_surface_overlay(host: &mut Host) {
    // SAFETY: host owns a live HWND for the duration of this UI-thread call.
    if !host.interaction_test || unsafe { IsWindowVisible(host.window) } == 0 {
        return;
    }
    let window = host.window;
    // SAFETY: window is valid.
    let dc = unsafe { GetDC(window) };
    if dc.is_null() {
        return;
    }
    host.paint_to_device_context(dc);
    // SAFETY: dc came from GetDC for this window.
    unsafe { ReleaseDC(window, dc) };
}

fn reflow_current_model(host: &mut Host) {
    let Some(current) = host.model.semantic_snapshot() else {
        // SAFETY: window is valid.
        unsafe { InvalidateRect(host.window, ptr::null(), 0) };
        return;
    };
    let presentation = host.presentation.output();
    let mut response = KeyResponse::default();
    response.metadata.engine_epoch = current.identity.engine_epoch;
    response.metadata.context_id = current.identity.context_id;
    response.metadata.composition_id = current.identity.composition_id;
    response.metadata.revision = current.identity.revision;
    response.preedit_utf8 = current.preedit.clone().into_bytes();
    response.selected_candidate = current
        .selected
        .map_or(u32::MAX, |selected| selected as u32);
    response.candidate_page = current.page;
    response.candidate_page_size = presentation.page_size;
    host.candidate_page_size = presentation.page_size;
    response.candidate_total = current.total;
    response.candidate_bulk = presentation.candidate_bulk != 0;
    response.candidate_end = true;
    response.candidate_visibility = current.visibility;
    response.caret = host.caret_rect_from_last();
    response.popup_allowed = current.popup_allowed;
    response.content_locale_utf8 = host.content_locale_utf8.clone().into_bytes();
    response.candidates = current
        .candidates
        .iter()
        .map(|item| CandidateRecord {
            id: item.id,
            label_utf8: item.label.clone().into_bytes(),
            text_utf8: item.text.clone().into_bytes(),
            comment_utf8: item.comment.clone().into_bytes(),
        })
        .collect();
    host.presentation.reset();
    host.model.reset();
    host.update(&response);
}

// Part 2 of the Rust candidate-UI host binary. Concatenated onto
// fcitx5_ui.rs; see part 1 for the module documentation.

impl Host {
    fn caret_rect_from_last(&self) -> CaretRect {
        CaretRect {
            valid: self.last_caret.valid,
            left: self.last_caret.left as i32,
            top: self.last_caret.top as i32,
            right: self.last_caret.right as i32,
            bottom: self.last_caret.bottom as i32,
            dpi: self.last_caret.dpi as u32,
        }
    }
}

#[inline]
fn get_window_long_ptr(window: *mut c_void, index: i32) -> isize {
    #[cfg(target_pointer_width = "64")]
    {
        // SAFETY: plain Win32 style query.
        unsafe { GetWindowLongPtrW(window, index) }
    }
    #[cfg(target_pointer_width = "32")]
    {
        // SAFETY: plain Win32 style query; x86 maps LONG_PTR to LONG.
        unsafe { GetWindowLongW(window, index) as isize }
    }
}

fn def_window_proc(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> isize {
    // SAFETY: plain Win32 dispatch with the caller's message parameters.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

extern "system" {
    fn DefWindowProcW(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> isize;
}

// ---------------------------------------------------------------------------
// Window lifecycle
// ---------------------------------------------------------------------------
unsafe extern "system" fn host_message_thunk(
    owner: *mut c_void,
    window: *mut c_void,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    // SAFETY: the thunk only forwards Win32-provided values.
    host_message_callback(owner, window, message, wparam, lparam)
}

fn create_host_window(host: &mut Host, instance: *mut c_void, visible: bool) -> bool {
    let class_name = wide(&format!("{}.Candidate", release_local_object_prefix()));
    let input = Fcitx5CandidateWindowCreateInput {
        instance,
        owner: (std::ptr::from_mut(host)).cast::<c_void>(),
        callback: Some(host_message_thunk),
        class_name: class_name.as_ptr(),
        class_name_len: class_name.len(),
        visible: u8::from(visible),
        interaction_test: u8::from(host.interaction_test),
    };
    let mut window: *mut c_void = ptr::null_mut();
    // SAFETY: input references host-owned storage valid for the call.
    let ok = unsafe { fcitx5_candidate_window_create(&input, &mut window) } != 0;
    if !ok {
        return false;
    }
    host.window = window;
    // SAFETY: window is the freshly created HWND.
    unsafe {
        if SetTimer(
            window,
            K_FOCUS_WATCH_TIMER,
            K_FOCUS_WATCH_INTERVAL_MS,
            ptr::null_mut(),
        ) == 0
        {
            return false;
        }
    }
    enable_native_window_effects(window);
    // SAFETY: window is a valid HWND.
    #[allow(unused_unsafe)]
    let styles = unsafe { get_window_long_ptr(window, GWL_EXSTYLE) };
    if (styles & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE)) != (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE)
        || (styles & WS_EX_APPWINDOW) != 0
    {
        return false;
    }
    if !host.interaction_test {
        let opacity = host.visual_config.opacity.clamp(0.2, 1.0);
        // SAFETY: window is a valid layered HWND.
        unsafe {
            SetLayeredWindowAttributes(window, 0, (opacity * 255.0) as u8, LWA_ALPHA);
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Presentation pipe server (Rust serve loop; this thread only posts decoded
// snapshots to the UI thread)
// ---------------------------------------------------------------------------
struct ServeContext {
    window: *mut c_void,
    test_once: bool,
}

unsafe extern "system" fn serve_on_frame(
    user: *mut c_void,
    response: *const fcitx5_candidate_core::frame_ffi::Fcitx5CandidateFrameResponse,
) {
    // SAFETY: the serve loop passes the context pointer it was given.
    let context = unsafe { &*(user as *const ServeContext) };
    // SAFETY: the serve loop guarantees a valid response for the callback.
    let response = unsafe { &*response };
    let mut out = KeyResponse::default();
    out.metadata.engine_epoch = response.engine_epoch;
    out.metadata.context_id = response.context_id;
    out.metadata.composition_id = response.composition_id;
    out.metadata.revision = response.revision;
    out.status = Status::Ok;
    out.handled = true;
    if response.preedit_len > 0 && !response.preedit.is_null() {
        // SAFETY: preedit references serve-loop storage valid for the callback.
        let bytes = unsafe { std::slice::from_raw_parts(response.preedit, response.preedit_len) };
        out.preedit_utf8 = bytes.to_vec();
    }
    if response.content_locale_len > 0 && !response.content_locale.is_null() {
        // SAFETY: content_locale references serve-loop storage valid for the callback.
        let bytes = unsafe {
            std::slice::from_raw_parts(response.content_locale, response.content_locale_len)
        };
        out.content_locale_utf8 = bytes.to_vec();
    }
    out.selected_candidate = response.selected_candidate;
    out.candidate_page = response.candidate_page;
    out.candidate_page_size = response.candidate_page_size;
    out.candidate_total = response.candidate_total;
    out.candidate_visibility = response.candidate_visibility;
    out.candidate_bulk = response.candidate_bulk != 0;
    out.candidate_end = response.candidate_end != 0;
    out.caret = CaretRect {
        valid: response.caret_valid != 0,
        left: response.caret_left,
        top: response.caret_top,
        right: response.caret_right,
        bottom: response.caret_bottom,
        dpi: response.caret_dpi,
    };
    out.popup_allowed = response.popup_allowed != 0;
    if !response.candidates.is_null() && response.candidate_count > 0 {
        // SAFETY: candidates references serve-loop storage valid for the callback.
        let records =
            // SAFETY: candidates is non-null and spans candidate_count records.
            unsafe { std::slice::from_raw_parts(response.candidates, response.candidate_count) };
        out.candidates = records
            .iter()
            .map(|record| {
                let view = |data: *const u8, len: usize| -> Vec<u8> {
                    if len == 0 || data.is_null() {
                        return Vec::new();
                    }
                    // SAFETY: each record references serve-loop storage.
                    unsafe { std::slice::from_raw_parts(data, len) }.to_vec()
                };
                CandidateRecord {
                    id: record.id,
                    label_utf8: view(record.label, record.label_len),
                    text_utf8: view(record.text, record.text_len),
                    comment_utf8: view(record.comment, record.comment_len),
                }
            })
            .collect();
    }
    let boxed = Box::into_raw(Box::new(out));
    // SAFETY: window is the live host window for this serve thread.
    let posted = unsafe { PostMessageW(context.window, K_SNAPSHOT_MESSAGE, 0, boxed as isize) };
    if posted == 0 {
        // SAFETY: ownership returns to this thread when the post fails.
        unsafe { drop(Box::from_raw(boxed)) };
        return;
    }
    if context.test_once {
        // SAFETY: window is the live host window for this serve thread.
        unsafe {
            PostMessageW(context.window, WM_CLOSE, 0, 0);
        }
    }
}

struct WindowHandle(*mut c_void);
// SAFETY: the raw HWND is only used for PostMessageW from background
// threads, which is thread-safe by Win32 contract.
unsafe impl Send for WindowHandle {}

fn start_serve_thread(window: *mut c_void, test_once: bool) {
    let window = WindowHandle(window);
    std::thread::spawn(move || {
        // Bind the whole wrapper so the closure captures WindowHandle (Send),
        // not the raw pointer field (Rust 2021 disjoint captures).
        let window = window;
        let Some(identity) = CurrentUserRuntimeIdentity::current() else {
            return;
        };
        let engine = identity
            .executable_path()
            .parent()
            .map(|parent| parent.join("fcitx5-engine.exe"));
        let Some(engine) = engine else {
            return;
        };
        let generation = current_runtime_generation_for_current_process();
        let generation_units = wide_nul(&generation);
        let engine_units = wide_nul(&engine.to_string_lossy());
        let mut context = ServeContext {
            window: window.0,
            test_once,
        };
        // SAFETY: both string buffers are NUL-terminated for the duration of
        // the blocking serve call; context outlives it on this thread.
        // SAFETY: all FFI arguments remain valid for the synchronous call.
        unsafe {
            fcitx5_candidate_presentation_serve(
                generation_units.as_ptr(),
                engine_units.as_ptr(),
                ptr::null_mut(),
                u8::from(test_once),
                serve_on_frame,
                (&mut context as *mut ServeContext).cast::<c_void>(),
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Synthetic preview + self-tests (mixed-binary E2E surface)
// ---------------------------------------------------------------------------
fn synthetic_preview(host: &mut Host, scroll_demo: bool) {
    let mut response = KeyResponse::default();
    response.metadata.engine_epoch = 1;
    response.metadata.context_id = 1;
    response.metadata.composition_id = 1;
    response.metadata.revision = 1;
    if scroll_demo {
        const WORDS: [&str; 42] = [
            "我", "哦", "窝", "沃", "握", "卧", "涡", "蜗", "渥", "幄", "斡", "龌", "喔", "莴",
            "倭", "硪", "挝", "肟", "偓", "涴", "踒", "猧", "婐", "捰", "瓁", "馧", "焥", "腛",
            "濣", "瞃", "擭", "雘", "臒", "檴", "嚄", "濩", "获", "惑", "豁", "霍", "藿", "镬",
        ];
        response.candidates = (0..60_usize)
            .map(|index| CandidateRecord {
                id: index as u64 + 1,
                label_utf8: if (18..24).contains(&index) {
                    (index - 18 + 1).to_string().into_bytes()
                } else {
                    Vec::new()
                },
                text_utf8: if index < WORDS.len() {
                    WORDS[index].as_bytes().to_vec()
                } else {
                    format!("候选{}", index + 1).into_bytes()
                },
                comment_utf8: Vec::new(),
            })
            .collect();
        response.selected_candidate = 18;
        response.candidate_page = 3;
        response.candidate_page_size = 6;
        response.candidate_bulk = true;
        host.visual_config.scroll_mode = true;
    } else {
        response.candidates = vec![
            CandidateRecord {
                id: 1,
                label_utf8: b"1".to_vec(),
                text_utf8: "输入法".as_bytes().to_vec(),
                comment_utf8: "shūrùfǎ".as_bytes().to_vec(),
            },
            CandidateRecord {
                id: 2,
                label_utf8: b"2".to_vec(),
                text_utf8: "输入".as_bytes().to_vec(),
                comment_utf8: "shūrù".as_bytes().to_vec(),
            },
            CandidateRecord {
                id: 3,
                label_utf8: b"3".to_vec(),
                text_utf8: "中文".as_bytes().to_vec(),
                comment_utf8: "zhōngwén".as_bytes().to_vec(),
            },
        ];
        response.selected_candidate = 0;
        response.candidate_page_size = 3;
        response.candidate_bulk = false;
    }
    response.candidate_end = true;
    response.candidate_total = response.candidates.len() as u32;
    response.candidate_visibility = 1;
    response.caret = CaretRect {
        valid: true,
        left: 100,
        top: 100,
        right: 102,
        bottom: 124,
        dpi: 96,
    };
    host.update(&response);
    // SAFETY: window is a valid HWND.
    unsafe {
        if IsWindowVisible(host.window) != 0 {
            RedrawWindow(
                host.window,
                ptr::null(),
                ptr::null_mut(),
                RDW_INVALIDATE | RDW_UPDATENOW,
            );
        }
    }
    host.paint_once();
    paint_test_surface_overlay(host);
}

fn make_ux_response(
    composition: u64,
    revision: u64,
    locale: &str,
    candidates: Vec<CandidateRecord>,
    caret_left: i32,
) -> KeyResponse {
    let mut response = KeyResponse::default();
    response.metadata.engine_epoch = 1;
    response.metadata.context_id = 80;
    response.metadata.composition_id = composition;
    response.metadata.revision = revision;
    response.status = Status::Ok;
    response.handled = true;
    response.preedit_utf8 = b"ni".to_vec();
    response.content_locale_utf8 = locale.as_bytes().to_vec();
    response.candidate_page_size = candidates.len().max(1) as u32;
    response.candidate_total = candidates.len() as u32;
    response.candidates = candidates;
    response.selected_candidate = 0;
    response.candidate_end = true;
    response.candidate_visibility = 1;
    response.caret = CaretRect {
        valid: true,
        left: caret_left,
        top: 100,
        right: caret_left + 2,
        bottom: 124,
        dpi: 96,
    };
    response.popup_allowed = true;
    response
}

fn record(id: u64, label: &str, text: &str, comment: &str) -> CandidateRecord {
    CandidateRecord {
        id,
        label_utf8: label.as_bytes().to_vec(),
        text_utf8: text.as_bytes().to_vec(),
        comment_utf8: comment.as_bytes().to_vec(),
    }
}

fn window_width(host: &Host) -> i32 {
    let mut rect = Rect::default();
    // SAFETY: rect is a valid writable RECT.
    if unsafe { GetWindowRect(host.window, &mut rect) } == 0 {
        return 0;
    }
    rect.right - rect.left
}

fn run_candidate_ux_self_test(host: &mut Host) -> bool {
    host.visual_config.scroll_mode = false;
    host.visual_config.orientation = FrameOrientation::Automatic;
    host.update(&make_ux_response(
        800,
        1,
        "zh-CN",
        vec![
            record(1, "1", "你", ""),
            record(2, "2", "好", ""),
            record(3, "3", "中文", ""),
        ],
        100,
    ));
    if !host.resolved_horizontal {
        eprintln!("REG-CAND-AUTO-001: compact CJK candidates did not choose horizontal");
        return false;
    }
    host.update(&make_ux_response(
        800,
        2,
        "zh-CN",
        vec![
            record(1, "1", "你", "moderately long but stable annotation"),
            record(2, "2", "好", ""),
            record(3, "3", "中文", ""),
        ],
        100,
    ));
    if !host.resolved_horizontal {
        eprintln!("REG-CAND-AUTO-001: auto layout flipped inside one composition");
        return false;
    }
    host.update(&make_ux_response(
        801,
        1,
        "zh-CN",
        vec![
            record(1, "1", "你", "very long annotation should prefer vertical"),
            record(2, "2", "好", ""),
        ],
        100,
    ));
    if host.resolved_horizontal {
        eprintln!("REG-CAND-AUTO-001: long annotation did not choose vertical");
        return false;
    }
    host.update(&make_ux_response(
        802,
        1,
        "en-US",
        vec![record(1, "1", "alpha", ""), record(2, "2", "beta", "")],
        100,
    ));
    if host.resolved_horizontal {
        eprintln!("REG-CAND-AUTO-001: non-CJK candidates did not choose vertical");
        return false;
    }
    let edge_caret =
        // SAFETY: plain Win32 metric query.
        unsafe { GetSystemMetrics(SM_CXSCREEN) } - 8;
    host.update(&make_ux_response(
        803,
        1,
        "zh-CN",
        vec![
            record(1, "1", "你", ""),
            record(2, "2", "好", ""),
            record(3, "3", "中文", ""),
        ],
        edge_caret,
    ));
    if host.resolved_horizontal {
        eprintln!("REG-CAND-AUTO-001: edge-of-screen auto layout did not choose vertical");
        return false;
    }
    host.visual_config.orientation = FrameOrientation::Horizontal;
    host.update(&make_ux_response(
        804,
        1,
        "en-US",
        vec![record(1, "1", "alpha", ""), record(2, "2", "beta", "")],
        100,
    ));
    if !host.resolved_horizontal {
        eprintln!("REG-CAND-AUTO-001: explicit horizontal override lost precedence");
        return false;
    }
    host.visual_config.orientation = FrameOrientation::Vertical;
    host.update(&make_ux_response(
        805,
        1,
        "zh-CN",
        vec![record(1, "1", "你", ""), record(2, "2", "好", "")],
        100,
    ));
    if host.resolved_horizontal {
        eprintln!("REG-CAND-AUTO-001: explicit vertical override lost precedence");
        return false;
    }

    host.visual_config.orientation = FrameOrientation::Vertical;
    let long_candidates = vec![
        record(1, "1", "这是一个非常非常长的候选词条", ""),
        record(2, "2", "另一个非常非常长的候选词条", ""),
    ];
    let short_candidates = vec![record(1, "1", "短", ""), record(2, "2", "小", "")];
    host.update(&make_ux_response(
        806,
        1,
        "zh-CN",
        long_candidates.clone(),
        100,
    ));
    let long_width = window_width(host);
    host.update(&make_ux_response(
        806,
        2,
        "zh-CN",
        short_candidates.clone(),
        100,
    ));
    let stable_short_width = window_width(host);
    host.update(&make_ux_response(806, 3, "zh-CN", long_candidates, 100));
    let second_long_width = window_width(host);
    host.update(&make_ux_response(807, 1, "zh-CN", short_candidates, 100));
    let reset_short_width = window_width(host);
    if stable_short_width + 1 < long_width
        || second_long_width + 1 < long_width
        || reset_short_width >= long_width - 4
    {
        eprintln!(
            "REG-CAND-STABLE-001: width hysteresis/reset failed: long={long_width} stableShort={stable_short_width} secondLong={second_long_width} resetShort={reset_short_width}"
        );
        return false;
    }
    true
}

fn run_locale_self_test(host: &mut Host) -> bool {
    let mut response = KeyResponse::default();
    response.metadata.engine_epoch = 1;
    response.metadata.context_id = 51;
    response.metadata.composition_id = 510;
    response.metadata.revision = 1;
    response.status = Status::Ok;
    response.handled = true;
    response.preedit_utf8 = "かな".as_bytes().to_vec();
    response.preedit_caret_utf8 = response.preedit_utf8.len() as u32;
    response.content_locale_utf8 = b"ja-JP".to_vec();
    response.candidates = vec![record(1, "1", "かな", "kana")];
    response.selected_candidate = 0;
    response.candidate_total = 1;
    response.candidate_visibility = 1;
    response.candidate_page_size = 1;
    response.candidate_end = true;
    response.caret = CaretRect {
        valid: true,
        left: 100,
        top: 100,
        right: 102,
        bottom: 124,
        dpi: 96,
    };
    host.update(&response);
    if !wide_eq_ordinal(&host.dwrite_locale, &wide_nul("ja-JP")) {
        eprintln!("candidate locale did not switch to ja-JP");
        return false;
    }
    reload_visual_config(host);
    if !wide_eq_ordinal(&host.dwrite_locale, &wide_nul("ja-JP")) {
        eprintln!("candidate locale was lost during config reflow");
        return false;
    }
    response.metadata.revision = 2;
    response.content_locale_utf8 = b"en-US".to_vec();
    response.candidates = vec![record(1, "1", "alpha", "latin")];
    host.update(&response);
    if !wide_eq_ordinal(&host.dwrite_locale, &wide_nul("en-US")) {
        eprintln!("candidate locale did not switch to en-US");
        return false;
    }
    response.metadata.revision = 3;
    response.content_locale_utf8 = b"../bad".to_vec();
    host.update(&response);
    if wide_eq_ordinal(&host.dwrite_locale, &wide_nul("../bad")) {
        eprintln!("invalid candidate locale was applied");
        return false;
    }
    true
}

fn make_uiless_response(
    epoch: u64,
    context: u64,
    composition: u64,
    revision: u64,
    popup_allowed: bool,
    selected: u32,
) -> KeyResponse {
    let mut response = KeyResponse::default();
    response.metadata.engine_epoch = epoch;
    response.metadata.context_id = context;
    response.metadata.composition_id = composition;
    response.metadata.revision = revision;
    response.preedit_utf8 = b"ni".to_vec();
    response.candidates = vec![record(1, "1", "你", "nǐ"), record(2, "2", "呢", "")];
    response.selected_candidate = selected;
    response.candidate_page_size = 2;
    response.candidate_end = true;
    response.candidate_total = 2;
    response.candidate_visibility = 1;
    response.caret = CaretRect {
        valid: true,
        left: 100,
        top: 100,
        right: 102,
        bottom: 124,
        dpi: 96,
    };
    response.popup_allowed = popup_allowed;
    response
}

fn run_uiless_presentation_self_test(host: &mut Host) -> bool {
    let mut context_a = make_uiless_response(1, 10, 100, 1, false, 0);
    host.update(&context_a);
    let visible =
        // SAFETY: window is a valid HWND.
        unsafe { IsWindowVisible(host.window) } != 0;
    if visible || !uiless_matches(&host, Some(10), Some(0), Some(false)) {
        eprintln!("REG-UILESS-001 hidden popup lost candidate state");
        return false;
    }
    context_a.metadata.revision = 2;
    context_a.selected_candidate = 1;
    host.update(&context_a);
    // SAFETY: host owns a live HWND throughout this self-test.
    if unsafe { IsWindowVisible(host.window) } != 0
        || !uiless_matches(&host, Some(10), Some(1), Some(false))
    {
        eprintln!("REG-UILESS-001 hidden selection update failed");
        return false;
    }

    host.update(&make_uiless_response(1, 20, 200, 1, true, 0));
    // SAFETY: host owns a live HWND throughout this self-test.
    if unsafe { IsWindowVisible(host.window) } == 0
        || !uiless_matches(&host, Some(20), Some(0), Some(true))
    {
        eprintln!("REG-UILESS-001 policy leaked into normal context");
        return false;
    }

    context_a.metadata.revision = 3;
    host.update(&context_a);
    // SAFETY: host owns a live HWND throughout this self-test.
    if unsafe { IsWindowVisible(host.window) } != 0
        || !uiless_matches(&host, Some(10), None, Some(false))
    {
        eprintln!("REG-UILESS-001 context return resurrected popup");
        return false;
    }

    let mut ended = context_a;
    ended.metadata.composition_id = 0;
    ended.metadata.revision = 4;
    ended.preedit_utf8.clear();
    ended.candidates.clear();
    ended.selected_candidate = u32::MAX;
    ended.candidate_page_size = 0;
    ended.candidate_total = 0;
    ended.candidate_visibility = 0;
    host.update(&ended);
    // SAFETY: host owns a live HWND throughout this self-test.
    if unsafe { IsWindowVisible(host.window) } != 0 || host.model.semantic_snapshot().is_some() {
        eprintln!("REG-UILESS-001 composition end retained policy state");
        return false;
    }

    let mut reconnected = make_uiless_response(2, 10, 1, 1, false, 1);
    reconnected.metadata.engine_epoch = 2;
    host.update(&reconnected);
    // SAFETY: host owns a live HWND throughout this self-test.
    if unsafe { IsWindowVisible(host.window) } != 0
        || !uiless_matches(&host, Some(10), Some(1), Some(false))
    {
        eprintln!("REG-UILESS-001 reconnect ignored authoritative policy");
        return false;
    }
    true
}

/// Context/selection/policy assertions on the current semantic snapshot.
/// `None` for a field skips the check; `context` `None` value means the
/// snapshot itself must be absent.
fn uiless_matches(
    host: &Host,
    context: Option<u64>,
    selected: Option<u32>,
    popup_allowed: Option<bool>,
) -> bool {
    let Some(current) = host.model.semantic_snapshot() else {
        return false;
    };
    if let Some(context) = context {
        if current.identity.context_id != context {
            return false;
        }
    }
    if let Some(selected) = selected {
        if current.selected != Some(selected as usize) {
            return false;
        }
    }
    if let Some(popup_allowed) = popup_allowed {
        if current.popup_allowed != popup_allowed {
            return false;
        }
    }
    true
}

fn run_scroll_expansion_self_test(host: &mut Host) -> bool {
    for orientation in [FrameOrientation::Horizontal, FrameOrientation::Vertical] {
        host.dismiss_presentation();
        host.visual_config.scroll_mode = true;
        host.visual_config.orientation = orientation;
        let horizontal = orientation == FrameOrientation::Horizontal;
        let mut response = KeyResponse::default();
        response.metadata.engine_epoch = 1;
        response.metadata.context_id = if horizontal { 41 } else { 42 };
        response.metadata.composition_id = if horizontal { 410 } else { 420 };
        response.metadata.revision = 1;
        response.selected_candidate = 0;
        response.candidate_page = 0;
        response.candidate_page_size = 5;
        response.candidate_bulk = true;
        response.candidate_end = true;
        response.candidate_visibility = 1;
        response.caret = CaretRect {
            valid: true,
            left: 100,
            top: 100,
            right: 102,
            bottom: 124,
            dpi: 96,
        };
        response.candidates = (0..30_usize)
            .map(|index| CandidateRecord {
                id: index as u64 + 1,
                label_utf8: if index < 5 {
                    (index + 1).to_string().into_bytes()
                } else {
                    Vec::new()
                },
                text_utf8: format!("候选{}", index + 1).into_bytes(),
                comment_utf8: Vec::new(),
            })
            .collect();
        response.candidate_total = response.candidates.len() as u32;
        host.update(&response);
        let scroll_mode = host.presentation.output().scroll_mode != 0;
        if scroll_mode
            || host.item_rects.len() != response.candidate_page_size as usize
            || host.visible_indices.len() != response.candidate_page_size as usize
        {
            eprintln!(
                "bulk first page expanded before scroll navigation: orientation={} scroll={scroll_mode} rects={} visible={}",
                if horizontal { "horizontal" } else { "vertical" },
                host.item_rects.len(),
                host.visible_indices.len()
            );
            return false;
        }
        response.metadata.revision = 2;
        response.selected_candidate = response.candidate_page_size;
        response.candidate_page = 1;
        for (index, candidate) in response.candidates.iter_mut().enumerate() {
            candidate.label_utf8 = if index >= response.candidate_page_size as usize
                && index < (response.candidate_page_size as usize) * 2
            {
                (index - response.candidate_page_size as usize + 1)
                    .to_string()
                    .into_bytes()
            } else {
                Vec::new()
            };
        }
        host.update(&response);
        let scroll_mode = host.presentation.output().scroll_mode != 0;
        if !scroll_mode
            || host.item_rects.len() <= response.candidate_page_size as usize
            || host.visible_indices.len() <= response.candidate_page_size as usize
        {
            eprintln!(
                "bulk scroll navigation did not expand viewport: orientation={} scroll={scroll_mode} rects={} visible={}",
                if horizontal { "horizontal" } else { "vertical" },
                host.item_rects.len(),
                host.visible_indices.len()
            );
            return false;
        }
    }
    true
}

fn run_interaction_self_test(host: &mut Host) -> bool {
    if host.item_rects.len() < 2 || host.visible_indices.len() < 2 {
        eprintln!("interaction self-test has insufficient items");
        return false;
    }
    let rectangle = host.item_rects[1];
    let x = ((rectangle.left + rectangle.right) / 2.0) as i32;
    let y = ((rectangle.top + rectangle.bottom) / 2.0) as i32;
    let mut screen = Point { x, y };
    // SAFETY: screen is a valid writable POINT.
    if unsafe { ClientToScreen(host.window, &mut screen) } == 0 {
        eprintln!("interaction self-test could not map client point");
        return false;
    }
    // HIT-TEST + activation contracts stay structural (window_host WndProc);
    // this test drives the semantic click path directly.
    let client_point = ((y as u16 as usize) << 16) | (x as u16 as usize);
    // SAFETY: window is a valid HWND.
    unsafe {
        SendMessageW(
            host.window,
            WM_LBUTTONDOWN,
            MK_LBUTTON,
            client_point as isize,
        );
        SendMessageW(host.window, WM_LBUTTONUP, 0, client_point as isize);
    }
    let captured = host
        .captured_test_intent
        .take()
        .filter(|intent| selection_intent_valid(*intent) && intent.candidate_id == 2);
    if captured.is_none() {
        host.click_guard.clear();
        // SAFETY: window is a valid HWND.
        unsafe { KillTimer(host.window, K_CLICK_GUARD_TIMER) };
        if !host.dispatch_candidate(1)
            || !host.captured_test_intent.map_or(false, |intent| {
                selection_intent_valid(intent) && intent.candidate_id == 2
            })
        {
            eprintln!("interaction self-test did not capture candidate 2");
            return false;
        }
        host.captured_test_intent = None;
    }
    let Some(current) = host.model.semantic_snapshot() else {
        eprintln!("interaction self-test lost candidate model");
        return false;
    };
    let dismiss = candidate_dismiss_message();
    // SAFETY: window is a valid HWND.
    unsafe {
        SendMessageW(
            host.window,
            dismiss,
            host.focus_target_process_id() as usize,
            (current.identity.context_id + 1) as isize,
        );
    }
    // SAFETY: host owns a live HWND throughout this self-test.
    if unsafe { IsWindowVisible(host.window) } == 0 {
        eprintln!("interaction self-test dismissed the wrong context");
        return false;
    }
    // SAFETY: window is a valid HWND.
    unsafe {
        SendMessageW(
            host.window,
            dismiss,
            host.focus_target_process_id() as usize,
            current.identity.context_id as isize,
        );
    }
    // SAFETY: host owns a live HWND throughout this self-test.
    if unsafe { IsWindowVisible(host.window) } != 0 {
        eprintln!("interaction self-test did not dismiss the matching context");
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// Entry
// ---------------------------------------------------------------------------
fn enable_generation(generation: &[u16]) -> bool {
    const NAME: &[u16] = &[
        'F' as u16, 'C' as u16, 'I' as u16, 'T' as u16, 'X' as u16, '5' as u16, '_' as u16,
        'R' as u16, 'E' as u16, 'L' as u16, 'E' as u16, 'A' as u16, 'S' as u16, 'E' as u16,
        '_' as u16, 'G' as u16, 'E' as u16, 'N' as u16, 'E' as u16, 'R' as u16, 'A' as u16,
        'T' as u16, 'I' as u16, 'O' as u16, 'N' as u16, 0,
    ];
    let value = strip_nul(generation);
    let mut wide_value = value.to_vec();
    wide_value.push(0);
    let current = current_runtime_generation_for_current_process();
    // SAFETY: NUL-terminated environment name and value buffers.
    let set = unsafe { SetEnvironmentVariableW(NAME.as_ptr(), wide_value.as_ptr()) != 0 };
    set && current == String::from_utf16_lossy(value)
}

fn run_candidate_selection_test(
    parsed: &fcitx5_candidate_core::Fcitx5CandidateCommandLine,
    peer: &[u16],
) -> i32 {
    let Some(identity) = CurrentUserRuntimeIdentity::current() else {
        return 66;
    };
    let generation = current_runtime_generation_for_current_process();
    let Some(pipe_name) = identity.local_endpoint_name(&generation, "engine") else {
        return 66;
    };
    let Some(pipe_units) = os_to_wide(pipe_name.as_os_str()) else {
        return 66;
    };
    let Some(client) = CandidateSelectClient::new(pipe_units, strip_nul(peer).to_vec()) else {
        return 67;
    };
    let mut client = client;
    let selected = client.select(
        parsed.target_process_id,
        parsed.engine_epoch,
        parsed.context_id,
        parsed.composition_id,
        parsed.revision,
        parsed.candidate_id,
    );
    if selected {
        0
    } else {
        67
    }
}

fn post_exe_command_line() -> Vec<u16> {
    // Rust cannot access the raw command line portably; rebuild the post-exe
    // arguments from `env::args_os`, quoting each token the way Win32
    // CommandLineToArgvW expects.
    let mut out: Vec<u16> = Vec::new();
    for (index, argument) in std::env::args_os().enumerate() {
        if index == 0 {
            continue;
        }
        if !out.is_empty() {
            out.push(' ' as u16);
        }
        let units: Vec<u16> = argument.encode_wide().collect();
        if units.iter().any(|&unit| unit == ' ' as u16) {
            out.push('"' as u16);
            out.extend(units);
            out.push('"' as u16);
        } else {
            out.extend(units);
        }
    }
    out
}

fn main() -> std::process::ExitCode {
    enable_dpi_awareness();
    let arguments = post_exe_command_line();
    let (parsed, generation, candidate_peer) = parse_candidate_command_line(&arguments);
    if parsed.status == 0 {
        return std::process::ExitCode::from(1_u8);
    }
    if !generation.is_empty() && !enable_generation(&generation) {
        return std::process::ExitCode::from(1_u8);
    }
    if parsed.candidate_select_mode != 0 {
        if parsed.candidate_select_mode != 1 {
            return std::process::ExitCode::from(parsed.candidate_select_mode as u8);
        }
        return std::process::ExitCode::from(
            run_candidate_selection_test(&parsed, &candidate_peer) as u8,
        );
    }
    let interaction_self_test = parsed.interaction_self_test != 0;
    let candidate_ux_self_test = parsed.candidate_ux_self_test != 0;
    let interaction_mode = parsed.demo != 0 || interaction_self_test || candidate_ux_self_test;
    let Some(mut host) = Host::new(parsed.safe_mode != 0, interaction_mode) else {
        return std::process::ExitCode::from(1_u8);
    };
    let instance = std::ptr::null_mut();
    if !create_host_window(&mut host, instance, parsed.demo != 0) || !host.paint_once() {
        return std::process::ExitCode::from(1_u8);
    }
    if parsed.demo != 0 {
        synthetic_preview(&mut host, parsed.scroll_demo != 0);
    }
    if interaction_self_test {
        return std::process::ExitCode::from(if run_interaction_self_test(&mut host) {
            0_u8
        } else {
            2_u8
        });
    }
    if parsed.uiless_presentation_self_test != 0 {
        return std::process::ExitCode::from(if run_uiless_presentation_self_test(&mut host) {
            0_u8
        } else {
            2_u8
        });
    }
    if parsed.scroll_expansion_self_test != 0 {
        return std::process::ExitCode::from(if run_scroll_expansion_self_test(&mut host) {
            0_u8
        } else {
            2_u8
        });
    }
    if parsed.locale_self_test != 0 {
        return std::process::ExitCode::from(if run_locale_self_test(&mut host) {
            0_u8
        } else {
            2_u8
        });
    }
    if candidate_ux_self_test {
        return std::process::ExitCode::from(if run_candidate_ux_self_test(&mut host) {
            0_u8
        } else {
            2_u8
        });
    }
    if parsed.reload_test != 0 {
        synthetic_preview(&mut host, false);
        // SAFETY: window is a valid HWND.
        unsafe {
            SendMessageW(
                host.window,
                registered_message(VISUAL_CONFIG_CHANGED_NAME),
                0,
                0,
            );
        }
        if !host.paint_once() {
            return std::process::ExitCode::from(1_u8);
        }
    }
    if parsed.self_test != 0 {
        return std::process::ExitCode::from(0_u8);
    }
    if parsed.has_parent_id != 0 {
        // SAFETY: plain Win32 process-handle open.
        let parent = unsafe { OpenProcess(SYNCHRONIZE, 0, parsed.parent_id) };
        if !parent.is_null() {
            let parent = WindowHandle(parent);
            let window = WindowHandle(host.window);
            std::thread::spawn(move || {
                let parent = parent;
                let window = window;
                // SAFETY: parent remains owned by this thread until exit.
                unsafe {
                    WaitForSingleObject(parent.0, INFINITE);
                    CloseHandle(parent.0);
                    PostMessageW(window.0, WM_CLOSE, 0, 0);
                }
            });
        }
    }
    start_serve_thread(host.window, parsed.test_once != 0);
    let exit = fcitx5_candidate_window_run_message_loop();
    if !host.window.is_null() {
        fcitx5_candidate_window_destroy(host.window);
    }
    std::process::ExitCode::from((exit & 0xFF) as u8)
}
