//! Assembles the candidate window rect from measured items, preedit and work area.

#![forbid(unsafe_code)]

use super::*;

/// Horizontal natural-width downgrade decision (081D slice 6).
pub(crate) fn horizontal_natural_downgrade(
    widths: &[f32],
    padding_x: f32,
    column_gap: f32,
    preedit_width: f32,
    hard_limit: f32,
) -> bool {
    let mut natural = 0.0_f32;
    for width in widths {
        if natural > 0.0 {
            natural += column_gap;
        }
        natural += width;
    }
    let natural = (natural + padding_x * 2.0).max(preedit_width);
    natural > hard_limit + 0.5
}

/// Axis-layout item rect + visibility handed to the window-assembly ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Fcitx5CandidateAssemblyItem {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Inputs for the frozen post-layout window assembly (081D model slice 2).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Fcitx5CandidateWindowAssemblyInput<'a> {
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
    pub items: &'a [Fcitx5CandidateAssemblyItem],
}

/// Frozen assembly result: final window rect, preedit panel, scrollbar, and
/// window-local item rects (written to `out_items` when non-null).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Fcitx5CandidateWindowAssemblyOutput {
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
pub(crate) fn window_assembly(
    input: &Fcitx5CandidateWindowAssemblyInput<'_>,
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
    let count = input.items.len().min(out_items.len());
    for (index, item) in input.items.iter().enumerate().take(count) {
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

#[cfg(test)]
mod candidate_horizontal_downgrade_tests {
    use super::horizontal_natural_downgrade;

    #[test]
    fn downgrade_follows_the_frozen_natural_width_contract() {
        let widths = [100.0_f32, 80.0, 60.0];
        // sum + gaps + padding = 240 + 20 + 24 = 284; below limit → keep.
        assert!(!horizontal_natural_downgrade(
            &widths, 12.0, 10.0, 0.0, 300.0
        ));
        // Preedit width wins over the natural sum.
        assert!(horizontal_natural_downgrade(
            &widths, 12.0, 10.0, 320.0, 300.0
        ));
        // The +0.5 tolerance holds a borderline width.
        assert!(!horizontal_natural_downgrade(
            &widths, 12.0, 10.0, 0.0, 283.5
        ));
        assert!(horizontal_natural_downgrade(
            &widths, 12.0, 10.0, 0.0, 283.4
        ));
    }
}
