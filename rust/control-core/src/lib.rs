#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_void, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::ptr::null_mut;

type Dword = u32;
type Lstatus = i32;
type Hkey = *mut c_void;

const GENERIC_WRITE: Dword = 0x4000_0000;
const CREATE_NEW: Dword = 1;
const FILE_ATTRIBUTE_NORMAL: Dword = 0x0000_0080;
const FILE_FLAG_WRITE_THROUGH: Dword = 0x8000_0000;
const MOVEFILE_REPLACE_EXISTING: Dword = 0x0000_0001;
const MOVEFILE_WRITE_THROUGH: Dword = 0x0000_0008;
const ERROR_SUCCESS: Lstatus = 0;
const ERROR_FILE_NOT_FOUND: Lstatus = 2;
const KEY_QUERY_VALUE: Dword = 0x0001;
const KEY_SET_VALUE: Dword = 0x0002;
const REG_SZ: Dword = 1;
const HKEY_CURRENT_USER: Hkey = 0x8000_0001_usize as Hkey;
const INVALID_HANDLE_VALUE: *mut c_void = usize::MAX as *mut c_void;
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
const CONTROL_TSF_GUARD_RESET_JSON: &str = r#"{"format_version":1,"tsf_guard":"enabled"}"#;
const CONTROL_LAUNCHER_ACTION_RESTART_ENGINE: u32 = 1;
const CONTROL_LAUNCHER_ACTION_SHUTDOWN: u32 = 2;
const CONTROL_ROOT_ACTION_UNKNOWN: u32 = 0;
const CONTROL_ROOT_ACTION_VERSION: u32 = 1;
const CONTROL_ROOT_ACTION_SCHEMA: u32 = 2;
const CONTROL_ROOT_ACTION_GET_STARTUP: u32 = 3;
const CONTROL_ROOT_ACTION_SET_STARTUP_ENABLED: u32 = 4;
const CONTROL_ROOT_ACTION_SET_STARTUP_DISABLED: u32 = 5;
const CONTROL_ROOT_ACTION_GET_TSF_GUARD: u32 = 6;
const CONTROL_ROOT_ACTION_RESET_TSF_GUARD: u32 = 7;
const CONTROL_ROOT_ACTION_STATUS: u32 = 8;
const CONTROL_ROOT_ACTION_RESTART_ENGINE: u32 = 9;
const CONTROL_ROOT_ACTION_SHUTDOWN: u32 = 10;
const CONTROL_ROOT_ACTION_DIAGNOSTICS_PLAN: u32 = 11;
const CONTROL_FILE_READ_OK: i32 = 0;
const CONTROL_FILE_READ_INVALID_FILE: i32 = 1;
const CONTROL_FILE_READ_IO_ERROR: i32 = 2;
const CONTROL_FILE_READ_MISSING: i32 = 3;
const CONTROL_ARCHIVE_CACHE_INVALID: i32 = 1;
const CONTROL_ARCHIVE_CACHE_STALE_REMOVED: i32 = 2;
const CONTROL_MAXIMUM_MANIFEST_BYTES: u64 = 1024 * 1024;
const CONTROL_CONFIG_ACTION_UNKNOWN: u32 = 0;
const CONTROL_CONFIG_ACTION_VALIDATE: u32 = 1;
const CONTROL_CONFIG_ACTION_APPLY: u32 = 2;
const CONTROL_CONFIG_ACTION_RESET_CONFIG: u32 = 3;
const CONTROL_PACKAGE_TYPE_CORE: u32 = 0;
const CONTROL_PACKAGE_TYPE_ADDON: u32 = 1;
const CONTROL_PACKAGE_TYPE_INPUT_METHOD_DATA: u32 = 2;
const CONTROL_PACKAGE_TYPE_THEME: u32 = 3;
const CONTROL_PACKAGE_TYPE_TRANSLATION: u32 = 4;
const LANG_CHINESE: u16 = 0x04;
const CONFIG_LOCALE_EN_US_FILE: &[u16] = &[
    b'e' as u16,
    b'n' as u16,
    b'-' as u16,
    b'U' as u16,
    b'S' as u16,
    b'.' as u16,
    b'j' as u16,
    b's' as u16,
    b'o' as u16,
    b'n' as u16,
];
const CONFIG_LOCALE_ZH_CN_FILE: &[u16] = &[
    b'z' as u16,
    b'h' as u16,
    b'-' as u16,
    b'C' as u16,
    b'N' as u16,
    b'.' as u16,
    b'j' as u16,
    b's' as u16,
    b'o' as u16,
    b'n' as u16,
];
const CONTROL_ENGINE_ACTION_UNKNOWN: u32 = 0;
const CONTROL_ENGINE_ACTION_GET_INPUT_METHODS: u32 = 1;
const CONTROL_ENGINE_ACTION_SET_INPUT_METHOD: u32 = 2;
const CONTROL_PACKAGE_ACTION_UNKNOWN: u32 = 0;
const CONTROL_PACKAGE_ACTION_PACKAGES_LIST: u32 = 1;
const CONTROL_PACKAGE_ACTION_THEMES_LIST: u32 = 2;
const CONTROL_PACKAGE_ACTION_THEMES_DETAIL: u32 = 3;
const CONTROL_PACKAGE_ACTION_ADDONS_LIST: u32 = 4;
const CONTROL_PACKAGE_ACTION_PACKAGES_DETAIL: u32 = 5;
const CONTROL_PACKAGE_ACTION_PACKAGES_REFRESH: u32 = 6;
const CONTROL_PACKAGE_ACTION_PACKAGES_INSTALL: u32 = 7;
const CONTROL_PACKAGE_ACTION_PACKAGES_UPDATE: u32 = 8;
const CONTROL_PACKAGE_ACTION_PACKAGES_STATE: u32 = 9;
const CONTROL_PACKAGE_ACTION_PACKAGES_REMOVE: u32 = 10;
const CONTROL_PACKAGE_ACTION_PACKAGES_REPAIR: u32 = 11;
const CONTROL_PACKAGE_ACTION_THEMES_EXPORT: u32 = 12;
const CONTROL_PACKAGE_ACTION_THEMES_IMPORT: u32 = 13;
const CONTROL_PACKAGE_ACTION_THEMES_DUPLICATE: u32 = 14;
const CONTROL_PACKAGE_ACTION_THEMES_DELETE: u32 = 15;
const CONTROL_PACKAGE_ACTION_THEMES_EXPORT_TO: u32 = 16;
const CONTROL_THEME_OPERATION_INVALID: i32 = 1;
const CONTROL_THEME_OPERATION_NOT_FOUND: i32 = 2;
const CONTROL_THEME_OPERATION_IO_ERROR: i32 = 3;
const CONTROL_THEME_OPERATION_READ_ONLY: i32 = 4;
const CONTROL_THEME_OPERATION_UNSAFE_IMPORT: i32 = 5;
const CONTROL_THEME_OPERATION_ALREADY_EXISTS: i32 = 6;
const LAUNCHER_COMMAND_START_DEMAND: u32 = 1;
const LAUNCHER_COMMAND_USER_STOP: u32 = 2;
const LAUNCHER_COMMAND_RESUME: u32 = 3;
const LAUNCHER_COMMAND_SHUTDOWN: u32 = 9;
const CONTROL_RESTART_ENGINE_COMMANDS: &[u32] = &[
    LAUNCHER_COMMAND_USER_STOP,
    LAUNCHER_COMMAND_RESUME,
    LAUNCHER_COMMAND_START_DEMAND,
];
const CONTROL_SHUTDOWN_COMMANDS: &[u32] = &[LAUNCHER_COMMAND_SHUTDOWN];

#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

pub fn control_schema_json() -> &'static str {
    CONTROL_SCHEMA_JSON
}

pub fn control_usage_text() -> &'static str {
    CONTROL_USAGE_TEXT
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
        reason: Fcitx5ControlUtf8 {
            ptr: reason.as_ptr(),
            len: reason.len(),
        },
        marker_path: Fcitx5ControlUtf8 {
            ptr: marker.as_bytes().as_ptr(),
            len: marker.len(),
        },
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

pub fn control_diagnostics_plan_json(data_root: &str, config_valid: bool) -> Option<String> {
    control_diagnostics_plan_for_path(std::path::Path::new(data_root), config_valid)
}

pub fn control_diagnostics_plan_for_path(
    data_root: &std::path::Path,
    config_valid: bool,
) -> Option<String> {
    let (tsf_guard_disabled, tsf_guard_reason) = tsf_guard_state(data_root).ok()?;
    let data_root = data_root.to_string_lossy();
    let empty = Fcitx5ControlUtf8::default();
    let root = Fcitx5ControlUtf8 {
        ptr: data_root.as_bytes().as_ptr(),
        len: data_root.len(),
    };
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
        tsf_guard_reason: Fcitx5ControlUtf8 {
            ptr: tsf_guard_reason.as_ptr(),
            len: tsf_guard_reason.len(),
        },
        data_root: root,
        update_owner: empty,
    };
    String::from_utf8(diagnostics_plan_json(&status)?).ok()
}

fn control_install_root() -> Result<PathBuf, &'static str> {
    let executable = std::env::current_exe().map_err(|_| "unable to resolve installation root")?;
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

