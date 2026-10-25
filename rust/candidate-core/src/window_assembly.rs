//! Assembles the candidate window rect from measured items, preedit and work area.

#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

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
