//! The single-pass candidate layout (`layout`) behind the layout ABI, plus the flow-paging and vertical-text helpers axis_layout reuses.

#![forbid(unsafe_code)]

use super::*;

/// Multi-row horizontal layout for `flow` (rabbit `flow`/`flow_paging`):
/// items are placed left-to-right and wrap onto further rows once a row would
/// exceed `row_content_width`. Pure geometry on zero-origin content space; the
/// caller adds padding and the caret/placement offset. Returns the per-item
/// rects (row-major) and the tight content width/height. Item sizes already
/// carry any DPI scale, so the geometry scales with the caller-provided sizes.
#[must_use]
pub(crate) fn flow_paged_bounds(
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
pub(crate) fn vertical_text_columns(
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
