#![deny(unsafe_op_in_unsafe_fn)]
// 084: per-site SAFETY documentation is enforced by clippy; keep it green.
#![warn(clippy::undocumented_unsafe_blocks)]
use std::ffi::{c_void, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::ptr::null_mut;

type Dword = u32;
type Lstatus = i32;
type Hkey = *mut c_void;

const ERROR_SUCCESS: Lstatus = 0;
const ERROR_FILE_NOT_FOUND: Lstatus = 2;
const KEY_QUERY_VALUE: Dword = 0x0001;
const KEY_SET_VALUE: Dword = 0x0002;
const REG_SZ: Dword = 1;
const HKEY_CURRENT_USER: Hkey = 0x8000_0001_usize as Hkey;
const RUN_KEY: &[u16] = &[
    b'S' as u16,
    b'o' as u16,
    b'f' as u16,
    b't' as u16,
    b'w' as u16,
    b'a' as u16,
    b'r' as u16,
    b'e' as u16,
    b'\\' as u16,
    b'M' as u16,
    b'i' as u16,
    b'c' as u16,
    b'r' as u16,
    b'o' as u16,
    b's' as u16,
    b'o' as u16,
    b'f' as u16,
    b't' as u16,
    b'\\' as u16,
    b'W' as u16,
    b'i' as u16,
    b'n' as u16,
    b'd' as u16,
    b'o' as u16,
    b'w' as u16,
    b's' as u16,
    b'\\' as u16,
    b'C' as u16,
    b'u' as u16,
    b'r' as u16,
    b'r' as u16,
    b'e' as u16,
    b'n' as u16,
    b't' as u16,
    b'V' as u16,
    b'e' as u16,
    b'r' as u16,
    b's' as u16,
    b'i' as u16,
    b'o' as u16,
    b'n' as u16,
    b'\\' as u16,
    b'R' as u16,
    b'u' as u16,
    b'n' as u16,
    0,
];
const CONTROL_SCHEMA_JSON: &str = concat!(
    r#"{"format_version":1,"commands":["#,
    r#""status","diagnostics_plan","restart_engine","shutdown","validate_config","apply_config","#,
    r#""reset_config","get_startup","set_startup","#,
    r#""get_input_methods","set_input_method","#,
    r#""themes_list","themes_detail","themes_export","themes_export_to","themes_import","#,
    r#""themes_duplicate","themes_delete","addons_list","packages_list","packages_detail","#,
    r#""packages_refresh","packages_install","packages_update","packages_state","#,
    r#""packages_remove","packages_repair","get_tsf_guard","reset_tsf_guard"],"#,
    r#""sensitive_input":false,"package_network_owner":"fcitx5-downloader.exe"}"#
);
const CONTROL_USAGE_TEXT: &str = concat!(
    "Usage: fcitx5-control [--data-root PATH] ",
    "--status|--diagnostics-plan|--restart-engine|--validate-config FILE|--apply-config FILE|",
    "--reset-config|--get-startup|--set-startup enabled|disabled|",
    "--get-input-methods|--set-input-method ID|--shutdown|",
    "--themes-list|--themes-detail ID|--themes-export ID|--themes-export-to ID FILE|",
    "--themes-import FILE|",
    "--themes-duplicate SOURCE_ID NEW_ID|--themes-delete ID|",
    "--addons-list|",
    "--packages-list|--packages-detail ID|--packages-refresh [HTTPS_BASE]|",
    "--packages-install ID|--packages-update ID|",
    "--packages-state ID enabled|disabled|--packages-remove ID|",
    "--packages-repair|--get-tsf-guard|--reset-tsf-guard|--schema|--version\n"
);
const CONTROL_FILE_READ_INVALID_FILE: i32 = 1;
const CONTROL_FILE_READ_IO_ERROR: i32 = 2;
const LANG_CHINESE: u16 = 0x04;
const LANG_JAPANESE: u16 = 0x11;
const LANG_KOREAN: u16 = 0x12;
const LANG_THAI: u16 = 0x1e;
const LANG_VIETNAMESE: u16 = 0x2a;
const LANG_SINHALA: u16 = 0x5b;
const SUBLANG_CHINESE_TRADITIONAL: u16 = 0x01;
const SUBLANG_CHINESE_SIMPLIFIED: u16 = 0x02;
const SUBLANG_CHINESE_HONG_KONG: u16 = 0x03;
const SUBLANG_CHINESE_SINGAPORE: u16 = 0x04;
const SUBLANG_CHINESE_MACAO: u16 = 0x05;

const fn ascii_wide<const N: usize>(value: &[u8; N]) -> [u16; N] {
    let mut result = [0; N];
    let mut index = 0;
    while index < N {
        result[index] = value[index] as u16;
        index += 1;
    }
    result
}

const CONFIG_LOCALE_EN_US_FILE: &[u16] = &ascii_wide(b"en-US.json");
const CONFIG_LOCALE_ZH_CN_FILE: &[u16] = &ascii_wide(b"zh-CN.json");
const CONFIG_LOCALE_ZH_TW_FILE: &[u16] = &ascii_wide(b"zh-TW.json");
const CONFIG_LOCALE_JA_JP_FILE: &[u16] = &ascii_wide(b"ja-JP.json");
const CONFIG_LOCALE_KO_KR_FILE: &[u16] = &ascii_wide(b"ko-KR.json");
const CONFIG_LOCALE_VI_VN_FILE: &[u16] = &ascii_wide(b"vi-VN.json");
const CONFIG_LOCALE_TH_TH_FILE: &[u16] = &ascii_wide(b"th-TH.json");
const CONFIG_LOCALE_SI_LK_FILE: &[u16] = &ascii_wide(b"si-LK.json");
const CONFIG_LOCALE_FILES: &[(&str, &[u16])] = &[
    (
        fcitx5_config_core::UI_LANGUAGE_EN_US,
        CONFIG_LOCALE_EN_US_FILE,
    ),
    (
        fcitx5_config_core::UI_LANGUAGE_ZH_CN,
        CONFIG_LOCALE_ZH_CN_FILE,
    ),
    (
        fcitx5_config_core::UI_LANGUAGE_ZH_TW,
        CONFIG_LOCALE_ZH_TW_FILE,
    ),
    (
        fcitx5_config_core::UI_LANGUAGE_JA_JP,
        CONFIG_LOCALE_JA_JP_FILE,
    ),
    (
        fcitx5_config_core::UI_LANGUAGE_KO_KR,
        CONFIG_LOCALE_KO_KR_FILE,
    ),
    (
        fcitx5_config_core::UI_LANGUAGE_VI_VN,
        CONFIG_LOCALE_VI_VN_FILE,
    ),
    (
        fcitx5_config_core::UI_LANGUAGE_TH_TH,
        CONFIG_LOCALE_TH_TH_FILE,
    ),
    (
        fcitx5_config_core::UI_LANGUAGE_SI_LK,
        CONFIG_LOCALE_SI_LK_FILE,
    ),
];
const CONTROL_THEME_OPERATION_INVALID: i32 = 1;
const CONTROL_THEME_OPERATION_NOT_FOUND: i32 = 2;
const CONTROL_THEME_OPERATION_IO_ERROR: i32 = 3;
const CONTROL_THEME_OPERATION_READ_ONLY: i32 = 4;
const CONTROL_THEME_OPERATION_UNSAFE_IMPORT: i32 = 5;
const CONTROL_THEME_OPERATION_ALREADY_EXISTS: i32 = 6;

pub fn control_schema_json() -> &'static str {
    CONTROL_SCHEMA_JSON
}

pub fn control_usage_text() -> &'static str {
    CONTROL_USAGE_TEXT
}

/// Resolves the installation root for the current Control executable.
///
/// # Errors
///
/// Returns an error when the current executable path cannot be resolved.
pub fn control_install_root() -> Result<PathBuf, &'static str> {
    let executable = std::env::current_exe().map_err(|_| "unable to resolve installation root")?;
    // A copy under `runtime\<generation>\bin` belongs to the installation two
    // levels further up: `security`, `current.json` and the package data live
    // only there, not in the generation.
    if let Some(root) =
        fcitx5_windows_common_core::engine_client::installation_root_for_module(&executable)
    {
        return Ok(root);
    }
    let directory = executable
        .parent()
        .ok_or("unable to resolve installation root")?;
    Ok(if directory.file_name().is_some_and(|name| name == "bin") {
        directory
            .parent()
            .ok_or("unable to resolve installation root")?
            .to_owned()
    } else {
        directory.to_owned()
    })
}

/// Returns whether `id` is a valid Control input-method identifier.
#[must_use]
pub fn control_input_method_id_valid(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.bytes().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, b'-' | b'_')
        })
}

/// Returns the release repository base URL selected for this Control binary.
#[must_use]
pub fn control_repository_default_base_url() -> String {
    String::from_utf16(&repository_default_base_url(b"").expect("static URL is valid UTF-16"))
        .expect("static URL is valid UTF-16")
}

/// Appends one fixed repository metadata file name to a repository base URL.
///
/// # Errors
///
/// Returns an error when either argument cannot form a safe metadata URL.
pub fn control_repository_metadata_url(
    base_url: &str,
    metadata_name: &str,
) -> Result<String, &'static str> {
    let base_url = base_url.encode_utf16().collect::<Vec<_>>();
    let url = repository_metadata_url(&base_url, metadata_name.as_bytes())
        .ok_or("repository metadata URL is invalid")?;
    String::from_utf16(&url).map_err(|_| "repository metadata URL is invalid")
}

pub fn control_startup_json(enabled: bool) -> String {
    String::from_utf8(startup_json(enabled)).expect("startup JSON is static UTF-8")
}

pub fn control_startup_query() -> Result<bool, &'static str> {
    let executable_directory = std::env::current_exe()
        .map_err(|_| "unable to resolve executable path")?
        .parent()
        .map(PathBuf::from)
        .ok_or("unable to resolve executable directory")?;
    query_startup(
        executable_directory.into_os_string(),
        OsString::from(release_registry_value()),
    )
    .map_err(|()| "unable to query startup")
}

pub fn control_startup_set(enabled: bool) -> Result<(), &'static str> {
    let executable_directory = std::env::current_exe()
        .map_err(|_| "unable to resolve executable path")?
        .parent()
        .map(PathBuf::from)
        .ok_or("unable to resolve executable directory")?;
    set_startup(
        executable_directory.into_os_string(),
        OsString::from(release_registry_value()),
        enabled,
    )
    .map_err(|()| "unable to set startup")
}

fn release_registry_value_for_channel(channel: &str) -> &'static str {
    match channel {
        "beta" => "Fcitx5-Beta",
        "nightly" => "Fcitx5-Nightly",
        _ => "Fcitx5-Stable",
    }
}

pub fn control_release_registry_value() -> &'static str {
    release_registry_value_for_channel(
        option_env!("FCITX_RELEASE_CHANNEL_NAME").unwrap_or("stable"),
    )
}

fn release_registry_value() -> &'static str {
    control_release_registry_value()
}

pub fn control_tsf_guard_json(data_root: &std::path::Path) -> Result<String, &'static str> {
    let (disabled, reason) = tsf_guard_state(data_root)?;
    let marker = data_root
        .join("recovery")
        .join("tsf-activation-disabled.v1");
    let marker = marker.to_string_lossy().into_owned();
    let status = Fcitx5ControlTsfGuard {
        disabled: u8::from(disabled),
        reason: Fcitx5ControlUtf8::new(&reason),
        marker_path: Fcitx5ControlUtf8::new(marker.as_bytes()),
    };
    String::from_utf8(tsf_guard_json(&status).ok_or("unable to format TSF guard")?)
        .map_err(|_| "unable to format TSF guard")
}

fn tsf_guard_state(data_root: &std::path::Path) -> Result<(bool, String), &'static str> {
    let marker = data_root
        .join("recovery")
        .join("tsf-activation-disabled.v1");
    if !marker.is_file() {
        return Ok((false, String::new()));
    }
    let bytes = std::fs::read(marker).map_err(|_| "unable to read TSF activation guard")?;
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]);
    let reason = text
        .split_once("reason=")
        .and_then(|(_, value)| value.lines().next())
        .unwrap_or_default()
        .to_owned();
    Ok((true, reason))
}

pub fn control_tsf_guard_reset(data_root: &std::path::Path) -> Result<(), &'static str> {
    let recovery = data_root.join("recovery");
    let marker = recovery.join("tsf-activation-disabled.v1");
    if let Err(error) = std::fs::remove_file(marker) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err("unable to clear TSF activation guard");
        }
    }
    if let Ok(entries) = std::fs::read_dir(recovery) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if name.starts_with("tsf-activation-attempt.") && name.ends_with(".v1") {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    Ok(())
}

pub fn control_diagnostics_plan_for_path(
    data_root: &std::path::Path,
    config_valid: bool,
) -> Option<String> {
    let (tsf_guard_disabled, tsf_guard_reason) = tsf_guard_state(data_root).ok()?;
    let data_root = data_root.to_string_lossy();
    let empty = Fcitx5ControlUtf8::default();
    let root = Fcitx5ControlUtf8::new(data_root.as_bytes());
    let status = Fcitx5ControlStatus {
        launcher_reachable: 0,
        launcher_state: 0,
        engine_state: 0,
        current_input_method_id: empty,
        current_input_method_name: empty,
        current_input_method_native_name: empty,
        current_input_method_short_label: empty,
        config_valid: u8::from(config_valid),
        tsf_guard_disabled: u8::from(tsf_guard_disabled),
        tsf_guard_reason: Fcitx5ControlUtf8::new(&tsf_guard_reason),
        data_root: root,
        update_owner: empty,
    };
    String::from_utf8(diagnostics_plan_json(&status)?).ok()
}

fn view(value: &str) -> Fcitx5ControlUtf8<'_> {
    Fcitx5ControlUtf8::new(value)
}

fn package_catalog_options<'a>(
    probes: &'a [fcitx5_package_core::BundledPackageProbe],
) -> fcitx5_package_core::PackageCatalogReadOptions<'a> {
    fcitx5_package_core::PackageCatalogReadOptions {
        expected_channel: option_env!("FCITX_RELEASE_CHANNEL_NAME").unwrap_or("stable"),
        architecture: native_package_architecture_str(),
        release_version: option_env!("FCITX_WINDOWS_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
        bundled: probes,
        repository: fcitx5_package_core::PackageCatalogRepositoryRead::Cached,
    }
}

