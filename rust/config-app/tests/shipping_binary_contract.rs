#![forbid(unsafe_code)]

use std::process::Command;

#[test]
fn shipping_settings_binary_uses_the_product_name_and_rust_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_fcitx5-config"))
        .arg("--self-check")
        .output()
        .expect("shipping Settings binary should launch");

    assert!(
        output.status.success(),
        "shipping Settings self-check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = String::from_utf8(output.stdout).expect("self-check output should be UTF-8");
    assert!(report.contains("\"component\":\"fcitx5-config\""));
    assert!(report.contains("\"shipping_config_replaced\":true"));
    assert!(report.contains("\"permanent_runtime_selector\":false"));
}
