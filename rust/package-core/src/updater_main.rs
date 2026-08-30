#![forbid(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;

use fcitx5_package_core::{update, PackageCoreFacade};

fn version() -> &'static str {
    option_env!("FCITX_WINDOWS_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn main() {
    let exit_code = match run(std::env::args_os().collect()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("update_failed: {error}");
            2
        }
    };
    std::process::exit(exit_code);
}

fn run(args: Vec<OsString>) -> Result<i32, Box<dyn Error>> {
    if args.len() == 2 && args[1] == "--version" {
        println!("fcitx5-updater {}", version());
        return Ok(0);
    }
    if args.len() == 4 && args[1] == "--record-owner" {
        let owner = update::parse_owner(&token_arg(&args[3])?).ok_or("update owner is invalid")?;
        update::write_update_owner(&path_arg(&args[2]), owner)?;
        return Ok(0);
    }
    if args.len() == 7 && args[1] == "--activate" {
        let archive = path_arg(&args[2]);
        let root = path_arg(&args[3]);
        let transaction = token_arg(&args[4])?;
        let keyring = path_arg(&args[5]);
        let channel = token_arg(&args[6])?;
        let facade = PackageCoreFacade::new(root.clone(), root, "any");
        let result = facade.activate_core_update(&archive, &transaction, &keyring, &channel)?;
        println!("activation=pending_health\nversion={}", result.version());
        return Ok(0);
    }
    if args.len() == 4 && args[1] == "--health" {
        let root = path_arg(&args[2]);
        let facade = PackageCoreFacade::new(root.clone(), root, "any");
        facade.mark_core_update_healthy(&token_arg(&args[3])?)?;
        return Ok(0);
    }
    if args.len() == 6 && args[1] == "--rollback" {
        let root = path_arg(&args[2]);
        let channel = token_arg(&args[3])?;
        let package_id = token_arg(&args[4])?;
        let keyring = path_arg(&args[5]);
        let facade = PackageCoreFacade::new(root.clone(), root, "any");
        let target = facade.rollback_core_update(&channel, &package_id, &keyring)?;
        println!("rollback={target}");
        return Ok(0);
    }
    if args.len() == 5 && args[1] == "--cleanup-previous" {
        let root = path_arg(&args[2]);
        let facade = PackageCoreFacade::new(root.clone(), root, "any");
        facade.cleanup_previous_core_update(&token_arg(&args[3])?, &token_arg(&args[4])?)?;
        return Ok(0);
    }
    if args.len() == 5 && args[1] == "--install-tsf-dll" {
        let result = update::install_tsf_dll_generation(
            &path_arg(&args[2]),
            &path_arg(&args[3]),
            &token_arg(&args[4])?,
        )?;
        println!(
            "tsf_installed=true\nold_renamed={}\nold_cleanup_pending={}\nold_cleanup_scheduled_for_reboot={}",
            bool_text(result.old_dll_renamed != 0),
            bool_text(result.old_cleanup_pending != 0),
            bool_text(result.old_cleanup_scheduled_for_reboot != 0)
        );
        return Ok(0);
    }
    if args.len() == 3 && args[1] == "--cleanup-old-tsf-dlls" {
        let pending = update::cleanup_old_tsf_dlls(&path_arg(&args[2]))?;
        println!("pending_old_tsf_dlls={}", pending.len());
        return Ok(if pending.is_empty() { 0 } else { 3 });
    }
    if args.len() == 6 && args[1] == "--activate-runtime-generation" {
        let result = update::install_runtime_generation(
            &path_arg(&args[2]),
            &path_arg(&args[3]),
            &token_arg(&args[4])?,
            &token_arg(&args[5])?,
        )?;
        println!(
            "runtime_installed={}\ntsf_x64_installed={}\ntsf_x86_installed={}\ncurrent_published={}",
            bool_text(result.runtime_installed != 0),
            bool_text(result.tsf_x64_installed != 0),
            bool_text(result.tsf_x86_installed != 0),
            bool_text(result.current_published != 0)
        );
        return Ok(0);
    }
    if args.len() == 5 && args[1] == "--publish-generation" {
        update::publish_runtime_generation(
            &path_arg(&args[2]),
            &token_arg(&args[3])?,
            &token_arg(&args[4])?,
        )?;
        return Ok(0);
    }
    if args.len() == 3 && args[1] == "--generation-status" {
        let state = update::read_runtime_generation_state(&path_arg(&args[2]))?;
        println!(
            "current_generation={}\nprevious_generation={}\nbuild_id={}",
            ascii_buffer(&state.current_generation)?,
            ascii_buffer(&state.previous_generation)?,
            ascii_buffer(&state.build_id)?
        );
        return Ok(0);
    }
    if args.len() == 4 && args[1] == "--status" {
        let state = update::read_deployment_state(&path_arg(&args[2]), &token_arg(&args[3])?)?;
        println!(
            "channel={}\nupdate_owner={}\ncurrent={}\nprevious={}\npending={}\nhealthy={}",
            ascii_buffer(&state.channel)?,
            update::owner_name(update_owner_from_raw(state.owner)?),
            ascii_buffer(&state.current)?,
            ascii_buffer(&state.previous)?,
            ascii_buffer(&state.pending)?,
            bool_text(state.healthy != 0)
        );
        return Ok(0);
    }
    Ok(usage())
}

fn usage() -> i32 {
    eprintln!(
        "Usage:\n  fcitx5-updater --record-owner ROOT OWNER\n  fcitx5-updater --activate ARCHIVE ROOT TRANSACTION KEYRING CHANNEL\n  fcitx5-updater --health ROOT CHANNEL\n  fcitx5-updater --rollback ROOT CHANNEL CORE_PACKAGE_ID KEYRING\n  fcitx5-updater --cleanup-previous ROOT CHANNEL CORE_PACKAGE_ID\n  fcitx5-updater --install-tsf-dll REGISTERED_DLL NEW_DLL GENERATION\n  fcitx5-updater --cleanup-old-tsf-dlls TSF_ARCH_DIR\n  fcitx5-updater --activate-runtime-generation ROOT VERIFIED_PAYLOAD GENERATION BUILD_ID\n  fcitx5-updater --publish-generation ROOT GENERATION BUILD_ID\n  fcitx5-updater --generation-status ROOT\n  fcitx5-updater --status ROOT CHANNEL"
    );
    1
}

fn path_arg(value: &OsString) -> PathBuf {
    PathBuf::from(value)
}

fn token_arg(value: &OsString) -> Result<String, Box<dyn Error>> {
    value
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "argument is not valid Unicode".into())
}

fn ascii_buffer(buffer: &[u8]) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(
        buffer
            .iter()
            .copied()
            .take_while(|byte| *byte != 0)
            .collect(),
    )?)
}

fn update_owner_from_raw(value: u32) -> Result<update::Fcitx5UpdateOwner, Box<dyn Error>> {
    match value {
        0 => Ok(update::Fcitx5UpdateOwner::Builtin),
        1 => Ok(update::Fcitx5UpdateOwner::Chocolatey),
        2 => Ok(update::Fcitx5UpdateOwner::Winget),
        3 => Ok(update::Fcitx5UpdateOwner::Enterprise),
        4 => Ok(update::Fcitx5UpdateOwner::Manual),
        _ => Err("deployment owner is invalid".into()),
    }
}

fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
