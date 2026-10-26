#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use fcitx5_config_core::{CommitFault, ConfigCore, ConfigEdit, FileStore};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "fcitx5-config-cli-{}-{}",
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

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn get_uses_recovery_without_overwriting_a_bad_current_file() {
    let directory = TestDirectory::new();
    let path = directory.config_path();
    let store = FileStore::new();
    let mut core = ConfigCore::compiled_defaults();
    core.set(ConfigEdit::CandidatePageSize(7));
    core.apply(&store, &path, CommitFault::None)
        .expect("valid config should create a recovery record");
    fs::write(&path, "candidate.page_size = 0\n").expect("bad current fixture should write");

    let output = Command::new(env!("CARGO_BIN_EXE_fcitx5-config"))
        .args(["--config", path.to_str().expect("UTF-8 path"), "get"])
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("\"page_size\": 7"));
    assert_eq!(
        fs::read_to_string(&path).expect("bad current should remain"),
        "candidate.page_size = 0\n"
    );
}

fn run_config(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fcitx5-config"))
        .args(args)
        .output()
        .expect("CLI should run")
}

#[test]
fn doctor_reports_safe_defaults_for_a_missing_file_without_creating_it() {
    let directory = TestDirectory::new();
    let path = directory.config_path();

    let output = run_config(&["--config", path.to_str().expect("UTF-8 path"), "doctor"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("recovery=compiled-safe-defaults"));
    assert!(!path.exists());
}

#[test]
fn validate_accepts_the_shipped_defaults_and_rejects_an_invalid_file() {
    let defaults = concat!(env!("CARGO_MANIFEST_DIR"), "/../../resources/config.toml");
    let output = run_config(&["--config", defaults, "validate"]);
    assert!(
        output.status.success(),
        "shipped defaults should validate: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let directory = TestDirectory::new();
    let path = directory.config_path();
    fs::write(&path, "candidate.page_size = 0\n").expect("bad fixture should write");
    let output = run_config(&["--config", path.to_str().expect("UTF-8 path"), "validate"]);
    assert!(!output.status.success());
}

#[test]
fn retired_self_check_modes_are_rejected() {
    for argument in [
        "--self-check",
        "--self-test",
        "--window-smoke",
        "--ui-contract-test",
    ] {
        let output = run_config(&[argument]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{argument} should be rejected"
        );
    }
}
