#![deny(unsafe_op_in_unsafe_fn)]

const VERSION_FALLBACK: &str = env!("CARGO_PKG_VERSION");
const RELEASE_CHANNEL_FALLBACK: &str = "stable";

fn version() -> &'static str {
    option_env!("FCITX_WINDOWS_VERSION").unwrap_or(VERSION_FALLBACK)
}

fn release_channel() -> &'static str {
    option_env!("FCITX_RELEASE_CHANNEL_NAME").unwrap_or(RELEASE_CHANNEL_FALLBACK)
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_version() -> *const u8 {
    version().as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_version_len() -> usize {
    version().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_release_channel() -> *const u8 {
    release_channel().as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_release_channel_len() -> usize {
    release_channel().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn fcitx5_windows_common_architecture() -> u32 {
    if cfg!(target_pointer_width = "64") {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_and_release_channel_are_stable_static_strings() {
        assert!(!version().is_empty());
        assert_eq!(version().as_ptr(), fcitx5_windows_common_version());
        assert_eq!(version().len(), fcitx5_windows_common_version_len());
        assert!(!release_channel().is_empty());
        assert_eq!(
            release_channel().as_ptr(),
            fcitx5_windows_common_release_channel()
        );
        assert_eq!(
            release_channel().len(),
            fcitx5_windows_common_release_channel_len()
        );
    }

    #[test]
    fn architecture_matches_target_pointer_width() {
        let expected = if cfg!(target_pointer_width = "64") {
            1
        } else {
            0
        };
        assert_eq!(fcitx5_windows_common_architecture(), expected);
    }
}
