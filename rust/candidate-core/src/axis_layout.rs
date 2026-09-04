//! Three-axis candidate geometry (design docs/tasks/080-layout-naming-design.md):
//! orientation x overflow x writing-mode produce one deterministic rect set per
//! visible page/viewport. The renderer receives only per-item rects plus a
//! visibility flag, an optional viewport offset, and an optional per-item writing
//! token, so painting has a single path that never re-branches on the layout mode.
#![forbid(unsafe_code)]
//!
//! The three enums mirror the frozen config-core model on purpose: this crate
//! is the geometry engine and must not depend on the config/serde crate. The
//! config boundary decodes legacy strings into the same three-axis shape and
//! maps it here (see `CandidateLayoutOptions` below for the exact mirror).

use crate::{flow_paged_bounds, vertical_text_columns};
use crate::{Orientation, Placement, Point, Rect, Size};

/// What happens when candidates exceed the visible space (mirror of
/// `fcitx5_config_core::OverflowBehavior`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverflowBehavior {
    /// A page holds at most `page_size` candidates; navigation pages.
    Paging,
    /// A fixed-size viewport scrolls; the highlighted candidate stays visible.
    Scrolling,
    /// Real measured widths wrap onto further rows until the row budget is
    /// reached, then the rest pages.
    Wrapping,
}

impl Default for OverflowBehavior {
    fn default() -> Self {
        Self::Paging
    }
}

/// The candidate text's own direction, independent of item arrangement
/// (mirror of `fcitx5_config_core::WritingMode`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WritingMode {
    /// Horizontal glyph runs left-to-right.
    Horizontal,
    /// Vertical typesetting with columns ordered right-to-left.
    VerticalRl,
    /// Vertical typesetting with columns ordered left-to-right.
    VerticalLr,
}

impl Default for WritingMode {
    fn default() -> Self {
        Self::Horizontal
    }
}

/// The unified three-axis layout model consumed by the geometry engine
/// (mirror of `fcitx5_config_core::CandidateLayoutOptions`; the arrangement
/// axis uses this crate's `Orientation`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateLayoutOptions {
    pub orientation: Orientation,
    pub overflow: OverflowBehavior,
    pub writing_mode: WritingMode,
}

impl Default for CandidateLayoutOptions {
    fn default() -> Self {
        Self {
            orientation: Orientation::Horizontal,
            overflow: OverflowBehavior::Paging,
            writing_mode: WritingMode::Horizontal,
        }
    }
}

impl CandidateLayoutOptions {
    /// Arranges `candidates` with the effective `overflow` behavior.
    #[must_use]
    pub fn with_overflow(mut self, overflow: OverflowBehavior) -> Self {
        self.overflow = overflow;
        self
    }

    /// Sets the text writing mode without touching the arrangement axis.
    #[must_use]
    pub fn with_writing(mut self, writing_mode: WritingMode) -> Self {
        self.writing_mode = writing_mode;
        self
    }
}

/// All geometry inputs for one layout pass. Item sizes carry measured width
/// and height (already DPI scaled); column width is the glyph advance for
/// vertical writing and the row cell budget otherwise.
#[derive(Clone, Debug)]
pub struct AxisLayoutInput {
    pub options: CandidateLayoutOptions,
    pub items: Vec<Size>,
    pub caret: Point,
    pub caret_height: f32,
    pub work_area: Rect,
    /// Hard window-width budget (0 = work-area width).
    pub max_width: f32,
    /// Hard window-height/viewport budget (0 = work-area height).
    pub max_height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub row_gap: f32,
    pub column_gap: f32,
    /// Paging capacity. 0 = no cap (page holds every given candidate).
    pub page_size: usize,
    /// Highlighted candidate; Scrolling keeps it scroll-into-view.
    pub selected: usize,
    /// Manual scroll override for Scrolling overflow (DIP px).
    /// `None` = auto scroll-into-view; `Some(px)` = use this offset
    /// clamped to [0, scroll_max]. Used by WM_MOUSEWHEEL.
    pub scroll_override: Option<f32>,
    pub placement: Placement,
}

