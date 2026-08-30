#![forbid(unsafe_code)]

use std::process::Command;

#[test]
fn launcher_binary_reports_the_public_version_and_protocol() {
    let output = Command::new(env!("CARGO_BIN_EXE_fcitx5-launcher"))
        .arg("--version")
        .output()
        .expect("launcher binary should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .expect("launcher version should be UTF-8")
            .trim(),
        "fcitx5-launcher 0.1.0 protocol 14"
    );
}
