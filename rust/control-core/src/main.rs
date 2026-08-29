#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use fcitx5_config_core::{CommitFault, ConfigCore, FileStore};
use fcitx5_control_core::{
    control_schema_json, control_unreachable_status_json, control_usage_text,
};

enum Command {
    Schema,
    Status,
    Validate(PathBuf),
    Apply(PathBuf),
}

fn parse() -> Result<(PathBuf, Command), ()> {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let mut data_root = std::env::current_dir().map_err(|_| ())?;
    let mut command = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].to_str() {
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
            _ => return Err(()),
        }
    }
    Ok((data_root, command.ok_or(())?))
}

fn run(data_root: PathBuf, command: Command) -> Result<(), String> {
    match command {
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
