# Current Truth Snapshot

Date: 2026-08-25 (updated after CONFIG-RUST-CUTOVER-001 side-by-side Rust Settings target)

HEAD recorded at snapshot refresh: `009eb0f7ed2f6d4601c46c9a3537fa6adef35447`

Working tree at snapshot refresh: clean after archiving `CONFIG-UX-009`.

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
- New product-owned Windows code defaults to Rust. Already Rust-owned components must not regress
  to C++ because older audit text described a C++ baseline. C++ is reserved for direct
  Fcitx-facing Engine object manipulation. Remaining native code is temporary adapter/host code that
  must delegate product semantics to Rust and be retired once equivalent regression, accessibility,
  DPI, localization, visual, and package evidence exists.

## Language Map

| Surface | Current state | Direction |
|---|---|---|
| TSF | Shipping Rust target is packaged for x64/x86; package cold-start Notepad candidate UI and `nihao + Space => 你好` smokes are green; real-host matrix remains pending | Rust product component gated by host evidence |
| Engine | C++ Fcitx runtime owns direct Fcitx objects; product protocol/ledger/event decisions/session/snapshot/pending-state policy are Rust-owned | Continue shrinking toward Rust Engine Product Core + thin C++ Fcitx adapter |
| Candidate | Rust candidate-core owns model/layout/interaction; C++ UI window/renderer remains | Continue Rust authority and shrink adapter |
| Config | C++ WTL/Win32 shell remains only as a temporary shipping adapter; `CONFIG-UX-009` froze the Settings UX/theme/preview/package/localization/no-overlap corpus; Rust config PoC/control/theme operation owners exist; side-by-side `fcitx5-config-rust.exe` now builds against that corpus | Current task: move shipping `fcitx5-config.exe` to Rust through `CONFIG-RUST-CUTOVER-001` cutover slices |
| Launcher | Rust launcher-core owns state/path/tray/command/frame policy; C++ shell remains | Continue Rust cutover |
| Control | Rust control/package/process-exec cores linked; C++ command shell remains | Continue Rust cutover |
| Package/provider/downloader/updater/deployer | Package/provider/deployer/updater/downloader Rust CLIs and Rust package-core are wired; adapters remain where needed | Rust authority |
| Register/bootstrap | Rust policy/CLI work exists; Windows registry/elevation/export adapter remains | Thin adapter + Rust policy where useful |

## PoC And Migration State

| PoC / migration lane | State | Exit condition |
|---|---|---|
| Rust TSF | `SHIPPING-AUTOMATED-GREEN` / real-host matrix pending | Full host matrix evidence before release readiness |
| Rust Config | `CUTOVER-AUTHORIZED / UX-CORPUS-FROZEN / SIDE-BY-SIDE-RUST-SETTINGS-TARGET-GREEN` | Run side-by-side differential/visual/accessibility/package evidence, then cut over `fcitx5-config.exe` and delete the old C++ WTL shell |
| Candidate Rust core | `SHIPPING-DOMAIN` | Remove duplicated C++ validation/state, preserve renderer evidence, add candidate-action/upstream alignment |
| Rust package/update/control/launcher cores | `SHIPPING-DOMAIN` where already cut over, `MIGRATION-CANDIDATE` where shell remains | Delete replaced C++ authoritative implementation and keep adapter thin |
| Engine E1 `protocol-core` | `CUTOVER-GREEN` (Rust authoritative; C++ is a thin marshalling adapter) | Delete the old C++ codec internals (done); keep `protocol.h` API and call sites unchanged (done); future FCW4 wire changes must regenerate `protocol_wire_golden.inc` from the pre-change codec |
| Engine E2 `engine-core` ledger | `CUTOVER-GREEN` (Rust authoritative; ledger + carets/popupAllowed/selectedOverride/inputMethodOverridden cut over, C++ maps deleted) | — |
| Engine E3 Event→Action | `CUTOVER-GREEN` (unified `handle_key_event` entry; 4 product decisions Rust-owned, `processKey` executes the returned decision) | — |
| Engine E4 IPC scope | `PARTIAL` (engine epoch, request ordering, key deadline policy, and per-connection session state are Rust-owned; transport/framing primitives are partially prepared in `windows-common-core`) | Remaining E4: server-side transport/framing consolidation; `Generation` stays release-platform scope |
| Engine E5 snapshot/status canonicalization | `CUTOVER-GREEN` (canonicalization + typed `EngineSnapshot` DTO limits + `pendingStates` store Rust-owned; `collectResult`/`selectCandidate`/`takePendingState` apply them) | — |
| Engine E6 C++ product-state deletion | `CUTOVER-GREEN` (`candidate_navigation.h` + unit test deleted; scroll label offset Rust-owned; only Fcitx adapter + Windows process shell remain) | — |