fn native_package_architecture_str() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "x64"
    }
}

fn package_probes() -> Vec<fcitx5_package_core::BundledPackageProbe> {
    BUNDLED_PACKAGES
        .iter()
        .filter_map(|package| {
            Some(fcitx5_package_core::BundledPackageProbe::new(
                fcitx5_package_core::PackageId::parse(package.id).ok()?,
                package.title.to_owned(),
                fcitx5_package_core::SafeRelativePackagePath::parse(package.probe_relative_path)
                    .ok()?,
            ))
        })
        .collect()
}

fn package_summary_row(
    package: &fcitx5_package_core::PackageCatalogEntry,
) -> (String, String, String, String, String, String, String, bool) {
    (
        package.id().to_owned(),
        package.title().to_owned(),
        package.summary().to_owned(),
        package.package_type().as_str().to_owned(),
        package.available_version().unwrap_or_default().to_owned(),
        package.installed_version().unwrap_or_default().to_owned(),
        package.state().unwrap_or_default().to_owned(),
        package.update_available(),
    )
}

fn catalog_summary_json(
    catalog: &fcitx5_package_core::PackageCatalog,
) -> Result<String, &'static str> {
    let rows = catalog
        .packages()
        .iter()
        .map(package_summary_row)
        .collect::<Vec<_>>();
    let views = rows
        .iter()
        .map(|row| Fcitx5ControlPackageSummary {
            id: view(&row.0),
            title: view(&row.1),
            summary: view(&row.2),
            package_type: view(&row.3),
            available_version: view(&row.4),
            installed_version: view(&row.5),
            state: view(&row.6),
            update_available: u8::from(row.7),
        })
        .collect::<Vec<_>>();
    let repository_error = catalog
        .repository_error()
        .map_or("", |error| error.as_str());
    let list = Fcitx5ControlPackagesList {
        repository_available: u8::from(catalog.repository_available()),
        repository_error: view(repository_error),
        packages: &views,
    };
    String::from_utf8(packages_list_json(&list).ok_or("unable to format packages")?)
        .map_err(|_| "unable to format packages")
}

pub fn control_packages_list_json(data_root: &std::path::Path) -> Result<String, &'static str> {
    let install_root = control_install_root()?;
    control_packages_list_json_for_paths(&install_root, data_root)
}

pub fn control_packages_list_json_for_paths(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
) -> Result<String, &'static str> {
    let probes = package_probes();
    let catalog = fcitx5_package_core::read_package_catalog(
        install_root,
        data_root,
        package_catalog_options(&probes),
    )
    .map_err(|error| error.code())?;
    catalog_summary_json(&catalog)
}

fn catalog_detail_json(
    catalog: &fcitx5_package_core::PackageCatalog,
    package: &fcitx5_package_core::PackageCatalogEntry,
) -> Result<String, &'static str> {
    let dependency_views = package
        .dependencies()
        .iter()
        .map(|dependency| Fcitx5ControlPackageDependency {
            id: view(dependency.id()),
            version: view(dependency.version()),
        })
        .collect::<Vec<_>>();
    let dependencies = String::from_utf8(
        package_dependencies_json(&dependency_views).ok_or("unable to format dependencies")?,
    )
    .map_err(|_| "unable to format dependencies")?;
    let permission_views = package
        .permissions()
        .iter()
        .map(|permission| view(permission))
        .collect::<Vec<_>>();
    let permissions = String::from_utf8(
        string_array_json(&permission_views).ok_or("unable to format permissions")?,
    )
    .map_err(|_| "unable to format permissions")?;
    let surface_views = package
        .config_surfaces()
        .iter()
        .map(|surface| view(surface.kind()))
        .collect::<Vec<_>>();
    let config_surface = String::from_utf8(
        config_surfaces_json(view(package.id()), &surface_views)
            .ok_or("unable to format config surfaces")?,
    )
    .map_err(|_| "unable to format config surfaces")?;
    let detail = Fcitx5ControlPackageDetail {
        repository_available: u8::from(catalog.repository_available()),
        repository_error: view(
            catalog
                .repository_error()
                .map_or("", |error| error.as_str()),
        ),
        id: view(package.id()),
        title: view(package.detail_title()),
        summary: view(package.detail_summary()),
        package_type: view(package.detail_package_type().as_str()),
        available_version: view(package.available_version().unwrap_or_default()),
        installed_version: view(package.installed_version().unwrap_or_default()),
        state: view(package.state().unwrap_or_default()),
        bundled: u8::from(package.bundled()),
        update_available: u8::from(package.update_available()),
        manifest_sha256: view(package.manifest_sha256().unwrap_or_default()),
        source_commit: view(package.source_commit().unwrap_or_default()),
        dependencies_json: view(&dependencies),
        permissions_json: view(&permissions),
        config_surface_json: view(&config_surface),
    };
    String::from_utf8(package_detail_json(&detail).ok_or("unable to format package")?)
        .map_err(|_| "unable to format package")
}

type ThemeRecordOwned = (String, String, String, String, String, String, bool, bool);

pub fn control_package_detail_json(
    data_root: &std::path::Path,
    requested_id: &str,
) -> Result<String, &'static str> {
    let install_root = control_install_root()?;
    control_package_detail_json_for_paths(&install_root, data_root, requested_id)
}

pub fn control_package_detail_json_for_paths(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
    requested_id: &str,
) -> Result<String, &'static str> {
    if fcitx5_package_core::PackageId::parse(requested_id).is_err() {
        return Err("invalid_package");
    }
    let probes = package_probes();
    let catalog = fcitx5_package_core::read_package_catalog(
        install_root,
        data_root,
        package_catalog_options(&probes),
    )
    .map_err(|error| error.code())?;
    let package = catalog.package(requested_id).ok_or("package_not_found")?;
    catalog_detail_json(&catalog, package)
}

fn theme_records_for_paths(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
) -> Vec<ThemeRecordOwned> {
    discover_themes(install_root, data_root)
        .into_iter()
        .filter_map(|entry| {
            let text = read_file_bounded(entry.path, 512 * 1024).ok()?;
            let summary = parse_theme_summary(&text)?;
            (summary.id == entry.id || entry.source == "builtin").then_some((
                entry.id,
                entry.source.to_owned(),
                summary.name,
                summary.version,
                summary.license,
                summary.description,
                summary.has_light_branch,
                summary.has_dark_branch,
            ))
        })
        .collect()
}

fn theme_records(data_root: &std::path::Path) -> Vec<ThemeRecordOwned> {
    control_install_root()
        .map(|install_root| theme_records_for_paths(&install_root, data_root))
        .unwrap_or_default()
}

pub fn control_themes_list_json(data_root: &std::path::Path) -> Result<String, &'static str> {
    let records = theme_records(data_root);
    let views: Vec<_> = records
        .iter()
        .map(|record| Fcitx5ControlThemeRecord {
            id: view(&record.0),
            source: view(&record.1),
            name: view(&record.2),
            version: view(&record.3),
            license: view(&record.4),
            description: view(&record.5),
        })
        .collect();
    let json = themes_json(&views).ok_or("unable to format themes")?;
    String::from_utf8(json).map_err(|_| "unable to format themes")
}

pub fn control_themes_list_json_for_paths(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
) -> Result<String, &'static str> {
    let records = theme_records_for_paths(install_root, data_root);
    let views: Vec<_> = records
        .iter()
        .map(|record| Fcitx5ControlThemeRecord {
            id: view(&record.0),
            source: view(&record.1),
            name: view(&record.2),
            version: view(&record.3),
            license: view(&record.4),
            description: view(&record.5),
        })
        .collect();
    String::from_utf8(themes_json(&views).ok_or("unable to format themes")?)
        .map_err(|_| "unable to format themes")
}

pub fn control_theme_detail_json(
    data_root: &std::path::Path,
    requested_id: &str,
) -> Result<String, &'static str> {
    let record = theme_records(data_root)
        .into_iter()
        .find(|record| record.0 == requested_id)
        .ok_or("theme is unknown")?;
    let detail = Fcitx5ControlThemeDetail {
        theme: Fcitx5ControlThemeRecord {
            id: view(&record.0),
            source: view(&record.1),
            name: view(&record.2),
            version: view(&record.3),
            license: view(&record.4),
            description: view(&record.5),
        },
        has_light_branch: u8::from(record.6),
        has_dark_branch: u8::from(record.7),
    };
    String::from_utf8(theme_detail_json(&detail).ok_or("unable to format theme")?)
        .map_err(|_| "unable to format theme")
}

pub fn control_theme_detail_json_for_paths(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
    requested_id: &str,
) -> Result<String, &'static str> {
    let record = theme_records_for_paths(install_root, data_root)
        .into_iter()
        .find(|record| record.0 == requested_id)
        .ok_or("theme is unknown")?;
    let detail = Fcitx5ControlThemeDetail {
        theme: Fcitx5ControlThemeRecord {
            id: view(&record.0),
            source: view(&record.1),
            name: view(&record.2),
            version: view(&record.3),
            license: view(&record.4),
            description: view(&record.5),
        },
        has_light_branch: u8::from(record.6),
        has_dark_branch: u8::from(record.7),
    };
    String::from_utf8(theme_detail_json(&detail).ok_or("unable to format theme")?)
        .map_err(|_| "unable to format theme")
}

pub fn control_addons_list_json() -> Result<String, &'static str> {
    let install_root = control_install_root()?;
    control_addons_list_json_for_path(&install_root)
}

pub fn control_addons_list_json_for_path(
    install_root: &std::path::Path,
) -> Result<String, &'static str> {
    let addon_root = install_root.join("share/fcitx5/addon");
    let library_root = install_root.join("lib/fcitx5");
    let mut records = Vec::new();
    let entries = match std::fs::read_dir(addon_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let json = addons_json(&[]).ok_or("unable to format addons")?;
            return String::from_utf8(json).map_err(|_| "unable to format addons");
        }
        Err(_) => return Err("unable to read addon directory"),
    };
    for entry in entries.flatten() {
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("conf") {
            continue;
        }
        let Ok(text) = read_file_bounded(entry.path(), 64 * 1024) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&text) else {
            continue;
        };
        let mut in_addon = false;
        let mut values = [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        let mut configurable = false;
        let mut on_demand = false;
        for line in text.lines().map(str::trim) {
            if line.starts_with('[') && line.ends_with(']') {
                in_addon = line == "[Addon]";
                continue;
            }
            if !in_addon {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim().to_owned();
            match key.trim() {
                "Name" => values[0] = value,
                "Category" => values[1] = value,
                "Library" => values[2] = value,
                "Type" => values[3] = value,
                "Version" => values[4] = value,
                "Configurable" => configurable = addon_metadata_bool(value.as_bytes()),
                "OnDemand" => on_demand = addon_metadata_bool(value.as_bytes()),
                _ => {}
            }
        }
        let id = entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned();
        if !theme_id_valid(&id) || values[0].is_empty() {
            continue;
        }
        let library_present =
            !values[2].is_empty() && library_root.join(format!("{}.dll", values[2])).is_file();
        records.push((id, values, configurable, on_demand, library_present));
    }
    records.sort_by(|left, right| left.0.cmp(&right.0));
    let views: Vec<_> = records
        .iter()
        .map(
            |(id, values, configurable, on_demand, present)| Fcitx5ControlAddonDescriptor {
                id: view(id),
                name: view(&values[0]),
                category: view(&values[1]),
                library: view(&values[2]),
                addon_type: view(&values[3]),
                version: view(&values[4]),
                configurable: u8::from(*configurable),
                on_demand: u8::from(*on_demand),
                library_present: u8::from(*present),
            },
        )
        .collect();
    String::from_utf8(addons_json(&views).ok_or("unable to format addons")?)
        .map_err(|_| "unable to format addons")
}

pub fn control_config_reset(data_root: &std::path::Path) -> Result<(), &'static str> {
    let store = fcitx5_config_core::FileStore::new();
    let path = data_root.join("config.toml");
    let mut core = fcitx5_config_core::ConfigCore::compiled_defaults();
    core.reset(fcitx5_config_core::ConfigField::All);
    core.apply(&store, &path, fcitx5_config_core::CommitFault::None)
        .map_err(|_| "unable to reset config")
}

fn theme_operation_error(status: i32) -> &'static str {
    match status {
        CONTROL_THEME_OPERATION_INVALID => "invalid_theme",
        CONTROL_THEME_OPERATION_NOT_FOUND => "theme_not_found",
        CONTROL_THEME_OPERATION_ALREADY_EXISTS => "theme_already_exists",
        CONTROL_THEME_OPERATION_UNSAFE_IMPORT => "unsafe_theme",
        CONTROL_THEME_OPERATION_READ_ONLY => "theme_read_only",
        _ => "theme_operation_failed",
    }
}

fn theme_install_root() -> Result<PathBuf, &'static str> {
    control_install_root().map_err(|_| "unable to resolve installation root")
}

pub fn control_theme_export(data_root: &std::path::Path, id: &str) -> Result<String, &'static str> {
    control_theme_export_for_paths(&theme_install_root()?, data_root, id)
}

pub fn control_theme_export_for_paths(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
    id: &str,
) -> Result<String, &'static str> {
    let bytes = export_theme(install_root, data_root, id).map_err(theme_operation_error)?;
    String::from_utf8(bytes).map_err(|_| "theme is not UTF-8")
}

pub fn control_theme_export_to(
    data_root: &std::path::Path,
    id: &str,
    destination: &std::path::Path,
) -> Result<String, &'static str> {
    control_theme_export_to_for_paths(&theme_install_root()?, data_root, id, destination)
}

pub fn control_theme_export_to_for_paths(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
    id: &str,
    destination: &std::path::Path,
) -> Result<String, &'static str> {
    let bytes = export_theme_to_file(install_root, data_root, id, destination)
        .map_err(theme_operation_error)?;
    String::from_utf8(bytes).map_err(|_| "unable to format theme operation")
}

pub fn control_theme_import(
    data_root: &std::path::Path,
    source: &std::path::Path,
) -> Result<String, &'static str> {
    control_theme_import_for_paths(data_root, source)
}

