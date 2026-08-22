#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;

const MAX_CANDIDATES: usize = 128;
const MAX_CANDIDATE_TEXT_UTF8: usize = 4096;
const MAX_TRACKED_CONTEXTS: usize = 64;

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
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateRenderItemInput {
    pub bounds: Fcitx5CandidateLayoutRect,
    pub label_width: f32,
    pub text_width: f32,
    pub comment_width: f32,
    pub has_label: u8,
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
#[derive(Clone, Copy, Debug)]
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
struct CandidateModel {
    current: Option<CandidateSnapshot>,
    engine_epoch: u64,
    freshness: HashMap<u64, Freshness>,
    freshness_order: VecDeque<u64>,
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

impl CandidateModel {
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
        self.engine_epoch = 0;
        self.freshness.clear();
        self.freshness_order.clear();
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
    value.len() <= MAX_CANDIDATE_TEXT_UTF8 && !value.contains(&0)
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
/// `items` and `out_items` must be valid for `item_count` elements when
/// `item_count` is non-zero. Pointers are not retained.
pub unsafe extern "C" fn fcitx5_candidate_render_segments(
    items: *const Fcitx5CandidateRenderItemInput,
    item_count: usize,
    horizontal_layout: u8,
    scroll_mode: u8,
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
    let label_column_width = items.iter().fold(0.0_f32, |width, item| {
        if item.has_label != 0 {
            width.max(item.label_width)
        } else {
            width
        }
    });
    unsafe {
        *out_label_column_width = label_column_width;
    }
    for (source, target) in items.iter().zip(out_items.iter_mut()) {
        let reserve_label = !(scroll_mode != 0 && horizontal_layout == 0 && source.has_label == 0);
        let effective_label_column_width = if reserve_label {
            label_column_width
        } else {
            0.0
        };
        let text_left = source.bounds.left + effective_label_column_width;
        let text_available = (source.bounds.right - text_left).max(1.0);
        let text_draw_width = source.text_width.max(0.0).min(text_available);
        let comment_left = text_left + text_draw_width + 4.0;
        let comment_available = (source.bounds.right - comment_left).max(1.0);
        let draw_comment = u8::from(
            source.comment_width <= 0.0 || source.comment_width <= comment_available + 2.0,
        );
        *target = Fcitx5CandidateRenderItemOutput {
            label: Fcitx5CandidateLayoutRect {
                left: source.bounds.left,
                top: source.bounds.top,
                right: source.bounds.left + label_column_width,
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
        };
    }
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

pub fn run_candidate_poc_self_check() -> Result<String, String> {
    let mut evidence = Vec::new();
    for scenario in candidate_poc_scenarios() {
        evidence.push(check_candidate_poc_scenario(&scenario)?);
    }
    Ok(render_candidate_poc_report(&evidence))
}

fn candidate_poc_scenarios() -> Vec<PocScenario> {
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
    let label = text_units(&candidate.label) * 8.0 * scale;
    let text = text_units(&candidate.text) * 16.0 * scale;
    let comment = text_units(&candidate.comment) * 9.0 * scale;
    Size {
        width: (label + 10.0 * scale + text + comment + 24.0 * scale).max(40.0 * scale),
        height: 34.0 * scale,
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

    fn width(result: &LayoutResult) -> f32 {
        result.window.right - result.window.left
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
    fn render_segments_match_label_column_and_comment_contract() {
        let input = [
            Fcitx5CandidateRenderItemInput {
                bounds: Fcitx5CandidateLayoutRect {
                    left: 10.0,
                    top: 20.0,
                    right: 210.0,
                    bottom: 48.0,
                },
                label_width: 18.0,
                text_width: 80.0,
                comment_width: 40.0,
                has_label: 1,
            },
            Fcitx5CandidateRenderItemInput {
                bounds: Fcitx5CandidateLayoutRect {
                    left: 10.0,
                    top: 52.0,
                    right: 110.0,
                    bottom: 80.0,
                },
                label_width: 10.0,
                text_width: 96.0,
                comment_width: 20.0,
                has_label: 1,
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
        assert_eq!(label_column, 18.0);
        assert_eq!(output[0].text.left, 28.0);
        assert_eq!(output[0].comment.left, 112.0);
        assert_eq!(output[0].draw_comment, 1);
        assert_eq!(output[1].text.left, 28.0);
        assert_eq!(output[1].comment.left, 114.0);
        assert_eq!(output[1].draw_comment, 0);

        let no_label = [Fcitx5CandidateRenderItemInput {
            bounds: input[0].bounds,
            label_width: 18.0,
            text_width: 80.0,
            comment_width: 0.0,
            has_label: 0,
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
}
