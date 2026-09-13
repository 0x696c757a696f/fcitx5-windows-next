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
}