pub fn control_theme_import_for_paths(
    data_root: &std::path::Path,
    source: &std::path::Path,
) -> Result<String, &'static str> {
    let bytes = import_theme(data_root, source).map_err(theme_operation_error)?;
    String::from_utf8(bytes).map_err(|_| "unable to format theme operation")
}

pub fn control_theme_duplicate(
    data_root: &std::path::Path,
    source_id: &str,
    new_id: &str,
) -> Result<String, &'static str> {
    control_theme_duplicate_for_paths(&theme_install_root()?, data_root, source_id, new_id)
}

pub fn control_theme_duplicate_for_paths(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
    source_id: &str,
    new_id: &str,
) -> Result<String, &'static str> {
    let bytes = duplicate_theme(install_root, data_root, source_id, new_id)
        .map_err(theme_operation_error)?;
    String::from_utf8(bytes).map_err(|_| "unable to format theme operation")
}

pub fn control_theme_delete(data_root: &std::path::Path, id: &str) -> Result<String, &'static str> {
    let bytes = delete_theme(data_root, id).map_err(theme_operation_error)?;
    String::from_utf8(bytes).map_err(|_| "unable to format theme operation")
}

/// Borrowed UTF-8 field of a Control report; `None` marks a value that is absent.
#[derive(Clone, Copy, Default)]
pub struct Fcitx5ControlUtf8<'a> {
    bytes: Option<&'a [u8]>,
}

impl<'a> Fcitx5ControlUtf8<'a> {
    fn new(value: &'a (impl AsRef<[u8]> + ?Sized)) -> Self {
        Self {
            bytes: Some(value.as_ref()),
        }
    }
}

pub struct Fcitx5ControlStatus<'a> {
    launcher_reachable: u8,
    launcher_state: i32,
    engine_state: i32,
    current_input_method_id: Fcitx5ControlUtf8<'a>,
    current_input_method_name: Fcitx5ControlUtf8<'a>,
    current_input_method_native_name: Fcitx5ControlUtf8<'a>,
    current_input_method_short_label: Fcitx5ControlUtf8<'a>,
    config_valid: u8,
    tsf_guard_disabled: u8,
    tsf_guard_reason: Fcitx5ControlUtf8<'a>,
    data_root: Fcitx5ControlUtf8<'a>,
    update_owner: Fcitx5ControlUtf8<'a>,
}

pub struct Fcitx5ControlTsfGuard<'a> {
    disabled: u8,
    reason: Fcitx5ControlUtf8<'a>,
    marker_path: Fcitx5ControlUtf8<'a>,
}

pub struct Fcitx5ControlAddonDescriptor<'a> {
    id: Fcitx5ControlUtf8<'a>,
    name: Fcitx5ControlUtf8<'a>,
    category: Fcitx5ControlUtf8<'a>,
    library: Fcitx5ControlUtf8<'a>,
    addon_type: Fcitx5ControlUtf8<'a>,
    version: Fcitx5ControlUtf8<'a>,
    configurable: u8,
    on_demand: u8,
    library_present: u8,
}

#[derive(Clone, Copy)]
pub struct Fcitx5ControlThemeRecord<'a> {
    id: Fcitx5ControlUtf8<'a>,
    source: Fcitx5ControlUtf8<'a>,
    name: Fcitx5ControlUtf8<'a>,
    version: Fcitx5ControlUtf8<'a>,
    license: Fcitx5ControlUtf8<'a>,
    description: Fcitx5ControlUtf8<'a>,
}

pub struct Fcitx5ControlThemeDetail<'a> {
    theme: Fcitx5ControlThemeRecord<'a>,
    has_light_branch: u8,
    has_dark_branch: u8,
}

pub struct Fcitx5ControlPackageSummary<'a> {
    id: Fcitx5ControlUtf8<'a>,
    title: Fcitx5ControlUtf8<'a>,
    summary: Fcitx5ControlUtf8<'a>,
    package_type: Fcitx5ControlUtf8<'a>,
    available_version: Fcitx5ControlUtf8<'a>,
    installed_version: Fcitx5ControlUtf8<'a>,
    state: Fcitx5ControlUtf8<'a>,
    update_available: u8,
}

pub struct Fcitx5ControlPackagesList<'a> {
    repository_available: u8,
    repository_error: Fcitx5ControlUtf8<'a>,
    packages: &'a [Fcitx5ControlPackageSummary<'a>],
}

pub struct Fcitx5ControlPackageDependency<'a> {
    id: Fcitx5ControlUtf8<'a>,
    version: Fcitx5ControlUtf8<'a>,
}

pub struct Fcitx5ControlPackageDetail<'a> {
    repository_available: u8,
    repository_error: Fcitx5ControlUtf8<'a>,
    id: Fcitx5ControlUtf8<'a>,
    title: Fcitx5ControlUtf8<'a>,
    summary: Fcitx5ControlUtf8<'a>,
    package_type: Fcitx5ControlUtf8<'a>,
    available_version: Fcitx5ControlUtf8<'a>,
    installed_version: Fcitx5ControlUtf8<'a>,
    state: Fcitx5ControlUtf8<'a>,
    bundled: u8,
    update_available: u8,
    manifest_sha256: Fcitx5ControlUtf8<'a>,
    source_commit: Fcitx5ControlUtf8<'a>,
    dependencies_json: Fcitx5ControlUtf8<'a>,
    permissions_json: Fcitx5ControlUtf8<'a>,
    config_surface_json: Fcitx5ControlUtf8<'a>,
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        h_key: Hkey,
        sub_key: *const u16,
        options: Dword,
        sam_desired: Dword,
        result: *mut Hkey,
    ) -> Lstatus;
    fn RegCreateKeyExW(
        h_key: Hkey,
        sub_key: *const u16,
        reserved: Dword,
        class: *mut u16,
        options: Dword,
        sam_desired: Dword,
        security_attributes: *mut c_void,
        result: *mut Hkey,
        disposition: *mut Dword,
    ) -> Lstatus;
    fn RegQueryValueExW(
        h_key: Hkey,
        value_name: *const u16,
        reserved: *mut Dword,
        value_type: *mut Dword,
        data: *mut u8,
        data_size: *mut Dword,
    ) -> Lstatus;
    fn RegSetValueExW(
        h_key: Hkey,
        value_name: *const u16,
        reserved: Dword,
        value_type: Dword,
        data: *const u8,
        data_size: Dword,
    ) -> Lstatus;
    fn RegDeleteValueW(h_key: Hkey, value_name: *const u16) -> Lstatus;
    fn RegCloseKey(h_key: Hkey) -> Lstatus;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

struct RegistryKey(Hkey);

impl RegistryKey {
    fn get(&self) -> Hkey {
        self.0
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        // SAFETY: RegistryKey exclusively owns this successfully opened HKEY and closes it once.
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

fn wide_z(value: &std::ffi::OsStr) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_wide().collect();
    wide.push(0);
    wide
}

fn primary_lang_id(language: u16) -> u16 {
    language & 0x03ff
}

fn sub_lang_id(language: u16) -> u16 {
    language >> 10
}

fn config_locale_file_for_system_language(language: u16) -> &'static [u16] {
    match primary_lang_id(language) {
        LANG_CHINESE => match sub_lang_id(language) {
            SUBLANG_CHINESE_SIMPLIFIED | SUBLANG_CHINESE_SINGAPORE => CONFIG_LOCALE_ZH_CN_FILE,
            SUBLANG_CHINESE_TRADITIONAL | SUBLANG_CHINESE_HONG_KONG | SUBLANG_CHINESE_MACAO => {
                CONFIG_LOCALE_ZH_TW_FILE
            }
            _ => CONFIG_LOCALE_EN_US_FILE,
        },
        LANG_JAPANESE => CONFIG_LOCALE_JA_JP_FILE,
        LANG_KOREAN => CONFIG_LOCALE_KO_KR_FILE,
        LANG_VIETNAMESE => CONFIG_LOCALE_VI_VN_FILE,
        LANG_THAI => CONFIG_LOCALE_TH_TH_FILE,
        LANG_SINHALA => CONFIG_LOCALE_SI_LK_FILE,
        _ => CONFIG_LOCALE_EN_US_FILE,
    }
}

fn user_default_ui_language_locale_file() -> &'static [u16] {
    // SAFETY: GetUserDefaultUILanguage has no pointer, ownership, or lifetime preconditions.
    config_locale_file_for_system_language(unsafe { GetUserDefaultUILanguage() })
}

/// Returns the canonical supported Settings locale for the Windows display language.
#[must_use]
pub fn current_settings_ui_locale() -> &'static str {
    let file = user_default_ui_language_locale_file();
    CONFIG_LOCALE_FILES
        .iter()
        .find_map(|(locale, candidate)| (*candidate == file).then_some(*locale))
        .unwrap_or(fcitx5_config_core::UI_LANGUAGE_EN_US)
}

fn temporary_path_for_atomic_write(destination: &std::path::Path) -> PathBuf {
    static NEXT_TEMPORARY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut temporary = destination.as_os_str().to_owned();
    temporary.push(format!(
        ".{}-{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    PathBuf::from(temporary)
}

fn atomic_write_utf8_file(destination: PathBuf, text: &[u8]) -> Result<(), ()> {
    use std::io::Write;

    if destination.as_os_str().is_empty() {
        return Err(());
    }
    let parent = destination.parent().ok_or(())?;
    std::fs::create_dir_all(parent).map_err(|_| ())?;
    let temporary = temporary_path_for_atomic_write(&destination);
    let written = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(text)?;
        file.sync_all()
    })();
    if written
        .and_then(|()| std::fs::rename(&temporary, &destination))
        .is_err()
    {
        let _ = std::fs::remove_file(&temporary);
        return Err(());
    }
    Ok(())
}

fn read_file_bounded(path: PathBuf, maximum: u64) -> Result<Vec<u8>, i32> {
    if path.as_os_str().is_empty() {
        return Err(CONTROL_FILE_READ_INVALID_FILE);
    }
    let metadata = std::fs::metadata(&path).map_err(|_| CONTROL_FILE_READ_INVALID_FILE)?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(CONTROL_FILE_READ_INVALID_FILE);
    }
    let bytes = std::fs::read(&path).map_err(|_| CONTROL_FILE_READ_IO_ERROR)?;
    if bytes.len() as u64 > maximum {
        return Err(CONTROL_FILE_READ_INVALID_FILE);
    }
    Ok(bytes)
}

fn quote(value: &std::ffi::OsStr) -> OsString {
    let wide: Vec<u16> = value.encode_wide().collect();
    let mut result = Vec::with_capacity(wide.len() + 2);
    result.push(b'"' as u16);
    let mut backslashes = 0_usize;
    for character in wide {
        if character == b'\\' as u16 {
            backslashes += 1;
        } else if character == b'"' as u16 {
            result.extend(std::iter::repeat_n(b'\\' as u16, backslashes + 1));
            backslashes = 0;
            result.push(character);
        } else {
            result.extend(std::iter::repeat_n(b'\\' as u16, backslashes));
            backslashes = 0;
            result.push(character);
        }
    }
    result.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2));
    result.push(b'"' as u16);
    OsString::from_wide(&result)
}

