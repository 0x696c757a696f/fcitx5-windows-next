# Completed Task — ENGINE-E4-TRANSPORT-FRAMING-001

**Mode:** ENGINE-RUST-MIGRATION
**State:** COMPLETED / E4-SERVER-PIPE-TRANSPORT-RUST-GREEN
**Task ID:** `051-ENGINE-E4-TRANSPORT-FRAMING-001`
**Prerequisite:** `050-ENGINE-IDLE-PACKAGE-GATE-050` package gate green

## Goal

Continue Engine E4 by shrinking the remaining production Engine server-side named-pipe
transport/framing glue toward the existing Rust-owned `windows-common-core` pipe primitives.

## Result

Production `src/engine/fcitx_engine_main.cpp` delegates server-side overlapped pipe connect and
byte transfer to Rust-owned `windows-common-core` C ABI helpers:

- `fcitx5_windows_common_deadline_after_milliseconds`
- `fcitx5_windows_common_pipe_connect_client`
- `fcitx5_windows_common_pipe_transfer_with_stop`

The C++ Engine process shell still owns the Windows server loop and direct Fcitx adapter dispatch,
but it no longer owns duplicate production `ConnectNamedPipe`/`ReadFile`/`WriteFile` overlapped
loops.

## Validation

- x64/x86 `cargo test --locked -p fcitx5-windows-common-core --target ... pipe_`
- native-engine `cmake --build out\build\native-engine --target fcitx5_engine --parallel`
- x64/x86 Debug CTest `engine-core-contract|source-contract`
- direct x64/x86 Release `fcitx5_engine_integration_test.exe` baseline against staged
  `out\stage\fcitx5\bin\fcitx5-engine.exe`

## Remaining blockers

`RELEASE-01` remains gated on production release assets, signing/key evidence, and required
real-host/manual compatibility evidence.
