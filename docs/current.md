# Current Truth Snapshot

Date: 2026-08-24

HEAD recorded at snapshot start: `0584572fb71d0d5597395d5ebee5f87f3bd49856`

Working tree at snapshot start: clean.

## Shipping Architecture

```text
Windows host
  -> fcitx5-tsf.dll
  -> fcitx5-launcher.exe
  -> fcitx5-engine.exe
  -> fcitx5-ui.exe
```

- Windows exposes one product TSF profile: `Fcitx5`.
- TSF, UI, Config, Launcher, Control, package/update/provider/deployer are product-owned Windows surfaces and continue moving toward Rust authority.
- Engine is not a permanent all-C++ exception. Its direct Fcitx object adapter remains C++; its product protocol, state, validation, IPC, deadline/fail-open, generation, revision, snapshot and diagnostics logic should move to Rust.

## Language Map

| Surface | Current state | Direction |
|---|---|---|
| TSF | Shipping Rust target exists; real-host matrix remains pending | Rust product component gated by host evidence |
| Engine | C++ Fcitx runtime owns direct Fcitx objects and still owns product state maps | Split into Rust Engine Product Core + thin C++ Fcitx adapter |
| Candidate | Rust candidate-core owns model/layout/interaction; C++ UI window/renderer remains | Continue Rust authority and shrink adapter |
| Config | C++ WTL/Win32 shell; Rust config PoC and Rust text/process adapters exist | Rust product logic with native adapter or time-boxed product spike decision |
| Launcher | Rust launcher-core owns state/path/tray/command/frame policy; C++ shell remains | Continue Rust cutover |
| Control | Rust control/package/process-exec cores linked; C++ command shell remains | Continue Rust cutover |
| Package/provider/downloader/updater/deployer | Package/provider/deployer/updater/downloader Rust CLIs and Rust package-core are wired; adapters remain where needed | Rust authority |
| Register/bootstrap | Rust policy/CLI work exists; Windows registry/elevation/export adapter remains | Thin adapter + Rust policy where useful |

## PoC And Migration State

| PoC / migration lane | State | Exit condition |
|---|---|---|
| Rust TSF | `MIGRATION-CANDIDATE` / real-host matrix pending | Full key/focus/composition/UILess/bounded IPC parity, x86/x64, unload/refcount, host matrix |
| Rust Config | `MIGRATION-CANDIDATE` | Time-boxed product spike or explicit ADR choosing WTL+D2D vs Rust native Config |
| Candidate Rust core | `SHIPPING-DOMAIN` | Remove duplicated C++ validation/state, preserve renderer evidence, add candidate-action/upstream alignment |
| Rust package/update/control/launcher cores | `SHIPPING-DOMAIN` where already cut over, `MIGRATION-CANDIDATE` where shell remains | Delete replaced C++ authoritative implementation and keep adapter thin |

## Current Red Lights

- Real-host matrix for Rust TSF remains manual-pending.
- Strict direct clippy with `-D warnings` still hits existing `too_many_arguments` helpers in `rust/windows-common-core`; adjusted clippy with that existing lint allowed is green.
- A unified current Cargo + C++ dependency/license/SBOM inventory needs to stay in lockstep with every new Rust crate or Windows crate update.
- Engine product state is still mostly C++; the Fcitx adapter boundary is documented but not yet fully cut.
- Existing long-form specs may contain historical task text. ADR 0009 and this snapshot control the current Fcitx/Rust boundary.

## Next Five Code/Design Tasks

1. Freeze Engine call graph and C++/Rust ABI in `docs/engine-boundary.md`.
2. Rebase the old task queue against current HEAD as `TODO`, `ALREADY-GREEN`, `PARTIAL`, `MANUAL-PENDING`, or `BLOCKED`.
3. Close Rust supply-chain inventory gaps: dependency check, license check, SBOM, Rust source policy and runtime-security Rust scanning.
4. Continue shrinking non-Engine product C++ adapters only where a Rust owner and regression evidence already exist.
5. Prepare real-host evidence for TSF generation draining and Rust TSF host matrix.
