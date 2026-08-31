#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fcitx5_config_core::{CommitFault, ConfigCore, FileStore};
use fcitx5_control_core::{
    control_addons_list_json, control_config_reset, control_diagnostics_plan_for_path,
    control_input_method_id_valid, control_install_root, control_package_detail_json,
    control_packages_list_json, control_repository_default_base_url,
    control_repository_metadata_url, control_schema_json, control_startup_json,
    control_startup_query, control_startup_set, control_theme_delete, control_theme_detail_json,
    control_theme_duplicate, control_theme_export, control_theme_export_to, control_theme_import,
    control_themes_list_json, control_tsf_guard_json, control_tsf_guard_reset,
    control_unreachable_status_json, control_usage_text,
};
use fcitx5_package_core::{
    DownloaderRepositoryTransport, PackageCoreFacade, PackageId, PackageInstallRequest,
    PackageLifecycleState, PackageRepairVerification, RepositoryRefreshRequest,
};
use fcitx5_process_execution_core::run_process_bounded;
use fcitx5_protocol_core::{
    decode_frame, decode_launcher_response, encode_launcher_request, LauncherCommand,
    LauncherRequest, Metadata, Status, MAX_CONTROL_FRAME_SIZE,
};
use fcitx5_windows_common_core::{
    current_runtime_generation_for_current_process, next_launcher_request_id,
    CurrentUserRuntimeIdentity, VerifiedPipeClient,
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
    GetInputMethods,
    SetInputMethod(String),
    RestartEngine,
    Shutdown,
    PackagesRefresh(Option<String>),
    PackagesInstall(String),
    PackagesUpdate(String),
    PackagesState(String, PackageLifecycleState),
    PackagesRemove(String),
    PackagesRepair,
}

struct Failure {
    code: u8,
    message: String,
}

impl Failure {
    fn launcher(message: impl Into<String>) -> Self {
        Self {
            code: 4,
            message: message.into(),
        }
    }

    fn package(message: impl Into<String>) -> Self {
        Self {
            code: 6,
            message: message.into(),
        }
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self { code: 2, message }
    }
}

impl From<&'static str> for Failure {
    fn from(message: &'static str) -> Self {
        Self {
            code: 2,
            message: message.to_owned(),
        }
    }
}

fn text_argument(arguments: &[OsString], index: usize) -> Result<String, ()> {
    arguments
        .get(index)
        .and_then(|argument| argument.to_str())
        .filter(|argument| !argument.is_empty() && !argument.starts_with('-'))
        .map(str::to_owned)
        .ok_or(())
}