fn view(value: &str) -> Fcitx5ControlUtf8 {
    Fcitx5ControlUtf8 {
        ptr: value.as_ptr(),
        len: value.len(),
    }
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
        packages: views.as_ptr(),
        package_count: views.len(),
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
            (summary.id == entry.id || entry.id == "builtin:default").then_some((
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

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Fcitx5ControlUtf16 {
    ptr: *const u16,
    len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Fcitx5ControlUtf8 {
    ptr: *const u8,
    len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Fcitx5ControlParsedConfigCommandLine {
    status: u8,
    command_len: usize,
    locale_len: usize,
}

#[repr(C)]
pub struct Fcitx5ControlStatus {
    launcher_reachable: u8,
    launcher_state: i32,
    engine_state: i32,
    current_input_method_id: Fcitx5ControlUtf8,
    current_input_method_name: Fcitx5ControlUtf8,
    current_input_method_native_name: Fcitx5ControlUtf8,
    current_input_method_short_label: Fcitx5ControlUtf8,
    config_valid: u8,
    tsf_guard_disabled: u8,
    tsf_guard_reason: Fcitx5ControlUtf8,
    data_root: Fcitx5ControlUtf8,
    update_owner: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlTsfGuard {
    disabled: u8,
    reason: Fcitx5ControlUtf8,
    marker_path: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlPackageRepair {
    repository_sequence_state: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlAddonDescriptor {
    id: Fcitx5ControlUtf8,
    name: Fcitx5ControlUtf8,
    category: Fcitx5ControlUtf8,
    library: Fcitx5ControlUtf8,
    addon_type: Fcitx5ControlUtf8,
    version: Fcitx5ControlUtf8,
    configurable: u8,
    on_demand: u8,
    library_present: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Fcitx5ControlThemeRecord {
    id: Fcitx5ControlUtf8,
    source: Fcitx5ControlUtf8,
    name: Fcitx5ControlUtf8,
    version: Fcitx5ControlUtf8,
    license: Fcitx5ControlUtf8,
    description: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlThemeDiscoveryEntry {
    path: Fcitx5ControlUtf16,
    id: Fcitx5ControlUtf8,
    source: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlThemeDiscoveryResult {
    status: i32,
    entries: *mut Fcitx5ControlThemeDiscoveryEntry,
    entry_count: usize,
}

#[repr(C)]
pub struct Fcitx5ControlThemeSummaryResult {
    status: i32,
    id: Fcitx5ControlUtf8,
    name: Fcitx5ControlUtf8,
    version: Fcitx5ControlUtf8,
    license: Fcitx5ControlUtf8,
    description: Fcitx5ControlUtf8,
    has_light_branch: u8,
    has_dark_branch: u8,
}

#[repr(C)]
pub struct Fcitx5ControlThemeDetail {
    theme: Fcitx5ControlThemeRecord,
    has_light_branch: u8,
    has_dark_branch: u8,
}

#[repr(C)]
pub struct Fcitx5ControlPackageSummary {
    id: Fcitx5ControlUtf8,
    title: Fcitx5ControlUtf8,
    summary: Fcitx5ControlUtf8,
    package_type: Fcitx5ControlUtf8,
    available_version: Fcitx5ControlUtf8,
    installed_version: Fcitx5ControlUtf8,
    state: Fcitx5ControlUtf8,
    update_available: u8,
}

#[repr(C)]
pub struct Fcitx5ControlPackagesList {
    repository_available: u8,
    repository_error: Fcitx5ControlUtf8,
    packages: *const Fcitx5ControlPackageSummary,
    package_count: usize,
}

#[repr(C)]
pub struct Fcitx5ControlPackageDependency {
    id: Fcitx5ControlUtf8,
    version: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlPackageDetail {
    repository_available: u8,
    repository_error: Fcitx5ControlUtf8,
    id: Fcitx5ControlUtf8,
    title: Fcitx5ControlUtf8,
    summary: Fcitx5ControlUtf8,
    package_type: Fcitx5ControlUtf8,
    available_version: Fcitx5ControlUtf8,
    installed_version: Fcitx5ControlUtf8,
    state: Fcitx5ControlUtf8,
    bundled: u8,
    update_available: u8,
    manifest_sha256: Fcitx5ControlUtf8,
    source_commit: Fcitx5ControlUtf8,
    dependencies_json: Fcitx5ControlUtf8,
    permissions_json: Fcitx5ControlUtf8,
    config_surface_json: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlBundledPackageDescriptor {
    id: Fcitx5ControlUtf8,
    title: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlPathResult {
    status: i32,
    path_len: usize,
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
    fn CreateFileW(
        file_name: *const u16,
        desired_access: Dword,
        share_mode: Dword,
        security_attributes: *mut c_void,
        creation_disposition: Dword,
        flags_and_attributes: Dword,
        template_file: *mut c_void,
    ) -> *mut c_void;
    fn WriteFile(
        file: *mut c_void,
        buffer: *const c_void,
        number_of_bytes_to_write: Dword,
        number_of_bytes_written: *mut Dword,
        overlapped: *mut c_void,
    ) -> i32;
    fn FlushFileBuffers(file: *mut c_void) -> i32;
    fn CloseHandle(object: *mut c_void) -> i32;
    fn DeleteFileW(file_name: *const u16) -> i32;
    fn MoveFileExW(existing_file_name: *const u16, new_file_name: *const u16, flags: Dword) -> i32;
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoCreateGuid(guid: *mut Guid) -> i32;
    fn StringFromGUID2(guid: *const Guid, string: *mut u16, max: i32) -> i32;
}

struct RegistryKey(Hkey);

impl RegistryKey {
    fn get(&self) -> Hkey {
        self.0
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

struct FileHandle(*mut c_void);

impl FileHandle {
    fn get(&self) -> *mut c_void {
        self.0
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn string_from_utf16(value: Fcitx5ControlUtf16) -> Option<OsString> {
    if value.ptr.is_null() {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    Some(OsString::from_wide(slice))
}

fn wide_z(value: &std::ffi::OsStr) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_wide().collect();
    wide.push(0);
    wide
}

fn write_wide_units(value: &[u16], out: *mut u16, capacity: usize) -> usize {
    if !out.is_null() && capacity != 0 {
        let count = value.len().min(capacity);
        if count != 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), out, count);
            }
        }
    }
    value.len()
}

fn write_wide_path(value: &std::path::Path, out: *mut u16, capacity: usize) -> usize {
    let wide: Vec<u16> = value.as_os_str().encode_wide().collect();
    write_wide_units(&wide, out, capacity)
}

fn split_windows_argument_string(input: &[u16]) -> Vec<Vec<u16>> {
    let mut arguments = Vec::new();
    let mut index = 0usize;
    while index < input.len() {
        while index < input.len() && matches!(input[index], 0x20 | 0x09) {
            index += 1;
        }
        if index >= input.len() {
            break;
        }
        let mut argument = Vec::new();
        let mut quoted = false;
        while index < input.len() {
            if !quoted && matches!(input[index], 0x20 | 0x09) {
                break;
            }
            let mut backslashes = 0usize;
            while index < input.len() && input[index] == b'\\' as u16 {
                backslashes += 1;
                index += 1;
            }
            if index < input.len() && input[index] == b'"' as u16 {
                argument.extend(std::iter::repeat_n(b'\\' as u16, backslashes / 2));
                if backslashes.is_multiple_of(2) {
                    quoted = !quoted;
                } else {
                    argument.push(b'"' as u16);
                }
                index += 1;
                continue;
            }
            argument.extend(std::iter::repeat_n(b'\\' as u16, backslashes));
            if index >= input.len() {
                break;
            }
            argument.push(input[index]);
            index += 1;
        }
        arguments.push(argument);
    }
    arguments
}

fn utf16_starts_with_ascii(value: &[u16], prefix: &[u8]) -> bool {
    value.len() >= prefix.len()
        && value
            .iter()
            .zip(prefix)
            .all(|(character, ascii)| *character == *ascii as u16)
}

fn utf16_eq_ascii(value: &[u16], expected: &[u8]) -> bool {
    value.len() == expected.len()
        && value
            .iter()
            .zip(expected)
            .all(|(character, ascii)| *character == *ascii as u16)
}

fn parse_config_command_line(input: &[u16]) -> Option<(Vec<u16>, Vec<u16>)> {
    let mut command = Vec::new();
    let mut locale = Vec::new();
    for argument in split_windows_argument_string(input) {
        if utf16_starts_with_ascii(&argument, b"--lang=") {
            if !locale.is_empty() {
                return None;
            }
            locale = argument[7..].to_vec();
        } else if !argument.is_empty() {
            if !command.is_empty() {
                return None;
            }
            command = argument;
        }
    }
    Some((command, locale))
}

fn primary_lang_id(language: u16) -> u16 {
    language & 0x03ff
}

fn user_default_ui_language_prefers_chinese() -> bool {
    primary_lang_id(unsafe { GetUserDefaultUILanguage() }) == LANG_CHINESE
}

fn config_locale_file_for_override(override_locale: &[u16]) -> Option<&'static [u16]> {
    if override_locale.is_empty() || utf16_eq_ascii(override_locale, b"system") {
        return Some(if user_default_ui_language_prefers_chinese() {
            CONFIG_LOCALE_ZH_CN_FILE
        } else {
            CONFIG_LOCALE_EN_US_FILE
        });
    }
    if utf16_eq_ascii(override_locale, b"zh-CN") {
        return Some(CONFIG_LOCALE_ZH_CN_FILE);
    }
    if utf16_eq_ascii(override_locale, b"en-US") {
        return Some(CONFIG_LOCALE_EN_US_FILE);
    }
    None
}

fn guid_suffix() -> Option<OsString> {
    let mut guid = Guid {
        data1: 0,
        data2: 0,
        data3: 0,
        data4: [0; 8],
    };
    let status = unsafe { CoCreateGuid(&mut guid) };
    if status < 0 {
        return None;
    }
    let mut buffer = [0_u16; 40];
    let len = unsafe { StringFromGUID2(&guid, buffer.as_mut_ptr(), buffer.len() as i32) };
    if len <= 1 || len as usize > buffer.len() {
        return None;
    }
    Some(OsString::from_wide(&buffer[..len as usize - 1]))
}

fn temporary_path_for_atomic_write(destination: &std::path::Path) -> Option<PathBuf> {
    let mut temporary = destination.as_os_str().to_owned();
    temporary.push(".");
    temporary.push(guid_suffix()?);
    temporary.push(".tmp");
    Some(PathBuf::from(temporary))
}

fn atomic_write_utf8_file(destination: PathBuf, text: &[u8]) -> Result<(), ()> {
    if destination.as_os_str().is_empty() || text.len() > Dword::MAX as usize {
        return Err(());
    }
    let parent = destination.parent().ok_or(())?;
    std::fs::create_dir_all(parent).map_err(|_| ())?;
    let temporary = temporary_path_for_atomic_write(&destination).ok_or(())?;
    let temporary_wide = wide_z(temporary.as_os_str());
    let destination_wide = wide_z(destination.as_os_str());
    let raw_file = unsafe {
        CreateFileW(
            temporary_wide.as_ptr(),
            GENERIC_WRITE,
            0,
            null_mut(),
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH,
            null_mut(),
        )
    };
    if raw_file == INVALID_HANDLE_VALUE || raw_file.is_null() {
        return Err(());
    }
    let file = FileHandle(raw_file);
    let mut written = 0_u32;
    let write_ok = unsafe {
        WriteFile(
            file.get(),
            text.as_ptr().cast(),
            text.len() as Dword,
            &mut written,
            null_mut(),
        ) != 0
    } && written as usize == text.len()
        && unsafe { FlushFileBuffers(file.get()) != 0 };
    drop(file);
    if !write_ok {
        unsafe {
            let _ = DeleteFileW(temporary_wide.as_ptr());
        }
        return Err(());
    }
    let moved = unsafe {
        MoveFileExW(
            temporary_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        ) != 0
    };
    if !moved {
        unsafe {
            let _ = DeleteFileW(temporary_wide.as_ptr());
        }
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

fn read_optional_file_bounded(path: PathBuf, maximum: u64) -> Result<Option<Vec<u8>>, i32> {
    if path.as_os_str().is_empty() {
        return Err(CONTROL_FILE_READ_INVALID_FILE);
    }
    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(CONTROL_FILE_READ_INVALID_FILE),
    };
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(CONTROL_FILE_READ_INVALID_FILE);
    }
    let bytes = std::fs::read(&path).map_err(|_| CONTROL_FILE_READ_IO_ERROR)?;
    if bytes.len() as u64 > maximum {
        return Err(CONTROL_FILE_READ_INVALID_FILE);
    }
    Ok(Some(bytes))
}

fn installed_manifest_path(
    package_root: &std::path::Path,
    id: &str,
    version: &str,
) -> Option<PathBuf> {
    if package_root.as_os_str().is_empty() || id.is_empty() || version.is_empty() {
        return None;
    }
    Some(
        package_root
            .join("manifests")
            .join(id)
            .join(format!("{version}.json")),
    )
}

fn read_installed_manifest_bytes(
    package_root: &std::path::Path,
    id: &str,
    version: &str,
) -> Result<Vec<u8>, i32> {
    let path =
        installed_manifest_path(package_root, id, version).ok_or(CONTROL_FILE_READ_INVALID_FILE)?;
    read_file_bounded(path, CONTROL_MAXIMUM_MANIFEST_BYTES)
}

fn repository_cache_incoming_path(path: &std::path::Path) -> Option<PathBuf> {
    if path.as_os_str().is_empty() {
        return None;
    }
    let mut incoming = path.as_os_str().to_owned();
    incoming.push(".new");
    Some(PathBuf::from(incoming))
}

fn remove_repository_incoming(
    index: &std::path::Path,
    signature: &std::path::Path,
) -> Result<(), ()> {
    let incoming_index = repository_cache_incoming_path(index).ok_or(())?;
    let incoming_signature = repository_cache_incoming_path(signature).ok_or(())?;
    for path in [incoming_index, incoming_signature] {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(()),
        }
    }
    Ok(())
}

fn prepare_repository_cache(
    index: &std::path::Path,
    signature: &std::path::Path,
) -> Result<(), ()> {
    if let Some(parent) = index.parent() {
        std::fs::create_dir_all(parent).map_err(|_| ())?;
    } else {
        return Err(());
    }
    if let Some(parent) = signature.parent() {
        std::fs::create_dir_all(parent).map_err(|_| ())?;
    } else {
        return Err(());
    }
    remove_repository_incoming(index, signature)
}

fn move_replace_write_through(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), ()> {
    let source = wide_z(source.as_os_str());
    let destination = wide_z(destination.as_os_str());
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        ) != 0
    };
    moved.then_some(()).ok_or(())
}

fn publish_repository_cache(
    index: &std::path::Path,
    signature: &std::path::Path,
) -> Result<(), ()> {
    let incoming_index = repository_cache_incoming_path(index).ok_or(())?;
    let incoming_signature = repository_cache_incoming_path(signature).ok_or(())?;
    move_replace_write_through(&incoming_signature, signature)?;
    move_replace_write_through(&incoming_index, index)
}

fn ascii_token_from_utf8(value: Fcitx5ControlUtf8) -> Option<String> {
    let bytes = utf8_slice(value)?;
    if bytes.is_empty()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return None;
    }
    String::from_utf8(bytes.to_vec()).ok()
}

fn package_archive_cache_path(
    data_root: &std::path::Path,
    id: &str,
    version: &str,
) -> Option<PathBuf> {
    if data_root.as_os_str().is_empty() || id.is_empty() || version.is_empty() {
        return None;
    }
    Some(
        data_root
            .join("downloads")
            .join(format!("{id}-{version}.fcpkg")),
    )
}

fn prepare_package_archive_cache(
    data_root: &std::path::Path,
    id: &str,
    version: &str,
    existing_hash_matches: bool,
) -> Result<(PathBuf, bool), i32> {
    let archive =
        package_archive_cache_path(data_root, id, version).ok_or(CONTROL_ARCHIVE_CACHE_INVALID)?;
    let downloads = archive.parent().ok_or(CONTROL_ARCHIVE_CACHE_INVALID)?;
    std::fs::create_dir_all(downloads).map_err(|_| CONTROL_ARCHIVE_CACHE_INVALID)?;
    let existed = archive.exists();
    if existed && !existing_hash_matches {
        std::fs::remove_file(&archive).map_err(|_| CONTROL_ARCHIVE_CACHE_INVALID)?;
        return Ok((archive, true));
    }
    Ok((archive, false))
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

fn utf8_slice(value: Fcitx5ControlUtf8) -> Option<&'static [u8]> {
    if value.ptr.is_null() {
        return None;
    }
    Some(unsafe { std::slice::from_raw_parts(value.ptr, value.len) })
}

fn boxed_utf8_result(value: Vec<u8>, out_ptr: *mut *mut u8, out_len: *mut usize) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    let mut bytes = value.into_boxed_slice();
    let ptr = bytes.as_mut_ptr();
    let len = bytes.len();
    std::mem::forget(bytes);
    unsafe {
        *out_ptr = ptr;
        *out_len = len;
    }
    0
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
    value: Fcitx5ControlUtf8,
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
    fn view(bytes: &[u8]) -> Fcitx5ControlUtf8 {
        Fcitx5ControlUtf8 {
            ptr: bytes.as_ptr(),
            len: bytes.len(),
        }
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

fn launcher_action_sequence(action: u32) -> Option<&'static [u32]> {
    match action {
        CONTROL_LAUNCHER_ACTION_RESTART_ENGINE => Some(CONTROL_RESTART_ENGINE_COMMANDS),
        CONTROL_LAUNCHER_ACTION_SHUTDOWN => Some(CONTROL_SHUTDOWN_COMMANDS),
        _ => None,
    }
}

fn package_repair_json(repair: &Fcitx5ControlPackageRepair) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"repair":"verified","#);
    push_json_string_field(
        &mut output,
        b"\"repository_sequence_state\"",
        utf8_slice(repair.repository_sequence_state)?,
    )?;
    output.push(b'}');
    Some(output)
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
    if list.packages.is_null() && list.package_count != 0 {
        return None;
    }
    let packages = if list.package_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(list.packages, list.package_count) }
    };
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
    value: Fcitx5ControlUtf8,
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

fn static_utf8_view(value: &'static str) -> Fcitx5ControlUtf8 {
    Fcitx5ControlUtf8 {
        ptr: value.as_ptr(),
        len: value.len(),
    }
}

fn package_type_name(package_type: u32) -> Fcitx5ControlUtf8 {
    match package_type {
        CONTROL_PACKAGE_TYPE_CORE => static_utf8_view("core"),
        CONTROL_PACKAGE_TYPE_ADDON => static_utf8_view("addon"),
        CONTROL_PACKAGE_TYPE_INPUT_METHOD_DATA => static_utf8_view("inputmethod-data"),
        CONTROL_PACKAGE_TYPE_THEME => static_utf8_view("theme"),
        CONTROL_PACKAGE_TYPE_TRANSLATION => static_utf8_view("translation"),
        _ => static_utf8_view("unknown"),
    }
}

fn builtin_theme_id() -> Fcitx5ControlUtf8 {
    static_utf8_view("builtin:default")
}

const THEME_FILE_NAME: &str = "theme.toml";

fn builtin_theme_source() -> Fcitx5ControlUtf8 {
    static_utf8_view("builtin")
}

fn user_theme_source() -> Fcitx5ControlUtf8 {
    static_utf8_view("user")
}

fn theme_record_matches_requested_id(source: &[u8], requested_id: &[u8], theme_id: &[u8]) -> bool {
    source != b"user" || theme_id == requested_id
}

fn builtin_theme_path(install_root: &std::path::Path) -> Option<PathBuf> {
    if install_root.as_os_str().is_empty() {
        return None;
    }
    Some(
        install_root
            .join("resources")
            .join("themes")
            .join("default")
            .join(THEME_FILE_NAME),
    )
}

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
        return (requested_id == "builtin:default")
            .then(|| builtin_theme_path(install_root))
            .flatten();
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
    let builtin = id == "builtin:default";
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
    if let Some(path) = builtin_theme_path(install_root) {
        entries.push(ThemeDiscoveryEntryOwned {
            path,
            id: "builtin:default".to_owned(),
            source: "builtin",
        });
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

fn deep_merge_tables(destination: &mut toml_edit::Table, source: &toml_edit::Table) {
    for (key, value) in source.iter() {
        if key == "palette" {
            continue;
        }
        if let (Some(destination_table), Some(source_table)) = (
            destination
                .get_mut(key)
                .and_then(toml_edit::Item::as_table_mut),
            value.as_table(),
        ) {
            deep_merge_tables(destination_table, source_table);
        } else {
            destination.insert(key, value.clone());
        }
    }
}

fn collect_theme_palette(
    root: &toml_edit::Table,
    branch_key: &str,
) -> Option<std::collections::HashMap<String, String>> {
    let mut palette = std::collections::HashMap::new();
    if let Some(table) = root.get("palette") {
        insert_theme_palette_entries(table.as_table()?, &mut palette)?;
    }
    if let Some(branch) = theme_branch_table(root, branch_key)? {
        if let Some(table) = branch.get("palette") {
            insert_theme_palette_entries(table.as_table()?, &mut palette)?;
        }
    }
    Some(palette)
}

fn insert_theme_palette_entries(
    table: &toml_edit::Table,
    palette: &mut std::collections::HashMap<String, String>,
) -> Option<()> {
    validate_theme_palette_table(table)?;
    for (key, value) in table.iter() {
        palette.insert(key.to_string(), value.as_str()?.trim().to_string());
    }
    Some(())
}

fn resolve_theme_palette_name(
    name: &str,
    palette: &std::collections::HashMap<String, String>,
    resolved: &mut std::collections::HashMap<String, String>,
    visiting: &mut std::collections::HashSet<String>,
) -> Option<String> {
    if let Some(value) = resolved.get(name) {
        return Some(value.clone());
    }
    if !visiting.insert(name.to_string()) {
        return None;
    }
    let raw = palette.get(name)?;
    let value = if let Some(reference) = theme_palette_reference(raw) {
        resolve_theme_palette_name(reference, palette, resolved, visiting)?
    } else if theme_color_hex_valid(raw) {
        raw.to_string()
    } else {
        return None;
    };
    visiting.remove(name);
    resolved.insert(name.to_string(), value.clone());
    Some(value)
}

fn resolve_theme_color_value(
    value: &str,
    palette: &std::collections::HashMap<String, String>,
    resolved: &mut std::collections::HashMap<String, String>,
) -> Option<String> {
    let value = value.trim();
    let value = if let Some(reference) = theme_palette_reference(value) {
        let mut visiting = std::collections::HashSet::new();
        resolve_theme_palette_name(reference, palette, resolved, &mut visiting)?
    } else {
        value.to_string()
    };
    theme_color_hex_valid(&value).then_some(value)
}

fn resolve_theme_candidate_color_refs(
    document: &mut toml_edit::DocumentMut,
    palette: &std::collections::HashMap<String, String>,
) -> Option<()> {
    let Some(candidate) = document
        .as_table_mut()
        .get_mut("candidate")
        .and_then(toml_edit::Item::as_table_mut)
    else {
        return Some(());
    };
    let Some(colors) = candidate
        .get_mut("colors")
        .and_then(toml_edit::Item::as_table_mut)
    else {
        return Some(());
    };
    let mut resolved = std::collections::HashMap::new();
    for (_, item) in colors.iter_mut() {
        let value = item.as_str()?;
        let value = resolve_theme_color_value(value, palette, &mut resolved)?;
        *item = toml_edit::value(value);
    }
    Some(())
}

fn resolved_theme_config(
    text: &[u8],
    requested_id: &[u8],
    builtin: bool,
    dark: bool,
) -> Option<Vec<u8>> {
    let requested_id = std::str::from_utf8(requested_id).ok()?;
    let value = parse_theme_document(text)?;
    let root = value.as_table();
    let summary = theme_summary_from_root(root)?;
    let source = if builtin {
        b"builtin".as_slice()
    } else {
        b"user".as_slice()
    };
    if !theme_record_matches_requested_id(source, requested_id.as_bytes(), summary.id.as_bytes()) {
        return None;
    }
    let branch_key = if dark { "dark" } else { "light" };
    let palette = collect_theme_palette(root, branch_key)?;
    let mut resolved = toml_edit::DocumentMut::new();
    resolved["format_version"] = toml_edit::value(1);
    if let Some(common) = theme_common_table(root)? {
        deep_merge_tables(resolved.as_table_mut(), common);
    }
    if let Some(branch) = theme_branch_table(root, branch_key)? {
        deep_merge_tables(resolved.as_table_mut(), branch);
    }
    resolve_theme_candidate_color_refs(&mut resolved, &palette)?;
    Some(resolved.to_string().into_bytes())
}

fn leak_utf8_slice(value: &str) -> Fcitx5ControlUtf8 {
    let mut boxed = value.as_bytes().to_vec().into_boxed_slice();
    let view = Fcitx5ControlUtf8 {
        ptr: boxed.as_mut_ptr(),
        len: boxed.len(),
    };
    std::mem::forget(boxed);
    view
}

fn leak_utf16_path(value: &std::path::Path) -> Fcitx5ControlUtf16 {
    let mut boxed = value
        .as_os_str()
        .encode_wide()
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let view = Fcitx5ControlUtf16 {
        ptr: boxed.as_mut_ptr(),
        len: boxed.len(),
    };
    std::mem::forget(boxed);
    view
}

unsafe fn free_utf8_slice(value: Fcitx5ControlUtf8) {
    if !value.ptr.is_null() {
        let slice = std::ptr::slice_from_raw_parts_mut(value.ptr as *mut u8, value.len);
        unsafe {
            drop(Box::from_raw(slice));
        }
    }
}

unsafe fn free_utf16_slice(value: Fcitx5ControlUtf16) {
    if !value.ptr.is_null() {
        let slice = std::ptr::slice_from_raw_parts_mut(value.ptr as *mut u16, value.len);
        unsafe {
            drop(Box::from_raw(slice));
        }
    }
}

fn native_package_architecture() -> Fcitx5ControlUtf8 {
    if cfg!(target_pointer_width = "64") {
        static_utf8_view("x64")
    } else {
        static_utf8_view("x86")
    }
}

fn package_architecture_matches_native(architecture: &[u8]) -> bool {
    let native = if cfg!(target_pointer_width = "64") {
        b"x64".as_slice()
    } else {
        b"x86".as_slice()
    };
    architecture == b"any" || architecture == native
}

const ADDON_METADATA_KEY_UNKNOWN: u32 = 0;
const ADDON_METADATA_KEY_NAME: u32 = 1;
const ADDON_METADATA_KEY_CATEGORY: u32 = 2;
const ADDON_METADATA_KEY_LIBRARY: u32 = 3;
const ADDON_METADATA_KEY_TYPE: u32 = 4;
const ADDON_METADATA_KEY_VERSION: u32 = 5;
const ADDON_METADATA_KEY_CONFIGURABLE: u32 = 6;
const ADDON_METADATA_KEY_ON_DEMAND: u32 = 7;

fn addon_metadata_section_is_addon(section: &[u8]) -> bool {
    section == b"[Addon]"
}

fn addon_metadata_key(key: &[u8]) -> u32 {
    match key {
        b"Name" => ADDON_METADATA_KEY_NAME,
        b"Category" => ADDON_METADATA_KEY_CATEGORY,
        b"Library" => ADDON_METADATA_KEY_LIBRARY,
        b"Type" => ADDON_METADATA_KEY_TYPE,
        b"Version" => ADDON_METADATA_KEY_VERSION,
        b"Configurable" => ADDON_METADATA_KEY_CONFIGURABLE,
        b"OnDemand" => ADDON_METADATA_KEY_ON_DEMAND,
        _ => ADDON_METADATA_KEY_UNKNOWN,
    }
}

fn addon_metadata_bool(value: &[u8]) -> bool {
    matches!(value, b"True" | b"true" | b"1")
}

fn package_update_available(
    installed_present: bool,
    installed_version: &[u8],
    available_version: &[u8],
) -> bool {
    installed_present
        && !installed_version.is_empty()
        && !available_version.is_empty()
        && installed_version != available_version
}

fn package_state_satisfies_dependency(state: &[u8]) -> bool {
    !matches!(
        state,
        b"disabled" | b"pending_remove" | b"broken" | b"quarantined"
    )
}

fn package_state_keeps_installed_version(state: &[u8]) -> bool {
    state != b"pending_remove"
}

fn repository_max_release_sequence(sequences: &[u64]) -> u64 {
    sequences.iter().copied().max().unwrap_or(0)
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

fn package_transaction_id(sha256: &[u8]) -> Vec<u8> {
    let mut result = b"pkg-".to_vec();
    result.extend_from_slice(&sha256[..sha256.len().min(24)]);
    result
}

fn package_config_surface_kinds(
    package_type: u32,
    permissions: &[Fcitx5ControlUtf8],
    file_paths: &[Fcitx5ControlUtf8],
) -> Option<Vec<&'static str>> {
    let mut surfaces = std::collections::BTreeSet::new();
    match package_type {
        CONTROL_PACKAGE_TYPE_THEME => {
            surfaces.insert("theme");
        }
        CONTROL_PACKAGE_TYPE_INPUT_METHOD_DATA => {
            surfaces.insert("input-method-data");
        }
        CONTROL_PACKAGE_TYPE_ADDON => {
            surfaces.insert("fcitx-addon");
        }
        CONTROL_PACKAGE_TYPE_CORE | CONTROL_PACKAGE_TYPE_TRANSLATION => {}
        _ => return None,
    }
    for permission in permissions {
        if utf8_slice(*permission)? == b"input-data" {
            surfaces.insert("input-method-data");
        }
    }
    for path in file_paths {
        let path = utf8_slice(*path)?;
        if path.starts_with(b"share/fcitx5/addon/") && path.ends_with(b".conf") {
            surfaces.insert("fcitx-addon-config");
        }
        if path.starts_with(b"lib/fcitx5/") && path.ends_with(b".dll") {
            surfaces.insert("fcitx-addon");
        }
        if path.starts_with(b"share/rime-data/") {
            surfaces.insert("rime-data");
        }
        if path.starts_with(b"themes/") || path.starts_with(b"share/themes/") {
            surfaces.insert("theme");
        }
    }
    Some(surfaces.into_iter().collect())
}

fn package_config_surface_json(
    owner: Fcitx5ControlUtf8,
    package_type: u32,
    permissions: &[Fcitx5ControlUtf8],
    file_paths: &[Fcitx5ControlUtf8],
) -> Option<Vec<u8>> {
    let kinds = package_config_surface_kinds(package_type, permissions, file_paths)?;
    let kind_views = kinds
        .iter()
        .map(|kind| static_utf8_view(kind))
        .collect::<Vec<_>>();
    config_surfaces_json(owner, &kind_views)
}

fn classify_repository_error(error_code: &[u8], keyring: &std::path::Path) -> Vec<u8> {
    if error_code == b"invalid_file" && !keyring.exists() {
        b"missing_key".to_vec()
    } else {
        error_code.to_vec()
    }
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

fn bundled_package_descriptor(index: usize) -> Option<Fcitx5ControlBundledPackageDescriptor> {
    let package = BUNDLED_PACKAGES.get(index)?;
    Some(Fcitx5ControlBundledPackageDescriptor {
        id: static_utf8_view(package.id),
        title: static_utf8_view(package.title),
    })
}

fn bundled_package_present(install_root: &std::path::Path, id: &[u8]) -> bool {
    if install_root.as_os_str().is_empty() {
        return false;
    }
    let Ok(id) = std::str::from_utf8(id) else {
        return false;
    };
    let Some(package) = BUNDLED_PACKAGES.iter().find(|package| package.id == id) else {
        return false;
    };
    install_root.join(package.probe_relative_path).is_file()
}

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

fn config_action(command: &[u16], argc: usize) -> u32 {
    match argc {
        1 if ascii_utf16_eq(command, b"--reset-config") => CONTROL_CONFIG_ACTION_RESET_CONFIG,
        2 if ascii_utf16_eq(command, b"--validate-config") => CONTROL_CONFIG_ACTION_VALIDATE,
        2 if ascii_utf16_eq(command, b"--apply-config") => CONTROL_CONFIG_ACTION_APPLY,
        _ => CONTROL_CONFIG_ACTION_UNKNOWN,
    }
}

fn engine_management_action(command: &[u16], argc: usize) -> u32 {
    match argc {
        1 if ascii_utf16_eq(command, b"--get-input-methods") => {
            CONTROL_ENGINE_ACTION_GET_INPUT_METHODS
        }
        2 if ascii_utf16_eq(command, b"--set-input-method") => {
            CONTROL_ENGINE_ACTION_SET_INPUT_METHOD
        }
        _ => CONTROL_ENGINE_ACTION_UNKNOWN,
    }
}

fn root_action(command: &[u16], argc: usize, value: Option<&[u16]>) -> u32 {
    match argc {
        1 if ascii_utf16_eq(command, b"--version") => CONTROL_ROOT_ACTION_VERSION,
        1 if ascii_utf16_eq(command, b"--schema") => CONTROL_ROOT_ACTION_SCHEMA,
        1 if ascii_utf16_eq(command, b"--get-startup") => CONTROL_ROOT_ACTION_GET_STARTUP,
        1 if ascii_utf16_eq(command, b"--get-tsf-guard") => CONTROL_ROOT_ACTION_GET_TSF_GUARD,
        1 if ascii_utf16_eq(command, b"--reset-tsf-guard") => CONTROL_ROOT_ACTION_RESET_TSF_GUARD,
        1 if ascii_utf16_eq(command, b"--status") => CONTROL_ROOT_ACTION_STATUS,
        1 if ascii_utf16_eq(command, b"--diagnostics-plan") => CONTROL_ROOT_ACTION_DIAGNOSTICS_PLAN,
        1 if ascii_utf16_eq(command, b"--restart-engine") => CONTROL_ROOT_ACTION_RESTART_ENGINE,
        1 if ascii_utf16_eq(command, b"--shutdown") => CONTROL_ROOT_ACTION_SHUTDOWN,
        2 if ascii_utf16_eq(command, b"--set-startup")
            && value.is_some_and(|value| ascii_utf16_eq(value, b"enabled")) =>
        {
            CONTROL_ROOT_ACTION_SET_STARTUP_ENABLED
        }
        2 if ascii_utf16_eq(command, b"--set-startup")
            && value.is_some_and(|value| ascii_utf16_eq(value, b"disabled")) =>
        {
            CONTROL_ROOT_ACTION_SET_STARTUP_DISABLED
        }
        _ => CONTROL_ROOT_ACTION_UNKNOWN,
    }
}

fn ascii_utf16_eq(value: &[u16], ascii: &[u8]) -> bool {
    value.len() == ascii.len()
        && value
            .iter()
            .zip(ascii)
            .all(|(left, right)| *left == u16::from(*right))
}

fn package_action(command: &[u16], argc: usize, state: Option<&[u16]>) -> u32 {
    match argc {
        1 if ascii_utf16_eq(command, b"--packages-list") => CONTROL_PACKAGE_ACTION_PACKAGES_LIST,
        1 if ascii_utf16_eq(command, b"--themes-list") => CONTROL_PACKAGE_ACTION_THEMES_LIST,
        1 if ascii_utf16_eq(command, b"--addons-list") => CONTROL_PACKAGE_ACTION_ADDONS_LIST,
        1 if ascii_utf16_eq(command, b"--packages-repair") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_REPAIR
        }
        1 | 2 if ascii_utf16_eq(command, b"--packages-refresh") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_REFRESH
        }
        2 if ascii_utf16_eq(command, b"--themes-detail") => CONTROL_PACKAGE_ACTION_THEMES_DETAIL,
        2 if ascii_utf16_eq(command, b"--themes-export") => CONTROL_PACKAGE_ACTION_THEMES_EXPORT,
        2 if ascii_utf16_eq(command, b"--themes-import") => CONTROL_PACKAGE_ACTION_THEMES_IMPORT,
        2 if ascii_utf16_eq(command, b"--themes-delete") => CONTROL_PACKAGE_ACTION_THEMES_DELETE,
        2 if ascii_utf16_eq(command, b"--packages-detail") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_DETAIL
        }
        2 if ascii_utf16_eq(command, b"--packages-install") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_INSTALL
        }
        2 if ascii_utf16_eq(command, b"--packages-update") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_UPDATE
        }
        2 if ascii_utf16_eq(command, b"--packages-remove") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_REMOVE
        }
        3 if ascii_utf16_eq(command, b"--themes-duplicate") => {
            CONTROL_PACKAGE_ACTION_THEMES_DUPLICATE
        }
        3 if ascii_utf16_eq(command, b"--themes-export-to") => {
            CONTROL_PACKAGE_ACTION_THEMES_EXPORT_TO
        }
        3 if ascii_utf16_eq(command, b"--packages-state")
            && state.is_some_and(|value| {
                ascii_utf16_eq(value, b"enabled") || ascii_utf16_eq(value, b"disabled")
            }) =>
        {
            CONTROL_PACKAGE_ACTION_PACKAGES_STATE
        }
        _ => CONTROL_PACKAGE_ACTION_UNKNOWN,
    }
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
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_startup_query_utf16(
    executable_directory: Fcitx5ControlUtf16,
    registry_value: Fcitx5ControlUtf16,
    out_enabled: *mut u8,
) -> i32 {
    if out_enabled.is_null() {
        return 1;
    }
    let Some(executable_directory) = string_from_utf16(executable_directory) else {
        return 1;
    };
    let Some(registry_value) = string_from_utf16(registry_value) else {
        return 1;
    };
    match query_startup(executable_directory, registry_value) {
        Ok(enabled) => {
            unsafe {
                *out_enabled = u8::from(enabled);
            }
            0
        }
        Err(()) => 1,
    }
}

/// # Safety
///
/// UTF-16 slices must remain valid for the duration of the call. No pointer is
/// retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_startup_set_utf16(
    executable_directory: Fcitx5ControlUtf16,
    registry_value: Fcitx5ControlUtf16,
    enabled: u8,
) -> i32 {
    let Some(executable_directory) = string_from_utf16(executable_directory) else {
        return 1;
    };
    let Some(registry_value) = string_from_utf16(registry_value) else {
        return 1;
    };
    match set_startup(executable_directory, registry_value, enabled != 0) {
        Ok(()) => 0,
        Err(()) => 1,
    }
}

/// # Safety
///
/// `command_line` must remain valid UTF-16 for the duration of the call.
/// `command_out` and `locale_out` may be null for size queries or point to
/// writable UTF-16 storage for their respective capacities. No pointer is
/// retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_parse_config_command_line_utf16(
    command_line: Fcitx5ControlUtf16,
    command_out: *mut u16,
    command_capacity: usize,
    locale_out: *mut u16,
    locale_capacity: usize,
) -> Fcitx5ControlParsedConfigCommandLine {
    if command_line.ptr.is_null() && command_line.len != 0 {
        return Fcitx5ControlParsedConfigCommandLine::default();
    }
    let command_line = if command_line.ptr.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(command_line.ptr, command_line.len) }
    };
    let Some((command, locale)) = parse_config_command_line(command_line) else {
        return Fcitx5ControlParsedConfigCommandLine::default();
    };
    let command_len = write_wide_units(&command, command_out, command_capacity);
    let locale_len = write_wide_units(&locale, locale_out, locale_capacity);
    Fcitx5ControlParsedConfigCommandLine {
        status: 1,
        command_len,
        locale_len,
    }
}

