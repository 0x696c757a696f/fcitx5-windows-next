//! Paints a laid-out candidate frame: theme conversion, bitmap render and DC blit.

#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

#[cfg(windows)]
fn render_theme(theme: &Fcitx5CandidateRenderThemeInput) -> renderer::RenderTheme {
    renderer::RenderTheme {
        background: RenderColor::rgba(
            theme.background_r,
            theme.background_g,
            theme.background_b,
            255,
        ),
        text: RenderColor::rgba(theme.text_r, theme.text_g, theme.text_b, 255),
        selected_background: RenderColor::rgba(
            theme.selected_background_r,
            theme.selected_background_g,
            theme.selected_background_b,
            255,
        ),
        selected_text: RenderColor::rgba(
            theme.selected_text_r,
            theme.selected_text_g,
            theme.selected_text_b,
            255,
        ),
        comment_color: RenderColor::rgba(theme.comment_r, theme.comment_g, theme.comment_b, 255),
        border: RenderColor::rgba(theme.border_r, theme.border_g, theme.border_b, 255),
        scrollbar: RenderColor::rgba(theme.scrollbar_r, theme.scrollbar_g, theme.scrollbar_b, 255),
        preedit_background: RenderColor::rgba(
            theme.preedit_background_r,
            theme.preedit_background_g,
            theme.preedit_background_b,
            255,
        ),
        preedit_text: RenderColor::rgba(
            theme.preedit_text_r,
            theme.preedit_text_g,
            theme.preedit_text_b,
            255,
        ),
        selection_inflate_x: theme.selection_inflate_x,
        selection_inflate_y: theme.selection_inflate_y,
        corner_radius: theme.corner_radius,
    }
}

/// Colors (RGB, alpha forced to 255 by the painter) + selection inflate +
/// corner radius for `render_frame`.
#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateRenderThemeInput {
    pub background_r: u8,
    pub background_g: u8,
    pub background_b: u8,
    pub text_r: u8,
    pub text_g: u8,
    pub text_b: u8,
    pub selected_background_r: u8,
    pub selected_background_g: u8,
    pub selected_background_b: u8,
    pub selected_text_r: u8,
    pub selected_text_g: u8,
    pub selected_text_b: u8,
    pub comment_r: u8,
    pub comment_g: u8,
    pub comment_b: u8,
    pub border_r: u8,
    pub border_g: u8,
    pub border_b: u8,
    pub scrollbar_r: u8,
    pub scrollbar_g: u8,
    pub scrollbar_b: u8,
    pub preedit_background_r: u8,
    pub preedit_background_g: u8,
    pub preedit_background_b: u8,
    pub preedit_text_r: u8,
    pub preedit_text_g: u8,
    pub preedit_text_b: u8,
    pub selection_inflate_x: f32,
    pub selection_inflate_y: f32,
    pub corner_radius: f32,
}

/// One frame to paint, positioned exactly as the frame update laid it out.
#[cfg(windows)]
pub struct FramePaint<'a> {
    pub candidates: &'a [CandidateRenderData],
    /// Window-local content rects from the frame update, one per candidate.
    pub item_rects: &'a [Rect],
    pub window_width: f32,
    pub window_height: f32,
    /// Window-local y where the candidate area starts, below the preedit row.
    pub candidate_top: f32,
    pub viewport_offset: Option<(f32, f32)>,
    pub content_size: Size,
    pub writing: axis_layout::WritingMode,
    pub theme: Fcitx5CandidateRenderThemeInput,
    pub geometry: RenderGeometry,
    pub preedit: Option<&'a str>,
    /// Must be the family the frame was measured with.
    pub font_family: &'a str,
    pub high_contrast: bool,
    pub selected: Option<usize>,
}

/// Paints `frame` into `dc` without laying the candidates out again, so the
/// pixels land on the same rects the window uses for hit testing.
///
/// # Safety
///
/// `dc` must be a live device context for the call.
#[cfg(windows)]
pub unsafe fn render_frame_to_dc(
    frame: &FramePaint<'_>,
    dc: *mut c_void,
    client_width: i32,
    client_height: i32,
) -> bool {
    let render = render_frame(frame);
    if render.pixels.is_empty() {
        return false;
    }
    // SAFETY: `dc` validity is the caller's contract.
    unsafe {
        crate::window_host::blit_bgra_to_dc(
            dc,
            render.pixels.as_ptr(),
            render.pixels.len(),
            render.stride as usize,
            client_width,
            client_height,
        ) != 0
    }
}

/// Renders `frame` to a window-sized BGRA bitmap.
#[cfg(windows)]
pub fn render_frame(frame: &FramePaint<'_>) -> RenderWindowOutput {
    let preedit = frame.preedit.filter(|text| !text.is_empty());
    let candidate_top = if preedit.is_some() {
        frame.candidate_top
    } else {
        0.0
    };
    let window = Rect {
        left: 0.0,
        top: 0.0,
        right: frame.window_width,
        bottom: (frame.window_height - candidate_top).max(0.0),
    };
    // The frame stores content rects; the renderer insets item padding itself.
    let pad_x = frame.geometry.item_padding_x.max(0.0);
    let pad_y = frame.geometry.item_padding_y.max(0.0);
    let items: Vec<axis_layout::AxisLayoutItem> = frame
        .item_rects
        .iter()
        .map(|rect| {
            let rect = Rect {
                left: rect.left - pad_x,
                top: rect.top - pad_y - candidate_top,
                right: rect.right + pad_x,
                bottom: rect.bottom + pad_y - candidate_top,
            };
            axis_layout::AxisLayoutItem {
                rect,
                visible: rect.left < window.right
                    && rect.right > window.left
                    && rect.top < window.bottom
                    && rect.bottom > window.top,
                writing: frame.writing,
            }
        })
        .collect();
    let axis_result = axis_layout::AxisLayoutResult {
        window,
        placement: Placement::Unlocked,
        first_visible: items.iter().position(|item| item.visible).unwrap_or(0),
        items,
        viewport_offset: frame.viewport_offset,
        content_size: frame.content_size,
    };
    let geometry = RenderGeometry {
        preedit_height: candidate_top,
        ..frame.geometry
    };
    renderer::render_candidate_window(&renderer::RenderWindowInput {
        axis_result: &axis_result,
        candidates: frame.candidates,
        theme: &render_theme(&frame.theme),
        geometry: &geometry,
        font_family: frame.font_family,
        preedit,
        dpi_scale: 1.0,
        high_contrast: frame.high_contrast,
        selected: frame.selected,
    })
}
