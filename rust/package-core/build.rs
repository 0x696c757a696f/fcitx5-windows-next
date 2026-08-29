#![forbid(unsafe_code)]

use std::env;
use std::path::PathBuf;

fn main() {
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=bcrypt");
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=advapi32");
    }

    let repo_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf();
    let mldsa_root = env::var_os("FCITX_MLDSA_NATIVE_SOURCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("out/toolchains/mldsa-native-2.0.0/mldsa-native-2.0.0"));
    let miniz_root = env::var_os("FCITX_MINIZ_SOURCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("out/toolchains/miniz-3.1.2"));
    let source = mldsa_root.join("mldsa/mldsa_native.c");
    let header = mldsa_root.join("mldsa/mldsa_native.h");
    let config = repo_root.join("src/package/fcitx5_mldsa65_config.h");
    if !source.is_file() || !header.is_file() || !config.is_file() {
        panic!(
            "pinned mldsa-native verify source is missing; run tools/prepare-package-dependencies.ps1"
        );
    }
    println!("cargo:rerun-if-env-changed=FCITX_MLDSA_NATIVE_SOURCE_DIR");
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rerun-if-changed={}", config.display());

    cc::Build::new()
        .file(source)
        .include(&mldsa_root)
        .include(repo_root.join("src/package"))
        .define("MLD_CONFIG_FILE", Some("\"fcitx5_mldsa65_config.h\""))
        .define("MLD_CONFIG_NO_ASM", None)
        .warnings(false)
        .compile("fcitx5_mldsa65_verify");

    let miniz_source = miniz_root.join("miniz.c");
    let miniz_header = miniz_root.join("miniz.h");
    let miniz_wrapper = repo_root.join("rust/package-core/c/miniz_archive.c");
    if !miniz_source.is_file() || !miniz_header.is_file() || !miniz_wrapper.is_file() {
        panic!(
            "pinned miniz archive source is missing; run tools/prepare-package-dependencies.ps1"
        );
    }
    println!("cargo:rerun-if-env-changed=FCITX_MINIZ_SOURCE_DIR");
    println!("cargo:rerun-if-changed={}", miniz_source.display());
    println!("cargo:rerun-if-changed={}", miniz_header.display());
    println!("cargo:rerun-if-changed={}", miniz_wrapper.display());

    cc::Build::new()
        .file(miniz_source)
        .file(miniz_wrapper)
        .include(miniz_root)
        .define("MINIZ_NO_ZLIB_APIS", None)
        .warnings(false)
        .compile("fcitx5_miniz_archive");
}
