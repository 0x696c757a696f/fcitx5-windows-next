#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn control_binary() -> String {
    std::env::var("CARGO_BIN_EXE_fcitx5-control")
        .expect("Cargo must provide the fcitx5-control binary path")
}

fn temporary_directory(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("fcitx5-control-{label}-{nonce}"));
    fs::create_dir_all(&path).expect("temporary directory must be created");
    path
}

#[test]
fn schema_and_invalid_invocation_preserve_the_public_cli_contract() {
    let schema = Command::new(control_binary())
        .arg("--schema")
        .output()
        .expect("control schema command must start");
    assert!(schema.status.success());
    assert!(String::from_utf8_lossy(&schema.stdout).contains("\"validate_config\""));

    let invalid = Command::new(control_binary())
        .arg("--validate-config")
        .output()
        .expect("invalid control command must start");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).starts_with("Usage: fcitx5-control "));
}

#[test]
fn status_reports_shared_data_root_and_config_health_without_a_launcher() {
    let root = temporary_directory("status");
    let output = Command::new(control_binary())
        .args([
            "--data-root",
            root.to_str().expect("temporary path is UTF-8"),
            "--status",
        ])
        .output()
        .expect("status command must start");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("status must be UTF-8 JSON");
    assert!(stdout.contains("\"launcher_reachable\":false"));
    assert!(stdout.contains("\"config_valid\":true"));
    let escaped_root = root
        .to_str()
        .expect("temporary path is UTF-8")
        .replace('\\', "\\\\");
    assert!(stdout.contains(&escaped_root));
    fs::remove_dir_all(root).expect("temporary directory must be removed");
}

#[test]
fn validate_and_apply_use_the_typed_config_transaction() {
    let root = temporary_directory("config");
    let source = root.join("candidate-font-config.toml");
    fs::write(
        &source,
        "format_version = 1\n[fonts.candidate]\nfamilies = [\"Segoe UI Emoji\", \"system\"]\nsize_dip = 20.0\n",
    )
    .expect("fixture must be written");

    let validate = Command::new(control_binary())
        .args([
            "--data-root",
            root.to_str().expect("temporary path is UTF-8"),
            "--validate-config",
            source.to_str().expect("fixture path is UTF-8"),
        ])
        .output()
        .expect("validate command must start");
    assert!(
        validate.status.success(),
        "{}",
        String::from_utf8_lossy(&validate.stderr)
    );

    let apply = Command::new(control_binary())
        .args([
            "--data-root",
            root.to_str().expect("temporary path is UTF-8"),
            "--apply-config",
            source.to_str().expect("fixture path is UTF-8"),
        ])
        .output()
        .expect("apply command must start");
    assert!(
        apply.status.success(),
        "{}",
        String::from_utf8_lossy(&apply.stderr)
    );
    let persisted =
        fs::read_to_string(root.join("config.toml")).expect("apply must persist config");
    assert!(persisted.contains("Segoe UI Emoji"));
    assert!(persisted.contains("size_dip = 20.0"));

    fs::remove_dir_all(root).expect("temporary directory must be removed");
}
