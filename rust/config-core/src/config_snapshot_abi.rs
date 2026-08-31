#![deny(unsafe_op_in_unsafe_fn)]

//! Read-only C ABI for the resolved Config Core Current snapshot.

use std::ffi::c_void;
use std::panic;
use std::path::PathBuf;

use crate::{ConfigCore, ConfigSnapshot, FileStore, FontFamilies};

/// Borrowed UTF-8 bytes owned by a live Config snapshot handle.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5ConfigUtf8 {
    /// UTF-8 byte pointer, or null for an empty value.
    pub ptr: *const u8,
    /// Byte length; the value is not NUL-terminated.
    pub len: usize,
}

/// Borrowed UTF-16 input passed to the Config snapshot loader.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5ConfigUtf16 {
    /// UTF-16 code-unit pointer, or null only when `len` is zero.
    pub ptr: *const u16,
    /// UTF-16 code-unit length; the value is not NUL-terminated.
    pub len: usize,
}

/// A flat, borrowed view of every Config field consumed by native adapters.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5ConfigSnapshot {
    /// `0` Current, `1` last-known-good, or `2` compiled safe defaults.
    pub recovery_source: u32,
    /// Exact Config Core schema version.
    pub format_version: u32,
    /// UI language.
    pub ui_language: Fcitx5ConfigUtf8,
    /// Appearance mode.
    pub appearance_mode: Fcitx5ConfigUtf8,
    /// Appearance theme ID.
    pub appearance_theme: Fcitx5ConfigUtf8,
    /// Candidate orientation.
    pub candidate_orientation: Fcitx5ConfigUtf8,
    /// Candidate page size.
    pub candidate_page_size: u8,
    /// Candidate scroll mode as `0` or `1`.
    pub candidate_scroll_mode: u8,
    /// Candidate maximum width in DIP.
    pub candidate_max_width_dip: f32,
    /// Candidate scroll-cell width in DIP.
    pub candidate_scroll_cell_width_dip: f32,
    /// Candidate opacity.
    pub candidate_opacity: f32,
    /// Candidate preedit presentation mode.
    pub candidate_preedit_mode: Fcitx5ConfigUtf8,
    /// Candidate panel horizontal padding in DIP.
    pub candidate_padding_x_dip: f32,
    /// Candidate panel vertical padding in DIP.
    pub candidate_padding_y_dip: f32,
    /// Candidate-item horizontal padding in DIP.
    pub candidate_item_padding_x_dip: f32,
    /// Candidate-item vertical padding in DIP.
    pub candidate_item_padding_y_dip: f32,
    /// Candidate row gap in DIP.
    pub candidate_row_gap_dip: f32,
    /// Candidate column gap in DIP.
    pub candidate_column_gap_dip: f32,
    /// Candidate border width in DIP.
    pub candidate_border_width_dip: f32,
    /// Candidate corner radius in DIP.
    pub candidate_corner_radius_dip: f32,
    /// Candidate shadow as `0` or `1`.
    pub candidate_shadow: u8,
    /// Candidate-label visibility as `0` or `1`.
    pub candidate_label_visible: u8,
    /// Candidate-label style.
    pub candidate_label_style: Fcitx5ConfigUtf8,
    /// Candidate-label font scale.
    pub candidate_label_font_scale: f32,
    /// Candidate-label gap in DIP.
    pub candidate_label_gap_dip: f32,
    /// Number of candidate-label sequence entries.
    pub candidate_label_count: usize,
    /// Number of candidate color entries.
    pub candidate_color_count: usize,
    /// Number of UI font fallbacks.
    pub ui_font_family_count: usize,
    /// Number of candidate font fallbacks.
    pub candidate_font_family_count: usize,
    /// Candidate font size in DIP.
    pub candidate_font_size_dip: f32,
    /// Candidate font weight.
    pub candidate_font_weight: u16,
    /// Number of annotation font fallbacks.
    pub annotation_font_family_count: usize,
    /// Annotation font scale.
    pub annotation_font_scale: f32,
    /// Number of monospace font fallbacks.
    pub monospace_font_family_count: usize,
    /// Number of enabled input methods.
    pub input_method_count: usize,
    /// Default input-method ID.
    pub default_input_method: Fcitx5ConfigUtf8,
    /// Input-method toggle hotkey.
    pub hotkey_toggle_input_method: Fcitx5ConfigUtf8,
    /// Next-input-method hotkey.
    pub hotkey_next_input_method: Fcitx5ConfigUtf8,
}

/// UI font-family selector for `fcitx5_config_snapshot_font_family_at`.
pub const FCITX5_CONFIG_FONT_UI: u32 = 0;
/// Candidate font-family selector for `fcitx5_config_snapshot_font_family_at`.
pub const FCITX5_CONFIG_FONT_CANDIDATE: u32 = 1;
/// Annotation font-family selector for `fcitx5_config_snapshot_font_family_at`.
pub const FCITX5_CONFIG_FONT_ANNOTATION: u32 = 2;
/// Monospace font-family selector for `fcitx5_config_snapshot_font_family_at`.
pub const FCITX5_CONFIG_FONT_MONOSPACE: u32 = 3;