fn package_id_argument(arguments: &[OsString], index: usize) -> Result<String, ()> {
    let id = text_argument(arguments, index)?;
    PackageId::parse(&id).map_err(|_| ())?;
    Ok(id)
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
                command = Some(Command::PackagesDetail(text_argument(
                    &arguments,
                    index + 1,
                )?));
                index += 2;
            }
            Some("--themes-list") if command.is_none() => {
                command = Some(Command::ThemesList);
                index += 1;
            }
            Some("--themes-detail") if command.is_none() => {
                command = Some(Command::ThemesDetail(text_argument(&arguments, index + 1)?));
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
                command = Some(Command::ThemesExport(text_argument(&arguments, index + 1)?));
                index += 2;
            }
            Some("--themes-export-to") if command.is_none() => {
                command = Some(Command::ThemesExportTo(
                    text_argument(&arguments, index + 1)?,
                    PathBuf::from(text_argument(&arguments, index + 2)?),
                ));
                index += 3;
            }
            Some("--themes-import") if command.is_none() => {
                command = Some(Command::ThemesImport(PathBuf::from(text_argument(
                    &arguments,
                    index + 1,
                )?)));
                index += 2;
            }
            Some("--themes-duplicate") if command.is_none() => {
                command = Some(Command::ThemesDuplicate(
                    text_argument(&arguments, index + 1)?,
                    text_argument(&arguments, index + 2)?,
                ));
                index += 3;
            }
            Some("--themes-delete") if command.is_none() => {
                command = Some(Command::ThemesDelete(text_argument(&arguments, index + 1)?));
                index += 2;
            }
            Some("--get-input-methods") if command.is_none() => {
                command = Some(Command::GetInputMethods);
                index += 1;
            }
            Some("--set-input-method") if command.is_none() => {
                let id = text_argument(&arguments, index + 1)?;
                if !control_input_method_id_valid(&id) {
                    return Err(());
                }
                command = Some(Command::SetInputMethod(id));
                index += 2;
            }
            Some("--restart-engine") if command.is_none() => {
                command = Some(Command::RestartEngine);
                index += 1;
            }
            Some("--shutdown") if command.is_none() => {
                command = Some(Command::Shutdown);
                index += 1;
            }
            Some("--packages-refresh") if command.is_none() => {
                let base = arguments
                    .get(index + 1)
                    .filter(|argument| !argument.to_string_lossy().starts_with('-'))
                    .map(|argument| argument.to_string_lossy().into_owned());
                index += usize::from(base.is_some()) + 1;
                command = Some(Command::PackagesRefresh(base));
            }
            Some("--packages-install") if command.is_none() => {
                command = Some(Command::PackagesInstall(package_id_argument(
                    &arguments,
                    index + 1,
                )?));
                index += 2;
            }
            Some("--packages-update") if command.is_none() => {
                command = Some(Command::PackagesUpdate(package_id_argument(
                    &arguments,
                    index + 1,
                )?));
                index += 2;
            }
            Some("--packages-state") if command.is_none() => {
                let id = package_id_argument(&arguments, index + 1)?;
                let state = match text_argument(&arguments, index + 2)?.as_str() {
                    "enabled" => PackageLifecycleState::Enabled,
                    "disabled" => PackageLifecycleState::Disabled,
                    _ => return Err(()),
                };
                command = Some(Command::PackagesState(id, state));
                index += 3;
            }
            Some("--packages-remove") if command.is_none() => {
                command = Some(Command::PackagesRemove(package_id_argument(
                    &arguments,
                    index + 1,
                )?));
                index += 2;
            }
            Some("--packages-repair") if command.is_none() => {
                command = Some(Command::PackagesRepair);
                index += 1;
            }
            _ => return Err(()),
        }
    }
    Ok((data_root, command.ok_or(())?))
}

fn repository_now_seconds() -> Result<u64, Failure> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| Failure::package("repository clock is unavailable"))
}

fn launcher_path() -> Result<PathBuf, Failure> {
    let executable =
        std::env::current_exe().map_err(|_| Failure::launcher("launcher unavailable"))?;
    let directory = executable
        .parent()
        .ok_or_else(|| Failure::launcher("launcher unavailable"))?;
    let generation = current_runtime_generation_for_current_process();
    let install_root = control_install_root().map_err(Failure::launcher)?;
    let generation_launcher = install_root
        .join("runtime")
        .join(generation)
        .join("bin")
        .join("fcitx5-launcher.exe");
    Ok(if generation_launcher.is_file() {
        generation_launcher
    } else {
        directory.join("fcitx5-launcher.exe")
    })
}

fn launcher_command(command: LauncherCommand) -> Result<(), Failure> {
    let identity = CurrentUserRuntimeIdentity::current()
        .ok_or_else(|| Failure::launcher("launcher unavailable"))?;
    let generation = current_runtime_generation_for_current_process();
    let endpoint = identity
        .local_endpoint_name(&generation, "launcher")
        .ok_or_else(|| Failure::launcher("launcher unavailable"))?;
    let expected_server = launcher_path()?;
    let mut client = VerifiedPipeClient::connect_exact(
        endpoint.as_os_str(),
        &expected_server,
        Duration::from_secs(1),
    )
    .ok_or_else(|| Failure::launcher("launcher unavailable"))?;
    let request_id = next_launcher_request_id();
    let request = LauncherRequest {
        metadata: Metadata {
            request_id,
            session_id: identity.session_id(),
            ..Metadata::default()
        },
        command,
    };
    let request = encode_launcher_request(&request)
        .ok_or_else(|| Failure::launcher("launcher request rejected"))?;
    let mut response = vec![0_u8; MAX_CONTROL_FRAME_SIZE];
    let response_len = client
        .transact(&request, &mut response, Duration::from_secs(1))
        .ok_or_else(|| Failure::launcher("launcher unavailable"))?;
    let frame = decode_frame(&response[..response_len])
        .ok_or_else(|| Failure::launcher("launcher returned an invalid response"))?;
    let response = decode_launcher_response(&frame)
        .ok_or_else(|| Failure::launcher("launcher returned an invalid response"))?;
    if response.metadata.response_to != request_id
        || response.metadata.session_id != identity.session_id()
        || response.status != Status::Ok
    {
        return Err(Failure::launcher("launcher rejected the command"));
    }
    Ok(())
}

