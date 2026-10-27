#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;

pub mod axis_layout;
mod colors;
mod command_line;
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
#[cfg(windows)]
pub mod presentation_server;
#[cfg(windows)]
pub mod renderer;
pub mod theme_tokens;
mod visual;
mod window_assembly;
#[cfg(windows)]
pub mod window_host;

pub use colors::*;
pub use command_line::*;
pub use interaction::*;
pub(crate) use labels::*;
pub(crate) use layout_engine::*;
pub use locale::*;
pub use model::*;
pub use paint::*;
pub use presentation::*;
pub use visual::*;
pub(crate) use window_assembly::*;

#[cfg(all(test, windows))]
mod fuzz_regression;

#[cfg(windows)]
pub use renderer::{CandidateRenderData, RenderGeometry};
#[cfg(windows)]
pub(crate) use renderer::{RenderColor, RenderWindowOutput};

pub(crate) const MAX_CANDIDATES: usize = 128;
const MAX_CANDIDATE_TEXT_UTF8: usize = 4096;
const MAX_TRACKED_CONTEXTS: usize = 64;
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetUserDefaultLocaleName(locale_name: *mut u16, locale_name_capacity: i32) -> i32;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Point {
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
pub(crate) enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PresentationOrientation {
    Automatic,
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Placement {
    Unlocked,
    Below,
    Above,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Fcitx5CandidateLayoutSize {
    pub width: f32,
    pub height: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Fcitx5CandidateLayoutRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
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

#[cfg(test)]
mod tests;
