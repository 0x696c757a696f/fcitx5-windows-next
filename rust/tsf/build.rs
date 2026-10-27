#![forbid(unsafe_code)]

#[path = "../build-support/windows_resource.rs"]
mod windows_resource;

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("repo root");
    let resource_dir = repo_root.join("resources").join("windows");
    let resource_script = resource_dir.join("tsf.rc");
    let icon = repo_root
        .join("resources")
        .join("icons")
        .join("fcitx5-tsf.ico");
    let watched_icons = [icon.as_path()];
    // The DLL loads into every app with a text field, games included. Link
    // the VC runtime statically and the Universal CRT from the OS, so the DLL
    // needs no Visual C++ redistributable and never binds to an older
    // vcruntime140.dll that an app ships next to its executable.
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        for arg in [
            "/NODEFAULTLIB:msvcrt.lib",
            "/NODEFAULTLIB:vcruntime.lib",
            "/NODEFAULTLIB:libucrt.lib",
            "/DEFAULTLIB:libcmt.lib",
            "/DEFAULTLIB:libvcruntime.lib",
            "/DEFAULTLIB:ucrt.lib",
        ] {
            println!("cargo:rustc-cdylib-link-arg={arg}");
        }
    }
    windows_resource::compile_for_cdylib(
        &resource_script,
        &resource_dir,
        &watched_icons,
        "fcitx5-tsf.res",
    );
}
