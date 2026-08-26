# Current Truth Snapshot

Date: 2026-08-26 (updated after CONFIG-WINDUI-SETTINGS-SHELL-001)

HEAD recorded at snapshot refresh: `d1c87129d03b6460222534b130b016d32348aad2`

Working tree at snapshot refresh: clean after `CONFIG-WINDUI-SETTINGS-SHELL-001` validation,
commit, and push.

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
| Candidate | Rust candidate-core owns model/layout/interaction; C++ UI window/renderer remains | Continue Rust authority and shrink adapter; next renderer work must preserve configurable label/ordinal slots, selected-row/column label reveal, and stable text-column alignment |
| Config | Stage 2 Rust Config backend is shipping, and the default interactive `fcitx5-config.exe` window now opens the vendored `huanfeng/wind-ui-rust` Settings shell. The old C++ WTL/Win32 shell remains `fcitx5-config-legacy.exe` as an `EXCLUDE_FROM_ALL` regression baseline; the old Rust Win32/D2D preview host is QA-only behind `FCITX5_CONFIG_RUST_PREVIEW_STATE` or explicit smoke/test modes. | Do not claim release readiness or final Stage 4 completion until real Settings GUI persistence/plugin pages, DPI/dark-mode/keyboard/accessibility, candidate preview parity, Narrator/NVDA, and real Win7/Win10/Win11 evidence are green |
| Launcher | Rust launcher-core owns state/path/tray/command/frame policy; C++ shell remains | Continue Rust cutover |
| Control | Rust control/package/process-exec cores linked; C++ command shell remains | Continue Rust cutover |
| Package/provider/downloader/updater/deployer | Package/provider/deployer/updater/downloader Rust CLIs and Rust package-core are wired; adapters remain where needed | Rust authority |
| Register/bootstrap | Rust policy/CLI work exists; Windows registry/elevation/export adapter remains | Thin adapter + Rust policy where useful |

## PoC And Migration State

| PoC / migration lane | State | Exit condition |
|---|---|---|
| Rust TSF | `SHIPPING-AUTOMATED-GREEN` / real-host matrix pending | Full host matrix evidence before release readiness |
| Rust Config | `CUTOVER-AUTHORIZED / STAGE-1-CORE-GREEN / STAGE-2-BACKEND-SHIPPED-GREEN / WINDUI-DEFAULT-SHELL-GREEN / STAGE-4-REAL-HOST-EVIDENCE-PENDING` | Non-interactive Config paths are Rust-owned under the shipping `fcitx5-config.exe` name with no legacy C++ GUI fallback. The default GUI now uses the vendored `windui` Settings shell and has screenshot/source-contract evidence. Stage 4 still requires real interactive persistence/plugin-page/candidate-preview parity, DPI/dark-mode/keyboard/accessibility, and real Windows QA before “Config fully migrated to Rust” can be claimed. |
| Candidate Rust core | `SHIPPING-DOMAIN` | Remove duplicated C++ validation/state, preserve renderer evidence, add candidate-action/upstream alignment |
| Rust package/update/control/launcher cores | `SHIPPING-DOMAIN` where already cut over, `MIGRATION-CANDIDATE` where shell remains | Delete replaced C++ authoritative implementation and keep adapter thin |
| Engine E1 `protocol-core` | `CUTOVER-GREEN` (Rust authoritative; C++ is a thin marshalling adapter) | Delete the old C++ codec internals (done); keep `protocol.h` API and call sites unchanged (done); future FCW4 wire changes must regenerate `protocol_wire_golden.inc` from the pre-change codec |
| Engine E2 `engine-core` ledger | `CUTOVER-GREEN` (Rust authoritative; ledger + carets/popupAllowed/selectedOverride/inputMethodOverridden cut over, C++ maps deleted) | — |
| Engine E3 Event→Action | `CUTOVER-GREEN` (unified `handle_key_event` entry; 4 product decisions Rust-owned, `processKey` executes the returned decision) | — |
| Engine E4 IPC scope | `CUTOVER-GREEN FOR CURRENT E4 SCOPE` (engine epoch, request ordering, key deadline policy, per-connection session state, and production server-side pipe connect/byte transfer are Rust-owned) | `Generation` stays release-platform scope; direct Fcitx object dispatch remains in the C++ adapter |
| Engine E5 snapshot/status canonicalization | `CUTOVER-GREEN` (canonicalization + typed `EngineSnapshot` DTO limits + `pendingStates` store Rust-owned; `collectResult`/`selectCandidate`/`takePendingState` apply them) | — |
| Engine E6 C++ product-state deletion | `CUTOVER-GREEN` (`candidate_navigation.h` + unit test deleted; scroll label offset Rust-owned; only Fcitx adapter + Windows process shell remain) | — |

## Current Red Lights

- Real-host matrix for Rust TSF remains manual-pending.
- Full `tools/build.ps1 package -Architecture all -Configuration Release` passed locally after
  `ENGINE-IDLE-PACKAGE-GATE-050`. The tested package stage was
  `D:\Documents\GitHub\fcitx5-windows-next\out\package\stage-dee8db43e74d4793b0e4ecca2dc5a4e9\Fcitx5`;
  artifacts were emitted under `D:\Documents\GitHub\fcitx5-windows-next\out\package\artifacts`.
