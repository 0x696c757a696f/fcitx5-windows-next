#![forbid(unsafe_code)]

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn resource_architecture() -> &'static str {
    match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86") => "x86",
        Ok("aarch64") => "arm64",
        _ => "x64",
    }
}

fn resource_compiler() -> Option<PathBuf> {
    if let Some(configured) = env::var_os("RC").map(PathBuf::from) {
        if configured.is_file() {
            return Some(configured);
        }
    }
    for directory in env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default()
    {
        for executable in ["llvm-rc.exe", "rc.exe"] {
            let candidate = directory.join(executable);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let sdk = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut candidates = std::fs::read_dir(sdk)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join(resource_architecture()).join("rc.exe"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.pop()
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let repository = manifest
        .parent()
        .and_then(Path::parent)
        .expect("repository root");
    let resources = repository.join("resources").join("windows");
    let script = resources.join("launcher.rc");
    let header = resources.join("resource.h");
    let icons = repository.join("resources").join("icons");
    println!("cargo:rerun-if-changed={}", script.display());
    println!("cargo:rerun-if-changed={}", header.display());
    for icon in ["fcitx5.ico", "fcitx5-paused.ico", "fcitx5-error.ico"] {
        println!("cargo:rerun-if-changed={}", icons.join(icon).display());
    }

    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("build output")).join("fcitx5-launcher.res");
    let compiler = resource_compiler().expect("rc.exe or llvm-rc.exe is required");
    let status = Command::new(&compiler)
        .arg("/nologo")
        .arg(format!("/I{}", resources.display()))
        .arg(format!("/fo{}", output.display()))
        .arg(&script)
        .status()
        .expect("run resource compiler");
    assert!(status.success(), "resource compiler failed");
    println!(
        "cargo:rustc-link-arg-bin=fcitx5-launcher={}",
        output.display()
    );
}