struct SnapshotHandle {
    snapshot: ConfigSnapshot,
    recovery_source: u32,
}

fn utf8(value: &str) -> Fcitx5ConfigUtf8 {
    Fcitx5ConfigUtf8 {
        ptr: value.as_ptr(),
        len: value.len(),
    }
}

fn snapshot_view(handle: &SnapshotHandle) -> Fcitx5ConfigSnapshot {
    let snapshot = &handle.snapshot;
    let candidate = snapshot.candidate();
    let geometry = candidate.geometry();
    let label = candidate.label();
    let fonts = snapshot.fonts();
    Fcitx5ConfigSnapshot {
        recovery_source: handle.recovery_source,
        format_version: snapshot.format_version(),
        ui_language: utf8(snapshot.ui().language()),
        appearance_mode: utf8(snapshot.appearance().mode()),
        appearance_theme: utf8(snapshot.appearance().theme()),
        candidate_orientation: utf8(candidate.orientation()),
        candidate_page_size: candidate.page_size(),
        candidate_scroll_mode: u8::from(candidate.scroll_mode()),
        candidate_max_width_dip: candidate.max_width_dip(),
        candidate_scroll_cell_width_dip: candidate.scroll_cell_width_dip(),
        candidate_opacity: candidate.opacity(),
        candidate_preedit_mode: utf8(candidate.preedit_mode()),
        candidate_padding_x_dip: geometry.padding_x_dip(),
        candidate_padding_y_dip: geometry.padding_y_dip(),
        candidate_item_padding_x_dip: geometry.item_padding_x_dip(),
        candidate_item_padding_y_dip: geometry.item_padding_y_dip(),
        candidate_row_gap_dip: geometry.row_gap_dip(),
        candidate_column_gap_dip: geometry.column_gap_dip(),
        candidate_border_width_dip: geometry.border_width_dip(),
        candidate_corner_radius_dip: geometry.corner_radius_dip(),
        candidate_shadow: u8::from(geometry.shadow()),
        candidate_label_visible: u8::from(label.visible()),
        candidate_label_style: utf8(label.style()),
        candidate_label_font_scale: label.font_scale(),
        candidate_label_gap_dip: label.gap_dip(),
        candidate_label_count: label.sequence().len(),
        candidate_color_count: candidate.colors().len(),
        ui_font_family_count: fonts.ui().families().len(),
        candidate_font_family_count: fonts.candidate().families().len(),
        candidate_font_size_dip: fonts.candidate().size_dip(),
        candidate_font_weight: fonts.candidate().weight(),
        annotation_font_family_count: fonts.annotation().families().len(),
        annotation_font_scale: fonts.annotation().scale(),
        monospace_font_family_count: fonts.monospace().families().len(),
        input_method_count: snapshot.input_methods().enabled().len(),
        default_input_method: utf8(snapshot.input_methods().default_id()),
        hotkey_toggle_input_method: utf8(snapshot.hotkeys().toggle_input_method()),
        hotkey_next_input_method: utf8(snapshot.hotkeys().next_input_method()),
    }
}

fn utf16_path(value: Fcitx5ConfigUtf16) -> Option<PathBuf> {
    if value.len == 0 {
        return None;
    }
    if value.ptr.is_null() {
        return None;
    }
    // SAFETY: pointer/null validation above; caller promises `len` readable UTF-16 code units.
    let value = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    String::from_utf16(value).ok().map(PathBuf::from)
}

/// # Safety
///
/// `handle` must be null or a live handle returned by
/// `fcitx5_config_snapshot_load_current_utf16` that has not been destroyed.
unsafe fn handle_ref(handle: *mut c_void) -> Option<&'static SnapshotHandle> {
    if handle.is_null() {
        return None;
    }
    // SAFETY: the caller's handle contract supplies one live `SnapshotHandle` allocation.
    unsafe { handle.cast::<SnapshotHandle>().as_ref() }
}

fn font_families(snapshot: &ConfigSnapshot, kind: u32) -> Option<&FontFamilies> {
    let fonts = snapshot.fonts();
    match kind {
        FCITX5_CONFIG_FONT_UI => Some(fonts.ui()),
        FCITX5_CONFIG_FONT_CANDIDATE => None,
        FCITX5_CONFIG_FONT_ANNOTATION => None,
        FCITX5_CONFIG_FONT_MONOSPACE => Some(fonts.monospace()),
        _ => None,
    }
}

fn font_family_at(snapshot: &ConfigSnapshot, kind: u32, index: usize) -> Fcitx5ConfigUtf8 {
    let families = match kind {
        FCITX5_CONFIG_FONT_CANDIDATE => snapshot.fonts().candidate().families(),
        FCITX5_CONFIG_FONT_ANNOTATION => snapshot.fonts().annotation().families(),
        _ => font_families(snapshot, kind)
            .map(FontFamilies::families)
            .unwrap_or(&[]),
    };
    families
        .get(index)
        .map_or_else(Fcitx5ConfigUtf8::default, |value| utf8(value))
}

