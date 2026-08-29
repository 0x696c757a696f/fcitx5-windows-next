#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use fcitx5_config_core::{CommitFault, ConfigCore, FileStore};
use fcitx5_control_core::{
    control_addons_list_json, control_config_reset, control_diagnostics_plan_for_path,
    control_package_detail_json, control_packages_list_json, control_schema_json,
    control_startup_json, control_startup_query, control_startup_set, control_theme_delete,
    control_theme_detail_json, control_theme_duplicate, control_theme_export,
    control_theme_export_to, control_theme_import, control_themes_list_json,
    control_tsf_guard_json, control_tsf_guard_reset, control_unreachable_status_json,
    control_usage_text,
};

enum Command {
    Version,
    Schema,
    Status,
    DiagnosticsPlan,
    GetStartup,
    SetStartup(bool),
    GetTsfGuard,
    ResetTsfGuard,
    PackagesList,
    PackagesDetail(String),
    ThemesList,
    ThemesDetail(String),
    AddonsList,
    Validate(PathBuf),
    Apply(PathBuf),
    ResetConfig,
    ThemesExport(String),
    ThemesExportTo(String, PathBuf),
    ThemesImport(PathBuf),
    ThemesDuplicate(String, String),
    ThemesDelete(String),
}

fn parse() -> Result<(PathBuf, Command), ()> {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let mut data_root = std::env::current_dir().map_err(|_| ())?;
    let mut command = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].to_str() {
            Some("--version") if command.is_none() => {
                command = Some(Command::Version);
                index += 1;
            }
            Some("--data-root") => {
                data_root = arguments.get(index + 1).map(PathBuf::from).ok_or(())?;
                index += 2;
            }
            Some("--schema") if command.is_none() => {
                command = Some(Command::Schema);
                index += 1;
            }
            Some("--status") if command.is_none() => {
                command = Some(Command::Status);
                index += 1;
            }
            Some("--diagnostics-plan") if command.is_none() => {
                command = Some(Command::DiagnosticsPlan);
                index += 1;
            }
            Some("--get-startup") if command.is_none() => {
                command = Some(Command::GetStartup);
                index += 1;
            }
            Some("--set-startup") if command.is_none() => {
                let enabled = match arguments
                    .get(index + 1)
                    .and_then(|argument| argument.to_str())
                {
                    Some("enabled") => true,
                    Some("disabled") => false,
                    _ => return Err(()),
                };
                command = Some(Command::SetStartup(enabled));
                index += 2;
            }
            Some("--get-tsf-guard") if command.is_none() => {
                command = Some(Command::GetTsfGuard);
                index += 1;
            }
            Some("--reset-tsf-guard") if command.is_none() => {
                command = Some(Command::ResetTsfGuard);
                index += 1;
            }
            Some("--packages-list") if command.is_none() => {
                command = Some(Command::PackagesList);
                index += 1;
            }
            Some("--packages-detail") if command.is_none() => {
                let argument = arguments
                    .get(index + 1)
                    .filter(|argument| !argument.to_string_lossy().starts_with('-'))
                    .ok_or(())?;
                command = Some(Command::PackagesDetail(
                    argument.to_string_lossy().into_owned(),
                ));
                index += 2;
            }
            Some("--themes-list") if command.is_none() => {
                command = Some(Command::ThemesList);
                index += 1;
            }
            Some("--themes-detail") if command.is_none() => {
                let argument = arguments
                    .get(index + 1)
                    .filter(|argument| !argument.to_string_lossy().starts_with('-'))
                    .ok_or(())?;
                command = Some(Command::ThemesDetail(
                    argument.to_string_lossy().into_owned(),
                ));
                index += 2;
            }
            Some("--addons-list") if command.is_none() => {
                command = Some(Command::AddonsList);
                index += 1;
            }
            Some("--validate-config") if command.is_none() => {
                command = Some(Command::Validate(
                    arguments.get(index + 1).map(PathBuf::from).ok_or(())?,
                ));
                index += 2;
            }
            Some("--apply-config") if command.is_none() => {
                command = Some(Command::Apply(
                    arguments.get(index + 1).map(PathBuf::from).ok_or(())?,
                ));
                index += 2;
            }
            Some("--reset-config") if command.is_none() => {
                command = Some(Command::ResetConfig);
                index += 1;
            }
            Some("--themes-export") if command.is_none() => {
                command = Some(Command::ThemesExport(
                    arguments
                        .get(index + 1)
                        .filter(|value| !value.to_string_lossy().starts_with('-'))
                        .map(|value| value.to_string_lossy().into_owned())
                        .ok_or(())?,
                ));
                index += 2;
            }
            Some("--themes-export-to") if command.is_none() => {
                command = Some(Command::ThemesExportTo(
                    arguments
                        .get(index + 1)
                        .filter(|value| !value.to_string_lossy().starts_with('-'))
                        .map(|value| value.to_string_lossy().into_owned())
                        .ok_or(())?,
                    PathBuf::from(
                        arguments
                            .get(index + 2)
                            .filter(|value| !value.to_string_lossy().starts_with('-'))
                            .ok_or(())?,
                    ),
                ));
                index += 3;
            }
            Some("--themes-import") if command.is_none() => {
                command = Some(Command::ThemesImport(PathBuf::from(
                    arguments
                        .get(index + 1)
                        .filter(|value| !value.to_string_lossy().starts_with('-'))
                        .ok_or(())?,
                )));
                index += 2;
            }
            Some("--themes-duplicate") if command.is_none() => {
                command = Some(Command::ThemesDuplicate(
                    arguments
                        .get(index + 1)
                        .filter(|value| !value.to_string_lossy().starts_with('-'))
                        .map(|value| value.to_string_lossy().into_owned())
                        .ok_or(())?,
                    arguments
                        .get(index + 2)
                        .filter(|value| !value.to_string_lossy().starts_with('-'))
                        .map(|value| value.to_string_lossy().into_owned())
                        .ok_or(())?,
                ));
                index += 3;
            }
            Some("--themes-delete") if command.is_none() => {
                command = Some(Command::ThemesDelete(
                    arguments
                        .get(index + 1)
                        .filter(|value| !value.to_string_lossy().starts_with('-'))
                        .map(|value| value.to_string_lossy().into_owned())
                        .ok_or(())?,
                ));
                index += 2;
            }
            _ => return Err(()),
        }
    }
    Ok((data_root, command.ok_or(())?))
}