/// # Safety
///
/// `override_locale` must remain valid UTF-16 for the duration of the call.
/// The returned pointer, when non-null, refers to static storage and must not
/// be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_config_locale_file_for_override_utf16(
    override_locale: Fcitx5ControlUtf16,
) -> Fcitx5ControlUtf16 {
    if override_locale.ptr.is_null() && override_locale.len != 0 {
        return Fcitx5ControlUtf16::default();
    }
    let override_locale = if override_locale.ptr.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(override_locale.ptr, override_locale.len) }
    };
    let Some(file_name) = config_locale_file_for_override(override_locale) else {
        return Fcitx5ControlUtf16::default();
    };
    Fcitx5ControlUtf16 {
        ptr: file_name.as_ptr(),
        len: file_name.len(),
    }
}

/// # Safety
///
/// `destination` must remain valid UTF-16 for the duration of the call.
/// `content` must remain readable for the duration of the call. No pointer is
/// retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_atomic_write_utf8_file_utf16(
    destination: Fcitx5ControlUtf16,
    content: Fcitx5ControlUtf8,
) -> i32 {
    let Some(destination) = string_from_utf16(destination) else {
        return 1;
    };
    let Some(content) = utf8_slice(content) else {
        return 1;
    };
    match atomic_write_utf8_file(PathBuf::from(destination), content) {
        Ok(()) => 0,
        Err(()) => 1,
    }
}

/// # Safety
///
/// `path` must remain valid UTF-16 for the duration of the call. `out_ptr` and
/// `out_len` must point to writable storage. On success, the returned buffer
/// must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_read_file_utf16(
    path: Fcitx5ControlUtf16,
    maximum: u64,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return CONTROL_FILE_READ_IO_ERROR;
    }
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    let Some(path) = string_from_utf16(path) else {
        return CONTROL_FILE_READ_INVALID_FILE;
    };
    match read_file_bounded(PathBuf::from(path), maximum) {
        Ok(bytes) => {
            let status = boxed_utf8_result(bytes, out_ptr, out_len);
            if status == 0 {
                CONTROL_FILE_READ_OK
            } else {
                status
            }
        }
        Err(status) => status,
    }
}

/// # Safety
///
/// `path` must remain valid UTF-16 for the duration of the call. `out_ptr` and
/// `out_len` must point to writable storage. On `CONTROL_FILE_READ_OK`, the
/// returned buffer must be freed with `fcitx5_control_utf8_free`. Missing files
/// return `CONTROL_FILE_READ_MISSING` and no buffer.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_read_optional_config_utf16(
    path: Fcitx5ControlUtf16,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return CONTROL_FILE_READ_IO_ERROR;
    }
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    let Some(path) = string_from_utf16(path) else {
        return CONTROL_FILE_READ_INVALID_FILE;
    };
    match read_optional_file_bounded(PathBuf::from(path), 256 * 1024) {
        Ok(Some(bytes)) => {
            let status = boxed_utf8_result(bytes, out_ptr, out_len);
            if status == 0 {
                CONTROL_FILE_READ_OK
            } else {
                status
            }
        }
        Ok(None) => CONTROL_FILE_READ_MISSING,
        Err(status) => status,
    }
}