fn request_engine_reload() {
    if launcher_command(LauncherCommand::Status).is_ok()
        && (launcher_command(LauncherCommand::UserStop).is_err()
            || launcher_command(LauncherCommand::Resume).is_err()
            || launcher_command(LauncherCommand::StartDemand).is_err())
    {
        eprintln!("warning: package change is saved; restart the input service to activate it");
    }
}

fn run_engine_management(arguments: &[OsString]) -> Result<String, Failure> {
    let launcher_reachable = launcher_command(LauncherCommand::Status).is_ok();
    if launcher_reachable {
        launcher_command(LauncherCommand::UserStop)?;
    }
    let executable = std::env::current_exe()
        .map_err(|_| Failure::launcher("engine unavailable"))?
        .parent()
        .ok_or_else(|| Failure::launcher("engine unavailable"))?
        .join("fcitx5-engine.exe");
    let result = run_process_bounded(&executable, arguments, 120_000, 2 * 1024 * 1024)
        .map_err(|_| Failure::launcher("engine management failed"));
    let restored = !launcher_reachable
        || (launcher_command(LauncherCommand::Resume).is_ok()
            && launcher_command(LauncherCommand::StartDemand).is_ok());
    let result = result?;
    if !result.success || !restored {
        return Err(Failure::launcher("engine management failed"));
    }
    Ok(result.output)
}

fn package_facade(data_root: &std::path::Path) -> Result<PackageCoreFacade, Failure> {
    let install_root = control_install_root().map_err(Failure::package)?;
    let architecture = if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "x64"
    };
    Ok(PackageCoreFacade::new(
        install_root,
        data_root.to_owned(),
        architecture,
    ))
}

fn package_transport(
    data_root: &std::path::Path,
) -> Result<DownloaderRepositoryTransport, Failure> {
    let downloader = std::env::current_exe()
        .map_err(|_| Failure::package("downloader unavailable"))?
        .parent()
        .map(|directory| directory.join("fcitx5-downloader.exe"))
        .ok_or_else(|| Failure::package("downloader unavailable"))?;
    Ok(DownloaderRepositoryTransport::new(
        downloader,
        data_root.join("packages").join("control-downloads"),
    ))
}

fn refresh_packages(data_root: &std::path::Path, base_url: Option<String>) -> Result<(), Failure> {
    let base_url = base_url.unwrap_or_else(control_repository_default_base_url);
    let index_url =
        control_repository_metadata_url(&base_url, "index.json").map_err(Failure::package)?;
    let signature_url =
        control_repository_metadata_url(&base_url, "index.sig.json").map_err(Failure::package)?;
    let facade = package_facade(data_root)?;
    let mut transport = package_transport(data_root)?;
    facade
        .refresh_repository(
            RepositoryRefreshRequest {
                index_url: &index_url,
                signature_url: &signature_url,
                repository_id: "fcitx5-windows-next",
                channel: option_env!("FCITX_RELEASE_CHANNEL_NAME").unwrap_or("stable"),
                mirror_id: "official",
                now_seconds: repository_now_seconds()?,
            },
            &mut transport,
        )
        .map_err(|error| Failure::package(format!("{}: {error}", error.code())))?;
    println!(
        "{}",
        control_packages_list_json(data_root).map_err(Failure::package)?
    );
    Ok(())
}

