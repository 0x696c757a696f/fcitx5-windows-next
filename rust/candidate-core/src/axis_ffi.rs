//! C ABI over the three-axis candidate layout.

#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

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

pub(crate) fn axis_overflow_from_ffi(value: u8) -> Option<axis_layout::OverflowBehavior> {
    match value {
        0 => Some(axis_layout::OverflowBehavior::Paging),
        1 => Some(axis_layout::OverflowBehavior::Scrolling),
        2 => Some(axis_layout::OverflowBehavior::Wrapping),
        _ => None,
    }
}

pub(crate) fn axis_writing_from_ffi(value: u8) -> Option<axis_layout::WritingMode> {
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