/// # Safety
///
/// `package_root` must remain valid UTF-16 for the duration of the call. `id`
/// and `version` must remain valid UTF-8 for the duration of the call.
/// `out_ptr` and `out_len` must point to writable storage. On success, the
/// returned buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_installed_manifest_bytes_utf16(
    package_root: Fcitx5ControlUtf16,
    id: Fcitx5ControlUtf8,
    version: Fcitx5ControlUtf8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return CONTROL_FILE_READ_IO_ERROR;
    }
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    let Some(package_root) = string_from_utf16(package_root) else {
        return CONTROL_FILE_READ_INVALID_FILE;
    };
    let Some(id) = ascii_token_from_utf8(id) else {
        return CONTROL_FILE_READ_INVALID_FILE;
    };
    let Some(version) = ascii_token_from_utf8(version) else {
        return CONTROL_FILE_READ_INVALID_FILE;
    };
    match read_installed_manifest_bytes(&PathBuf::from(package_root), &id, &version) {
        Ok(bytes) => {
            let status = boxed_utf8_result(bytes, out_ptr, out_len);
            if status == 0 {
                CONTROL_FILE_READ_OK
            } else {
                status
            }
        }
        Err(status) => status,
    }
}

/// # Safety
///
/// `path` must remain valid UTF-16 for the duration of the call. `output` may
/// be null for size queries or writable UTF-16 storage for `capacity` code
/// units. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_repository_cache_incoming_path_utf16(
    path: Fcitx5ControlUtf16,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(path) = string_from_utf16(path) else {
        return 0;
    };
    let Some(incoming) = repository_cache_incoming_path(&PathBuf::from(path)) else {
        return 0;
    };
    write_wide_path(&incoming, output, capacity)
}

/// # Safety
///
/// `index` and `signature` must remain valid UTF-16 for the duration of the
/// call. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_repository_cache_prepare_utf16(
    index: Fcitx5ControlUtf16,
    signature: Fcitx5ControlUtf16,
) -> i32 {
    let Some(index) = string_from_utf16(index) else {
        return 1;
    };
    let Some(signature) = string_from_utf16(signature) else {
        return 1;
    };
    match prepare_repository_cache(&PathBuf::from(index), &PathBuf::from(signature)) {
        Ok(()) => 0,
        Err(()) => 1,
    }
}

/// # Safety
///
/// `index` and `signature` must remain valid UTF-16 for the duration of the
/// call. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_repository_cache_cleanup_utf16(
    index: Fcitx5ControlUtf16,
    signature: Fcitx5ControlUtf16,
) -> i32 {
    let Some(index) = string_from_utf16(index) else {
        return 1;
    };
    let Some(signature) = string_from_utf16(signature) else {
        return 1;
    };
    match remove_repository_incoming(&PathBuf::from(index), &PathBuf::from(signature)) {
        Ok(()) => 0,
        Err(()) => 1,
    }
}

/// # Safety
///
/// `index` and `signature` must remain valid UTF-16 for the duration of the
/// call. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_repository_cache_publish_utf16(
    index: Fcitx5ControlUtf16,
    signature: Fcitx5ControlUtf16,
) -> i32 {
    let Some(index) = string_from_utf16(index) else {
        return 1;
    };
    let Some(signature) = string_from_utf16(signature) else {
        return 1;
    };
    match publish_repository_cache(&PathBuf::from(index), &PathBuf::from(signature)) {
        Ok(()) => 0,
        Err(()) => 1,
    }
}

/// # Safety
///
/// `data_root` must remain valid UTF-16 for the duration of the call. `id` and
/// `version` must remain valid UTF-8 for the duration of the call. `output` may
/// be null for size queries or writable UTF-16 storage for `capacity` code
/// units. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_archive_cache_prepare_utf16(
    data_root: Fcitx5ControlUtf16,
    id: Fcitx5ControlUtf8,
    version: Fcitx5ControlUtf8,
    existing_hash_matches: u8,
    output: *mut u16,
    capacity: usize,
) -> Fcitx5ControlPathResult {
    let Some(data_root) = string_from_utf16(data_root) else {
        return Fcitx5ControlPathResult {
            status: CONTROL_ARCHIVE_CACHE_INVALID,
            path_len: 0,
        };
    };
    let Some(id) = ascii_token_from_utf8(id) else {
        return Fcitx5ControlPathResult {
            status: CONTROL_ARCHIVE_CACHE_INVALID,
            path_len: 0,
        };
    };
    let Some(version) = ascii_token_from_utf8(version) else {
        return Fcitx5ControlPathResult {
            status: CONTROL_ARCHIVE_CACHE_INVALID,
            path_len: 0,
        };
    };
    let (archive, stale_removed) = match prepare_package_archive_cache(
        &PathBuf::from(data_root),
        &id,
        &version,
        existing_hash_matches != 0,
    ) {
        Ok(value) => value,
        Err(status) => {
            return Fcitx5ControlPathResult {
                status,
                path_len: 0,
            };
        }
    };
    let path_len = write_wide_path(&archive, output, capacity);
    let status = if stale_removed {
        CONTROL_ARCHIVE_CACHE_STALE_REMOVED
    } else {
        0
    };
    Fcitx5ControlPathResult { status, path_len }
}

/// # Safety
///
/// `data_root` must remain valid UTF-16 for the duration of the call. `id` and
/// `version` must remain valid UTF-8 for the duration of the call. `output` may
/// be null for size queries or writable UTF-16 storage for `capacity` code
/// units. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_archive_cache_path_utf16(
    data_root: Fcitx5ControlUtf16,
    id: Fcitx5ControlUtf8,
    version: Fcitx5ControlUtf8,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(data_root) = string_from_utf16(data_root) else {
        return 0;
    };
    let Some(id) = ascii_token_from_utf8(id) else {
        return 0;
    };
    let Some(version) = ascii_token_from_utf8(version) else {
        return 0;
    };
    let Some(path) = package_archive_cache_path(&PathBuf::from(data_root), &id, &version) else {
        return 0;
    };
    write_wide_path(&path, output, capacity)
}

/// # Safety
///
/// `out_ptr` and `out_len` must point to writable storage. The returned pointer
/// is process-static UTF-8 data and must not be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_schema_json_utf8(
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    unsafe {
        *out_ptr = CONTROL_SCHEMA_JSON.as_ptr();
        *out_len = CONTROL_SCHEMA_JSON.len();
    }
    0
}

/// # Safety
///
/// `out_ptr` and `out_len` must point to writable storage. The returned pointer
/// is process-static UTF-8 data and must not be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_usage_text_utf8(
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    unsafe {
        *out_ptr = CONTROL_USAGE_TEXT.as_ptr();
        *out_len = CONTROL_USAGE_TEXT.len();
    }
    0
}

/// # Safety
///
/// `id` must remain valid for the duration of the call. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_input_method_id_valid_utf16(id: Fcitx5ControlUtf16) -> u8 {
    if id.ptr.is_null() {
        return 0;
    }
    let value = unsafe { std::slice::from_raw_parts(id.ptr, id.len) };
    if value.is_empty() || value.len() > 64 {
        return 0;
    }
    u8::from(value.iter().all(|character| {
        (*character >= b'a' as u16 && *character <= b'z' as u16)
            || (*character >= b'0' as u16 && *character <= b'9' as u16)
            || *character == b'-' as u16
            || *character == b'_' as u16
    }))
}

/// # Safety
///
/// `value` must remain valid for the duration of the call. `out_ptr` and
/// `out_len` must point to writable storage. Any returned buffer must be freed
/// with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_json_string_utf8(
    value: Fcitx5ControlUtf8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if value.ptr.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let bytes = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    match json_string(bytes) {
        Some(escaped) => boxed_utf8_result(escaped, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `status` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_status_json_utf8(
    status: *const Fcitx5ControlStatus,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if status.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let status = unsafe { &*status };
    match status_json(status) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `status` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_diagnostics_plan_json_utf8(
    status: *const Fcitx5ControlStatus,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if status.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let status = unsafe { &*status };
    match diagnostics_plan_json(status) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `status` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_tsf_guard_json_utf8(
    status: *const Fcitx5ControlTsfGuard,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if status.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let status = unsafe { &*status };
    match tsf_guard_json(status) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `out_ptr` and `out_len` must point to writable storage. The returned pointer
/// is process-static UTF-8 data and must not be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_tsf_guard_reset_json_utf8(
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    unsafe {
        *out_ptr = CONTROL_TSF_GUARD_RESET_JSON.as_ptr();
        *out_len = CONTROL_TSF_GUARD_RESET_JSON.len();
    }
    0
}

/// # Safety
///
/// `out_ptr` and `out_len` must point to writable storage. Any returned buffer
/// must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_startup_json_utf8(
    enabled: u8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    boxed_utf8_result(startup_json(enabled != 0), out_ptr, out_len)
}

/// # Safety
///
/// `out_ptr` and `out_len` must point to writable storage. The returned pointer
/// is process-static command data and must not be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_launcher_action_sequence(
    action: u32,
    out_ptr: *mut *const u32,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    let Some(commands) = launcher_action_sequence(action) else {
        return 1;
    };
    unsafe {
        *out_ptr = commands.as_ptr();
        *out_len = commands.len();
    }
    0
}

/// # Safety
///
/// All UTF-8 slices inside `repair` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_repair_json_utf8(
    repair: *const Fcitx5ControlPackageRepair,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if repair.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let repair = unsafe { &*repair };
    match package_repair_json(repair) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `command` and `value` must remain valid for the duration of the call when
/// their pointers are non-null. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_root_action_utf16(
    command: Fcitx5ControlUtf16,
    argc: usize,
    value: Fcitx5ControlUtf16,
) -> u32 {
    if command.ptr.is_null() {
        return CONTROL_ROOT_ACTION_UNKNOWN;
    }
    let command = unsafe { std::slice::from_raw_parts(command.ptr, command.len) };
    let value = if value.ptr.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(value.ptr, value.len) })
    };
    root_action(command, argc, value)
}

/// # Safety
///
/// `command` must remain valid for the duration of the call. No pointer is
/// retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_config_action_utf16(
    command: Fcitx5ControlUtf16,
    argc: usize,
) -> u32 {
    if command.ptr.is_null() {
        return CONTROL_CONFIG_ACTION_UNKNOWN;
    }
    let command = unsafe { std::slice::from_raw_parts(command.ptr, command.len) };
    config_action(command, argc)
}

/// # Safety
///
/// `command` must remain valid for the duration of the call. No pointer is
/// retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_engine_management_action_utf16(
    command: Fcitx5ControlUtf16,
    argc: usize,
) -> u32 {
    if command.ptr.is_null() {
        return CONTROL_ENGINE_ACTION_UNKNOWN;
    }
    let command = unsafe { std::slice::from_raw_parts(command.ptr, command.len) };
    engine_management_action(command, argc)
}

/// # Safety
///
/// `command` and `state` must remain valid for the duration of the call when
/// their pointers are non-null. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_action_utf16(
    command: Fcitx5ControlUtf16,
    argc: usize,
    state: Fcitx5ControlUtf16,
) -> u32 {
    if command.ptr.is_null() {
        return CONTROL_PACKAGE_ACTION_UNKNOWN;
    }
    let command = unsafe { std::slice::from_raw_parts(command.ptr, command.len) };
    let state = if state.ptr.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(state.ptr, state.len) })
    };
    package_action(command, argc, state)
}

/// # Safety
///
/// `addons` must either be null with `addon_count == 0` or point to
/// `addon_count` valid descriptors. All UTF-8 slices inside descriptors must
/// remain valid for the duration of the call. `out_ptr` and `out_len` must
/// point to writable storage. Any returned buffer must be freed with
/// `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_addons_json_utf8(
    addons: *const Fcitx5ControlAddonDescriptor,
    addon_count: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if addons.is_null() && addon_count != 0 {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let addons = if addon_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(addons, addon_count) }
    };
    match addons_json(addons) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `themes` must either be null with `theme_count == 0` or point to
/// `theme_count` valid records. All UTF-8 slices inside records must remain
/// valid for the duration of the call. `out_ptr` and `out_len` must point to
/// writable storage. Any returned buffer must be freed with
/// `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_themes_json_utf8(
    themes: *const Fcitx5ControlThemeRecord,
    theme_count: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if themes.is_null() && theme_count != 0 {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let themes = if theme_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(themes, theme_count) }
    };
    match themes_json(themes) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `detail` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_theme_detail_json_utf8(
    detail: *const Fcitx5ControlThemeDetail,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if detail.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let detail = unsafe { &*detail };
    match theme_detail_json(detail) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `list` and its rows must remain valid for the
/// duration of the call. `out_ptr` and `out_len` must point to writable storage.
/// Any returned buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_packages_list_json_utf8(
    list: *const Fcitx5ControlPackagesList,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if list.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let list = unsafe { &*list };
    match packages_list_json(list) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `dependencies` must either be null with `dependency_count == 0` or point to
/// `dependency_count` valid dependency records. All UTF-8 slices inside records
/// must remain valid for the duration of the call. `out_ptr` and `out_len` must
/// point to writable storage. Any returned buffer must be freed with
/// `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_dependencies_json_utf8(
    dependencies: *const Fcitx5ControlPackageDependency,
    dependency_count: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if dependencies.is_null() && dependency_count != 0 {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let dependencies = if dependency_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(dependencies, dependency_count) }
    };
    match package_dependencies_json(dependencies) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `values` must either be null with `value_count == 0` or point to
/// `value_count` valid UTF-8 slices. The slices must remain valid for the
/// duration of the call. `out_ptr` and `out_len` must point to writable storage.
/// Any returned buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_string_array_json_utf8(
    values: *const Fcitx5ControlUtf8,
    value_count: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if values.is_null() && value_count != 0 {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let values = if value_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(values, value_count) }
    };
    match string_array_json(values) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `kinds` must either be null with `kind_count == 0` or point to `kind_count`
/// valid UTF-8 slices. `owner` and all `kinds` slices must remain valid for the
/// duration of the call. `out_ptr` and `out_len` must point to writable storage.
/// Any returned buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_config_surfaces_json_utf8(
    owner: Fcitx5ControlUtf8,
    kinds: *const Fcitx5ControlUtf8,
    kind_count: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if kinds.is_null() && kind_count != 0 {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let kinds = if kind_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(kinds, kind_count) }
    };
    match config_surfaces_json(owner, kinds) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `permissions` and `file_paths` must either be null with a zero count or
/// point to the corresponding count of valid UTF-8 slices. `owner` and all
/// slices must remain valid for the duration of the call. `out_ptr` and
/// `out_len` must point to writable storage. Any returned buffer must be freed
/// with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_config_surface_json_utf8(
    owner: Fcitx5ControlUtf8,
    package_type: u32,
    permissions: *const Fcitx5ControlUtf8,
    permission_count: usize,
    file_paths: *const Fcitx5ControlUtf8,
    file_path_count: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if (permissions.is_null() && permission_count != 0)
        || (file_paths.is_null() && file_path_count != 0)
    {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let permissions = if permission_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(permissions, permission_count) }
    };
    let file_paths = if file_path_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(file_paths, file_path_count) }
    };
    match package_config_surface_json(owner, package_type, permissions, file_paths) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `error_code` must remain valid UTF-8 for the duration of the call.
/// `keyring` must remain valid UTF-16 for the duration of the call. `out_ptr`
/// and `out_len` must point to writable storage. Any returned buffer must be
/// freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_repository_error_utf8(
    error_code: Fcitx5ControlUtf8,
    keyring: Fcitx5ControlUtf16,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    let Some(error_code) = utf8_slice(error_code) else {
        return 1;
    };
    let Some(keyring) = string_from_utf16(keyring) else {
        return 1;
    };
    boxed_utf8_result(
        classify_repository_error(error_code, &PathBuf::from(keyring)),
        out_ptr,
        out_len,
    )
}

#[no_mangle]
pub extern "C" fn fcitx5_control_bundled_package_count() -> usize {
    BUNDLED_PACKAGES.len()
}

/// # Safety
///
/// `descriptor` must point to writable storage. Returned UTF-8 slices point to
/// static strings and do not need to be freed.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_bundled_package_descriptor(
    index: usize,
    descriptor: *mut Fcitx5ControlBundledPackageDescriptor,
) -> u8 {
    if descriptor.is_null() {
        return 0;
    }
    let Some(value) = bundled_package_descriptor(index) else {
        return 0;
    };
    unsafe {
        *descriptor = value;
    }
    1
}

/// # Safety
///
/// `install_root` must remain valid UTF-16 for the duration of the call, and
/// `id` must remain valid UTF-8 for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_bundled_package_present_utf16(
    install_root: Fcitx5ControlUtf16,
    id: Fcitx5ControlUtf8,
) -> u8 {
    let Some(install_root) = string_from_utf16(install_root) else {
        return 0;
    };
    let Some(id) = utf8_slice(id) else {
        return 0;
    };
    bundled_package_present(&PathBuf::from(install_root), id) as u8
}

#[no_mangle]
pub extern "C" fn fcitx5_control_package_type_name_utf8(package_type: u32) -> Fcitx5ControlUtf8 {
    package_type_name(package_type)
}

#[no_mangle]
pub extern "C" fn fcitx5_control_builtin_theme_id_utf8() -> Fcitx5ControlUtf8 {
    builtin_theme_id()
}

#[no_mangle]
pub extern "C" fn fcitx5_control_builtin_theme_source_utf8() -> Fcitx5ControlUtf8 {
    builtin_theme_source()
}

#[no_mangle]
pub extern "C" fn fcitx5_control_user_theme_source_utf8() -> Fcitx5ControlUtf8 {
    user_theme_source()
}

