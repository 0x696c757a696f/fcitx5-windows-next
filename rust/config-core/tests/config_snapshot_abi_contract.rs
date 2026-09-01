#![deny(unsafe_op_in_unsafe_fn)]

use std::fs;
use std::path::PathBuf;

use fcitx5_config_core::{
    fcitx5_config_snapshot_candidate_color_at, fcitx5_config_snapshot_candidate_label_at,
    fcitx5_config_snapshot_destroy, fcitx5_config_snapshot_font_family_at,
    fcitx5_config_snapshot_input_method_at, fcitx5_config_snapshot_load_current_utf16,
    fcitx5_config_snapshot_load_visual_utf16, fcitx5_config_snapshot_view, Fcitx5ConfigSnapshot,
    Fcitx5ConfigUtf16, Fcitx5ConfigUtf8, FCITX5_CONFIG_FONT_CANDIDATE,
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "fcitx5-config-core-snapshot-abi-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("test directory should be created");
        Self(path)
    }

    fn config_path(&self) -> PathBuf {
        self.0.join("config.toml")
    }
}

#[test]
fn native_adapters_can_load_resolved_visual_snapshot_and_access_label_sequence() {
    let directory = TestDirectory::new();
    let install_root = directory.0.join("install");
    let data_root = directory.0.join("data");
    let builtin_theme = install_root
        .join("resources")
        .join("themes")
        .join("default");
    fs::create_dir_all(&builtin_theme).expect("builtin theme directory should be created");
    fs::write(
        builtin_theme.join("theme.toml"),
        r##"
format_version = 1
[theme]
id = "builtin.default"
name = "Builtin"
version = "1"
license = "MIT"
[dark.candidate.colors]
background = "#101820FF"
"##,
    )
    .expect("builtin theme should be written");
    fs::create_dir_all(&data_root).expect("data directory should be created");
    let config_path = data_root.join("config.toml");
    fs::write(
        &config_path,
        r##"
format_version = 1
[candidate.label]
sequence = ["一", "二", "三"]
"##,
    )
    .expect("Current should be written");

    let config_utf16: Vec<u16> = config_path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .collect();
    let install_utf16: Vec<u16> = install_root
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .collect();
    let data_utf16: Vec<u16> = data_root
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .collect();

    // SAFETY: every input borrows one live UTF-16 buffer for the duration of the call.
    let snapshot = unsafe {
        fcitx5_config_snapshot_load_visual_utf16(
            Fcitx5ConfigUtf16 {
                ptr: config_utf16.as_ptr(),
                len: config_utf16.len(),
            },
            Fcitx5ConfigUtf16 {
                ptr: install_utf16.as_ptr(),
                len: install_utf16.len(),
            },
            Fcitx5ConfigUtf16 {
                ptr: data_utf16.as_ptr(),
                len: data_utf16.len(),
            },
            0,
            1,
        )
    };
    assert!(!snapshot.is_null(), "visual ABI should return a snapshot");

    // SAFETY: the handle remains live until it is destroyed below.
    unsafe {
        assert_eq!(
            utf8(fcitx5_config_snapshot_candidate_label_at(snapshot, 1)),
            "二"
        );
        assert_eq!(
            utf8(fcitx5_config_snapshot_candidate_label_at(snapshot, 3)),
            ""
        );
        fcitx5_config_snapshot_destroy(snapshot);
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn utf8(value: Fcitx5ConfigUtf8) -> String {
    if value.len == 0 {
        return String::new();
    }
    // SAFETY: the ABI contracts guarantee an in-bounds UTF-8 span while the snapshot is live.
    unsafe { std::str::from_utf8(std::slice::from_raw_parts(value.ptr, value.len)) }
        .expect("Config Core only returns UTF-8 text")
        .to_owned()
}

#[test]
fn native_adapters_can_load_one_resolved_current_snapshot() {
    let directory = TestDirectory::new();
    let path = directory.config_path();
    fs::write(
        &path,
        include_str!("fixtures/legacy-production-config-v1.toml"),
    )
    .expect("production corpus should be written");
    let path_utf16: Vec<u16> = path.as_os_str().to_string_lossy().encode_utf16().collect();
    let path = Fcitx5ConfigUtf16 {
        ptr: path_utf16.as_ptr(),
        len: path_utf16.len(),
    };

    // SAFETY: `path` borrows the live UTF-16 buffer above for this call.
    let snapshot = unsafe { fcitx5_config_snapshot_load_current_utf16(path) };
    assert!(
        !snapshot.is_null(),
        "Config Core should load the production corpus"
    );

    let mut view = Fcitx5ConfigSnapshot::default();
    // SAFETY: `snapshot` came from the load call and `view` is writable.
    assert_eq!(
        unsafe { fcitx5_config_snapshot_view(snapshot, &mut view) },
        1
    );
    assert_eq!(utf8(view.appearance_mode), "system");
    assert_eq!(utf8(view.appearance_theme), "builtin:default");
    assert_eq!(utf8(view.candidate_orientation), "automatic");
    assert_eq!(view.candidate_page_size, 5);
    assert_eq!(view.candidate_scroll_mode, 1);
    assert_eq!(view.candidate_max_width_dip, 860.0);
    assert_eq!(view.candidate_opacity, 1.0);
    assert_eq!(utf8(view.candidate_preedit_mode), "inline");
    assert_eq!(view.candidate_corner_radius_dip, 12.0);
    assert_eq!(view.candidate_label_visible, 1);
    assert_eq!(utf8(view.candidate_label_style), "dot");
    assert_eq!(view.candidate_font_size_dip, 18.0);
    assert_eq!(view.candidate_font_weight, 400);
    assert_eq!(utf8(view.default_input_method), "pinyin");
    assert_eq!(utf8(view.hotkey_toggle_input_method), "Ctrl+Space");
    assert_eq!(utf8(view.hotkey_next_input_method), "Ctrl+Shift");
    assert_eq!(view.input_method_count, 3);

    // SAFETY: all outputs are writable and the snapshot remains live for every query.
    unsafe {
        assert_eq!(
            utf8(fcitx5_config_snapshot_input_method_at(snapshot, 1)),
            "rime"
        );
        assert_eq!(
            utf8(fcitx5_config_snapshot_font_family_at(
                snapshot,
                FCITX5_CONFIG_FONT_CANDIDATE,
                0,
            )),
            "Microsoft YaHei"
        );
        let mut color_name = Fcitx5ConfigUtf8::default();
        let mut color_value = Fcitx5ConfigUtf8::default();
        assert_eq!(
            fcitx5_config_snapshot_candidate_color_at(
                snapshot,
                0,
                &mut color_name,
                &mut color_value,
            ),
            1
        );
        assert_eq!(utf8(color_name), "background");
        assert_eq!(utf8(color_value), "#FFFFFF");
        fcitx5_config_snapshot_destroy(snapshot);
    }
}
