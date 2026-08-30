#![windows_subsystem = "windows"]
#![forbid(unsafe_code)]

use std::process::ExitCode;

use fcitx5_launcher_core::{parse_launcher_arguments, run_launcher, LauncherInvocation};

fn run() -> Result<(), String> {
    let invocation =
        parse_launcher_arguments(std::env::args_os().skip(1)).map_err(|error| error.to_string())?;
    match invocation {
        LauncherInvocation::Version => {
            println!(
                "fcitx5-launcher {} protocol {}",
                env!("CARGO_PKG_VERSION"),
                fcitx5_protocol_core::VERSION
            );
            Ok(())
        }
        LauncherInvocation::Supervise(options) => {
            run_launcher(*options).map_err(|error| error.to_string())
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
