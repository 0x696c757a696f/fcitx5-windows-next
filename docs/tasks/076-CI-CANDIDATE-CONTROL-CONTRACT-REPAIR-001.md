# Task 076 - Candidate and Control CI contract repair

**Task ID:** `CI-CANDIDATE-CONTROL-CONTRACT-REPAIR-001`
**Mode:** CHANGE / CI-REGRESSION
**Prerequisite:** 075; 074 and `REL-01` external evidence may remain pending.

Repair the reproducible CTest failures found after 075 without restoring obsolete C++ product
authority: update stale Candidate source-contract markers to current Rust-owned behavior and
replace the stopped-service test's removed `--set-presentation-font` invocation with the current
typed Rust Config/Control contract.

Use PowerShell 7 at `D:\Program Files\PowerShell\7\pwsh.exe`.
Toolchain root: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains`.
Fast tools: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast`.
Pinned Cargo: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home\bin\cargo.exe`.
Any `CARGO_TARGET_DIR` must be inside the implementation worktree.

New product code and behavior tests are Rust. C++ is limited to final mixed E2E, source gates,
and native adapters. Do not reintroduce `set-presentation-font`, duplicate Candidate state, or a
second Config authority. Do not claim online CI, real-host, signing, UAC, Accessibility, Win7,
offline, or low-resource evidence from local tests.

Acceptance: correct-path x64/x86 configure/build/test routes pass for `source-contract` and the
stopped-service Control/package contract, Rust Candidate/Control tests remain green, and exact
paths/results/remaining external limitations are recorded in `docs/tasks/status.md`.
