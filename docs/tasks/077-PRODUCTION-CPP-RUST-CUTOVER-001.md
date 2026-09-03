# Task 077 - Production C++ to Rust full cutover

**Task ID:** `PRODUCTION-CPP-RUST-CUTOVER-001`
**Mode:** CHANGE / RUST-MIGRATION / FULL-CUTOVER
**Prerequisite:** 071-076; `REL-01` external evidence remains parked and does not block code work.

## Goal and completion rule

Finish the production C++ migration as one indivisible task. Move every product-owned Windows,
protocol, Config, Candidate, control, package, launcher, and IPC policy authority to Rust and delete
the replaced C++ implementation in the same cutover.

077 may use multiple internal commits, phases, and subagents. It is not complete until the live
inventory has no `MIGRATE`, `DELETE`, `KEEP-TEMP`, conditional, or unclassified production row.
`MANUAL-PENDING` must not be used to hide unfinished source migration.

## Required environment for every implementation agent

- PowerShell 7: `D:\Program Files\PowerShell\7\pwsh.exe`
- Toolchain root: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains`
- Fast tools: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast`
- CMake/CTest: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast\cmake-3.31.8\cmake-3.31.8-windows-x86_64\bin`
- Ninja: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast\ninja-1.13.2\ninja.exe`
- LLVM: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast\llvm-22.1.8\clang+llvm-22.1.8-x86_64-pc-windows-msvc\bin`
- sccache: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast\sccache-0.17.0\sccache-v0.17.0-x86_64-pc-windows-msvc\sccache.exe`
- Pinned Cargo: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home\bin\cargo.exe`
- `RUSTUP_HOME`: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\rustup-home`
- `CARGO_HOME`: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home`
- `RUSTUP_TOOLCHAIN`: `1.98.0-x86_64-pc-windows-msvc`
- `RUSTUP_IO_THREADS`: `1`
- Python: `D:\Dev\pixi\envs\python\python.exe`
- `CARGO_TARGET_DIR` must be inside the agent's own worktree.
- The repository `.cargo/config.toml` points at the worktree-relative pinned sccache executable;
  agents still set `RUSTC_WRAPPER` to the absolute path above for auditable command lines.
- Use PowerShell syntax only on this Windows task. Do not use Bash quoting, heredocs, or wildcard
  search paths; expand wildcard directories before passing real paths to `rg`.

Each subagent reads `AGENTS.md`, this task, the live inventory, and only the relevant spec/source
slice; follows `rust-skills`; and does not commit or push. The root agent owns
integration, conflict resolution, validation, commits, and pushes.

## Permanent C++ boundary

Long-term C++ production code is limited to direct Fcitx object/addon adapters, a demonstrably
necessary thin Win32/COM/C-ABI seam with no product state or policy, and upstream native addon
integration. New product code and behavior tests are Rust. C++ tests remain only for direct Fcitx,
necessary Win32/COM/ABI, or final mixed-binary integration/E2E.

## TDD cutover sequence

Every internal vertical slice follows:

`public behavior/corpus -> RED Rust/public boundary test -> minimal Rust implementation -> x64/x86`
`differential or final mixed evidence -> shipping cutover -> delete old C++ authority -> source gate`.

No permanent old/new runtime selector or duplicate DTO truth is permitted.

## Internal phases

### 0. Freeze final seams

Freeze final flat ABIs in the existing Rust protocol, Engine, Config, package, Control, Candidate,
launcher, Windows-common, and process-execution owners. Do not add a second framework or crate when
an existing owner can be deepened.

### 1. Remove Config/package/control C++ authority

Route Config, theme, package, repository, CLI JSON, and process policy through Rust. Delete the
legacy WTL Config target and C++ Config model/parser, make the shipping Control process Rust-owned,
and remove the C++ package bridge after final mixed package corpus passes.

Deletion set: `src/config/app_main.cpp`, `config_model.h`, `config_parser.cpp`,
`src/control/control_main.cpp`, `src/package/package_core.cpp`, and `package_core.h`.

### 2. Close Engine product plane

Move Engine protocol decode/encode, request routing, presentation queue/policy, and mock fixture to
Rust. Retain only direct Fcitx object/event/key/addon access and the necessary native process/pipe
host. Rust must not hold `fcitx::*` pointers or emulate Fcitx inheritance/vtables.

Deletion set: `src/engine/presentation_publisher.*` and `mock_engine_main.cpp`; mixed Engine files
must be reduced by symbol according to the live ledger.

### 3. Close Candidate and launcher product shells

Candidate snapshot transport, model, presentation, layout, hit-test, config, accessibility, and
selection orchestration stay Rust-owned. Reduce `ui_main.cpp` to HWND/D2D/DWrite/device-loss/DPI
painting from a Rust render plan. Move launcher supervisor, CLI, command, state, and store ownership
to Rust; retain tray C++ only if it is a thin required native adapter.

Deletion set: `src/launcher/launcher_main.cpp`, `state_machine.h`, `state_store.h`, and any tray code
that is not a necessary native seam. The current Release `/WX` failure at the old Candidate local is
resolved by deleting its obsolete product-state path, not by restoring C++ authority.

### 4. Delete protocol/IPC bridges and prove closure

Delete `protocol/protocol.cpp` and `protocol.h` after direct Fcitx/native consumers use the Rust
flat ABI. Delete C++ pipe and launcher clients after Rust-owned processes cut over. Reduce peer,
identity, pipe-security, and UI/tray adapters to necessary mechanics only. Delete obsolete PCH and
native ML-DSA configuration when no remaining native adapter requires them.

One integration owner updates `CMakeLists.txt`, `native-engine/CMakeLists.txt`, source gates, build
scripts, and final inventory after all workstreams land.

## Win7 rule

Do not silently drop the declared Windows 7 lineage. Any Rust shipping binary replacing a Legacy
C++ product path requires the repository Win7 target/import/dependency PoC and recorded result.
Platform differences stay in minimal capability adapters; do not copy business logic into separate
`win7/` and `modern/` trees.

## Acceptance

- The live inventory enumerates every current project-owned production `.cpp/.h`; no unfinished or
  conditional row remains.
- Every retained C++ symbol maps to direct Fcitx, necessary Win32/COM/C ABI, or upstream addon code
  and is guarded against renewed product ownership.
- Replaced product code, duplicate DTO/state, obsolete targets, and obsolete test authority are
  deleted with no permanent dual stack.
- Rust public behavior/unit/property/fault/fuzz/performance coverage owns migrated semantics.
- x64/x86 Cargo test, clippy, and fmt pass for every affected crate.
- x64/x86 target-clean CMake/Ninja builds and affected CTest routes pass.
- Release package builds from the final HEAD and its manifest records that HEAD/version/channel.
- Text, dependency, license, and source-structure checks plus `git diff --check` pass.
- Required real-host visual/Accessibility/Win7 evidence is recorded honestly; if a cutover gate
  itself needs unavailable evidence, 077 remains active rather than archiving partial migration.

Do not copy 077 into `completed/` or select `REL-01` until all source-cutover acceptance items pass.