## Current Red Lights

- Real-host matrix for Rust TSF remains manual-pending.
- Package cold-start candidate UI is green on local Notepad x64: final package stage `D:\Documents\GitHub\fcitx5-windows-next\out\package\stage-389ce50e5e5b4981ae3527bc18053fc6\Fcitx5` contains rebuilt x64/x86 Rust TSF DLLs; `--candidate-window` smoke sees `candidates=128 visibility=1`; `nihao + Space` commits `你好`.
- Strict direct clippy with `-D warnings` still hits existing `too_many_arguments` helpers in `rust/windows-common-core`; adjusted clippy with that existing lint allowed is green.
- Cargo registry crates are checked against `third_party/dependencies.json` by name and version before dependency checks and SBOM generation. Advisory review for the declared dependency set remains an external process.
- Runtime security now has explicit `Win10` and `Win7` lanes. The modern `Win10` lane remains the default full PE/source audit; product networking is enforced through source-boundary scanning plus PE blocking for explicit HTTP/URL stacks because Rust-linked MSVC binaries import `WS2_32.dll` through Rust std even without product network code. The legacy `Win7` lane is expected to stay red until the launcher/Rust runtime hard import of `GetSystemTimePreciseAsFileTime` is removed or a separate legacy strategy is implemented.
- Engine product state is migrating to Rust: E1 protocol codec, E2 ledger, E3 event/action decisions, E4 epoch/request/session policy, and E5 snapshot/pending-state policy are cut over (`CUTOVER-GREEN` where recorded). The Fcitx adapter and remaining Windows process/transport shell shrink remain.
- Existing long-form specs may contain historical task text. ADR 0009, this snapshot, `docs/engine-boundary.md`, and `docs/tasks/rebaseline.md` control the current Fcitx/Rust boundary and task interpretation.
- The executable queue has been re-cleaned after the 2026-08-24 review: completed/current R3
  FUTURE-GATED duplicates are not active queue items. `CONFIG-UX-009` is completed and archived
  with staged-app QA evidence. Current product work is `CONFIG-RUST-CUTOVER-001` for the shipping
  Config Rust cutover.
- The TSF profile boundary is now frozen in `docs/tsf-profile-boundary.md`: Windows exposes only the single product profile `Fcitx5`; internal engines/addons remain Fcitx state; obsolete dynamic profile data is cleanup input only.
- `rust/protocol-core` is now the single authoritative FCW4 codec: `protocol/protocol.cpp` is a thin marshalling adapter over the C ABI (`protocol/protocol_ffi.h`, typed encode/decode + `decode_header` in `capi.rs`), and `protocol-differential-contract` pins the pre-cutover wire bytes via `tests/unit/protocol_wire_golden.inc` (19 samples). C++ `protocol.h` API and all call sites are unchanged; see `docs/fcitx-upstream-rebaseline-audit.md` for the Fcitx5 upstream baseline audit (fork is official `ebf24ddc` + 41 lines; all three Windows-local changes are not yet upstream; 1 of 6 patches applies clean to master).

## Next Five Code/Design Tasks

1. Execute `CONFIG-RUST-CUTOVER-001`: replace the shipping C++ WTL
   Config shell with Rust while preserving the frozen Settings behavior corpus.
2. Continue Engine E4 transport/framing consolidation from the prepared `windows-common-core`
   stop-aware pipe primitives, keeping direct Fcitx object ownership in the C++ adapter.
3. Prepare/run generation-drain, installer/UAC, plugin lifecycle, and host evidence before any
   release-readiness claim.