fn json_string(value: &[u8]) -> Option<Vec<u8>> {
    let mut result = Vec::with_capacity(value.len() + 2);
    result.push(b'"');
    for byte in value {
        match *byte {
            b'\\' => result.extend_from_slice(b"\\\\"),
            b'"' => result.extend_from_slice(br#"\""#),
            b'\x08' => result.extend_from_slice(br#"\b"#),
            b'\x0c' => result.extend_from_slice(br#"\f"#),
            b'\n' => result.extend_from_slice(br#"\n"#),
            b'\r' => result.extend_from_slice(br#"\r"#),
            b'\t' => result.extend_from_slice(br#"\t"#),
            0x00..=0x1f => return None,
            other => result.push(other),
        }
    }
    result.push(b'"');
    Some(result)
}

fn utf8_slice(value: Fcitx5ControlUtf8<'_>) -> Option<&[u8]> {
    value.bytes
}

fn push_json_string_field(output: &mut Vec<u8>, name: &[u8], value: &[u8]) -> Option<()> {
    output.extend_from_slice(name);
    output.push(b':');
    output.extend_from_slice(&json_string(value)?);
    Some(())
}

fn push_json_optional_reachable_string_field(
    output: &mut Vec<u8>,
    name: &[u8],
    reachable: bool,
    value: Fcitx5ControlUtf8<'_>,
) -> Option<()> {
    output.extend_from_slice(name);
    output.push(b':');
    if reachable {
        output.extend_from_slice(&json_string(utf8_slice(value)?)?);
    } else {
        output.extend_from_slice(b"null");
    }
    Some(())
}

fn status_json(status: &Fcitx5ControlStatus) -> Option<Vec<u8>> {
    let reachable = status.launcher_reachable != 0;
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"launcher_reachable":"#);
    output.extend_from_slice(if reachable { b"true" } else { b"false" });
    output.extend_from_slice(b",\"launcher_state\":");
    if reachable {
        output.extend_from_slice(status.launcher_state.to_string().as_bytes());
    } else {
        output.extend_from_slice(b"null");
    }
    output.extend_from_slice(b",\"engine_state\":");
    if reachable {
        output.extend_from_slice(status.engine_state.to_string().as_bytes());
    } else {
        output.extend_from_slice(b"null");
    }
    output.push(b',');
    push_json_optional_reachable_string_field(
        &mut output,
        b"\"current_input_method_id\"",
        reachable,
        status.current_input_method_id,
    )?;
    output.push(b',');
    push_json_optional_reachable_string_field(
        &mut output,
        b"\"current_input_method_name\"",
        reachable,
        status.current_input_method_name,
    )?;
    output.push(b',');
    push_json_optional_reachable_string_field(
        &mut output,
        b"\"current_input_method_native_name\"",
        reachable,
        status.current_input_method_native_name,
    )?;
    output.push(b',');
    push_json_optional_reachable_string_field(
        &mut output,
        b"\"current_input_method_short_label\"",
        reachable,
        status.current_input_method_short_label,
    )?;
    output.extend_from_slice(b",\"config_valid\":");
    output.extend_from_slice(if status.config_valid != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"tsf_guard_disabled\":");
    output.extend_from_slice(if status.tsf_guard_disabled != 0 {
        b"true"
    } else {
        b"false"
    });
    output.push(b',');
    push_json_string_field(
        &mut output,
        b"\"tsf_guard_reason\"",
        utf8_slice(status.tsf_guard_reason)?,
    )?;
    output.push(b',');
    push_json_string_field(&mut output, b"\"data_root\"", utf8_slice(status.data_root)?)?;
    output.push(b',');
    push_json_string_field(
        &mut output,
        b"\"update_owner\"",
        utf8_slice(status.update_owner)?,
    )?;
    output.push(b'}');
    Some(output)
}

#[must_use]
pub fn control_unreachable_status_json(data_root: &str, config_valid: bool) -> Option<String> {
    fn view(bytes: &[u8]) -> Fcitx5ControlUtf8<'_> {
        Fcitx5ControlUtf8::new(bytes)
    }

    let empty = view(&[]);
    let status = Fcitx5ControlStatus {
        launcher_reachable: 0,
        launcher_state: 0,
        engine_state: 0,
        current_input_method_id: empty,
        current_input_method_name: empty,
        current_input_method_native_name: empty,
        current_input_method_short_label: empty,
        config_valid: u8::from(config_valid),
        tsf_guard_disabled: 0,
        tsf_guard_reason: empty,
        data_root: view(data_root.as_bytes()),
        update_owner: empty,
    };
    String::from_utf8(status_json(&status)?).ok()
}

fn push_diagnostics_check(
    output: &mut Vec<u8>,
    first: &mut bool,
    id: &[u8],
    state: &[u8],
    detail: &[u8],
    repair_action: Option<&[u8]>,
) -> Option<()> {
    if !*first {
        output.push(b',');
    }
    *first = false;
    output.push(b'{');
    push_json_string_field(output, b"\"id\"", id)?;
    output.push(b',');
    push_json_string_field(output, b"\"state\"", state)?;
    output.push(b',');
    push_json_string_field(output, b"\"detail\"", detail)?;
    output.extend_from_slice(b",\"repair_action\":");
    if let Some(repair_action) = repair_action {
        output.extend_from_slice(&json_string(repair_action)?);
    } else {
        output.extend_from_slice(b"null");
    }
    output.push(b'}');
    Some(())
}

fn push_diagnostics_action(
    output: &mut Vec<u8>,
    first: &mut bool,
    id: &[u8],
    command: &[u8],
) -> Option<()> {
    if !*first {
        output.push(b',');
    }
    *first = false;
    output.push(b'{');
    push_json_string_field(output, b"\"id\"", id)?;
    output.extend_from_slice(b",\"kind\":\"control\",");
    push_json_string_field(output, b"\"command\"", command)?;
    output.extend_from_slice(b",\"destructive\":false}");
    Some(())
}

fn diagnostics_plan_json(status: &Fcitx5ControlStatus) -> Option<Vec<u8>> {
    let reachable = status.launcher_reachable != 0;
    let config_valid = status.config_valid != 0;
    let tsf_guard_disabled = status.tsf_guard_disabled != 0;
    let overall = if !reachable || !config_valid {
        b"error".as_slice()
    } else if tsf_guard_disabled {
        b"warning".as_slice()
    } else {
        b"ok".as_slice()
    };
    let mut output = Vec::new();
    output.extend_from_slice(
        br#"{"format_version":1,"surface":"diagnostics","sensitive_input":false,"overall":""#,
    );
    output.extend_from_slice(overall);
    output.extend_from_slice(br#"","checks":["#);
    let mut first_check = true;
    push_diagnostics_check(
        &mut output,
        &mut first_check,
        b"launcher",
        if reachable { b"ok" } else { b"error" },
        if reachable {
            b"reachable"
        } else {
            b"unreachable"
        },
        (!reachable).then_some(b"restart_engine".as_slice()),
    )?;
    push_diagnostics_check(
        &mut output,
        &mut first_check,
        b"config",
        if config_valid { b"ok" } else { b"error" },
        if config_valid {
            b"valid"
        } else {
            b"invalid_config"
        },
        (!config_valid).then_some(b"validate_config".as_slice()),
    )?;
    push_diagnostics_check(
        &mut output,
        &mut first_check,
        b"tsf_guard",
        if tsf_guard_disabled {
            b"warning"
        } else {
            b"ok"
        },
        if tsf_guard_disabled {
            utf8_slice(status.tsf_guard_reason)?
        } else {
            b"enabled"
        },
        tsf_guard_disabled.then_some(b"reset_tsf_guard".as_slice()),
    )?;
    output.extend_from_slice(br#"],"repair":{"mode":"dry_run","result":"not_run","actions":["#);
    let mut first_action = true;
    if !reachable {
        push_diagnostics_action(
            &mut output,
            &mut first_action,
            b"restart_engine",
            b"--restart-engine",
        )?;
    }
    if !config_valid {
        push_diagnostics_action(
            &mut output,
            &mut first_action,
            b"validate_config",
            b"--validate-config",
        )?;
    }
    if tsf_guard_disabled {
        push_diagnostics_action(
            &mut output,
            &mut first_action,
            b"reset_tsf_guard",
            b"--reset-tsf-guard",
        )?;
    }
    output.extend_from_slice(b"]}}");
    Some(output)
}

fn tsf_guard_json(status: &Fcitx5ControlTsfGuard) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"disabled":"#);
    output.extend_from_slice(if status.disabled != 0 {
        b"true"
    } else {
        b"false"
    });
    output.push(b',');
    push_json_string_field(&mut output, b"\"reason\"", utf8_slice(status.reason)?)?;
    output.push(b',');
    push_json_string_field(
        &mut output,
        b"\"marker_path\"",
        utf8_slice(status.marker_path)?,
    )?;
    output.push(b'}');
    Some(output)
}

fn startup_json(enabled: bool) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"enabled":"#);
    output.extend_from_slice(if enabled { b"true" } else { b"false" });
    output.push(b'}');
    output
}

fn addons_json(addons: &[Fcitx5ControlAddonDescriptor]) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(
        br#"{"format_version":1,"surface":"descriptor-inventory","typed_config":"not_available","addons":["#,
    );
    for (index, addon) in addons.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        output.push(b'{');
        push_json_string_field(&mut output, b"\"id\"", utf8_slice(addon.id)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"name\"", utf8_slice(addon.name)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"category\"", utf8_slice(addon.category)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"library\"", utf8_slice(addon.library)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"type\"", utf8_slice(addon.addon_type)?)?;
        output.extend_from_slice(b",\"version\":");
        let version = utf8_slice(addon.version)?;
        if version.is_empty() {
            output.extend_from_slice(b"null");
        } else {
            output.extend_from_slice(&json_string(version)?);
        }
        output.extend_from_slice(b",\"configurable\":");
        output.extend_from_slice(if addon.configurable != 0 {
            b"true"
        } else {
            b"false"
        });
        output.extend_from_slice(b",\"on_demand\":");
        output.extend_from_slice(if addon.on_demand != 0 {
            b"true"
        } else {
            b"false"
        });
        output.extend_from_slice(b",\"library_present\":");
        output.extend_from_slice(if addon.library_present != 0 {
            b"true"
        } else {
            b"false"
        });
        output.push(b'}');
    }
    output.extend_from_slice(b"]}");
    Some(output)
}

fn push_theme_record(output: &mut Vec<u8>, theme: &Fcitx5ControlThemeRecord) -> Option<()> {
    output.push(b'{');
    push_json_string_field(output, b"\"id\"", utf8_slice(theme.id)?)?;
    output.push(b',');
    push_json_string_field(output, b"\"source\"", utf8_slice(theme.source)?)?;
    output.push(b',');
    push_json_string_field(output, b"\"name\"", utf8_slice(theme.name)?)?;
    output.push(b',');
    push_json_string_field(output, b"\"version\"", utf8_slice(theme.version)?)?;
    output.push(b',');
    push_json_string_field(output, b"\"license\"", utf8_slice(theme.license)?)?;
    output.push(b',');
    push_json_string_field(output, b"\"description\"", utf8_slice(theme.description)?)?;
    output.push(b'}');
    Some(())
}

fn themes_json(themes: &[Fcitx5ControlThemeRecord]) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"themes":["#);
    for (index, theme) in themes.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        push_theme_record(&mut output, theme)?;
    }
    output.extend_from_slice(b"]}");
    Some(output)
}

fn theme_editable_fields_json(output: &mut Vec<u8>) {
    const FIELDS: &[&[u8]] = &[
        b"appearance.mode",
        b"candidate.orientation",
        b"candidate.page_size",
        b"candidate.scroll_mode",
        b"candidate.max_width_dip",
        b"candidate.scroll_cell_width_dip",
        b"candidate.opacity",
        b"candidate.preedit_mode",
        b"candidate.geometry.padding_x_dip",
        b"candidate.geometry.padding_y_dip",
        b"candidate.geometry.item_padding_x_dip",
        b"candidate.geometry.item_padding_y_dip",
        b"candidate.geometry.row_gap_dip",
        b"candidate.geometry.column_gap_dip",
        b"candidate.geometry.border_width_dip",
        b"candidate.geometry.corner_radius_dip",
        b"candidate.geometry.shadow",
        b"candidate.label.visible",
        b"candidate.label.style",
        b"candidate.label.font_scale",
        b"candidate.label.gap_dip",
        b"fonts.candidate.families",
        b"fonts.candidate.size_dip",
        b"fonts.candidate.weight",
        b"fonts.annotation.scale",
        b"candidate.colors.background",
        b"candidate.colors.border",
        b"candidate.colors.candidate_text",
        b"candidate.colors.label_text",
        b"candidate.colors.comment_text",
        b"candidate.colors.selected_background",
        b"candidate.colors.selected_candidate_text",
        b"candidate.colors.selected_label_text",
        b"candidate.colors.selected_comment_text",
    ];
    output.push(b'[');
    for (index, field) in FIELDS.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        output.extend_from_slice(&json_string(field).expect("static editable field is JSON-safe"));
    }
    output.push(b']');
}

fn theme_detail_json(detail: &Fcitx5ControlThemeDetail) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"theme":"#);
    push_theme_record(&mut output, &detail.theme)?;
    output.extend_from_slice(b",\"has_light_branch\":");
    output.extend_from_slice(if detail.has_light_branch != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"has_dark_branch\":");
    output.extend_from_slice(if detail.has_dark_branch != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"editable_fields\":");
    theme_editable_fields_json(&mut output);
    output.extend_from_slice(
        br#","security":{"script_allowed":false,"network_allowed":false,"unknown_fields":"reject","path_scope":"theme-directory"}}"#,
    );
    Some(output)
}

fn packages_list_json(list: &Fcitx5ControlPackagesList) -> Option<Vec<u8>> {
    let packages = list.packages;
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"repository_available":"#);
    output.extend_from_slice(if list.repository_available != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"repository_error\":");
    let repository_error = utf8_slice(list.repository_error)?;
    if repository_error.is_empty() {
        output.extend_from_slice(b"null");
    } else {
        output.extend_from_slice(&json_string(repository_error)?);
    }
    output.extend_from_slice(b",\"packages\":[");
    for (index, package) in packages.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        output.push(b'{');
        push_json_string_field(&mut output, b"\"id\"", utf8_slice(package.id)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"title\"", utf8_slice(package.title)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"summary\"", utf8_slice(package.summary)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"type\"", utf8_slice(package.package_type)?)?;
        output.extend_from_slice(b",\"available_version\":");
        let available = utf8_slice(package.available_version)?;
        if available.is_empty() {
            output.extend_from_slice(b"null");
        } else {
            output.extend_from_slice(&json_string(available)?);
        }
        output.extend_from_slice(b",\"installed_version\":");
        let installed = utf8_slice(package.installed_version)?;
        if installed.is_empty() {
            output.extend_from_slice(b"null");
        } else {
            output.extend_from_slice(&json_string(installed)?);
        }
        output.extend_from_slice(b",\"state\":");
        let state = utf8_slice(package.state)?;
        if state.is_empty() {
            output.extend_from_slice(b"null");
        } else {
            output.extend_from_slice(&json_string(state)?);
        }
        output.extend_from_slice(b",\"update_available\":");
        output.extend_from_slice(if package.update_available != 0 {
            b"true"
        } else {
            b"false"
        });
        output.push(b'}');
    }
    output.extend_from_slice(b"]}");
    Some(output)
}

fn push_json_nullable_string(
    output: &mut Vec<u8>,
    name: &[u8],
    value: Fcitx5ControlUtf8<'_>,
) -> Option<()> {
    output.extend_from_slice(name);
    output.push(b':');
    let value = utf8_slice(value)?;
    if value.is_empty() {
        output.extend_from_slice(b"null");
    } else {
        output.extend_from_slice(&json_string(value)?);
    }
    Some(())
}

fn push_json_raw_field(output: &mut Vec<u8>, name: &[u8], value: Fcitx5ControlUtf8) -> Option<()> {
    output.extend_from_slice(name);
    output.push(b':');
    let value = utf8_slice(value)?;
    if value.is_empty() {
        return None;
    }
    output.extend_from_slice(value);
    Some(())
}

fn string_array_json(values: &[Fcitx5ControlUtf8]) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.push(b'[');
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        output.extend_from_slice(&json_string(utf8_slice(*value)?)?);
    }
    output.push(b']');
    Some(output)
}

fn package_dependencies_json(dependencies: &[Fcitx5ControlPackageDependency]) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.push(b'[');
    for (index, dependency) in dependencies.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        output.extend_from_slice(b"{");
        push_json_string_field(&mut output, b"\"id\"", utf8_slice(dependency.id)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"version\"", utf8_slice(dependency.version)?)?;
        output.push(b'}');
    }
    output.push(b']');
    Some(output)
}

fn config_surfaces_json(owner: Fcitx5ControlUtf8, kinds: &[Fcitx5ControlUtf8]) -> Option<Vec<u8>> {
    let owner = utf8_slice(owner)?;
    let mut output = Vec::new();
    output.push(b'[');
    for (index, kind) in kinds.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        output.extend_from_slice(b"{");
        push_json_string_field(&mut output, b"\"kind\"", utf8_slice(*kind)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"owner\"", owner)?;
        output.extend_from_slice(br#","schema":"generic-fcitx-config-v1"}"#);
    }
    output.push(b']');
    Some(output)
}

