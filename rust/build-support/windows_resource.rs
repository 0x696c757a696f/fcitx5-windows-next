#![forbid(unsafe_code)]

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn target_architecture() -> &'static str {
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
        .map(|entry| entry.path().join(target_architecture()).join("rc.exe"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.pop()
}

fn watch(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}

fn compile(resource_script: &Path, include_dir: &Path, output_name: &str) -> PathBuf {
    watch(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../build-support/windows_resource.rs")
            .as_path(),
    );
    watch(resource_script);
    watch(include_dir.join("resource.h").as_path());
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("build output")).join(output_name);
    let compiler = resource_compiler().expect("rc.exe or llvm-rc.exe is required");
    let status = Command::new(&compiler)
        .arg("/nologo")
        .arg(format!("/I{}", include_dir.display()))
        .arg(format!("/fo{}", output.display()))
        .arg(resource_script)
        .status()
        .expect("run resource compiler");
    assert!(
        status.success(),
        "resource compiler {} failed for {}",
        compiler.display(),
        resource_script.display()
    );
    output
}

#[allow(dead_code)]
pub fn compile_for_bins(
    resource_script: &Path,
    include_dir: &Path,
    icons: &[&Path],
    output_name: &str,
    binaries: &[&str],
) {
    if !cfg!(target_os = "windows") {
        return;
    }
    for icon in icons {
        watch(icon);
    }
    let output = compile(resource_script, include_dir, output_name);
    for binary in binaries {
        println!("cargo:rustc-link-arg-bin={binary}={}", output.display());
    }
}

#[allow(dead_code)]
pub fn compile_for_cdylib(
    resource_script: &Path,
    include_dir: &Path,
    icons: &[&Path],
    output_name: &str,
) {
    if !cfg!(target_os = "windows") {
        return;
    }
    for icon in icons {
        watch(icon);
    }
    let output = compile(resource_script, include_dir, output_name);
    println!("cargo:rustc-link-arg={}", output.display());
}
