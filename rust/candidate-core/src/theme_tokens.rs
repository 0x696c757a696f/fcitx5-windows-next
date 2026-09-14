#![forbid(unsafe_code)]

//! Product-owned default candidate colours.
//!
//! These tokens do not import vendored renderer or theme code, so upstream
//! renderer updates cannot change the product palette contract.

/// WeChat IME green in RGB channel order.
pub const WECHAT_GREEN_RGB: [u8; 3] = [7, 193, 96];
/// White in RGB channel order.
pub const WHITE_RGB: [u8; 3] = [255, 255, 255];
/// WeChat IME green in Win32 `COLORREF` channel order.
pub const WECHAT_GREEN_COLORREF: u32 = 0x0060_c107;
/// White in Win32 `COLORREF` channel order.
pub const WHITE_COLORREF: u32 = 0x00ff_ffff;
/// Outer floating candidate-surface radius in device-independent pixels.
pub const WECHAT_WINDOW_RADIUS_DIP: f32 = 12.0;
/// Inset selected-candidate pill radius in device-independent pixels.
pub const WECHAT_SELECTION_RADIUS_DIP: f32 = 10.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorref_tokens_preserve_rgb_channel_order() {
        assert_eq!(WECHAT_GREEN_COLORREF & 0xFF, u32::from(WECHAT_GREEN_RGB[0]));
        assert_eq!(
            (WECHAT_GREEN_COLORREF >> 8) & 0xFF,
            u32::from(WECHAT_GREEN_RGB[1])
        );
        assert_eq!(
            (WECHAT_GREEN_COLORREF >> 16) & 0xFF,
            u32::from(WECHAT_GREEN_RGB[2])
        );
    }

    #[test]
    fn candidate_shape_tokens_keep_the_selection_as_an_inset_pill() {
        assert!(WECHAT_WINDOW_RADIUS_DIP > WECHAT_SELECTION_RADIUS_DIP);
        assert!(WECHAT_SELECTION_RADIUS_DIP > 0.0);
    }
}