const THEME_FILE_NAME: &str = "theme.toml";

fn user_themes_dir(data_root: &std::path::Path) -> Option<PathBuf> {
    if data_root.as_os_str().is_empty() {
        return None;
    }
    Some(data_root.join("themes"))
}

fn theme_file_path(theme_dir: &std::path::Path) -> Option<PathBuf> {
    if theme_dir.as_os_str().is_empty() {
        return None;
    }
    Some(theme_dir.join(THEME_FILE_NAME))
}

fn resolve_theme_path(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
    requested_id: &str,
    builtin: bool,
) -> Option<PathBuf> {
    if builtin {
        return fcitx5_config_core::builtin_theme_file(install_root, requested_id);
    }
    if !theme_id_valid(requested_id) {
        return None;
    }
    Some(
        user_themes_dir(data_root)?
            .join(requested_id)
            .join(THEME_FILE_NAME),
    )
}

fn user_theme_file(data_root: &std::path::Path, id: &str) -> Option<PathBuf> {
    if !theme_id_valid(id) {
        return None;
    }
    Some(user_themes_dir(data_root)?.join(id).join(THEME_FILE_NAME))
}

fn theme_operation_ok_json(id: &str, operation: &str) -> Vec<u8> {
    format!(
        "{{\"format_version\":1,\"operation\":\"{}\",\"theme\":\"{}\",\"result\":\"ok\"}}",
        operation, id
    )
    .into_bytes()
}

fn validate_theme_import_text(text: &[u8]) -> Result<ThemeSummary, i32> {
    let summary = parse_theme_summary(text).ok_or(CONTROL_THEME_OPERATION_UNSAFE_IMPORT)?;
    let text = std::str::from_utf8(text).map_err(|_| CONTROL_THEME_OPERATION_UNSAFE_IMPORT)?;
    let lower = text.to_ascii_lowercase();
    if lower.contains("..\\")
        || lower.contains("../")
        || lower.contains("http://")
        || lower.contains("https://")
        || lower.contains("\\\\")
        || lower.contains(".ps1")
        || lower.contains(".exe")
    {
        return Err(CONTROL_THEME_OPERATION_UNSAFE_IMPORT);
    }
    Ok(summary)
}

fn rewrite_theme_id(text: &[u8], new_id: &str) -> Option<Vec<u8>> {
    if !theme_id_valid(new_id) {
        return None;
    }
    let mut document = parse_theme_document(text)?;
    let metadata = document["theme"].as_table_mut()?;
    metadata["id"] = toml_edit::value(new_id);
    Some(document.to_string().into_bytes())
}

fn export_theme(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
    id: &str,
) -> Result<Vec<u8>, i32> {
    let builtin = fcitx5_config_core::builtin_theme_name(id).is_some();
    if !builtin && !theme_id_valid(id) {
        return Err(CONTROL_THEME_OPERATION_INVALID);
    }
    let path = resolve_theme_path(install_root, data_root, id, builtin)
        .ok_or(CONTROL_THEME_OPERATION_INVALID)?;
    read_file_bounded(path, 512 * 1024).map_err(|status| {
        if status == CONTROL_FILE_READ_INVALID_FILE {
            CONTROL_THEME_OPERATION_NOT_FOUND
        } else {
            CONTROL_THEME_OPERATION_IO_ERROR
        }
    })
}

fn import_theme(
    data_root: &std::path::Path,
    source_path: &std::path::Path,
) -> Result<Vec<u8>, i32> {
    let text = read_file_bounded(source_path.to_path_buf(), 512 * 1024)
        .map_err(|_| CONTROL_THEME_OPERATION_IO_ERROR)?;
    let summary = validate_theme_import_text(&text)?;
    let destination =
        user_theme_file(data_root, &summary.id).ok_or(CONTROL_THEME_OPERATION_INVALID)?;
    if destination.exists() {
        return Err(CONTROL_THEME_OPERATION_ALREADY_EXISTS);
    }
    atomic_write_utf8_file(destination, &text).map_err(|_| CONTROL_THEME_OPERATION_IO_ERROR)?;
    Ok(theme_operation_ok_json(&summary.id, "import"))
}

fn export_theme_to_file(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
    id: &str,
    destination_path: &std::path::Path,
) -> Result<Vec<u8>, i32> {
    let text = export_theme(install_root, data_root, id)?;
    atomic_write_utf8_file(destination_path.to_path_buf(), &text)
        .map_err(|_| CONTROL_THEME_OPERATION_IO_ERROR)?;
    Ok(theme_operation_ok_json(id, "export"))
}

fn duplicate_theme(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
    source_id: &str,
    new_id: &str,
) -> Result<Vec<u8>, i32> {
    if !theme_id_valid(new_id) {
        return Err(CONTROL_THEME_OPERATION_INVALID);
    }
    let destination = user_theme_file(data_root, new_id).ok_or(CONTROL_THEME_OPERATION_INVALID)?;
    if destination.exists() {
        return Err(CONTROL_THEME_OPERATION_ALREADY_EXISTS);
    }
    let source = export_theme(install_root, data_root, source_id)?;
    validate_theme_import_text(&source)?;
    let rewritten =
        rewrite_theme_id(&source, new_id).ok_or(CONTROL_THEME_OPERATION_UNSAFE_IMPORT)?;
    atomic_write_utf8_file(destination, &rewritten)
        .map_err(|_| CONTROL_THEME_OPERATION_IO_ERROR)?;
    Ok(theme_operation_ok_json(new_id, "duplicate"))
}

fn delete_theme(data_root: &std::path::Path, id: &str) -> Result<Vec<u8>, i32> {
    let theme_dir = theme_id_valid(id)
        .then(|| user_themes_dir(data_root).map(|dir| dir.join(id)))
        .flatten()
        .ok_or(CONTROL_THEME_OPERATION_READ_ONLY)?;
    if !theme_dir.exists() {
        return Err(CONTROL_THEME_OPERATION_NOT_FOUND);
    }
    let metadata =
        std::fs::symlink_metadata(&theme_dir).map_err(|_| CONTROL_THEME_OPERATION_IO_ERROR)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(CONTROL_THEME_OPERATION_INVALID);
    }
    std::fs::remove_dir_all(&theme_dir).map_err(|_| CONTROL_THEME_OPERATION_IO_ERROR)?;
    Ok(theme_operation_ok_json(id, "delete"))
}

fn is_lower_theme_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
}

struct ThemeDiscoveryEntryOwned {
    path: PathBuf,
    id: String,
    source: &'static str,
}

fn discover_themes(
    install_root: &std::path::Path,
    data_root: &std::path::Path,
) -> Vec<ThemeDiscoveryEntryOwned> {
    let mut entries = Vec::new();
    for name in fcitx5_config_core::BUILTIN_THEME_NAMES {
        let id = format!("builtin:{name}");
        if let Some(path) = fcitx5_config_core::builtin_theme_file(install_root, &id) {
            entries.push(ThemeDiscoveryEntryOwned {
                path,
                id,
                source: "builtin",
            });
        }
    }
    let Some(user_themes) = user_themes_dir(data_root) else {
        return entries;
    };
    let Ok(directory) = std::fs::read_dir(user_themes) else {
        return entries;
    };
    for entry in directory.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        if !is_lower_theme_id(&id) {
            continue;
        }
        let Some(path) = theme_file_path(&entry.path()) else {
            continue;
        };
        entries.push(ThemeDiscoveryEntryOwned {
            path,
            id,
            source: "user",
        });
    }
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    entries
}

struct ThemeSummary {
    id: String,
    name: String,
    version: String,
    license: String,
    description: String,
    has_light_branch: bool,
    has_dark_branch: bool,
}

fn table_has_only(table: &toml_edit::Table, allowed: &[&str]) -> bool {
    table.iter().all(|(key, _)| allowed.contains(&key))
}

fn theme_id_valid(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
}

fn theme_palette_key_valid(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
}

fn theme_color_hex_valid(value: &str) -> bool {
    let bytes = value.as_bytes();
    matches!(bytes.len(), 7 | 9) && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit)
}

fn theme_palette_reference(value: &str) -> Option<&str> {
    let value = value.trim();
    let name = value.strip_prefix("${")?.strip_suffix('}')?;
    theme_palette_key_valid(name).then_some(name)
}

fn required_theme_string(table: &toml_edit::Table, key: &str) -> Option<String> {
    let value = table.get(key)?.as_str()?;
    (!value.is_empty()).then(|| value.to_owned())
}

fn theme_common_table(root: &toml_edit::Table) -> Option<Option<&toml_edit::Table>> {
    let Some(common) = root.get("common") else {
        return Some(None);
    };
    let common = common.as_table()?;
    table_has_only(common, &["candidate", "fonts"]).then_some(Some(common))
}

fn theme_branch_table<'a>(
    root: &'a toml_edit::Table,
    key: &str,
) -> Option<Option<&'a toml_edit::Table>> {
    let Some(branch) = root.get(key) else {
        return Some(None);
    };
    let branch = branch.as_table()?;
    if !table_has_only(branch, &["candidate", "palette"]) {
        return None;
    }
    if let Some(palette) = branch.get("palette") {
        validate_theme_palette_table(palette.as_table()?)?;
    }
    let Some(candidate) = branch.get("candidate") else {
        return Some(Some(branch));
    };
    let candidate = candidate.as_table()?;
    if !table_has_only(candidate, &["colors"]) {
        return None;
    }
    Some(Some(branch))
}

fn theme_branch_has_colors(root: &toml_edit::Table, key: &str) -> Option<bool> {
    let Some(branch) = theme_branch_table(root, key)? else {
        return Some(false);
    };
    let Some(candidate) = branch.get("candidate") else {
        return Some(false);
    };
    let candidate = candidate.as_table()?;
    let Some(colors) = candidate.get("colors") else {
        return Some(false);
    };
    let colors = colors.as_table()?;
    Some(!colors.is_empty())
}

fn validate_theme_palette_table(table: &toml_edit::Table) -> Option<()> {
    for (key, value) in table.iter() {
        if !theme_palette_key_valid(key) {
            return None;
        }
        let value = value.as_str()?.trim();
        if value.is_empty()
            || value.len() > 96
            || (!theme_color_hex_valid(value) && theme_palette_reference(value).is_none())
        {
            return None;
        }
    }
    Some(())
}

fn parse_theme_document(text: &[u8]) -> Option<toml_edit::DocumentMut> {
    if text.len() > 512 * 1024 {
        return None;
    }
    let text = std::str::from_utf8(text).ok()?;
    text.parse::<toml_edit::DocumentMut>().ok()
}

fn theme_summary_from_root(root: &toml_edit::Table) -> Option<ThemeSummary> {
    if !table_has_only(
        root,
        &[
            "format_version",
            "theme",
            "palette",
            "common",
            "light",
            "dark",
        ],
    ) {
        return None;
    }
    if root.get("format_version")?.as_integer()? != 1 {
        return None;
    }
    if let Some(palette) = root.get("palette") {
        validate_theme_palette_table(palette.as_table()?)?;
    }
    theme_common_table(root)?;
    let metadata = root.get("theme")?.as_table()?;
    if !table_has_only(
        metadata,
        &["id", "name", "version", "license", "description"],
    ) {
        return None;
    }
    let id = required_theme_string(metadata, "id")?;
    if !theme_id_valid(&id) {
        return None;
    }
    Some(ThemeSummary {
        id,
        name: required_theme_string(metadata, "name")?,
        version: required_theme_string(metadata, "version")?,
        license: required_theme_string(metadata, "license")?,
        description: metadata
            .get("description")
            .and_then(toml_edit::Item::as_str)
            .unwrap_or_default()
            .to_owned(),
        has_light_branch: theme_branch_has_colors(root, "light")?,
        has_dark_branch: theme_branch_has_colors(root, "dark")?,
    })
}

fn parse_theme_summary(text: &[u8]) -> Option<ThemeSummary> {
    let value = parse_theme_document(text)?;
    theme_summary_from_root(value.as_table())
}

fn addon_metadata_bool(value: &[u8]) -> bool {
    matches!(value, b"True" | b"true" | b"1")
}

fn repository_metadata_url(base_url: &[u16], metadata_name: &[u8]) -> Option<Vec<u16>> {
    if metadata_name.is_empty()
        || metadata_name
            .iter()
            .any(|byte| !byte.is_ascii() || *byte == b'/' || *byte == b'\\' || *byte == 0)
    {
        return None;
    }
    let mut trimmed = base_url;
    while trimmed.last().copied() == Some(b'/' as u16) {
        trimmed = &trimmed[..trimmed.len() - 1];
    }
    let mut result = Vec::with_capacity(trimmed.len() + 1 + metadata_name.len());
    result.extend_from_slice(trimmed);
    result.push(b'/' as u16);
    result.extend(metadata_name.iter().map(|byte| *byte as u16));
    Some(result)
}

fn repository_default_base_url(channel: &[u8]) -> Option<Vec<u16>> {
    if channel
        .iter()
        .any(|byte| !byte.is_ascii() || *byte == b'/' || *byte == b'\\' || *byte == 0)
    {
        return None;
    }
    let mut result =
        b"https://github.com/0x696c757a696f/fcitx5-windows-next/releases/download/v".to_vec();
    result.extend_from_slice(
        option_env!("FCITX_WINDOWS_VERSION")
            .unwrap_or(env!("CARGO_PKG_VERSION"))
            .as_bytes(),
    );
    Some(result.into_iter().map(u16::from).collect())
}

struct BundledPackage {
    id: &'static str,
    title: &'static str,
    probe_relative_path: &'static str,
}

