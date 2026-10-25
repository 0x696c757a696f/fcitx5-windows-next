#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;

pub mod axis_layout;
mod candidate_abi;
pub mod frame_ffi;
pub mod frame_update;
#[cfg(windows)]
mod measure_ffi;
mod presentation;
#[cfg(windows)]
pub mod presentation_server;
pub mod qingfeng;
#[cfg(windows)]
pub mod renderer;
pub mod theme_tokens;
mod ui_plan;
pub use presentation::*;
mod model;
pub use model::*;
mod interaction;
pub use interaction::*;
mod layout_engine;
pub use layout_engine::*;
mod paint;
pub use paint::*;
mod self_check;
pub use self_check::*;
#[cfg(windows)]
pub mod window_host;

#[cfg(all(test, windows))]
mod fuzz_regression;

#[cfg(windows)]
pub use renderer::{
    render_candidate_window, CandidateRenderData, RenderColor, RenderGeometry, RenderTheme,
    RenderWindowInput, RenderWindowOutput,
};

pub use candidate_abi::{
    fcitx5_candidate_ui_apply, fcitx5_candidate_ui_build_plan, fcitx5_candidate_ui_create,
    fcitx5_candidate_ui_destroy, fcitx5_candidate_ui_measurement_texts, Fcitx5CandidateUiColor,
    Fcitx5CandidateUiInput, Fcitx5CandidateUiMeasurement, Fcitx5CandidateUiPlanOutput,
    Fcitx5CandidateUiRenderItemOutput, Fcitx5CandidateUiTextOutput, Fcitx5CandidateUiUiaItemOutput,
};
pub use ui_plan::{
    CandidateRenderItem, CandidateTheme, CandidateUiApplyResult, CandidateUiColor,
    CandidateUiColors, CandidateUiConfig, CandidateUiInput, CandidateUiMeasurement,
    CandidateUiPlan, CandidateUiState, CandidateUiText, CandidateUiaItem, CandidateUiaPlan,
};

