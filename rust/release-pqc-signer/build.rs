use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let repo_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf();
    let mldsa_root = env::var_os("FCITX_MLDSA_NATIVE_SOURCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("out/toolchains/mldsa-native-2.0.0/mldsa-native-2.0.0"));
    let source = mldsa_root.join("mldsa/mldsa_native.c");
    let header = mldsa_root.join("mldsa/mldsa_native.h");
    if !source.is_file() || !header.is_file() {
        panic!("pinned mldsa-native signing source is missing; run tools/prepare-package-dependencies.ps1");
    }
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("build output dir"));
    let config = out_dir.join("fcitx5_mldsa65_release_sign_config.h");
    fs::write(
        &config,
        "#pragma once\n\n#define MLD_CONFIG_PARAMETER_SET 65\n#define MLD_CONFIG_NAMESPACE_PREFIX fcitx5_mldsa65_release_sign\n#define MLD_CONFIG_NO_ASM\n",
    )
    .expect("release signing config should write");
    println!("cargo:rerun-if-env-changed=FCITX_MLDSA_NATIVE_SOURCE_DIR");
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rustc-link-lib=bcrypt");

    cc::Build::new()
        .file(source)
        .include(&mldsa_root)
        .include(&out_dir)
        .define(
            "MLD_CONFIG_FILE",
            Some("\"fcitx5_mldsa65_release_sign_config.h\""),
        )
        .define("MLD_CONFIG_NO_ASM", None)
        .warnings(false)
        .compile("fcitx5_mldsa65_release_sign");
}
