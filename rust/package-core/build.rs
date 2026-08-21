use std::env;
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
}
