#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use fcitx5_register_core::{
    invoke_registration_export, is_elevated, operation_export, operation_requires_admin,
    parse_operation, registered_path_for_display, registration_status_for_dll,
    validate_dll_argument, validate_product_artifact, REGISTER_ARTIFACT_CURRENT_DLL_MISSING,
    REGISTER_ARTIFACT_DLL_OUTSIDE_PRODUCT, REGISTER_ARTIFACT_HELPER_LOCATION,
    REGISTER_ARTIFACT_INVALID_ARGUMENT, REGISTER_ARTIFACT_OK, REGISTER_ARTIFACT_PAIRED_DLL_MISSING,
    REGISTER_DLL_ARGUMENT_OK, REGISTER_OPERATION_REGISTER, REGISTER_OPERATION_REPAIR,
    REGISTER_OPERATION_STATUS, REGISTER_OPERATION_UNREGISTER, REGISTER_OPERATION_VALIDATE_ARTIFACT,
    REGISTER_STATUS_NOT_REGISTERED, REGISTER_STATUS_PATH_MISMATCH, REGISTER_STATUS_REGISTERED,
};

fn current_architecture_bits() -> u32 {
    if cfg!(target_pointer_width = "64") {
        64
    } else {
        32
    }
}

fn version() -> &'static str {
    option_env!("FCITX_WINDOWS_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn usage() {
    eprintln!(
        "Usage: fcitx5-register --register|--unregister|--repair|--status|--validate-artifact --dll ABSOLUTE_PATH"
    );
}

fn artifact_error(status: u32) -> &'static str {
    match status {
        REGISTER_ARTIFACT_HELPER_LOCATION => {
            "register helper is not running from the product bin directory"
        }
        REGISTER_ARTIFACT_CURRENT_DLL_MISSING => {
            "current architecture TSF DLL is missing from the product artifact"
        }
        REGISTER_ARTIFACT_PAIRED_DLL_MISSING => {
            "paired architecture TSF DLL is missing from the product artifact"
        }
        REGISTER_ARTIFACT_DLL_OUTSIDE_PRODUCT => {
            "TSF DLL path does not belong to this product artifact"
        }
        REGISTER_ARTIFACT_INVALID_ARGUMENT | REGISTER_ARTIFACT_OK => {
            "register artifact validation failed"
        }
        _ => "register artifact validation failed",
    }
}

fn executable_path() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

fn print_path_mismatch() {
    let actual = registered_path_for_display()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    println!("path_mismatch {actual}");
}

fn validate_artifact(helper: &Path, dll: &Path) -> Result<(), u32> {
    let status = validate_product_artifact(helper, dll, current_architecture_bits());
    if status == REGISTER_ARTIFACT_OK {
        Ok(())
    } else {
        Err(status)
    }
}

fn run(args: &[OsString]) -> i32 {
    if args.len() == 2 && args[1] == "--version" {
        println!("{}", version());
        return 0;
    }
    if args.len() != 4 || args[2] != "--dll" {
        usage();
        return 2;
    }

    let operation = parse_operation(&args[1]);
    let dll = PathBuf::from(&args[3]);
    if validate_dll_argument(&dll) != REGISTER_DLL_ARGUMENT_OK {
        eprintln!("The TSF DLL must be an absolute path ending in fcitx5-tsf.dll.");
        return 2;
    }

    let Some(helper) = executable_path() else {
        eprintln!("register helper path could not be resolved");
        return 2;
    };
    if let Err(status) = validate_artifact(&helper, &dll) {
        eprintln!("{}", artifact_error(status));
        return 2;
    }

    if operation == REGISTER_OPERATION_VALIDATE_ARTIFACT {
        println!("artifact_valid");
        return 0;
    }

    if operation == REGISTER_OPERATION_STATUS {
        match registration_status_for_dll(&dll) {
            REGISTER_STATUS_REGISTERED => {
                println!("registered");
                return 0;
            }
            REGISTER_STATUS_NOT_REGISTERED => {
                println!("not_registered");
                return 3;
            }
            REGISTER_STATUS_PATH_MISMATCH => {
                print_path_mismatch();
                return 3;
            }
            _ => {
                println!("not_registered");
                return 3;
            }
        }
    }

    if !matches!(
        operation,
        REGISTER_OPERATION_REGISTER | REGISTER_OPERATION_REPAIR | REGISTER_OPERATION_UNREGISTER
    ) {
        usage();
        return 2;
    }
    if operation_requires_admin(operation) != 0 && !is_elevated() {
        eprintln!("Registration changes require an elevated administrator token.");
        return 5;
    }
    if !dll.is_file() {
        eprintln!("TSF DLL does not exist: {}", dll.display());
        return 2;
    }

    let result = invoke_registration_export(&dll, operation_export(operation));
    if result < 0 {
        eprintln!("Registration operation failed: 0x{:08x}", result as u32);
        return 6;
    }
    if operation != REGISTER_OPERATION_UNREGISTER
        && registration_status_for_dll(&dll) != REGISTER_STATUS_REGISTERED
    {
        eprintln!("Registration completed but the registered path does not match.");
        return 6;
    }
    0
}

fn main() {
    let args: Vec<OsString> = std::env::args_os().collect();
    std::process::exit(run(&args));
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcitx5_register_core::REGISTER_OPERATION_UNKNOWN;

    #[test]
    fn usage_errors_match_register_contract() {
        let args = vec![OsString::from("fcitx5-register")];
        assert_eq!(run(&args), 2);
    }

    #[test]
    fn version_command_succeeds() {
        let args = vec![
            OsString::from("fcitx5-register"),
            OsString::from("--version"),
        ];
        assert_eq!(run(&args), 0);
    }

    #[test]
    fn rejects_non_absolute_dll_argument_before_artifact_checks() {
        let args = vec![
            OsString::from("fcitx5-register"),
            OsString::from("--validate-artifact"),
            OsString::from("--dll"),
            OsString::from("fcitx5-tsf.dll"),
        ];
        assert_eq!(run(&args), 2);
    }

    #[test]
    fn unknown_operation_after_valid_dll_shape_returns_usage_error() {
        let args = vec![
            OsString::from("fcitx5-register"),
            OsString::from("--unknown"),
            OsString::from("--dll"),
            OsString::from("C:\\Fcitx5\\tsf\\x64\\fcitx5-tsf.dll"),
        ];
        assert_eq!(parse_operation(&args[1]), REGISTER_OPERATION_UNKNOWN);
        assert_eq!(
            validate_dll_argument(Path::new(&args[3])),
            REGISTER_DLL_ARGUMENT_OK
        );
    }
}