/// # Safety
///
/// `install_root` and `data_root` must remain valid UTF-16 for the duration of
/// the call.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_discover_themes_utf16(
    install_root: Fcitx5ControlUtf16,
    data_root: Fcitx5ControlUtf16,
) -> Fcitx5ControlThemeDiscoveryResult {
    let Some(install_root) = string_from_utf16(install_root) else {
        return Fcitx5ControlThemeDiscoveryResult {
            status: 1,
            entries: std::ptr::null_mut(),
            entry_count: 0,
        };
    };
    let Some(data_root) = string_from_utf16(data_root) else {
        return Fcitx5ControlThemeDiscoveryResult {
            status: 1,
            entries: std::ptr::null_mut(),
            entry_count: 0,
        };
    };
    let entries = discover_themes(&PathBuf::from(install_root), &PathBuf::from(data_root));
    let entries = entries
        .iter()
        .map(|entry| Fcitx5ControlThemeDiscoveryEntry {
            path: leak_utf16_path(&entry.path),
            id: leak_utf8_slice(&entry.id),
            source: leak_utf8_slice(entry.source),
        })
        .collect::<Vec<_>>();
    let mut entries = entries.into_boxed_slice();
    let result = Fcitx5ControlThemeDiscoveryResult {
        status: 0,
        entries: entries.as_mut_ptr(),
        entry_count: entries.len(),
    };
    std::mem::forget(entries);
    result
}

/// # Safety
///
/// `entries` must be a result previously returned by
/// `fcitx5_control_discover_themes_utf16`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_theme_discovery_free(
    entries: Fcitx5ControlThemeDiscoveryResult,
) {
    if entries.entries.is_null() {
        return;
    }
    let slice = std::ptr::slice_from_raw_parts_mut(entries.entries, entries.entry_count);
    let entries = unsafe { Box::from_raw(slice) };
    for entry in entries.iter() {
        unsafe {
            free_utf16_slice(entry.path);
            free_utf8_slice(entry.id);
            free_utf8_slice(entry.source);
        }
    }
}

/// # Safety
///
/// `text` must remain valid UTF-8 for the duration of the call when its pointer
/// is non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_parse_theme_summary_utf8(
    text: Fcitx5ControlUtf8,
) -> Fcitx5ControlThemeSummaryResult {
    let Some(text) = utf8_slice(text) else {
        return Fcitx5ControlThemeSummaryResult {
            status: 1,
            id: Fcitx5ControlUtf8::default(),
            name: Fcitx5ControlUtf8::default(),
            version: Fcitx5ControlUtf8::default(),
            license: Fcitx5ControlUtf8::default(),
            description: Fcitx5ControlUtf8::default(),
            has_light_branch: 0,
            has_dark_branch: 0,
        };
    };
    let Some(summary) = parse_theme_summary(text) else {
        return Fcitx5ControlThemeSummaryResult {
            status: 1,
            id: Fcitx5ControlUtf8::default(),
            name: Fcitx5ControlUtf8::default(),
            version: Fcitx5ControlUtf8::default(),
            license: Fcitx5ControlUtf8::default(),
            description: Fcitx5ControlUtf8::default(),
            has_light_branch: 0,
            has_dark_branch: 0,
        };
    };
    Fcitx5ControlThemeSummaryResult {
        status: 0,
        id: leak_utf8_slice(&summary.id),
        name: leak_utf8_slice(&summary.name),
        version: leak_utf8_slice(&summary.version),
        license: leak_utf8_slice(&summary.license),
        description: leak_utf8_slice(&summary.description),
        has_light_branch: summary.has_light_branch as u8,
        has_dark_branch: summary.has_dark_branch as u8,
    }
}

/// # Safety
///
/// `summary` must be a result previously returned by
/// `fcitx5_control_parse_theme_summary_utf8`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_theme_summary_free(
    summary: Fcitx5ControlThemeSummaryResult,
) {
    if summary.status != 0 {
        return;
    }
    unsafe {
        free_utf8_slice(summary.id);
        free_utf8_slice(summary.name);
        free_utf8_slice(summary.version);
        free_utf8_slice(summary.license);
        free_utf8_slice(summary.description);
    }
}

/// # Safety
///
/// `text` and `requested_id` must remain valid UTF-8 for the duration of the
/// call when their pointers are non-null. `out_ptr` and `out_len` must point to
/// writable storage. Any returned buffer must be freed with
/// `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_resolve_theme_config_utf8(
    text: Fcitx5ControlUtf8,
    requested_id: Fcitx5ControlUtf8,
    builtin: u8,
    dark: u8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    let Some(text) = utf8_slice(text) else {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    };
    let Some(requested_id) = utf8_slice(requested_id) else {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    };
    match resolved_theme_config(text, requested_id, builtin != 0, dark != 0) {
        Some(config) => boxed_utf8_result(config, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `install_root`, `data_root`, and `requested_id` must remain valid for the
/// duration of the call when their pointers are non-null. `output` must be
/// writable for `capacity` UTF-16 code units when non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_resolve_theme_path_utf16(
    install_root: Fcitx5ControlUtf16,
    data_root: Fcitx5ControlUtf16,
    requested_id: Fcitx5ControlUtf8,
    builtin: u8,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(install_root) = string_from_utf16(install_root) else {
        return 0;
    };
    let Some(data_root) = string_from_utf16(data_root) else {
        return 0;
    };
    let Some(requested_id) =
        utf8_slice(requested_id).and_then(|value| std::str::from_utf8(value).ok())
    else {
        return 0;
    };
    let Some(path) = resolve_theme_path(
        &PathBuf::from(install_root),
        &PathBuf::from(data_root),
        requested_id,
        builtin != 0,
    ) else {
        return 0;
    };
    write_wide_path(&path, output, capacity)
}

/// # Safety
///
/// `install_root`, `data_root`, and `requested_id` must remain valid for the
/// duration of the call when their pointers are non-null. `out_ptr` and
/// `out_len` must point to writable storage. Any returned buffer must be freed
/// with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_theme_export_utf8(
    install_root: Fcitx5ControlUtf16,
    data_root: Fcitx5ControlUtf16,
    requested_id: Fcitx5ControlUtf8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return CONTROL_THEME_OPERATION_IO_ERROR;
    }
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    let Some(install_root) = string_from_utf16(install_root) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    let Some(data_root) = string_from_utf16(data_root) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    let Some(requested_id) = utf8_slice(requested_id).and_then(|id| std::str::from_utf8(id).ok())
    else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    match export_theme(
        &PathBuf::from(install_root),
        &PathBuf::from(data_root),
        requested_id,
    ) {
        Ok(bytes) => boxed_utf8_result(bytes, out_ptr, out_len),
        Err(status) => status,
    }
}

/// # Safety
///
/// `data_root` and `source_path` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_theme_import_file_utf16(
    data_root: Fcitx5ControlUtf16,
    source_path: Fcitx5ControlUtf16,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return CONTROL_THEME_OPERATION_IO_ERROR;
    }
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    let Some(data_root) = string_from_utf16(data_root) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    let Some(source_path) = string_from_utf16(source_path) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    match import_theme(&PathBuf::from(data_root), &PathBuf::from(source_path)) {
        Ok(bytes) => boxed_utf8_result(bytes, out_ptr, out_len),
        Err(status) => status,
    }
}

/// # Safety
///
/// `install_root`, `data_root`, `requested_id`, and `destination_path` must
/// remain valid for the duration of the call. `out_ptr` and `out_len` must
/// point to writable storage. Any returned buffer must be freed with
/// `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_theme_export_file_utf16(
    install_root: Fcitx5ControlUtf16,
    data_root: Fcitx5ControlUtf16,
    requested_id: Fcitx5ControlUtf8,
    destination_path: Fcitx5ControlUtf16,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return CONTROL_THEME_OPERATION_IO_ERROR;
    }
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    let Some(install_root) = string_from_utf16(install_root) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    let Some(data_root) = string_from_utf16(data_root) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    let Some(requested_id) = utf8_slice(requested_id).and_then(|id| std::str::from_utf8(id).ok())
    else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    let Some(destination_path) = string_from_utf16(destination_path) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    match export_theme_to_file(
        &PathBuf::from(install_root),
        &PathBuf::from(data_root),
        requested_id,
        &PathBuf::from(destination_path),
    ) {
        Ok(bytes) => boxed_utf8_result(bytes, out_ptr, out_len),
        Err(status) => status,
    }
}

/// # Safety
///
/// `install_root`, `data_root`, `source_id`, and `new_id` must remain valid for
/// the duration of the call. `out_ptr` and `out_len` must point to writable
/// storage. Any returned buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_theme_duplicate_utf8(
    install_root: Fcitx5ControlUtf16,
    data_root: Fcitx5ControlUtf16,
    source_id: Fcitx5ControlUtf8,
    new_id: Fcitx5ControlUtf8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return CONTROL_THEME_OPERATION_IO_ERROR;
    }
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    let Some(install_root) = string_from_utf16(install_root) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    let Some(data_root) = string_from_utf16(data_root) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    let Some(source_id) = utf8_slice(source_id).and_then(|id| std::str::from_utf8(id).ok()) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    let Some(new_id) = utf8_slice(new_id).and_then(|id| std::str::from_utf8(id).ok()) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    match duplicate_theme(
        &PathBuf::from(install_root),
        &PathBuf::from(data_root),
        source_id,
        new_id,
    ) {
        Ok(bytes) => boxed_utf8_result(bytes, out_ptr, out_len),
        Err(status) => status,
    }
}

/// # Safety
///
/// `data_root` and `requested_id` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_theme_delete_utf8(
    data_root: Fcitx5ControlUtf16,
    requested_id: Fcitx5ControlUtf8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return CONTROL_THEME_OPERATION_IO_ERROR;
    }
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    let Some(data_root) = string_from_utf16(data_root) else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    let Some(requested_id) = utf8_slice(requested_id).and_then(|id| std::str::from_utf8(id).ok())
    else {
        return CONTROL_THEME_OPERATION_INVALID;
    };
    match delete_theme(&PathBuf::from(data_root), requested_id) {
        Ok(bytes) => boxed_utf8_result(bytes, out_ptr, out_len),
        Err(status) => status,
    }
}

/// # Safety
///
/// `source`, `requested_id`, and `theme_id` must remain valid UTF-8 for the
/// duration of the call when their pointers are non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_theme_record_matches_requested_id_utf8(
    source: Fcitx5ControlUtf8,
    requested_id: Fcitx5ControlUtf8,
    theme_id: Fcitx5ControlUtf8,
) -> u8 {
    let source = utf8_slice(source).unwrap_or(&[]);
    let requested_id = utf8_slice(requested_id).unwrap_or(&[]);
    let theme_id = utf8_slice(theme_id).unwrap_or(&[]);
    theme_record_matches_requested_id(source, requested_id, theme_id) as u8
}

#[no_mangle]
pub extern "C" fn fcitx5_control_native_package_architecture_utf8() -> Fcitx5ControlUtf8 {
    native_package_architecture()
}

/// # Safety
///
/// `architecture` must remain valid UTF-8 for the duration of the call when its
/// pointer is non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_architecture_matches_native_utf8(
    architecture: Fcitx5ControlUtf8,
) -> u8 {
    let Some(architecture) = utf8_slice(architecture) else {
        return 0;
    };
    package_architecture_matches_native(architecture) as u8
}

/// # Safety
///
/// `section` must remain valid UTF-8 for the duration of the call when its
/// pointer is non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_addon_metadata_section_is_addon_utf8(
    section: Fcitx5ControlUtf8,
) -> u8 {
    addon_metadata_section_is_addon(utf8_slice(section).unwrap_or(&[])) as u8
}

/// # Safety
///
/// `key` must remain valid UTF-8 for the duration of the call when its pointer
/// is non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_addon_metadata_key_utf8(key: Fcitx5ControlUtf8) -> u32 {
    addon_metadata_key(utf8_slice(key).unwrap_or(&[]))
}

/// # Safety
///
/// `value` must remain valid UTF-8 for the duration of the call when its
/// pointer is non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_addon_metadata_bool_utf8(value: Fcitx5ControlUtf8) -> u8 {
    addon_metadata_bool(utf8_slice(value).unwrap_or(&[])) as u8
}

/// # Safety
///
/// `installed_version` and `available_version` must remain valid UTF-8 for the
/// duration of the call when their pointers are non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_update_available_utf8(
    installed_present: u8,
    installed_version: Fcitx5ControlUtf8,
    available_version: Fcitx5ControlUtf8,
) -> u8 {
    let installed_version = utf8_slice(installed_version).unwrap_or(&[]);
    let available_version = utf8_slice(available_version).unwrap_or(&[]);
    package_update_available(installed_present != 0, installed_version, available_version) as u8
}

/// # Safety
///
/// `state` must remain valid UTF-8 for the duration of the call when its
/// pointer is non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_state_satisfies_dependency_utf8(
    state: Fcitx5ControlUtf8,
) -> u8 {
    package_state_satisfies_dependency(utf8_slice(state).unwrap_or(&[])) as u8
}

/// # Safety
///
/// `state` must remain valid UTF-8 for the duration of the call when its
/// pointer is non-null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_state_keeps_installed_version_utf8(
    state: Fcitx5ControlUtf8,
) -> u8 {
    package_state_keeps_installed_version(utf8_slice(state).unwrap_or(&[])) as u8
}

/// # Safety
///
/// `sequences` must point to `sequence_count` readable `u64` values, or be
/// null only when `sequence_count` is zero.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_repository_max_release_sequence(
    sequences: *const u64,
    sequence_count: usize,
) -> u64 {
    if sequence_count == 0 {
        return 0;
    }
    if sequences.is_null() {
        return 0;
    }
    let sequences = unsafe { std::slice::from_raw_parts(sequences, sequence_count) };
    repository_max_release_sequence(sequences)
}

/// # Safety
///
/// `base_url` must remain valid UTF-16 for the duration of the call.
/// `metadata_name` must remain valid ASCII for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_repository_metadata_url_utf16(
    base_url: Fcitx5ControlUtf16,
    metadata_name: Fcitx5ControlUtf8,
    output: *mut u16,
    capacity: usize,
) -> usize {
    if base_url.ptr.is_null() {
        return 0;
    }
    let base_url = unsafe { std::slice::from_raw_parts(base_url.ptr, base_url.len) };
    let Some(metadata_name) = utf8_slice(metadata_name) else {
        return 0;
    };
    let Some(url) = repository_metadata_url(base_url, metadata_name) else {
        return 0;
    };
    write_wide_units(&url, output, capacity)
}

/// # Safety
///
/// `channel` must remain valid ASCII for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_repository_default_base_url_utf16(
    channel: Fcitx5ControlUtf8,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(channel) = utf8_slice(channel) else {
        return 0;
    };
    let Some(url) = repository_default_base_url(channel) else {
        return 0;
    };
    write_wide_units(&url, output, capacity)
}

/// # Safety
///
/// `sha256` must remain valid UTF-8 for the duration of the call. `out_ptr`
/// and `out_len` must point to writable storage. Any returned buffer must be
/// freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_transaction_id_utf8(
    sha256: Fcitx5ControlUtf8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    let Some(sha256) = utf8_slice(sha256) else {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    };
    boxed_utf8_result(package_transaction_id(sha256), out_ptr, out_len)
}

/// # Safety
///
/// All UTF-8 slices inside `detail` must remain valid for the duration of the
/// call. Raw JSON fields must contain valid JSON fragments produced by the
/// existing package/config-surface serializers. `out_ptr` and `out_len` must
/// point to writable storage. Any returned buffer must be freed with
/// `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_detail_json_utf8(
    detail: *const Fcitx5ControlPackageDetail,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if detail.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let detail = unsafe { &*detail };
    match package_detail_json(detail) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `ptr` and `len` must be the exact buffer returned by a Control core UTF-8
