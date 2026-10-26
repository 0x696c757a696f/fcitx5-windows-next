#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;

mod axis_ffi;
pub mod axis_layout;
mod candidate_abi;
mod colors;
mod command_line;
pub mod frame_ffi;
pub mod frame_update;
mod interaction;
mod labels;
mod layout_engine;
mod locale;
#[cfg(windows)]
mod measure_ffi;
mod model;
mod paint;
mod presentation;
mod presentation_ffi;
#[cfg(windows)]
pub mod presentation_server;
#[cfg(windows)]
pub mod renderer;
pub mod theme_tokens;
mod ui_plan;
mod visual;
mod window_assembly;
#[cfg(windows)]
pub mod window_host;

pub use axis_ffi::*;
pub use colors::*;
pub use command_line::*;
pub use interaction::*;
pub use labels::*;
pub use layout_engine::*;
pub use locale::*;
pub use model::*;
pub use paint::*;
pub use presentation::*;
pub use presentation_ffi::*;
pub use visual::*;
pub use window_assembly::*;

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
mod tests;