impl Default for AxisLayoutInput {
    fn default() -> Self {
        Self {
            options: CandidateLayoutOptions::default(),
            items: Vec::new(),
            caret: Point::default(),
            caret_height: 0.0,
            work_area: Rect::default(),
            max_width: 720.0,
            max_height: 0.0,
            padding_x: 8.0,
            padding_y: 6.0,
            row_gap: 2.0,
            column_gap: 8.0,
            page_size: 0,
            selected: 0,
            scroll_override: None,
            placement: Placement::Unlocked,
        }
    }
}

/// One laid-out candidate for the single paint path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisLayoutItem {
    /// Final rectangle in window coordinates (viewport scroll already applied).
    pub rect: Rect,
    /// Whether the rectangle intersects the window and should be painted.
    pub visible: bool,
    /// The candidate text's writing direction token.
    pub writing: WritingMode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AxisLayoutResult {
    /// The window/viewport rectangle (already placement-clamped to work area).
    pub window: Rect,
    pub placement: Placement,
    /// Laid-out candidates in input order (the visible page or content range).
    pub items: Vec<AxisLayoutItem>,
    /// Scroll amount applied to the content (window shows content at this
    /// natural offset). `None` for paging/wrapping layouts.
    pub viewport_offset: Option<(f32, f32)>,
    /// Natural content size (padding included) before viewport clipping; the
    /// caller uses it for scrollbar/wheel extent without re-laying out.
    pub content_size: Size,
    /// First candidate index whose rect intersects the window.
    pub first_visible: usize,
}

impl Default for AxisLayoutResult {
    fn default() -> Self {
        Self {
            window: Rect::default(),
            placement: Placement::Unlocked,
            items: Vec::new(),
            viewport_offset: None,
            content_size: Size::default(),
            first_visible: 0,
        }
    }
}

/// Single entry point for the effective three-axis combinations. Branches only
/// on the model, never on paint-time state, and returns only geometry.
#[must_use]
pub fn layout(input: &AxisLayoutInput) -> AxisLayoutResult {
    if input.items.is_empty() {
        return AxisLayoutResult::default();
    }
    let options = input.options;
    if options.writing_mode != WritingMode::Horizontal {
        return vertical_columns(input, options);
    }
    match (options.orientation, options.overflow) {
        (Orientation::Horizontal, OverflowBehavior::Paging) => paged_horizontal(input, options),
        (Orientation::Vertical, OverflowBehavior::Paging) => paged_vertical(input, options),
        (Orientation::Horizontal, OverflowBehavior::Scrolling) => scroll_horizontal(input, options),
        (Orientation::Vertical, OverflowBehavior::Scrolling) => scroll_vertical(input, options),
        (Orientation::Horizontal, OverflowBehavior::Wrapping) => wrapped(input, options),
        // Vertical + Wrapping has no displayed semantics (frozen design matrix).
        (Orientation::Vertical, OverflowBehavior::Wrapping) => AxisLayoutResult::default(),
    }
}

/// Number of candidates a page holds (0 page_size = unbounded).
fn page_capacity(input: &AxisLayoutInput) -> usize {
    if input.page_size == 0 {
        input.items.len()
    } else {
        input.page_size
    }
}

/// Hard width budget in window pixels (never larger than the work area).
fn width_budget(input: &AxisLayoutInput) -> f32 {
    let work = (input.work_area.right - input.work_area.left).max(0.0);
    if input.max_width > 0.0 {
        input.max_width.min(work)
    } else {
        work
    }
}

/// Hard height/viewport budget (defaults to the work-area height).
fn height_budget(input: &AxisLayoutInput) -> f32 {
    let work = (input.work_area.bottom - input.work_area.top).max(0.0);
    if input.max_height > 0.0 {
        input.max_height.min(work)
    } else {
        work
    }
}

