#![forbid(unsafe_code)]

use std::process::ExitCode;

use fcitx5_launcher_core::{
    format_launcher_status, launcher_tick_milliseconds, parse_launcher_arguments,
    prepare_supervisor_start, LauncherInvocation,
};

fn run() -> Result<(), String> {
    let invocation =
        parse_launcher_arguments(std::env::args_os().skip(1)).map_err(|error| error.to_string())?;
    match invocation {
        LauncherInvocation::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        LauncherInvocation::TraySelfTest => {
            Err("tray self-test remains in the temporary native adapter".to_owned())
        }
        LauncherInvocation::Supervise(options) => {
            let executable = std::env::current_exe()
                .map_err(|error| format!("cannot resolve launcher executable: {error}"))?;
            let executable_directory = executable
                .parent()
                .ok_or_else(|| "launcher executable has no parent directory".to_owned())?;
            let startup = prepare_supervisor_start(
                &options,
                executable_directory,
                launcher_tick_milliseconds(),
            )
            .map_err(|error| error.to_string())?;
            println!("{}", format_launcher_status(&startup.status));
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
