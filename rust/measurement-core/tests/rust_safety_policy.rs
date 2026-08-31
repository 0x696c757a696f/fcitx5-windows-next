#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const UNSAFE_EXCEPTIONS: &[&str] = &[
    "rust/candidate-core/src/candidate_abi.rs",
    "rust/candidate-core/src/bin/candidate_poc.rs",
    "rust/candidate-core/src/lib.rs",
    "rust/config-core/src/config_snapshot_abi.rs",
    "rust/config-core/src/lib.rs",
    "rust/config-core/tests/config_snapshot_abi_contract.rs",
    "rust/config-poc/src/bin/fcitx5_config.rs",
    "rust/config-poc/src/main.rs",
    "rust/config-poc/src/win32_window_smoke.rs",
    "rust/config-qa/src/main.rs",
    "rust/control-core/src/lib.rs",
    "rust/engine-core/src/capi.rs",
    "rust/engine-core/src/capi_tests.rs",
    "rust/engine-core/src/lib.rs",
    "rust/launcher-core/src/lib.rs",
    "rust/package-core/src/bootstrap_main.rs",
    "rust/package-core/src/deployer_main.rs",
    "rust/package-core/src/downloader_main.rs",
    "rust/package-core/src/lib.rs",
    "rust/process-execution-core/src/lib.rs",
    "rust/protocol-core/src/capi.rs",
    "rust/protocol-core/src/capi_tests.rs",
    "rust/protocol-core/src/lib.rs",
    "rust/protocol-core/src/tests.rs",
    "rust/register-core/src/lib.rs",
    "rust/release-pqc-signer/src/main.rs",
    "rust/tsf-poc/src/lib.rs",
    "rust/tsf-support-core/src/lib.rs",
    "rust/windows-common-core/src/lib.rs",
];
const UNSAFE_ALLOW_ATTRIBUTE: &str = concat!("allow", "(unsafe_code)");

fn rust_files(directory: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("Rust source directory must be readable") {
        let path = entry.expect("Rust source entry must be readable").path();
        if path.is_dir() {
            rust_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}

#[test]
fn every_rust_file_is_safe_by_default_or_an_explicit_boundary_exception() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("measurement-core must remain under rust/");
    let exceptions: BTreeSet<&str> = UNSAFE_EXCEPTIONS.iter().copied().collect();
    let mut files = Vec::new();
    rust_files(&repository.join("rust"), &mut files);

    let mut seen_exceptions = BTreeSet::new();
    for file in files {
        let relative = file
            .strip_prefix(repository)
            .expect("Rust source must be inside the repository")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&file).expect("Rust source must be UTF-8");
        assert!(
            !source.contains(UNSAFE_ALLOW_ATTRIBUTE),
            "Rust source must not allow unsafe code: {relative}"
        );
        if exceptions.contains(relative.as_str()) {
            seen_exceptions.insert(relative.clone());
            assert!(
                source.contains("#![deny(unsafe_op_in_unsafe_fn)]"),
                "unsafe boundary lacks #![deny(unsafe_op_in_unsafe_fn)]: {relative}"
            );
        } else {
            assert!(
                source.contains("#![forbid(unsafe_code)]"),
                "safe Rust source lacks #![forbid(unsafe_code)]: {relative}"
            );
        }
    }

    assert_eq!(
        seen_exceptions,
        exceptions.into_iter().map(str::to_owned).collect(),
        "unsafe exception allowlist contains a missing Rust source"
    );
}