pub(crate) const MAX_CANDIDATES: usize = 128;
const MAX_CANDIDATE_TEXT_UTF8: usize = 4096;
const MAX_TRACKED_CONTEXTS: usize = 64;
const MAX_CONTENT_LOCALE_UTF8: usize = 35;
const LOCALE_NAME_MAX_LENGTH: usize = 85;
const DEFAULT_DWRITE_LOCALE: &[u16] = &[
    b'e' as u16,
    b'n' as u16,
    b'-' as u16,
    b'U' as u16,
    b'S' as u16,
];

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetUserDefaultLocaleName(locale_name: *mut u16, locale_name_capacity: i32) -> i32;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationOrientation {
    Automatic,
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Placement {
    Unlocked,
    Below,
    Above,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateLayoutPoint {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateLayoutSize {
    pub width: f32,
    pub height: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateLayoutRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateLayoutInput {
    pub orientation: u32,
    pub caret: Fcitx5CandidateLayoutPoint,
    pub caret_height: f32,
    pub work_area: Fcitx5CandidateLayoutRect,
    pub max_width: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub row_gap: f32,
    pub column_gap: f32,
    pub placement: u32,
    pub scroll_mode: u8,
    pub scroll_columns: usize,
    pub scroll_visible_rows: usize,
    pub selected: usize,
    pub scroll_cell_width: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateLayoutOutput {
    pub window: Fcitx5CandidateLayoutRect,
    pub scrollbar_track: Fcitx5CandidateLayoutRect,
    pub scrollbar_thumb: Fcitx5CandidateLayoutRect,
    pub has_scrollbar: u8,
    pub first_visible: usize,
    pub placement: u32,
    pub item_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateSelectionIntent {
    pub target_process_id: u32,
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub candidate_id: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateUtf16 {
    pub ptr: *const u16,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateCommandLine {
    pub status: u8,
    pub candidate_select_mode: u8,
    pub self_test: u8,
    pub interaction_self_test: u8,
    pub uiless_presentation_self_test: u8,
    pub scroll_expansion_self_test: u8,
    pub locale_self_test: u8,
    pub candidate_ux_self_test: u8,
    pub reload_test: u8,
    pub simulate_device_loss: u8,
    pub scroll_demo: u8,
    pub demo: u8,
    pub test_once: u8,
    pub safe_mode: u8,
    pub has_parent_id: u8,
    pub reserved: u8,
    pub generation_len: usize,
    pub candidate_peer_len: usize,
    pub parent_id: u32,
    pub target_process_id: u32,
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub candidate_id: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateRenderItemInput {
    pub bounds: Fcitx5CandidateLayoutRect,
    pub label_width: f32,
    pub label_gap: f32,
    pub text_width: f32,
    pub comment_width: f32,
    pub has_label: u8,
    pub reserve_label: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateRenderItemOutput {
    pub label: Fcitx5CandidateLayoutRect,
    pub text: Fcitx5CandidateLayoutRect,
    pub comment: Fcitx5CandidateLayoutRect,
    pub draw_comment: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateScrollLabel {
    pub reserve: u8,
    pub show: u8,
    pub slot: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabelStyle {
    Plain,
    Dot,
    Paren,
    Bracket,
    Circled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabelDisplay {
    Always,
    SelectedScope,
    Hidden,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabelScope {
    Item,
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabelAlign {
    Right,
    Left,
    Center,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLabelWidthStrategy {
    Fixed,
    PageMax,
    GridMax,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateLabelSlotConfig {
    pub display: CandidateLabelDisplay,
    pub scope: CandidateLabelScope,
    pub reserve_when_hidden: bool,
    pub align: CandidateLabelAlign,
    pub width_strategy: CandidateLabelWidthStrategy,
    pub min_width: f32,
    pub gap: f32,
}

impl Default for CandidateLabelSlotConfig {
    fn default() -> Self {
        Self {
            display: CandidateLabelDisplay::Always,
            scope: CandidateLabelScope::Item,
            reserve_when_hidden: true,
            align: CandidateLabelAlign::Right,
            width_strategy: CandidateLabelWidthStrategy::PageMax,
            min_width: 0.0,
            gap: 4.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateLabelSlotSource {
    pub candidate_index: usize,
    pub row: usize,
    pub column: usize,
    pub label_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateLabelSlotPlanItem {
    pub reserve_label: bool,
    pub show_label: bool,
    pub label_slot_width: f32,
    pub label_gap: f32,
    pub label_align: CandidateLabelAlign,
    pub text_origin_offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateLabelSlotPlan {
    pub label_slot_width: f32,
    pub items: Vec<CandidateLabelSlotPlanItem>,
    pub stable_text_origin: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateUtf8 {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateModelItem {
    pub id: u64,
    pub label: Fcitx5CandidateUtf8,
    pub text: Fcitx5CandidateUtf8,
    pub comment: Fcitx5CandidateUtf8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidatePresentationText {
    pub text: Fcitx5CandidateUtf8,
    pub comment: Fcitx5CandidateUtf8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateModelSnapshot {
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub preedit: Fcitx5CandidateUtf8,
    pub auxiliary_up: Fcitx5CandidateUtf8,
    pub auxiliary_down: Fcitx5CandidateUtf8,
    pub candidates: *const Fcitx5CandidateModelItem,
    pub candidate_count: usize,
    pub selected: usize,
    pub has_selected: u8,
    pub page: u32,
    pub total: u32,
    pub visibility: u8,
    pub popup_allowed: u8,
}

fn split_windows_argument_string(input: &[u16]) -> Vec<Vec<u16>> {
    let mut arguments = Vec::new();
    let mut index = 0usize;
    while index < input.len() {
        while index < input.len() && matches!(input[index], 0x20 | 0x09) {
            index += 1;
        }
        if index >= input.len() {
            break;
        }
        let mut argument = Vec::new();
        let mut quoted = false;
        while index < input.len() {
            if !quoted && matches!(input[index], 0x20 | 0x09) {
                break;
            }
            let mut backslashes = 0usize;
            while index < input.len() && input[index] == b'\\' as u16 {
                backslashes += 1;
                index += 1;
            }
            if index < input.len() && input[index] == b'"' as u16 {
                argument.extend(std::iter::repeat_n(b'\\' as u16, backslashes / 2));
                if backslashes.is_multiple_of(2) {
                    quoted = !quoted;
                } else {
                    argument.push(b'"' as u16);
                }
                index += 1;
                continue;
            }
            argument.extend(std::iter::repeat_n(b'\\' as u16, backslashes));
            if index >= input.len() {
                break;
            }
            argument.push(input[index]);
            index += 1;
        }
        arguments.push(argument);
    }
    arguments
}

fn wide_ascii(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

fn utf16_eq_ascii(value: &[u16], ascii: &str) -> bool {
    value.len() == ascii.len()
        && value
            .iter()
            .zip(ascii.as_bytes())
            .all(|(left, right)| *left == *right as u16)
}

fn contains_utf16_ascii(haystack: &[u16], needle: &str) -> bool {
    let needle = wide_ascii(needle);
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn find_utf16_ascii(haystack: &[u16], needle: &str) -> Option<usize> {
    let needle = wide_ascii(needle);
    (!needle.is_empty())
        .then(|| {
            haystack
                .windows(needle.len())
                .position(|window| window == needle)
        })
        .flatten()
}

fn parse_u64_utf16(value: &[u16]) -> Option<u64> {
    if value.is_empty() {
        return None;
    }
    let mut parsed = 0_u64;
    for character in value {
        let digit = character.checked_sub(b'0' as u16)?;
        if digit > 9 {
            return None;
        }
        parsed = parsed.checked_mul(10)?.checked_add(u64::from(digit))?;
    }
    Some(parsed)
}

fn parse_parent_id(arguments: &[u16]) -> Option<u32> {
    let marker = wide_ascii("--parent-pid ");
    let begin = find_utf16_ascii(arguments, "--parent-pid ")? + marker.len();
    let mut parsed = 0_u64;
    let mut saw_digit = false;
    for character in &arguments[begin..] {
        let Some(digit) = character.checked_sub(b'0' as u16) else {
            break;
        };
        if digit > 9 {
            break;
        }
        saw_digit = true;
        parsed = parsed.saturating_mul(10).saturating_add(u64::from(digit));
    }
    (saw_digit && parsed != 0).then_some(parsed.min(u64::from(u32::MAX)) as u32)
}

fn write_wide_units(value: &[u16], out: *mut u16, capacity: usize) -> usize {
    if !out.is_null() && capacity != 0 {
        let count = value.len().min(capacity);
        if count != 0 {
            // SAFETY: `out` is non-null and the caller provides `capacity` writable u16 units.
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), out, count);
            }
        }
    }
    value.len()
}

#[must_use]
pub fn default_dwrite_locale() -> Vec<u16> {
    let mut locale = [0_u16; LOCALE_NAME_MAX_LENGTH];
    // SAFETY: `locale` is writable storage for exactly the documented buffer length.
    let length = unsafe { GetUserDefaultLocaleName(locale.as_mut_ptr(), locale.len() as i32) };
    if length > 1 && length as usize <= locale.len() {
        return locale[..length as usize - 1].to_vec();
    }
    DEFAULT_DWRITE_LOCALE.to_vec()
}

#[must_use]
pub fn content_locale_valid(locale: &[u8]) -> bool {
    if locale.is_empty() || locale.len() > MAX_CONTENT_LOCALE_UTF8 {
        return false;
    }
    let mut has_letter = false;
    for character in locale {
        let letter = character.is_ascii_alphabetic();
        has_letter |= letter;
        if !letter && !character.is_ascii_digit() && *character != b'-' {
            return false;
        }
    }
    has_letter
}

#[must_use]
pub fn content_locale_or_default(locale: &[u8]) -> Vec<u16> {
    if !content_locale_valid(locale) {
        return default_dwrite_locale();
    }
    locale
        .iter()
        .map(|character| u16::from(*character))
        .collect()
}

fn scroll_label_policy(
    candidate_index: usize,
    selected_index: usize,
    page_size: usize,
    total_candidates: usize,
) -> Fcitx5CandidateScrollLabel {
    if page_size == 0 || total_candidates <= page_size || candidate_index >= total_candidates {
        return Fcitx5CandidateScrollLabel::default();
    }
    let slot = candidate_index % page_size + 1;
    Fcitx5CandidateScrollLabel {
        reserve: 1,
        show: (candidate_index / page_size == selected_index / page_size) as u8,
        slot: slot.min(u32::MAX as usize) as u32,
    }
}

/// Resolved per-candidate scroll-label reservation (081C glue slice).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateScrollReservation {
    /// 0 skip, 1 reserve the slot, 2 reserve + show the label.
    pub action: u8,
    pub slot: u32,
}

/// Host-supplied config colors in 0..1 float RGBA (081C theme slice).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateConfigColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Resolved 8-bit render theme colors for one paint.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateResolvedColors {
    pub background: [u8; 3],
    pub text: [u8; 3],
    pub selected_background: [u8; 3],
    pub selected_text: [u8; 3],
    pub comment: [u8; 3],
    pub border: [u8; 3],
    pub preedit_text: [u8; 3],
}

const COLOR_WINDOW_COLORREF_BGR: [u8; 3] = [255, 255, 255];
const COLOR_WINDOWTEXT_COLORREF_BGR: [u8; 3] = [0, 0, 0];
const COLOR_HIGHLIGHT_COLORREF_BGR: [u8; 3] = [215, 120, 0];
const COLOR_HIGHLIGHTTEXT_COLORREF_BGR: [u8; 3] = [255, 255, 255];

fn clamp_color_byte(value: f32, high_contrast: bool, system: [u8; 3], channel: usize) -> u8 {
    if high_contrast {
        return system[channel];
    }
    ((value.clamp(0.0, 1.0)) * 255.0).round() as u8
}

/// Resolves one config color into the C++ channel order (R, G, B).
///
/// `system` must already be ordered to match the extraction indices used by
/// the C++ `to_u8` lambda: channel 2 = R, channel 1 = G, channel 0 = B, i.e.
/// the COLORREF little-endian byte layout (B, G, R).
fn resolve_rgb(
    color: Fcitx5CandidateConfigColor,
    high_contrast: bool,
    colorref_bgr: [u8; 3],
) -> [u8; 3] {
    // COLORREF layout: 0x00BBGGRR → byte[0]=B, byte[1]=G, byte[2]=R.
    // to_u8 channel 2 selects R, 1 selects G, 0 selects B.
    [
        clamp_color_byte(color.r, high_contrast, colorref_bgr, 2),
        clamp_color_byte(color.g, high_contrast, colorref_bgr, 1),
        clamp_color_byte(color.b, high_contrast, colorref_bgr, 0),
    ]
}

fn srgb_luminance(color: [u8; 3]) -> f32 {
    let channel = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
}

fn contrast_ratio(first: [u8; 3], second: [u8; 3]) -> f32 {
    let first = srgb_luminance(first);
    let second = srgb_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn readable_selected_text(selected_text: [u8; 3], selected_background: [u8; 3]) -> [u8; 3] {
    // A persisted/custom theme may carry the old green-on-green pair even
    // when the built-in theme is correct. Keep the user's background, but
    // fail closed to whichever neutral text color has stronger contrast when
    // the requested foreground is not readable.
    // Candidate text is rendered at a large UI size; preserve the product's
    // white-on-WeChat-green appearance once it clears the large-text floor.
    // The shipped 27px candidate text is large UI text. Keep the white
    // WeChat-green treatment for the stock accent (#07c160), while still
    // rejecting genuinely invisible custom foregrounds.
    const MIN_SELECTED_TEXT_CONTRAST: f32 = 2.0;
    if contrast_ratio(selected_text, selected_background) >= MIN_SELECTED_TEXT_CONTRAST {
        return selected_text;
    }
    let white = [255, 255, 255];
    let black = [0, 0, 0];
    if contrast_ratio(white, selected_background) >= MIN_SELECTED_TEXT_CONTRAST {
        white
    } else {
        black
    }
}

/// Resolves the eight paint colors from config colors and the high-contrast
/// policy, mirroring the frozen shipping `paintOnceToDC` color mapping.
#[must_use]
pub fn resolve_paint_colors(
    background: Fcitx5CandidateConfigColor,
    candidate_text: Fcitx5CandidateConfigColor,
    selected_background: Fcitx5CandidateConfigColor,
    selected_text: Fcitx5CandidateConfigColor,
    comment_text: Fcitx5CandidateConfigColor,
    border: Fcitx5CandidateConfigColor,
    preedit_text: Fcitx5CandidateConfigColor,
    high_contrast: bool,
) -> Fcitx5CandidateResolvedColors {
    let hc = high_contrast;
    let selected_background = resolve_rgb(selected_background, hc, COLOR_HIGHLIGHT_COLORREF_BGR);
    let selected_text = readable_selected_text(
        resolve_rgb(selected_text, hc, COLOR_HIGHLIGHTTEXT_COLORREF_BGR),
        selected_background,
    );
    Fcitx5CandidateResolvedColors {
        background: resolve_rgb(background, hc, COLOR_WINDOW_COLORREF_BGR),
        text: resolve_rgb(candidate_text, hc, COLOR_WINDOWTEXT_COLORREF_BGR),
        selected_background,
        selected_text,
        comment: resolve_rgb(comment_text, hc, COLOR_WINDOWTEXT_COLORREF_BGR),
        border: resolve_rgb(border, hc, COLOR_WINDOWTEXT_COLORREF_BGR),
        preedit_text: resolve_rgb(preedit_text, hc, COLOR_WINDOWTEXT_COLORREF_BGR),
    }
}

/// Computes the scroll-label reservation for one candidate.
///
/// `source_label` is 0/1 whether the candidate carries its own label; only
/// generated candidates receive a synthesized scroll slot. Mirrors the
/// shipping C++ `applyScrollLabelReservations` loop body.
#[no_mangle]
pub extern "C" fn fcitx5_candidate_scroll_reservation_for(
    candidate_index: usize,
    source_label: u8,
    selected_index: usize,
    scroll_mode: u8,
    labels_visible: u8,
    scroll_columns: usize,
    total_candidates: usize,
) -> Fcitx5CandidateScrollReservation {
    if source_label != 0 {
        return Fcitx5CandidateScrollReservation::default();
    }
    if scroll_mode == 0 || labels_visible == 0 || scroll_columns == 0 {
        return Fcitx5CandidateScrollReservation::default();
    }
    let policy = scroll_label_policy(
        candidate_index,
        selected_index,
        scroll_columns,
        total_candidates,
    );
    if policy.reserve == 0 {
        return Fcitx5CandidateScrollReservation::default();
    }
    Fcitx5CandidateScrollReservation {
        action: if policy.show != 0 { 2 } else { 1 },
        slot: policy.slot,
    }
}

pub fn format_candidate_label(
    slot: u32,
    source_label: &str,
    style: CandidateLabelStyle,
    custom_prefix: &str,
    custom_suffix: &str,
) -> String {
    let label = if source_label.is_empty() {
        if slot == 0 { 1 } else { slot }.to_string()
    } else {
        source_label.to_owned()
    };
    if !custom_prefix.is_empty() || !custom_suffix.is_empty() {
        return format!("{custom_prefix}{label}{custom_suffix}");
    }
    match style {
        CandidateLabelStyle::Plain => label,
        CandidateLabelStyle::Dot => format!("{label}."),
        CandidateLabelStyle::Paren => format!("({label})"),
        CandidateLabelStyle::Bracket => format!("[{label}]"),
        CandidateLabelStyle::Circled => {
            let mut chars = label.chars();
            match (chars.next(), chars.next()) {
                (Some(character @ '1'..='9'), None) => {
                    char::from_u32(0x2460 + character as u32 - '1' as u32)
                        .unwrap_or(character)
                        .to_string()
                }
                _ => label,
            }
        }
    }
}

/// Rust-owned visual construction for one candidate (081D model slice).
///
/// Mirrors the frozen C++ `update()` loop body: the label is formatted when
/// labels are visible and non-empty, the reserved label starts as the label,
/// and a non-empty comment gains the fixed two-space prefix. Scroll-label
/// reservations (081D slice 3) apply only to generated labels: action 1
/// reserves the configured slot label, action 2 also shows it.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateVisualBuildInput {
    pub label: Fcitx5CandidateUtf8,
    pub text: Fcitx5CandidateUtf8,
    pub comment: Fcitx5CandidateUtf8,
    pub label_style: u32,
    pub labels_visible: u8,
    pub reservation_action: u8,
    pub reservation_slot: u32,
}

/// Build-wide configuration: the configured label sequence (1-based slots).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateVisualBuildConfig {
    pub configured_labels: *const Fcitx5CandidateUtf8,
    pub configured_label_count: usize,
}

/// Resolved visual fields; string storage is owned by the Rust arena.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateVisualBuildOutput {
    pub label: Fcitx5CandidateUtf8,
    pub reserved_label: Fcitx5CandidateUtf8,
    pub text: Fcitx5CandidateUtf8,
    pub comment: Fcitx5CandidateUtf8,
    /// 1 when the candidate carries its own formatted label.
    pub source_label: u8,
}

impl Fcitx5CandidateVisualBuildOutput {
    #[must_use]
    pub fn null_output() -> Self {
        Self {
            label: Fcitx5CandidateUtf8::default(),
            reserved_label: Fcitx5CandidateUtf8::default(),
            text: Fcitx5CandidateUtf8::default(),
            comment: Fcitx5CandidateUtf8::default(),
            source_label: 0,
        }
    }
}

/// Arena that owns built visual strings for the current composition.
#[derive(Default)]
pub struct CandidateVisualArena {
    strings: Vec<Vec<u8>>,
    outputs: Vec<Fcitx5CandidateVisualBuildOutput>,
}

impl CandidateVisualArena {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Built outputs from the last [`Self::build`] call; string pointers stay
    /// arena-owned and valid until the next build.
    pub fn built_outputs(&self) -> &[Fcitx5CandidateVisualBuildOutput] {
        &self.outputs
    }

    pub(crate) fn build(
        &mut self,
        inputs: &[Fcitx5CandidateVisualBuildInput],
        config: &Fcitx5CandidateVisualBuildConfig,
    ) -> usize {
        self.strings.clear();
        self.outputs.clear();
        // Each visual record owns four byte buffers. Reserve the outer vector
        // before exposing any pointers; a later Vec reallocation would leave
        // every previously returned `Fcitx5CandidateUtf8::ptr` dangling.
        self.strings.reserve(inputs.len().saturating_mul(4));
        let mut owned = |text: &[u8]| -> Fcitx5CandidateUtf8 {
            self.strings.push(text.to_vec());
            let stored = self.strings.last().expect("pushed above");
            Fcitx5CandidateUtf8 {
                ptr: stored.as_ptr(),
                len: stored.len(),
            }
        };
        // SAFETY: the C ABI contract requires configured_labels to cover
        // configured_label_count elements when non-zero.
        let configured_labels: &[Fcitx5CandidateUtf8] = if config.configured_label_count == 0 {
            &[]
        } else {
            // SAFETY: the C ABI contract requires configured_labels to cover this non-zero count.
            unsafe {
                std::slice::from_raw_parts(config.configured_labels, config.configured_label_count)
            }
        };
        for input in inputs {
            let label_bytes = utf8_slice(input.label);
            let text_bytes = utf8_slice(input.text);
            let comment_bytes = utf8_slice(input.comment);
            let label_text = String::from_utf8_lossy(label_bytes).into_owned();
            let comment_text = String::from_utf8_lossy(comment_bytes).into_owned();
            let Some(style) = label_style_from_ffi(input.label_style) else {
                return self.outputs.len();
            };
            let formatted = if input.labels_visible != 0 && !label_text.is_empty() {
                format_candidate_label(0, &label_text, style, "", "")
            } else {
                String::new()
            };
            let source_label = u8::from(!formatted.is_empty());
            // Generated scroll-label reservations (081D slice 3): mirror the
            // frozen applyScrollLabelReservations semantics.
            let (reserved, show) = if source_label != 0 {
                (formatted.clone(), false)
            } else if matches!(input.reservation_action, 1 | 2) {
                let slot_index = input.reservation_slot.max(1) as usize - 1;
                let slot_source = configured_labels
                    .get(slot_index)
                    .map(|value| String::from_utf8_lossy(utf8_slice(*value)).into_owned())
                    .unwrap_or_default();
                (
                    format_candidate_label(input.reservation_slot, &slot_source, style, "", ""),
                    input.reservation_action == 2,
                )
            } else {
                (String::new(), false)
            };
            let label = if source_label != 0 {
                formatted.clone()
            } else if show {
                reserved.clone()
            } else {
                String::new()
            };
            let comment = if comment_text.is_empty() {
                String::new()
            } else {
                format!("  {comment_text}")
            };
            let output = Fcitx5CandidateVisualBuildOutput {
                label: owned(label.as_bytes()),
                reserved_label: owned(reserved.as_bytes()),
                text: owned(text_bytes),
                comment: owned(comment.as_bytes()),
                source_label,
            };
            self.outputs.push(output);
        }
        self.outputs.len()
    }
}

fn utf8_slice(value: Fcitx5CandidateUtf8) -> &'static [u8] {
    if value.len == 0 {
        return &[];
    }
    // SAFETY: the C ABI contract requires valid `len` readable bytes.
    unsafe { std::slice::from_raw_parts(value.ptr, value.len) }
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_visual_arena_create() -> *mut c_void {
    Box::into_raw(Box::<CandidateVisualArena>::default()).cast()
}

#[no_mangle]
/// # Safety
///
/// `arena` must be null or a pointer returned by
/// `fcitx5_candidate_visual_arena_create` that has not already been destroyed.
pub unsafe extern "C" fn fcitx5_candidate_visual_arena_destroy(arena: *mut c_void) {
    if !arena.is_null() {
        // SAFETY: caller guarantees this is the unique allocation from create.
        unsafe { drop(Box::from_raw(arena.cast::<CandidateVisualArena>())) };
    }
}

#[no_mangle]
/// # Safety
///
/// `arena` must be a valid arena pointer; `inputs` must be valid for
/// `input_count` elements when non-zero; `config.configured_labels` must be
/// valid for `config.configured_label_count` elements when non-zero;
/// `out_outputs` must point to writable storage for `input_count` outputs when
/// non-zero. Returned string pointers remain valid until the next build call
/// or arena destruction.
pub unsafe extern "C" fn fcitx5_candidate_visual_build(
    arena: *mut c_void,
    inputs: *const Fcitx5CandidateVisualBuildInput,
    input_count: usize,
    config: *const Fcitx5CandidateVisualBuildConfig,
    out_outputs: *mut Fcitx5CandidateVisualBuildOutput,
) -> usize {
    if arena.is_null()
        || config.is_null()
        || (input_count > 0 && (inputs.is_null() || out_outputs.is_null()))
    {
        return 0;
    }
    // SAFETY: caller guarantees the arena allocation is valid for this call.
    let arena = unsafe { &mut *arena.cast::<CandidateVisualArena>() };
    // SAFETY: the C ABI contract requires this slice to be valid.
    let inputs = if input_count == 0 {
        &[]
    } else {
        // SAFETY: non-null was checked above and the ABI provides `input_count` readable records.
        unsafe { std::slice::from_raw_parts(inputs, input_count) }
    };
    // SAFETY: non-null checked above; callers provide an initialized config.
    let config = unsafe { *config };
    let built = arena.build(inputs, &config);
    if built > 0 {
        // SAFETY: out_outputs is non-null for non-zero counts and covers all `built` outputs.
        unsafe { std::ptr::copy_nonoverlapping(arena.outputs.as_ptr(), out_outputs, built) };
    }
    built
}

#[cfg(test)]
mod candidate_horizontal_downgrade_tests {
    use super::fcitx5_candidate_horizontal_natural_downgrade;

    #[test]
    fn downgrade_follows_the_frozen_natural_width_contract() {
        let widths = [100.0_f32, 80.0, 60.0];
        // sum + gaps + padding = 240 + 20 + 24 = 284; below limit → keep.
        assert_eq!(
            // SAFETY: the test passes a live `widths` slice for the exact count.
            unsafe {
                fcitx5_candidate_horizontal_natural_downgrade(
                    widths.as_ptr(),
                    3,
                    12.0,
                    10.0,
                    0.0,
                    300.0,
                )
            },
            0
        );
        // Preedit width wins over the natural sum.
        assert_eq!(
            // SAFETY: the test passes a live `widths` slice for the exact count.
            unsafe {
                fcitx5_candidate_horizontal_natural_downgrade(
                    widths.as_ptr(),
                    3,
                    12.0,
                    10.0,
                    320.0,
                    300.0,
                )
            },
            1
        );
        // The +0.5 tolerance holds a borderline width.
        assert_eq!(
            // SAFETY: the test passes a live `widths` slice for the exact count.
            unsafe {
                fcitx5_candidate_horizontal_natural_downgrade(
                    widths.as_ptr(),
                    3,
                    12.0,
                    10.0,
                    0.0,
                    283.5,
                )
            },
            0
        );
        assert_eq!(
            // SAFETY: the test passes a live `widths` slice for the exact count.
            unsafe {
                fcitx5_candidate_horizontal_natural_downgrade(
                    widths.as_ptr(),
                    3,
                    12.0,
                    10.0,
                    0.0,
                    283.4,
                )
            },
            1
        );
    }
}

#[cfg(test)]
mod candidate_window_assembly_tests {
    use super::*;

    fn base_input() -> Fcitx5CandidateWindowAssemblyInput {
        Fcitx5CandidateWindowAssemblyInput {
            presentation: core::ptr::null(),
            window_x: 500.0,
            window_y: 400.0,
            window_w: 407.6,
            window_h: 35.8,
            placement: 1, // Below
            preedit_panel_height: 0.0,
            preedit_panel_width: 0.0,
            row_gap: 8.0,
            max_width: 720.0,
            work_left: 0.0,
            work_top: 0.0,
            work_right: 1920.0,
            work_bottom: 1040.0,
            item_padding_x: 10.0,
            item_padding_y: 8.0,
            viewport_dx: 0.0,
            viewport_dy: 0.0,
            items: core::ptr::null(),
            item_count: 0,
        }
    }

    #[test]
    fn assembly_clamps_to_work_area_and_converts_item_rects() {
        let mut input = base_input();
        input.window_x = 1900.0; // pushed past the right edge
        let items = [Fcitx5CandidateAssemblyItem {
            x: 500.0,
            y: 400.0,
            w: 80.0,
            h: 24.0,
        }];
        input.items = items.as_ptr();
        input.item_count = 1;
        let mut output = Fcitx5CandidateWindowAssemblyOutput::default();
        let mut out_items = [Fcitx5CandidateLayoutRect::default(); 1];
        // SAFETY: this test supplies live input/output/item storage with the declared item count.
        // SAFETY: test supplies valid state and writable decision pointers for this call.
        let ok = unsafe {
            fcitx5_candidate_window_assembly(&input, &mut output, out_items.as_mut_ptr())
        };
        assert_eq!(ok, 1);
        assert!(
            output.window_left + output.window_width <= input.work_right + f32::EPSILON,
            "window must clamp into the work area"
        );
        // Below placement with no preedit keeps item rects at the axis origin.
        assert!((out_items[0].left - (500.0 - output.window_left + 10.0)).abs() < 0.01);
        assert!((out_items[0].top - (400.0 - output.window_top + 8.0)).abs() < 0.01);
        assert_eq!(output.has_scrollbar, 0);
        assert_eq!(output.preedit_divider_y, 0.0);
    }

    #[test]
    fn assembly_width_follows_content_within_one_composition() {
        let mut presentation = CandidatePresentationState::default();
        let mut input = base_input();
        input.presentation = &mut presentation as *mut CandidatePresentationState as *const c_void;
        let mut widths = Vec::new();
        for window_w in [420.0, 140.0, 500.0] {
            input.window_w = window_w;
            let mut output = Fcitx5CandidateWindowAssemblyOutput::default();
            // SAFETY: live input/output storage; the item count is zero.
            let ok = unsafe {
                fcitx5_candidate_window_assembly(&input, &mut output, core::ptr::null_mut())
            };
            assert_eq!(ok, 1);
            widths.push(output.window_width);
        }
        assert_eq!(
            widths,
            [420.0, 140.0, 500.0],
            "backspace shrinks the window again"
        );
    }

    #[test]
    fn assembly_preedit_block_shifts_above_placement_and_sets_panel() {
        let mut input = base_input();
        input.placement = 2; // Above
        input.preedit_panel_height = 30.0;
        input.preedit_panel_width = 120.0;
        input.window_y = 400.0;
        input.window_h = 35.8;
        let mut output = Fcitx5CandidateWindowAssemblyOutput::default();
        // SAFETY: this test supplies live input/output storage and a null zero-length item buffer.
        let ok =
            unsafe { fcitx5_candidate_window_assembly(&input, &mut output, core::ptr::null_mut()) };
        assert_eq!(ok, 1);
        assert!(
            (output.window_height - (35.8_f32 + 30.0 + 8.0).min(input.work_bottom)).abs() < 0.01
        );
        assert!((output.preedit_panel.left - input.item_padding_x).abs() < 0.01);
        assert!((output.preedit_divider_y - (30.0 + 4.0)).abs() < 0.01);
        assert!(
            output.window_top < input.window_y,
            "above placement shifts the window up by the preedit block"
        );
    }

    #[test]
    fn assembly_viewport_overflow_marks_scrollbar() {
        let mut input = base_input();
        input.viewport_dx = 12.0;
        let mut output = Fcitx5CandidateWindowAssemblyOutput::default();
        // SAFETY: this test supplies live input/output storage and a null zero-length item buffer.
        let ok =
            unsafe { fcitx5_candidate_window_assembly(&input, &mut output, core::ptr::null_mut()) };
        assert_eq!(ok, 1);
        assert_eq!(output.has_scrollbar, 1);
        assert!((output.scrollbar_track.left - (input.window_w - 6.0)).abs() < 0.01);
        assert!((output.scrollbar_track.right - (input.window_w - 2.0)).abs() < 0.01);
        assert!(
            output.scrollbar_thumb.left == output.scrollbar_track.left
                && output.scrollbar_thumb.top == output.scrollbar_track.top
                && output.scrollbar_thumb.right == output.scrollbar_track.right
                && output.scrollbar_thumb.bottom == output.scrollbar_track.bottom
        );
    }

    #[test]
    fn assembly_abi_rejects_null_input_or_output() {
        let input = base_input();
        let mut output = Fcitx5CandidateWindowAssemblyOutput::default();
        assert_eq!(
            // SAFETY: the null input is intentional and output is live writable test storage.
            unsafe {
                fcitx5_candidate_window_assembly(
                    core::ptr::null(),
                    &mut output,
                    core::ptr::null_mut(),
                )
            },
            0
        );
        assert_eq!(
            // SAFETY: the null output is intentional and input is live test storage.
            unsafe {
                fcitx5_candidate_window_assembly(
                    &input,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                )
            },
            0
        );
    }
}

#[cfg(test)]
mod candidate_visual_arena_tests {
    use super::*;

    fn input(
        label: &str,
        text: &str,
        comment: &str,
        visible: u8,
    ) -> Fcitx5CandidateVisualBuildInput {
        Fcitx5CandidateVisualBuildInput {
            label: Fcitx5CandidateUtf8 {
                ptr: label.as_ptr(),
                len: label.len(),
            },
            text: Fcitx5CandidateUtf8 {
                ptr: text.as_ptr(),
                len: text.len(),
            },
            comment: Fcitx5CandidateUtf8 {
                ptr: comment.as_ptr(),
                len: comment.len(),
            },
            label_style: 1,
            labels_visible: visible,
            reservation_action: 0,
            reservation_slot: 0,
        }
    }

    fn empty_config() -> Fcitx5CandidateVisualBuildConfig {
        Fcitx5CandidateVisualBuildConfig {
            configured_labels: core::ptr::null(),
            configured_label_count: 0,
        }
    }

    fn read(value: &Fcitx5CandidateUtf8) -> String {
        // SAFETY: arena-owned pointers from a live build are valid.
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(value.ptr, value.len) })
            .into_owned()
    }

    #[test]
    fn build_formats_labels_and_prefixes_comments() {
        let arena = CandidateVisualArena::default();
        let inputs = [
            input("1", "你", "nǐ", 1),
            input("", "好", "", 1),
            input("2", "汉", "hàn", 0),
        ];
        let mut outputs = [Fcitx5CandidateVisualBuildOutput::null_output(); 3];
        let config = empty_config();
        // SAFETY: this test supplies live arena/input/config/output storage for the declared count.
        let built = unsafe {
            fcitx5_candidate_visual_build(
                &arena as *const _ as *mut c_void,
                inputs.as_ptr(),
                3,
                &config,
                outputs.as_mut_ptr(),
            )
        };
        assert_eq!(built, 3);
        assert_eq!(read(&outputs[0].label), "1.");
        assert_eq!(read(&outputs[0].reserved_label), "1.");
        assert_eq!(outputs[0].source_label, 1);
        assert_eq!(read(&outputs[0].comment), "  nǐ");
        assert_eq!(read(&outputs[1].label), "");
        assert_eq!(outputs[1].source_label, 0);
        assert_eq!(read(&outputs[2].label), "", "hidden labels stay empty");
        assert_eq!(outputs[2].source_label, 0);
        assert_eq!(read(&outputs[2].comment), "  hàn");
    }

    #[test]
    fn build_keeps_text_pointers_valid_across_the_full_candidate_batch() {
        let arena = CandidateVisualArena::default();
        let texts: Vec<String> = (0..32).map(|index| format!("候选{index}🙂")).collect();
        let labels: Vec<String> = (1..=32).map(|index| index.to_string()).collect();
        let inputs: Vec<_> = texts
            .iter()
            .enumerate()
            .map(|(index, text)| input(&labels[index], text, "注释", 1))
            .collect();
        let mut outputs = vec![Fcitx5CandidateVisualBuildOutput::null_output(); inputs.len()];
        let config = empty_config();
        // SAFETY: this test supplies live arena/input/config/output storage for the declared count.
        let built = unsafe {
            fcitx5_candidate_visual_build(
                &arena as *const _ as *mut c_void,
                inputs.as_ptr(),
                inputs.len(),
                &config,
                outputs.as_mut_ptr(),
            )
        };
        assert_eq!(built, inputs.len());
        for (output, expected) in outputs.iter().zip(texts) {
            assert_eq!(read(&output.text), expected);
            assert_eq!(read(&output.comment), "  注释");
        }
    }

    #[test]
    fn build_rejects_invalid_style_and_null_arena() {
        let arena = CandidateVisualArena::default();
        let mut bad = input("1", "你", "", 1);
        bad.label_style = 99;
        let mut outputs = [Fcitx5CandidateVisualBuildOutput::null_output(); 1];
        let config = empty_config();
        // SAFETY: this test supplies live arena/input/config/output storage for the declared count.
        let built = unsafe {
            fcitx5_candidate_visual_build(
                &arena as *const _ as *mut c_void,
                &bad,
                1,
                &config,
                outputs.as_mut_ptr(),
            )
        };
        assert_eq!(built, 0, "invalid style stops the build at the offender");
        assert_eq!(
            // SAFETY: null arena is intentional while all other test buffers remain live.
            unsafe {
                fcitx5_candidate_visual_build(
                    core::ptr::null_mut(),
                    &bad,
                    1,
                    &config,
                    outputs.as_mut_ptr(),
                )
            },
            0
        );
    }

    #[test]
    fn build_applies_scroll_label_reservations_to_generated_labels() {
        let arena = CandidateVisualArena::default();
        let configured = ["甲", "乙"];
        let configured_refs = [
            Fcitx5CandidateUtf8 {
                ptr: configured[0].as_ptr(),
                len: configured[0].len(),
            },
            Fcitx5CandidateUtf8 {
                ptr: configured[1].as_ptr(),
                len: configured[1].len(),
            },
        ];
        let config = Fcitx5CandidateVisualBuildConfig {
            configured_labels: configured_refs.as_ptr(),
            configured_label_count: configured_refs.len(),
        };
        let mut inputs = [
            input("1", "你", "", 1),
            input("", "好", "", 1),
            input("", "汉", "", 1),
        ];
        inputs[1].reservation_action = 2;
        inputs[1].reservation_slot = 1; // configured 甲
        inputs[2].reservation_action = 1;
        inputs[2].reservation_slot = 5; // out of range → slot number 5
        let mut outputs = [Fcitx5CandidateVisualBuildOutput::null_output(); 3];
        // SAFETY: this test supplies live arena/input/config/output storage for the declared count.
        let built = unsafe {
            fcitx5_candidate_visual_build(
                &arena as *const _ as *mut c_void,
                inputs.as_ptr(),
                3,
                &config,
                outputs.as_mut_ptr(),
            )
        };
        assert_eq!(built, 3);
        // Source labels keep their own formatted label.
        assert_eq!(read(&outputs[0].label), "1.");
        assert_eq!(read(&outputs[0].reserved_label), "1.");
        // Reserved + shown uses the configured slot label.
        assert_eq!(read(&outputs[1].label), "甲.");
        assert_eq!(read(&outputs[1].reserved_label), "甲.");
        // Reserve-only leaves the shown label empty.
        assert_eq!(read(&outputs[2].label), "");
        assert_eq!(read(&outputs[2].reserved_label), "5.");
    }
}

/// Horizontal natural-width downgrade decision (081D slice 6).
#[no_mangle]
/// # Safety
///
/// `item_widths` must be valid for `item_count` elements when non-zero.
pub unsafe extern "C" fn fcitx5_candidate_horizontal_natural_downgrade(
    item_widths: *const f32,
    item_count: usize,
    padding_x: f32,
    column_gap: f32,
    preedit_width: f32,
    hard_limit: f32,
) -> u8 {
    if item_count > 0 && item_widths.is_null() {
        return 0;
    }
    // SAFETY: the C ABI contract requires this slice to be valid.
    let widths = if item_count == 0 {
        &[]
    } else {
        // SAFETY: non-null was checked above and the ABI supplies `item_count` readable widths.
        unsafe { std::slice::from_raw_parts(item_widths, item_count) }
    };
    let mut natural = 0.0_f32;
    for width in widths {
        if natural > 0.0 {
            natural += column_gap;
        }
        natural += width;
    }
    let natural = (natural + padding_x * 2.0).max(preedit_width);
    u8::from(natural > hard_limit + 0.5)
}

pub fn candidate_label_slot_plan(
    config: CandidateLabelSlotConfig,
    sources: &[CandidateLabelSlotSource],
    selected_index: usize,
) -> CandidateLabelSlotPlan {
    let widest_label = sources.iter().fold(0.0_f32, |width, source| {
        if source.label_width.is_finite() && source.label_width > 0.0 {
            width.max(source.label_width)
        } else {
            width
        }
    });
    let label_slot_width = match config.width_strategy {
        CandidateLabelWidthStrategy::Fixed => config.min_width.max(0.0),
        CandidateLabelWidthStrategy::PageMax | CandidateLabelWidthStrategy::GridMax => {
            config.min_width.max(0.0).max(widest_label)
        }
    };
    let selected = sources
        .iter()
        .find(|source| source.candidate_index == selected_index)
        .copied();
    let label_gap = config.gap.max(0.0);
    let items = sources
        .iter()
        .map(|source| {
            let selected_scope = match (config.scope, selected) {
                (CandidateLabelScope::Item, _) => source.candidate_index == selected_index,
                (CandidateLabelScope::Row, Some(selected)) => source.row == selected.row,
                (CandidateLabelScope::Column, Some(selected)) => source.column == selected.column,
                (_, None) => false,
            };
            let show_label = match config.display {
                CandidateLabelDisplay::Always => true,
                CandidateLabelDisplay::SelectedScope => selected_scope,
                CandidateLabelDisplay::Hidden => false,
            };
            let reserve_label = show_label || config.reserve_when_hidden;
            let text_origin_offset = if reserve_label && label_slot_width > 0.0 {
                label_slot_width + label_gap
            } else {
                0.0
            };
            CandidateLabelSlotPlanItem {
                reserve_label,
                show_label,
                label_slot_width,
                label_gap: if reserve_label { label_gap } else { 0.0 },
                label_align: config.align,
                text_origin_offset,
            }
        })
        .collect::<Vec<_>>();
    let mut text_origins = items
        .iter()
        .filter(|item| item.reserve_label)
        .map(|item| item.text_origin_offset);
    let first_origin = text_origins.next();
    let stable_text_origin = first_origin
        .is_none_or(|first| text_origins.all(|origin| (origin - first).abs() <= f32::EPSILON));
    CandidateLabelSlotPlan {
        label_slot_width,
        items,
        stable_text_origin,
    }
}

pub fn parse_candidate_command_line(
    arguments: &[u16],
) -> (Fcitx5CandidateCommandLine, Vec<u16>, Vec<u16>) {
    let tokens = split_windows_argument_string(arguments);
    let mut generation = Vec::new();
    for pair in tokens.windows(2) {
        if utf16_eq_ascii(&pair[0], "--generation") {
            generation = pair[1].clone();
        }
    }

    let interaction_self_test = contains_utf16_ascii(arguments, "--interaction-self-test");
    let scroll_expansion_self_test =
        contains_utf16_ascii(arguments, "--scroll-expansion-self-test");
    let locale_self_test = contains_utf16_ascii(arguments, "--locale-self-test");
    let candidate_ux_self_test = contains_utf16_ascii(arguments, "--candidate-ux-self-test");
    let scroll_demo = contains_utf16_ascii(arguments, "--scroll-demo");
    let demo = interaction_self_test
        || scroll_expansion_self_test
        || locale_self_test
        || candidate_ux_self_test
        || scroll_demo
        || contains_utf16_ascii(arguments, "--demo");
    let parent_id = parse_parent_id(arguments);

    let mut parsed = Fcitx5CandidateCommandLine {
        status: 1,
        self_test: contains_utf16_ascii(arguments, "--self-test") as u8,
        interaction_self_test: interaction_self_test as u8,
        uiless_presentation_self_test: contains_utf16_ascii(
            arguments,
            "--uiless-presentation-self-test",
        ) as u8,
        scroll_expansion_self_test: scroll_expansion_self_test as u8,
        locale_self_test: locale_self_test as u8,
        candidate_ux_self_test: candidate_ux_self_test as u8,
        reload_test: contains_utf16_ascii(arguments, "--reload-test") as u8,
        simulate_device_loss: contains_utf16_ascii(arguments, "--simulate-device-loss") as u8,
        scroll_demo: scroll_demo as u8,
        demo: demo as u8,
        test_once: contains_utf16_ascii(arguments, "--test-once") as u8,
        safe_mode: contains_utf16_ascii(arguments, "--safe-mode") as u8,
        has_parent_id: parent_id.is_some() as u8,
        parent_id: parent_id.unwrap_or(0),
        generation_len: generation.len(),
        ..Default::default()
    };

    let mut candidate_peer = Vec::new();
    if tokens
        .first()
        .is_some_and(|token| utf16_eq_ascii(token, "--candidate-select-test"))
    {
        if tokens.len() != 8 {
            parsed.candidate_select_mode = 64;
        } else if let (
            Some(target_process_id),
            Some(engine_epoch),
            Some(context_id),
            Some(composition_id),
            Some(revision),
            Some(candidate_id),
        ) = (
            parse_u64_utf16(&tokens[2]),
            parse_u64_utf16(&tokens[3]),
            parse_u64_utf16(&tokens[4]),
            parse_u64_utf16(&tokens[5]),
            parse_u64_utf16(&tokens[6]),
            parse_u64_utf16(&tokens[7]),
        ) {
            if target_process_id <= u64::from(u32::MAX) {
                parsed.candidate_select_mode = 1;
                parsed.target_process_id = target_process_id as u32;
                parsed.engine_epoch = engine_epoch;
                parsed.context_id = context_id;
                parsed.composition_id = composition_id;
                parsed.revision = revision;
                parsed.candidate_id = candidate_id;
                candidate_peer = tokens[1].clone();
                parsed.candidate_peer_len = candidate_peer.len();
            } else {
                parsed.candidate_select_mode = 65;
            }
        } else {
            parsed.candidate_select_mode = 65;
        }
    }

    (parsed, generation, candidate_peer)
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_presentation_create() -> *mut c_void {
    Box::into_raw(Box::<CandidatePresentationState>::default()) as *mut c_void
}

#[no_mangle]
/// # Safety
///
/// `state` must be either null or a pointer returned by
/// `fcitx5_candidate_presentation_create` that has not already been destroyed.
pub unsafe extern "C" fn fcitx5_candidate_presentation_destroy(state: *mut c_void) {
    if state.is_null() {
        return;
    }
    // SAFETY: the API contract makes this a unique allocation from the matching create export.
    drop(unsafe { Box::from_raw(state.cast::<CandidatePresentationState>()) });
}

#[no_mangle]
/// # Safety
///
/// `state` must be either null or a valid pointer returned by
/// `fcitx5_candidate_presentation_create`.
pub unsafe extern "C" fn fcitx5_candidate_presentation_reset(state: *mut c_void) {
    if state.is_null() {
        return;
    }
    // SAFETY: the API contract supplies a live uniquely borrowed presentation allocation.
    unsafe { &mut *state.cast::<CandidatePresentationState>() }.reset();
}

#[no_mangle]
/// # Safety
///
/// `state` and `input` must be valid pointers for the duration of this call.
/// Neither pointer is retained.
pub unsafe extern "C" fn fcitx5_candidate_presentation_apply(
    state: *mut c_void,
    input: *const CandidatePresentationUpdate,
) -> u32 {
    if state.is_null() || input.is_null() {
        return 3;
    }
    // SAFETY: the API contract supplies live state/input storage for this call without retained pointers.
    unsafe { &mut *state.cast::<CandidatePresentationState>() }.apply(unsafe { *input })
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_presentation_create`, and `output` must point to writable
/// storage for one output value.
pub unsafe extern "C" fn fcitx5_candidate_presentation_current(
    state: *mut c_void,
    output: *mut CandidatePresentationOutput,
) -> u8 {
    if state.is_null() || output.is_null() {
        return 0;
    }
    // SAFETY: the API contract supplies live state plus writable output storage.
    unsafe {
        *output = (&*state.cast::<CandidatePresentationState>()).output();
    }
    1
}

#[no_mangle]
/// Writes the Rust-owned visible candidate order for the current presentation.
///
/// # Safety
///
/// `state` must be a valid presentation pointer, `output` must point to writable
/// storage, and `indices` must point to `capacity` writable `usize` values when
/// `capacity` is non-zero. No pointer is retained.
pub unsafe extern "C" fn fcitx5_candidate_presentation_render_plan(
    state: *mut c_void,
    indices: *mut usize,
    capacity: usize,
    output: *mut Fcitx5CandidatePresentationRenderPlan,
) -> u8 {
    if state.is_null() || output.is_null() || (capacity != 0 && indices.is_null()) {
        return 0;
    }
    // SAFETY: the API contract supplies a live presentation allocation for this read.
    let state = unsafe { &*state.cast::<CandidatePresentationState>() };
    let (start, count) = if state.scroll_mode {
        (0, state.candidate_count)
    } else {
        (state.ordinary_start, state.ordinary_count)
    };
    let Some(end) = start.checked_add(count) else {
        return 0;
    };
    if end > state.candidate_count || count > capacity {
        return 0;
    }
    let target = if count == 0 {
        &mut []
    } else {
        // SAFETY: non-null was checked above and `count <= capacity` was verified.
        unsafe { std::slice::from_raw_parts_mut(indices, count) }
    };
    for (slot, candidate_index) in target.iter_mut().zip(start..end) {
        *slot = candidate_index;
    }
    // SAFETY: the API contract supplies writable output storage for one render plan.
    unsafe {
        *output = Fcitx5CandidatePresentationRenderPlan {
            selected: state.selected.unwrap_or_default(),
            has_selected: u8::from(state.selected.is_some()),
            render_count: count,
        };
    }
    1
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_presentation_create`.
pub unsafe extern "C" fn fcitx5_candidate_presentation_set_placement(
    state: *mut c_void,
    placement: u32,
) -> u8 {
    let Some(placement) = placement_from_ffi(placement) else {
        return 0;
    };
    if state.is_null() {
        return 0;
    }
    // SAFETY: the API contract supplies a live uniquely borrowed presentation allocation.
    unsafe { &mut *state.cast::<CandidatePresentationState>() }.set_placement(placement);
    1
}

/// Axis-layout item rect + visibility handed to the window-assembly ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateAssemblyItem {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Inputs for the frozen post-layout window assembly (081D model slice 2).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateWindowAssemblyInput {
    pub presentation: *const c_void,
    pub window_x: f32,
    pub window_y: f32,
    pub window_w: f32,
    pub window_h: f32,
    pub placement: u32,
    pub preedit_panel_height: f32,
    pub preedit_panel_width: f32,
    pub row_gap: f32,
    pub max_width: f32,
    pub work_left: f32,
    pub work_top: f32,
    pub work_right: f32,
    pub work_bottom: f32,
    pub item_padding_x: f32,
    pub item_padding_y: f32,
    pub viewport_dx: f32,
    pub viewport_dy: f32,
    pub items: *const Fcitx5CandidateAssemblyItem,
    pub item_count: usize,
}

/// Frozen assembly result: final window rect, preedit panel, scrollbar, and
/// window-local item rects (written to `out_items` when non-null).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateWindowAssemblyOutput {
    pub window_left: f32,
    pub window_top: f32,
    pub window_width: f32,
    pub window_height: f32,
    pub preedit_panel: Fcitx5CandidateLayoutRect,
    pub preedit_divider_y: f32,
    pub has_scrollbar: u8,
    pub scrollbar_track: Fcitx5CandidateLayoutRect,
    pub scrollbar_thumb: Fcitx5CandidateLayoutRect,
    pub item_count: usize,
}

/// Runs the shipping `update()` window-assembly chain in Rust: preedit block,
/// measured/stable width clamps, work-area clamps, window-local item rects,
/// and scrollbar rects.
///
/// # Safety
///
/// `input.presentation` must be null or a valid presentation pointer; the
/// optional work-area rectangle. When not valid, the item rect conversion is
/// skipped and only the window/preedit/scrollbar fields are populated.
pub fn window_assembly(
    input: &Fcitx5CandidateWindowAssemblyInput,
    out: &mut Fcitx5CandidateWindowAssemblyOutput,
    out_items: &mut [Fcitx5CandidateLayoutRect],
) -> bool {
    let Some(placement) = placement_from_ffi(input.placement) else {
        return false;
    };
    let work_width = (input.work_right - input.work_left).max(0.0);
    let work_height = (input.work_bottom - input.work_top).max(0.0);
    let preedit_block = if input.preedit_panel_height > 0.0 {
        input.preedit_panel_height + input.row_gap
    } else {
        0.0
    };
    let measured_width = (input.window_w.max(input.preedit_panel_width))
        .min(input.max_width)
        .min(work_width);
    // The window follows its content: it shrinks again when a backspace
    // brings back shorter candidates.
    let window_width = measured_width;
    let window_height = (input.window_h + preedit_block).min(work_height);
    let window_left = input
        .window_x
        .clamp(input.work_left, input.work_right - window_width);
    let mut window_top = input.window_y;
    if preedit_block > 0.0 && placement == Placement::Above {
        window_top -= preedit_block;
    }
    window_top = window_top.clamp(input.work_top, input.work_bottom - window_height);

    // Paint always draws the preedit row at the top of the window, so the
    // candidates sit below it whichever side of the caret the window is on.
    let candidate_offset = preedit_block;
    if preedit_block > 0.0 {
        out.preedit_panel = Fcitx5CandidateLayoutRect {
            left: input.item_padding_x,
            top: input.item_padding_y,
            right: window_width - input.item_padding_x,
            bottom: (input.item_padding_y).max(input.preedit_panel_height - input.item_padding_y),
        };
        out.preedit_divider_y = input.preedit_panel_height + input.row_gap / 2.0;
    }
    out.has_scrollbar = u8::from(input.viewport_dx != 0.0 || input.viewport_dy != 0.0);
    // Scrollbar track/thumb mirror the frozen viewport-overflow contract.
    if out.has_scrollbar != 0 {
        out.scrollbar_track = Fcitx5CandidateLayoutRect {
            left: input.window_w - 6.0,
            top: input.item_padding_y + candidate_offset,
            right: input.window_w - 2.0,
            bottom: input.window_h - input.item_padding_y + candidate_offset,
        };
        out.scrollbar_thumb = out.scrollbar_track;
    }
    let count = input.item_count.min(out_items.len());
    // SAFETY: the C ABI contract requires `items` to cover `item_count`.
    let items = if input.item_count == 0 {
        &[]
    } else {
        // SAFETY: `input.items` covers `input.item_count` readable layout items by the C ABI contract.
        unsafe { std::slice::from_raw_parts(input.items, input.item_count) }
    };
    for (index, item) in items.iter().enumerate().take(count) {
        out_items[index] = Fcitx5CandidateLayoutRect {
            left: item.x - window_left + input.item_padding_x,
            top: item.y + candidate_offset - window_top + input.item_padding_y,
            right: item.x + item.w - window_left - input.item_padding_x,
            bottom: item.y + item.h + candidate_offset - window_top - input.item_padding_y,
        };
    }
    out.window_left = window_left;
    out.window_top = window_top;
    out.window_width = window_width;
    out.window_height = window_height;
    out.item_count = count;
    true
}

#[no_mangle]
/// # Safety
///
/// `input` and `output` must point to writable storage; `input.items` must be
/// valid for `input.item_count` elements when non-zero; `out_items` must cover
/// `input.item_count` writable rects when non-zero. `input.presentation` must
/// be null or a valid presentation pointer.
pub unsafe extern "C" fn fcitx5_candidate_window_assembly(
    input: *const Fcitx5CandidateWindowAssemblyInput,
    output: *mut Fcitx5CandidateWindowAssemblyOutput,
    out_items: *mut Fcitx5CandidateLayoutRect,
) -> u8 {
    if input.is_null() || output.is_null() {
        return 0;
    }
    // SAFETY: non-null checked above; callers provide initialized storage.
    let input = unsafe { *input };
    if input.item_count > 0 && (input.items.is_null() || out_items.is_null()) {
        return 0;
    }
    // SAFETY: caller provides writable output storage.
    let mut out = unsafe { *output };
    // SAFETY: `out_items` covers item_count writable rects (validated above).
    let items = if input.item_count == 0 || out_items.is_null() {
        &mut []
    } else {
        // SAFETY: the validated `out_items` pointer covers `input.item_count` writable rects.
        unsafe { std::slice::from_raw_parts_mut(out_items, input.item_count) }
    };
    if !window_assembly(&input, &mut out, items) {
        return 0;
    }
    // SAFETY: caller provides writable output storage.
    unsafe { *output = out };
    1
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_presentation_create`. Candidate arrays and locale bytes
/// must be valid for their declared lengths and are not retained.
pub unsafe extern "C" fn fcitx5_candidate_presentation_resolve_orientation(
    state: *mut c_void,
    configured: u32,
    candidates: *const Fcitx5CandidatePresentationText,
    candidate_count: usize,
    locale: Fcitx5CandidateUtf8,
    work_area: Fcitx5CandidateLayoutRect,
    caret_x: f32,
    scale: f32,
    page_size: u32,
) -> u32 {
    if state.is_null() || (candidate_count != 0 && candidates.is_null()) {
        return 0;
    }
    let Some(configured) = (match configured {
        0 => Some(PresentationOrientation::Automatic),
        1 => Some(PresentationOrientation::Vertical),
        2 => Some(PresentationOrientation::Horizontal),
        _ => None,
    }) else {
        return 0;
    };
    // SAFETY: the ABI keeps the locale byte span readable for this conversion and it is not retained.
    let locale = unsafe { bytes_from_ffi(locale) }
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .unwrap_or_default();
    let candidate_inputs = if candidate_count == 0 {
        &[]
    } else {
        // SAFETY: the validated candidates pointer covers `candidate_count` readable records.
        unsafe { std::slice::from_raw_parts(candidates, candidate_count) }
    };
    let Some(candidates) = candidate_inputs
        .iter()
        .map(|candidate| {
            // SAFETY: the ABI keeps each candidate text span readable during this owned conversion.
            let text = unsafe { bytes_from_ffi(candidate.text) }?;
            // SAFETY: the ABI keeps each candidate comment span readable during this owned conversion.
            let comment = unsafe { bytes_from_ffi(candidate.comment) }?;
            Some(CandidateText {
                text: std::str::from_utf8(text).ok()?.to_owned(),
                comment: std::str::from_utf8(comment).ok()?.to_owned(),
            })
        })
        .collect::<Option<Vec<_>>>()
    else {
        return 0;
    };
    // SAFETY: the non-null state pointer is a uniquely borrowed presentation allocation for this call.
    let orientation = unsafe { &mut *state.cast::<CandidatePresentationState>() }
        .layout
        .resolve_orientation(
            configured,
            AutomaticOrientationInput {
                candidates: &candidates,
                locale,
                work_area: Rect {
                    left: work_area.left,
                    top: work_area.top,
                    right: work_area.right,
                    bottom: work_area.bottom,
                },
                caret_x,
                scale,
                page_size,
            },
        );
    match orientation {
        Orientation::Vertical => 0,
        Orientation::Horizontal => 1,
    }
}

// SAFETY: when non-empty, the caller supplies a non-null, aligned UTF-8 byte span valid for the
// returned borrow; callers immediately copy it and do not retain the forged `'static` reference.
unsafe fn bytes_from_ffi(value: Fcitx5CandidateUtf8) -> Option<&'static [u8]> {
    if value.len == 0 {
        return Some(&[]);
    }
    if value.ptr.is_null() {
        return None;
    }
    // SAFETY: the non-null pointer covers `len` readable bytes for the caller-guaranteed lifetime.
    Some(unsafe { std::slice::from_raw_parts(value.ptr, value.len) })
}

// SAFETY: when non-empty, the caller supplies a non-null, aligned UTF-16 span valid for the
// returned borrow; callers immediately copy it and do not retain the forged `'static` reference.
unsafe fn utf16_from_ffi(value: Fcitx5CandidateUtf16) -> Option<&'static [u16]> {
    if value.len == 0 {
        return Some(&[]);
    }
    if value.ptr.is_null() {
        return None;
    }
    // SAFETY: the non-null pointer covers `len` readable u16 elements for the caller-guaranteed lifetime.
    Some(unsafe { std::slice::from_raw_parts(value.ptr, value.len) })
}

fn label_style_from_ffi(value: u32) -> Option<CandidateLabelStyle> {
    match value {
        0 => Some(CandidateLabelStyle::Plain),
        1 => Some(CandidateLabelStyle::Dot),
        2 => Some(CandidateLabelStyle::Paren),
        3 => Some(CandidateLabelStyle::Bracket),
        4 => Some(CandidateLabelStyle::Circled),
        _ => None,
    }
}

#[cfg(test)]
mod candidate_paint_color_tests {
    use super::*;

    fn color(r: f32, g: f32, b: f32) -> Fcitx5CandidateConfigColor {
        Fcitx5CandidateConfigColor { r, g, b, a: 1.0 }
    }

    #[test]
    fn paint_colors_clamp_and_scale_config_colors() {
        let resolved = resolve_paint_colors(
            color(0.97, 0.98, 0.98),
            color(0.13, 0.13, 0.14),
            color(0.027, 0.757, 0.376),
            color(0.027, 0.757, 0.376),
            color(0.13, 0.13, 0.14),
            color(0.82, 0.82, 0.82),
            color(0.13, 0.13, 0.14),
            false,
        );
        assert_eq!(resolved.background, [247, 250, 250]);
        assert_eq!(resolved.selected_background, [7, 193, 96]);
        assert_eq!(resolved.selected_text, [255, 255, 255]);
        assert_eq!(resolved.border, [209, 209, 209]);
    }

    #[test]
    fn selected_text_fails_closed_when_a_theme_requests_green_on_green() {
        let resolved = resolve_paint_colors(
            color(1.0, 1.0, 1.0),
            color(0.1, 0.1, 0.1),
            color(0.027, 0.757, 0.376),
            color(0.027, 0.757, 0.376),
            color(0.1, 0.1, 0.1),
            color(0.8, 0.8, 0.8),
            color(0.1, 0.1, 0.1),
            false,
        );
        assert_eq!(resolved.selected_text, [255, 255, 255]);
        assert!(contrast_ratio(resolved.selected_text, resolved.selected_background) >= 2.0);
    }

    #[test]
    fn high_contrast_maps_colors_to_system_palette() {
        let resolved = resolve_paint_colors(
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(0.0, 0.0, 0.0),
            true,
        );
        assert_eq!(resolved.background, COLOR_WINDOW_COLORREF_BGR);
        assert_eq!(resolved.text, COLOR_WINDOWTEXT_COLORREF_BGR);
        assert_eq!(
            resolved.selected_background,
            [0, 120, 215],
            "COLORREF [B,G,R] extracts to RGB"
        );
        assert_eq!(resolved.selected_text, COLOR_HIGHLIGHTTEXT_COLORREF_BGR);
    }
}

#[cfg(test)]
mod candidate_scroll_reservation_tests {
    use super::fcitx5_candidate_scroll_reservation_for;

    #[test]
    fn reservations_follow_the_frozen_scroll_label_policy() {
        // Source labels keep their own label; no synthesized slot.
        let source = fcitx5_candidate_scroll_reservation_for(3, 1, 0, 1, 1, 6, 60);
        assert_eq!(source.action, 0);
        // Non-scroll or hidden labels never reserve.
        let off = fcitx5_candidate_scroll_reservation_for(3, 0, 0, 0, 1, 6, 60);
        assert_eq!(off.action, 0);
        let hidden = fcitx5_candidate_scroll_reservation_for(3, 0, 0, 1, 0, 6, 60);
        assert_eq!(hidden.action, 0);
        // Current page reserves and shows (slot = index % page + 1).
        let show = fcitx5_candidate_scroll_reservation_for(3, 0, 3, 1, 1, 6, 60);
        assert_eq!(show.action, 2);
        assert_eq!(show.slot, 4);
        // Other pages reserve but hide.
        let reserve = fcitx5_candidate_scroll_reservation_for(9, 0, 3, 1, 1, 6, 60);
        assert_eq!(reserve.action, 1);
        assert_eq!(reserve.slot, 4);
    }
}

#[cfg(test)]
mod candidate_presentation_decision_tests {
    use super::{CandidatePresentationDecision, CandidatePresentationState};

    #[test]
    fn decide_follows_the_frozen_update_dismiss_chain() {
        let state = CandidatePresentationState::default();
        assert_eq!(
            state.decide(0, 5, true, true),
            1,
            "hidden visibility dismisses"
        );
        assert_eq!(
            state.decide(1, 0, true, true),
            1,
            "empty candidates dismiss"
        );
        assert_eq!(
            state.decide(1, 5, false, true),
            1,
            "invalid caret dismisses"
        );
        assert_eq!(
            state.decide(1, 5, true, false),
            2,
            "popup-blocked composition only hides"
        );
        assert_eq!(state.decide(2, 5, true, true), 0, "normal update paints");
    }

    #[test]
    fn decision_abi_rejects_null_state_or_output() {
        let state = CandidatePresentationState::default();
        let mut decision = CandidatePresentationDecision::default();
        // SAFETY: this test passes a live presentation state and writable decision output.
        let ok = unsafe {
            super::fcitx5_candidate_presentation_decide(
                (&state as *const CandidatePresentationState).cast(),
                1,
                5,
                1,
                1,
                0,
                &mut decision,
            )
        };
        assert_eq!(ok, 1);
        assert_eq!(decision.action, 0);
    }
}

#[no_mangle]
/// # Safety
///
/// `arguments` must be null only when `arguments_len` is zero, or point to a
/// readable UTF-16 buffer with exactly the provided length. Output buffers may
/// be null for size queries or point to writable UTF-16 storage for their
/// capacities. No pointer is retained.
pub unsafe extern "C" fn fcitx5_candidate_parse_command_line_utf16(
    arguments: *const u16,
    arguments_len: usize,
    generation_out: *mut u16,
    generation_capacity: usize,
    candidate_peer_out: *mut u16,
    candidate_peer_capacity: usize,
) -> Fcitx5CandidateCommandLine {
    if arguments.is_null() && arguments_len != 0 {
        return Fcitx5CandidateCommandLine::default();
    }
    let arguments = if arguments.is_null() {
        &[]
    } else {
        // SAFETY: the ABI contract keeps `arguments_len` UTF-16 units readable for this call.
        unsafe { std::slice::from_raw_parts(arguments, arguments_len) }
    };
    let (mut parsed, generation, candidate_peer) = parse_candidate_command_line(arguments);
    parsed.generation_len = write_wide_units(&generation, generation_out, generation_capacity);
    parsed.candidate_peer_len =
        write_wide_units(&candidate_peer, candidate_peer_out, candidate_peer_capacity);
    parsed
}

#[no_mangle]
/// # Safety
///
/// `locale_out` may be null for size queries or point to writable UTF-16
/// storage for `locale_capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_candidate_default_dwrite_locale_utf16(
    locale_out: *mut u16,
    locale_capacity: usize,
) -> usize {
    write_wide_units(&default_dwrite_locale(), locale_out, locale_capacity)
}

#[no_mangle]
/// # Safety
///
/// `locale` must either point to `locale.len` readable bytes or be null when
/// `locale.len` is zero. The pointer is not retained.
pub unsafe extern "C" fn fcitx5_candidate_content_locale_valid_utf8(
    locale: Fcitx5CandidateUtf8,
) -> u8 {
    // SAFETY: the ABI keeps the locale byte span readable for this call and it is not retained.
    let Some(locale) = (unsafe { bytes_from_ffi(locale) }) else {
        return 0;
    };
    content_locale_valid(locale) as u8
}

#[no_mangle]
/// # Safety
///
/// `locale` must either point to `locale.len` readable bytes or be null when
/// `locale.len` is zero. `locale_out` may be null for size queries or point to
/// writable UTF-16 storage for `locale_capacity` code units. No pointer is
/// retained.
pub unsafe extern "C" fn fcitx5_candidate_content_locale_or_default_utf16(
    locale: Fcitx5CandidateUtf8,
    locale_out: *mut u16,
    locale_capacity: usize,
) -> usize {
    // SAFETY: the ABI keeps the locale byte span readable for this call and it is not retained.
    let locale = (unsafe { bytes_from_ffi(locale) }).unwrap_or_default();
    write_wide_units(
        &content_locale_or_default(locale),
        locale_out,
        locale_capacity,
    )
}

#[no_mangle]
/// # Safety
///
/// `locale` must either point to `locale.len` readable bytes or be null when
/// `locale.len` is zero. The pointer is not retained.
pub unsafe extern "C" fn fcitx5_candidate_locale_prefers_compact_horizontal_utf8(
    locale: Fcitx5CandidateUtf8,
) -> u8 {
    // SAFETY: the ABI keeps the locale byte span readable for this call and it is not retained.
    let Some(locale) = (unsafe { bytes_from_ffi(locale) }) else {
        return 0;
    };
    let Ok(locale) = std::str::from_utf8(locale) else {
        return 0;
    };
    locale_prefers_compact_horizontal(locale) as u8
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_scroll_label_policy(
    candidate_index: usize,
    selected_index: usize,
    page_size: usize,
    total_candidates: usize,
) -> Fcitx5CandidateScrollLabel {
    scroll_label_policy(candidate_index, selected_index, page_size, total_candidates)
}

/// # Safety
///
/// `source_label`, `custom_prefix`, and `custom_suffix` must point to readable
/// UTF-16 buffers for their declared lengths, or be null only when the length
/// is zero. `output` may be null for size queries or point to writable UTF-16
/// storage for `output_capacity` code units. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_format_label_utf16(
    slot: u32,
    style: u32,
    source_label: Fcitx5CandidateUtf16,
    custom_prefix: Fcitx5CandidateUtf16,
    custom_suffix: Fcitx5CandidateUtf16,
    output: *mut u16,
    output_capacity: usize,
) -> usize {
    let Some(style) = label_style_from_ffi(style) else {
        return 0;
    };
    // SAFETY: the ABI keeps the source label UTF-16 span readable for this owned conversion.
    let Some(source_label) = (unsafe { utf16_from_ffi(source_label) }) else {
        return 0;
    };
    // SAFETY: the ABI keeps the prefix UTF-16 span readable for this owned conversion.
    let Some(custom_prefix) = (unsafe { utf16_from_ffi(custom_prefix) }) else {
        return 0;
    };
    // SAFETY: the ABI keeps the suffix UTF-16 span readable for this owned conversion.
    let Some(custom_suffix) = (unsafe { utf16_from_ffi(custom_suffix) }) else {
        return 0;
    };
    let source_label = String::from_utf16_lossy(source_label);
    let custom_prefix = String::from_utf16_lossy(custom_prefix);
    let custom_suffix = String::from_utf16_lossy(custom_suffix);
    let formatted =
        format_candidate_label(slot, &source_label, style, &custom_prefix, &custom_suffix);
    write_wide_units(
        &formatted.encode_utf16().collect::<Vec<_>>(),
        output,
        output_capacity,
    )
}

pub fn candidate_render_segments(
    items: &[Fcitx5CandidateRenderItemInput],
) -> (Vec<Fcitx5CandidateRenderItemOutput>, f32) {
    const LABEL_CELL_SAFETY_PADDING: f32 = 2.0;
    let label_column_width = items.iter().fold(0.0_f32, |width, item| {
        if item.reserve_label != 0 || item.has_label != 0 {
            width.max(item.label_width.max(0.0) + LABEL_CELL_SAFETY_PADDING)
        } else {
            width
        }
    });
    let outputs = items
        .iter()
        .map(|source| {
            let reserve_label = source.reserve_label != 0 || source.has_label != 0;
            let effective_label_column_width = if reserve_label {
                label_column_width
            } else {
                0.0
            };
            let label_gap = if reserve_label && label_column_width > 0.0 {
                source.label_gap.max(0.0)
            } else {
                0.0
            };
            let text_left = source.bounds.left + effective_label_column_width + label_gap;
            let text_available = (source.bounds.right - text_left).max(1.0);
            let text_draw_width = source.text_width.max(0.0).min(text_available);
            let comment_left = text_left + text_draw_width + 4.0;
            let comment_available = (source.bounds.right - comment_left).max(1.0);
            let draw_comment = u8::from(
                source.comment_width <= 0.0 || source.comment_width <= comment_available + 2.0,
            );
            Fcitx5CandidateRenderItemOutput {
                label: Fcitx5CandidateLayoutRect {
                    left: source.bounds.left,
                    top: source.bounds.top,
                    right: source.bounds.left + effective_label_column_width,
                    bottom: source.bounds.bottom,
                },
                text: Fcitx5CandidateLayoutRect {
                    left: text_left,
                    top: source.bounds.top,
                    right: source.bounds.right,
                    bottom: source.bounds.bottom,
                },
                comment: Fcitx5CandidateLayoutRect {
                    left: comment_left,
                    top: source.bounds.top,
                    right: source.bounds.right,
                    bottom: source.bounds.bottom,
                },
                draw_comment,
            }
        })
        .collect::<Vec<_>>();
    (outputs, label_column_width)
}

#[no_mangle]
/// # Safety
///
/// `items` and `out_items` must be valid for `item_count` elements when
/// `item_count` is non-zero. Pointers are not retained.
pub unsafe extern "C" fn fcitx5_candidate_render_segments(
    items: *const Fcitx5CandidateRenderItemInput,
    item_count: usize,
    _horizontal_layout: u8,
    _scroll_mode: u8,
    out_items: *mut Fcitx5CandidateRenderItemOutput,
    out_label_column_width: *mut f32,
) -> i32 {
    if out_label_column_width.is_null()
        || (item_count > 0 && (items.is_null() || out_items.is_null()))
    {
        return 1;
    }
    let items = if item_count == 0 {
        &[]
    } else {
        // SAFETY: the validated items pointer covers `item_count` readable render-item inputs.
        unsafe { std::slice::from_raw_parts(items, item_count) }
    };
    let out_items = if item_count == 0 {
        &mut []
    } else {
        // SAFETY: the validated output pointer covers `item_count` writable segment outputs.
        unsafe { std::slice::from_raw_parts_mut(out_items, item_count) }
    };
    let (segments, label_column_width) = candidate_render_segments(items);
    // SAFETY: the validated label-width pointer is writable for one f32 result.
    unsafe {
        *out_label_column_width = label_column_width;
    }
    out_items.copy_from_slice(&segments);
    0
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateAxisLayoutItemOutput {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub visible: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateAxisLayoutOutput {
    pub window_x: f32,
    pub window_y: f32,
    pub window_w: f32,
    pub window_h: f32,
    pub placement: u32,
    pub viewport_dx: f32,
    pub viewport_dy: f32,
    pub content_width: f32,
    pub content_height: f32,
    pub first_visible: usize,
    pub item_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateAxisLayoutInput {
    pub page_size: u32,
    pub max_width: f32,
    pub max_height: f32,
    pub dpi_scale: f32,
    pub highlighted_index: usize,
    pub orientation: u8,
    pub overflow: u8,
    pub writing: u8,
    pub caret: Fcitx5CandidateLayoutPoint,
    pub caret_height: f32,
    pub work_area: Fcitx5CandidateLayoutRect,
    pub padding_x: f32,
    pub padding_y: f32,
    pub row_gap: f32,
    pub column_gap: f32,
    pub placement: u32,
    /// Manual scroll override in DIP pixels. -1.0 = auto (scroll-into-view);
    /// >= 0 = use this offset clamped to [0, scroll_max].
    pub scroll_override_px: f32,
}

fn axis_overflow_from_ffi(value: u8) -> Option<axis_layout::OverflowBehavior> {
    match value {
        0 => Some(axis_layout::OverflowBehavior::Paging),
        1 => Some(axis_layout::OverflowBehavior::Scrolling),
        2 => Some(axis_layout::OverflowBehavior::Wrapping),
        _ => None,
    }
}

fn axis_writing_from_ffi(value: u8) -> Option<axis_layout::WritingMode> {
    match value {
        0 => Some(axis_layout::WritingMode::Horizontal),
        1 => Some(axis_layout::WritingMode::VerticalRl),
        2 => Some(axis_layout::WritingMode::VerticalLr),
        _ => None,
    }
}

#[no_mangle]
/// # Safety
///
/// `input` and `output` must point to valid storage. `items` must be valid for
/// `item_count` elements and `out_items` for `out_capacity` elements when their
/// count is non-zero; out_capacity must cover item_count. Geometry values are
/// caller-scaled (the native DWrite adapter already multiplies by the DPI
/// scale), so `dpi_scale` only gates validity. Pointers are not retained.
pub unsafe extern "C" fn fcitx5_candidate_axis_layout(
    input: *const Fcitx5CandidateAxisLayoutInput,
    items: *const Fcitx5CandidateLayoutSize,
    item_count: usize,
    out_items: *mut Fcitx5CandidateAxisLayoutItemOutput,
    out_capacity: usize,
    output: *mut Fcitx5CandidateAxisLayoutOutput,
) -> i32 {
    if input.is_null() || output.is_null() || item_count > out_capacity {
        return 1;
    }
    if item_count > 0 && (items.is_null() || out_items.is_null()) {
        return 1;
    }
    // SAFETY: the validated input pointer is aligned and readable for one axis-layout input.
    let input = unsafe { *input };
    if !input.dpi_scale.is_finite() || !(0.5..=4.0).contains(&input.dpi_scale) {
        return 1;
    }
    let orientation = match input.orientation {
        0 => Orientation::Horizontal,
        1 => Orientation::Vertical,
        _ => return 1,
    };
    let Some(overflow) = axis_overflow_from_ffi(input.overflow) else {
        return 1;
    };
    let Some(writing_mode) = axis_writing_from_ffi(input.writing) else {
        return 1;
    };
    let Some(placement) = placement_from_ffi(input.placement) else {
        return 1;
    };
    let item_slice = if item_count == 0 {
        &[]
    } else {
        // SAFETY: the validated items pointer covers `item_count` readable sizes.
        unsafe { std::slice::from_raw_parts(items, item_count) }
    };
    let result = axis_layout::layout(&axis_layout::AxisLayoutInput {
        options: axis_layout::CandidateLayoutOptions {
            orientation,
            overflow,
            writing_mode,
        },
        items: item_slice
            .iter()
            .map(|item| Size {
                width: item.width,
                height: item.height,
            })
            .collect(),
        caret: Point {
            x: input.caret.x,
            y: input.caret.y,
        },
        caret_height: input.caret_height,
        work_area: Rect {
            left: input.work_area.left,
            top: input.work_area.top,
            right: input.work_area.right,
            bottom: input.work_area.bottom,
        },
        max_width: input.max_width,
        max_height: input.max_height,
        padding_x: input.padding_x,
        padding_y: input.padding_y,
        row_gap: input.row_gap,
        column_gap: input.column_gap,
        page_size: input.page_size as usize,
        selected: input.highlighted_index,
        scroll_override: if input.scroll_override_px >= 0.0 {
            Some(input.scroll_override_px)
        } else {
            None
        },
        placement,
    });
    if result.items.len() > out_capacity {
        return 1;
    }
    if !result.items.is_empty() {
        // SAFETY: the validated output buffer covers `result.items.len()` writable entries.
        let out_items = unsafe { std::slice::from_raw_parts_mut(out_items, result.items.len()) };
        for (target, source) in out_items.iter_mut().zip(result.items.iter()) {
            *target = Fcitx5CandidateAxisLayoutItemOutput {
                x: source.rect.left,
                y: source.rect.top,
                w: source.rect.right - source.rect.left,
                h: source.rect.bottom - source.rect.top,
                visible: u8::from(source.visible),
            };
        }
    }
    let (viewport_dx, viewport_dy) = result.viewport_offset.unwrap_or((0.0, 0.0));
    // SAFETY: the validated output pointer is writable for one axis-layout result.
    unsafe {
        *output = Fcitx5CandidateAxisLayoutOutput {
            window_x: result.window.left,
            window_y: result.window.top,
            window_w: result.window.right - result.window.left,
            window_h: result.window.bottom - result.window.top,
            placement: placement_to_ffi(result.placement),
            viewport_dx,
            viewport_dy,
            content_width: result.content_size.width,
            content_height: result.content_size.height,
            first_visible: result.first_visible,
            item_count: result.items.len(),
        };
    }
    0
}

fn orientation_from_ffi(value: u32) -> Option<Orientation> {
    match value {
        0 => Some(Orientation::Vertical),
        1 => Some(Orientation::Horizontal),
        _ => None,
    }
}

pub(crate) fn placement_from_ffi(value: u32) -> Option<Placement> {
    match value {
        0 => Some(Placement::Unlocked),
        1 => Some(Placement::Below),
        2 => Some(Placement::Above),
        _ => None,
    }
}

pub(crate) fn placement_to_ffi(value: Placement) -> u32 {
    match value {
        Placement::Unlocked => 0,
        Placement::Below => 1,
        Placement::Above => 2,
    }
}

fn rect_to_ffi(rect: Rect) -> Fcitx5CandidateLayoutRect {
    Fcitx5CandidateLayoutRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immutable_ui_plan_keeps_cjk_annotation_and_selected_uia_semantics() {
        let mut state = CandidateUiState::default();
        assert_eq!(
            state.apply(CandidateUiInput {
                snapshot: CandidateSemanticSnapshot {
                    identity: CandidateSnapshotIdentity {
                        engine_epoch: 1,
                        context_id: 7,
                        composition_id: 9,
                        revision: 1,
                    },
                    preedit: "ni".to_owned(),
                    auxiliary_up: String::new(),
                    auxiliary_down: String::new(),
                    candidates: vec![
                        CandidateSemanticItem {
                            id: 1,
                            label: "1".to_owned(),
                            text: "你".to_owned(),
                            comment: "nǐ".to_owned(),
                        },
                        CandidateSemanticItem {
                            id: 2,
                            label: "2".to_owned(),
                            text: "呢".to_owned(),
                            comment: "ne".to_owned(),
                        },
                    ],
                    selected: Some(0),
                    page: 0,
                    total: 2,
                    visibility: 1,
                    popup_allowed: true,
                },
                locale: "zh-CN".to_owned(),
                caret: Point { x: 40.0, y: 60.0 },
                caret_height: 24.0,
                work_area: Rect {
                    left: 0.0,
                    top: 0.0,
                    right: 800.0,
                    bottom: 600.0,
                },
                candidate_bulk: false,
                config: CandidateUiConfig::default(),
            }),
            CandidateUiApplyResult::Applied
        );

        let plan = state.render_plan(&[
            CandidateUiMeasurement::new(18.0, 20.0, 24.0, 26.0),
            CandidateUiMeasurement::new(18.0, 20.0, 16.0, 26.0),
        ]);

        assert!(plan.popup_visible);
        assert_eq!(plan.orientation, Orientation::Horizontal);
        assert_eq!(plan.items.len(), 2);
        assert!(plan.items[0].text_rect.right - plan.items[0].text_rect.left >= 22.0);
        assert_eq!(plan.uia.items[0].name, "1. 你 nǐ");
        assert!(plan.uia.items[0].selected);
        assert!(!plan.uia.items[1].selected);
    }

    #[test]
    fn immutable_ui_plan_keeps_selected_candidate_in_both_six_cell_scroll_axes() {
        let candidates = (0..42)
            .map(|index| CandidateSemanticItem {
                id: (index + 1) as u64,
                label: ((index % 6) + 1).to_string(),
                text: format!("候选{}", index + 1),
                comment: String::new(),
            })
            .collect::<Vec<_>>();
        let input = |orientation| CandidateUiInput {
            snapshot: CandidateSemanticSnapshot {
                identity: CandidateSnapshotIdentity {
                    engine_epoch: 1,
                    context_id: 8,
                    composition_id: 10,
                    revision: 1,
                },
                preedit: String::new(),
                auxiliary_up: String::new(),
                auxiliary_down: String::new(),
                candidates: candidates.clone(),
                selected: Some(30),
                page: 5,
                total: 42,
                visibility: 1,
                popup_allowed: true,
            },
            locale: "zh-CN".to_owned(),
            caret: Point { x: 40.0, y: 60.0 },
            caret_height: 24.0,
            work_area: Rect {
                left: 0.0,
                top: 0.0,
                right: 1600.0,
                bottom: 1200.0,
            },
            candidate_bulk: true,
            config: CandidateUiConfig {
                orientation,
                scroll_mode: true,
                page_size: 6,
                ..CandidateUiConfig::default()
            },
        };
        let measurements = vec![CandidateUiMeasurement::new(18.0, 40.0, 0.0, 26.0); 42];

        for orientation in [
            PresentationOrientation::Vertical,
            PresentationOrientation::Horizontal,
        ] {
            let mut state = CandidateUiState::default();
            assert_eq!(
                state.apply(input(orientation)),
                CandidateUiApplyResult::Applied
            );
            let plan = state.render_plan(&measurements);
            assert_eq!(plan.items.len(), 36);
            assert!(plan
                .items
                .iter()
                .any(|item| item.candidate_index == 30 && item.selected));
            assert!(plan
                .items
                .iter()
                .all(|item| item.text_rect.right > item.text_rect.left));
        }
    }

    fn snapshot(revision: u64) -> CandidateSnapshot {
        snapshot_with_identity(10, 20, 30, revision)
    }

    fn snapshot_with_identity(
        engine_epoch: u64,
        context_id: u64,
        composition_id: u64,
        revision: u64,
    ) -> CandidateSnapshot {
        CandidateSnapshot {
            engine_epoch,
            context_id,
            composition_id,
            revision,
            preedit: b"ni".to_vec(),
            auxiliary_up: Vec::new(),
            auxiliary_down: Vec::new(),
            candidates: vec![
                CandidateItem {
                    id: 1,
                    label: b"1".to_vec(),
                    text: "你".as_bytes().to_vec(),
                    comment: "nǐ".as_bytes().to_vec(),
                },
                CandidateItem {
                    id: 2,
                    label: b"2".to_vec(),
                    text: "呢".as_bytes().to_vec(),
                    comment: Vec::new(),
                },
            ],
            selected: Some(0),
            page: 0,
            total: 2,
            visibility: Visibility::Composition,
            popup_allowed: true,
        }
    }

    #[test]
    fn semantic_snapshot_is_the_model_projection_for_all_consumers() {
        let mut model = CandidateModel::default();
        assert_eq!(model.apply(snapshot(1)), 0);
        let semantic = model.semantic_snapshot().expect("semantic snapshot");
        assert_eq!(semantic.identity.revision, 1);
        assert_eq!(semantic.selected, Some(0));
        assert_eq!(semantic.candidates[0].text, "你");
    }

    #[test]
    fn semantic_snapshot_rejects_invalid_utf8_instead_of_replacing_text() {
        let mut model = CandidateModel::default();
        let mut invalid = snapshot(1);
        invalid.candidates[0].text = vec![0xff];
        assert_eq!(model.apply(invalid), 3);
        assert!(model.semantic_snapshot().is_none());
    }

    #[test]
    fn ffi_current_snapshot_is_the_model_projection_for_native_adapters() {
        let mut model = CandidateModel::default();
        assert_eq!(model.apply(snapshot(1)), 0);
        let mut output = Fcitx5CandidateModelSnapshot {
            engine_epoch: 0,
            context_id: 0,
            composition_id: 0,
            revision: 0,
            preedit: Fcitx5CandidateUtf8 {
                ptr: std::ptr::null(),
                len: 0,
            },
            auxiliary_up: Fcitx5CandidateUtf8 {
                ptr: std::ptr::null(),
                len: 0,
            },
            auxiliary_down: Fcitx5CandidateUtf8 {
                ptr: std::ptr::null(),
                len: 0,
            },
            candidates: std::ptr::null(),
            candidate_count: 0,
            selected: 0,
            has_selected: 0,
            page: 0,
            total: 0,
            visibility: 0,
            popup_allowed: 0,
        };
        assert_eq!(
            // SAFETY: model and output are valid, uniquely borrowed test storage.
            unsafe {
                fcitx5_candidate_model_current(
                    (&mut model as *mut CandidateModel).cast(),
                    &mut output,
                )
            },
            1
        );
        assert_eq!(output.revision, 1);
        assert_eq!(output.selected, 0);
        assert_eq!(output.candidate_count, 2);
        // SAFETY: successful current call returned a non-null candidate pointer valid for model lifetime.
        let first = unsafe { &*output.candidates };
        assert_eq!(
            // SAFETY: candidate text pointer and length come from the valid model snapshot above.
            unsafe { std::slice::from_raw_parts(first.text.ptr, first.text.len) },
            "你".as_bytes()
        );
    }

    #[test]
    fn notification_queue_drops_stale_and_cancelled_revisions() {
        let mut model = CandidateModel::default();
        assert_eq!(model.apply(snapshot(1)), 0);
        let first = model.semantic_snapshot().expect("first snapshot");
        let mut changed = snapshot(2);
        changed.selected = Some(1);
        assert_eq!(model.apply(changed), 0);
        let second = model.semantic_snapshot().expect("second snapshot");
        let mut queue = CandidateNotificationQueue::default();
        queue.enqueue(
            &first,
            CandidateCapabilities {
                narrator_nvda: true,
                ..Default::default()
            },
            CandidatePrivacyContext::Normal,
        );
        queue.enqueue(
            &second,
            CandidateCapabilities {
                narrator_nvda: true,
                ..Default::default()
            },
            CandidatePrivacyContext::Normal,
        );
        assert!(queue.drain_for(first.identity).is_empty());
        let second_notifications = queue.drain_for(second.identity);
        assert_eq!(second_notifications.len(), 1);
        assert_eq!(
            second_notifications[0].kind,
            CandidateNotificationKind::Selection
        );
        queue.cancel(second.identity);
        assert!(queue.drain_for(second.identity).is_empty());
    }

    #[test]
    fn notification_queue_keeps_new_revision_when_context_returns_after_switch() {
        let mut model = CandidateModel::default();
        assert_eq!(model.apply(snapshot_with_identity(10, 20, 30, 1)), 0);
        let a1 = model.semantic_snapshot().expect("A1");
        assert_eq!(model.apply(snapshot_with_identity(10, 21, 40, 1)), 0);
        let b1 = model.semantic_snapshot().expect("B1");
        let mut returned = snapshot_with_identity(10, 20, 30, 2);
        returned.selected = Some(1);
        assert_eq!(model.apply(returned), 0);
        let a2 = model.semantic_snapshot().expect("A2");
        let mut queue = CandidateNotificationQueue::default();
        let caps = CandidateCapabilities::default();
        queue.enqueue(&a1, caps, CandidatePrivacyContext::Normal);
        queue.enqueue(&b1, caps, CandidatePrivacyContext::Normal);
        queue.enqueue(&a2, caps, CandidatePrivacyContext::Normal);
        assert_eq!(queue.drain_for(a1.identity).len(), 0);
        assert_eq!(queue.drain_for(b1.identity).len(), 1);
        assert_eq!(queue.drain_for(a2.identity).len(), 1);
    }

    #[test]
    fn notification_text_is_suppressed_for_sensitive_contexts() {
        let mut model = CandidateModel::default();
        assert_eq!(model.apply(snapshot(1)), 0);
        let semantic = model.semantic_snapshot().expect("semantic snapshot");
        let mut queue = CandidateNotificationQueue::default();
        queue.enqueue(
            &semantic,
            CandidateCapabilities {
                narrator_nvda: true,
                ..Default::default()
            },
            CandidatePrivacyContext::Password,
        );
        let notification = queue
            .drain_for(semantic.identity)
            .pop()
            .expect("notification");
        assert!(notification.text.is_none());
        assert!(CandidatePrivacyContext::Pin.suppress_text());
        assert!(CandidatePrivacyContext::Sensitive.suppress_text());
        assert_eq!(
            CandidatePrivacyContext::Password.policy(),
            CandidatePrivacyPolicy {
                allow_speech: false,
                allow_text_logging: false,
                allow_learning: false,
                allow_network: false,
            }
        );
        let policy = CandidatePrivacyContext::Sensitive.policy();
        assert!(!policy.allows_speech());
        assert!(!policy.allows_text_logging());
        assert!(!policy.allows_learning());
        assert!(!policy.allows_network());
    }

    #[test]
    fn notification_queue_emits_only_changed_selection_count_and_state() {
        let mut model = CandidateModel::default();
        assert_eq!(model.apply(snapshot(1)), 0);
        let first = model.semantic_snapshot().expect("first snapshot");
        let mut queue = CandidateNotificationQueue::default();
        let capabilities = CandidateCapabilities {
            narrator_nvda: true,
            ..Default::default()
        };
        queue.enqueue(&first, capabilities, CandidatePrivacyContext::Normal);
        assert_eq!(
            queue.drain_for(first.identity)[0].kind,
            CandidateNotificationKind::Snapshot
        );

        let mut changed = first.clone();
        changed.identity.revision = 2;
        changed.selected = Some(1);
        changed.total += 1;
        changed.visibility = 2;
        queue.enqueue(&changed, capabilities, CandidatePrivacyContext::Normal);
        let notifications = queue.drain_for(changed.identity);
        assert_eq!(notifications.len(), 3);
        assert_eq!(notifications[0].kind, CandidateNotificationKind::Selection);
        assert_eq!(notifications[1].kind, CandidateNotificationKind::Count);
        assert_eq!(notifications[2].kind, CandidateNotificationKind::State);
        assert_eq!(notifications[0].text.as_deref(), Some("呢"));
        assert!(notifications[1].text.is_none());
        assert!(notifications[2].text.is_none());

        let mut unchanged = changed.clone();
        unchanged.identity.revision = 3;
        queue.enqueue(&unchanged, capabilities, CandidatePrivacyContext::Normal);
        assert!(queue.drain_for(unchanged.identity).is_empty());
    }

    #[test]
    fn capability_flags_compose_without_a_disability_mode() {
        let capabilities = CandidateCapabilities {
            keyboard: true,
            uia: true,
            narrator_nvda: true,
            high_contrast: true,
            large_text: true,
            reduced_motion: true,
            reduced_candidates: true,
            stable_layout: true,
        };
        assert!(capabilities.keyboard && capabilities.uia && capabilities.narrator_nvda);
        assert!(
            capabilities.high_contrast && capabilities.large_text && capabilities.reduced_motion
        );
        assert!(capabilities.reduced_candidates && capabilities.stable_layout);
    }

    fn width(result: &LayoutResult) -> f32 {
        result.window.right - result.window.left
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().collect()
    }

    #[test]
    fn candidate_ui_command_line_matches_cpp_contract() {
        let arguments =
            wide("--generation nightly --self-test --safe-mode --parent-pid 42 --reload-test");
        let (parsed, generation, peer) = parse_candidate_command_line(&arguments);
        assert_eq!(parsed.status, 1);
        assert_eq!(generation, wide("nightly"));
        assert!(peer.is_empty());
        assert_eq!(parsed.self_test, 1);
        assert_eq!(parsed.safe_mode, 1);
        assert_eq!(parsed.reload_test, 1);
        assert_eq!(parsed.has_parent_id, 1);
        assert_eq!(parsed.parent_id, 42);

        let demo = parse_candidate_command_line(&wide("--locale-self-test --scroll-demo")).0;
        assert_eq!(demo.locale_self_test, 1);
        assert_eq!(demo.scroll_demo, 1);
        assert_eq!(demo.demo, 1);

        let (select, _, peer) = parse_candidate_command_line(&wide(
            r#"--candidate-select-test "C:\Program Files\fcitx5-engine.exe" 12 20 30 40 50 60"#,
        ));
        assert_eq!(select.candidate_select_mode, 1);
        assert_eq!(peer, wide(r"C:\Program Files\fcitx5-engine.exe"));
        assert_eq!(select.target_process_id, 12);
        assert_eq!(select.engine_epoch, 20);
        assert_eq!(select.context_id, 30);
        assert_eq!(select.composition_id, 40);
        assert_eq!(select.revision, 50);
        assert_eq!(select.candidate_id, 60);

        assert_eq!(
            parse_candidate_command_line(&wide("--candidate-select-test only-peer"))
                .0
                .candidate_select_mode,
            64
        );
        assert_eq!(
            parse_candidate_command_line(&wide("--candidate-select-test peer nope 2 3 4 5 6"))
                .0
                .candidate_select_mode,
            65
        );
    }

    #[test]
    fn default_dwrite_locale_matches_cpp_fallback_contract() {
        let locale = default_dwrite_locale();
        assert!(!locale.is_empty());
        assert!(locale.len() < LOCALE_NAME_MAX_LENGTH);
        assert!(!locale.contains(&0));
    }

    #[test]
    fn default_dwrite_locale_abi_uses_two_phase_utf16_output() {
        // SAFETY: null output with zero capacity is the documented size-query form.
        let required =
            unsafe { fcitx5_candidate_default_dwrite_locale_utf16(std::ptr::null_mut(), 0) };
        assert!(required > 0);
        let mut output = vec![0_u16; required];
        // SAFETY: output owns writable storage for exactly its supplied u16 capacity.
        let written = unsafe {
            fcitx5_candidate_default_dwrite_locale_utf16(output.as_mut_ptr(), output.len())
        };
        assert_eq!(written, required);
        assert!(!output.contains(&0));
    }

    #[test]
    fn content_locale_policy_matches_cpp_contract() {
        assert!(content_locale_valid(b"zh-CN"));
        assert!(content_locale_valid(b"en-US"));
        assert!(content_locale_valid(b"ja-JP-u-ca-japanese"));
        assert!(!content_locale_valid(b""));
        assert!(!content_locale_valid(b"----"));
        assert!(!content_locale_valid(b"zh_CN"));
        assert!(!content_locale_valid(&[b'a'; MAX_CONTENT_LOCALE_UTF8 + 1]));
        assert_eq!(content_locale_or_default(b"zh-CN"), wide("zh-CN"));
        assert_eq!(
            // SAFETY: UTF-8 view points to the static byte literal for its exact length.
            unsafe {
                fcitx5_candidate_content_locale_valid_utf8(Fcitx5CandidateUtf8 {
                    ptr: b"ko-KR".as_ptr(),
                    len: b"ko-KR".len(),
                })
            },
            1
        );
        assert_eq!(
            // SAFETY: UTF-8 view points to the static byte literal for its exact length.
            unsafe {
                fcitx5_candidate_locale_prefers_compact_horizontal_utf8(Fcitx5CandidateUtf8 {
                    ptr: b"JA-jp".as_ptr(),
                    len: b"JA-jp".len(),
                })
            },
            1
        );
    }

    #[test]
    fn content_locale_or_default_abi_uses_two_phase_utf16_output() {
        let locale = Fcitx5CandidateUtf8 {
            ptr: b"en-US".as_ptr(),
            len: b"en-US".len(),
        };
        // SAFETY: locale view references the static byte literal and null output is a size query.
        let required = unsafe {
            fcitx5_candidate_content_locale_or_default_utf16(locale, std::ptr::null_mut(), 0)
        };
        assert_eq!(required, wide("en-US").len());
        let mut output = vec![0_u16; required];
        // SAFETY: locale view is valid and output owns writable storage for its supplied capacity.
        let written = unsafe {
            fcitx5_candidate_content_locale_or_default_utf16(
                locale,
                output.as_mut_ptr(),
                output.len(),
            )
        };
        assert_eq!(written, required);
        assert_eq!(output, wide("en-US"));

        let invalid = Fcitx5CandidateUtf8 {
            ptr: b"zh_CN".as_ptr(),
            len: b"zh_CN".len(),
        };
        // SAFETY: invalid locale bytes remain valid readable storage for the supplied length.
        let fallback_required = unsafe {
            fcitx5_candidate_content_locale_or_default_utf16(invalid, std::ptr::null_mut(), 0)
        };
        assert!(fallback_required > 0);
    }

    #[test]
    fn scroll_label_policy_reserves_and_shows_current_row_or_column() {
        let inactive = scroll_label_policy(2, 8, 6, 20);
        assert_eq!(inactive.reserve, 1);
        assert_eq!(inactive.show, 0);
        assert_eq!(inactive.slot, 3);

        let active = scroll_label_policy(10, 8, 6, 20);
        assert_eq!(active.reserve, 1);
        assert_eq!(active.show, 1);
        assert_eq!(active.slot, 5);

        assert_eq!(scroll_label_policy(0, 0, 6, 6).reserve, 0);
        assert_eq!(scroll_label_policy(0, 0, 0, 20).reserve, 0);
    }

    #[test]
    fn candidate_label_formatting_supports_custom_ordinals() {
        assert_eq!(
            format_candidate_label(1, "", CandidateLabelStyle::Dot, "", ""),
            "1."
        );
        assert_eq!(
            format_candidate_label(10, "", CandidateLabelStyle::Dot, "", ""),
            "10."
        );
        assert_eq!(
            format_candidate_label(10, "", CandidateLabelStyle::Dot, "#", ":"),
            "#10:"
        );
        assert_eq!(
            format_candidate_label(3, "abc", CandidateLabelStyle::Bracket, "", ""),
            "[abc]"
        );
        assert_eq!(
            format_candidate_label(1, "1", CandidateLabelStyle::Circled, "", ""),
            "①"
        );
    }

    #[test]
    fn candidate_label_slot_plan_keeps_hidden_labels_aligned() {
        let sources = [
            CandidateLabelSlotSource {
                candidate_index: 0,
                row: 0,
                column: 0,
                label_width: 12.0,
            },
            CandidateLabelSlotSource {
                candidate_index: 1,
                row: 1,
                column: 0,
                label_width: 28.0,
            },
            CandidateLabelSlotSource {
                candidate_index: 2,
                row: 2,
                column: 0,
                label_width: 20.0,
            },
        ];
        let plan = candidate_label_slot_plan(
            CandidateLabelSlotConfig {
                display: CandidateLabelDisplay::SelectedScope,
                scope: CandidateLabelScope::Item,
                reserve_when_hidden: true,
                align: CandidateLabelAlign::Right,
                width_strategy: CandidateLabelWidthStrategy::PageMax,
                min_width: 0.0,
                gap: 4.0,
            },
            &sources,
            1,
        );
        assert_eq!(plan.label_slot_width, 28.0);
        assert!(plan.stable_text_origin);
        assert_eq!(
            plan.items
                .iter()
                .map(|item| item.show_label)
                .collect::<Vec<_>>(),
            vec![false, true, false]
        );
        assert!(plan.items.iter().all(|item| item.reserve_label));
        assert!(plan
            .items
            .iter()
            .all(|item| (item.text_origin_offset - 32.0).abs() <= f32::EPSILON));
    }

    #[test]
    fn candidate_label_slot_plan_reveals_selected_rows_and_columns() {
        let sources = [
            CandidateLabelSlotSource {
                candidate_index: 0,
                row: 0,
                column: 0,
                label_width: 10.0,
            },
            CandidateLabelSlotSource {
                candidate_index: 1,
                row: 0,
                column: 1,
                label_width: 10.0,
            },
            CandidateLabelSlotSource {
                candidate_index: 2,
                row: 1,
                column: 0,
                label_width: 24.0,
            },
            CandidateLabelSlotSource {
                candidate_index: 3,
                row: 1,
                column: 1,
                label_width: 24.0,
            },
        ];
        let row_plan = candidate_label_slot_plan(
            CandidateLabelSlotConfig {
                display: CandidateLabelDisplay::SelectedScope,
                scope: CandidateLabelScope::Row,
                ..CandidateLabelSlotConfig::default()
            },
            &sources,
            2,
        );
        assert_eq!(
            row_plan
                .items
                .iter()
                .map(|item| item.show_label)
                .collect::<Vec<_>>(),
            vec![false, false, true, true]
        );
        let column_plan = candidate_label_slot_plan(
            CandidateLabelSlotConfig {
                display: CandidateLabelDisplay::SelectedScope,
                scope: CandidateLabelScope::Column,
                ..CandidateLabelSlotConfig::default()
            },
            &sources,
            1,
        );
        assert_eq!(
            column_plan
                .items
                .iter()
                .map(|item| item.show_label)
                .collect::<Vec<_>>(),
            vec![false, true, false, true]
        );
        assert!(row_plan.stable_text_origin);
        assert!(column_plan.stable_text_origin);
    }

    #[test]
    fn interaction_helpers_match_cpp_contract() {
        let rectangles = [
            Fcitx5CandidateLayoutRect {
                left: 8.0,
                top: 8.0,
                right: 120.0,
                bottom: 36.0,
            },
            Fcitx5CandidateLayoutRect {
                left: 8.0,
                top: 38.0,
                right: 120.0,
                bottom: 66.0,
            },
        ];
        let mut index = usize::MAX;
        assert_eq!(
            // SAFETY: rectangles is a live contiguous slice and index is writable test storage.
            unsafe {
                fcitx5_candidate_hit_test(
                    rectangles.as_ptr(),
                    rectangles.len(),
                    20.0,
                    50.0,
                    &mut index,
                )
            },
            1
        );
        assert_eq!(index, 1);
        assert_eq!(
            // SAFETY: rectangles is a live contiguous slice and index is writable test storage.
            unsafe {
                fcitx5_candidate_hit_test(
                    rectangles.as_ptr(),
                    rectangles.len(),
                    4.0,
                    50.0,
                    &mut index,
                )
            },
            0
        );
        assert_eq!(
            // SAFETY: rectangles is a live contiguous slice and index is writable test storage.
            unsafe {
                fcitx5_candidate_hit_test(
                    rectangles.as_ptr(),
                    rectangles.len(),
                    20.0,
                    37.0,
                    &mut index,
                )
            },
            0
        );

        let intent = fcitx5_candidate_selection_intent(41, 9, 10, 11, 12, 13);
        assert!(selection_intent_valid(intent));
        assert_eq!(intent.target_process_id, 41);
        assert_eq!(intent.engine_epoch, 9);
        assert_eq!(intent.context_id, 10);
        assert_eq!(intent.composition_id, 11);
        assert_eq!(intent.revision, 12);
        assert_eq!(intent.candidate_id, 13);
        assert!(!selection_intent_valid(fcitx5_candidate_selection_intent(
            0, 9, 10, 11, 12, 13
        )));
        assert!(!selection_intent_valid(fcitx5_candidate_selection_intent(
            41, 0, 10, 11, 12, 13
        )));
        assert!(!selection_intent_valid(fcitx5_candidate_selection_intent(
            41, 9, 10, 0, 12, 13
        )));
        assert!(!selection_intent_valid(fcitx5_candidate_selection_intent(
            41, 9, 10, 11, 12, 0
        )));
    }

    #[test]
    fn render_segments_match_label_column_gap_and_comment_contract() {
        let input = [
            Fcitx5CandidateRenderItemInput {
                bounds: Fcitx5CandidateLayoutRect {
                    left: 10.0,
                    top: 20.0,
                    right: 210.0,
                    bottom: 48.0,
                },
                label_width: 18.0,
                label_gap: 4.0,
                text_width: 80.0,
                comment_width: 40.0,
                has_label: 1,
                reserve_label: 1,
            },
            Fcitx5CandidateRenderItemInput {
                bounds: Fcitx5CandidateLayoutRect {
                    left: 10.0,
                    top: 52.0,
                    right: 110.0,
                    bottom: 80.0,
                },
                label_width: 10.0,
                label_gap: 4.0,
                text_width: 96.0,
                comment_width: 20.0,
                has_label: 1,
                reserve_label: 1,
            },
        ];
        let mut output = [Fcitx5CandidateRenderItemOutput::default(); 2];
        let mut label_column = 0.0_f32;
        assert_eq!(
            // SAFETY: input and output arrays are live contiguous test storage for their supplied lengths.
            unsafe {
                fcitx5_candidate_render_segments(
                    input.as_ptr(),
                    input.len(),
                    1,
                    0,
                    output.as_mut_ptr(),
                    &mut label_column,
                )
            },
            0
        );
        assert_eq!(label_column, 20.0);
        assert_eq!(output[0].label.right, 30.0);
        assert_eq!(output[0].text.left, 34.0);
        assert_eq!(output[0].comment.left, 118.0);
        assert_eq!(output[0].draw_comment, 1);
        assert_eq!(output[1].label.right, 30.0);
        assert_eq!(output[1].text.left, 34.0);
        assert_eq!(output[1].comment.left, 114.0);
        assert_eq!(output[1].draw_comment, 0);

        let no_label = [Fcitx5CandidateRenderItemInput {
            bounds: input[0].bounds,
            label_width: 18.0,
            label_gap: 4.0,
            text_width: 80.0,
            comment_width: 0.0,
            has_label: 0,
            reserve_label: 0,
        }];
        let mut no_label_output = [Fcitx5CandidateRenderItemOutput::default(); 1];
        assert_eq!(
            // SAFETY: input and output arrays are live contiguous test storage for their supplied lengths.
            unsafe {
                fcitx5_candidate_render_segments(
                    no_label.as_ptr(),
                    no_label.len(),
                    0,
                    1,
                    no_label_output.as_mut_ptr(),
                    &mut label_column,
                )
            },
            0
        );
        assert_eq!(label_column, 0.0);
        assert_eq!(no_label_output[0].text.left, 10.0);

        let hidden_label = [Fcitx5CandidateRenderItemInput {
            bounds: input[0].bounds,
            label_width: 18.0,
            label_gap: 4.0,
            text_width: 80.0,
            comment_width: 0.0,
            has_label: 0,
            reserve_label: 1,
        }];
        let mut hidden_output = [Fcitx5CandidateRenderItemOutput::default(); 1];
        assert_eq!(
            // SAFETY: input and output arrays are live contiguous test storage for their supplied lengths.
            unsafe {
                fcitx5_candidate_render_segments(
                    hidden_label.as_ptr(),
                    hidden_label.len(),
                    1,
                    0,
                    hidden_output.as_mut_ptr(),
                    &mut label_column,
                )
            },
            0
        );
        assert_eq!(label_column, 20.0);
        assert_eq!(hidden_output[0].label.right, 30.0);
        assert_eq!(hidden_output[0].text.left, 34.0);
    }

    #[test]
    fn candidate_model_matches_frozen_cpp_contract() {
        let mut model = CandidateModel::default();
        assert_eq!(model.apply(snapshot(1)), 0);
        assert_eq!(model.apply(snapshot(1)), 1);
        assert_eq!(model.apply(snapshot(0)), 3);
        assert_eq!(model.apply(snapshot(2)), 0);

        let mut uiless = snapshot(3);
        uiless.popup_allowed = false;
        uiless.selected = Some(1);
        assert_eq!(model.apply(uiless), 0);
        assert!(!model.current.as_ref().expect("current").popup_allowed);
        assert_eq!(model.current.as_ref().expect("current").candidates.len(), 2);
        assert_eq!(model.current.as_ref().expect("current").selected, Some(1));

        let mut stale = snapshot(4);
        stale.engine_epoch = 9;
        assert_eq!(model.apply(stale), 2);

        let mut prediction = snapshot(4);
        prediction.preedit.clear();
        prediction.visibility = Visibility::Prediction;
        assert_eq!(model.apply(prediction), 0);

        let mut invalid = snapshot(5);
        invalid.selected = Some(5);
        assert_eq!(model.apply(invalid), 3);

        let mut switched = snapshot(1);
        switched.context_id = 21;
        switched.composition_id = 40;
        assert_eq!(model.apply(switched), 0);
        assert_eq!(model.current.as_ref().expect("current").context_id, 21);
        assert!(model.current.as_ref().expect("current").popup_allowed);

        let mut returned = snapshot(5);
        returned.context_id = 20;
        returned.composition_id = 30;
        returned.popup_allowed = false;
        assert_eq!(model.apply(returned), 0);
        assert_eq!(model.current.as_ref().expect("current").context_id, 20);
        assert_eq!(model.current.as_ref().expect("current").revision, 5);
        assert!(!model.current.as_ref().expect("current").popup_allowed);

        let mut duplicate_inactive_a = snapshot_with_identity(10, 20, 30, 5);
        duplicate_inactive_a.popup_allowed = false;
        assert_eq!(model.apply(duplicate_inactive_a), 1);

        let mut newer_a = snapshot_with_identity(10, 20, 30, 6);
        newer_a.selected = Some(1);
        newer_a.popup_allowed = false;
        assert_eq!(model.apply(newer_a), 0);
        assert_eq!(model.current.as_ref().expect("current").context_id, 20);
        assert_eq!(model.current.as_ref().expect("current").revision, 6);
        assert_eq!(model.current.as_ref().expect("current").selected, Some(1));
        assert!(!model.current.as_ref().expect("current").popup_allowed);

        assert_eq!(model.apply(snapshot_with_identity(10, 20, 30, 5)), 2);
        assert_eq!(model.current.as_ref().expect("current").revision, 6);

        let mut smaller_b = snapshot_with_identity(10, 22, 2, 1);
        smaller_b.preedit = b"bo".to_vec();
        smaller_b.candidates = vec![CandidateItem {
            id: 3,
            label: b"1".to_vec(),
            text: b"bo".to_vec(),
            comment: Vec::new(),
        }];
        smaller_b.total = 1;
        assert_eq!(model.apply(smaller_b), 0);
        assert_eq!(model.current.as_ref().expect("current").context_id, 22);
        assert_eq!(model.current.as_ref().expect("current").composition_id, 2);
        assert_eq!(model.current.as_ref().expect("current").candidates.len(), 1);

        let mut final_a = snapshot_with_identity(10, 20, 30, 7);
        final_a.auxiliary_down = b"visible".to_vec();
        assert_eq!(model.apply(final_a), 0);
        assert_eq!(
            model.current.as_ref().expect("current").auxiliary_down,
            b"visible"
        );

        assert_eq!(model.apply(snapshot_with_identity(9, 20, 30, 6)), 2);
        assert_eq!(model.current.as_ref().expect("current").engine_epoch, 10);

        let mut new_composition = snapshot_with_identity(10, 20, 31, 1);
        new_composition.preedit = b"xin".to_vec();
        assert_eq!(model.apply(new_composition), 0);
        assert_eq!(model.current.as_ref().expect("current").composition_id, 31);
        assert_eq!(model.current.as_ref().expect("current").revision, 1);
        assert_eq!(model.apply(snapshot_with_identity(10, 20, 30, 6)), 2);
        assert_eq!(model.current.as_ref().expect("current").composition_id, 31);

        let mut preedit_only = snapshot(2);
        preedit_only.context_id = 23;
        preedit_only.candidates.clear();
        preedit_only.selected = None;
        preedit_only.total = 0;
        preedit_only.visibility = Visibility::Hidden;
        assert_eq!(model.apply(preedit_only), 0);

        model.reset();
        assert!(model.current.is_none());

        let mut reconnected = snapshot(1);
        reconnected.popup_allowed = false;
        assert_eq!(model.apply(reconnected), 0);
        assert!(!model.current.as_ref().expect("current").popup_allowed);
        assert_eq!(model.current.as_ref().expect("current").candidates.len(), 2);
    }

    #[test]
    fn layout_matches_frozen_cpp_contract() {
        let mut input = LayoutInput {
            orientation: Orientation::Vertical,
            items: vec![
                Size {
                    width: 100.0,
                    height: 24.0,
                },
                Size {
                    width: 140.0,
                    height: 24.0,
                },
            ],
            caret: Point {
                x: 1900.0,
                y: 1060.0,
            },
            caret_height: 20.0,
            work_area: Rect {
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
            placement: Placement::Unlocked,
            ..LayoutInput::default()
        };
        let first = layout(&input);
        assert_eq!(first.placement, Placement::Above);
        assert!(first.window.right <= 1920.0);
        assert!(first.window.bottom <= 1080.0);
        assert_eq!(first.items.len(), 2);

        input.placement = first.placement;
        input.items[1].width = 300.0;
        let stable = layout(&input);
        assert_eq!(stable.placement, Placement::Above);

        input.orientation = Orientation::Horizontal;
        input.placement = Placement::Below;
        input.caret = Point { x: 10.0, y: 10.0 };
        let horizontal = layout(&input);
        assert!(horizontal.items[1].left > horizontal.items[0].right);
    }

    #[test]
    fn flow_paged_bounds_wraps_into_rows_without_overlap() {
        let sizes = vec![
            Size {
                width: 100.0,
                height: 24.0
            };
            5
        ];
        let (rects, width, height) = flow_paged_bounds(&sizes, 250.0, 10.0, 2.0);
        // Row capacity: 100+10+100=210<=250, +10+100=320>250 => 2 per row => 2,2,1 rows.
        assert_eq!(rects.len(), 5);
        assert_eq!(width, 210.0);
        assert_eq!(height, 24.0 * 3.0 + 2.0 * 2.0);
        assert_eq!(rects[0].top, rects[1].top);
        assert!(rects[1].left > rects[0].right);
        assert!(rects[2].top >= rects[1].bottom + 2.0);
        assert!(rects[3].top == rects[2].top);
    }

    #[test]
    fn flow_paged_bounds_scales_with_dpi_item_sizes() {
        // 200% DPI doubles every item; the wrap geometry stays non-overlapping
        // and keeps full-item bounds.
        let sizes = vec![
            Size {
                width: 200.0,
                height: 48.0
            };
            6
        ];
        let (rects, width, height) = flow_paged_bounds(&sizes, 500.0, 20.0, 4.0);
        assert_eq!(rects.len(), 6);
        assert!(width <= 500.0);
        assert!(height > 48.0);
        for window in rects.windows(2) {
            if (window[1].top - window[0].top).abs() < f32::EPSILON {
                assert!(window[1].left >= window[0].right);
            } else {
                assert!(window[1].top >= window[0].bottom);
            }
        }
    }

    #[test]
    fn vertical_text_columns_left_to_right_keep_full_glyph_budget() {
        // 150% DPI: full CJK advance 36.0 * 1.5 = 54.0 column budget.
        let sizes = vec![
            Size {
                width: 54.0,
                height: 96.0
            };
            3
        ];
        let (rects, width, height) = vertical_text_columns(&sizes, 12.0, true);
        assert_eq!(rects.len(), 3);
        assert_eq!(width, 54.0 * 3.0 + 12.0 * 2.0);
        assert_eq!(height, 96.0);
        assert!(rects[1].left >= rects[0].right);
        assert!(rects[2].left >= rects[1].right);
        assert_eq!(rects[0].top, 0.0);
        // Each column keeps its full-glyph advance budget (no truncation).
        for rect in &rects {
            assert_eq!(rect.right - rect.left, 54.0);
        }
    }

    #[test]
    fn vertical_text_columns_right_to_left_reverse_order() {
        let sizes = vec![
            Size {
                width: 40.0,
                height: 80.0,
            },
            Size {
                width: 40.0,
                height: 80.0,
            },
        ];
        let (rects, width, _) = vertical_text_columns(&sizes, 10.0, false);
        assert_eq!(rects.len(), 2);
        assert_eq!(width, 90.0);
        assert!(
            rects[0].left > rects[1].left,
            "right-to-left puts first candidate on the right"
        );
        assert_eq!(rects[1].left, 0.0);
        assert!(rects[0].left >= rects[1].right);
    }

    #[test]
    fn scaled_negative_monitor_is_clamped() {
        for scale in [1.25_f32, 1.5, 2.0] {
            let input = LayoutInput {
                orientation: Orientation::Vertical,
                caret: Point {
                    x: -1900.0,
                    y: 900.0,
                },
                caret_height: 20.0 * scale,
                work_area: Rect {
                    left: -1920.0,
                    top: 0.0,
                    right: 0.0,
                    bottom: 1080.0,
                },
                max_width: 720.0 * scale,
                padding_x: 8.0 * scale,
                padding_y: 6.0 * scale,
                items: vec![
                    Size {
                        width: 500.0 * scale,
                        height: 24.0 * scale,
                    },
                    Size {
                        width: 900.0 * scale,
                        height: 24.0 * scale,
                    },
                ],
                ..LayoutInput::default()
            };
            let scaled = layout(&input);
            assert!(scaled.window.left >= input.work_area.left);
            assert!(scaled.window.right <= input.work_area.right);
            assert!(scaled.window.top >= input.work_area.top);
            assert!(scaled.window.bottom <= input.work_area.bottom);
        }
    }

    #[test]
    fn horizontal_scroll_viewport_and_width_match_cpp_contract() {
        let mut scroll = LayoutInput {
            scroll_mode: true,
            scroll_columns: 6,
            scroll_visible_rows: 6,
            caret: Point { x: 100.0, y: 100.0 },
            caret_height: 24.0,
            work_area: Rect {
                left: 0.0,
                top: 0.0,
                right: 1920.0,
                bottom: 1080.0,
            },
            max_width: 860.0,
            items: vec![
                Size {
                    width: 120.0,
                    height: 34.0,
                };
                60
            ],
            orientation: Orientation::Horizontal,
            selected: 6,
            ..LayoutInput::default()
        };
        let same_viewport = layout(&scroll);
        assert_eq!(same_viewport.first_visible, 0);
        assert_eq!(same_viewport.item_indices.first(), Some(&0));
        assert_eq!(same_viewport.item_indices.last(), Some(&35));

        let mut horizontal_baseline = scroll.clone();
        horizontal_baseline.scroll_mode = false;
        horizontal_baseline.items.truncate(6);
        let horizontal_baseline_layout = layout(&horizontal_baseline);
        assert!((width(&same_viewport) - width(&horizontal_baseline_layout)).abs() <= 0.01);

        scroll.selected = 42;
        let next_viewport = layout(&scroll);
        assert_eq!(next_viewport.items.len(), 36);
        assert_eq!(next_viewport.item_indices.first(), Some(&24));
        assert_eq!(next_viewport.item_indices.last(), Some(&59));
        assert!(next_viewport.has_scrollbar);
        assert!(next_viewport.scrollbar_thumb.bottom <= next_viewport.scrollbar_track.bottom);
    }

    #[test]
    fn horizontal_scroll_grid_gives_each_column_its_widest_candidate() {
        let widths = [200.0, 40.0, 40.0, 60.0, 40.0, 40.0, 50.0, 120.0, 40.0];
        let input = LayoutInput {
            scroll_mode: true,
            scroll_columns: 3,
            scroll_visible_rows: 6,
            caret: Point { x: 100.0, y: 100.0 },
            caret_height: 24.0,
            work_area: Rect {
                left: 0.0,
                top: 0.0,
                right: 1920.0,
                bottom: 1080.0,
            },
            max_width: 860.0,
            items: widths
                .iter()
                .map(|width| Size {
                    width: *width,
                    height: 34.0,
                })
                .collect(),
            orientation: Orientation::Horizontal,
            ..LayoutInput::default()
        };
        let grid = layout(&input);
        assert_eq!(grid.items.len(), 9);
        for (rect, natural) in grid.items.iter().zip(widths) {
            assert!(rect.right - rect.left >= natural - 0.01);
        }
        for row in grid.items.chunks(3) {
            for pair in row.windows(2) {
                assert!(pair[0].right + input.column_gap <= pair[1].left + 0.01);
            }
        }
        for column in 0..3 {
            let left = grid.items[column].left;
            assert!(grid
                .items
                .iter()
                .skip(column)
                .step_by(3)
                .all(|r| (r.left - left).abs() < 0.01));
        }
        let expected = 200.0 + 120.0 + 40.0 + input.column_gap * 2.0 + input.padding_x * 2.0;
        assert!((width(&grid) - expected).abs() <= 0.01);
    }

    #[test]
    fn horizontal_scroll_grid_shrinks_widest_columns_first() {
        let input = LayoutInput {
            scroll_mode: true,
            scroll_columns: 3,
            scroll_visible_rows: 6,
            work_area: Rect {
                left: 0.0,
                top: 0.0,
                right: 1920.0,
                bottom: 1080.0,
            },
            max_width: 300.0,
            items: [500.0, 40.0, 60.0]
                .iter()
                .map(|width| Size {
                    width: *width,
                    height: 34.0,
                })
                .collect(),
            orientation: Orientation::Horizontal,
            ..LayoutInput::default()
        };
        let grid = layout(&input);
        assert!(width(&grid) <= 300.0 + 0.01);
        assert!((grid.items[1].right - grid.items[1].left - 40.0).abs() < 0.01);
        assert!((grid.items[2].right - grid.items[2].left - 60.0).abs() < 0.01);
        assert!(grid.items[2].right <= grid.window.right - input.padding_x + 0.01);
    }

    #[test]
    fn vertical_scroll_uses_bounded_natural_columns() {
        let mut vertical_scroll = LayoutInput {
            scroll_mode: true,
            scroll_columns: 6,
            scroll_visible_rows: 6,
            caret: Point { x: 100.0, y: 100.0 },
            caret_height: 24.0,
            work_area: Rect {
                left: 0.0,
                top: 0.0,
                right: 1920.0,
                bottom: 1080.0,
            },
            max_width: 720.0,
            scroll_cell_width: 96.0,
            orientation: Orientation::Vertical,
            selected: 6,
            items: vec![
                Size {
                    width: 48.0,
                    height: 34.0,
                };
                60
            ],
            ..LayoutInput::default()
        };
        let first_columns = layout(&vertical_scroll);
        let fixed_cell_width = 96.0 * 6.0 + 8.0 * 2.0 + 8.0 * 5.0;
        assert_eq!(first_columns.items.len(), 36);
        assert_eq!(first_columns.item_indices.first(), Some(&0));
        assert_eq!(first_columns.item_indices.last(), Some(&35));
        assert!(width(&first_columns) < fixed_cell_width - 0.01);
        for item in &first_columns.items {
            assert!(((item.right - item.left) - 48.0).abs() <= 0.01);
        }

        vertical_scroll.items = vec![
            Size {
                width: 420.0,
                height: 34.0,
            };
            60
        ];
        let long_candidate_columns = layout(&vertical_scroll);
        assert_eq!(long_candidate_columns.items.len(), 36);
        assert!(width(&long_candidate_columns) <= vertical_scroll.max_width + 0.01);

        vertical_scroll.selected = 58;
        let final_column = layout(&vertical_scroll);
        assert_eq!(final_column.items.len(), 36);
        assert_eq!(final_column.item_indices.first(), Some(&24));
        assert_eq!(final_column.item_indices.last(), Some(&59));
    }

    #[test]
    fn automatic_orientation_and_width_stability_are_composition_scoped() {
        let mut state = CompositionLayoutState::default();
        let identity = CompositionIdentity {
            engine_epoch: 1,
            context_id: 2,
            composition_id: 3,
        };
        state.begin(identity);
        let work_area = Rect {
            left: 0.0,
            top: 0.0,
            right: 1920.0,
            bottom: 1080.0,
        };
        let compact = vec![
            CandidateText {
                text: "你".into(),
                comment: String::new(),
            },
            CandidateText {
                text: "好".into(),
                comment: String::new(),
            },
        ];
        assert_eq!(
            state.resolve_orientation(
                PresentationOrientation::Automatic,
                AutomaticOrientationInput {
                    candidates: &compact,
                    locale: "zh-CN",
                    work_area,
                    caret_x: 100.0,
                    scale: 1.0,
                    page_size: 9,
                },
            ),
            Orientation::Horizontal
        );
        let annotated = vec![CandidateText {
            text: "你".into(),
            comment: "annotation-longer-than-limit".into(),
        }];
        assert_eq!(
            state.resolve_orientation(
                PresentationOrientation::Automatic,
                AutomaticOrientationInput {
                    candidates: &annotated,
                    locale: "zh-CN",
                    work_area,
                    caret_x: 100.0,
                    scale: 1.0,
                    page_size: 9,
                },
            ),
            Orientation::Horizontal,
            "orientation remains stable inside one composition"
        );
        assert_eq!(
            state.resolve_orientation(
                PresentationOrientation::Horizontal,
                AutomaticOrientationInput {
                    candidates: &annotated,
                    locale: "en-US",
                    work_area,
                    caret_x: 1910.0,
                    scale: 1.0,
                    page_size: 9,
                },
            ),
            Orientation::Horizontal
        );
        assert_eq!(
            state.resolve_orientation(
                PresentationOrientation::Vertical,
                AutomaticOrientationInput {
                    candidates: &compact,
                    locale: "zh-CN",
                    work_area,
                    caret_x: 100.0,
                    scale: 1.0,
                    page_size: 9,
                },
            ),
            Orientation::Vertical
        );

        state.begin(CompositionIdentity {
            engine_epoch: 1,
            context_id: 2,
            composition_id: 4,
        });
        assert_eq!(
            state.resolve_orientation(
                PresentationOrientation::Automatic,
                AutomaticOrientationInput {
                    candidates: &annotated,
                    locale: "zh-CN",
                    work_area,
                    caret_x: 100.0,
                    scale: 1.0,
                    page_size: 9,
                },
            ),
            Orientation::Vertical
        );
    }

    #[test]
    fn automatic_orientation_prefers_vertical_near_right_edge_or_non_cjk() {
        let mut state = CompositionLayoutState::default();
        let work_area = Rect {
            left: 0.0,
            top: 0.0,
            right: 1920.0,
            bottom: 1080.0,
        };
        let compact = vec![CandidateText {
            text: "你".into(),
            comment: String::new(),
        }];
        state.begin(CompositionIdentity {
            engine_epoch: 1,
            context_id: 1,
            composition_id: 1,
        });
        assert_eq!(
            state.resolve_orientation(
                PresentationOrientation::Automatic,
                AutomaticOrientationInput {
                    candidates: &compact,
                    locale: "zh-CN",
                    work_area,
                    caret_x: 1900.0,
                    scale: 1.0,
                    page_size: 9,
                },
            ),
            Orientation::Vertical
        );
        state.begin(CompositionIdentity {
            engine_epoch: 1,
            context_id: 1,
            composition_id: 2,
        });
        assert_eq!(
            state.resolve_orientation(
                PresentationOrientation::Automatic,
                AutomaticOrientationInput {
                    candidates: &compact,
                    locale: "en-US",
                    work_area,
                    caret_x: 100.0,
                    scale: 1.0,
                    page_size: 9,
                },
            ),
            Orientation::Vertical
        );
    }

    #[test]
    fn candidate_presentation_state_owns_scroll_and_composition_transitions() {
        let mut state = CandidatePresentationState::default();
        let first = CandidatePresentationUpdate {
            engine_epoch: 1,
            context_id: 2,
            composition_id: 3,
            revision: 1,
            selected: 0,
            has_selected: 1,
            candidate_count: 30,
            page: 0,
            page_size: 5,
            candidate_bulk: 1,
            configured_scroll_mode: 1,
        };

        assert_eq!(state.apply(first), 0);
        let output = state.output();
        assert_eq!(output.selected, 0);
        assert_eq!(output.has_selected, 1);
        assert_eq!(output.scroll_mode, 0);
        assert_eq!(output.scroll_columns, 5);
        assert_eq!(output.ordinary_start, 0);
        assert_eq!(output.ordinary_count, 5);

        assert_eq!(state.apply(first), 1);
        let mut second = first;
        second.revision = 2;
        second.page = 1;
        second.selected = 5;
        assert_eq!(state.apply(second), 0);
        let output = state.output();
        assert_eq!(output.scroll_mode, 1);
        assert_eq!(output.scroll_expanded, 1);
        assert_eq!(output.ordinary_start, 0);
        assert_eq!(output.ordinary_count, 5);
        assert_eq!(state.apply(first), 2);

        let ended = CandidatePresentationUpdate {
            composition_id: 0,
            revision: 3,
            selected: 0,
            has_selected: 0,
            candidate_count: 0,
            page: 0,
            page_size: 0,
            candidate_bulk: 0,
            configured_scroll_mode: 1,
            ..second
        };
        assert_eq!(state.apply(ended), 0);

        state.set_placement(Placement::Above);
        assert_eq!(state.output().placement, 2);
        let mut next_composition = second;
        next_composition.composition_id = 4;
        next_composition.revision = 1;
        next_composition.page = 0;
        next_composition.selected = 0;
        assert_eq!(state.apply(next_composition), 0);
        let output = state.output();
        assert_eq!(output.scroll_mode, 0);
        assert_eq!(output.scroll_expanded, 0);
        assert_eq!(output.placement, 0);
    }

    #[test]
    fn candidate_presentation_state_tracks_bulk_page_window() {
        let mut state = CandidatePresentationState::default();
        assert_eq!(
            state.apply(CandidatePresentationUpdate {
                engine_epoch: 1,
                context_id: 2,
                composition_id: 3,
                revision: 1,
                selected: 2,
                has_selected: 1,
                candidate_count: 12,
                page: 2,
                page_size: 3,
                candidate_bulk: 1,
                configured_scroll_mode: 0,
            }),
            0
        );
        let output = state.output();
        assert_eq!(output.selected, 2);
        assert_eq!(output.ordinary_start, 6);
        assert_eq!(output.ordinary_count, 3);
        assert_eq!(output.scroll_mode, 0);
    }

    #[test]
    fn candidate_presentation_render_plan_owns_selected_and_page_indices() {
        let state = fcitx5_candidate_presentation_create();
        let update = CandidatePresentationUpdate {
            engine_epoch: 1,
            context_id: 2,
            composition_id: 3,
            revision: 1,
            selected: 2,
            has_selected: 1,
            candidate_count: 12,
            page: 2,
            page_size: 3,
            candidate_bulk: 1,
            configured_scroll_mode: 0,
        };
        assert_eq!(
            // SAFETY: state was created by the paired constructor and update is a valid live reference.
            unsafe { fcitx5_candidate_presentation_apply(state, &update) },
            0
        );

        let mut indices = [usize::MAX; 12];
        let mut plan = Fcitx5CandidatePresentationRenderPlan::default();
        assert_eq!(
            // SAFETY: state is live; indices and plan provide writable storage for their supplied bounds.
            unsafe {
                fcitx5_candidate_presentation_render_plan(
                    state,
                    indices.as_mut_ptr(),
                    indices.len(),
                    &mut plan,
                )
            },
            1
        );
        assert_eq!(plan.selected, 2);
        assert_eq!(plan.has_selected, 1);
        assert_eq!(plan.render_count, 3);
        assert_eq!(&indices[..plan.render_count], &[6, 7, 8]);

        let scroll_update = CandidatePresentationUpdate {
            revision: 2,
            selected: 5,
            page: 1,
            configured_scroll_mode: 1,
            ..update
        };
        assert_eq!(
            // SAFETY: state is live and scroll_update is a valid live reference.
            unsafe { fcitx5_candidate_presentation_apply(state, &scroll_update) },
            0
        );
        assert_eq!(
            // SAFETY: state is live; indices and plan provide writable storage for their supplied bounds.
            unsafe {
                fcitx5_candidate_presentation_render_plan(
                    state,
                    indices.as_mut_ptr(),
                    indices.len(),
                    &mut plan,
                )
            },
            1
        );
        assert_eq!(plan.selected, 5);
        assert_eq!(plan.render_count, 12);
        assert!(indices[..plan.render_count].iter().copied().eq(0..12));

        // SAFETY: state is the unique live allocation returned by the paired constructor.
        unsafe { fcitx5_candidate_presentation_destroy(state) };
    }

    #[test]
    fn candidate_presentation_orientation_fails_soft_on_invalid_utf8() {
        let state = fcitx5_candidate_presentation_create();
        let candidate = Fcitx5CandidatePresentationText {
            text: Fcitx5CandidateUtf8 {
                ptr: [0xff].as_ptr(),
                len: 1,
            },
            comment: Fcitx5CandidateUtf8::default(),
        };
        // SAFETY: state is live and candidate points to valid test storage for the supplied count.
        let orientation = unsafe {
            fcitx5_candidate_presentation_resolve_orientation(
                state,
                0,
                &candidate,
                1,
                Fcitx5CandidateUtf8::default(),
                Fcitx5CandidateLayoutRect {
                    left: 0.0,
                    top: 0.0,
                    right: 1920.0,
                    bottom: 1080.0,
                },
                100.0,
                1.0,
                9,
            )
        };
        assert_eq!(orientation, 0);
        // SAFETY: state is the unique live allocation returned by the paired constructor.
        unsafe { fcitx5_candidate_presentation_destroy(state) };
    }

    #[test]
    fn candidate_poc_self_check_covers_required_evidence() {
        let report = run_candidate_poc_self_check().expect("candidate poc self-check");
        assert!(report.contains("\"cpp_ffi\":false"));
        assert!(report.contains("\"global_hooks\":false"));
        assert!(report.contains("\"process_injection\":false"));
        assert!(report.contains("\"name\":\"vscode-uiless-accessibility\""));
        assert!(report.contains("\"popup_allowed\":false"));
        assert!(report.contains("\"name\":\"emoji-color-font\""));
        assert!(report.contains("\"color_font_candidate_present\":true"));
        assert!(report.contains("\"dpi_scale\":2.00"));
        assert!(report.contains("\"result\":\"PASS\""));
    }

    #[test]
    fn config_preview_paint_plan_preserves_labels_emoji_and_bounds() {
        let plan =
            candidate_preview_paint_plan(1.0, 596.0, 166.0).expect("candidate preview paint plan");
        assert_eq!(plan.selected_background_color, 0x0060_c107);
        assert_eq!(plan.selected_text_color, 0x00ff_ffff);
        assert_eq!(plan.items.len(), 3);
        assert!(plan.items[0].selected);
        assert!(plan.items.iter().any(|item| item.text.contains("1.")));
        assert!(plan.items.iter().any(|item| item.text.contains('你')));
        assert!(plan.items.iter().any(|item| item.text.contains('好')));
        assert!(plan.items.iter().any(|item| item.text.contains('😀')));
        let emoji_item = plan
            .items
            .iter()
            .find(|item| item.text == "3. 😀 emoji")
            .expect("emoji preview candidate");
        assert!(emoji_item.bounds.right - emoji_item.bounds.left >= 128.0);
        assert!(emoji_item.bounds.bottom - emoji_item.bounds.top >= 42.0);
        for item in &plan.items {
            assert!(rect_inside(
                item.bounds,
                Rect {
                    left: 0.0,
                    top: 0.0,
                    right: 596.0,
                    bottom: 166.0,
                }
            ));
        }
    }
}

#[cfg(test)]
mod axis_layout_ffi_tests {
    use super::*;

    fn item_size(width: f32, height: f32) -> Fcitx5CandidateLayoutSize {
        Fcitx5CandidateLayoutSize { width, height }
    }

    fn axis_input(
        orientation: u8,
        overflow: u8,
        writing: u8,
        page_size: u32,
    ) -> Fcitx5CandidateAxisLayoutInput {
        Fcitx5CandidateAxisLayoutInput {
            page_size,
            max_width: 0.0,
            max_height: 0.0,
            dpi_scale: 1.0,
            highlighted_index: 0,
            orientation,
            overflow,
            writing,
            caret: Fcitx5CandidateLayoutPoint { x: 100.0, y: 300.0 },
            caret_height: 20.0,
            work_area: Fcitx5CandidateLayoutRect {
                left: 0.0,
                top: 0.0,
                right: 800.0,
                bottom: 600.0,
            },
            padding_x: 8.0,
            padding_y: 6.0,
            row_gap: 2.0,
            column_gap: 8.0,
            placement: 0,
            scroll_override_px: -1.0,
        }
    }

    #[test]
    fn axis_layout_ffi_paging_vertical_returns_page_capped_rects() {
        let input = axis_input(1, 0, 0, 5);
        let items = [item_size(60.0, 24.0); 8];
        let mut out_items = vec![Fcitx5CandidateAxisLayoutItemOutput::default(); items.len()];
        let mut output = Fcitx5CandidateAxisLayoutOutput::default();
        assert_eq!(
            // SAFETY: input, items, output items, and output are live storage for their supplied bounds.
            unsafe {
                fcitx5_candidate_axis_layout(
                    &input,
                    items.as_ptr(),
                    items.len(),
                    out_items.as_mut_ptr(),
                    out_items.len(),
                    &mut output,
                )
            },
            0
        );
        assert_eq!(output.item_count, 5, "page size caps the laid-out page");
        assert_eq!(output.viewport_dx, 0.0);
        assert_eq!(output.viewport_dy, 0.0);
        assert!(output.window_w > 0.0 && output.window_h > 0.0);
        for (local, item) in out_items.iter().take(output.item_count).enumerate() {
            assert!(item.visible != 0);
            assert_eq!(item.w, 60.0);
            assert_eq!(item.h, 24.0);
            if local > 0 {
                assert!(out_items[local - 1].y + out_items[local - 1].h <= item.y);
            }
        }
    }

    #[test]
    fn axis_layout_ffi_rejects_invalid_dpi_and_bad_capacity() {
        let mut input = axis_input(0, 0, 0, 0);
        input.dpi_scale = 8.0;
        let items = [item_size(60.0, 24.0); 2];
        let mut out_items = vec![Fcitx5CandidateAxisLayoutItemOutput::default(); items.len()];
        let mut output = Fcitx5CandidateAxisLayoutOutput::default();
        assert_eq!(
            // SAFETY: input, items, output items, and output are live storage for their supplied bounds.
            unsafe {
                fcitx5_candidate_axis_layout(
                    &input,
                    items.as_ptr(),
                    items.len(),
                    out_items.as_mut_ptr(),
                    out_items.len(),
                    &mut output,
                )
            },
            1,
            "dpi scale outside 0.5..=4 is rejected"
        );
        let input = axis_input(0, 0, 0, 0);
        assert_eq!(
            // SAFETY: input, items, output items, and output are live storage for their supplied bounds.
            unsafe {
                fcitx5_candidate_axis_layout(
                    &input,
                    items.as_ptr(),
                    items.len(),
                    out_items.as_mut_ptr(),
                    items.len() - 1,
                    &mut output,
                )
            },
            1,
            "out_capacity below item_count is rejected"
        );
    }

    fn render_theme_input() -> Fcitx5CandidateRenderThemeInput {
        Fcitx5CandidateRenderThemeInput {
            background_r: 255,
            background_g: 255,
            background_b: 255,
            text_r: 32,
            text_g: 33,
            text_b: 36,
            selected_background_r: 7,
            selected_background_g: 193,
            selected_background_b: 96,
            selected_text_r: 255,
            selected_text_g: 255,
            selected_text_b: 255,
            comment_r: 120,
            comment_g: 120,
            comment_b: 120,
            border_r: 215,
            border_g: 215,
            border_b: 215,
            scrollbar_r: 128,
            scrollbar_g: 128,
            scrollbar_b: 128,
            preedit_background_r: 255,
            preedit_background_g: 255,
            preedit_background_b: 255,
            preedit_text_r: 32,
            preedit_text_g: 33,
            preedit_text_b: 36,
            selection_inflate_x: 2.0,
            selection_inflate_y: 2.0,
            corner_radius: 8.0,
        }
    }

    #[test]
    #[cfg(windows)]
    fn render_frame_paints_the_frame_rects_below_the_preedit_row() {
        let candidates = ["你", "好", "汉"].map(|text| CandidateRenderData {
            label: "1".to_owned(),
            reserved_label: "1".to_owned(),
            text: text.to_owned(),
            comment: String::new(),
        });
        // Content rects as the frame stores them: window-local, below a
        // 30 px preedit block, inset by the item padding.
        let item_rects = [0.0_f32, 1.0, 2.0].map(|row| Rect {
            left: 18.0,
            top: 30.0 + 6.0 + row * 36.0 + 6.0,
            right: 100.0,
            bottom: 30.0 + 6.0 + row * 36.0 + 30.0,
        });
        let geometry = RenderGeometry {
            item_padding_x: 8.0,
            item_padding_y: 6.0,
            ..RenderGeometry::default()
        };
        let frame = FramePaint {
            candidates: &candidates,
            item_rects: &item_rects,
            window_width: 120.0,
            window_height: 150.0,
            candidate_top: 30.0,
            viewport_offset: None,
            content_size: Size {
                width: 120.0,
                height: 120.0,
            },
            writing: axis_layout::WritingMode::Horizontal,
            theme: render_theme_input(),
            geometry,
            preedit: Some("ni"),
            high_contrast: false,
            selected: Some(1),
        };
        let render = render_frame(&frame);
        assert_eq!((render.width, render.height), (120, 150));
        // The highlight fills the second candidate's rect, not a row shifted
        // by the preedit block.
        let inside = render.pixel(60, 36 + 36 + 2).expect("inside the window");
        assert_eq!(&inside[..3], &[96, 193, 7], "selected background (BGR)");
        let first = render.pixel(60, 36 + 2).expect("inside the window");
        assert_ne!(
            &first[..3],
            &[96, 193, 7],
            "first candidate stays unselected"
        );
    }
}