/// Loads one read-only, resolved Current snapshot from `path`.
///
/// Current, last-known-good, and compiled safe-default recovery are selected by
/// `ConfigCore`; this ABI never parses or validates TOML independently.
///
/// # Safety
///
/// `path.ptr` must be null only when `path.len` is zero. Otherwise it must
/// designate `path.len` readable UTF-16 code units for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_config_snapshot_load_current_utf16(
    path: Fcitx5ConfigUtf16,
) -> *mut c_void {
    panic::catch_unwind(|| {
        let Some(path) = utf16_path(path) else {
            return std::ptr::null_mut();
        };
        let Ok(recovery) = ConfigCore::recover(&FileStore::new(), &path) else {
            return std::ptr::null_mut();
        };
        let recovery_source = match recovery.source {
            crate::RecoverySource::Current => 0,
            crate::RecoverySource::LastKnownGood => 1,
            crate::RecoverySource::SafeDefaults => 2,
        };
        Box::into_raw(Box::new(SnapshotHandle {
            snapshot: recovery.core.current(),
            recovery_source,
        }))
        .cast()
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Destroys a snapshot handle returned by `fcitx5_config_snapshot_load_current_utf16`.
///
/// # Safety
///
/// `handle` must be null or a live, unique snapshot handle that has not already
/// been destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_config_snapshot_destroy(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }
    // SAFETY: the caller supplies one unique opaque handle from the load function.
    unsafe { drop(Box::from_raw(handle.cast::<SnapshotHandle>())) };
}

/// Writes a borrowed flat view for a live Config snapshot handle.
///
/// Returned UTF-8 spans remain valid until `fcitx5_config_snapshot_destroy`.
/// Returns `1` on success and `0` for invalid pointers.
///
/// # Safety
///
/// `handle` must be a live snapshot handle and `output` must designate writable
/// storage for one `Fcitx5ConfigSnapshot`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_config_snapshot_view(
    handle: *mut c_void,
    output: *mut Fcitx5ConfigSnapshot,
) -> u8 {
    if output.is_null() {
        return 0;
    }
    // SAFETY: the FFI contract requires a live handle.
    let Some(handle) = (unsafe { handle_ref(handle) }) else {
        return 0;
    };
    // SAFETY: the FFI contract requires writable output storage.
    unsafe { *output = snapshot_view(handle) };
    1
}

/// Returns the enabled input-method ID at `index`, or an empty span when out of range.
///
/// # Safety
///
/// `handle` must be a live snapshot handle. The returned span is borrowed from
/// that handle and becomes invalid when it is destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_config_snapshot_input_method_at(
    handle: *mut c_void,
    index: usize,
) -> Fcitx5ConfigUtf8 {
    // SAFETY: the FFI contract requires a live handle.
    let Some(handle) = (unsafe { handle_ref(handle) }) else {
        return Fcitx5ConfigUtf8::default();
    };
    handle
        .snapshot
        .input_methods()
        .enabled()
        .get(index)
        .map_or_else(Fcitx5ConfigUtf8::default, |value| utf8(value))
}

/// Returns the font fallback at `index` for one documented font selector.
///
/// # Safety
///
/// `handle` must be a live snapshot handle. The returned span is borrowed from
/// that handle and becomes invalid when it is destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_config_snapshot_font_family_at(
    handle: *mut c_void,
    kind: u32,
    index: usize,
) -> Fcitx5ConfigUtf8 {
    // SAFETY: the FFI contract requires a live handle.
    let Some(handle) = (unsafe { handle_ref(handle) }) else {
        return Fcitx5ConfigUtf8::default();
    };
    font_family_at(&handle.snapshot, kind, index)
}

/// Writes the candidate color key and value at `index` in deterministic key order.
///
/// Returns `1` on success and `0` for invalid pointers or an out-of-range index.
///
/// # Safety
///
/// `handle` must be a live snapshot handle. `name` and `value` must designate
/// writable `Fcitx5ConfigUtf8` values. Returned spans borrow `handle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_config_snapshot_candidate_color_at(
    handle: *mut c_void,
    index: usize,
    name: *mut Fcitx5ConfigUtf8,
    value: *mut Fcitx5ConfigUtf8,
) -> u8 {
    if name.is_null() || value.is_null() {
        return 0;
    }
    // SAFETY: the FFI contract requires a live handle.
    let Some(handle) = (unsafe { handle_ref(handle) }) else {
        return 0;
    };
    let Some((name_value, color_value)) = handle.snapshot.candidate().colors().iter().nth(index)
    else {
        return 0;
    };
    // SAFETY: the FFI contract requires writable output storage for both spans.
    unsafe {
        *name = utf8(name_value);
        *value = utf8(color_value);
    }
    1
}
