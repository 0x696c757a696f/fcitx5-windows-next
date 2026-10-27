#![forbid(unsafe_code)]

//! Product-owned default candidate colours.
//!
//! These tokens do not import vendored renderer or theme code, so upstream
//! renderer updates cannot change the product palette contract.

/// WeChat IME green in RGB channel order.
pub(crate) const WECHAT_GREEN_RGB: [u8; 3] = [7, 193, 96];
/// White in RGB channel order.
pub(crate) const WHITE_RGB: [u8; 3] = [255, 255, 255];
/// Outer floating candidate-surface radius in device-independent pixels.
pub(crate) const WECHAT_WINDOW_RADIUS_DIP: f32 = 12.0;
/// Inset selected-candidate pill radius in device-independent pixels.
pub(crate) const WECHAT_SELECTION_RADIUS_DIP: f32 = 10.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_shape_tokens_keep_the_selection_as_an_inset_pill() {
        const { assert!(WECHAT_WINDOW_RADIUS_DIP > WECHAT_SELECTION_RADIUS_DIP) };
        const { assert!(WECHAT_SELECTION_RADIUS_DIP > 0.0) };
    }
}