fn clamped_selected(input: &AxisLayoutInput) -> usize {
    input.selected.min(input.items.len() - 1)
}

/// Places the window below/above the caret and clamps it into the work area.
/// Width/height must already be capped to the work area.
fn place_window(input: &AxisLayoutInput, width: f32, height: f32) -> (Rect, Placement) {
    let work = input.work_area;
    let below = input.caret.y + input.caret_height;
    let mut placement = input.placement;
    if placement == Placement::Unlocked {
        placement = if below + height <= work.bottom {
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
    .clamp(work.top, (work.bottom - height).max(work.top));
    let left = input
        .caret
        .x
        .clamp(work.left, (work.right - width).max(work.left));
    (
        Rect {
            left,
            top,
            right: left + width,
            bottom: top + height,
        },
        placement,
    )
}

/// True when the item rectangle has positive area inside the window.
fn intersects(rect: Rect, window: Rect) -> bool {
    rect.left < window.right
        && rect.right > window.left
        && rect.top < window.bottom
        && rect.bottom > window.top
}

/// Horizontal + Paging: one row, at most `page_size` candidates, capped again
/// so the row never exceeds the width budget (matches the wrap rule at a
/// single row).
fn paged_horizontal(input: &AxisLayoutInput, options: CandidateLayoutOptions) -> AxisLayoutResult {
    let padding_x = input.padding_x.max(0.0);
    let padding_y = input.padding_y.max(0.0);
    let column_gap = input.column_gap.max(0.0);
    let content_budget = (width_budget(input) - padding_x * 2.0).max(0.0);
    let mut count = 0;
    let mut width = 0.0_f32;
    let mut row_height = 0.0_f32;
    for item in &input.items[..page_capacity(input).min(input.items.len())] {
        if count > 0 && width + column_gap + item.width > content_budget {
            break;
        }
        if count > 0 {
            width += column_gap;
        }
        width += item.width;
        row_height = row_height.max(item.height);
        count += 1;
    }
    let window_width = (width + padding_x * 2.0).min(width_budget(input));
    let window_height = (row_height + padding_y * 2.0).min(height_budget(input));
    let (window, placement) = place_window(input, window_width, window_height);
    let mut rects = Vec::with_capacity(count);
    let mut x = window.left + padding_x;
    for item in input.items.iter().take(count) {
        rects.push(Rect {
            left: x,
            top: window.top + padding_y,
            right: x + item.width,
            bottom: window.top + padding_y + item.height,
        });
        x += item.width + column_gap;
    }
    AxisLayoutResult {
        window,
        placement,
        items: rects
            .into_iter()
            .map(|rect| AxisLayoutItem {
                rect,
                visible: true,
                writing: options.writing_mode,
            })
            .collect(),
        viewport_offset: None,
        content_size: Size {
            width: window_width,
            height: window_height,
        },
        first_visible: 0,
    }
}

/// Vertical + Paging: row-per-candidate list with `page_size` capacity.
fn paged_vertical(input: &AxisLayoutInput, options: CandidateLayoutOptions) -> AxisLayoutResult {
    let padding_x = input.padding_x.max(0.0);
    let padding_y = input.padding_y.max(0.0);
    let row_gap = input.row_gap.max(0.0);
    let count = page_capacity(input).min(input.items.len());
    let mut width = 0.0_f32;
    let mut height = 0.0_f32;
    for item in input.items.iter().take(count) {
        width = width.max(item.width);
        if height > 0.0 {
            height += row_gap;
        }
        height += item.height;
    }
    let window_width = (width + padding_x * 2.0).min(width_budget(input));
    let window_height = (height + padding_y * 2.0).min(height_budget(input));
    let (window, placement) = place_window(input, window_width, window_height);
    let mut y = window.top + padding_y;
    let mut items = Vec::with_capacity(count);
    for item in input.items.iter().take(count) {
        let rect = Rect {
            left: window.left + padding_x,
            top: y,
            right: window.left + padding_x + item.width,
            bottom: y + item.height,
        };
        items.push(AxisLayoutItem {
            rect,
            visible: true,
            writing: options.writing_mode,
        });
        y += item.height + row_gap;
    }
    AxisLayoutResult {
        window,
        placement,
        items,
        viewport_offset: None,
        content_size: Size {
            width: window_width,
            height: window_height,
        },
        first_visible: 0,
    }
}

/// Horizontal + Scrolling: one row in a fixed-width viewport. When content is
/// wider than the viewport, the highlight scrolls into view and every item
/// rect is shifted by the horizontal offset; items outside the viewport keep
/// their rect but are flagged not visible.
fn scroll_horizontal(input: &AxisLayoutInput, options: CandidateLayoutOptions) -> AxisLayoutResult {
    let padding_x = input.padding_x.max(0.0);
    let padding_y = input.padding_y.max(0.0);
    let column_gap = input.column_gap.max(0.0);
    let mut width = 0.0_f32;
    let mut row_height = 0.0_f32;
    let mut offsets = Vec::with_capacity(input.items.len());
    for item in &input.items {
        offsets.push(width);
        if width > 0.0 {
            width += column_gap;
        }
        width += item.width;
        row_height = row_height.max(item.height);
    }
    let natural_width = width + padding_x * 2.0;
    let natural_height = row_height + padding_y * 2.0;
    let viewport = width_budget(input);
    let window_width = natural_width.min(viewport);
    let window_height = natural_height.min(height_budget(input));
    let (window, placement) = place_window(input, window_width, window_height);
    let selected = clamped_selected(input);
    let selected_rect = |offset: f32| Rect {
        left: window.left + padding_x + offset,
        top: window.top + padding_y,
        right: window.left + padding_x + offset + input.items[selected].width,
        bottom: window.top + padding_y + row_height,
    };
    let scroll_max = (natural_width - window_width).max(0.0);
    let dx = if scroll_max > 0.0 {
        let natural = selected_rect(offsets[selected]);
        let mut scroll = 0.0_f32;
        if natural.left < window.left {
            scroll = natural.left - window.left;
        }
        if natural.right - scroll > window.right {
            scroll = natural.right - window.right;
        }
        scroll.clamp(0.0, scroll_max)
    } else {
        0.0
    };
    let dx = input
        .scroll_override
        .map(|o| o.clamp(0.0, scroll_max))
        .unwrap_or(dx);
    let mut items = Vec::with_capacity(input.items.len());
    let mut first_visible = usize::MAX;
    for (index, (offset, item)) in offsets.into_iter().zip(input.items.iter()).enumerate() {
        let rect = Rect {
            left: window.left + padding_x + offset - dx,
            top: window.top + padding_y,
            right: window.left + padding_x + offset + item.width - dx,
            bottom: window.top + padding_y + item.height,
        };
        if first_visible == usize::MAX && intersects(rect, window) {
            first_visible = index;
        }
        items.push(AxisLayoutItem {
            rect,
            visible: intersects(rect, window),
            writing: options.writing_mode,
        });
    }
    AxisLayoutResult {
        window,
        placement,
        items,
        viewport_offset: (scroll_max > 0.0).then_some((dx, 0.0)),
        content_size: Size {
            width: natural_width,
            height: natural_height,
        },
        first_visible: if first_visible == usize::MAX {
            0
        } else {
            first_visible
        },
    }
}

/// Vertical + Scrolling: row-per-candidate list inside a fixed-height
/// viewport; the highlight scrolls into view along the vertical axis.
fn scroll_vertical(input: &AxisLayoutInput, options: CandidateLayoutOptions) -> AxisLayoutResult {
    let padding_x = input.padding_x.max(0.0);
    let padding_y = input.padding_y.max(0.0);
    let row_gap = input.row_gap.max(0.0);
    let mut width = 0.0_f32;
    let mut height = 0.0_f32;
    let mut offsets = Vec::with_capacity(input.items.len());
    for item in &input.items {
        offsets.push(height);
        if height > 0.0 {
            height += row_gap;
        }
        height += item.height;
        width = width.max(item.width);
    }
    let natural_width = width + padding_x * 2.0;
    let natural_height = height + padding_y * 2.0;
    let viewport = height_budget(input);
    let window_width = natural_width.min(width_budget(input));
    let window_height = natural_height.min(viewport);
    let (window, placement) = place_window(input, window_width, window_height);
    let selected = clamped_selected(input);
    let selected_rect = |offset: f32| Rect {
        left: window.left + padding_x,
        top: window.top + padding_y + offset,
        right: window.left + padding_x + input.items[selected].width,
        bottom: window.top + padding_y + offset + input.items[selected].height,
    };
    let scroll_max = (natural_height - window_height).max(0.0);
    let dy = if scroll_max > 0.0 {
        let natural = selected_rect(offsets[selected]);
        let mut scroll = 0.0_f32;
        if natural.top < window.top {
            scroll = natural.top - window.top;
        }
        if natural.bottom - scroll > window.bottom {
            scroll = natural.bottom - window.bottom;
        }
        scroll.clamp(0.0, scroll_max)
    } else {
        0.0
    };
    let dy = input
        .scroll_override
        .map(|o| o.clamp(0.0, scroll_max))
        .unwrap_or(dy);
    let mut items = Vec::with_capacity(input.items.len());
    let mut first_visible = usize::MAX;
    for (index, (offset, item)) in offsets.into_iter().zip(input.items.iter()).enumerate() {
        let rect = Rect {
            left: window.left + padding_x,
            top: window.top + padding_y + offset - dy,
            right: window.left + padding_x + item.width,
            bottom: window.top + padding_y + offset + item.height - dy,
        };
        if first_visible == usize::MAX && intersects(rect, window) {
            first_visible = index;
        }
        items.push(AxisLayoutItem {
            rect,
            visible: intersects(rect, window),
            writing: options.writing_mode,
        });
    }
    AxisLayoutResult {
        window,
        placement,
        items,
        viewport_offset: (scroll_max > 0.0).then_some((0.0, dy)),
        content_size: Size {
            width: natural_width,
            height: natural_height,
        },
        first_visible: if first_visible == usize::MAX {
            0
        } else {
            first_visible
        },
    }
}

/// Horizontal + Wrapping: real measured widths wrap onto rows (same greedy
/// rule as `flow_paged_bounds`); candidates that would exceed the height
/// budget page instead of overflowing the window.
fn wrapped(input: &AxisLayoutInput, options: CandidateLayoutOptions) -> AxisLayoutResult {
    let padding_x = input.padding_x.max(0.0);
    let padding_y = input.padding_y.max(0.0);
    let column_gap = input.column_gap.max(0.0);
    let row_gap = input.row_gap.max(0.0);
    let content_budget = (width_budget(input) - padding_x * 2.0).max(0.0);
    let budget_height = (height_budget(input) - padding_y * 2.0).max(0.0);
    let mut count = input.items.len();
    while count > 1 {
        let (_, _, wrapped_height) =
            flow_paged_bounds(&input.items[..count], content_budget, column_gap, row_gap);
        if wrapped_height <= budget_height {
            break;
        }
        count -= 1;
    }
    if count == 0 {
        count = 1;
    }
    let (mut rects, wrapped_width, wrapped_height) =
        flow_paged_bounds(&input.items[..count], content_budget, column_gap, row_gap);
    let window_width = (wrapped_width + padding_x * 2.0).min(width_budget(input));
    let window_height = (wrapped_height + padding_y * 2.0).min(height_budget(input));
    let (window, placement) = place_window(input, window_width, window_height);
    for rect in &mut rects {
        rect.left += window.left + padding_x;
        rect.right += window.left + padding_x;
        rect.top += window.top + padding_y;
        rect.bottom += window.top + padding_y;
    }
    AxisLayoutResult {
        window,
        placement,
        items: rects
            .into_iter()
            .map(|rect| AxisLayoutItem {
                rect,
                visible: true,
                writing: options.writing_mode,
            })
            .collect(),
        viewport_offset: None,
        content_size: Size {
            width: window_width,
            height: window_height,
        },
        first_visible: 0,
    }
}

/// Vertical writing (VerticalRl/VerticalLr): every candidate is one column of
/// top-to-bottom text; `vertical_text_columns` orders the columns left-to-right
/// (VerticalLr) or right-to-left (VerticalRl). Paging/Wrapping cap the page at
/// `page_size` columns; Scrolling keeps every column in a horizontally
/// scrollable viewport. Per-glyph vertical glyph drawing stays a renderer
/// concern; this owns column geometry plus the writing token.
fn vertical_columns(input: &AxisLayoutInput, options: CandidateLayoutOptions) -> AxisLayoutResult {
    let padding_x = input.padding_x.max(0.0);
    let padding_y = input.padding_y.max(0.0);
    let column_gap = input.column_gap.max(0.0);
    let scrolling = options.overflow == OverflowBehavior::Scrolling;
    let count = if scrolling {
        input.items.len()
    } else {
        page_capacity(input).min(input.items.len())
    };
    let left_to_right = options.writing_mode == WritingMode::VerticalLr;
    let (mut rects, columns_width, columns_height) =
        vertical_text_columns(&input.items[..count], column_gap, left_to_right);
    let natural_width = columns_width + padding_x * 2.0;
    let natural_height = columns_height + padding_y * 2.0;
    let viewport = width_budget(input);
    let window_width = natural_width.min(viewport);
    let window_height = natural_height.min(height_budget(input));
    let (window, placement) = place_window(input, window_width, window_height);
    let selected = clamped_selected(input).min(count - 1);
    let selected_rect = |offset: f32| Rect {
        left: window.left + padding_x + offset,
        top: window.top + padding_y,
        right: window.left + padding_x + offset + input.items[selected].width,
        bottom: window.top + padding_y + input.items[selected].height,
    };
    let scroll_max = (natural_width - window_width).max(0.0);
    let dx = if scrolling && scroll_max > 0.0 {
        let natural = selected_rect(rects[selected].left - window.left - padding_x);
        let mut scroll = 0.0_f32;
        if natural.left < window.left {
            scroll = natural.left - window.left;
        }
        if natural.right - scroll > window.right {
            scroll = natural.right - window.right;
        }
        scroll.clamp(0.0, scroll_max)
    } else {
        0.0
    };
    let dx = input
        .scroll_override
        .map(|o| o.clamp(0.0, scroll_max))
        .unwrap_or(dx);
    for rect in &mut rects {
        rect.left += window.left + padding_x - dx;
        rect.right += window.left + padding_x - dx;
        rect.top += window.top + padding_y;
        rect.bottom += window.top + padding_y;
    }
    let mut items = Vec::with_capacity(rects.len());
    let mut first_visible = usize::MAX;
    for (index, rect) in rects.into_iter().enumerate() {
        if first_visible == usize::MAX && intersects(rect, window) {
            first_visible = index;
        }
        items.push(AxisLayoutItem {
            rect,
            visible: intersects(rect, window),
            writing: options.writing_mode,
        });
    }
    AxisLayoutResult {
        window,
        placement,
        items,
        viewport_offset: if scrolling && scroll_max > 0.0 {
            Some((dx, 0.0))
        } else {
            None
        },
        content_size: Size {
            width: natural_width,
            height: natural_height,
        },
        first_visible: if first_visible == usize::MAX {
            0
        } else {
            first_visible
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(width: f32, height: f32) -> Size {
        Size { width, height }
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    /// Standard landscape work area plus a caret below-center; most tests
    /// override budgets to force overflow.
    fn input(items: Vec<Size>) -> AxisLayoutInput {
        AxisLayoutInput {
            items,
            caret: Point { x: 50.0, y: 300.0 },
            caret_height: 20.0,
            work_area: Rect {
                left: 0.0,
                top: 0.0,
                right: 800.0,
                bottom: 600.0,
            },
            max_width: 0.0,
            max_height: 0.0,
            ..AxisLayoutInput::default()
        }
    }

    fn options() -> CandidateLayoutOptions {
        CandidateLayoutOptions::default()
    }

    #[test]
    fn paging_horizontal_lays_single_row_capped_at_page_size() {
        let input = input(vec![item(60.0, 20.0); 8]);
        let mut input = input;
        input.options = options();
        input.page_size = 4;
        let result = layout(&input);
        assert_eq!(result.items.len(), 4, "page cap truncates the row");
        assert!(result.viewport_offset.is_none());
        let first_top = result.items[0].rect.top;
        let window_width = result.window.right - result.window.left;
        assert!(close(window_width, 60.0 * 4.0 + 8.0 * 3.0 + 8.0 * 2.0));
        for (local, entry) in result.items.iter().enumerate() {
            assert!(entry.visible);
            assert_eq!(entry.writing, WritingMode::Horizontal);
            assert!(close(entry.rect.top, first_top), "one row only");
            assert!(close(entry.rect.right - entry.rect.left, 60.0));
            if local > 0 {
                assert!(entry.rect.left > result.items[local - 1].rect.right);
            }
        }
    }

    #[test]
    fn paging_vertical_lays_row_per_candidate_capped_at_page_size() {
        let input = input(vec![item(100.0, 24.0); 6]);
        let mut input = input;
        input.options = options().with_overflow(OverflowBehavior::Paging);
        input.options.orientation = Orientation::Vertical;
        input.page_size = 5;
        let result = layout(&input);
        assert_eq!(result.items.len(), 5);
        assert!(result.viewport_offset.is_none());
        let first_left = result.items[0].rect.left;
        for (local, entry) in result.items.iter().enumerate() {
            assert!(close(entry.rect.left, first_left), "column-aligned list");
            assert!(close(entry.rect.right - entry.rect.left, 100.0));
            assert!(close(entry.rect.bottom - entry.rect.top, 24.0));
            if local > 0 {
                assert!(entry.rect.top > result.items[local - 1].rect.bottom);
            }
        }
    }

    #[test]
    fn scrolling_horizontal_keeps_highlight_inside_narrow_viewport() {
        let input = input(vec![item(160.0, 24.0); 6]);
        let mut input = input;
        input.options = options().with_overflow(OverflowBehavior::Scrolling);
        input.max_width = 400.0;
        input.selected = 5;
        let result = layout(&input);
        assert_eq!(result.items.len(), 6, "scrolling lays the full row");
        let (dx, dy) = result.viewport_offset.expect("narrow viewport must scroll");
        assert!(close(dy, 0.0));
        assert!(dx > 0.0, "highlight beyond viewport must scroll");
        let highlighted = &result.items[5];
        assert!(highlighted.visible);
        assert!(highlighted.rect.left >= result.window.left);
        assert!(highlighted.rect.right <= result.window.right);
        assert!(!result.items[0].visible, "start of row scrolled out");
        assert!(close(
            result.content_size.width,
            160.0 * 6.0 + 8.0 * 5.0 + 8.0 * 2.0
        ));
    }

    #[test]
    fn scrolling_vertical_keeps_highlight_inside_fixed_height_viewport() {
        let input = input(vec![item(100.0, 24.0); 20]);
        let mut input = input;
        input.options = options().with_overflow(OverflowBehavior::Scrolling);
        input.options.orientation = Orientation::Vertical;
        input.max_height = 200.0;
        input.selected = 19;
        let result = layout(&input);
        assert_eq!(result.items.len(), 20);
        let (dx, dy) = result.viewport_offset.expect("tall list must scroll");
        assert!(close(dx, 0.0));
        assert!(dy > 0.0);
        let highlighted = &result.items[19];
        assert!(highlighted.visible);
        assert!(highlighted.rect.top >= result.window.top);
        assert!(highlighted.rect.bottom <= result.window.bottom);
        assert!(close(result.window.bottom - result.window.top, 200.0));
    }

    #[test]
    fn wrapping_uses_real_widths_then_pages_past_height_budget() {
        // 2 per row at the width budget; row budget allows two rows => 4 shown.
        let input = input(vec![item(190.0, 30.0); 7]);
        let mut input = input;
        input.options = options().with_overflow(OverflowBehavior::Wrapping);
        input.max_width = 500.0;
        input.max_height = 90.0;
        let result = layout(&input);
        assert_eq!(result.items.len(), 4, "rest pages past the height budget");
        assert!(result.viewport_offset.is_none());
        assert!(close(result.items[0].rect.top, result.items[1].rect.top));
        assert!(result.items[2].rect.top > result.items[1].rect.bottom);
        assert!(close(result.items[2].rect.top, result.items[3].rect.top));
        for entry in &result.items {
            assert!(entry.visible);
        }
        // Real measured widths drive the wrap: no fixed-cell squeeze.
        assert!(close(
            result.items[0].rect.right - result.items[0].rect.left,
            190.0
        ));
    }

    #[test]
    fn vertical_writing_columns_order_left_to_right() {
        let input = input(vec![item(40.0, 80.0); 3]);
        let mut input = input;
        input.options = options().with_writing(WritingMode::VerticalLr);
        let result = layout(&input);
        assert_eq!(result.items.len(), 3);
        assert!(result.viewport_offset.is_none());
        assert!(result.items[0].rect.left < result.items[1].rect.left);
        assert!(result.items[1].rect.left < result.items[2].rect.left);
        for entry in &result.items {
            assert_eq!(entry.writing, WritingMode::VerticalLr);
            assert!(close(entry.rect.right - entry.rect.left, 40.0));
        }
    }

    #[test]
    fn vertical_writing_columns_order_right_to_left() {
        let input = input(vec![item(40.0, 80.0); 3]);
        let mut input = input;
        input.options = options().with_writing(WritingMode::VerticalRl);
        let result = layout(&input);
        assert_eq!(result.items.len(), 3);
        assert!(result.items[0].rect.left > result.items[1].rect.left);
        assert!(result.items[1].rect.left > result.items[2].rect.left);
        for entry in &result.items {
            assert_eq!(entry.writing, WritingMode::VerticalRl);
        }
    }

    #[test]
    fn vertical_writing_pages_at_page_size() {
        let input = input(vec![item(40.0, 80.0); 8]);
        let mut input = input;
        input.options = options().with_writing(WritingMode::VerticalRl);
        input.page_size = 5;
        let result = layout(&input);
        assert_eq!(result.items.len(), 5);
        assert_eq!(result.items[4].writing, WritingMode::VerticalRl);
    }

    #[test]
    fn vertical_wrapping_has_no_displayed_semantics() {
        let input = input(vec![item(40.0, 30.0); 3]);
        let mut input = input;
        input.options = CandidateLayoutOptions {
            orientation: Orientation::Vertical,
            overflow: OverflowBehavior::Wrapping,
            writing_mode: WritingMode::Horizontal,
        };
        let result = layout(&input);
        assert!(result.items.is_empty(), "Vertical+Wrapping is never shown");
    }
}
