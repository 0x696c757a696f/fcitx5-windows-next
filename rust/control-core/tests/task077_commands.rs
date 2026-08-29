#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fcitx5_control_core::{
    control_addons_list_json_for_path, control_package_detail_json_for_paths,
    control_packages_list_json_for_paths, control_theme_delete,
    control_theme_detail_json_for_paths, control_theme_duplicate_for_paths,
    control_theme_export_for_paths, control_theme_export_to_for_paths,
    control_theme_import_for_paths, control_themes_list_json_for_paths,
};

fn root() -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("fixture clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "fcitx5-control-task077-{}-{nonce}",
        std::process::id()
    ));
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
    assert_eq!(
        fcitx5_package_core::read_installed_lockfile(data.join("packages"))
            .expect("lockfile reads")
            .len(),
        1
    );

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
        vec!["--themes-export", "--addons-list"],
        vec!["--themes-export-to", "default", "--addons-list"],
        vec!["--themes-import", "--addons-list"],
        vec!["--themes-duplicate", "default", "--addons-list"],
        vec!["--themes-delete", "--addons-list"],
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

#[test]
fn theme_mutations_are_validated_atomic_and_scoped() {
    let fixture = root();
    let install = fixture.join("install");
    let data = fixture.join("data");
    let builtin = install.join("resources/themes/default/theme.toml");
    fs::create_dir_all(builtin.parent().expect("builtin parent")).expect("builtin directory");
    let theme = "format_version = 1\n[theme]\nid = \"default\"\nname = \"Default\"\nversion = \"1\"\nlicense = \"MIT\"\ndescription = \"Built in\"\n[light.candidate.colors]\nbackground = \"#ffffff\"\n";
    fs::write(&builtin, theme).expect("builtin theme");

    let imported = fixture.join("import.toml");
    fs::write(
        &imported,
        theme
            .replace("id = \"default\"", "id = \"solar\"")
            .replace("Default", "Solar"),
    )
    .expect("import theme");
    let imported_result = control_theme_import_for_paths(&data, &imported).expect("import");
    assert_eq!(
        imported_result,
        r#"{"format_version":1,"operation":"import","theme":"solar","result":"ok"}"#
    );

    let exported = control_theme_export_for_paths(&install, &data, "solar").expect("export");
    assert!(exported.contains("name = \"Solar\""));
    let output = fixture.join("exported/theme.toml");
    assert_eq!(
        control_theme_export_to_for_paths(&install, &data, "solar", &output).expect("export to"),
        r#"{"format_version":1,"operation":"export","theme":"solar","result":"ok"}"#
    );
    assert_eq!(
        fs::read_to_string(&output).expect("exported file"),
        exported
    );

    let duplicate = control_theme_duplicate_for_paths(&install, &data, "builtin:default", "copy")
        .expect("duplicate");
    assert!(duplicate.contains(r#""operation":"duplicate"#));
    assert!(data.join("themes/copy/theme.toml").is_file());
    assert_eq!(
        control_theme_delete(&data, "copy").expect("delete"),
        r#"{"format_version":1,"operation":"delete","theme":"copy","result":"ok"}"#
    );
    assert!(!data.join("themes/copy").exists());
    assert_eq!(
        control_theme_delete(&data, "builtin:default"),
        Err("theme_read_only")
    );
    assert_eq!(
        control_theme_export_for_paths(&install, &data, "../escape"),
        Err("invalid_theme")
    );
    assert_eq!(
        control_theme_duplicate_for_paths(&install, &data, "solar", "../escape"),
        Err("invalid_theme")
    );
    let _ = fs::remove_dir_all(fixture);
}

#[test]
fn reset_config_cli_replaces_current_with_config_core_defaults() {
    let fixture = root();
    let config = fixture.join("config.toml");
    fs::write(
        &config,
        "format_version = 1\n[appearance]\ntheme = \"solar\"\n",
    )
    .expect("config");
    let binary = std::env::var("CARGO_BIN_EXE_fcitx5-control").expect("control binary");
    let output = Command::new(binary)
        .args([
            "--data-root",
            fixture.to_str().expect("path"),
            "--reset-config",
        ])
        .output()
        .expect("reset starts");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(config).expect("reset config"),
        "format_version = 1\n"
    );
    let _ = fs::remove_dir_all(fixture);
}
