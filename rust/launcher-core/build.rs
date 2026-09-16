#![forbid(unsafe_code)]

#[path = "../build-support/windows_resource.rs"]
mod windows_resource;

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let repository = manifest
        .parent()
        .and_then(Path::parent)
        .expect("repository root");
    let resources = repository.join("resources").join("windows");
    let script = resources.join("launcher.rc");
    let icons = repository.join("resources").join("icons");
    let icon_paths = [
        icons.join("fcitx5.ico"),
        icons.join("fcitx5-paused.ico"),
        icons.join("fcitx5-error.ico"),
    ];
    let watched_icons = icon_paths.iter().map(PathBuf::as_path).collect::<Vec<_>>();
    windows_resource::compile_for_bins(
        &script,
        &resources,
        &watched_icons,
        "fcitx5-launcher.res",
        &["fcitx5-launcher"],
    );
}