const BUNDLED_PACKAGES: &[BundledPackage] = &[
    BundledPackage {
        id: "fcitx5-chinese-addons",
        title: "Fcitx5 Chinese Addons",
        probe_relative_path: "lib/fcitx5/libpinyin.dll",
    },
    BundledPackage {
        id: "fcitx5-rime",
        title: "Rime",
        probe_relative_path: "lib/fcitx5/librime.dll",
    },
    BundledPackage {
        id: "fcitx5-lua",
        title: "Fcitx5 Lua",
        probe_relative_path: "lib/fcitx5/libluaaddonloader.dll",
    },
    BundledPackage {
        id: "fcitx5-chttrans",
        title: "Simplified / Traditional Conversion",
        probe_relative_path: "lib/fcitx5/libchttrans.dll",
    },
    BundledPackage {
        id: "librime-lua",
        title: "Rime Lua",
        probe_relative_path: "bin/lua54.dll",
    },
];

fn package_detail_json(detail: &Fcitx5ControlPackageDetail) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"repository_available":"#);
    output.extend_from_slice(if detail.repository_available != 0 {
        b"true"
    } else {
        b"false"
    });
    output.push(b',');
    push_json_nullable_string(
        &mut output,
        b"\"repository_error\"",
        detail.repository_error,
    )?;
    output.push(b',');
    push_json_string_field(&mut output, b"\"id\"", utf8_slice(detail.id)?)?;
    output.push(b',');
    push_json_string_field(&mut output, b"\"title\"", utf8_slice(detail.title)?)?;
    output.push(b',');
    push_json_string_field(&mut output, b"\"summary\"", utf8_slice(detail.summary)?)?;
    output.push(b',');
    push_json_string_field(&mut output, b"\"type\"", utf8_slice(detail.package_type)?)?;
    output.push(b',');
    push_json_nullable_string(
        &mut output,
        b"\"available_version\"",
        detail.available_version,
    )?;
    output.push(b',');
    push_json_nullable_string(
        &mut output,
        b"\"installed_version\"",
        detail.installed_version,
    )?;
    output.push(b',');
    push_json_nullable_string(&mut output, b"\"state\"", detail.state)?;
    output.extend_from_slice(b",\"bundled\":");
    output.extend_from_slice(if detail.bundled != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"update_available\":");
    output.extend_from_slice(if detail.update_available != 0 {
        b"true"
    } else {
        b"false"
    });
    output.push(b',');
    push_json_nullable_string(&mut output, b"\"manifest_sha256\"", detail.manifest_sha256)?;
    output.push(b',');
    push_json_nullable_string(&mut output, b"\"source_commit\"", detail.source_commit)?;
    output.push(b',');
    push_json_raw_field(&mut output, b"\"dependencies\"", detail.dependencies_json)?;
    output.push(b',');
    push_json_raw_field(&mut output, b"\"permissions\"", detail.permissions_json)?;
    output.push(b',');
    push_json_raw_field(
        &mut output,
        b"\"config_surface\"",
        detail.config_surface_json,
    )?;
    output.push(b'}');
    Some(output)
}

fn startup_command(executable_directory: OsString) -> Vec<u16> {
    let launcher = PathBuf::from(executable_directory).join("fcitx5-launcher.exe");
    let mut command = quote(launcher.as_os_str());
    command.push(" --background");
    wide_z(&command)
}

fn query_startup(executable_directory: OsString, registry_value: OsString) -> Result<bool, ()> {
    let expected = startup_command(executable_directory);
    let value_name = wide_z(&registry_value);
    let mut raw_key = null_mut();
    // SAFETY: static key path is NUL-terminated and `raw_key` is writable output storage.
    let open_result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut raw_key,
        )
    };
    if open_result != ERROR_SUCCESS {
        return Ok(false);
    }
    let key = RegistryKey(raw_key);
    let mut value_type = 0_u32;
    let mut bytes = 0_u32;
    // SAFETY: `key` owns a live HKEY and the type/size outputs are writable.
    let size_result = unsafe {
        RegQueryValueExW(
            key.get(),
            value_name.as_ptr(),
            null_mut(),
            &mut value_type,
            null_mut(),
            &mut bytes,
        )
    };
    if size_result == ERROR_FILE_NOT_FOUND {
        return Ok(false);
    }
    if size_result != ERROR_SUCCESS || value_type != REG_SZ || !(2..=64 * 1024).contains(&bytes) {
        return Err(());
    }
    let mut value = vec![0_u16; (bytes as usize).div_ceil(2)];
    // SAFETY: `value` is writable for the queried byte count and all output pointers are valid.
    let read_result = unsafe {
        RegQueryValueExW(
            key.get(),
            value_name.as_ptr(),
            null_mut(),
            &mut value_type,
            value.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    while value.last().copied() == Some(0) {
        value.pop();
    }
    let mut expected_trimmed = expected;
    while expected_trimmed.last().copied() == Some(0) {
        expected_trimmed.pop();
    }
    if read_result != ERROR_SUCCESS {
        return Err(());
    }
    Ok(value == expected_trimmed)
}

fn set_startup(
    executable_directory: OsString,
    registry_value: OsString,
    enabled: bool,
) -> Result<(), ()> {
    let value_name = wide_z(&registry_value);
    let mut raw_key = null_mut();
    // SAFETY: static key path is NUL-terminated and all C-layout output pointers are writable.
    let create_result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY.as_ptr(),
            0,
            null_mut(),
            0,
            KEY_SET_VALUE,
            null_mut(),
            &mut raw_key,
            null_mut(),
        )
    };
    if create_result != ERROR_SUCCESS {
        return Err(());
    }
    let key = RegistryKey(raw_key);
    let result = if enabled {
        let command = startup_command(executable_directory);
        // SAFETY: `key` owns a live HKEY; both UTF-16 buffers are NUL-terminated and live for the call.
        unsafe {
            RegSetValueExW(
                key.get(),
                value_name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as Dword,
            )
        }
    } else {
        // SAFETY: `key` owns a live HKEY and `value_name` is a live NUL-terminated UTF-16 string.
        let delete_result = unsafe { RegDeleteValueW(key.get(), value_name.as_ptr()) };
        if delete_result == ERROR_FILE_NOT_FOUND {
            ERROR_SUCCESS
        } else {
            delete_result
        }
    };
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(())
    }
}

/// # Safety
///
/// UTF-16 slices must remain valid for the duration of the call. `out_enabled`
/// must point to writable storage. No pointer is retained.
#[cfg(test)]
mod tests {
    use super::*;

    fn wide(value: &str) -> Vec<u16> {
        OsString::from(value).encode_wide().collect()
    }

