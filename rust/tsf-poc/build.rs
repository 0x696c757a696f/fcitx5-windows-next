#![forbid(unsafe_code)]

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn target_rc_arch() -> &'static str {
    match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86") => "x86",
        Ok("aarch64") => "arm64",
        _ => "x64",
    }
}

fn path_entries() -> Vec<PathBuf> {
    env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default()
}

fn newest_windows_sdk_rc(arch: &str) -> Option<PathBuf> {
    let sdk_bin = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut candidates = std::fs::read_dir(sdk_bin)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join(arch).join("rc.exe"))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.pop()
}

fn find_resource_compiler() -> Option<PathBuf> {
    if let Some(configured) = env::var_os("RC").map(PathBuf::from) {
        if configured.exists() {
            return Some(configured);
        }
    }
    for directory in path_entries() {
        for executable in ["llvm-rc.exe", "rc.exe"] {
            let candidate = directory.join(executable);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    newest_windows_sdk_rc(target_rc_arch())
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("repo root");
    let resource_dir = repo_root.join("resources").join("windows");
    let resource_script = resource_dir.join("tsf.rc");
    let resource_header = resource_dir.join("resource.h");
    let icon = repo_root
        .join("resources")
        .join("icons")
        .join("fcitx5-tsf.ico");
    println!("cargo:rerun-if-changed={}", resource_script.display());
    println!("cargo:rerun-if-changed={}", resource_header.display());
    println!("cargo:rerun-if-changed={}", icon.display());

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("out dir"));
    let compiled_resource = out_dir.join("fcitx5-tsf.res");
    let Some(resource_compiler) = find_resource_compiler() else {
        panic!("rc.exe or llvm-rc.exe is required to embed the TSF icon resource");
    };
    let status = Command::new(&resource_compiler)
        .arg("/nologo")
        .arg(format!("/I{}", resource_dir.display()))
        .arg(format!("/fo{}", compiled_resource.display()))
        .arg(&resource_script)
        .status()
        .expect("run resource compiler");
    if !status.success() {
        panic!(
            "resource compiler {} failed for {}",
            resource_compiler.display(),
            resource_script.display()
        );
    }
    println!("cargo:rustc-link-arg={}", compiled_resource.display());
}
