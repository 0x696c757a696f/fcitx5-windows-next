use std::fs;
use std::path::PathBuf;
use std::process::Command;

use fcitx5_config_core::{CommitFault, ConfigCore, ConfigEdit, FileStore};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "fcitx5-config-poc-cli-{}-{}",
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

    let output = Command::new(env!("CARGO_BIN_EXE_fcitx5-config-poc"))
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