    #[test]
    fn startup_command_quotes_launcher_path() {
        let command = startup_command(OsString::from(r"C:\Program Files\Fcitx5\bin"));
        let mut trimmed = command;
        while trimmed.last().copied() == Some(0) {
            trimmed.pop();
        }
        assert_eq!(
            trimmed,
            wide(r#""C:\Program Files\Fcitx5\bin\fcitx5-launcher.exe" --background"#)
        );
    }

    #[test]
    fn startup_json_preserves_control_contract() {
        assert_eq!(
            startup_json(true).as_slice(),
            br#"{"format_version":1,"enabled":true}"#
        );
        assert_eq!(
            startup_json(false).as_slice(),
            br#"{"format_version":1,"enabled":false}"#
        );
    }

    #[test]
    fn system_locale_mapping_covers_phase_a_languages_and_falls_back_to_english() {
        let cases = [
            (0x0804, CONFIG_LOCALE_ZH_CN_FILE),
            (0x1004, CONFIG_LOCALE_ZH_CN_FILE),
            (0x0404, CONFIG_LOCALE_ZH_TW_FILE),
            (0x0c04, CONFIG_LOCALE_ZH_TW_FILE),
            (0x1404, CONFIG_LOCALE_ZH_TW_FILE),
            (0x0411, CONFIG_LOCALE_JA_JP_FILE),
            (0x0412, CONFIG_LOCALE_KO_KR_FILE),
            (0x042a, CONFIG_LOCALE_VI_VN_FILE),
            (0x041e, CONFIG_LOCALE_TH_TH_FILE),
            (0x045b, CONFIG_LOCALE_SI_LK_FILE),
        ];
        for (language, expected_file) in cases {
            assert_eq!(
                config_locale_file_for_system_language(language),
                expected_file,
                "Windows LANGID {language:#06x} should select its canonical locale file"
            );
        }
        assert_eq!(
            config_locale_file_for_system_language(0x040c),
            CONFIG_LOCALE_EN_US_FILE
        );
        assert_eq!(
            config_locale_file_for_system_language(LANG_CHINESE),
            CONFIG_LOCALE_EN_US_FILE
        );
    }

    #[test]
    fn phase_a_locale_resources_are_utf8_complete_and_nonempty() {
        fn entries(bytes: &[u8]) -> std::collections::BTreeMap<&str, &str> {
            let document = std::str::from_utf8(bytes).expect("locale resource must be UTF-8");
            let mut lines = document.lines();
            assert_eq!(lines.next().map(str::trim), Some("{"));
            let mut result = std::collections::BTreeMap::new();
            let mut closed = false;
            for line in lines {
                let line = line.trim();
                if line == "}" {
                    closed = true;
                    break;
                }
                let line = line.strip_prefix(',').unwrap_or(line);
                let line = line.strip_suffix(',').unwrap_or(line);
                let (key, value) = line.split_once(':').expect("flat locale entry");
                let key = key
                    .trim()
                    .strip_prefix('"')
                    .and_then(|key| key.strip_suffix('"'))
                    .expect("quoted locale key");
                let value = value.trim();
                assert!(!value.is_empty(), "{key} must have a value");
                if key == "format_version" {
                    assert_eq!(value, "1");
                } else {
                    assert!(value.starts_with('"') && value.ends_with('"'));
                    assert!(
                        !value[1..value.len() - 1].trim().is_empty(),
                        "{key} is blank"
                    );
                }
                assert!(result.insert(key, value).is_none(), "duplicate key: {key}");
            }
            assert!(closed, "locale resource must close its JSON object");
            result
        }

        let canonical = entries(include_bytes!("../../../locales/en-US.json"));
        for (locale, bytes) in [
            (
                "zh-CN",
                include_bytes!("../../../locales/zh-CN.json") as &[u8],
            ),
            (
                "zh-TW",
                include_bytes!("../../../locales/zh-TW.json") as &[u8],
            ),
            (
                "ja-JP",
                include_bytes!("../../../locales/ja-JP.json") as &[u8],
            ),
            (
                "ko-KR",
                include_bytes!("../../../locales/ko-KR.json") as &[u8],
            ),
            (
                "vi-VN",
                include_bytes!("../../../locales/vi-VN.json") as &[u8],
            ),
            (
                "th-TH",
                include_bytes!("../../../locales/th-TH.json") as &[u8],
            ),
            (
                "si-LK",
                include_bytes!("../../../locales/si-LK.json") as &[u8],
            ),
        ] {
            let translated = entries(bytes);
            assert_eq!(
                translated.keys().collect::<Vec<_>>(),
                canonical.keys().collect::<Vec<_>>(),
                "{locale} keys must match en-US"
            );
        }
    }

    #[test]
    fn addons_json_preserves_control_contract() {
        let id = b"pinyin";
        let name = "拼音".as_bytes();
        let category = b"InputMethod";
        let library = b"pinyin";
        let addon_type = b"SharedLibrary";
        let version = b"5.1";
        let empty_version = b"";
        let addons = [
            Fcitx5ControlAddonDescriptor {
                id: Fcitx5ControlUtf8::new(id),
                name: Fcitx5ControlUtf8::new(name),
                category: Fcitx5ControlUtf8::new(category),
                library: Fcitx5ControlUtf8::new(library),
                addon_type: Fcitx5ControlUtf8::new(addon_type),
                version: Fcitx5ControlUtf8::new(version),
                configurable: 1,
                on_demand: 0,
                library_present: 1,
            },
            Fcitx5ControlAddonDescriptor {
                id: Fcitx5ControlUtf8::new(b"test"),
                name: Fcitx5ControlUtf8::new(b"Test"),
                category: Fcitx5ControlUtf8::new(b"Module"),
                library: Fcitx5ControlUtf8::new(b""),
                addon_type: Fcitx5ControlUtf8::new(b"Static"),
                version: Fcitx5ControlUtf8::new(empty_version),
                configurable: 0,
                on_demand: 1,
                library_present: 0,
            },
        ];
        let json = addons_json(&addons).expect("addons should format");
        let text = String::from_utf8(json).expect("addons JSON should be UTF-8");
        assert!(text.starts_with(
            r#"{"format_version":1,"surface":"descriptor-inventory","typed_config":"not_available","addons":["#
        ));
        assert!(text.contains(r#""name":"拼音""#));
        assert!(text.contains(r#""version":"5.1""#));
        assert!(text.contains(r#""version":null"#));
        assert!(text.contains(r#""on_demand":true"#));
    }

    #[test]
    fn empty_addons_json_preserves_control_contract() {
        assert_eq!(
            addons_json(&[]).as_deref(),
            Some(
                &br#"{"format_version":1,"surface":"descriptor-inventory","typed_config":"not_available","addons":[]}"#[..]
            )
        );
    }

    fn test_theme_record() -> Fcitx5ControlThemeRecord<'static> {
        Fcitx5ControlThemeRecord {
            id: Fcitx5ControlUtf8::new(b"builtin:default"),
            source: Fcitx5ControlUtf8::new(b"builtin"),
            name: Fcitx5ControlUtf8::new("默认".as_bytes()),
            version: Fcitx5ControlUtf8::new(b"1"),
            license: Fcitx5ControlUtf8::new(b"MIT"),
            description: Fcitx5ControlUtf8::new(b"default theme"),
        }
    }

    #[test]
    fn themes_json_preserves_control_contract() {
        let theme = test_theme_record();
        let json = themes_json(&[theme]).expect("themes should format");
        let text = String::from_utf8(json).expect("themes JSON should be UTF-8");
        assert_eq!(
            text,
            r#"{"format_version":1,"themes":[{"id":"builtin:default","source":"builtin","name":"默认","version":"1","license":"MIT","description":"default theme"}]}"#
        );
        assert_eq!(
            themes_json(&[]).as_deref(),
            Some(&br#"{"format_version":1,"themes":[]}"#[..])
        );
    }

    #[test]
    fn theme_detail_json_preserves_control_contract() {
        let detail = Fcitx5ControlThemeDetail {
            theme: test_theme_record(),
            has_light_branch: 1,
            has_dark_branch: 0,
        };
        let json = theme_detail_json(&detail).expect("theme detail should format");
        let text = String::from_utf8(json).expect("theme detail JSON should be UTF-8");
        assert!(text.starts_with(
            r#"{"format_version":1,"theme":{"id":"builtin:default","source":"builtin""#
        ));
        assert!(text.contains(r#""has_light_branch":true"#));
        assert!(text.contains(r#""has_dark_branch":false"#));
        assert!(text.contains(r#""appearance.mode""#));
        assert!(text.contains(
            r#""security":{"script_allowed":false,"network_allowed":false,"unknown_fields":"reject","path_scope":"theme-directory"}"#
        ));
    }

    #[test]
    fn packages_list_json_preserves_control_contract() {
        let id = b"fcitx5-rime";
        let title = b"Rime";
        let summary = b"Rime input method";
        let package_type = b"addon";
        let available = b"1.2.3";
        let installed = b"1.0.0";
        let state = b"enabled";
        let package = Fcitx5ControlPackageSummary {
            id: Fcitx5ControlUtf8::new(id),
            title: Fcitx5ControlUtf8::new(title),
            summary: Fcitx5ControlUtf8::new(summary),
            package_type: Fcitx5ControlUtf8::new(package_type),
            available_version: Fcitx5ControlUtf8::new(available),
            installed_version: Fcitx5ControlUtf8::new(installed),
            state: Fcitx5ControlUtf8::new(state),
            update_available: 1,
        };
        let list = Fcitx5ControlPackagesList {
            repository_available: 1,
            repository_error: Fcitx5ControlUtf8::new(b""),
            packages: std::slice::from_ref(&package),
        };
        let json = packages_list_json(&list).expect("packages list should format");
        let text = String::from_utf8(json).expect("packages list JSON should be UTF-8");
        assert_eq!(
            text,
            r#"{"format_version":1,"repository_available":true,"repository_error":null,"packages":[{"id":"fcitx5-rime","title":"Rime","summary":"Rime input method","type":"addon","available_version":"1.2.3","installed_version":"1.0.0","state":"enabled","update_available":true}]}"#
        );
    }

    #[test]
    fn packages_list_json_preserves_null_fields() {
        let package = Fcitx5ControlPackageSummary {
            id: Fcitx5ControlUtf8::new(b"orphan"),
            title: Fcitx5ControlUtf8::new(b"orphan"),
            summary: Fcitx5ControlUtf8::new(b""),
            package_type: Fcitx5ControlUtf8::new(b"unknown"),
            available_version: Fcitx5ControlUtf8::new(b""),
            installed_version: Fcitx5ControlUtf8::new(b"1"),
            state: Fcitx5ControlUtf8::new(b""),
            update_available: 0,
        };
        let error = b"missing_key";
        let list = Fcitx5ControlPackagesList {
            repository_available: 0,
            repository_error: Fcitx5ControlUtf8::new(error),
            packages: std::slice::from_ref(&package),
        };
        let json = packages_list_json(&list).expect("packages list should format");
        let text = String::from_utf8(json).expect("packages list JSON should be UTF-8");
        assert!(text.contains(r#""repository_available":false"#));
        assert!(text.contains(r#""repository_error":"missing_key""#));
        assert!(text.contains(r#""available_version":null"#));
        assert!(text.contains(r#""state":null"#));
    }

    #[test]
    #[allow(clippy::manual_c_str_literals)]
    fn package_dependencies_json_preserves_control_contract() {
        let dependencies = [
            Fcitx5ControlPackageDependency {
                id: Fcitx5ControlUtf8::new(b"dep-one"),
                version: Fcitx5ControlUtf8::new(b"1.0"),
            },
            Fcitx5ControlPackageDependency {
                id: Fcitx5ControlUtf8::new(b"dep\"two"),
                version: Fcitx5ControlUtf8::new(b"2\\0"),
            },
        ];
        let json = package_dependencies_json(&dependencies).expect("dependencies should format");
        let text = String::from_utf8(json).expect("dependency JSON should be UTF-8");
        assert_eq!(
            text,
            r#"[{"id":"dep-one","version":"1.0"},{"id":"dep\"two","version":"2\\0"}]"#
        );
    }

    #[test]
    fn string_array_json_preserves_control_contract() {
        let values = [
            Fcitx5ControlUtf8::new(b"input-data"),
            Fcitx5ControlUtf8::new(b"quotes\"ok"),
        ];
        let json = string_array_json(&values).expect("string array should format");
        let text = String::from_utf8(json).expect("string array JSON should be UTF-8");
        assert_eq!(text, r#"["input-data","quotes\"ok"]"#);
    }

    #[test]
    fn config_surfaces_json_preserves_control_contract() {
        let owner = Fcitx5ControlUtf8::new(b"fcitx5-rime");
        let kinds = [
            Fcitx5ControlUtf8::new(b"fcitx-addon"),
            Fcitx5ControlUtf8::new(b"rime-data"),
        ];
        let json = config_surfaces_json(owner, &kinds).expect("surfaces should format");
        let text = String::from_utf8(json).expect("surfaces JSON should be UTF-8");
        assert_eq!(
            text,
            r#"[{"kind":"fcitx-addon","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"},{"kind":"rime-data","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"}]"#
        );
    }

    #[test]
    fn theme_file_operations_are_rust_owned_and_scoped_to_user_theme_dir() {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-control-core-theme-ops-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let install = root.join("install");
        let data = root.join("data");
        let builtin_dir = install.join("themes").join("default");
        std::fs::create_dir_all(&builtin_dir).expect("create builtin theme");
        let builtin_text = br##"
format_version = 1

[theme]
id = "builtin.default"
name = "Default"
version = "1"
license = "MIT"

[light.candidate.colors]
background = "#ffffffff"
"##;
        std::fs::write(builtin_dir.join(THEME_FILE_NAME), builtin_text)
            .expect("write builtin theme");

        let imported = root.join("imported.toml");
        let imported_text = br##"
format_version = 1

[theme]
id = "soft-blue"
name = "Soft Blue"
version = "1"
license = "MIT"

[dark.candidate.colors]
background = "#101820ff"
"##;
        std::fs::write(&imported, imported_text).expect("write imported theme");
        let import_json = import_theme(&data, &imported).expect("import user theme");
        assert!(String::from_utf8(import_json)
            .expect("json")
            .contains(r#""operation":"import""#));
        assert!(data
            .join("themes")
            .join("soft-blue")
            .join(THEME_FILE_NAME)
            .exists());

        let exported = export_theme(&install, &data, "soft-blue").expect("export user theme");
        assert!(std::str::from_utf8(&exported)
            .expect("utf-8")
            .contains("Soft Blue"));
        let exported_file = root.join("exported-soft-blue.toml");
        let export_json = export_theme_to_file(&install, &data, "soft-blue", &exported_file)
            .expect("export user theme to file");
        assert!(String::from_utf8(export_json)
            .expect("json")
            .contains(r#""operation":"export""#));
        assert!(std::fs::read_to_string(&exported_file)
            .expect("read exported theme")
            .contains("Soft Blue"));

        duplicate_theme(&install, &data, "builtin:default", "default-copy")
            .expect("duplicate builtin theme into user scope");
        let copied = std::fs::read_to_string(
            data.join("themes")
                .join("default-copy")
                .join(THEME_FILE_NAME),
        )
        .expect("read duplicate");
        assert!(copied.contains("id = \"default-copy\""));
        assert_eq!(
            duplicate_theme(&install, &data, "builtin:default", "default-copy"),
            Err(CONTROL_THEME_OPERATION_ALREADY_EXISTS)
        );

        assert_eq!(
            delete_theme(&data, "builtin:default"),
            Err(CONTROL_THEME_OPERATION_READ_ONLY)
        );
        delete_theme(&data, "soft-blue").expect("delete user theme");
        assert!(!data.join("themes").join("soft-blue").exists());

        let unsafe_import = root.join("unsafe.toml");
        std::fs::write(
            &unsafe_import,
            br##"
format_version = 1
[theme]
id = "unsafe-theme"
name = "Unsafe"
version = "1"
license = "MIT"
script = "run-me.ps1"
"##,
        )
        .expect("write unsafe theme");
        assert_eq!(
            import_theme(&data, &unsafe_import),
            Err(CONTROL_THEME_OPERATION_UNSAFE_IMPORT)
        );

        let description_import = root.join("description.toml");
        std::fs::write(
            &description_import,
            br##"
format_version = 1
[theme]
id = "description-ok"
name = "Description OK"
version = "1"
license = "MIT"
description = "description is ordinary theme metadata"
"##,
        )
        .expect("write description theme");
        import_theme(&data, &description_import)
            .expect("description metadata must not trip executable hook checks");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn package_detail_json_preserves_control_contract() {
        let deps = br#"[{"id":"dep","version":"1"}]"#;
        let permissions = br#"["input-data"]"#;
        let surfaces =
            br#"[{"kind":"fcitx-addon","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"}]"#;
        let detail = Fcitx5ControlPackageDetail {
            repository_available: 1,
            repository_error: Fcitx5ControlUtf8::new(b""),
            id: Fcitx5ControlUtf8::new(b"fcitx5-rime"),
            title: Fcitx5ControlUtf8::new(b"Rime"),
            summary: Fcitx5ControlUtf8::new(b"Rime input method"),
            package_type: Fcitx5ControlUtf8::new(b"addon"),
            available_version: Fcitx5ControlUtf8::new(b"1.2.3"),
            installed_version: Fcitx5ControlUtf8::new(b"1.0.0"),
            state: Fcitx5ControlUtf8::new(b"enabled"),
            bundled: 0,
            update_available: 1,
            manifest_sha256: Fcitx5ControlUtf8::new(b"abc"),
            source_commit: Fcitx5ControlUtf8::new(b"def"),
            dependencies_json: Fcitx5ControlUtf8::new(deps),
            permissions_json: Fcitx5ControlUtf8::new(permissions),
            config_surface_json: Fcitx5ControlUtf8::new(surfaces),
        };
        let json = package_detail_json(&detail).expect("package detail should format");
        let text = String::from_utf8(json).expect("package detail JSON should be UTF-8");
        assert!(text.starts_with(r#"{"format_version":1,"repository_available":true"#));
        assert!(text.contains(r#""repository_error":null"#));
        assert!(text.contains(r#""update_available":true"#));
        assert!(text.contains(r#""manifest_sha256":"abc""#));
        assert!(text.contains(r#""dependencies":[{"id":"dep","version":"1"}]"#));
        assert!(text.contains(r#""permissions":["input-data"]"#));
        assert!(text.contains(r#""config_surface":[{"kind":"fcitx-addon""#));
    }

    #[test]
    fn package_detail_json_preserves_nullable_fields() {
        let empty = Fcitx5ControlUtf8::new(b"");
        let array = Fcitx5ControlUtf8::new(b"[]");
        let detail = Fcitx5ControlPackageDetail {
            repository_available: 0,
            repository_error: Fcitx5ControlUtf8::new(b"missing_key"),
            id: Fcitx5ControlUtf8::new(b"bundled"),
            title: Fcitx5ControlUtf8::new(b"bundled"),
            summary: empty,
            package_type: Fcitx5ControlUtf8::new(b"addon"),
            available_version: empty,
            installed_version: empty,
            state: empty,
            bundled: 1,
            update_available: 0,
            manifest_sha256: empty,
            source_commit: empty,
            dependencies_json: array,
            permissions_json: array,
            config_surface_json: array,
        };
        let json = package_detail_json(&detail).expect("package detail should format");
        let text = String::from_utf8(json).expect("package detail JSON should be UTF-8");
        assert!(text.contains(r#""repository_error":"missing_key""#));
        assert!(text.contains(r#""available_version":null"#));
        assert!(text.contains(r#""manifest_sha256":null"#));
        assert!(text.contains(r#""bundled":true"#));
    }

    #[test]
    fn schema_documents_typed_control_commands() {
        assert!(CONTROL_SCHEMA_JSON.contains(r#""format_version":1"#));
        assert!(!CONTROL_SCHEMA_JSON.contains(r#""set_presentation""#));
        assert!(CONTROL_SCHEMA_JSON.contains(r#""diagnostics_plan""#));
        assert!(CONTROL_SCHEMA_JSON.contains(r#""packages_repair""#));
        assert!(CONTROL_SCHEMA_JSON.contains(r#""package_network_owner":"fcitx5-downloader.exe""#));
        assert!(!CONTROL_SCHEMA_JSON.contains("sensitive_input\":true"));
    }

    #[test]
    fn usage_documents_typed_control_commands() {
        assert!(CONTROL_USAGE_TEXT.starts_with("Usage: fcitx5-control "));
        assert!(CONTROL_USAGE_TEXT.contains("--diagnostics-plan"));
        assert!(CONTROL_USAGE_TEXT.contains("--packages-install ID"));
        assert!(!CONTROL_USAGE_TEXT.contains("--set-presentation"));
        assert!(CONTROL_USAGE_TEXT.ends_with("--schema|--version\n"));
    }

    #[test]
    fn json_string_matches_control_output_contract() {
        assert_eq!(json_string(b"plain").as_deref(), Some(&b"\"plain\""[..]));
        assert_eq!(
            json_string(b"quote\"slash\\\n\t").as_deref(),
            Some(&br#""quote\"slash\\\n\t""#[..])
        );
        assert_eq!(
            json_string("企鹅".as_bytes()).as_deref(),
            Some(&b"\"\xe4\xbc\x81\xe9\xb9\x85\""[..])
        );
        assert_eq!(json_string(&[0x01]), None);
    }

    #[test]
    fn status_json_preserves_reachable_control_contract() {
        let id = b"rime";
        let name = b"Rime";
        let native = "中州韵".as_bytes();
        let label = "中".as_bytes();
        let reason = b"manual";
        let data_root = b"C:/Users/Test/Fcitx5";
        let owner = b"builtin";
        let status = Fcitx5ControlStatus {
            launcher_reachable: 1,
            launcher_state: 2,
            engine_state: 3,
            current_input_method_id: Fcitx5ControlUtf8::new(id),
            current_input_method_name: Fcitx5ControlUtf8::new(name),
            current_input_method_native_name: Fcitx5ControlUtf8::new(native),
            current_input_method_short_label: Fcitx5ControlUtf8::new(label),
            config_valid: 1,
            tsf_guard_disabled: 1,
            tsf_guard_reason: Fcitx5ControlUtf8::new(reason),
            data_root: Fcitx5ControlUtf8::new(data_root),
            update_owner: Fcitx5ControlUtf8::new(owner),
        };
        let json = status_json(&status).expect("status should format");
        let text = String::from_utf8(json).expect("status JSON should be UTF-8");
        assert_eq!(
            text,
            r#"{"format_version":1,"launcher_reachable":true,"launcher_state":2,"engine_state":3,"current_input_method_id":"rime","current_input_method_name":"Rime","current_input_method_native_name":"中州韵","current_input_method_short_label":"中","config_valid":true,"tsf_guard_disabled":true,"tsf_guard_reason":"manual","data_root":"C:/Users/Test/Fcitx5","update_owner":"builtin"}"#
        );
    }

    #[test]
    fn status_json_preserves_unreachable_null_fields() {
        let reason = b"";
        let data_root = br#"C:/Users/Test/Fcitx\"#;
        let owner = b"none";
        let empty = Fcitx5ControlUtf8::new(b"");
        let status = Fcitx5ControlStatus {
            launcher_reachable: 0,
            launcher_state: 9,
            engine_state: 9,
            current_input_method_id: empty,
            current_input_method_name: empty,
            current_input_method_native_name: empty,
            current_input_method_short_label: empty,
            config_valid: 0,
            tsf_guard_disabled: 0,
            tsf_guard_reason: Fcitx5ControlUtf8::new(reason),
            data_root: Fcitx5ControlUtf8::new(data_root),
            update_owner: Fcitx5ControlUtf8::new(owner),
        };
        let json = status_json(&status).expect("status should format");
        let text = String::from_utf8(json).expect("status JSON should be UTF-8");
        assert!(text.contains(r#""launcher_reachable":false"#));
        assert!(text.contains(r#""launcher_state":null"#));
        assert!(text.contains(r#""current_input_method_id":null"#));
        assert!(text.contains(r#""config_valid":false"#));
        assert!(text.contains(r#""data_root":"C:/Users/Test/Fcitx\\","#));
    }

    fn status_fixture(
        launcher_reachable: bool,
        config_valid: bool,
        tsf_guard_disabled: bool,
        tsf_guard_reason: &[u8],
    ) -> Fcitx5ControlStatus<'_> {
        let id = b"rime";
        let name = b"Rime";
        let native = "中州韵".as_bytes();
        let label = "中".as_bytes();
        let data_root = b"C:/Users/Test/Fcitx5";
        let owner = b"builtin";
        Fcitx5ControlStatus {
            launcher_reachable: launcher_reachable as u8,
            launcher_state: 2,
            engine_state: 3,
            current_input_method_id: Fcitx5ControlUtf8::new(id),
            current_input_method_name: Fcitx5ControlUtf8::new(name),
            current_input_method_native_name: Fcitx5ControlUtf8::new(native),
            current_input_method_short_label: Fcitx5ControlUtf8::new(label),
            config_valid: config_valid as u8,
            tsf_guard_disabled: tsf_guard_disabled as u8,
            tsf_guard_reason: Fcitx5ControlUtf8::new(tsf_guard_reason),
            data_root: Fcitx5ControlUtf8::new(data_root),
            update_owner: Fcitx5ControlUtf8::new(owner),
        }
    }

    #[test]
    fn diagnostics_plan_json_preserves_good_health_contract() {
        let status = status_fixture(true, true, false, b"");
        let json = diagnostics_plan_json(&status).expect("diagnostics plan should format");
        let text = String::from_utf8(json).expect("diagnostics plan should be UTF-8");
        assert_eq!(
            text,
            r#"{"format_version":1,"surface":"diagnostics","sensitive_input":false,"overall":"ok","checks":[{"id":"launcher","state":"ok","detail":"reachable","repair_action":null},{"id":"config","state":"ok","detail":"valid","repair_action":null},{"id":"tsf_guard","state":"ok","detail":"enabled","repair_action":null}],"repair":{"mode":"dry_run","result":"not_run","actions":[]}}"#
        );
    }

    #[test]
    fn diagnostics_plan_json_preserves_bad_health_repair_dry_run_contract() {
        let reason = b"manual \"disable\"\\line\nnext";
        let status = status_fixture(false, false, true, reason);
        let json = diagnostics_plan_json(&status).expect("diagnostics plan should format");
        let text = String::from_utf8(json).expect("diagnostics plan should be UTF-8");
        assert!(text.contains(r#""surface":"diagnostics""#));
        assert!(text.contains(r#""sensitive_input":false"#));
        assert!(text.contains(r#""overall":"error""#));
        assert!(text.contains(
            r#"{"id":"launcher","state":"error","detail":"unreachable","repair_action":"restart_engine"}"#
        ));
        assert!(text.contains(
            r#"{"id":"config","state":"error","detail":"invalid_config","repair_action":"validate_config"}"#
        ));
        assert!(text.contains(
            r#"{"id":"tsf_guard","state":"warning","detail":"manual \"disable\"\\line\nnext","repair_action":"reset_tsf_guard"}"#
        ));
        assert!(text.contains(r#""repair":{"mode":"dry_run","result":"not_run","actions":["#));
        assert!(text.contains(
            r#"{"id":"restart_engine","kind":"control","command":"--restart-engine","destructive":false}"#
        ));
        assert!(text.contains(
            r#"{"id":"validate_config","kind":"control","command":"--validate-config","destructive":false}"#
        ));
        assert!(text.contains(
            r#"{"id":"reset_tsf_guard","kind":"control","command":"--reset-tsf-guard","destructive":false}"#
        ));
        assert!(!text.contains("raw_key"));
        assert!(!text.contains("preedit"));
        assert!(!text.contains("candidate"));
        assert!(!text.contains("commit"));
        assert!(!text.contains("user_dictionary"));
    }

    #[test]
    fn atomic_config_file_write_matches_cpp_contract() {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-control-core-atomic-write-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let destination = root.join("nested").join("config.toml");

        atomic_write_utf8_file(destination.clone(), b"first = true\n")
            .expect("initial atomic write");
        assert_eq!(
            std::fs::read(&destination).expect("read initial content"),
            b"first = true\n"
        );

        let replacement = b"second = true\n";
        atomic_write_utf8_file(destination.clone(), replacement).expect("replacing write");
        assert_eq!(
            std::fs::read(&destination).expect("read replacement content"),
            replacement
        );
        let leftovers = std::fs::read_dir(destination.parent().expect("parent exists"))
            .expect("read parent directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
        assert!(atomic_write_utf8_file(PathBuf::new(), replacement).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn bounded_file_read_matches_cpp_contract() {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-control-core-bounded-read-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test root");
        let path = root.join("config.toml");
        std::fs::write(&path, b"format_version = 1\n").expect("write test file");
        assert_eq!(
            read_file_bounded(path.clone(), 256 * 1024).expect("read bounded file"),
            b"format_version = 1\n"
        );
        assert_eq!(
            read_file_bounded(path.clone(), 4).expect_err("too large should fail"),
            CONTROL_FILE_READ_INVALID_FILE
        );
        assert_eq!(
            read_file_bounded(root.join("missing.toml"), 256 * 1024)
                .expect_err("missing should fail"),
            CONTROL_FILE_READ_INVALID_FILE
        );

        let empty_path = root.join("empty");
        std::fs::write(&empty_path, b"").expect("write empty file");
        assert_eq!(
            read_file_bounded(empty_path, 256 * 1024).expect("read empty file"),
            b""
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn theme_discovery_matches_windinput_style_contract() {
        let install_root = PathBuf::from(r"C:\Fcitx5");
        let data_root = PathBuf::from(r"C:\Users\Alice\AppData\Roaming\Fcitx5");
        let theme_dir = data_root.join("themes").join("solar");
        assert_eq!(
            fcitx5_config_core::builtin_theme_file(&install_root, "builtin:default"),
            Some(
                install_root
                    .join("themes")
                    .join("default")
                    .join(THEME_FILE_NAME)
            )
        );
        assert_eq!(user_themes_dir(&data_root), Some(data_root.join("themes")));
        assert_eq!(
            theme_file_path(&theme_dir),
            Some(theme_dir.join(THEME_FILE_NAME))
        );
        assert!(fcitx5_config_core::builtin_theme_file(
            std::path::Path::new(""),
            "builtin:default"
        )
        .is_none());
        assert!(user_themes_dir(std::path::Path::new("")).is_none());
        assert!(theme_file_path(std::path::Path::new("")).is_none());

        let temp = std::env::temp_dir().join(format!(
            "fcitx5_control_theme_discovery_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("data").join("themes").join("solar")).unwrap();
        std::fs::create_dir_all(temp.join("data").join("themes").join("Bad")).unwrap();
        let install = temp.join("install");
        std::fs::create_dir_all(&install).unwrap();

        let entries = discover_themes(&install, &temp.join("data"));
        assert_eq!(
            entries.len(),
            fcitx5_config_core::BUILTIN_THEME_NAMES.len() + 1
        );
        assert!(entries.iter().any(|entry| {
            entry.id == "builtin:default"
                && entry.source == "builtin"
                && entry.path.ends_with(r"themes\default\theme.toml")
        }));
        assert!(entries.iter().any(|entry| {
            entry.id == "solar" && entry.source == "user" && entry.path.ends_with("theme.toml")
        }));
        assert!(!entries.iter().any(|entry| entry.id == "Bad"));

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn theme_summary_parse_matches_control_contract() {
        let text = br##"
format_version = 1

[theme]
id = "builtin.default"
name = "Default"
version = "1"
license = "LGPL-2.1-or-later"
description = "Built in"

[common.candidate.geometry]
margin = 8

[light.candidate.colors]
background = "#ffffff"

[dark.candidate.colors]
background = "#000000"
"##;
        let summary = parse_theme_summary(text).unwrap();
        assert_eq!(summary.id, "builtin.default");
        assert_eq!(summary.name, "Default");
        assert_eq!(summary.version, "1");
        assert_eq!(summary.license, "LGPL-2.1-or-later");
        assert_eq!(summary.description, "Built in");
        assert!(summary.has_light_branch);
        assert!(summary.has_dark_branch);

        assert!(parse_theme_summary(
            br##"
format_version = 2
[theme]
id = "solar"
name = "Solar"
version = "1"
license = "MIT"
"##
        )
        .is_none());
        assert!(parse_theme_summary(
            br##"
format_version = 1
unexpected = true
[theme]
id = "solar"
name = "Solar"
version = "1"
license = "MIT"
"##
        )
        .is_none());
        assert!(parse_theme_summary(
            br##"
format_version = 1
[theme]
id = "Bad ID"
name = "Solar"
version = "1"
license = "MIT"
"##
        )
        .is_none());
        assert!(parse_theme_summary(
            br##"
format_version = 1
[theme]
id = "solar"
name = "Solar"
version = "1"
[light]
"##
        )
        .is_none());
    }

    #[test]
    fn theme_path_resolve_matches_windinput_style_contract() {
        let install = std::path::PathBuf::from(r"C:\Program Files\Fcitx5");
        let data = std::path::PathBuf::from(r"C:\Users\Alice\AppData\Local\Fcitx5");

        let builtin = resolve_theme_path(&install, &data, "builtin:default", true).unwrap();
        assert!(builtin.ends_with(r"themes\default\theme.toml"));
        assert!(resolve_theme_path(&install, &data, "solar", true).is_none());

        let user = resolve_theme_path(&install, &data, "eosphoros-night", false).unwrap();
        assert!(user.ends_with(r"themes\eosphoros-night\theme.toml"));
        assert!(resolve_theme_path(&install, &data, "Bad ID", false).is_none());
        assert!(resolve_theme_path(&install, &std::path::PathBuf::new(), "solar", false).is_none());
    }

    #[test]
    fn addon_metadata_bool_matches_cpp_contract() {
        assert!(addon_metadata_bool(b"True"));
        assert!(addon_metadata_bool(b"true"));
        assert!(addon_metadata_bool(b"1"));
        assert!(!addon_metadata_bool(b"TRUE"));
        assert!(!addon_metadata_bool(b"false"));
        assert!(!addon_metadata_bool(b"0"));
        assert!(!addon_metadata_bool(b""));
    }

    #[test]
    fn repository_metadata_url_matches_cpp_contract() {
        let base = wide("https://packages.example/v1/dev///");
        assert_eq!(
            String::from_utf16(&repository_metadata_url(&base, b"index.json").unwrap()).unwrap(),
            "https://packages.example/v1/dev/index.json"
        );
        assert_eq!(
            String::from_utf16(&repository_metadata_url(&base, b"index.sig.json").unwrap())
                .unwrap(),
            "https://packages.example/v1/dev/index.sig.json"
        );
        assert!(repository_metadata_url(&base, b"nested/index.json").is_none());
    }

    #[test]
    fn repository_default_base_url_matches_cpp_contract() {
        assert_eq!(
            String::from_utf16(&repository_default_base_url(b"stable").unwrap()).unwrap(),
            "https://github.com/0x696c757a696f/fcitx5-windows-next/releases/download/v0.1.0"
        );
        assert_eq!(
            String::from_utf16(&repository_default_base_url(b"").unwrap()).unwrap(),
            "https://github.com/0x696c757a696f/fcitx5-windows-next/releases/download/v0.1.0"
        );
        assert!(repository_default_base_url(b"bad/channel").is_none());
    }

    #[test]
    fn tsf_guard_json_preserves_control_contract() {
        let reason = b"manual \"disable\"";
        let marker = br#"C:/Users/Test/Fcitx5/tsf-guard.txt"#;
        let status = Fcitx5ControlTsfGuard {
            disabled: 1,
            reason: Fcitx5ControlUtf8::new(reason),
            marker_path: Fcitx5ControlUtf8::new(marker),
        };
        let json = tsf_guard_json(&status).expect("guard status should format");
        let text = String::from_utf8(json).expect("guard JSON should be UTF-8");
        assert_eq!(
            text,
            r#"{"format_version":1,"disabled":true,"reason":"manual \"disable\"","marker_path":"C:/Users/Test/Fcitx5/tsf-guard.txt"}"#
        );
    }
}
