#![deny(unsafe_op_in_unsafe_fn)]

//! Typed Config state, transaction, recovery, and native snapshot ABI contract.

mod config_core;
mod config_snapshot_abi;

pub use config_core::*;
pub use config_snapshot_abi::{
    fcitx5_config_snapshot_candidate_color_at, fcitx5_config_snapshot_candidate_label_at,
    fcitx5_config_snapshot_destroy, fcitx5_config_snapshot_font_family_at,
    fcitx5_config_snapshot_input_method_at, fcitx5_config_snapshot_load_current_utf16,
    fcitx5_config_snapshot_load_visual_utf16, fcitx5_config_snapshot_view, Fcitx5ConfigSnapshot,
    Fcitx5ConfigUtf16, Fcitx5ConfigUtf8, FCITX5_CONFIG_FONT_ANNOTATION,
    FCITX5_CONFIG_FONT_CANDIDATE, FCITX5_CONFIG_FONT_MONOSPACE, FCITX5_CONFIG_FONT_UI,
};