fn install_or_update_package(data_root: &std::path::Path, id: &str) -> Result<(), Failure> {
    let facade = package_facade(data_root)?;
    let mut transport = package_transport(data_root)?;
    facade
        .install_or_update(
            "fcitx5-windows-next",
            option_env!("FCITX_RELEASE_CHANNEL_NAME").unwrap_or("stable"),
            "official",
            repository_now_seconds()?,
            PackageInstallRequest {
                requested_ids: &[id],
                transaction_id: id,
            },
            &mut transport,
        )
        .map_err(|error| Failure::package(format!("{}: {error}", error.code())))?;
    request_engine_reload();
    println!(
        "{}",
        control_packages_list_json(data_root).map_err(Failure::package)?
    );
    Ok(())
}

fn run(data_root: PathBuf, command: Command) -> Result<(), Failure> {
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
        Command::GetStartup => println!("{}", control_startup_json(control_startup_query()?)),
        Command::SetStartup(enabled) => control_startup_set(enabled)?,
        Command::GetTsfGuard => println!("{}", control_tsf_guard_json(&data_root)?),
        Command::ResetTsfGuard => {
            control_tsf_guard_reset(&data_root)?;
            println!("{{\"format_version\":1,\"tsf_guard\":\"enabled\"}}");
        }
        Command::PackagesList => println!("{}", control_packages_list_json(&data_root)?),
        Command::PackagesDetail(id) => {
            println!("{}", control_package_detail_json(&data_root, &id)?)
        }
        Command::ThemesList => println!("{}", control_themes_list_json(&data_root)?),
        Command::ThemesDetail(id) => println!("{}", control_theme_detail_json(&data_root, &id)?),
        Command::AddonsList => println!("{}", control_addons_list_json()?),
        Command::ResetConfig => control_config_reset(&data_root)?,
        Command::ThemesExport(id) => print!("{}", control_theme_export(&data_root, &id)?),
        Command::ThemesExportTo(id, path) => {
            println!("{}", control_theme_export_to(&data_root, &id, &path)?)
        }
        Command::ThemesImport(path) => println!("{}", control_theme_import(&data_root, &path)?),
        Command::ThemesDuplicate(source, id) => {
            println!("{}", control_theme_duplicate(&data_root, &source, &id)?)
        }
        Command::ThemesDelete(id) => println!("{}", control_theme_delete(&data_root, &id)?),
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
        Command::GetInputMethods => print!(
            "{}",
            run_engine_management(&[OsString::from("--list-input-methods")])?
        ),
        Command::SetInputMethod(id) => {
            let _ =
                run_engine_management(&[OsString::from("--set-input-method"), OsString::from(id)])?;
        }
        Command::RestartEngine => {
            for command in [
                LauncherCommand::UserStop,
                LauncherCommand::Resume,
                LauncherCommand::StartDemand,
            ] {
                launcher_command(command)?;
            }
        }
        Command::Shutdown => launcher_command(LauncherCommand::Shutdown)?,
        Command::PackagesRefresh(base_url) => refresh_packages(&data_root, base_url)?,
        Command::PackagesInstall(id) | Command::PackagesUpdate(id) => {
            install_or_update_package(&data_root, &id)?
        }
        Command::PackagesState(id, state) => {
            package_facade(&data_root)?
                .set_package_state(&id, state)
                .map_err(|error| Failure::package(format!("{}: {error}", error.code())))?;
            request_engine_reload();
        }
        Command::PackagesRemove(id) => {
            package_facade(&data_root)?
                .remove_package(&id)
                .map_err(|error| Failure::package(format!("{}: {error}", error.code())))?;
            request_engine_reload();
            println!(
                "{}",
                control_packages_list_json(&data_root).map_err(Failure::package)?
            );
        }
        Command::PackagesRepair => {
            let report = package_facade(&data_root)?
                .repair()
                .map_err(|error| Failure::package(format!("{}: {error}", error.code())))?;
            if !matches!(report.verification(), PackageRepairVerification::Verified) {
                return Err(Failure::package(
                    "repair_failed: installed package verification failed",
                ));
            }
            println!(
                "{{\"format_version\":1,\"repair\":\"verified\",\"repository_sequence_state\":\"not_applicable\"}}"
            );
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
            eprintln!("{}", error.message);
            ExitCode::from(error.code)
        }
    }
}
