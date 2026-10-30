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
/// Below this a nested radius reads as a rounding error, so the inner shape
/// stays square.
const MIN_NESTED_RADIUS_DIP: f32 = 2.0;

/// Radius of a shape nested `gap` inside a parent with `outer` radius, so the
/// two corners stay concentric.
pub(crate) fn nested_corner_radius(outer: f32, gap: f32) -> f32 {
    let inner = outer - gap.max(0.0);
    if inner > MIN_NESTED_RADIUS_DIP {
        inner
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_radius_stays_concentric_with_the_window() {
        assert_eq!(nested_corner_radius(WECHAT_WINDOW_RADIUS_DIP, 6.0), 6.0);
        assert_eq!(nested_corner_radius(16.0, 8.0), 8.0);
        assert_eq!(nested_corner_radius(12.0, 10.0), 0.0);
        assert_eq!(nested_corner_radius(12.0, -3.0), 12.0);
        assert_eq!(nested_corner_radius(0.0, 4.0), 0.0);
    }
}
