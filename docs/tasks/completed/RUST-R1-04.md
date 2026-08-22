# RUST-R1-04 Rust provider policy/runner

**State:** COMPLETED
**Canonical task:** `docs/tasks/rust/R1-04-PROVIDER.md`

## Completed slice

- Rust `fcitx5-provider.exe` is now authoritative.
- The old C++ provider runner/policy implementation was removed.
- Provider policy/runner coverage moved into Rust differential tests and CTest artifact smoke.

## Preserved boundary

- No arbitrary shell proxy.
- Pinned System32 `cmd.exe` entry for Plum only.
- Bounded absolute provider/user/cache paths.
- Pinned `rime-install.bat` entry point.
- Official/unverified trust classification with explicit `--allow-unverified`.
- Non-elevated provider boundary.
- Job-contained suspended launch with timeout termination.
- Non-zero child exit code propagation.
- No live key/preedit/candidate/commit content is routed to provider paths.

