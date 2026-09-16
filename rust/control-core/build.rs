#![forbid(unsafe_code)]

#[path = "../build-support/windows_resource.rs"]
mod windows_resource;

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let repository = manifest
        .parent()
        .and_then(|path| path.parent())
        .expect("repository root")
        .to_path_buf();
    let resources = repository.join("resources").join("windows");
    let icon = repository
        .join("resources")
        .join("icons")
        .join("fcitx5.ico");
    windows_resource::compile_for_bins(
        &resources.join("app.rc"),
        &resources,
        &[icon.as_path()],
        "fcitx5-control.res",
        &["fcitx5-control"],
    );
}
