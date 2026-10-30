#![forbid(unsafe_code)]

//! The Settings window's spacing, type and radius scales. Every size in the
//! UI picks a step from here instead of an ad hoc value, so pages line up on
//! one 4 px grid and text uses a short, consistent ladder.

/// Spacing scale in px (2, 4, then a 4/8 px grid).
pub const SPACE_2XS: i32 = 2;
pub const SPACE_XS: i32 = 4;
pub const SPACE_SM: i32 = 8;
pub const SPACE_MD: i32 = 12;
pub const SPACE_LG: i32 = 16;
pub const SPACE_XL: i32 = 24;

/// Type scale in px. 14 is the smallest size used for primary text: CJK
/// glyphs below it lose strokes at 100% scaling.
pub const TEXT_XS: f32 = 12.0;
pub const TEXT_SM: f32 = 14.0;
pub const TEXT_BASE: f32 = 16.0;
pub const TEXT_2XL: f32 = 24.0;

/// Headings and emphasized labels. Nothing in Settings is heavier.
pub const WEIGHT_SEMIBOLD: u16 = 600;

/// Corner radius scale in px.
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;
pub const RADIUS_XL: f32 = 12.0;

/// Heights of interactive rows: comfortable pointer and touch targets.
pub const NAV_ITEM_HEIGHT: i32 = 40;
pub const TITLEBAR_HEIGHT: i32 = 48;
pub const FORM_ROW_HEIGHT: i32 = 48;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_and_heights_sit_on_the_four_px_grid() {
        for value in [
            SPACE_XS,
            SPACE_SM,
            SPACE_MD,
            SPACE_LG,
            SPACE_XL,
            NAV_ITEM_HEIGHT,
            TITLEBAR_HEIGHT,
            FORM_ROW_HEIGHT,
        ] {
            assert_eq!(value % 4, 0, "{value}");
        }
    }
}
