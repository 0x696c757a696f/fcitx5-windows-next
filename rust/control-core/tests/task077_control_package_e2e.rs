#![cfg(windows)]
#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use fcitx5_package_core::{blake3_digest, read_installed_lockfile, sha256_digest};

const KEY_ID: &str = "official-test-2026-mldsa65";
const REPOSITORY_ID: &str = "fcitx5-windows-next";

struct Fixture {
    root: PathBuf,
    app_root: PathBuf,
    control: PathBuf,
    data_root: PathBuf,
}

impl Fixture {
    fn new(control_source: &Path, downloader_source: &Path) -> Self {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-task077-control-e2e-{}-{}",
            std::process::id(),
            unique_nonce()
        ));
        let app_root = root.join("app");
        let app_bin = app_root.join("bin");
        let data_root = root.join("data");
        fs::create_dir_all(&app_bin).expect("application directory should create");
        fs::create_dir_all(&data_root).expect("data directory should create");
        let control = app_bin.join("fcitx5-control.exe");
        fs::copy(control_source, &control).expect("shipping control should copy");
        fs::copy(downloader_source, app_bin.join("fcitx5-downloader.exe"))
            .expect("shipping downloader should copy");
        fs::create_dir_all(app_root.join("security")).expect("security directory should create");
        write_text(
            &app_root.join("share/fcitx5/addon/pinyin.conf"),
            "[Addon]\nName=Pinyin\nCategory=InputMethod\nVersion=5.1.12\nLibrary=libpinyin\nType=SharedLibrary\nConfigurable=True\nOnDemand=True\n",
        );
        write_text(
            &app_root.join("share/fcitx5/addon/clipboard.conf"),
            "[Addon]\nName=Clipboard\nCategory=Module\nLibrary=libclipboard\nType=SharedLibrary\nConfigurable=False\n",
        );
        write_text(&app_root.join("lib/fcitx5/libpinyin.dll"), "fixture");
        write_text(
            &app_root.join("resources/themes/default/theme.toml"),
            &theme_fixture("builtin.default", "Fcitx5 Default"),
        );
        write_text(
            &data_root.join("themes/eosphoros-night/theme.toml"),
            &theme_fixture("eosphoros-night", "Eosphoros Night"),
        );
        assert!(
            !app_bin.join("fcitx5-launcher.exe").exists(),
            "package lifecycle fixture must not have a launcher service"
        );
        Self {
            root,
            app_root,
            control,
            data_root,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.control);
        command.arg("--data-root").arg(&self.data_root);
        command
    }

    fn run(&self, arguments: &[&str]) -> Output {
        self.command()
            .args(arguments)
            .output()
            .expect("shipping control should start")
    }

    fn run_with_path(&self, arguments: &[&str], path: &Path) -> Output {
        self.command()
            .args(arguments)
            .arg(path)
            .output()
            .expect("shipping control should start")
    }

    fn keyring_path(&self) -> PathBuf {
        self.root.join("trusted-keys.json")
    }

    fn publish_repository(
        &self,
        signer: &Path,
        channel: &str,
        version: &str,
        sequence: u64,
        architecture: &str,
        archive_hash: &str,
    ) {
        let index = repository_index(channel, version, sequence, architecture, archive_hash);
        let index_path = self.root.join(format!("index-{channel}-{sequence}.json"));
        let signature_path = self
            .root
            .join(format!("index-{channel}-{sequence}.sig.json"));
        write_text(&index_path, &index);
        sign(
            signer,
            "repository-index",
            &index_path,
            &signature_path,
            &self.keyring_path(),
        );
        write_text(&self.data_root.join("repository/index.json"), &index);
        copy_file(
            &signature_path,
            &self.data_root.join("repository/index.sig.json"),
        );
        copy_file(
            &self.keyring_path(),
            &self.app_root.join("security/trusted-keys.json"),
        );
    }

    fn publish_package(&self, signer: &Path, version: &str, sequence: u64, payload: &[u8]) {
        let payload_sha256 = sha256_digest(payload);
        let payload_blake3 = blake3_digest(payload);
        let manifest = package_manifest(
            version,
            payload.len(),
            payload_blake3.as_str(),
            payload_sha256.as_str(),
        );
        let manifest_path = self.root.join(format!("manifest-{version}.json"));
        let manifest_signature_path = self.root.join(format!("manifest-{version}.sig.json"));
        write_text(&manifest_path, &manifest);
        sign(
            signer,
            "package-manifest",
            &manifest_path,
            &manifest_signature_path,
            &self.keyring_path(),
        );
        let manifest_signature =
            fs::read(&manifest_signature_path).expect("manifest signature should read");
        let archive = self.root.join(format!("fcitx5-rime-{version}.fcpkg"));
        write_store_zip(
            &archive,
            &[
                ("manifest.json", manifest.as_bytes()),
                ("manifest.sig.json", &manifest_signature),
                ("payload/bin/addon.dll", payload),
            ],
        );
        let archive_hash = sha256_digest(&fs::read(&archive).expect("archive should read"));
        self.publish_repository(
            signer,
            "stable",
            version,
            sequence,
            target_architecture(),
            archive_hash.as_str(),
        );
        write_text(
            &self.data_root.join("repository/sequence-stable.json"),
            &format!("format_version=1\nchannel=stable\nmax_release_sequence={sequence}\n"),
        );
        copy_file(
            &archive,
            &self
                .data_root
                .join(format!("downloads/fcitx5-rime-{version}.fcpkg")),
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
#[ignore = "requires shipping control/downloader and protected test signer paths"]
fn task077_control_owns_signed_package_theme_and_repository_lifecycle() {
    let (control_source, downloader_source, signer) = configured_tools();
    let fixture = Fixture::new(&control_source, &downloader_source);
    fixture.ensure_keyring(&signer);

    let themes = fixture.run(&["--themes-list"]);
    let themes_text = output_text(&themes);
    assert!(themes.status.success(), "theme list failed: {themes_text}");
    assert!(themes_text.contains(r#""id":"builtin:default""#));
    assert!(themes_text.contains(r#""id":"eosphoros-night""#));
    assert!(themes_text.contains(r#""source":"user""#));

    let theme_detail = fixture.run(&["--themes-detail", "eosphoros-night"]);
    let theme_detail_text = output_text(&theme_detail);
    assert!(
        theme_detail.status.success(),
        "theme detail failed: {theme_detail_text}"
    );
    assert!(theme_detail_text.contains(r#""editable_fields""#));
    assert!(theme_detail_text.contains(r#""candidate.colors.background""#));
    assert!(theme_detail_text.contains(r#""network_allowed":false"#));

    let theme_export = fixture.run(&["--themes-export", "eosphoros-night"]);
    let theme_export_text = output_text(&theme_export);
    assert!(
        theme_export.status.success() && theme_export_text.contains("Eosphoros Night"),
        "theme export failed: {theme_export_text}"
    );
    let export_path = fixture.root.join("exported-eosphoros-night.toml");
    let theme_export_to =
        fixture.run_with_path(&["--themes-export-to", "eosphoros-night"], &export_path);
    let theme_export_to_text = output_text(&theme_export_to);
    assert!(
        theme_export_to.status.success()
            && theme_export_to_text.contains(r#""operation":"export""#)
            && read_text(&export_path).contains("Eosphoros Night"),
        "theme export-to failed: {theme_export_to_text}"
    );

    let import_path = fixture.root.join("theme-import.toml");
    write_text(&import_path, &theme_fixture("soft-blue", "Soft Blue"));
    let theme_import = fixture.run_with_path(&["--themes-import"], &import_path);
    let theme_import_text = output_text(&theme_import);
    assert!(
        theme_import.status.success()
            && theme_import_text.contains(r#""operation":"import""#)
            && fixture
                .data_root
                .join("themes/soft-blue/theme.toml")
                .is_file(),
        "theme import failed: {theme_import_text}"
    );

    let theme_duplicate = fixture
        .command()
        .args(["--themes-duplicate", "builtin:default", "default-copy"])
        .output()
        .expect("shipping control should start");
    let theme_duplicate_text = output_text(&theme_duplicate);
    assert!(
        theme_duplicate.status.success()
            && theme_duplicate_text.contains(r#""operation":"duplicate""#)
            && fixture
                .data_root
                .join("themes/default-copy/theme.toml")
                .is_file(),
        "theme duplicate failed: {theme_duplicate_text}"
    );

    let readonly_delete = fixture.run(&["--themes-delete", "builtin:default"]);
    assert!(
        !readonly_delete.status.success(),
        "builtin theme deletion must be rejected: {}",
        output_text(&readonly_delete)
    );
    let delete = fixture.run(&["--themes-delete", "soft-blue"]);
    let delete_text = output_text(&delete);
    assert!(
        delete.status.success()
            && delete_text.contains(r#""operation":"delete""#)
            && !fixture.data_root.join("themes/soft-blue").exists(),
        "user theme deletion failed: {delete_text}"
    );

    let config_source = fixture.root.join("candidate-font-config.toml");
    write_text(
        &config_source,
        "format_version = 1\n[fonts.candidate]\nfamilies = [\"Segoe UI Emoji\", \"system\"]\nsize_dip = 20.0\n",
    );
    let config_validate = fixture.run_with_path(&["--validate-config"], &config_source);
    assert!(
        config_validate.status.success(),
        "typed Config validation failed: {}",
        output_text(&config_validate)
    );
    let config_apply = fixture.run_with_path(&["--apply-config"], &config_source);
    assert!(
        config_apply.status.success(),
        "typed Config apply failed with launcher stopped: {}",
        output_text(&config_apply)
    );
    let persisted_config = read_text(&fixture.data_root.join("config.toml"));
    assert!(persisted_config.contains("Segoe UI Emoji"));
    assert!(persisted_config.contains("size_dip = 20.0"));

    let addons = fixture.run(&["--addons-list"]);
    let addons_text = output_text(&addons);
    assert!(
        addons.status.success(),
        "addon inventory failed: {addons_text}"
    );
    assert!(addons_text.contains(r#""surface":"descriptor-inventory""#));
    assert!(addons_text.contains(r#""typed_config":"not_available""#));
    assert!(addons_text.contains(r#""id":"pinyin""#));
    assert!(addons_text.contains(r#""category":"InputMethod""#));
    assert!(addons_text.contains(r#""configurable":true"#));
    assert!(addons_text.contains(r#""library_present":true"#));
    assert!(addons_text.contains(r#""id":"clipboard""#));

    fixture.publish_package(
        &signer,
        "1.0.0",
        1,
        b"verified control package fixture v1\n",
    );
    let install = fixture.run(&["--packages-install", "fcitx5-rime"]);
    let install_text = output_text(&install);
    assert!(
        install.status.success(),
        "package install failed with launcher stopped: {install_text}"
    );
    let installed = read_installed_lockfile(fixture.data_root.join("packages"))
        .expect("installed lockfile should parse");
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].id().as_str(), "fcitx5-rime");
    assert_eq!(installed[0].version(), "1.0.0");
    assert_eq!(installed[0].state().as_str(), "installed");
    assert!(fixture
        .data_root
        .join("packages/versions/fcitx5-rime/1.0.0/bin/addon.dll")
        .is_file());

    let detail = fixture.run(&["--packages-detail", "fcitx5-rime"]);
    let detail_text = output_text(&detail);
    assert!(
        detail.status.success(),
        "package detail failed: {detail_text}"
    );
    assert!(detail_text.contains(r#""permissions":["native-code","input-data"]"#));
    assert!(detail_text.contains(r#""source_commit":"0123456789abcdef""#));
    assert!(detail_text.contains(r#""kind":"fcitx-addon""#));

    let disable = fixture.run(&["--packages-state", "fcitx5-rime", "disabled"]);
    assert!(
        disable.status.success(),
        "package disable failed: {}",
        output_text(&disable)
    );
    assert_eq!(
        read_installed_lockfile(fixture.data_root.join("packages"))
            .expect("disabled lockfile should parse")[0]
            .state()
            .as_str(),
        "disabled"
    );
    let enable = fixture.run(&["--packages-state", "fcitx5-rime", "enabled"]);
    assert!(
        enable.status.success(),
        "package enable failed: {}",
        output_text(&enable)
    );
    assert_eq!(
        read_installed_lockfile(fixture.data_root.join("packages"))
            .expect("enabled lockfile should parse")[0]
            .state()
            .as_str(),
        "enabled"
    );

    fixture.publish_package(
        &signer,
        "1.1.0",
        2,
        b"verified control package fixture v2\n",
    );
    let update = fixture.run(&["--packages-update", "fcitx5-rime"]);
    assert!(
        update.status.success(),
        "package update failed: {}",
        output_text(&update)
    );
    let updated = read_installed_lockfile(fixture.data_root.join("packages"))
        .expect("updated lockfile should parse");
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].version(), "1.1.0");
    assert_eq!(updated[0].state().as_str(), "installed");
    assert_eq!(
        fs::read(
            fixture
                .data_root
                .join("packages/versions/fcitx5-rime/1.1.0/bin/addon.dll")
        )
        .expect("updated payload should read"),
        b"verified control package fixture v2\n"
    );

    let repair = fixture.run(&["--packages-repair"]);
    assert!(
        repair.status.success(),
        "package repair failed: {}",
        output_text(&repair)
    );
    let user_data = fixture.data_root.join("rime/user.dict.yaml");
    write_text(&user_data, "irreplaceable user dictionary\n");
    let remove = fixture.run(&["--packages-remove", "fcitx5-rime"]);
    assert!(
        remove.status.success(),
        "package removal failed: {}",
        output_text(&remove)
    );
    assert!(read_installed_lockfile(fixture.data_root.join("packages"))
        .expect("empty lockfile should parse")
        .is_empty());
    assert!(!fixture
        .data_root
        .join("packages/versions/fcitx5-rime")
        .exists());
    assert_eq!(read_text(&user_data), "irreplaceable user dictionary\n");

    repository_rollback_contract(&fixture, &signer);
}

impl Fixture {
    fn ensure_keyring(&self, signer: &Path) {
        self.publish_repository(signer, "stable", "1.0.0", 1, "any", &"0".repeat(64));
        let _ = fs::remove_dir_all(self.data_root.join("repository"));
    }
}

fn repository_rollback_contract(fixture: &Fixture, signer: &Path) {
    let repository = fixture.data_root.join("repository");
    let _ = fs::remove_dir_all(&repository);
    let _ = fs::remove_dir_all(fixture.data_root.join("packages"));
    fs::create_dir_all(fixture.data_root.join("packages"))
        .expect("empty package root should exist for repair");

    let first_run = fixture.run(&["--packages-list"]);
    let first_run_text = output_text(&first_run);
    assert!(
        first_run.status.success(),
        "first-run list failed: {first_run_text}"
    );
    assert!(first_run_text.contains(r#""repository_available":false"#));

    fixture.publish_repository(signer, "beta", "1.0.0", 10, "any", &"1".repeat(64));
    let beta = fixture.run(&["--packages-list"]);
    assert!(
        !output_text(&beta).contains(r#""repository_available":true"#),
        "stable control accepted a signed beta repository: {}",
        output_text(&beta)
    );

    let reset = fixture.run(&["--packages-repair"]);
    let reset_text = output_text(&reset);
    assert!(
        reset.status.success() && reset_text.contains(r#""repository_sequence_state":"reset""#)
    );
    let reset_list = fixture.run(&["--packages-list"]);
    assert!(!output_text(&reset_list).contains(r#""repository_available":true"#));

    fixture.publish_repository(signer, "stable", "1.0.0", 8, "any", &"2".repeat(64));
    let _ = fs::remove_file(repository.join("sequence-stable.json"));
    let missing_sequence = fixture.run(&["--packages-list"]);
    assert!(!output_text(&missing_sequence).contains(r#""repository_available":true"#));
    let repaired = fixture.run(&["--packages-repair"]);
    let repaired_text = output_text(&repaired);
    assert!(
        repaired.status.success()
            && repaired_text.contains(r#""repository_sequence_state":"repaired""#)
            && read_text(&repository.join("sequence-stable.json"))
                .contains("max_release_sequence=8\n")
    );

    write_text(
        &repository.join("sequence-stable.json.new"),
        "format_version=1\nchannel=stable\nmax_release_sequence=0\n",
    );
    let orphan = fixture.run(&["--packages-list"]);
    assert!(output_text(&orphan).contains(r#""repository_available":true"#));

    fixture.publish_repository(signer, "stable", "1.0.0", 3, "any", &"3".repeat(64));
    write_text(
        &repository.join("sequence-stable.json"),
        "format_version=1\nchannel=stable\nmax_release_sequence=8\n",
    );
    let rollback = fixture.run(&["--packages-list"]);
    assert!(!output_text(&rollback).contains(r#""repository_available":true"#));

    let _ = fs::remove_file(repository.join("sequence-stable.json"));
    let missing_after_acceptance = fixture.run(&["--packages-list"]);
    assert!(!output_text(&missing_after_acceptance).contains(r#""repository_available":true"#));

    write_text(
        &repository.join("sequence-stable.json"),
        "format_version=1\nchannel=stable\nmax_release_sequence=\n",
    );
    let truncated = fixture.run(&["--packages-list"]);
    assert!(!output_text(&truncated).contains(r#""repository_available":true"#));

    write_text(
        &repository.join("sequence-stable.json"),
        "format_version=1\nchannel=stable\nmax_release_sequence=not-a-number\n",
    );
    let corrupt = fixture.run(&["--packages-list"]);
    assert!(!output_text(&corrupt).contains(r#""repository_available":true"#));

    fixture.publish_repository(signer, "stable", "1.0.0", 9, "any", &"4".repeat(64));
    let repaired_again = fixture.run(&["--packages-repair"]);
    let repaired_again_text = output_text(&repaired_again);
    assert!(
        repaired_again.status.success()
            && repaired_again_text.contains(r#""repository_sequence_state":"repaired""#)
            && read_text(&repository.join("sequence-stable.json"))
                .contains("max_release_sequence=9\n")
    );
    let fresh = fixture.run(&["--packages-list"]);
    assert!(output_text(&fresh).contains(r#""repository_available":true"#));
}

fn configured_tools() -> (PathBuf, PathBuf, PathBuf) {
    let control = configured_path(
        "FCITX5_TASK077_CONTROL",
        std::env::var_os("CARGO_BIN_EXE_fcitx5-control").map(PathBuf::from),
        "set FCITX5_TASK077_CONTROL to the shipping fcitx5-control.exe path",
    );
    let downloader_fallback = control
        .parent()
        .map(|path| path.join("fcitx5-downloader.exe"));
    let downloader = configured_path(
        "FCITX5_TASK077_DOWNLOADER",
        downloader_fallback,
        "set FCITX5_TASK077_DOWNLOADER to the shipping fcitx5-downloader.exe path",
    );
    let signer_fallback = std::env::var_os("FCITX_PQC_FIXTURE_SIGNER").map(PathBuf::from);
    let signer = configured_path(
        "FCITX5_TASK077_SIGNER",
        signer_fallback,
        "set FCITX5_TASK077_SIGNER or FCITX_PQC_FIXTURE_SIGNER to the protected test signer path",
    );
    (control, downloader, signer)
}

fn configured_path(name: &str, fallback: Option<PathBuf>, guidance: &str) -> PathBuf {
    let path = std::env::var_os(name)
        .map(PathBuf::from)
        .or(fallback)
        .unwrap_or_else(|| panic!("Task 077 E2E tool is unavailable: {guidance}"));
    assert!(
        path.is_file(),
        "Task 077 E2E tool is unavailable: {} does not name a file ({})",
        name,
        path.display()
    );
    path
}

fn sign(signer: &Path, object: &str, input: &Path, signature: &Path, keyring: &Path) {
    let output = Command::new(signer)
        .args(["--sign", object])
        .arg(input)
        .arg(signature)
        .arg(keyring)
        .arg(KEY_ID)
        .output()
        .expect("protected test signer should start");
    assert!(
        output.status.success(),
        "protected test signer failed for {object}: {}",
        output_text(&output)
    );
}

fn package_manifest(version: &str, size: usize, blake3: &str, sha256: &str) -> String {
    format!(
        "{{\"format_version\":2,\"id\":\"fcitx5-rime\",\"version\":\"{version}\",\"type\":\"addon\",\"architecture\":\"{}\",\"min_os\":\"6.1-sp1\",\"core_api\":\"1\",\"addon_abi\":\"1\",\"dependencies\":[],\"license\":\"MIT\",\"source_commit\":\"0123456789abcdef\",\"runtime_abi\":\"1\",\"runtime_build\":\"0123456789abcdef+tools/bootstrap-fcitx.ps1\",\"source\":{{\"repository\":\"https://github.com/fcitx/fcitx5-rime.git\",\"commit\":\"0123456789abcdef\",\"build_script\":\"tools/bootstrap-fcitx.ps1\"}},\"data_policy\":{{\"program\":\"versioned\",\"user_data\":\"durable\"}},\"permissions\":[\"native-code\",\"input-data\"],\"payload\":[{{\"path\":\"bin/addon.dll\",\"size\":{size},\"hashes\":{{\"blake3\":\"{blake3}\",\"sha256\":\"{sha256}\"}}}}],\"key_id\":\"{KEY_ID}\"}}",
        target_architecture()
    )
}

fn repository_index(
    channel: &str,
    version: &str,
    sequence: u64,
    architecture: &str,
    archive_hash: &str,
) -> String {
    let now = unix_now_seconds();
    let generated_at = format_repository_time(now.saturating_sub(60));
    let expires_at = format_repository_time(now.saturating_add(24 * 60 * 60));
    let target = format!("fcitx5-rime\t{version}\t{sequence}\t{architecture}\t{archive_hash}\n");
    let targets_hash = sha256_digest(target.as_bytes());
    format!(
        "{{\"format_version\":1,\"channel\":\"{channel}\",\"repository_id\":\"{REPOSITORY_ID}\",\"mirror_id\":\"official\",\"sequence\":{sequence},\"generated_at\":\"{generated_at}\",\"expires_at\":\"{expires_at}\",\"key_id\":\"{KEY_ID}\",\"targets\":{{\"count\":1,\"sha256\":\"{}\"}},\"packages\":[{{\"id\":\"fcitx5-rime\",\"title\":\"Rime\",\"summary\":\"Rime input engine\",\"version\":\"{version}\",\"release_sequence\":{sequence},\"type\":\"addon\",\"architecture\":\"{architecture}\",\"download_url\":\"https://packages.example.invalid/fcitx5-rime.fcpkg\",\"sha256\":\"{archive_hash}\",\"dependencies\":[]}}]}}",
        targets_hash.as_str()
    )
}

fn unix_now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("fixture clock should be after epoch")
        .as_secs()
}

fn format_repository_time(seconds: u64) -> String {
    let days = seconds / 86_400;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_date_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds_of_day / 3_600,
        (seconds_of_day % 3_600) / 60,
        seconds_of_day % 60
    )
}

fn civil_date_from_days(days_since_epoch: u64) -> (u64, u64, u64) {
    let days = i64::try_from(days_since_epoch).expect("fixture date should fit i64") + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month + 2) / 5 + 1;
    let year = year + if month < 10 { 0 } else { 1 };
    let month = month + if month < 10 { 3 } else { -9 };
    (
        u64::try_from(year).expect("fixture year should fit u64"),
        u64::try_from(month).expect("fixture month should fit u64"),
        u64::try_from(day).expect("fixture day should fit u64"),
    )
}

fn theme_fixture(id: &str, name: &str) -> String {
    format!(
        "format_version = 1\n[theme]\nid = \"{id}\"\nname = \"{name}\"\nversion = \"1.0.0\"\nlicense = \"MIT\"\ndescription = \"Theme fixture\"\n[common.candidate]\norientation = \"vertical\"\n[common.fonts.candidate]\nfamilies = [\"Microsoft YaHei\", \"system\"]\nsize_dip = 18.0\n[light.candidate.colors]\nbackground = \"#FFFFFFFF\"\ncandidate_text = \"#222222FF\"\n[dark.candidate.colors]\nbackground = \"#222222FF\"\ncandidate_text = \"#FFFFFFFF\"\n"
    )
}

fn target_architecture() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "x64"
    }
}

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture parent should create");
    }
    fs::write(path, text).expect("fixture text should write");
}

fn read_text(path: &Path) -> String {
    String::from_utf8(fs::read(path).expect("fixture text should read"))
        .expect("fixture text should be UTF-8")
}

fn copy_file(source: &Path, destination: &Path) {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).expect("fixture destination parent should create");
    }
    fs::copy(source, destination).expect("fixture file should copy");
}

fn output_text(output: &Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    text
}

fn unique_nonce() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("fixture clock should be after epoch")
        .as_nanos()
}

fn write_store_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let mut archive = Vec::new();
    let mut central_directory = Vec::new();
    for (name, contents) in entries {
        let offset = u32::try_from(archive.len()).expect("fixture archive should fit ZIP32");
        let name_bytes = name.as_bytes();
        let name_length = u16::try_from(name_bytes.len()).expect("fixture name should fit ZIP32");
        let content_length =
            u32::try_from(contents.len()).expect("fixture payload should fit ZIP32");
        let crc = crc32(contents);
        push_u32(&mut archive, 0x0403_4b50);
        push_u16(&mut archive, 20);
        push_u16(&mut archive, 0);
        push_u16(&mut archive, 0);
        push_u16(&mut archive, 0);
        push_u16(&mut archive, 0);
        push_u32(&mut archive, crc);
        push_u32(&mut archive, content_length);
        push_u32(&mut archive, content_length);
        push_u16(&mut archive, name_length);
        push_u16(&mut archive, 0);
        archive.extend_from_slice(name_bytes);
        archive.extend_from_slice(contents);

        push_u32(&mut central_directory, 0x0201_4b50);
        push_u16(&mut central_directory, 20);
        push_u16(&mut central_directory, 20);
        push_u16(&mut central_directory, 0);
        push_u16(&mut central_directory, 0);
        push_u16(&mut central_directory, 0);
        push_u16(&mut central_directory, 0);
        push_u32(&mut central_directory, crc);
        push_u32(&mut central_directory, content_length);
        push_u32(&mut central_directory, content_length);
        push_u16(&mut central_directory, name_length);
        push_u16(&mut central_directory, 0);
        push_u16(&mut central_directory, 0);
        push_u16(&mut central_directory, 0);
        push_u16(&mut central_directory, 0);
        push_u32(&mut central_directory, 0);
        push_u32(&mut central_directory, offset);
        central_directory.extend_from_slice(name_bytes);
    }
    let central_offset = u32::try_from(archive.len()).expect("fixture archive should fit ZIP32");
    let central_size =
        u32::try_from(central_directory.len()).expect("fixture directory should fit ZIP32");
    let entry_count = u16::try_from(entries.len()).expect("fixture entry count should fit ZIP32");
    archive.extend_from_slice(&central_directory);
    push_u32(&mut archive, 0x0605_4b50);
    push_u16(&mut archive, 0);
    push_u16(&mut archive, 0);
    push_u16(&mut archive, entry_count);
    push_u16(&mut archive, entry_count);
    push_u32(&mut archive, central_size);
    push_u32(&mut archive, central_offset);
    push_u16(&mut archive, 0);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture archive parent should create");
    }
    fs::write(path, archive).expect("fixture archive should write");
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}
