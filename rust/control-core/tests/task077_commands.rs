#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fcitx5_control_core::{
    control_addons_list_json_for_path, control_package_detail_json_for_paths,
    control_packages_list_json_for_paths, control_theme_detail_json_for_paths,
    control_themes_list_json_for_paths,
};

fn root() -> PathBuf {
    let path = std::env::temp_dir().join(format!("fcitx5-control-task077-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("fixture root");
    path
}

fn install_package_fixture(data: &Path) {
    let manifest = r#"{"format_version":1,"id":"fcitx5-rime","version":"1.0.0","type":"addon","architecture":"x64","min_os":"6.1-sp1","core_api":"1","addon_abi":"1","dependencies":[{"id":"rime-data","version":"1.0.0"}],"license":"MIT","source_commit":"0123456789abcdef","permissions":["native-code","input-data"],"files":[{"path":"lib/fcitx5/librime.dll","size":12,"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}],"key_id":"release-2026"}"#;
    let path = data.join("packages/manifests/fcitx5-rime/1.0.0.json");
    fs::create_dir_all(path.parent().expect("manifest parent")).expect("manifest directory");
    fs::write(&path, manifest).expect("manifest");
    let digest = fcitx5_package_core::sha256_digest(manifest.as_bytes());
    let lock = format!(
        r#"{{"format_version":1,"packages":[{{"id":"fcitx5-rime","version":"1.0.0","manifest_sha256":"{}","state":"enabled"}}]}}"#,
        digest.as_str()
    );
    fs::write(data.join("packages/packages.lock"), lock).expect("lockfile");
}

#[test]
fn catalog_commands_use_real_package_theme_and_addon_files() {
    let fixture = root();
    let install = fixture.join("install");
    let data = fixture.join("data");
    install_package_fixture(&data);

    let theme = data.join("themes/solar/theme.toml");
    fs::create_dir_all(theme.parent().expect("theme parent")).expect("theme directory");
    fs::write(&theme, "format_version = 1\n[theme]\nid = \"solar\"\nname = \"Solar\"\nversion = \"2.4\"\nlicense = \"MIT\"\ndescription = \"Warm\"\n[light.candidate.colors]\nbackground = \"#ffffff\"\n").expect("theme");
    let addon = install.join("share/fcitx5/addon/fixture.conf");
    fs::create_dir_all(addon.parent().expect("addon parent")).expect("addon directory");
    fs::create_dir_all(install.join("lib/fcitx5")).expect("library directory");
    fs::write(&addon, "[Addon]\nName=Fixture\nCategory=InputMethod\nLibrary=fixture\nType=UI\nVersion=7\nConfigurable=True\nOnDemand=True\n").expect("addon");
    fs::write(install.join("lib/fcitx5/fixture.dll"), b"fixture").expect("library");

    let packages = control_packages_list_json_for_paths(&install, &data).expect("packages list");
    assert_eq!(
        packages,
        r#"{"format_version":1,"repository_available":false,"repository_error":"repository_unavailable","packages":[{"id":"fcitx5-rime","title":"fcitx5-rime","summary":"","type":"unknown","available_version":null,"installed_version":"1.0.0","state":"enabled","update_available":false}]}"#
    );
    let detail = control_package_detail_json_for_paths(&install, &data, "fcitx5-rime")
        .expect("package detail");
    assert!(detail.contains(r#""id":"fcitx5-rime""#));
    assert!(detail.contains(r#""source_commit":"0123456789abcdef""#));
    assert_eq!(
        control_package_detail_json_for_paths(&install, &data, "bad/id"),
        Err("invalid_package")
    );
    assert!(control_themes_list_json_for_paths(&install, &data)
        .expect("themes")
        .contains(r#""id":"solar""#));
    assert!(
        control_theme_detail_json_for_paths(&install, &data, "solar")
            .expect("theme")
            .contains(r#""has_light_branch":true"#)
    );
    assert!(control_addons_list_json_for_path(&install)
        .expect("addons")
        .contains(r#""library_present":true"#));
    let _ = fs::remove_dir_all(fixture);
}

#[test]
fn command_arguments_are_rejected_without_mutating_commands() {
    let binary = std::env::var("CARGO_BIN_EXE_fcitx5-control").expect("control binary");
    for args in [
        vec!["--set-startup"],
        vec!["--set-startup", "maybe"],
        vec!["--packages-detail"],
        vec!["--themes-detail", "--addons-list"],
        vec!["--diagnostics-plan", "extra"],
    ] {
        let output = Command::new(&binary)
            .args(args)
            .output()
            .expect("control starts");
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("Usage: fcitx5-control "));
    }
}