- Package cold-start candidate UI is green on local Notepad x64: final package stage `D:\Documents\GitHub\fcitx5-windows-next\out\package\stage-389ce50e5e5b4981ae3527bc18053fc6\Fcitx5` contains rebuilt x64/x86 Rust TSF DLLs; `--candidate-window` smoke sees `candidates=128 visibility=1`; `nihao + Space` commits `你好`.
- Strict direct clippy with `-D warnings` still hits existing `too_many_arguments` helpers in `rust/windows-common-core`; adjusted clippy with that existing lint allowed is green.
- Cargo registry crates are checked against `third_party/dependencies.json` by name and version before dependency checks and SBOM generation. Advisory review for the declared dependency set remains an external process.
- Runtime security now has explicit `Win10` and `Win7` lanes. The modern `Win10` lane remains the default full PE/source audit; product networking is enforced through source-boundary scanning plus PE blocking for explicit HTTP/URL stacks because Rust-linked MSVC binaries import `WS2_32.dll` through Rust std even without product network code. The legacy `Win7` lane is expected to stay red until the launcher/Rust runtime hard import of `GetSystemTimePreciseAsFileTime` is removed or a separate legacy strategy is implemented.
- Engine product state is migrating to Rust: E1 protocol codec, E2 ledger, E3 event/action decisions, E4 epoch/request/session/deadline plus production server pipe connect/transfer, and E5 snapshot/pending-state policy are cut over (`CUTOVER-GREEN` where recorded). The Fcitx adapter and remaining Windows process shell shrink remain.
- Existing long-form specs may contain historical task text. ADR 0009, this snapshot, `docs/engine-boundary.md`, and `docs/tasks/rebaseline.md` control the current Fcitx/Rust boundary and task interpretation.
- `fcitx-contrib/fcitx5-windows` is explicitly excluded from architecture authority. Current Engine,
  Package, Candidate, Config, and addon decisions use `fcitx/fcitx5` core plus each addon upstream
  for semantics; Windows ports are compatibility case studies only.
- WindInput remains a Settings/theme management discipline reference. The retained constraints are
  task-oriented common/advanced settings, typed theme resolve before render, no dead theme fields,
  Light/Dark token parity, unit-aware geometry, one renderer-backed candidate preview path,
  validated numeric inputs, and system-font-backed font selection. `huanfeng/wind-ui-rust` is now a
  vendored Rust Config GUI dependency, not merely a reference: `fcitx5-config.exe` no-argument launch
  uses a real `windui::App` Settings shell aligned with upstream `settings-input.png`.
- Candidate label/ordinal UX has been tightened: labels are user-configurable presentation, not
  input semantics; every horizontal/vertical/grid cell reserves a label slot computed from the
  widest resolved label in scope; labels right-align inside that slot; candidate text begins after a
  stable gap; selected-row/column/item reveal modes keep hidden labels occupying the same space so
  rows and columns do not shift when `1.`/custom labels appear.
- The executable queue has been re-cleaned after the 2026-08-24 review: completed/current R3
  FUTURE-GATED duplicates are not active queue items. Completed queue source files are removed from
  `docs/tasks/queue` after their task files are archived. The next local code-only queue item is
  `059-CANDIDATE-LABEL-SLOT-RUST-DRAWING-001`; `RELEASE-01` stays gated on external/manual evidence.
- Rust Config Stage 2 and the code-only default GUI shell cutover are green. `fcitx5_config_app`
  builds the Rust binary into the shipping `fcitx5-config.exe` product name; no-argument launch now
  opens the vendored `windui` Settings shell; `fcitx5_config_legacy_app` emits
  `fcitx5-config-legacy.exe` only as an `EXCLUDE_FROM_ALL` differential-test baseline; the old Rust
  Win32/D2D preview host is QA-only. Current state marker:
  `STAGE-2-BACKEND-SHIPPED-GREEN / WINDUI-DEFAULT-SHELL-GREEN / STAGE-4-REAL-HOST-EVIDENCE-PENDING`.
  Release notes may still say “Rust configuration backend is now shipping; interactive settings UI migration is still in progress”; they must not claim “Config fully migrated to Rust” until full
  Stage 4 manual/real-host evidence is green.
- The TSF profile boundary is now frozen in `docs/tsf-profile-boundary.md`: Windows exposes only the single product profile `Fcitx5`; internal engines/addons remain Fcitx state; obsolete dynamic profile data is cleanup input only.
- `rust/protocol-core` is now the single authoritative FCW4 codec: `protocol/protocol.cpp` is a thin marshalling adapter over the C ABI (`protocol/protocol_ffi.h`, typed encode/decode + `decode_header` in `capi.rs`), and `protocol-differential-contract` pins the pre-cutover wire bytes via `tests/unit/protocol_wire_golden.inc` (19 samples). C++ `protocol.h` API and all call sites are unchanged; see `docs/fcitx-upstream-rebaseline-audit.md` for the Fcitx5 upstream baseline audit (fork is official `ebf24ddc` + 41 lines; all three Windows-local changes are not yet upstream; 1 of 6 patches applies clean to master).

## Next Five Code/Design Tasks

1. Prepare/run generation-drain, installer/UAC, plugin lifecycle, Narrator/NVDA, and real
   Win7/Win10/Win11 host evidence before any release-readiness claim.
2. For official add-ons/plugins, build reviewed Windows package artifacts in this project, publish
   them as signed GitHub Release-backed repository assets, and let Settings install only through
   verified package metadata.
3. Execute `RELEASE-01` only after required manual/production evidence
   are green.
4. Continue deeper Stage 4 Rust Config binding work only under an explicit eligible task; the
   current code-only shell cutover is green, while release remains manual/real-host gated.
5. Continue Candidate renderer Rust drawing work under an explicit task that freezes screenshot/golden
   evidence for configurable label slots and row/column alignment before deleting the native adapter.
6. Continue shrinking non-Engine product-owned C++ shells only under explicit Rust migration tasks
   with frozen behavior and regression evidence.
