#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;

use fcitx5_config_core::{ConfigCore, FileStore, RecoverySource, VisualSnapshotRequest};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "fcitx5-config-core-visual-snapshot-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("test directory should be created");
        Self(path)
    }

    fn install_root(&self) -> PathBuf {
        self.0.join("install")
    }

    fn data_root(&self) -> PathBuf {
        self.0.join("data")
    }

    fn config_path(&self) -> PathBuf {
        self.data_root().join("config.toml")
    }

    fn write_builtin_theme(&self) {
        let directory = self
            .install_root()
            .join("resources")
            .join("themes")
            .join("default");
        fs::create_dir_all(&directory).expect("builtin theme directory should be created");
        fs::write(
            directory.join("theme.toml"),
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
    }

    fn write_user_theme(&self) {
        let directory = self.data_root().join("themes").join("night");
        fs::create_dir_all(&directory).expect("user theme directory should be created");
        fs::write(
            directory.join("theme.toml"),
            r##"
format_version = 1

[theme]
id = "night"
name = "Night"
version = "1"
license = "MIT"

[common.candidate]
orientation = "horizontal"

[common.fonts.candidate]
families = ["Theme Font", "system"]
size_dip = 20.0
weight = 500

[light.candidate.colors]
background = "#EEEEEEFF"

[dark.candidate.colors]
background = "#111111FF"
candidate_text = "#FFFFFFFF"
"##,
        )
        .expect("user theme should be written");
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn visual_snapshot_layers_selected_dark_theme_beneath_current_overrides() {
    let directory = TestDirectory::new("layers");
    directory.write_builtin_theme();
    directory.write_user_theme();
    let config_path = directory.config_path();
    fs::create_dir_all(directory.data_root()).expect("data directory should be created");
    fs::write(
        &config_path,
        r##"
format_version = 1

[appearance]
mode = "system"
theme = "night"

[candidate]
page_size = 7

[candidate.label]
sequence = ["一", "二", "三"]

[candidate.colors]
background = "#222222FF"
"##,
    )
    .expect("Current should be written");

    let visual = ConfigCore::load_visual_snapshot(
        &FileStore::new(),
        VisualSnapshotRequest::new(
            &config_path,
            &directory.install_root(),
            &directory.data_root(),
            false,
            true,
        ),
    );

    assert_eq!(visual.recovery_source(), RecoverySource::Current);
    let snapshot = visual.snapshot();
    assert_eq!(snapshot.candidate().orientation(), "horizontal");
    assert_eq!(snapshot.candidate().page_size(), 7);
    assert_eq!(snapshot.candidate().label().sequence(), ["一", "二", "三"]);
    assert_eq!(snapshot.fonts().candidate().size_dip(), 20.0);
    assert_eq!(snapshot.fonts().candidate().weight(), 500);
    assert_eq!(
        snapshot.candidate().colors().get("background"),
        Some(&"#222222FF".to_owned())
    );
    assert_eq!(
        snapshot.candidate().colors().get("candidate_text"),
        Some(&"#FFFFFFFF".to_owned())
    );
}

#[test]
fn visual_snapshot_uses_last_known_good_and_fails_soft_for_missing_theme() {
    let directory = TestDirectory::new("recovery");
    directory.write_builtin_theme();
    let config_path = directory.config_path();
    fs::create_dir_all(directory.data_root()).expect("data directory should be created");
    fs::write(&config_path, "not valid toml = [").expect("bad Current should be written");
    fs::write(
        FileStore::last_known_good_path(&config_path),
        r##"
format_version = 1

[appearance]
mode = "dark"
theme = "missing-theme"

[candidate]
page_size = 8
"##,
    )
    .expect("LKG should be written");

    let visual = ConfigCore::load_visual_snapshot(
        &FileStore::new(),
        VisualSnapshotRequest::new(
            &config_path,
            &directory.install_root(),
            &directory.data_root(),
            false,
            false,
        ),
    );

    assert_eq!(visual.recovery_source(), RecoverySource::LastKnownGood);
    assert_eq!(visual.snapshot().candidate().page_size(), 8);
    assert!(visual.snapshot().candidate().colors().is_empty());
}

#[test]
fn safe_mode_ignores_persisted_config_and_uses_builtin_theme_defaults() {
    let directory = TestDirectory::new("safe-mode");
    directory.write_builtin_theme();
    let config_path = directory.config_path();
    fs::create_dir_all(directory.data_root()).expect("data directory should be created");
    fs::write(
        &config_path,
        r##"
format_version = 1

[appearance]
theme = "missing-theme"

[candidate]
page_size = 9
"##,
    )
    .expect("Current should be written");

    let visual = ConfigCore::load_visual_snapshot(
        &FileStore::new(),
        VisualSnapshotRequest::new(
            &config_path,
            &directory.install_root(),
            &directory.data_root(),
            true,
            true,
        ),
    );

    assert_eq!(visual.recovery_source(), RecoverySource::SafeDefaults);
    assert_eq!(visual.snapshot().candidate().page_size(), 5);
    assert_eq!(
        visual.snapshot().candidate().colors().get("background"),
        Some(&"#101820FF".to_owned())
    );
}

#[test]
fn visual_snapshot_rejects_theme_ids_that_would_escape_the_theme_directory() {
    let directory = TestDirectory::new("path-traversal");
    directory.write_builtin_theme();
    fs::create_dir_all(directory.data_root()).expect("data directory should be created");
    fs::write(
        directory.data_root().join("theme.toml"),
        r##"
format_version = 1
[theme]
id = ".."
name = "Outside"
version = "1"
license = "MIT"
[dark.candidate.colors]
background = "#DEADBEEFFF"
"##,
    )
    .expect("outside file should be written");
    let config_path = directory.config_path();
    fs::write(
        &config_path,
        r##"
format_version = 1
[appearance]
theme = ".."
"##,
    )
    .expect("Current should be written");

    let visual = ConfigCore::load_visual_snapshot(
        &FileStore::new(),
        VisualSnapshotRequest::new(
            &config_path,
            &directory.install_root(),
            &directory.data_root(),
            false,
            true,
        ),
    );

    assert_eq!(visual.recovery_source(), RecoverySource::Current);
    assert!(visual.snapshot().candidate().colors().is_empty());
}
