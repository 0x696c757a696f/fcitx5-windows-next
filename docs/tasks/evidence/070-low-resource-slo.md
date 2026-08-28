# Task 070 initial SLO evidence

`fcitx5-low-resource-harness` is a bounded Rust fixture. It uses a monotonic
fake clock and fixed operation counts, so x64 and x86 runs are comparable and
do not depend on sleeps, retries, user directories, input text, timestamps,
paths, or network access. The output contains aggregate latency, memory, and
operation counts only. Core is reported separately from the TSF shim,
Candidate UI, and each heavy activation workload (Rime, Mozc, and Lua).

The checked-in values and schema are initial SLO calibration inputs, not claims
about the current machine or product readiness. Real 2-core/4-GB, low-storage,
offline/constrained-network, accessibility, Windows 7, and signing/UAC
evidence remain `MANUAL-PENDING`; low-resource and accessibility remain release
gates.

Run with the pinned Cargo and a worktree-local `CARGO_TARGET_DIR`:

```text
cargo run --locked -q -p fcitx5-measurement-core --target x86_64-pc-windows-msvc
cargo run --locked -q -p fcitx5-measurement-core --target i686-pc-windows-msvc
```

The machine-specific fixture output is intentionally not committed as product
performance evidence. See `070-low-resource-slo.schema.json` for the bounded
report contract.
