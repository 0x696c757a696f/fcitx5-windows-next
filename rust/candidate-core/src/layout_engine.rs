//! The single-pass candidate layout (`layout`) behind the layout ABI, plus the flow-paging and vertical-text helpers axis_layout reuses.

#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

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
    // SAFETY: the validated input pointer is aligned and readable for one layout input.
    let input = unsafe { *input };
    let item_slice = if item_count == 0 {
        &[]
    } else {
        // SAFETY: the validated items pointer covers `item_count` readable sizes.
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
        // SAFETY: the validated output pointer covers exactly `result.items.len()` writable rects.
        let out_items = unsafe { std::slice::from_raw_parts_mut(out_items, result.items.len()) };
        // SAFETY: the validated index pointer covers exactly `result.item_indices.len()` writable indices.
        let out_indices =
            unsafe { std::slice::from_raw_parts_mut(out_item_indices, result.item_indices.len()) };
        for (target, source) in out_items.iter_mut().zip(result.items.iter()) {
            *target = rect_to_ffi(*source);
        }
        out_indices.copy_from_slice(&result.item_indices);
    }
    // SAFETY: the validated output pointer is writable for one layout result and no aliases are used.
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

/// Shrinks grid columns to fit `available`, taking width from the widest
/// columns first so short candidates keep their full glyph budget.
fn shrink_grid_columns(widths: &mut [f32], available: f32) {
    let mut sorted = widths.to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a));
    // Cap the k widest columns at one shared width; pick the smallest k whose
    // cap still leaves every remaining column at its natural width.
    let mut cap = available.max(0.0) / sorted.len().max(1) as f32;
    for k in 1..=sorted.len() {
        let rest: f32 = sorted[k..].iter().sum();
        let shared = (available - rest) / k as f32;
        if k == sorted.len() || shared >= sorted[k] {
            cap = shared.max(0.0);
            break;
        }
    }
    for width in widths.iter_mut() {
        *width = width.min(cap);
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
        // Each grid column takes the widest candidate it shows, so a long
        // candidate never shares an equal split with short neighbours.
        let used_columns = columns.min(end - first_visible);
        let mut column_widths = vec![0.0_f32; used_columns];
        for index in first_visible..end {
            let column = (index - first_visible) % columns;
            column_widths[column] = column_widths[column].max(input.items[index].width);
        }
        let column_gaps = input.column_gap * used_columns.saturating_sub(1) as f32;
        let natural_columns_width: f32 = column_widths.iter().sum();
        let max_width = if input.max_width > 0.0 {
            input.max_width
        } else {
            work_width
        };
        let width = (natural_columns_width + column_gaps + input.padding_x * 2.0)
            .min(max_width)
            .min(work_width);
        let available_columns_width = (width - input.padding_x * 2.0 - column_gaps).max(0.0);
        if natural_columns_width > available_columns_width && natural_columns_width > 0.0 {
            shrink_grid_columns(&mut column_widths, available_columns_width);
        }
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
        let mut column_lefts = Vec::with_capacity(used_columns);
        let mut x = left + input.padding_x;
        for column_width in &column_widths {
            column_lefts.push(x);
            x += column_width + input.column_gap;
        }
        for index in result.first_visible..end {
            let local = index - result.first_visible;
            let row = local / columns;
            let column = local % columns;
            let x = column_lefts[column];
            let y = top + input.padding_y + row as f32 * (row_height + input.row_gap);
            result.items.push(Rect {
                left: x,
                top: y,
                right: x + column_widths[column],
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

/// Multi-row horizontal layout for `flow` (rabbit `flow`/`flow_paging`):
/// items are placed left-to-right and wrap onto further rows once a row would
/// exceed `row_content_width`. Pure geometry on zero-origin content space; the
/// caller adds padding and the caret/placement offset. Returns the per-item
/// rects (row-major) and the tight content width/height. Item sizes already
/// carry any DPI scale, so the geometry scales with the caller-provided sizes.
#[must_use]
pub fn flow_paged_bounds(
    item_sizes: &[Size],
    row_content_width: f32,
    column_gap: f32,
    row_gap: f32,
) -> (Vec<Rect>, f32, f32) {
    if item_sizes.is_empty() {
        return (Vec::new(), 0.0, 0.0);
    }
    let mut rects = Vec::with_capacity(item_sizes.len());
    let mut x = 0.0_f32;
    let mut y = 0.0_f32;
    let mut row_height = 0.0_f32;
    let mut row_right = 0.0_f32;
    let mut content_width = 0.0_f32;
    for item in item_sizes {
        let item_width = item.width.max(1.0);
        let gap = if x > 0.0 { column_gap } else { 0.0 };
        if row_content_width > 0.0 && x + gap + item_width > row_content_width && x > 0.0 {
            content_width = content_width.max(row_right);
            y += row_height + row_gap;
            x = 0.0;
            row_height = 0.0;
            row_right = 0.0;
        }
        if x > 0.0 {
            x += column_gap;
        }
        rects.push(Rect {
            left: x,
            top: y,
            right: x + item_width,
            bottom: y + item.height,
        });
        row_height = row_height.max(item.height);
        row_right = row_right.max(x + item_width);
        x += item_width;
    }
    let height = y + row_height;
    (
        rects,
        content_width.max(row_right).max(0.0),
        height.max(0.0),
    )
}

/// `vertical_text` column geometry (rabbit `vertical_text`): every candidate
/// occupies one column that typesets its text top-to-bottom. Column width is
/// the full-glyph advance budget (CJK advance + DWrite margin) carried in the
/// item width; column height is the per-column glyph budget in the item
/// height. Columns are placed left-to-right (default) or right-to-left when
/// `left_to_right` is false. Actual vertical glyph typesetting is a renderer
/// concern (deferred to the renderer slice); this function owns the
/// deterministic column geometry. Returns the per-column rects and the tight
/// content width/height on zero-origin space.
#[must_use]
pub fn vertical_text_columns(
    item_sizes: &[Size],
    column_gap: f32,
    left_to_right: bool,
) -> (Vec<Rect>, f32, f32) {
    if item_sizes.is_empty() {
        return (Vec::new(), 0.0, 0.0);
    }
    let column_gaps = column_gap * item_sizes.len().saturating_sub(1) as f32;
    let width: f32 = item_sizes
        .iter()
        .map(|size| size.width.max(1.0))
        .sum::<f32>()
        + column_gaps;
    let height = item_sizes
        .iter()
        .fold(0.0_f32, |peak, size| peak.max(size.height));
    let mut rects = Vec::with_capacity(item_sizes.len());
    if left_to_right {
        let mut x = 0.0_f32;
        for size in item_sizes {
            let item_width = size.width.max(1.0);
            rects.push(Rect {
                left: x,
                top: 0.0,
                right: x + item_width,
                bottom: size.height,
            });
            x += item_width + column_gap;
        }
    } else {
        let mut x = width;
        for size in item_sizes {
            let item_width = size.width.max(1.0);
            x -= item_width;
            rects.push(Rect {
                left: x,
                top: 0.0,
                right: x + item_width,
                bottom: size.height,
            });
            if x > 0.0 {
                x -= column_gap;
            }
        }
    }
    (rects, width, height)
}