fn run(data_root: PathBuf, command: Command) -> Result<(), String> {
    match command {
        Command::Version => println!(
            "{}",
            option_env!("FCITX_WINDOWS_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
        ),
        Command::Schema => println!("{}", control_schema_json()),
        Command::Status => {
            let store = FileStore::new();
            let config_path = data_root.join("config.toml");
            let config_valid = ConfigCore::load_or_defaults(&store, &config_path)
                .and_then(|core| core.validate())
                .is_ok();
            let root = data_root.to_string_lossy();
            let status = control_unreachable_status_json(&root, config_valid)
                .ok_or_else(|| "cannot format control status".to_owned())?;
            println!("{status}");
        }
        Command::DiagnosticsPlan => {
            let store = FileStore::new();
            let config_path = data_root.join("config.toml");
            let config_valid = ConfigCore::load_or_defaults(&store, &config_path)
                .and_then(|core| core.validate())
                .is_ok();
            let plan = control_diagnostics_plan_for_path(&data_root, config_valid)
                .ok_or_else(|| "cannot format diagnostics plan".to_owned())?;
            println!("{plan}");
        }
        Command::GetStartup => println!(
            "{}",
            control_startup_json(control_startup_query().map_err(str::to_owned)?)
        ),
        Command::SetStartup(enabled) => control_startup_set(enabled).map_err(str::to_owned)?,
        Command::GetTsfGuard => println!(
            "{}",
            control_tsf_guard_json(&data_root).map_err(str::to_owned)?
        ),
        Command::ResetTsfGuard => {
            control_tsf_guard_reset(&data_root).map_err(str::to_owned)?;
            println!(r#"{{"format_version":1,"tsf_guard":"enabled"}}"#);
        }
        Command::PackagesList => println!(
            "{}",
            control_packages_list_json(&data_root).map_err(str::to_owned)?
        ),
        Command::PackagesDetail(id) => println!(
            "{}",
            control_package_detail_json(&data_root, &id).map_err(str::to_owned)?
        ),
        Command::ThemesList => println!(
            "{}",
            control_themes_list_json(&data_root).map_err(str::to_owned)?
        ),
        Command::ThemesDetail(id) => println!(
            "{}",
            control_theme_detail_json(&data_root, &id).map_err(str::to_owned)?
        ),
        Command::AddonsList => println!("{}", control_addons_list_json().map_err(str::to_owned)?),
        Command::ResetConfig => control_config_reset(&data_root).map_err(str::to_owned)?,
        Command::ThemesExport(id) => print!(
            "{}",
            control_theme_export(&data_root, &id).map_err(str::to_owned)?
        ),
        Command::ThemesExportTo(id, path) => println!(
            "{}",
            control_theme_export_to(&data_root, &id, &path).map_err(str::to_owned)?
        ),
        Command::ThemesImport(path) => println!(
            "{}",
            control_theme_import(&data_root, &path).map_err(str::to_owned)?
        ),
        Command::ThemesDuplicate(source, id) => println!(
            "{}",
            control_theme_duplicate(&data_root, &source, &id).map_err(str::to_owned)?
        ),
        Command::ThemesDelete(id) => println!(
            "{}",
            control_theme_delete(&data_root, &id).map_err(str::to_owned)?
        ),
        Command::Validate(source) => {
            let store = FileStore::new();
            let mut core = ConfigCore::compiled_defaults();
            core.import_from_path(&store, &source)
                .and_then(|()| core.validate())
                .map_err(|error| error.to_string())?;
        }
        Command::Apply(source) => {
            let store = FileStore::new();
            let config_path = data_root.join("config.toml");
            let mut core = ConfigCore::load_or_defaults(&store, &config_path)
                .map_err(|error| error.to_string())?;
            core.import_from_path(&store, &source)
                .and_then(|()| core.apply(&store, &config_path, CommitFault::None))
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let Ok((data_root, command)) = parse() else {
        eprint!("{}", control_usage_text());
        return ExitCode::from(2);
    };
    match run(data_root, command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
