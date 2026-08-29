# Rust unsafe exception ledger — 2026-08-29

Task 077 makes `#![forbid(unsafe_code)]` the default for every Rust source file. The files below are
the complete allowlist. An exception permits only the named boundary; product/domain logic remains
Safe Rust. Crate roots use `#![deny(unsafe_op_in_unsafe_fn)]`; child modules inherit that lint.

| File | Boundary owner / reason |
|---|---|
| `rust/windows-common-core/src/lib.rs` | Win32 identity, handles, registry, process, text, and security ABI |
| `rust/tsf-poc/src/lib.rs` | Shipping COM/TSF DLL exports, vtables, pointers, and Windows callbacks |
| `rust/tsf-support-core/src/lib.rs` | Narrow TSF activation-guard Win32 registry ABI |
| `rust/register-core/src/lib.rs` | COM registration and Windows registry ABI |
| `rust/process-execution-core/src/lib.rs` | Win32 process, job, pipe, and handle lifecycle ABI |
| `rust/release-pqc-signer/src/main.rs` | Audited native ML-DSA verifier/signer FFI |
| `rust/package-core/src/lib.rs` | Package C ABI plus audited native archive/crypto calls |
| `rust/package-core/src/bootstrap_main.rs` | Win32 bootstrap executable adapter |
| `rust/package-core/src/deployer_main.rs` | Win32 deployment/elevation executable adapter |
| `rust/package-core/src/downloader_main.rs` | WinHTTP/Win32 downloader executable adapter |
| `rust/protocol-core/src/lib.rs` | Flat protocol C ABI crate root |
| `rust/protocol-core/src/capi.rs` | Protocol pointer/slice C ABI implementation |
| `rust/protocol-core/src/capi_tests.rs` | Protocol C ABI pointer-contract tests |
| `rust/protocol-core/src/tests.rs` | Tests that directly exercise the unsafe protocol ABI |
| `rust/engine-core/src/lib.rs` | Engine flat C ABI crate root; safe product modules are individually forbidden |
| `rust/engine-core/src/capi.rs` | Engine pointer/slice C ABI implementation |
| `rust/engine-core/src/capi_tests.rs` | Engine C ABI pointer-contract tests |
| `rust/control-core/src/lib.rs` | Control C ABI and necessary Win32 registry/file adapter |
| `rust/launcher-core/src/lib.rs` | Launcher C ABI plus minimal Win32 file/time adapter |
| `rust/candidate-core/src/lib.rs` | Candidate flat C ABI and native render-plan pointer adapter |
| `rust/candidate-core/src/bin/candidate_poc.rs` | Native Win32/DWrite visual evidence host |
| `rust/config-qa/src/main.rs` | Native Win32/DWrite Settings QA evidence host |
| `rust/config-poc/src/main.rs` | Shipping WindUI/Win32 Settings host callbacks and native adapter |
| `rust/config-poc/src/bin/fcitx5_config_rust.rs` | Shipping Settings entry includes the exception implementation above |

The authoritative machine-readable allowlist is
`rust/measurement-core/tests/rust_safety_policy.rs`. Adding any Rust file without either
`#![forbid(unsafe_code)]` or an explicit reviewed row fails that test.
