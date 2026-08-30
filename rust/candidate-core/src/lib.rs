#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;

mod candidate_abi;
pub mod qingfeng;
mod ui_plan;

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

const MAX_CANDIDATES: usize = 128;
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

#[derive(Clone, Debug)]
pub struct LayoutInput {
    pub orientation: Orientation,
    pub items: Vec<Size>,
    pub caret: Point,
    pub caret_height: f32,
    pub work_area: Rect,
    pub max_width: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub row_gap: f32,
    pub column_gap: f32,
    pub placement: Placement,
    pub scroll_mode: bool,
    pub scroll_columns: usize,
    pub scroll_visible_rows: usize,
    pub selected: usize,
    pub scroll_cell_width: f32,
}

impl Default for LayoutInput {
    fn default() -> Self {
        Self {
            orientation: Orientation::Vertical,
            items: Vec::new(),
            caret: Point::default(),
            caret_height: 0.0,
            work_area: Rect::default(),
            max_width: 720.0,
            padding_x: 8.0,
            padding_y: 6.0,
            row_gap: 2.0,
            column_gap: 8.0,
            placement: Placement::Unlocked,
            scroll_mode: false,
            scroll_columns: 6,
            scroll_visible_rows: 6,
            selected: 0,
            scroll_cell_width: 96.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutResult {
    pub window: Rect,
    pub items: Vec<Rect>,
    pub item_indices: Vec<usize>,
    pub scrollbar_track: Rect,
    pub scrollbar_thumb: Rect,
    pub has_scrollbar: bool,
    pub first_visible: usize,
    pub placement: Placement,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Visibility {
    Hidden,
    Composition,
    Prediction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CandidateItem {
    id: u64,
    label: Vec<u8>,
    text: Vec<u8>,
    comment: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CandidateSnapshot {
    engine_epoch: u64,
    context_id: u64,
    composition_id: u64,
    revision: u64,
    preedit: Vec<u8>,
    auxiliary_up: Vec<u8>,
    auxiliary_down: Vec<u8>,
    candidates: Vec<CandidateItem>,
    selected: Option<usize>,
    page: u32,
    total: u32,
    visibility: Visibility,
    popup_allowed: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Freshness {
    composition_id: u64,
    latest_composition_id: u64,
    revision: u64,
}

#[derive(Default)]
pub struct CandidateModel {
    current: Option<CandidateSnapshot>,
    ffi_items: Vec<Fcitx5CandidateModelItem>,
    engine_epoch: u64,
    freshness: HashMap<u64, Freshness>,
    freshness_order: VecDeque<u64>,
}

/// Identity attached to every candidate semantic snapshot and notification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CandidateSnapshotIdentity {
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
}

/// Independent presentation capabilities. They intentionally compose instead of
/// selecting a mutually-exclusive accessibility mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CandidateCapabilities {
    pub keyboard: bool,
    pub uia: bool,
    pub narrator_nvda: bool,
    pub high_contrast: bool,
    pub large_text: bool,
    pub reduced_motion: bool,
    pub reduced_candidates: bool,
    pub stable_layout: bool,
}

/// Context classification used to enforce privacy at the semantic boundary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CandidatePrivacyContext {
    #[default]
    Normal,
    Password,
    Pin,
    Sensitive,
}

impl CandidatePrivacyContext {
    #[must_use]
    pub const fn suppress_text(self) -> bool {
        !matches!(self, Self::Normal)
    }

    #[must_use]
    pub const fn policy(self) -> CandidatePrivacyPolicy {
        if self.suppress_text() {
            CandidatePrivacyPolicy {
                allow_speech: false,
                allow_text_logging: false,
                allow_learning: false,
                allow_network: false,
            }
        } else {
            CandidatePrivacyPolicy {
                allow_speech: true,
                allow_text_logging: true,
                allow_learning: true,
                allow_network: true,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidatePrivacyPolicy {
    allow_speech: bool,
    allow_text_logging: bool,
    allow_learning: bool,
    allow_network: bool,
}

impl CandidatePrivacyPolicy {
    #[must_use]
    pub const fn allows_speech(self) -> bool {
        self.allow_speech
    }

    #[must_use]
    pub const fn allows_text_logging(self) -> bool {
        self.allow_text_logging
    }

    #[must_use]
    pub const fn allows_learning(self) -> bool {
        self.allow_learning
    }

    #[must_use]
    pub const fn allows_network(self) -> bool {
        self.allow_network
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateSemanticItem {
    pub id: u64,
    pub label: String,
    pub text: String,
    pub comment: String,
}

/// Immutable semantic projection shared by renderer, UIA, and notifications.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateSemanticSnapshot {
    pub identity: CandidateSnapshotIdentity,
    pub preedit: String,
    pub auxiliary_up: String,
    pub auxiliary_down: String,
    pub candidates: Vec<CandidateSemanticItem>,
    pub selected: Option<usize>,
    pub page: u32,
    pub total: u32,
    pub visibility: u8,
    pub popup_allowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateNotificationKind {
    Snapshot,
    Selection,
    Count,
    State,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateNotification {
    pub identity: CandidateSnapshotIdentity,
    pub kind: CandidateNotificationKind,
    pub selected: Option<usize>,
    pub count: usize,
    pub visibility: u8,
    pub text: Option<String>,
}

/// Deterministic revision-aware notification buffer.
#[derive(Clone, Debug, Default)]
pub struct CandidateNotificationQueue {
    pending: VecDeque<CandidateNotification>,
    latest: HashMap<(u64, u64), CandidateNotificationState>,
    cancelled: Vec<CandidateSnapshotIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CandidateNotificationState {
    identity: CandidateSnapshotIdentity,
    selected: Option<usize>,
    selected_text: Option<String>,
    count: usize,
    visibility: u8,
}

impl CandidateNotificationQueue {
    pub fn enqueue(
        &mut self,
        snapshot: &CandidateSemanticSnapshot,
        capabilities: CandidateCapabilities,
        privacy: CandidatePrivacyContext,
    ) {
        let identity = snapshot.identity;
        let scope = (identity.engine_epoch, identity.context_id);
        if self.latest.get(&scope).is_some_and(|latest| {
            identity.composition_id < latest.identity.composition_id
                || (identity.composition_id == latest.identity.composition_id
                    && identity.revision <= latest.identity.revision)
        }) {
            return;
        }
        let policy = privacy.policy();
        let selected_text = if policy.allows_speech() {
            snapshot
                .selected
                .and_then(|index| snapshot.candidates.get(index))
                .map(|candidate| candidate.text.clone())
        } else {
            None
        };
        let state = CandidateNotificationState {
            identity,
            selected: snapshot.selected,
            selected_text,
            count: snapshot.total as usize,
            visibility: snapshot.visibility,
        };
        let kinds = match self.latest.get(&scope) {
            None => vec![CandidateNotificationKind::Snapshot],
            Some(previous) => {
                let mut kinds = Vec::with_capacity(3);
                if previous.selected != state.selected
                    || previous.selected_text != state.selected_text
                {
                    kinds.push(CandidateNotificationKind::Selection);
                }
                if previous.count != state.count {
                    kinds.push(CandidateNotificationKind::Count);
                }
                if previous.visibility != state.visibility {
                    kinds.push(CandidateNotificationKind::State);
                }
                kinds
            }
        };
        self.pending.retain(|item| {
            let item_scope = (item.identity.engine_epoch, item.identity.context_id);
            item_scope != scope
        });
        for kind in kinds {
            self.pending.push_back(CandidateNotification {
                identity,
                kind,
                selected: state.selected,
                count: state.count,
                visibility: state.visibility,
                text: (capabilities.narrator_nvda
                    && policy.allows_speech()
                    && matches!(
                        kind,
                        CandidateNotificationKind::Snapshot | CandidateNotificationKind::Selection
                    ))
                .then(|| state.selected_text.clone())
                .flatten(),
            });
        }
        self.cancelled
            .retain(|cancelled| (cancelled.engine_epoch, cancelled.context_id) != scope);
        self.latest.insert(scope, state);
    }

    pub fn cancel(&mut self, identity: CandidateSnapshotIdentity) {
        self.cancelled.push(identity);
        self.pending.retain(|item| item.identity != identity);
    }

    #[must_use]
    pub fn drain_for(&mut self, identity: CandidateSnapshotIdentity) -> Vec<CandidateNotification> {
        if self.cancelled.contains(&identity)
            || self
                .latest
                .get(&(identity.engine_epoch, identity.context_id))
                .is_none_or(|latest| latest.identity != identity)
        {
            return Vec::new();
        }
        let mut notifications = Vec::new();
        self.pending.retain(|item| {
            if item.identity == identity {
                notifications.push(item.clone());
                false
            } else {
                true
            }
        });
        notifications
    }
}

impl Default for LayoutResult {
    fn default() -> Self {
        Self {
            window: Rect::default(),
            items: Vec::new(),
            item_indices: Vec::new(),
            scrollbar_track: Rect::default(),
            scrollbar_thumb: Rect::default(),
            has_scrollbar: false,
            first_visible: 0,
            placement: Placement::Below,
        }
    }
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
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), out, count);
            }
        }
    }
    value.len()
}

fn default_dwrite_locale() -> Vec<u16> {
    let mut locale = [0_u16; LOCALE_NAME_MAX_LENGTH];
    let length = unsafe { GetUserDefaultLocaleName(locale.as_mut_ptr(), locale.len() as i32) };
    if length > 1 && length as usize <= locale.len() {
        return locale[..length as usize - 1].to_vec();
    }
    DEFAULT_DWRITE_LOCALE.to_vec()
}

fn content_locale_valid(locale: &[u8]) -> bool {
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

fn content_locale_or_default(locale: &[u8]) -> Vec<u16> {
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

fn parse_candidate_command_line(
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
pub extern "C" fn fcitx5_candidate_model_create() -> *mut c_void {
    Box::into_raw(Box::<CandidateModel>::default()) as *mut c_void
}

#[no_mangle]
/// # Safety
///
/// `model` must be either null or a pointer returned by
/// `fcitx5_candidate_model_create` that has not already been destroyed.
pub unsafe extern "C" fn fcitx5_candidate_model_destroy(model: *mut c_void) {
    if model.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(model.cast::<CandidateModel>()) });
}

#[no_mangle]
/// # Safety
///
/// `model` must be either null or a valid pointer returned by
/// `fcitx5_candidate_model_create`.
pub unsafe extern "C" fn fcitx5_candidate_model_reset(model: *mut c_void) {
    if model.is_null() {
        return;
    }
    let model = unsafe { &mut *model.cast::<CandidateModel>() };
    model.reset();
}

#[no_mangle]
/// # Safety
///
/// `snapshot` and the candidate/string buffers it references must be valid for
/// the duration of this call. Pointers are not retained.
pub unsafe extern "C" fn fcitx5_candidate_model_validate(
    snapshot: *const Fcitx5CandidateModelSnapshot,
) -> u8 {
    let Some(snapshot) = (unsafe { snapshot_from_ffi(snapshot) }) else {
        return 0;
    };
    u8::from(validate_snapshot(&snapshot))
}

#[no_mangle]
/// # Safety
///
/// `model` must be a valid pointer returned by `fcitx5_candidate_model_create`.
/// `snapshot` and the buffers it references must remain valid for the duration
/// of this call. Pointers are not retained.
pub unsafe extern "C" fn fcitx5_candidate_model_apply(
    model: *mut c_void,
    snapshot: *const Fcitx5CandidateModelSnapshot,
) -> u32 {
    if model.is_null() {
        return 3;
    }
    let Some(snapshot) = (unsafe { snapshot_from_ffi(snapshot) }) else {
        return 3;
    };
    let model = unsafe { &mut *model.cast::<CandidateModel>() };
    model.apply(snapshot)
}

#[no_mangle]
/// # Safety
///
/// `model` must be a valid pointer returned by `fcitx5_candidate_model_create`.
/// `output` must point to writable storage for one snapshot. Its string and item
/// pointers remain valid until the next model mutation or destruction.
pub unsafe extern "C" fn fcitx5_candidate_model_current(
    model: *mut c_void,
    output: *mut Fcitx5CandidateModelSnapshot,
) -> u8 {
    if model.is_null() || output.is_null() {
        return 0;
    }
    let model = unsafe { &mut *model.cast::<CandidateModel>() };
    let Some(snapshot) = model.current.as_ref() else {
        return 0;
    };
    let visibility = match snapshot.visibility {
        Visibility::Hidden => 0,
        Visibility::Composition => 1,
        Visibility::Prediction => 2,
    };
    unsafe {
        *output = Fcitx5CandidateModelSnapshot {
            engine_epoch: snapshot.engine_epoch,
            context_id: snapshot.context_id,
            composition_id: snapshot.composition_id,
            revision: snapshot.revision,
            preedit: ffi_utf8(&snapshot.preedit),
            auxiliary_up: ffi_utf8(&snapshot.auxiliary_up),
            auxiliary_down: ffi_utf8(&snapshot.auxiliary_down),
            candidates: model.ffi_items.as_ptr(),
            candidate_count: model.ffi_items.len(),
            selected: snapshot.selected.unwrap_or(0),
            has_selected: u8::from(snapshot.selected.is_some()),
            page: snapshot.page,
            total: snapshot.total,
            visibility,
            popup_allowed: u8::from(snapshot.popup_allowed),
        };
    }
    1
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
        unsafe { std::slice::from_raw_parts_mut(indices, count) }
    };
    for (slot, candidate_index) in target.iter_mut().zip(start..end) {
        *slot = candidate_index;
    }
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
    unsafe { &mut *state.cast::<CandidatePresentationState>() }.set_placement(placement);
    1
}

#[no_mangle]
/// # Safety
///
/// `state` must be a valid pointer returned by
/// `fcitx5_candidate_presentation_create`.
pub unsafe extern "C" fn fcitx5_candidate_presentation_stable_window_width(
    state: *mut c_void,
    measured_width: f32,
    max_allowed_width: f32,
) -> f32 {
    if state.is_null() {
        return 0.0;
    }
    unsafe { &mut *state.cast::<CandidatePresentationState>() }
        .stable_window_width(measured_width, max_allowed_width)
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
    let locale = unsafe { bytes_from_ffi(locale) }
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .unwrap_or_default();
    let candidate_inputs = if candidate_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(candidates, candidate_count) }
    };
    let Some(candidates) = candidate_inputs
        .iter()
        .map(|candidate| {
            let text = unsafe { bytes_from_ffi(candidate.text) }?;
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

fn ffi_utf8(value: &[u8]) -> Fcitx5CandidateUtf8 {
    Fcitx5CandidateUtf8 {
        ptr: value.as_ptr(),
        len: value.len(),
    }
}

impl CandidateModel {
    /// Applies an owned semantic snapshot and returns the stable FFI-compatible
    /// result code: 0 applied, 1 duplicate, 2 stale, 3 invalid.
    pub fn apply_semantic_snapshot(&mut self, snapshot: CandidateSemanticSnapshot) -> u32 {
        let visibility = match snapshot.visibility {
            0 => Visibility::Hidden,
            1 => Visibility::Composition,
            2 => Visibility::Prediction,
            _ => return 3,
        };
        self.apply(CandidateSnapshot {
            engine_epoch: snapshot.identity.engine_epoch,
            context_id: snapshot.identity.context_id,
            composition_id: snapshot.identity.composition_id,
            revision: snapshot.identity.revision,
            preedit: snapshot.preedit.into_bytes(),
            auxiliary_up: snapshot.auxiliary_up.into_bytes(),
            auxiliary_down: snapshot.auxiliary_down.into_bytes(),
            candidates: snapshot
                .candidates
                .into_iter()
                .map(|item| CandidateItem {
                    id: item.id,
                    label: item.label.into_bytes(),
                    text: item.text.into_bytes(),
                    comment: item.comment.into_bytes(),
                })
                .collect(),
            selected: snapshot.selected,
            page: snapshot.page,
            total: snapshot.total,
            visibility,
            popup_allowed: snapshot.popup_allowed,
        })
    }

    #[must_use]
    pub fn semantic_snapshot(&self) -> Option<CandidateSemanticSnapshot> {
        self.current
            .as_ref()
            .map(|snapshot| CandidateSemanticSnapshot {
                identity: CandidateSnapshotIdentity {
                    engine_epoch: snapshot.engine_epoch,
                    context_id: snapshot.context_id,
                    composition_id: snapshot.composition_id,
                    revision: snapshot.revision,
                },
                preedit: String::from_utf8_lossy(&snapshot.preedit).into_owned(),
                auxiliary_up: String::from_utf8_lossy(&snapshot.auxiliary_up).into_owned(),
                auxiliary_down: String::from_utf8_lossy(&snapshot.auxiliary_down).into_owned(),
                candidates: snapshot
                    .candidates
                    .iter()
                    .map(|item| CandidateSemanticItem {
                        id: item.id,
                        label: String::from_utf8_lossy(&item.label).into_owned(),
                        text: String::from_utf8_lossy(&item.text).into_owned(),
                        comment: String::from_utf8_lossy(&item.comment).into_owned(),
                    })
                    .collect(),
                selected: snapshot.selected,
                page: snapshot.page,
                total: snapshot.total,
                visibility: match snapshot.visibility {
                    Visibility::Hidden => 0,
                    Visibility::Composition => 1,
                    Visibility::Prediction => 2,
                },
                popup_allowed: snapshot.popup_allowed,
            })
    }

    fn apply(&mut self, snapshot: CandidateSnapshot) -> u32 {
        if !validate_snapshot(&snapshot) {
            return 3;
        }
        if self.engine_epoch != 0 && snapshot.engine_epoch < self.engine_epoch {
            return 2;
        }
        if self.engine_epoch == 0 || snapshot.engine_epoch > self.engine_epoch {
            self.engine_epoch = snapshot.engine_epoch;
            self.current = None;
            self.freshness.clear();
            self.freshness_order.clear();
        }
        if let Some(freshness) = self.freshness.get(&snapshot.context_id) {
            if snapshot.composition_id == freshness.composition_id
                && snapshot.revision == freshness.revision
            {
                return if self.current.as_ref() == Some(&snapshot) {
                    1
                } else {
                    2
                };
            }
            if snapshot.composition_id == freshness.composition_id || snapshot.composition_id == 0 {
                if snapshot.revision < freshness.revision {
                    return 2;
                }
            } else if snapshot.composition_id <= freshness.latest_composition_id {
                return 2;
            }
        }
        self.remember_context(snapshot.context_id, &snapshot);
        self.current = Some(snapshot);
        self.refresh_ffi_items();
        0
    }

    fn remember_context(&mut self, context_id: u64, snapshot: &CandidateSnapshot) {
        let inserted = !self.freshness.contains_key(&context_id);
        if inserted {
            self.freshness_order.push_back(context_id);
            while self.freshness.len() > MAX_TRACKED_CONTEXTS {
                let Some(evicted) = self.freshness_order.pop_front() else {
                    break;
                };
                if evicted != context_id {
                    self.freshness.remove(&evicted);
                }
            }
        }
        let freshness = self.freshness.entry(context_id).or_default();
        freshness.composition_id = snapshot.composition_id;
        if snapshot.composition_id != 0 && snapshot.composition_id > freshness.latest_composition_id
        {
            freshness.latest_composition_id = snapshot.composition_id;
        }
        freshness.revision = snapshot.revision;
    }

    fn reset(&mut self) {
        self.current = None;
        self.ffi_items.clear();
        self.engine_epoch = 0;
        self.freshness.clear();
        self.freshness_order.clear();
    }

    fn refresh_ffi_items(&mut self) {
        self.ffi_items = self
            .current
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .candidates
                    .iter()
                    .map(|item| Fcitx5CandidateModelItem {
                        id: item.id,
                        label: ffi_utf8(&item.label),
                        text: ffi_utf8(&item.text),
                        comment: ffi_utf8(&item.comment),
                    })
                    .collect()
            })
            .unwrap_or_default();
    }
}

fn validate_snapshot(snapshot: &CandidateSnapshot) -> bool {
    if snapshot.engine_epoch == 0
        || snapshot.context_id == 0
        || snapshot.revision == 0
        || snapshot.candidates.len() > MAX_CANDIDATES
        || !valid_text(&snapshot.preedit)
        || !valid_text(&snapshot.auxiliary_up)
        || !valid_text(&snapshot.auxiliary_down)
    {
        return false;
    }
    if snapshot
        .selected
        .is_some_and(|selected| selected >= snapshot.candidates.len())
    {
        return false;
    }
    if snapshot.total < snapshot.candidates.len() as u32 {
        return false;
    }
    if snapshot.visibility == Visibility::Hidden && !snapshot.candidates.is_empty() {
        return false;
    }
    if snapshot.visibility != Visibility::Hidden && snapshot.composition_id == 0 {
        return false;
    }
    snapshot.candidates.iter().all(|item| {
        item.id != 0
            && valid_text(&item.label)
            && valid_text(&item.text)
            && valid_text(&item.comment)
    })
}

fn valid_text(value: &[u8]) -> bool {
    value.len() <= MAX_CANDIDATE_TEXT_UTF8
        && !value.contains(&0)
        && std::str::from_utf8(value).is_ok()
}

unsafe fn snapshot_from_ffi(
    snapshot: *const Fcitx5CandidateModelSnapshot,
) -> Option<CandidateSnapshot> {
    if snapshot.is_null() {
        return None;
    }
    let snapshot = unsafe { *snapshot };
    if snapshot.candidate_count > 0 && snapshot.candidates.is_null() {
        return None;
    }
    let candidates = if snapshot.candidate_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(snapshot.candidates, snapshot.candidate_count) }
    };
    let visibility = match snapshot.visibility {
        0 => Visibility::Hidden,
        1 => Visibility::Composition,
        2 => Visibility::Prediction,
        _ => return None,
    };
    let mut owned_candidates = Vec::with_capacity(candidates.len());
    for item in candidates {
        owned_candidates.push(CandidateItem {
            id: item.id,
            label: unsafe { bytes_from_ffi(item.label) }?.to_vec(),
            text: unsafe { bytes_from_ffi(item.text) }?.to_vec(),
            comment: unsafe { bytes_from_ffi(item.comment) }?.to_vec(),
        });
    }
    Some(CandidateSnapshot {
        engine_epoch: snapshot.engine_epoch,
        context_id: snapshot.context_id,
        composition_id: snapshot.composition_id,
        revision: snapshot.revision,
        preedit: unsafe { bytes_from_ffi(snapshot.preedit) }?.to_vec(),
        auxiliary_up: unsafe { bytes_from_ffi(snapshot.auxiliary_up) }?.to_vec(),
        auxiliary_down: unsafe { bytes_from_ffi(snapshot.auxiliary_down) }?.to_vec(),
        candidates: owned_candidates,
        selected: (snapshot.has_selected != 0).then_some(snapshot.selected),
        page: snapshot.page,
        total: snapshot.total,
        visibility,
        popup_allowed: snapshot.popup_allowed != 0,
    })
}

unsafe fn bytes_from_ffi(value: Fcitx5CandidateUtf8) -> Option<&'static [u8]> {
    if value.len == 0 {
        return Some(&[]);
    }
    if value.ptr.is_null() {
        return None;
    }
    Some(unsafe { std::slice::from_raw_parts(value.ptr, value.len) })
}

unsafe fn utf16_from_ffi(value: Fcitx5CandidateUtf16) -> Option<&'static [u16]> {
    if value.len == 0 {
        return Some(&[]);
    }
    if value.ptr.is_null() {
        return None;
    }
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

#[no_mangle]
/// # Safety
///
/// `input`, `output`, and the item/output buffers must either be valid for the
/// specified lengths or null when their corresponding length is zero. Pointers
/// are not retained after this call.
pub unsafe extern "C" fn fcitx5_candidate_layout_run(
    input: *const Fcitx5CandidateLayoutInput,
    items: *const Fcitx5CandidateLayoutSize,
    item_count: usize,
    out_items: *mut Fcitx5CandidateLayoutRect,
    out_item_indices: *mut usize,
    out_capacity: usize,
    output: *mut Fcitx5CandidateLayoutOutput,
) -> i32 {
    if input.is_null() || output.is_null() || item_count > out_capacity {
        return 1;
    }
    if item_count > 0 && (items.is_null() || out_items.is_null() || out_item_indices.is_null()) {
        return 1;
    }
    let input = unsafe { *input };
    let item_slice = if item_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(items, item_count) }
    };
    let Some(orientation) = orientation_from_ffi(input.orientation) else {
        return 1;
    };
    let Some(placement) = placement_from_ffi(input.placement) else {
        return 1;
    };
    let result = layout(&LayoutInput {
        orientation,
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
        padding_x: input.padding_x,
        padding_y: input.padding_y,
        row_gap: input.row_gap,
        column_gap: input.column_gap,
        placement,
        scroll_mode: input.scroll_mode != 0,
        scroll_columns: input.scroll_columns,
        scroll_visible_rows: input.scroll_visible_rows,
        selected: input.selected,
        scroll_cell_width: input.scroll_cell_width,
    });
    if result.items.len() > out_capacity || result.item_indices.len() != result.items.len() {
        return 1;
    }
    if !result.items.is_empty() {
        let out_items = unsafe { std::slice::from_raw_parts_mut(out_items, result.items.len()) };
        let out_indices =
            unsafe { std::slice::from_raw_parts_mut(out_item_indices, result.item_indices.len()) };
        for (target, source) in out_items.iter_mut().zip(result.items.iter()) {
            *target = rect_to_ffi(*source);
        }
        out_indices.copy_from_slice(&result.item_indices);
    }
    unsafe {
        *output = Fcitx5CandidateLayoutOutput {
            window: rect_to_ffi(result.window),
            scrollbar_track: rect_to_ffi(result.scrollbar_track),
            scrollbar_thumb: rect_to_ffi(result.scrollbar_thumb),
            has_scrollbar: u8::from(result.has_scrollbar),
            first_visible: result.first_visible,
            placement: placement_to_ffi(result.placement),
            item_count: result.items.len(),
        };
    }
    0
}

#[no_mangle]
/// # Safety
///
/// `rects` must be valid for `rect_count` elements when `rect_count` is
/// non-zero. `out_index` must point to writable storage. Pointers are not
/// retained.
pub unsafe extern "C" fn fcitx5_candidate_hit_test(
    rects: *const Fcitx5CandidateLayoutRect,
    rect_count: usize,
    x: f32,
    y: f32,
    out_index: *mut usize,
) -> u8 {
    if out_index.is_null() || (rect_count > 0 && rects.is_null()) {
        return 0;
    }
    let rects = if rect_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(rects, rect_count) }
    };
    for (index, rect) in rects.iter().enumerate() {
        if x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom {
            unsafe {
                *out_index = index;
            }
            return 1;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_selection_intent(
    target_process_id: u32,
    engine_epoch: u64,
    context_id: u64,
    composition_id: u64,
    revision: u64,
    candidate_id: u64,
) -> Fcitx5CandidateSelectionIntent {
    let intent = Fcitx5CandidateSelectionIntent {
        target_process_id,
        engine_epoch,
        context_id,
        composition_id,
        revision,
        candidate_id,
    };
    if selection_intent_valid(intent) {
        intent
    } else {
        Fcitx5CandidateSelectionIntent::default()
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
    let Some(source_label) = (unsafe { utf16_from_ffi(source_label) }) else {
        return 0;
    };
    let Some(custom_prefix) = (unsafe { utf16_from_ffi(custom_prefix) }) else {
        return 0;
    };
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
        unsafe { std::slice::from_raw_parts(items, item_count) }
    };
    let out_items = if item_count == 0 {
        &mut []
    } else {
        unsafe { std::slice::from_raw_parts_mut(out_items, item_count) }
    };
    let (segments, label_column_width) = candidate_render_segments(items);
    unsafe {
        *out_label_column_width = label_column_width;
    }
    out_items.copy_from_slice(&segments);
    0
}

fn selection_intent_valid(intent: Fcitx5CandidateSelectionIntent) -> bool {
    intent.target_process_id != 0
        && intent.engine_epoch != 0
        && intent.context_id != 0
        && intent.composition_id != 0
        && intent.revision != 0
        && intent.candidate_id != 0
}

fn orientation_from_ffi(value: u32) -> Option<Orientation> {
    match value {
        0 => Some(Orientation::Vertical),
        1 => Some(Orientation::Horizontal),
        _ => None,
    }
}

fn placement_from_ffi(value: u32) -> Option<Placement> {
    match value {
        0 => Some(Placement::Unlocked),
        1 => Some(Placement::Below),
        2 => Some(Placement::Above),
        _ => None,
    }
}

fn placement_to_ffi(value: Placement) -> u32 {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateText {
    pub text: String,
    pub comment: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompositionIdentity {
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct AutomaticOrientationInput<'a> {
    pub candidates: &'a [CandidateText],
    pub locale: &'a str,
    pub work_area: Rect,
    pub caret_x: f32,
    pub scale: f32,
    pub page_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CandidatePresentationUpdate {
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub selected: usize,
    pub has_selected: u8,
    pub candidate_count: usize,
    pub page: u32,
    pub page_size: u32,
    pub candidate_bulk: u8,
    pub configured_scroll_mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CandidatePresentationOutput {
    pub selected: usize,
    pub has_selected: u8,
    pub scroll_mode: u8,
    pub scroll_expanded: u8,
    pub scroll_columns: usize,
    pub ordinary_start: usize,
    pub ordinary_count: usize,
    pub candidate_bulk: u8,
    pub page_size: u32,
    pub placement: u32,
    pub stable_width: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidatePresentationRenderPlan {
    pub selected: usize,
    pub has_selected: u8,
    pub render_count: usize,
}

#[derive(Clone, Debug)]
pub struct CandidatePresentationState {
    identity: Option<CompositionIdentity>,
    revision: u64,
    selected: Option<usize>,
    scroll_mode: bool,
    scroll_expanded: bool,
    scroll_columns: usize,
    ordinary_start: usize,
    ordinary_count: usize,
    candidate_count: usize,
    candidate_bulk: bool,
    page_size: u32,
    placement: Placement,
    layout: CompositionLayoutState,
}

impl Default for CandidatePresentationState {
    fn default() -> Self {
        Self {
            identity: None,
            revision: 0,
            selected: None,
            scroll_mode: false,
            scroll_expanded: false,
            scroll_columns: 6,
            ordinary_start: 0,
            ordinary_count: 0,
            candidate_count: 0,
            candidate_bulk: false,
            page_size: 0,
            placement: Placement::Unlocked,
            layout: CompositionLayoutState::default(),
        }
    }
}

impl CandidatePresentationState {
    /// Applies the presentation metadata for one accepted semantic snapshot.
    /// Returns 0 for applied, 1 for duplicate, 2 for stale, and 3 for invalid.
    pub fn apply(&mut self, input: CandidatePresentationUpdate) -> u32 {
        if input.engine_epoch == 0
            || input.context_id == 0
            || input.revision == 0
            || input.candidate_count > MAX_CANDIDATES
            || (input.has_selected != 0 && input.selected >= input.candidate_count)
        {
            return 3;
        }
        let identity = CompositionIdentity {
            engine_epoch: input.engine_epoch,
            context_id: input.context_id,
            composition_id: input.composition_id,
        };
        if let Some(previous) = self.identity {
            if input.engine_epoch < previous.engine_epoch
                || (input.engine_epoch == previous.engine_epoch
                    && input.context_id == previous.context_id
                    && input.composition_id != 0
                    && previous.composition_id != 0
                    && input.composition_id < previous.composition_id)
            {
                return 2;
            }
            if identity == previous {
                if input.revision == self.revision {
                    return 1;
                }
                if input.revision < self.revision {
                    return 2;
                }
            }
        }
        if self.identity != Some(identity) {
            self.identity = Some(identity);
            self.layout.begin(identity);
            self.revision = 0;
            self.scroll_expanded = false;
            self.placement = Placement::Unlocked;
        }
        self.revision = input.revision;
        self.candidate_bulk = input.candidate_bulk != 0;
        self.candidate_count = input.candidate_count;
        self.page_size = input.page_size;
        self.selected = (input.has_selected != 0).then_some(input.selected);
        self.scroll_columns = (input.page_size as usize).clamp(1, 9);
        let scroll_eligible = input.configured_scroll_mode != 0
            && input.candidate_bulk != 0
            && input.page_size != 0
            && input.candidate_count > input.page_size as usize;
        let focus_beyond_first_page = self
            .selected
            .is_some_and(|selected| selected >= input.page_size as usize);
        self.scroll_expanded =
            scroll_eligible && (self.scroll_expanded || input.page > 0 || focus_beyond_first_page);
        self.scroll_mode = scroll_eligible && self.scroll_expanded;
        self.ordinary_count = if input.page_size == 0 {
            input.candidate_count
        } else {
            (input.page_size as usize).min(input.candidate_count)
        };
        self.ordinary_start = if input.candidate_bulk != 0 && !self.scroll_mode {
            (input.page as usize)
                .saturating_mul(input.page_size as usize)
                .min(input.candidate_count - self.ordinary_count)
        } else {
            0
        };
        if !self.scroll_mode && input.candidate_bulk == 0 {
            if let Some(selected) = self.selected {
                if selected < self.ordinary_count
                    && self.ordinary_start + selected < input.candidate_count
                {
                    self.selected = Some(self.ordinary_start + selected);
                }
            }
        }
        0
    }

    pub fn output(&self) -> CandidatePresentationOutput {
        CandidatePresentationOutput {
            selected: self.selected.unwrap_or_default(),
            has_selected: u8::from(self.selected.is_some()),
            scroll_mode: u8::from(self.scroll_mode),
            scroll_expanded: u8::from(self.scroll_expanded),
            scroll_columns: self.scroll_columns,
            ordinary_start: self.ordinary_start,
            ordinary_count: self.ordinary_count,
            candidate_bulk: u8::from(self.candidate_bulk),
            page_size: self.page_size,
            placement: placement_to_ffi(self.placement),
            stable_width: self.layout.stable_width,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn set_placement(&mut self, placement: Placement) {
        self.placement = placement;
    }

    pub fn stable_window_width(&mut self, measured_width: f32, max_allowed_width: f32) -> f32 {
        self.layout
            .stable_window_width(measured_width, max_allowed_width)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CompositionLayoutState {
    identity: Option<CompositionIdentity>,
    auto_orientation: Option<Orientation>,
    stable_width: f32,
}

impl CompositionLayoutState {
    pub fn reset(&mut self) {
        self.identity = None;
        self.auto_orientation = None;
        self.stable_width = 0.0;
    }

    pub fn begin(&mut self, identity: CompositionIdentity) {
        if self.identity != Some(identity) {
            self.identity = Some(identity);
            self.auto_orientation = None;
            self.stable_width = 0.0;
        }
    }

    pub fn resolve_orientation(
        &mut self,
        configured: PresentationOrientation,
        input: AutomaticOrientationInput<'_>,
    ) -> Orientation {
        match configured {
            PresentationOrientation::Vertical => Orientation::Vertical,
            PresentationOrientation::Horizontal => Orientation::Horizontal,
            PresentationOrientation::Automatic => {
                if let Some(orientation) = self.auto_orientation {
                    return orientation;
                }
                let orientation = resolve_automatic_orientation(
                    input.candidates,
                    input.locale,
                    input.work_area,
                    input.caret_x,
                    input.scale,
                    input.page_size,
                );
                self.auto_orientation = Some(orientation);
                orientation
            }
        }
    }

    pub fn stable_window_width(&mut self, measured_width: f32, max_allowed_width: f32) -> f32 {
        let max_allowed_width = max_allowed_width.max(0.0);
        if self.stable_width > max_allowed_width {
            self.stable_width = max_allowed_width;
        }
        let width = if self.stable_width > 0.0 && measured_width < self.stable_width {
            self.stable_width
        } else {
            measured_width.min(max_allowed_width)
        };
        self.stable_width = self.stable_width.max(width);
        width
    }
}

pub fn layout(input: &LayoutInput) -> LayoutResult {
    let mut result = LayoutResult::default();
    if input.scroll_mode && !input.items.is_empty() {
        let preferred_scroll_cell_width = input.scroll_cell_width.max(40.0);
        if input.orientation == Orientation::Vertical {
            let rows_per_column = input.scroll_columns.clamp(1, 9);
            let visible_columns = input.scroll_visible_rows.clamp(1, 6);
            let columns = input.items.len().div_ceil(rows_per_column);
            let selected_column = input.selected.min(input.items.len() - 1) / rows_per_column;
            let viewport_start = (selected_column / visible_columns) * visible_columns;
            let first_column = if columns > visible_columns {
                viewport_start.min(columns - visible_columns)
            } else {
                0
            };
            let mut shown_columns = visible_columns.min(columns - first_column);
            let row_height = input
                .items
                .iter()
                .fold(0.0_f32, |height, item| height.max(item.height));
            let work_width = (input.work_area.right - input.work_area.left).max(0.0);
            let work_height = (input.work_area.bottom - input.work_area.top).max(0.0);
            let target_width = work_width.min(if input.max_width > 0.0 {
                input.max_width
            } else {
                work_width
            });
            let build_column_widths = |count: usize| -> Vec<f32> {
                let mut widths = vec![0.0_f32; count];
                for (column, width) in widths.iter_mut().enumerate() {
                    for row in 0..rows_per_column {
                        let index = (first_column + column) * rows_per_column + row;
                        if index >= input.items.len() {
                            break;
                        }
                        *width = width.max(input.items[index].width);
                    }
                    *width = width.clamp(40.0, preferred_scroll_cell_width);
                }
                widths
            };
            let natural_width = |widths: &[f32]| -> f32 {
                let columns_width: f32 = widths.iter().sum();
                let column_gaps = input.column_gap * widths.len().saturating_sub(1) as f32;
                columns_width + column_gaps + input.padding_x * 2.0
            };
            let mut column_widths = build_column_widths(shown_columns);
            while shown_columns > 1 && natural_width(&column_widths) > target_width {
                shown_columns -= 1;
                column_widths = build_column_widths(shown_columns);
            }
            let first_visible = first_column * rows_per_column;
            let end = input
                .items
                .len()
                .min((first_column + shown_columns) * rows_per_column);
            let width = natural_width(&column_widths).min(target_width);
            if shown_columns == 1 && width < natural_width(&column_widths) {
                column_widths[0] = (width - input.padding_x * 2.0).max(1.0);
            }
            let height = (input.padding_y * 2.0
                + row_height * rows_per_column as f32
                + input.row_gap * rows_per_column.saturating_sub(1) as f32)
                .min(work_height);
            let below = input.caret.y + input.caret_height;
            let mut placement = input.placement;
            if placement == Placement::Unlocked {
                placement = if below + height <= input.work_area.bottom {
                    Placement::Below
                } else {
                    Placement::Above
                };
            }
            let top = if placement == Placement::Below {
                below
            } else {
                input.caret.y - height
            }
            .clamp(input.work_area.top, input.work_area.bottom - height);
            let left = input
                .caret
                .x
                .clamp(input.work_area.left, input.work_area.right - width);
            result.window = Rect {
                left,
                top,
                right: left + width,
                bottom: top + height,
            };
            result.placement = placement;
            result.first_visible = first_visible;
            let mut x = left + input.padding_x;
            for (column, column_width) in column_widths.iter().enumerate().take(shown_columns) {
                for row in 0..rows_per_column {
                    let index = (first_column + column) * rows_per_column + row;
                    if index >= end {
                        break;
                    }
                    let y = top + input.padding_y + row as f32 * (row_height + input.row_gap);
                    result.items.push(Rect {
                        left: x,
                        top: y,
                        right: x + *column_width,
                        bottom: y + row_height,
                    });
                    result.item_indices.push(index);
                }
                x += *column_width + input.column_gap;
            }
            if columns > shown_columns {
                result.has_scrollbar = true;
                result.scrollbar_track = Rect {
                    left: left + width - 6.0,
                    top: top + input.padding_y,
                    right: left + width - 2.0,
                    bottom: top + height - input.padding_y,
                };
                let track_height = result.scrollbar_track.bottom - result.scrollbar_track.top;
                let thumb_height = (track_height * shown_columns as f32 / columns as f32).max(18.0);
                let progress = if columns == shown_columns {
                    0.0
                } else {
                    first_column as f32 / (columns - shown_columns) as f32
                };
                let thumb_top =
                    result.scrollbar_track.top + (track_height - thumb_height) * progress;
                result.scrollbar_thumb = Rect {
                    left: result.scrollbar_track.left,
                    top: thumb_top,
                    right: result.scrollbar_track.right,
                    bottom: thumb_top + thumb_height,
                };
            }
            return result;
        }

        let columns = input.scroll_columns.clamp(1, 9);
        let visible_rows = input.scroll_visible_rows.clamp(1, 6);
        let rows = input.items.len().div_ceil(columns);
        let selected_row = input.selected.min(input.items.len() - 1) / columns;
        let viewport_start = (selected_row / visible_rows) * visible_rows;
        let first_row = if rows > visible_rows {
            viewport_start.min(rows - visible_rows)
        } else {
            0
        };
        let shown_rows = visible_rows.min(rows - first_row);
        let first_visible = first_row * columns;
        let end = input.items.len().min((first_row + shown_rows) * columns);
        let row_height = input
            .items
            .iter()
            .fold(0.0_f32, |height, item| height.max(item.height));
        let work_width = (input.work_area.right - input.work_area.left).max(0.0);
        let work_height = (input.work_area.bottom - input.work_area.top).max(0.0);
        let mut content_width = 0.0_f32;
        for row in 0..shown_rows {
            let mut row_width = 0.0_f32;
            for column in 0..columns {
                let index = first_visible + row * columns + column;
                if index >= end {
                    break;
                }
                if row_width > 0.0 {
                    row_width += input.column_gap;
                }
                row_width += input.items[index].width;
            }
            content_width = content_width.max(row_width);
        }
        let width = (content_width + input.padding_x * 2.0)
            .min(input.max_width)
            .min(work_width);
        let height = (input.padding_y * 2.0
            + row_height * shown_rows as f32
            + input.row_gap * shown_rows.saturating_sub(1) as f32)
            .min(work_height);
        let below = input.caret.y + input.caret_height;
        let mut placement = input.placement;
        if placement == Placement::Unlocked {
            placement = if below + height <= input.work_area.bottom {
                Placement::Below
            } else {
                Placement::Above
            };
        }
        let top = if placement == Placement::Below {
            below
        } else {
            input.caret.y - height
        }
        .clamp(input.work_area.top, input.work_area.bottom - height);
        let left = input
            .caret
            .x
            .clamp(input.work_area.left, input.work_area.right - width);
        result.window = Rect {
            left,
            top,
            right: left + width,
            bottom: top + height,
        };
        result.placement = placement;
        result.first_visible = first_visible;
        let usable_width =
            (width - input.padding_x * 2.0 - input.column_gap * columns.saturating_sub(1) as f32)
                .max(0.0);
        let cell_width = usable_width / columns as f32;
        for index in result.first_visible..end {
            let local = index - result.first_visible;
            let row = local / columns;
            let column = local % columns;
            let x = left + input.padding_x + column as f32 * (cell_width + input.column_gap);
            let y = top + input.padding_y + row as f32 * (row_height + input.row_gap);
            result.items.push(Rect {
                left: x,
                top: y,
                right: x + cell_width,
                bottom: y + row_height,
            });
            result.item_indices.push(index);
        }
        if rows > shown_rows {
            result.has_scrollbar = true;
            result.scrollbar_track = Rect {
                left: left + width - 6.0,
                top: top + input.padding_y,
                right: left + width - 2.0,
                bottom: top + height - input.padding_y,
            };
            let track_height = result.scrollbar_track.bottom - result.scrollbar_track.top;
            let thumb_height = (track_height * shown_rows as f32 / rows as f32).max(18.0);
            let progress = if rows == shown_rows {
                0.0
            } else {
                first_row as f32 / (rows - shown_rows) as f32
            };
            let thumb_top = result.scrollbar_track.top + (track_height - thumb_height) * progress;
            result.scrollbar_thumb = Rect {
                left: result.scrollbar_track.left,
                top: thumb_top,
                right: result.scrollbar_track.right,
                bottom: thumb_top + thumb_height,
            };
        }
        return result;
    }

    let mut content_width = 0.0_f32;
    let mut content_height = 0.0_f32;
    if input.orientation == Orientation::Vertical {
        for item in &input.items {
            content_width = content_width.max(item.width);
            if content_height > 0.0 {
                content_height += input.row_gap;
            }
            content_height += item.height;
        }
    } else {
        for item in &input.items {
            if content_width > 0.0 {
                content_width += input.column_gap;
            }
            content_width += item.width;
            content_height = content_height.max(item.height);
        }
    }
    let work_width = (input.work_area.right - input.work_area.left).max(0.0);
    let work_height = (input.work_area.bottom - input.work_area.top).max(0.0);
    let width = (content_width + input.padding_x * 2.0)
        .min(input.max_width)
        .min(work_width);
    let height = (content_height + input.padding_y * 2.0).min(work_height);
    let below = input.caret.y + input.caret_height;
    let mut placement = input.placement;
    if placement == Placement::Unlocked {
        placement = if below + height <= input.work_area.bottom {
            Placement::Below
        } else {
            Placement::Above
        };
    }
    let top = if placement == Placement::Below {
        below
    } else {
        input.caret.y - height
    }
    .clamp(input.work_area.top, input.work_area.bottom - height);
    let left = input
        .caret
        .x
        .clamp(input.work_area.left, input.work_area.right - width);
    result.window = Rect {
        left,
        top,
        right: left + width,
        bottom: top + height,
    };
    result.placement = placement;
    let mut x = left + input.padding_x;
    let mut y = top + input.padding_y;
    for item in &input.items {
        let item_width = item.width.min((width - input.padding_x * 2.0).max(0.0));
        result.items.push(Rect {
            left: x,
            top: y,
            right: x + item_width,
            bottom: y + item.height,
        });
        result.item_indices.push(result.item_indices.len());
        if input.orientation == Orientation::Vertical {
            y += item.height + input.row_gap;
        } else {
            x += item.width + input.column_gap;
        }
    }
    result
}

#[derive(Clone, Debug)]
pub struct PocCandidate {
    pub label: String,
    pub text: String,
    pub comment: String,
}

#[derive(Clone, Debug)]
pub struct PocScenario {
    pub name: &'static str,
    pub host: &'static str,
    pub locale: &'static str,
    pub dpi_scale: f32,
    pub popup_allowed: bool,
    pub presentation: PresentationOrientation,
    pub identity: CompositionIdentity,
    pub caret: Point,
    pub work_area: Rect,
    pub candidates: Vec<PocCandidate>,
    pub selected: usize,
    pub expected_orientation: Orientation,
}

#[derive(Clone, Debug)]
pub struct PocScenarioEvidence {
    pub name: &'static str,
    pub host: &'static str,
    pub locale: &'static str,
    pub dpi_scale: f32,
    pub popup_allowed: bool,
    pub orientation: Orientation,
    pub placement: Placement,
    pub window: Rect,
    pub visible_candidates: usize,
    pub selected: usize,
    pub accessibility_name: String,
    pub color_font_candidate_present: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidatePreviewPaintItem {
    pub text: String,
    pub bounds: Rect,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidatePreviewPaintPlan {
    pub dpi_scale: f32,
    pub background_color: u32,
    pub selected_background_color: u32,
    pub text_color: u32,
    pub selected_text_color: u32,
    pub items: Vec<CandidatePreviewPaintItem>,
}

pub fn run_candidate_poc_self_check() -> Result<String, String> {
    let mut evidence = Vec::new();
    for scenario in candidate_poc_scenarios() {
        evidence.push(check_candidate_poc_scenario(&scenario)?);
    }
    Ok(render_candidate_poc_report(&evidence))
}

pub fn candidate_preview_paint_plan(
    dpi_scale: f32,
    width: f32,
    height: f32,
) -> Result<CandidatePreviewPaintPlan, String> {
    if !dpi_scale.is_finite() || !(0.5..=4.0).contains(&dpi_scale) {
        return Err("candidate preview DPI scale must be finite and within 0.5..=4.0".to_owned());
    }
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err("candidate preview paint plan requires a positive finite surface".to_owned());
    }
    let candidates = [
        poc_candidate("1.", "你", ""),
        poc_candidate("2.", "好", ""),
        poc_candidate("3.", "😀", "emoji"),
    ];
    let item_sizes = candidates
        .iter()
        .map(|candidate| measure_candidate(candidate, dpi_scale))
        .collect::<Vec<_>>();
    let result = layout(&LayoutInput {
        orientation: Orientation::Horizontal,
        items: item_sizes,
        caret: Point { x: 0.0, y: 0.0 },
        caret_height: 0.0,
        work_area: Rect {
            left: 0.0,
            top: 0.0,
            right: width,
            bottom: height,
        },
        max_width: width,
        padding_x: 8.0 * dpi_scale,
        padding_y: 8.0 * dpi_scale,
        row_gap: 4.0 * dpi_scale,
        column_gap: 8.0 * dpi_scale,
        placement: Placement::Below,
        scroll_mode: false,
        scroll_columns: 6,
        scroll_visible_rows: 6,
        selected: 0,
        scroll_cell_width: 96.0 * dpi_scale,
    });
    if result.items.is_empty() {
        return Err("candidate preview paint plan produced no visible candidates".to_owned());
    }
    let mut items = Vec::with_capacity(result.items.len());
    for (visible_index, (candidate_index, bounds)) in result
        .item_indices
        .iter()
        .copied()
        .zip(result.items.iter().copied())
        .enumerate()
    {
        let candidate = candidates
            .get(candidate_index)
            .ok_or_else(|| "candidate preview paint plan produced an invalid index".to_owned())?;
        if bounds.left < 0.0 || bounds.top < 0.0 || bounds.right > width || bounds.bottom > height {
            return Err("candidate preview paint plan produced an out-of-bounds item".to_owned());
        }
        items.push(CandidatePreviewPaintItem {
            text: format!(
                "{} {} {}",
                candidate.label, candidate.text, candidate.comment
            )
            .trim()
            .to_owned(),
            bounds,
            selected: visible_index == 0,
        });
    }
    Ok(CandidatePreviewPaintPlan {
        dpi_scale,
        background_color: 0x00ee_f3f7,
        selected_background_color: 0x006f_a700,
        text_color: 0x0020_2020,
        selected_text_color: 0x00ff_ffff,
        items,
    })
}

pub fn candidate_poc_scenarios() -> Vec<PocScenario> {
    let work_area = Rect {
        left: 0.0,
        top: 0.0,
        right: 1920.0,
        bottom: 1080.0,
    };
    vec![
        PocScenario {
            name: "notepad-horizontal-zh",
            host: "mock-notepad",
            locale: "zh-CN",
            dpi_scale: 1.0,
            popup_allowed: true,
            presentation: PresentationOrientation::Automatic,
            identity: CompositionIdentity {
                engine_epoch: 7,
                context_id: 11,
                composition_id: 101,
            },
            caret: Point { x: 120.0, y: 240.0 },
            work_area,
            candidates: vec![
                poc_candidate("1", "你", ""),
                poc_candidate("2", "好", ""),
                poc_candidate("3", "呢", "particle"),
            ],
            selected: 1,
            expected_orientation: Orientation::Horizontal,
        },
        PocScenario {
            name: "word-vertical-annotation",
            host: "mock-word",
            locale: "zh-CN",
            dpi_scale: 1.25,
            popup_allowed: true,
            presentation: PresentationOrientation::Automatic,
            identity: CompositionIdentity {
                engine_epoch: 7,
                context_id: 12,
                composition_id: 201,
            },
            caret: Point { x: 640.0, y: 420.0 },
            work_area,
            candidates: vec![
                poc_candidate("1", "候选", "annotation-longer-than-compact-budget"),
                poc_candidate("2", "候補", "traditional"),
                poc_candidate("3", "candidate", "latin fallback"),
            ],
            selected: 0,
            expected_orientation: Orientation::Vertical,
        },
        PocScenario {
            name: "chromium-right-edge-dpi200",
            host: "mock-chromium",
            locale: "zh-CN",
            dpi_scale: 2.0,
            popup_allowed: true,
            presentation: PresentationOrientation::Automatic,
            identity: CompositionIdentity {
                engine_epoch: 7,
                context_id: 13,
                composition_id: 301,
            },
            caret: Point {
                x: 1880.0,
                y: 700.0,
            },
            work_area,
            candidates: vec![
                poc_candidate("1", "边缘", ""),
                poc_candidate("2", "edge", ""),
            ],
            selected: 0,
            expected_orientation: Orientation::Vertical,
        },
        PocScenario {
            name: "vscode-uiless-accessibility",
            host: "mock-vscode-uiless",
            locale: "en-US",
            dpi_scale: 1.5,
            popup_allowed: false,
            presentation: PresentationOrientation::Vertical,
            identity: CompositionIdentity {
                engine_epoch: 7,
                context_id: 14,
                composition_id: 401,
            },
            caret: Point { x: 320.0, y: 300.0 },
            work_area,
            candidates: vec![
                poc_candidate("1", "hello", ""),
                poc_candidate("2", "world", ""),
            ],
            selected: 1,
            expected_orientation: Orientation::Vertical,
        },
        PocScenario {
            name: "emoji-color-font",
            host: "mock-emoji-host",
            locale: "zh-CN",
            dpi_scale: 1.0,
            popup_allowed: true,
            presentation: PresentationOrientation::Horizontal,
            identity: CompositionIdentity {
                engine_epoch: 7,
                context_id: 15,
                composition_id: 501,
            },
            caret: Point { x: 180.0, y: 360.0 },
            work_area,
            candidates: vec![
                poc_candidate("1", "😀", "emoji"),
                poc_candidate("2", "🏳️‍🌈", "zwj color glyph"),
                poc_candidate("3", "候选", "text fallback"),
            ],
            selected: 0,
            expected_orientation: Orientation::Horizontal,
        },
    ]
}

fn check_candidate_poc_scenario(scenario: &PocScenario) -> Result<PocScenarioEvidence, String> {
    if scenario.candidates.is_empty() {
        return Err(format!("{} has no candidates", scenario.name));
    }
    if scenario.selected >= scenario.candidates.len() {
        return Err(format!("{} has invalid selected index", scenario.name));
    }

    let mut state = CompositionLayoutState::default();
    state.begin(scenario.identity);
    let orientation = state.resolve_orientation(
        scenario.presentation,
        AutomaticOrientationInput {
            candidates: &scenario
                .candidates
                .iter()
                .map(|candidate| CandidateText {
                    text: candidate.text.clone(),
                    comment: candidate.comment.clone(),
                })
                .collect::<Vec<_>>(),
            locale: scenario.locale,
            work_area: scenario.work_area,
            caret_x: scenario.caret.x,
            scale: scenario.dpi_scale,
            page_size: 9,
        },
    );
    if orientation != scenario.expected_orientation {
        return Err(format!(
            "{} orientation mismatch: got {:?}, expected {:?}",
            scenario.name, orientation, scenario.expected_orientation
        ));
    }

    let item_sizes = scenario
        .candidates
        .iter()
        .map(|candidate| measure_candidate(candidate, scenario.dpi_scale))
        .collect::<Vec<_>>();
    let result = layout(&LayoutInput {
        orientation,
        items: item_sizes,
        caret: scenario.caret,
        caret_height: 24.0 * scenario.dpi_scale,
        work_area: scenario.work_area,
        max_width: 720.0 * scenario.dpi_scale,
        padding_x: 8.0 * scenario.dpi_scale,
        padding_y: 6.0 * scenario.dpi_scale,
        row_gap: 2.0 * scenario.dpi_scale,
        column_gap: 8.0 * scenario.dpi_scale,
        selected: scenario.selected,
        ..LayoutInput::default()
    });
    if result.items.len() != scenario.candidates.len() {
        return Err(format!(
            "{} visible candidate count mismatch: got {}, expected {}",
            scenario.name,
            result.items.len(),
            scenario.candidates.len()
        ));
    }
    if !rect_inside(result.window, scenario.work_area) {
        return Err(format!("{} window is outside work area", scenario.name));
    }
    for (left_index, left) in result.items.iter().enumerate() {
        if !rect_inside(*left, result.window) {
            return Err(format!(
                "{} candidate {left_index} is outside window",
                scenario.name
            ));
        }
        for (right_index, right) in result.items.iter().enumerate().skip(left_index + 1) {
            if rects_overlap(*left, *right) {
                return Err(format!(
                    "{} candidate rectangles overlap: {left_index} and {right_index}",
                    scenario.name
                ));
            }
        }
    }

    let selected = &scenario.candidates[scenario.selected];
    let accessibility_name = format!("{} {} {}", selected.label, selected.text, selected.comment)
        .trim()
        .to_owned();
    if accessibility_name.is_empty() {
        return Err(format!(
            "{} selected candidate has no accessibility name",
            scenario.name
        ));
    }

    Ok(PocScenarioEvidence {
        name: scenario.name,
        host: scenario.host,
        locale: scenario.locale,
        dpi_scale: scenario.dpi_scale,
        popup_allowed: scenario.popup_allowed,
        orientation,
        placement: result.placement,
        window: result.window,
        visible_candidates: result.items.len(),
        selected: scenario.selected,
        accessibility_name,
        color_font_candidate_present: scenario
            .candidates
            .iter()
            .any(|candidate| contains_non_bmp_or_zwj(&candidate.text)),
    })
}

fn render_candidate_poc_report(evidence: &[PocScenarioEvidence]) -> String {
    let mut output = String::from(
        "{\n  \"component\":\"fcitx5-candidate-poc\",\n  \"kind\":\"rust-out-of-process-headless-poc\",\n  \"cpp_ffi\":false,\n  \"send_input\":false,\n  \"global_hooks\":false,\n  \"process_injection\":false,\n  \"scenarios\":[\n",
    );
    for (index, item) in evidence.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        output.push_str("    {");
        push_json_field(&mut output, "name", item.name, false);
        push_json_field(&mut output, "host", item.host, true);
        push_json_field(&mut output, "locale", item.locale, true);
        push_json_number(&mut output, "dpi_scale", item.dpi_scale, true);
        push_json_bool(&mut output, "popup_allowed", item.popup_allowed, true);
        push_json_field(
            &mut output,
            "orientation",
            orientation_name(item.orientation),
            true,
        );
        push_json_field(
            &mut output,
            "placement",
            placement_name(item.placement),
            true,
        );
        push_json_number(&mut output, "window_left", item.window.left, true);
        push_json_number(&mut output, "window_top", item.window.top, true);
        push_json_number(&mut output, "window_right", item.window.right, true);
        push_json_number(&mut output, "window_bottom", item.window.bottom, true);
        push_json_usize(
            &mut output,
            "visible_candidates",
            item.visible_candidates,
            true,
        );
        push_json_usize(&mut output, "selected", item.selected, true);
        push_json_field(
            &mut output,
            "accessibility_name",
            &item.accessibility_name,
            true,
        );
        push_json_bool(
            &mut output,
            "color_font_candidate_present",
            item.color_font_candidate_present,
            true,
        );
        output.push('}');
    }
    output.push_str("\n  ],\n  \"result\":\"PASS\"\n}");
    output
}

fn poc_candidate(label: &str, text: &str, comment: &str) -> PocCandidate {
    PocCandidate {
        label: label.to_owned(),
        text: text.to_owned(),
        comment: comment.to_owned(),
    }
}

fn measure_candidate(candidate: &PocCandidate, scale: f32) -> Size {
    let label = text_units(&candidate.label) * 10.0 * scale;
    let text = text_units(&candidate.text) * 18.0 * scale;
    let comment = text_units(&candidate.comment) * 14.0 * scale;
    Size {
        width: (label + 14.0 * scale + text + comment + 48.0 * scale).max(64.0 * scale),
        height: 42.0 * scale,
    }
}

fn text_units(value: &str) -> f32 {
    value
        .chars()
        .map(|character| if character.len_utf8() == 1 { 0.6 } else { 1.0 })
        .sum::<f32>()
}

fn rect_inside(inner: Rect, outer: Rect) -> bool {
    inner.left >= outer.left - 0.01
        && inner.top >= outer.top - 0.01
        && inner.right <= outer.right + 0.01
        && inner.bottom <= outer.bottom + 0.01
        && inner.right >= inner.left
        && inner.bottom >= inner.top
}

fn rects_overlap(left: Rect, right: Rect) -> bool {
    left.left < right.right - 0.01
        && left.right > right.left + 0.01
        && left.top < right.bottom - 0.01
        && left.bottom > right.top + 0.01
}

fn contains_non_bmp_or_zwj(value: &str) -> bool {
    value
        .chars()
        .any(|character| character as u32 > 0xFFFF || character == '\u{200d}')
}

fn orientation_name(value: Orientation) -> &'static str {
    match value {
        Orientation::Vertical => "vertical",
        Orientation::Horizontal => "horizontal",
    }
}

fn placement_name(value: Placement) -> &'static str {
    match value {
        Placement::Unlocked => "unlocked",
        Placement::Below => "below",
        Placement::Above => "above",
    }
}

fn push_json_field(output: &mut String, name: &str, value: &str, comma: bool) {
    if comma {
        output.push(',');
    }
    output.push('"');
    output.push_str(name);
    output.push_str("\":\"");
    push_json_escaped(output, value);
    output.push('"');
}

fn push_json_bool(output: &mut String, name: &str, value: bool, comma: bool) {
    if comma {
        output.push(',');
    }
    output.push('"');
    output.push_str(name);
    output.push_str("\":");
    output.push_str(if value { "true" } else { "false" });
}

fn push_json_number(output: &mut String, name: &str, value: f32, comma: bool) {
    if comma {
        output.push(',');
    }
    output.push('"');
    output.push_str(name);
    output.push_str("\":");
    output.push_str(&format!("{value:.2}"));
}

fn push_json_usize(output: &mut String, name: &str, value: usize, comma: bool) {
    if comma {
        output.push(',');
    }
    output.push('"');
    output.push_str(name);
    output.push_str("\":");
    output.push_str(&value.to_string());
}

fn push_json_escaped(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character < ' ' => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
}

fn resolve_automatic_orientation(
    candidates: &[CandidateText],
    locale: &str,
    work_area: Rect,
    caret_x: f32,
    scale: f32,
    page_size: u32,
) -> Orientation {
    let mut has_long_annotation = false;
    let mut compact_candidates = !candidates.is_empty();
    for candidate in candidates {
        has_long_annotation = has_long_annotation || candidate.comment.len() > 18;
        compact_candidates =
            compact_candidates && candidate.text.len() <= 6 && candidate.comment.len() <= 18;
    }
    let near_right_threshold = 360.0 * scale;
    let edge_constrained = work_area.right - caret_x < near_right_threshold;
    let page_size = if page_size == 0 { 9 } else { page_size } as usize;
    let compact_cjk = locale_prefers_compact_horizontal(locale)
        && compact_candidates
        && candidates.len() <= page_size.max(1);
    if compact_cjk && !has_long_annotation && !edge_constrained {
        Orientation::Horizontal
    } else {
        Orientation::Vertical
    }
}

fn locale_prefers_compact_horizontal(locale: &str) -> bool {
    let lower = locale.to_ascii_lowercase();
    lower.starts_with("zh") || lower.starts_with("ja") || lower.starts_with("ko")
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
        let first = unsafe { &*output.candidates };
        assert_eq!(
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
        let required =
            unsafe { fcitx5_candidate_default_dwrite_locale_utf16(std::ptr::null_mut(), 0) };
        assert!(required > 0);
        let mut output = vec![0_u16; required];
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
            unsafe {
                fcitx5_candidate_content_locale_valid_utf8(Fcitx5CandidateUtf8 {
                    ptr: b"ko-KR".as_ptr(),
                    len: b"ko-KR".len(),
                })
            },
            1
        );
        assert_eq!(
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
        let required = unsafe {
            fcitx5_candidate_content_locale_or_default_utf16(locale, std::ptr::null_mut(), 0)
        };
        assert_eq!(required, wide("en-US").len());
        let mut output = vec![0_u16; required];
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
        assert_eq!(
            model.current.as_ref().expect("current").popup_allowed,
            false
        );
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

        let long = state.stable_window_width(420.0, 720.0);
        let short = state.stable_window_width(140.0, 720.0);
        let longer = state.stable_window_width(500.0, 720.0);
        assert_eq!(long, 420.0);
        assert_eq!(short, 420.0);
        assert_eq!(longer, 500.0);

        state.begin(CompositionIdentity {
            engine_epoch: 1,
            context_id: 2,
            composition_id: 4,
        });
        assert_eq!(state.stable_window_width(140.0, 720.0), 140.0);
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
        assert_eq!(output.stable_width, 0.0);
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
            unsafe { fcitx5_candidate_presentation_apply(state, &update) },
            0
        );

        let mut indices = [usize::MAX; 12];
        let mut plan = Fcitx5CandidatePresentationRenderPlan::default();
        assert_eq!(
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
            unsafe { fcitx5_candidate_presentation_apply(state, &scroll_update) },
            0
        );
        assert_eq!(
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
        assert_eq!(plan.selected_background_color, 0x006f_a700);
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
