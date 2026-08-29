#![cfg(windows)]
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use fcitx5_package_core::update::{self, Fcitx5UpdateOwner};

fn updater_output(arguments: &[OsString]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fcitx5-updater"))
        .args(arguments)
        .output()
        .expect("updater process should start")
}

fn argument_list(values: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    values.into_iter().collect()
}

fn write_runtime_payload(root: &Path, marker: &str) {
    let files = [
        ("bin/fcitx5-engine.exe", marker.to_owned()),
        ("bin/fcitx5-launcher.exe", marker.to_owned()),
        ("bin/fcitx5-ui.exe", marker.to_owned()),
        ("lib/fcitx5/addon.dll", marker.to_owned()),
        ("share/fcitx5/profile", marker.to_owned()),
        ("tsf/x64/fcitx5-tsf.dll", format!("{marker}-x64")),
        ("tsf/x86/fcitx5-tsf.dll", format!("{marker}-x86")),
    ];
    for (relative, contents) in files {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("payload parent"))
            .expect("payload parent should create");
        std::fs::write(path, contents).expect("payload file should write");
    }
}

#[test]
fn updater_cli_preserves_generation_and_previous_cleanup_contract() {
    let root = std::env::temp_dir().join(format!(
        "fcitx5-package-core-updater-cli-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("CLI fixture directory should create");

    let registered = root.join("tsf/x64/fcitx5-tsf.dll");
    let incoming = root.join("staging/fcitx5-tsf.dll");
    std::fs::create_dir_all(registered.parent().expect("registered parent"))
        .expect("registered parent should create");
    std::fs::create_dir_all(incoming.parent().expect("incoming parent"))
        .expect("incoming parent should create");
    std::fs::write(&registered, b"old-tsf").expect("old TSF should write");
    std::fs::write(&incoming, b"new-tsf").expect("new TSF should write");
    let output = updater_output(&argument_list([
        OsString::from("--install-tsf-dll"),
        registered.as_os_str().to_owned(),
        incoming.as_os_str().to_owned(),
        OsString::from("00000042"),
    ]));
    assert!(output.status.success(), "TSF install failed: {:?}", output);
    assert_eq!(
        std::fs::read(&registered).expect("installed TSF should read"),
        b"new-tsf"
    );

    let output = updater_output(&argument_list([
        OsString::from("--install-tsf-dll"),
        root.as_os_str().to_owned(),
        incoming.as_os_str().to_owned(),
        OsString::from("00000042"),
    ]));
    assert!(!output.status.success(), "invalid TSF path was accepted");

    let old_tsf = root.join("tsf/x64/fcitx5-tsf.old.00000041.test.dll");
    let unrelated = root.join("tsf/x64/not-owned.dll");
    std::fs::write(&old_tsf, b"stale").expect("old TSF fixture should write");
    std::fs::write(&unrelated, b"keep").expect("unrelated fixture should write");
    let output = updater_output(&argument_list([
        OsString::from("--cleanup-old-tsf-dlls"),
        root.join("tsf/x64").as_os_str().to_owned(),
    ]));
    assert!(
        output.status.success(),
        "old TSF cleanup failed: {:?}",
        output
    );
    assert!(!old_tsf.exists());
    assert!(unrelated.exists());

    let output = updater_output(&argument_list([
        OsString::from("--publish-generation"),
        root.as_os_str().to_owned(),
        OsString::from("00000042"),
        OsString::from("build-42"),
    ]));
    assert!(!output.status.success(), "missing generation was published");
    std::fs::create_dir_all(root.join("runtime/00000042"))
        .expect("generation directory should create");
    let output = updater_output(&argument_list([
        OsString::from("--publish-generation"),
        root.as_os_str().to_owned(),
        OsString::from("00000042"),
        OsString::from("build-42"),
    ]));
    assert!(
        output.status.success(),
        "generation publication failed: {:?}",
        output
    );
    let output = updater_output(&argument_list([
        OsString::from("--generation-status"),
        root.as_os_str().to_owned(),
    ]));
    assert!(
        output.status.success(),
        "generation status failed: {:?}",
        output
    );
    let status = String::from_utf8(output.stdout).expect("status should be UTF-8");
    assert!(status.contains("current_generation=00000042"));

    let payload = root.join("runtime-payload");
    write_runtime_payload(&payload, "runtime-43");
    let output = updater_output(&argument_list([
        OsString::from("--activate-runtime-generation"),
        root.as_os_str().to_owned(),
        payload.as_os_str().to_owned(),
        OsString::from("00000043"),
        OsString::from("build-43"),
    ]));
    assert!(
        output.status.success(),
        "runtime activation failed: {:?}",
        output
    );
    assert_eq!(
        std::fs::read_to_string(root.join("runtime/00000043/bin/fcitx5-engine.exe"))
            .expect("runtime engine should read"),
        "runtime-43"
    );

    update::write_update_owner(&root, Fcitx5UpdateOwner::Builtin)
        .expect("builtin update owner should write");
    update::begin_activation(&root, "stable", "1.0.0", Fcitx5UpdateOwner::Builtin)
        .expect("first deployment should begin");
    update::mark_current_healthy(&root, "stable").expect("first deployment should be healthy");
    update::begin_activation(&root, "stable", "1.1.0", Fcitx5UpdateOwner::Builtin)
        .expect("second deployment should begin");
    update::mark_current_healthy(&root, "stable").expect("second deployment should be healthy");
    let previous = root.join("packages/versions/core/1.0.0/marker.txt");
    std::fs::create_dir_all(previous.parent().expect("previous parent"))
        .expect("previous parent should create");
    std::fs::write(&previous, b"keep").expect("previous marker should write");

    let output = updater_output(&argument_list([
        OsString::from("--cleanup-previous"),
        root.as_os_str().to_owned(),
        OsString::from("stable"),
        OsString::from(".."),
    ]));
    assert!(
        !output.status.success(),
        "malicious package id was accepted"
    );
    assert!(previous.exists());
    let output = updater_output(&argument_list([
        OsString::from("--cleanup-previous"),
        root.as_os_str().to_owned(),
        OsString::from("stable"),
        OsString::from("core"),
    ]));
    assert!(
        output.status.success(),
        "valid previous cleanup failed: {:?}",
        output
    );
    assert!(!previous.exists());

    let _ = std::fs::remove_dir_all(PathBuf::from(root));
}