/// allocation function, or `ptr` must be null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_utf8_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

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
    fn config_command_line_parser_matches_cpp_contract() {
        let (command, locale) =
            parse_config_command_line(&wide(r#"--lang=zh-CN "--ui-contract-test""#))
                .expect("valid command line");
        assert_eq!(command, wide("--ui-contract-test"));
        assert_eq!(locale, wide("zh-CN"));

        let (command, locale) =
            parse_config_command_line(&wide(r#""--ui-live-preview-contract-test""#))
                .expect("quoted command");
        assert_eq!(command, wide("--ui-live-preview-contract-test"));
        assert!(locale.is_empty());

        assert!(parse_config_command_line(&wide("--self-test --check-i18n")).is_none());
        assert!(parse_config_command_line(&wide("--lang=system --lang=zh-CN")).is_none());

        let (command, locale) =
            parse_config_command_line(&wide("--lang= --lang=en-US --self-test"))
                .expect("empty locale preserves legacy replacement behavior");
        assert_eq!(command, wide("--self-test"));
        assert_eq!(locale, wide("en-US"));
    }

    #[test]
    fn config_locale_file_selection_matches_cpp_contract() {
        assert_eq!(
            config_locale_file_for_override(&wide("zh-CN")).expect("zh locale"),
            CONFIG_LOCALE_ZH_CN_FILE
        );
        assert_eq!(
            config_locale_file_for_override(&wide("en-US")).expect("en locale"),
            CONFIG_LOCALE_EN_US_FILE
        );
        assert!(config_locale_file_for_override(&wide("fr-FR")).is_none());
        assert!(matches!(
            config_locale_file_for_override(&wide("system")),
            Some(CONFIG_LOCALE_EN_US_FILE) | Some(CONFIG_LOCALE_ZH_CN_FILE)
        ));
    }

    #[test]
    fn config_locale_file_abi_returns_static_utf16_file_name() {
        let zh = wide("zh-CN");
        let selected = unsafe {
            fcitx5_control_config_locale_file_for_override_utf16(Fcitx5ControlUtf16 {
                ptr: zh.as_ptr(),
                len: zh.len(),
            })
        };
        assert_eq!(
            unsafe { std::slice::from_raw_parts(selected.ptr, selected.len) },
            CONFIG_LOCALE_ZH_CN_FILE
        );

        let invalid = wide("fr-FR");
        let missing = unsafe {
            fcitx5_control_config_locale_file_for_override_utf16(Fcitx5ControlUtf16 {
                ptr: invalid.as_ptr(),
                len: invalid.len(),
            })
        };
        assert!(missing.ptr.is_null());
        assert_eq!(missing.len, 0);
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

        let path = destination.as_os_str().encode_wide().collect::<Vec<_>>();
        let replacement = b"second = true\n";
        let status = unsafe {
            fcitx5_control_atomic_write_utf8_file_utf16(
                Fcitx5ControlUtf16 {
                    ptr: path.as_ptr(),
                    len: path.len(),
                },
                Fcitx5ControlUtf8 {
                    ptr: replacement.as_ptr(),
                    len: replacement.len(),
                },
            )
        };
        assert_eq!(status, 0);
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
        let null_status = unsafe {
            fcitx5_control_atomic_write_utf8_file_utf16(
                Fcitx5ControlUtf16 {
                    ptr: std::ptr::null(),
                    len: 0,
                },
                Fcitx5ControlUtf8 {
                    ptr: replacement.as_ptr(),
                    len: replacement.len(),
                },
            )
        };
        assert_eq!(null_status, 1);
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
        let wide_path = empty_path.as_os_str().encode_wide().collect::<Vec<_>>();
        let mut bytes: *mut u8 = std::ptr::null_mut();
        let mut len = usize::MAX;
        let status = unsafe {
            fcitx5_control_read_file_utf16(
                Fcitx5ControlUtf16 {
                    ptr: wide_path.as_ptr(),
                    len: wide_path.len(),
                },
                256 * 1024,
                &mut bytes,
                &mut len,
            )
        };
        assert_eq!(status, CONTROL_FILE_READ_OK);
        assert_eq!(len, 0);
        unsafe {
            fcitx5_control_utf8_free(bytes, len);
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn optional_config_read_matches_cpp_contract() {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-control-core-optional-config-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test root");
        let config = root.join("config.toml");

        assert_eq!(
            read_optional_file_bounded(config.clone(), 256 * 1024).expect("missing is ok"),
            None
        );
        std::fs::write(&config, b"format_version = 1\n").expect("write config");
        assert_eq!(
            read_optional_file_bounded(config.clone(), 256 * 1024).expect("read optional config"),
            Some(b"format_version = 1\n".to_vec())
        );
        assert_eq!(
            read_optional_file_bounded(config.clone(), 4).expect_err("too large should fail"),
            CONTROL_FILE_READ_INVALID_FILE
        );

        let wide_path = config.as_os_str().encode_wide().collect::<Vec<_>>();
        let mut bytes: *mut u8 = std::ptr::null_mut();
        let mut len = 0;
        let status = unsafe {
            fcitx5_control_read_optional_config_utf16(
                Fcitx5ControlUtf16 {
                    ptr: wide_path.as_ptr(),
                    len: wide_path.len(),
                },
                &mut bytes,
                &mut len,
            )
        };
        assert_eq!(status, CONTROL_FILE_READ_OK);
        assert_eq!(len, b"format_version = 1\n".len());
        unsafe {
            fcitx5_control_utf8_free(bytes, len);
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn installed_manifest_bytes_match_cpp_contract() {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-control-core-installed-manifest-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let manifest_dir = root.join("manifests").join("fcitx5-rime");
        std::fs::create_dir_all(&manifest_dir).expect("create manifest dir");
        std::fs::write(manifest_dir.join("1.0.0.json"), br#"{"id":"fcitx5-rime"}"#)
            .expect("write manifest");

        assert_eq!(
            read_installed_manifest_bytes(&root, "fcitx5-rime", "1.0.0")
                .expect("read installed manifest"),
            br#"{"id":"fcitx5-rime"}"#
        );
        assert_eq!(
            read_installed_manifest_bytes(&root, "fcitx5-rime", "missing")
                .expect_err("missing manifest should fail"),
            CONTROL_FILE_READ_INVALID_FILE
        );
        assert!(installed_manifest_path(&root, "fcitx5-rime", "1.0.0")
            .expect("manifest path")
            .ends_with("manifests\\fcitx5-rime\\1.0.0.json"));

        let invalid = Fcitx5ControlUtf8 {
            ptr: b"..\\bad".as_ptr(),
            len: 6,
        };
        assert!(ascii_token_from_utf8(invalid).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn repository_cache_staging_and_publish_match_cpp_contract() {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-control-core-repository-cache-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let index = root.join("repository").join("index.json");
        let signature = root.join("repository").join("index.sig.json");
        let incoming_index = repository_cache_incoming_path(&index).expect("incoming index path");
        let incoming_signature =
            repository_cache_incoming_path(&signature).expect("incoming signature path");
        assert_eq!(
            incoming_index.file_name().and_then(|name| name.to_str()),
            Some("index.json.new")
        );
        assert_eq!(
            incoming_signature
                .file_name()
                .and_then(|name| name.to_str()),
            Some("index.sig.json.new")
        );

        std::fs::create_dir_all(index.parent().expect("repository parent"))
            .expect("create repository parent");
        std::fs::write(&incoming_index, b"stale-index").expect("write stale index");
        std::fs::write(&incoming_signature, b"stale-signature").expect("write stale signature");
        prepare_repository_cache(&index, &signature).expect("prepare repository cache");
        assert!(!incoming_index.exists());
        assert!(!incoming_signature.exists());

        std::fs::write(&index, b"old-index").expect("write old index");
        std::fs::write(&signature, b"old-signature").expect("write old signature");
        std::fs::write(&incoming_index, b"new-index").expect("write incoming index");
        std::fs::write(&incoming_signature, b"new-signature").expect("write incoming signature");
        publish_repository_cache(&index, &signature).expect("publish repository cache");
        assert_eq!(
            std::fs::read(&index).expect("read published index"),
            b"new-index"
        );
        assert_eq!(
            std::fs::read(&signature).expect("read published signature"),
            b"new-signature"
        );
        assert!(!incoming_index.exists());
        assert!(!incoming_signature.exists());

        std::fs::write(&incoming_index, b"cleanup-index").expect("write cleanup index");
        std::fs::write(&incoming_signature, b"cleanup-signature").expect("write cleanup signature");
        remove_repository_incoming(&index, &signature).expect("cleanup repository cache");
        assert!(!incoming_index.exists());
        assert!(!incoming_signature.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn package_archive_cache_prepare_matches_cpp_contract() {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-control-core-package-archive-cache-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let (archive, removed) =
            prepare_package_archive_cache(&root, "fcitx5-rime", "1.0.0", false)
                .expect("prepare fresh cache");
        assert!(!removed);
        assert_eq!(
            archive,
            root.join("downloads").join("fcitx5-rime-1.0.0.fcpkg")
        );
        assert!(archive.parent().expect("downloads parent").is_dir());

        std::fs::write(&archive, b"cached").expect("write cached archive");
        let (same_archive, removed) =
            prepare_package_archive_cache(&root, "fcitx5-rime", "1.0.0", true)
                .expect("prepare matching cache");
        assert_eq!(same_archive, archive);
        assert!(!removed);
        assert_eq!(
            std::fs::read(&archive).expect("cache still exists"),
            b"cached"
        );

        let (stale_archive, removed) =
            prepare_package_archive_cache(&root, "fcitx5-rime", "1.0.0", false)
                .expect("prepare stale cache");
        assert_eq!(stale_archive, archive);
        assert!(removed);
        assert!(!archive.exists());

        let invalid = Fcitx5ControlUtf8 {
            ptr: b"..\\bad".as_ptr(),
            len: 6,
        };
        assert!(ascii_token_from_utf8(invalid).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn root_actions_are_typed_control_commands() {
        let version = wide("--version");
        let schema = wide("--schema");
        let get_startup = wide("--get-startup");
        let set_startup = wide("--set-startup");
        let enabled = wide("enabled");
        let disabled = wide("disabled");
        let broken = wide("broken");
        let get_tsf_guard = wide("--get-tsf-guard");
        let reset_tsf_guard = wide("--reset-tsf-guard");
        let status = wide("--status");
        let diagnostics_plan = wide("--diagnostics-plan");
        let restart_engine = wide("--restart-engine");
        let shutdown = wide("--shutdown");
        assert_eq!(root_action(&version, 1, None), CONTROL_ROOT_ACTION_VERSION);
        assert_eq!(root_action(&schema, 1, None), CONTROL_ROOT_ACTION_SCHEMA);
        assert_eq!(
            root_action(&get_startup, 1, None),
            CONTROL_ROOT_ACTION_GET_STARTUP
        );
        assert_eq!(
            root_action(&set_startup, 2, Some(&enabled)),
            CONTROL_ROOT_ACTION_SET_STARTUP_ENABLED
        );
        assert_eq!(
            root_action(&set_startup, 2, Some(&disabled)),
            CONTROL_ROOT_ACTION_SET_STARTUP_DISABLED
        );
        assert_eq!(
            root_action(&set_startup, 2, Some(&broken)),
            CONTROL_ROOT_ACTION_UNKNOWN
        );
        assert_eq!(
            root_action(&get_tsf_guard, 1, None),
            CONTROL_ROOT_ACTION_GET_TSF_GUARD
        );
        assert_eq!(
            root_action(&reset_tsf_guard, 1, None),
            CONTROL_ROOT_ACTION_RESET_TSF_GUARD
        );
        assert_eq!(root_action(&status, 1, None), CONTROL_ROOT_ACTION_STATUS);
        assert_eq!(
            root_action(&diagnostics_plan, 1, None),
            CONTROL_ROOT_ACTION_DIAGNOSTICS_PLAN
        );
        assert_eq!(
            root_action(&restart_engine, 1, None),
            CONTROL_ROOT_ACTION_RESTART_ENGINE
        );
        assert_eq!(
            root_action(&shutdown, 1, None),
            CONTROL_ROOT_ACTION_SHUTDOWN
        );
        assert_eq!(root_action(&shutdown, 2, None), CONTROL_ROOT_ACTION_UNKNOWN);
    }

    #[test]
    fn launcher_action_sequences_are_typed_control_commands() {
        assert_eq!(
            launcher_action_sequence(CONTROL_LAUNCHER_ACTION_RESTART_ENGINE),
            Some(
                &[
                    LAUNCHER_COMMAND_USER_STOP,
                    LAUNCHER_COMMAND_RESUME,
                    LAUNCHER_COMMAND_START_DEMAND
                ][..]
            )
        );
        assert_eq!(
            launcher_action_sequence(CONTROL_LAUNCHER_ACTION_SHUTDOWN),
            Some(&[LAUNCHER_COMMAND_SHUTDOWN][..])
        );
        assert_eq!(launcher_action_sequence(99), None);
    }

    #[test]
    fn package_repair_json_preserves_control_contract() {
        let state = b"repaired";
        let repair = Fcitx5ControlPackageRepair {
            repository_sequence_state: Fcitx5ControlUtf8 {
                ptr: state.as_ptr(),
                len: state.len(),
            },
        };
        let json = package_repair_json(&repair).expect("package repair should format");
        assert_eq!(
            json.as_slice(),
            br#"{"format_version":1,"repair":"verified","repository_sequence_state":"repaired"}"#
        );
    }

    #[test]
    fn config_actions_are_typed_control_commands() {
        let validate = wide("--validate-config");
        let apply = wide("--apply-config");
        let reset = wide("--reset-config");
        let reset_presentation = wide("--reset-presentation");
        let get_presentation = wide("--get-presentation");
        let set_presentation = wide("--set-presentation");
        assert_eq!(config_action(&validate, 2), CONTROL_CONFIG_ACTION_VALIDATE);
        assert_eq!(config_action(&apply, 2), CONTROL_CONFIG_ACTION_APPLY);
        assert_eq!(config_action(&reset, 1), CONTROL_CONFIG_ACTION_RESET_CONFIG);
        assert_eq!(
            config_action(&reset_presentation, 1),
            CONTROL_CONFIG_ACTION_UNKNOWN
        );
        assert_eq!(
            config_action(&get_presentation, 1),
            CONTROL_CONFIG_ACTION_UNKNOWN
        );
        assert_eq!(
            config_action(&set_presentation, 7),
            CONTROL_CONFIG_ACTION_UNKNOWN
        );
        assert_eq!(config_action(&validate, 1), CONTROL_CONFIG_ACTION_UNKNOWN);
    }

    #[test]
    fn engine_management_actions_are_typed_control_commands() {
        let get = wide("--get-input-methods");
        let set = wide("--set-input-method");
        assert_eq!(
            engine_management_action(&get, 1),
            CONTROL_ENGINE_ACTION_GET_INPUT_METHODS
        );
        assert_eq!(
            engine_management_action(&set, 2),
            CONTROL_ENGINE_ACTION_SET_INPUT_METHOD
        );
        assert_eq!(
            engine_management_action(&set, 1),
            CONTROL_ENGINE_ACTION_UNKNOWN
        );
        assert_eq!(
            engine_management_action(&get, 2),
            CONTROL_ENGINE_ACTION_UNKNOWN
        );
    }

    #[test]
    fn package_actions_are_typed_control_commands() {
        let packages_list = wide("--packages-list");
        let themes_detail = wide("--themes-detail");
        let addons_list = wide("--addons-list");
        let packages_refresh = wide("--packages-refresh");
        let packages_install = wide("--packages-install");
        let packages_update = wide("--packages-update");
        let packages_state = wide("--packages-state");
        let packages_remove = wide("--packages-remove");
        let packages_repair = wide("--packages-repair");
        let enabled = wide("enabled");
        let disabled = wide("disabled");
        let broken = wide("broken");
        assert_eq!(
            package_action(&packages_list, 1, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_LIST
        );
        assert_eq!(
            package_action(&themes_detail, 2, None),
            CONTROL_PACKAGE_ACTION_THEMES_DETAIL
        );
        assert_eq!(
            package_action(&addons_list, 1, None),
            CONTROL_PACKAGE_ACTION_ADDONS_LIST
        );
        assert_eq!(
            package_action(&packages_refresh, 1, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_REFRESH
        );
        assert_eq!(
            package_action(&packages_refresh, 2, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_REFRESH
        );
        assert_eq!(
            package_action(&packages_install, 2, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_INSTALL
        );
        assert_eq!(
            package_action(&packages_update, 2, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_UPDATE
        );
        assert_eq!(
            package_action(&packages_state, 3, Some(&enabled)),
            CONTROL_PACKAGE_ACTION_PACKAGES_STATE
        );
        assert_eq!(
            package_action(&packages_state, 3, Some(&disabled)),
            CONTROL_PACKAGE_ACTION_PACKAGES_STATE
        );
        assert_eq!(
            package_action(&packages_state, 3, Some(&broken)),
            CONTROL_PACKAGE_ACTION_UNKNOWN
        );
        assert_eq!(
            package_action(&packages_remove, 2, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_REMOVE
        );
        assert_eq!(
            package_action(&packages_repair, 1, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_REPAIR
        );
        assert_eq!(
            package_action(&packages_list, 2, None),
            CONTROL_PACKAGE_ACTION_UNKNOWN
        );
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
                id: Fcitx5ControlUtf8 {
                    ptr: id.as_ptr(),
                    len: id.len(),
                },
                name: Fcitx5ControlUtf8 {
                    ptr: name.as_ptr(),
                    len: name.len(),
                },
                category: Fcitx5ControlUtf8 {
                    ptr: category.as_ptr(),
                    len: category.len(),
                },
                library: Fcitx5ControlUtf8 {
                    ptr: library.as_ptr(),
                    len: library.len(),
                },
                addon_type: Fcitx5ControlUtf8 {
                    ptr: addon_type.as_ptr(),
                    len: addon_type.len(),
                },
                version: Fcitx5ControlUtf8 {
                    ptr: version.as_ptr(),
                    len: version.len(),
                },
                configurable: 1,
                on_demand: 0,
                library_present: 1,
            },
            Fcitx5ControlAddonDescriptor {
                id: Fcitx5ControlUtf8 {
                    ptr: b"test".as_ptr(),
                    len: 4,
                },
                name: Fcitx5ControlUtf8 {
                    ptr: b"Test".as_ptr(),
                    len: 4,
                },
                category: Fcitx5ControlUtf8 {
                    ptr: b"Module".as_ptr(),
                    len: 6,
                },
                library: Fcitx5ControlUtf8 {
                    ptr: b"".as_ptr(),
                    len: 0,
                },
                addon_type: Fcitx5ControlUtf8 {
                    ptr: b"Static".as_ptr(),
                    len: 6,
                },
                version: Fcitx5ControlUtf8 {
                    ptr: empty_version.as_ptr(),
                    len: empty_version.len(),
                },
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

    fn test_theme_record() -> Fcitx5ControlThemeRecord {
        Fcitx5ControlThemeRecord {
            id: Fcitx5ControlUtf8 {
                ptr: b"builtin:default".as_ptr(),
                len: 15,
            },
            source: Fcitx5ControlUtf8 {
                ptr: b"builtin".as_ptr(),
                len: 7,
            },
            name: Fcitx5ControlUtf8 {
                ptr: "默认".as_bytes().as_ptr(),
                len: "默认".len(),
            },
            version: Fcitx5ControlUtf8 {
                ptr: b"1".as_ptr(),
                len: 1,
            },
            license: Fcitx5ControlUtf8 {
                ptr: b"MIT".as_ptr(),
                len: 3,
            },
            description: Fcitx5ControlUtf8 {
                ptr: b"default theme".as_ptr(),
                len: 13,
            },
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
            id: Fcitx5ControlUtf8 {
                ptr: id.as_ptr(),
                len: id.len(),
            },
            title: Fcitx5ControlUtf8 {
                ptr: title.as_ptr(),
                len: title.len(),
            },
            summary: Fcitx5ControlUtf8 {
                ptr: summary.as_ptr(),
                len: summary.len(),
            },
            package_type: Fcitx5ControlUtf8 {
                ptr: package_type.as_ptr(),
                len: package_type.len(),
            },
            available_version: Fcitx5ControlUtf8 {
                ptr: available.as_ptr(),
                len: available.len(),
            },
            installed_version: Fcitx5ControlUtf8 {
                ptr: installed.as_ptr(),
                len: installed.len(),
            },
            state: Fcitx5ControlUtf8 {
                ptr: state.as_ptr(),
                len: state.len(),
            },
            update_available: 1,
        };
        let list = Fcitx5ControlPackagesList {
            repository_available: 1,
            repository_error: Fcitx5ControlUtf8 {
                ptr: b"".as_ptr(),
                len: 0,
            },
            packages: &package,
            package_count: 1,
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
            id: Fcitx5ControlUtf8 {
                ptr: b"orphan".as_ptr(),
                len: 6,
            },
            title: Fcitx5ControlUtf8 {
                ptr: b"orphan".as_ptr(),
                len: 6,
            },
            summary: Fcitx5ControlUtf8 {
                ptr: b"".as_ptr(),
                len: 0,
            },
            package_type: Fcitx5ControlUtf8 {
                ptr: b"unknown".as_ptr(),
                len: 7,
            },
            available_version: Fcitx5ControlUtf8 {
                ptr: b"".as_ptr(),
                len: 0,
            },
            installed_version: Fcitx5ControlUtf8 {
                ptr: b"1".as_ptr(),
                len: 1,
            },
            state: Fcitx5ControlUtf8 {
                ptr: b"".as_ptr(),
                len: 0,
            },
            update_available: 0,
        };
        let error = b"missing_key";
        let list = Fcitx5ControlPackagesList {
            repository_available: 0,
            repository_error: Fcitx5ControlUtf8 {
                ptr: error.as_ptr(),
                len: error.len(),
            },
            packages: &package,
            package_count: 1,
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
                id: Fcitx5ControlUtf8 {
                    ptr: b"dep-one".as_ptr(),
                    len: 7,
                },
                version: Fcitx5ControlUtf8 {
                    ptr: b"1.0".as_ptr(),
                    len: 3,
                },
            },
            Fcitx5ControlPackageDependency {
                id: Fcitx5ControlUtf8 {
                    ptr: b"dep\"two".as_ptr(),
                    len: 7,
                },
                version: Fcitx5ControlUtf8 {
                    ptr: b"2\\0".as_ptr(),
                    len: 3,
                },
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
            Fcitx5ControlUtf8 {
                ptr: b"input-data".as_ptr(),
                len: 10,
            },
            Fcitx5ControlUtf8 {
                ptr: b"quotes\"ok".as_ptr(),
                len: 9,
            },
        ];
        let json = string_array_json(&values).expect("string array should format");
        let text = String::from_utf8(json).expect("string array JSON should be UTF-8");
        assert_eq!(text, r#"["input-data","quotes\"ok"]"#);
    }

    #[test]
    fn config_surfaces_json_preserves_control_contract() {
        let owner = Fcitx5ControlUtf8 {
            ptr: b"fcitx5-rime".as_ptr(),
            len: 11,
        };
        let kinds = [
            Fcitx5ControlUtf8 {
                ptr: b"fcitx-addon".as_ptr(),
                len: 11,
            },
            Fcitx5ControlUtf8 {
                ptr: b"rime-data".as_ptr(),
                len: 9,
            },
        ];
        let json = config_surfaces_json(owner, &kinds).expect("surfaces should format");
        let text = String::from_utf8(json).expect("surfaces JSON should be UTF-8");
        assert_eq!(
            text,
            r#"[{"kind":"fcitx-addon","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"},{"kind":"rime-data","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"}]"#
        );
    }

    #[test]
    fn package_config_surface_policy_matches_cpp_contract() {
        let owner = Fcitx5ControlUtf8 {
            ptr: b"fcitx5-rime".as_ptr(),
            len: 11,
        };
        let permissions = [Fcitx5ControlUtf8 {
            ptr: b"input-data".as_ptr(),
            len: 10,
        }];
        let file_paths = [
            Fcitx5ControlUtf8 {
                ptr: b"share/fcitx5/addon/rime.conf".as_ptr(),
                len: 28,
            },
            Fcitx5ControlUtf8 {
                ptr: b"lib/fcitx5/rime.dll".as_ptr(),
                len: 19,
            },
            Fcitx5ControlUtf8 {
                ptr: b"share/rime-data/default.yaml".as_ptr(),
                len: 28,
            },
            Fcitx5ControlUtf8 {
                ptr: b"themes/rime/theme.toml".as_ptr(),
                len: 22,
            },
        ];
        let json = package_config_surface_json(
            owner,
            CONTROL_PACKAGE_TYPE_ADDON,
            &permissions,
            &file_paths,
        )
        .expect("package config surface should format");
        let text = String::from_utf8(json).expect("surface JSON should be UTF-8");
        assert_eq!(
            text,
            r#"[{"kind":"fcitx-addon","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"},{"kind":"fcitx-addon-config","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"},{"kind":"input-method-data","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"},{"kind":"rime-data","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"},{"kind":"theme","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"}]"#
        );

        assert_eq!(
            package_config_surface_kinds(CONTROL_PACKAGE_TYPE_THEME, &[], &[])
                .expect("theme surfaces"),
            vec!["theme"]
        );
        assert!(
            package_config_surface_kinds(999, &[], &[]).is_none(),
            "unknown package types must not produce surfaces"
        );
    }

    #[test]
    fn repository_error_classification_matches_cpp_contract() {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-control-core-repository-error-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let keyring = root.join("repository").join("trusted-keys.json");

        assert_eq!(
            classify_repository_error(b"invalid_file", &keyring),
            b"missing_key"
        );
        std::fs::create_dir_all(keyring.parent().expect("keyring parent"))
            .expect("create keyring parent");
        std::fs::write(&keyring, b"[]").expect("write keyring");
        assert_eq!(
            classify_repository_error(b"invalid_file", &keyring),
            b"invalid_file"
        );
        assert_eq!(
            classify_repository_error(b"rollback_rejected", &keyring),
            b"rollback_rejected"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn bundled_package_probe_inventory_matches_cpp_contract() {
        assert_eq!(fcitx5_control_bundled_package_count(), 5);
        let descriptor = bundled_package_descriptor(0).expect("first bundled package");
        assert_eq!(
            utf8_slice(descriptor.id),
            Some(&b"fcitx5-chinese-addons"[..])
        );
        assert_eq!(
            utf8_slice(descriptor.title),
            Some(&b"Fcitx5 Chinese Addons"[..])
        );
        assert!(bundled_package_descriptor(99).is_none());

        let root = std::env::temp_dir().join(format!(
            "fcitx5-control-core-bundled-probe-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("lib/fcitx5")).expect("create lib probe dir");
        assert!(!bundled_package_present(&root, b"fcitx5-rime"));
        std::fs::write(root.join("lib/fcitx5/librime.dll"), b"fixture").expect("write probe");
        assert!(bundled_package_present(&root, b"fcitx5-rime"));
        assert!(!bundled_package_present(&root, b"unknown"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn package_type_names_match_cpp_contract() {
        let cases = [
            (CONTROL_PACKAGE_TYPE_CORE, b"core".as_slice()),
            (CONTROL_PACKAGE_TYPE_ADDON, b"addon".as_slice()),
            (
                CONTROL_PACKAGE_TYPE_INPUT_METHOD_DATA,
                b"inputmethod-data".as_slice(),
            ),
            (CONTROL_PACKAGE_TYPE_THEME, b"theme".as_slice()),
            (CONTROL_PACKAGE_TYPE_TRANSLATION, b"translation".as_slice()),
            (999, b"unknown".as_slice()),
        ];
        for (package_type, expected) in cases {
            assert_eq!(utf8_slice(package_type_name(package_type)), Some(expected));
            assert_eq!(
                utf8_slice(fcitx5_control_package_type_name_utf8(package_type)),
                Some(expected)
            );
        }
    }

    #[test]
    fn theme_vocabulary_and_match_policy_matches_cpp_contract() {
        assert_eq!(
            utf8_slice(fcitx5_control_builtin_theme_id_utf8()),
            Some(b"builtin:default".as_slice())
        );
        assert_eq!(
            utf8_slice(fcitx5_control_builtin_theme_source_utf8()),
            Some(b"builtin".as_slice())
        );
        assert_eq!(
            utf8_slice(fcitx5_control_user_theme_source_utf8()),
            Some(b"user".as_slice())
        );
        assert!(theme_record_matches_requested_id(
            b"builtin",
            b"builtin:default",
            b"other"
        ));
        assert!(theme_record_matches_requested_id(
            b"user", b"solar", b"solar"
        ));
        assert!(!theme_record_matches_requested_id(
            b"user",
            b"solar",
            b"midnight"
        ));

        let user = Fcitx5ControlUtf8 {
            ptr: b"user".as_ptr(),
            len: 4,
        };
        let requested = Fcitx5ControlUtf8 {
            ptr: b"solar".as_ptr(),
            len: 5,
        };
        let actual = Fcitx5ControlUtf8 {
            ptr: b"midnight".as_ptr(),
            len: 8,
        };
        assert_eq!(
            unsafe {
                fcitx5_control_theme_record_matches_requested_id_utf8(user, requested, actual)
            },
            0
        );
    }

    #[test]
    fn theme_discovery_matches_windinput_style_contract() {
        let install_root = PathBuf::from(r"C:\Fcitx5");
        let data_root = PathBuf::from(r"C:\Users\Alice\AppData\Roaming\Fcitx5");
        let theme_dir = data_root.join("themes").join("solar");
        assert_eq!(
            builtin_theme_path(&install_root),
            Some(
                install_root
                    .join("resources")
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
        assert!(builtin_theme_path(std::path::Path::new("")).is_none());
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
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|entry| {
            entry.id == "builtin:default"
                && entry.source == "builtin"
                && entry.path.ends_with(r"resources\themes\default\theme.toml")
        }));
        assert!(entries.iter().any(|entry| {
            entry.id == "solar" && entry.source == "user" && entry.path.ends_with("theme.toml")
        }));
        assert!(!entries.iter().any(|entry| entry.id == "Bad"));

        let install_wide = wide(&install.to_string_lossy());
        let data_wide = wide(&temp.join("data").to_string_lossy());
        let result = unsafe {
            fcitx5_control_discover_themes_utf16(
                Fcitx5ControlUtf16 {
                    ptr: install_wide.as_ptr(),
                    len: install_wide.len(),
                },
                Fcitx5ControlUtf16 {
                    ptr: data_wide.as_ptr(),
                    len: data_wide.len(),
                },
            )
        };
        assert_eq!(result.status, 0);
        assert_eq!(result.entry_count, 2);
        unsafe {
            fcitx5_control_theme_discovery_free(result);
        }
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

        let result = unsafe {
            fcitx5_control_parse_theme_summary_utf8(Fcitx5ControlUtf8 {
                ptr: text.as_ptr(),
                len: text.len(),
            })
        };
        assert_eq!(result.status, 0);
        assert_eq!(utf8_slice(result.id), Some(b"builtin.default".as_slice()));
        assert_eq!(utf8_slice(result.name), Some(b"Default".as_slice()));
        assert_eq!(utf8_slice(result.version), Some(b"1".as_slice()));
        assert_eq!(
            utf8_slice(result.license),
            Some(b"LGPL-2.1-or-later".as_slice())
        );
        assert_eq!(utf8_slice(result.description), Some(b"Built in".as_slice()));
        assert_eq!(result.has_light_branch, 1);
        assert_eq!(result.has_dark_branch, 1);
        unsafe {
            fcitx5_control_theme_summary_free(result);
        }

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
    fn theme_file_operations_are_rust_owned_and_scoped_to_user_theme_dir() {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-control-core-theme-ops-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let install = root.join("install");
        let data = root.join("data");
        let builtin_dir = install.join("resources").join("themes").join("default");
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
    fn theme_resolve_config_matches_cpp_contract() {
        let text = br##"
format_version = 1

[theme]
id = "solar"
name = "Solar"
version = "1"
license = "MIT"

[common.candidate]
orientation = "vertical"
page_size = 5

[common.candidate.colors]
candidate_text = "#101010FF"

[dark.candidate.colors]
background = "#242629F7"
candidate_text = "#FFFFFFFF"

[light.candidate.colors]
background = "#FCFCFCFA"
"##;
        let dark = resolved_theme_config(text, b"solar", false, true).unwrap();
        let dark_text = std::str::from_utf8(&dark).unwrap();
        assert!(dark_text.contains("format_version = 1"));
        assert!(dark_text.contains("orientation = \"vertical\""));
        assert!(dark_text.contains("page_size = 5"));
        assert!(dark_text.contains("background = \"#242629F7\""));
        assert!(dark_text.contains("candidate_text = \"#FFFFFFFF\""));

        let light = resolved_theme_config(text, b"solar", false, false).unwrap();
        let light_text = std::str::from_utf8(&light).unwrap();
        assert!(light_text.contains("background = \"#FCFCFCFA\""));
        assert!(light_text.contains("candidate_text = \"#101010FF\""));
        assert!(resolved_theme_config(text, b"midnight", false, true).is_none());
        assert!(resolved_theme_config(text, b"builtin:default", true, true).is_some());

        let mut ptr = std::ptr::null_mut();
        let mut len = 0;
        assert_eq!(
            unsafe {
                fcitx5_control_resolve_theme_config_utf8(
                    Fcitx5ControlUtf8 {
                        ptr: text.as_ptr(),
                        len: text.len(),
                    },
                    Fcitx5ControlUtf8 {
                        ptr: b"solar".as_ptr(),
                        len: 5,
                    },
                    0,
                    1,
                    &mut ptr,
                    &mut len,
                )
            },
            0
        );
        assert!(!ptr.is_null());
        assert!(len > 0);
        unsafe {
            fcitx5_control_utf8_free(ptr, len);
        }
    }

    #[test]
    fn theme_palette_resolve_matches_windinput_contract() {
        let text = br##"
format_version = 1

[theme]
id = "solar"
name = "Solar"
version = "1"
license = "MIT"

[palette]
surface = "#FCFCFCFA"
ink = "#101010FF"
candidate_text = "${ink}"

[common.candidate.colors]
background = "${surface}"
candidate_text = "${candidate_text}"

[dark.palette]
surface = "#242629F7"
ink = "#FFFFFFFF"

[dark.candidate.colors]
background = "${surface}"

[light.candidate.colors]
background = "${surface}"
"##;
        let dark = resolved_theme_config(text, b"solar", false, true).unwrap();
        let dark_text = std::str::from_utf8(&dark).unwrap();
        assert!(dark_text.contains("background = \"#242629F7\""));
        assert!(dark_text.contains("candidate_text = \"#FFFFFFFF\""));
        assert!(!dark_text.contains("[palette]"));
        assert!(!dark_text.contains("${"));

        let light = resolved_theme_config(text, b"solar", false, false).unwrap();
        let light_text = std::str::from_utf8(&light).unwrap();
        assert!(light_text.contains("background = \"#FCFCFCFA\""));
        assert!(light_text.contains("candidate_text = \"#101010FF\""));

        let palette_only_branch = br##"
format_version = 1
[theme]
id = "solar"
name = "Solar"
version = "1"
license = "MIT"
[dark.palette]
surface = "#242629F7"
"##;
        let summary = parse_theme_summary(palette_only_branch).unwrap();
        assert!(!summary.has_dark_branch);

        let unknown = br##"
format_version = 1
[theme]
id = "solar"
name = "Solar"
version = "1"
license = "MIT"
[common.candidate.colors]
background = "${missing}"
"##;
        assert!(resolved_theme_config(unknown, b"solar", false, false).is_none());

        let cycle = br##"
format_version = 1
[theme]
id = "solar"
name = "Solar"
version = "1"
license = "MIT"
[palette]
a = "${b}"
b = "${a}"
[common.candidate.colors]
background = "${a}"
"##;
        assert!(resolved_theme_config(cycle, b"solar", false, false).is_none());
    }

    #[test]
    fn theme_path_resolve_matches_windinput_style_contract() {
        let install = std::path::PathBuf::from(r"C:\Program Files\Fcitx5");
        let data = std::path::PathBuf::from(r"C:\Users\Alice\AppData\Local\Fcitx5");

        let builtin = resolve_theme_path(&install, &data, "builtin:default", true).unwrap();
        assert!(builtin.ends_with(r"resources\themes\default\theme.toml"));
        assert!(resolve_theme_path(&install, &data, "solar", true).is_none());

        let user = resolve_theme_path(&install, &data, "eosphoros-night", false).unwrap();
        assert!(user.ends_with(r"themes\eosphoros-night\theme.toml"));
        assert!(resolve_theme_path(&install, &data, "Bad ID", false).is_none());
        assert!(resolve_theme_path(&install, &std::path::PathBuf::new(), "solar", false).is_none());

        let install_wide = wide(&install.to_string_lossy());
        let data_wide = wide(&data.to_string_lossy());
        let required = unsafe {
            fcitx5_control_resolve_theme_path_utf16(
                Fcitx5ControlUtf16 {
                    ptr: install_wide.as_ptr(),
                    len: install_wide.len(),
                },
                Fcitx5ControlUtf16 {
                    ptr: data_wide.as_ptr(),
                    len: data_wide.len(),
                },
                Fcitx5ControlUtf8 {
                    ptr: b"eosphoros-night".as_ptr(),
                    len: 15,
                },
                0,
                std::ptr::null_mut(),
                0,
            )
        };
        assert!(required > 0);
        let mut output = vec![0u16; required];
        let written = unsafe {
            fcitx5_control_resolve_theme_path_utf16(
                Fcitx5ControlUtf16 {
                    ptr: install_wide.as_ptr(),
                    len: install_wide.len(),
                },
                Fcitx5ControlUtf16 {
                    ptr: data_wide.as_ptr(),
                    len: data_wide.len(),
                },
                Fcitx5ControlUtf8 {
                    ptr: b"eosphoros-night".as_ptr(),
                    len: 15,
                },
                0,
                output.as_mut_ptr(),
                output.len(),
            )
        };
        assert_eq!(written, required);
        assert!(String::from_utf16(&output)
            .unwrap()
            .ends_with(r"themes\eosphoros-night\theme.toml"));
    }

    #[test]
    fn native_package_architecture_matches_target_contract() {
        let expected = if cfg!(target_pointer_width = "64") {
            b"x64".as_slice()
        } else {
            b"x86".as_slice()
        };
        assert_eq!(utf8_slice(native_package_architecture()), Some(expected));
        assert_eq!(
            utf8_slice(fcitx5_control_native_package_architecture_utf8()),
            Some(expected)
        );
    }

    #[test]
    fn package_architecture_match_matches_cpp_contract() {
        let native = if cfg!(target_pointer_width = "64") {
            b"x64".as_slice()
        } else {
            b"x86".as_slice()
        };
        let other = if cfg!(target_pointer_width = "64") {
            b"x86".as_slice()
        } else {
            b"x64".as_slice()
        };
        assert!(package_architecture_matches_native(b"any"));
        assert!(package_architecture_matches_native(native));
        assert!(!package_architecture_matches_native(other));
        assert!(!package_architecture_matches_native(b"arm64"));

        let any = Fcitx5ControlUtf8 {
            ptr: b"any".as_ptr(),
            len: 3,
        };
        let unsupported = Fcitx5ControlUtf8 {
            ptr: b"arm64".as_ptr(),
            len: 5,
        };
        assert_eq!(
            unsafe { fcitx5_control_package_architecture_matches_native_utf8(any) },
            1
        );
        assert_eq!(
            unsafe { fcitx5_control_package_architecture_matches_native_utf8(unsupported) },
            0
        );
    }

    #[test]
    fn addon_metadata_vocabulary_matches_cpp_contract() {
        assert!(addon_metadata_section_is_addon(b"[Addon]"));
        assert!(!addon_metadata_section_is_addon(b"[Addon/Disabled]"));
        assert!(!addon_metadata_section_is_addon(b"Addon"));
        assert_eq!(addon_metadata_key(b"Name"), ADDON_METADATA_KEY_NAME);
        assert_eq!(addon_metadata_key(b"Category"), ADDON_METADATA_KEY_CATEGORY);
        assert_eq!(addon_metadata_key(b"Library"), ADDON_METADATA_KEY_LIBRARY);
        assert_eq!(addon_metadata_key(b"Type"), ADDON_METADATA_KEY_TYPE);
        assert_eq!(addon_metadata_key(b"Version"), ADDON_METADATA_KEY_VERSION);
        assert_eq!(
            addon_metadata_key(b"Configurable"),
            ADDON_METADATA_KEY_CONFIGURABLE
        );
        assert_eq!(
            addon_metadata_key(b"OnDemand"),
            ADDON_METADATA_KEY_ON_DEMAND
        );
        assert_eq!(addon_metadata_key(b"name"), ADDON_METADATA_KEY_UNKNOWN);

        let section = Fcitx5ControlUtf8 {
            ptr: b"[Addon]".as_ptr(),
            len: 7,
        };
        let key = Fcitx5ControlUtf8 {
            ptr: b"OnDemand".as_ptr(),
            len: 8,
        };
        assert_eq!(
            unsafe { fcitx5_control_addon_metadata_section_is_addon_utf8(section) },
            1
        );
        assert_eq!(
            unsafe { fcitx5_control_addon_metadata_key_utf8(key) },
            ADDON_METADATA_KEY_ON_DEMAND
        );
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

        let truthy = Fcitx5ControlUtf8 {
            ptr: b"True".as_ptr(),
            len: 4,
        };
        let falsey = Fcitx5ControlUtf8 {
            ptr: b"TRUE".as_ptr(),
            len: 4,
        };
        assert_eq!(
            unsafe { fcitx5_control_addon_metadata_bool_utf8(truthy) },
            1
        );
        assert_eq!(
            unsafe { fcitx5_control_addon_metadata_bool_utf8(falsey) },
            0
        );
    }

    #[test]
    fn package_update_available_matches_cpp_contract() {
        assert!(package_update_available(true, b"1.0.0", b"1.1.0"));
        assert!(!package_update_available(true, b"1.0.0", b"1.0.0"));
        assert!(!package_update_available(false, b"", b"1.1.0"));
        assert!(!package_update_available(true, b"1.0.0", b""));
        assert!(!package_update_available(true, b"", b"1.1.0"));

        let installed = Fcitx5ControlUtf8 {
            ptr: b"1.0.0".as_ptr(),
            len: 5,
        };
        let available = Fcitx5ControlUtf8 {
            ptr: b"1.1.0".as_ptr(),
            len: 5,
        };
        assert_eq!(
            unsafe { fcitx5_control_package_update_available_utf8(1, installed, available) },
            1
        );
        assert_eq!(
            unsafe { fcitx5_control_package_update_available_utf8(0, installed, available) },
            0
        );
    }

    #[test]
    fn package_lifecycle_state_policy_matches_cpp_contract() {
        for state in [
            b"installed".as_slice(),
            b"enabled".as_slice(),
            b"".as_slice(),
        ] {
            assert!(package_state_satisfies_dependency(state));
            assert!(package_state_keeps_installed_version(state));
        }
        for state in [
            b"disabled".as_slice(),
            b"pending_remove".as_slice(),
            b"broken".as_slice(),
            b"quarantined".as_slice(),
        ] {
            assert!(!package_state_satisfies_dependency(state));
        }
        assert!(package_state_keeps_installed_version(b"disabled"));
        assert!(package_state_keeps_installed_version(b"broken"));
        assert!(package_state_keeps_installed_version(b"quarantined"));
        assert!(!package_state_keeps_installed_version(b"pending_remove"));

        let disabled = Fcitx5ControlUtf8 {
            ptr: b"disabled".as_ptr(),
            len: 8,
        };
        let installed = Fcitx5ControlUtf8 {
            ptr: b"installed".as_ptr(),
            len: 9,
        };
        assert_eq!(
            unsafe { fcitx5_control_package_state_satisfies_dependency_utf8(disabled) },
            0
        );
        assert_eq!(
            unsafe { fcitx5_control_package_state_satisfies_dependency_utf8(installed) },
            1
        );
        assert_eq!(
            unsafe { fcitx5_control_package_state_keeps_installed_version_utf8(disabled) },
            1
        );
    }

    #[test]
    fn repository_max_release_sequence_matches_cpp_contract() {
        assert_eq!(repository_max_release_sequence(&[]), 0);
        assert_eq!(repository_max_release_sequence(&[7]), 7);
        assert_eq!(repository_max_release_sequence(&[3, 12, 9, 12, 1]), 12);
        assert_eq!(
            unsafe { fcitx5_control_repository_max_release_sequence(std::ptr::null(), 0) },
            0
        );
        let values = [10_u64, 2, 42, 8];
        assert_eq!(
            unsafe {
                fcitx5_control_repository_max_release_sequence(values.as_ptr(), values.len())
            },
            42
        );
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

        let name = Fcitx5ControlUtf8 {
            ptr: b"index.json".as_ptr(),
            len: 10,
        };
        let required = unsafe {
            fcitx5_control_repository_metadata_url_utf16(
                Fcitx5ControlUtf16 {
                    ptr: base.as_ptr(),
                    len: base.len(),
                },
                name,
                std::ptr::null_mut(),
                0,
            )
        };
        let mut output = vec![0_u16; required];
        let written = unsafe {
            fcitx5_control_repository_metadata_url_utf16(
                Fcitx5ControlUtf16 {
                    ptr: base.as_ptr(),
                    len: base.len(),
                },
                name,
                output.as_mut_ptr(),
                output.len(),
            )
        };
        assert_eq!(written, output.len());
        assert_eq!(
            String::from_utf16(&output).unwrap(),
            "https://packages.example/v1/dev/index.json"
        );
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

        let channel = Fcitx5ControlUtf8 {
            ptr: b"dev".as_ptr(),
            len: 3,
        };
        let required = unsafe {
            fcitx5_control_repository_default_base_url_utf16(channel, std::ptr::null_mut(), 0)
        };
        let mut output = vec![0_u16; required];
        let written = unsafe {
            fcitx5_control_repository_default_base_url_utf16(
                channel,
                output.as_mut_ptr(),
                output.len(),
            )
        };
        assert_eq!(written, output.len());
        assert_eq!(
            String::from_utf16(&output).unwrap(),
            "https://github.com/0x696c757a696f/fcitx5-windows-next/releases/download/v0.1.0"
        );
    }

    #[test]
    fn package_transaction_id_matches_cpp_contract() {
        assert_eq!(
            package_transaction_id(b"0123456789abcdef0123456789abcdef"),
            b"pkg-0123456789abcdef01234567"
        );
        assert_eq!(package_transaction_id(b"abc"), b"pkg-abc");
        assert_eq!(package_transaction_id(b""), b"pkg-");

        let sha = Fcitx5ControlUtf8 {
            ptr: b"abcdef0123456789abcdef0123456789".as_ptr(),
            len: 32,
        };
        let mut bytes = std::ptr::null_mut();
        let mut len = 0_usize;
        assert_eq!(
            unsafe { fcitx5_control_package_transaction_id_utf8(sha, &mut bytes, &mut len) },
            0
        );
        assert!(!bytes.is_null());
        let value = unsafe { std::slice::from_raw_parts(bytes, len).to_vec() };
        unsafe {
            fcitx5_control_utf8_free(bytes, len);
        }
        assert_eq!(value, b"pkg-abcdef0123456789abcdef01");
    }

    #[test]
    fn package_detail_json_preserves_control_contract() {
        let deps = br#"[{"id":"dep","version":"1"}]"#;
        let permissions = br#"["input-data"]"#;
        let surfaces =
            br#"[{"kind":"fcitx-addon","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"}]"#;
        let detail = Fcitx5ControlPackageDetail {
            repository_available: 1,
            repository_error: Fcitx5ControlUtf8 {
                ptr: b"".as_ptr(),
                len: 0,
            },
            id: Fcitx5ControlUtf8 {
                ptr: b"fcitx5-rime".as_ptr(),
                len: 11,
            },
            title: Fcitx5ControlUtf8 {
                ptr: b"Rime".as_ptr(),
                len: 4,
            },
            summary: Fcitx5ControlUtf8 {
                ptr: b"Rime input method".as_ptr(),
                len: 17,
            },
            package_type: Fcitx5ControlUtf8 {
                ptr: b"addon".as_ptr(),
                len: 5,
            },
            available_version: Fcitx5ControlUtf8 {
                ptr: b"1.2.3".as_ptr(),
                len: 5,
            },
            installed_version: Fcitx5ControlUtf8 {
                ptr: b"1.0.0".as_ptr(),
                len: 5,
            },
            state: Fcitx5ControlUtf8 {
                ptr: b"enabled".as_ptr(),
                len: 7,
            },
            bundled: 0,
            update_available: 1,
            manifest_sha256: Fcitx5ControlUtf8 {
                ptr: b"abc".as_ptr(),
                len: 3,
            },
            source_commit: Fcitx5ControlUtf8 {
                ptr: b"def".as_ptr(),
                len: 3,
            },
            dependencies_json: Fcitx5ControlUtf8 {
                ptr: deps.as_ptr(),
                len: deps.len(),
            },
            permissions_json: Fcitx5ControlUtf8 {
                ptr: permissions.as_ptr(),
                len: permissions.len(),
            },
            config_surface_json: Fcitx5ControlUtf8 {
                ptr: surfaces.as_ptr(),
                len: surfaces.len(),
            },
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
        let empty = Fcitx5ControlUtf8 {
            ptr: b"".as_ptr(),
            len: 0,
        };
        let array = Fcitx5ControlUtf8 {
            ptr: b"[]".as_ptr(),
            len: 2,
        };
        let detail = Fcitx5ControlPackageDetail {
            repository_available: 0,
            repository_error: Fcitx5ControlUtf8 {
                ptr: b"missing_key".as_ptr(),
                len: 11,
            },
            id: Fcitx5ControlUtf8 {
                ptr: b"bundled".as_ptr(),
                len: 7,
            },
            title: Fcitx5ControlUtf8 {
                ptr: b"bundled".as_ptr(),
                len: 7,
            },
            summary: empty,
            package_type: Fcitx5ControlUtf8 {
                ptr: b"addon".as_ptr(),
                len: 5,
            },
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
    fn input_method_ids_are_bounded_lowercase_ascii_tokens() {
        let valid = wide("rime-luna_pinyin");
        let upper = wide("Rime");
        let empty: Vec<u16> = Vec::new();
        let long = wide(&"a".repeat(65));
        unsafe {
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: valid.as_ptr(),
                    len: valid.len()
                }),
                1
            );
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: upper.as_ptr(),
                    len: upper.len()
                }),
                0
            );
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: empty.as_ptr(),
                    len: empty.len()
                }),
                0
            );
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: long.as_ptr(),
                    len: long.len()
                }),
                0
            );
        }
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
            current_input_method_id: Fcitx5ControlUtf8 {
                ptr: id.as_ptr(),
                len: id.len(),
            },
            current_input_method_name: Fcitx5ControlUtf8 {
                ptr: name.as_ptr(),
                len: name.len(),
            },
            current_input_method_native_name: Fcitx5ControlUtf8 {
                ptr: native.as_ptr(),
                len: native.len(),
            },
            current_input_method_short_label: Fcitx5ControlUtf8 {
                ptr: label.as_ptr(),
                len: label.len(),
            },
            config_valid: 1,
            tsf_guard_disabled: 1,
            tsf_guard_reason: Fcitx5ControlUtf8 {
                ptr: reason.as_ptr(),
                len: reason.len(),
            },
            data_root: Fcitx5ControlUtf8 {
                ptr: data_root.as_ptr(),
                len: data_root.len(),
            },
            update_owner: Fcitx5ControlUtf8 {
                ptr: owner.as_ptr(),
                len: owner.len(),
            },
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
        let empty = Fcitx5ControlUtf8 {
            ptr: b"".as_ptr(),
            len: 0,
        };
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
            tsf_guard_reason: Fcitx5ControlUtf8 {
                ptr: reason.as_ptr(),
                len: reason.len(),
            },
            data_root: Fcitx5ControlUtf8 {
                ptr: data_root.as_ptr(),
                len: data_root.len(),
            },
            update_owner: Fcitx5ControlUtf8 {
                ptr: owner.as_ptr(),
                len: owner.len(),
            },
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
    ) -> Fcitx5ControlStatus {
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
            current_input_method_id: Fcitx5ControlUtf8 {
                ptr: id.as_ptr(),
                len: id.len(),
            },
            current_input_method_name: Fcitx5ControlUtf8 {
                ptr: name.as_ptr(),
                len: name.len(),
            },
            current_input_method_native_name: Fcitx5ControlUtf8 {
                ptr: native.as_ptr(),
                len: native.len(),
            },
            current_input_method_short_label: Fcitx5ControlUtf8 {
                ptr: label.as_ptr(),
                len: label.len(),
            },
            config_valid: config_valid as u8,
            tsf_guard_disabled: tsf_guard_disabled as u8,
            tsf_guard_reason: Fcitx5ControlUtf8 {
                ptr: tsf_guard_reason.as_ptr(),
                len: tsf_guard_reason.len(),
            },
            data_root: Fcitx5ControlUtf8 {
                ptr: data_root.as_ptr(),
                len: data_root.len(),
            },
            update_owner: Fcitx5ControlUtf8 {
                ptr: owner.as_ptr(),
                len: owner.len(),
            },
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
    fn tsf_guard_json_preserves_control_contract() {
        let reason = b"manual \"disable\"";
        let marker = br#"C:/Users/Test/Fcitx5/tsf-guard.txt"#;
        let status = Fcitx5ControlTsfGuard {
            disabled: 1,
            reason: Fcitx5ControlUtf8 {
                ptr: reason.as_ptr(),
                len: reason.len(),
            },
            marker_path: Fcitx5ControlUtf8 {
                ptr: marker.as_ptr(),
                len: marker.len(),
            },
        };
        let json = tsf_guard_json(&status).expect("guard status should format");
        let text = String::from_utf8(json).expect("guard JSON should be UTF-8");
        assert_eq!(
            text,
            r#"{"format_version":1,"disabled":true,"reason":"manual \"disable\"","marker_path":"C:/Users/Test/Fcitx5/tsf-guard.txt"}"#
        );
        assert_eq!(
            CONTROL_TSF_GUARD_RESET_JSON,
            r#"{"format_version":1,"tsf_guard":"enabled"}"#
        );
    }
}
