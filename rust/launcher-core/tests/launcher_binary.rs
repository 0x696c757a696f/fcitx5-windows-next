#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

#[test]
fn launcher_binary_reports_bounded_startup_status() {
    let root = std::env::temp_dir().join(format!("fcitx5-launcher-binary-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("launcher binary test directory should be created");
    let engine = root.join("fcitx5-engine.exe");
    fs::write(&engine, b"engine").expect("engine stub should be created");
    let state_file = root.join("launcher-state.v1");

    let output = Command::new(env!("CARGO_BIN_EXE_fcitx5-launcher"))
        .args([
            "--engine",
            engine.to_str().expect("engine path should be UTF-8"),
            "--state-file",
            state_file.to_str().expect("state path should be UTF-8"),
            "--no-warmup",
        ])
        .output()
        .expect("launcher binary should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .expect("launcher status should be UTF-8")
            .trim(),
        "launcher_state=0 engine_state=0 start_disposition=1 retry_after_ms=0"
    );

    let _ = fs::remove_dir_all(root);
}
