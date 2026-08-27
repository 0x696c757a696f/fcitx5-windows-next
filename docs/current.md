# Current Truth Snapshot

Date: 2026-08-27 (updated for Candidate usability correction and production plugin inputs)

HEAD recorded at snapshot refresh: `881a8149fd6e4f5447af0ac5df418d400d1f345b`

Working tree at snapshot refresh: task completion documentation only.

## 2026-08-27 Integration Freeze

- Fcitx5 Core and upstream addon semantics remain authoritative. Addons never enter the TSF host;
  native in-process capability/permission declarations are audit metadata, not a sandbox. Core input
  remains offline-capable, and any Config, plugin, UI, or updater failure degrades fail-soft toward
  basic input.
- Rust Config uses one typed `Current`/`Draft`/`Defaults` model and shared validate/diff/transaction/
  atomic-commit semantics for GUI, CLI, and tests. Preview is read-only Draft; successful commits keep
  one last-known-good recovery state. Backend shipping is distinct from Config Cutover Complete.
- CandidateModel is the single semantic source for renderer, UIA, and notifications. Notification
  coalescing/cancellation must reject stale revisions; sensitive contexts never reach speech, logs, or
  network paths. Accessibility is compositional (keyboard/UIA/Narrator/NVDA/High Contrast/large text/
  reduced motion/reduced candidates/stable layout), not a mode.
- Plugin metadata must carry `runtime_abi`, `runtime_build`, and source provenance; `runtime_build`
  is provenance/diagnostic rather than an ABI-equality gate. Program/version directories stay separate
  from user data. Repository freshness/freeze/mix-and-match and mirror identity are explicit follow-up
  work. Current signing remains ML-DSA-65 v2; ARM64, TUF, and RemoteAddon/AppContainer are not current
  support claims.
- The 2-core/4-GB low-resource profile and latency/memory figures are initial SLOs pending real
  hardware calibration; accessibility and low-resource evidence are release gates.

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
| Candidate | Rust candidate-core owns model/layout/interaction plus label formatting/slot planning/render-segment evidence; the default Settings shell exposes Automatic, Horizontal, Vertical, Scroll, `6 x N`, and `N x 6`, with `N` from authoritative page size; C++ UI window/renderer remains | Continue Rust authority and shrink adapter; preserve typed persistence, full-glyph evidence, stable label slots, and the WindInput/Qingfeng-derived WeChat-green visual contract |
| Config | Stage 2 Rust Config backend is shipping, and the default interactive `fcitx5-config.exe` window opens the vendored `huanfeng/wind-ui-rust` Settings shell. The plugin page consumes the pinned `fcitx5-plugins` catalog and real typed Control package operations through the bounded Rust process executor. Production-input automation now generates a signed x64 Rime package and v2 immutable repository assets. The old C++ WTL/Win32 shell remains an `EXCLUDE_FROM_ALL` regression baseline; the old Rust Win32/D2D preview host is QA-only. | Unsupported plugin artifacts stay visibly unavailable. Do not claim release readiness or final Stage 4 completion until protected publication, real online lifecycle, DPI/dark-mode/keyboard/accessibility, Narrator/NVDA, and real Win7/Win10/Win11 evidence are green |
| Launcher | Rust launcher-core owns state/path/tray/command/frame policy; C++ shell remains | Continue Rust cutover |
| Control | Rust control/package/process-exec cores linked; C++ command shell remains | Continue Rust cutover |
| Package/provider/downloader/updater/deployer | Package/provider/deployer/updater/downloader Rust CLIs and Rust package-core are wired; adapters remain where needed | Rust authority |
| Register/bootstrap | Rust policy/CLI work exists; Windows registry/elevation/export adapter remains | Thin adapter + Rust policy where useful |

## PoC And Migration State

| PoC / migration lane | State | Exit condition |
|---|---|---|
| Rust TSF | `SHIPPING-AUTOMATED-GREEN` / real-host matrix pending | Full host matrix evidence before release readiness |
| Rust Config | `CUTOVER-AUTHORIZED / STAGE-1-CORE-GREEN / STAGE-2-BACKEND-SHIPPED-GREEN / WINDUI-DEFAULT-SHELL-GREEN / STAGE-4-REAL-HOST-EVIDENCE-PENDING` | Non-interactive Config paths are Rust-owned under the shipping `fcitx5-config.exe` name with no legacy C++ GUI fallback. The default GUI now uses the vendored `windui` Settings shell and has screenshot/source-contract evidence. Stage 4 still requires real interactive persistence/plugin-page/candidate-preview parity, DPI/dark-mode/keyboard/accessibility, and real Windows QA before “Config fully migrated to Rust” can be claimed. |
| Candidate Rust core | `SHIPPING-DOMAIN / PRODUCTION-VERTICAL-TYPOGRAPHY-GREEN` | Preserve Rust model/layout/interaction, typed typography tokens, windui DirectWrite authority, and the full-glyph fit proof for commented rows |
| Rust package/update/control/launcher cores | `SHIPPING-DOMAIN` where already cut over, `MIGRATION-CANDIDATE` where shell remains | Delete replaced C++ authoritative implementation and keep adapter thin |
| Engine E1 `protocol-core` | `CUTOVER-GREEN` (Rust authoritative; C++ is a thin marshalling adapter) | Delete the old C++ codec internals (done); keep `protocol.h` API and call sites unchanged (done); future FCW4 wire changes must regenerate `protocol_wire_golden.inc` from the pre-change codec |
| Engine E2 `engine-core` ledger | `CUTOVER-GREEN` (Rust authoritative; ledger + carets/popupAllowed/selectedOverride/inputMethodOverridden cut over, C++ maps deleted) | — |
| Engine E3 Event→Action | `CUTOVER-GREEN` (unified `handle_key_event` entry; 4 product decisions Rust-owned, `processKey` executes the returned decision) | — |
| Engine E4 IPC scope | `CUTOVER-GREEN FOR CURRENT E4 SCOPE` (engine epoch, request ordering, key deadline policy, per-connection session state, and production server-side pipe connect/byte transfer are Rust-owned) | `Generation` stays release-platform scope; direct Fcitx object dispatch remains in the C++ adapter |
| Engine E5 snapshot/status canonicalization | `CUTOVER-GREEN` (canonicalization + typed `EngineSnapshot` DTO limits + `pendingStates` store Rust-owned; `collectResult`/`selectCandidate`/`takePendingState` apply them) | — |
| Engine E6 C++ product-state deletion | `CUTOVER-GREEN` (`candidate_navigation.h` + unit test deleted; scroll label offset Rust-owned; only Fcitx adapter + Windows process shell remain) | — |

## Current Red Lights

- Real-host matrix for Rust TSF remains manual-pending.
- Production plugin inputs are prepared but not published: the release environment still needs the
  protected ML-DSA-65 secret and matching v2 keyring, Authenticode material, publication permission,
  immutable GitHub Release assets, and real x64/x86 refresh/install/enable/disable/update/remove/restart
  evidence. Local generation and verification do not count as an online production lifecycle.
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
- Candidate label/ordinal UX is now implemented for the focused Rust drawing slice: labels are user-configurable presentation, not
  input semantics; every horizontal/vertical/grid cell reserves a label slot computed from the
  widest resolved label in scope; labels right-align inside that slot; candidate text begins after a
  stable gap; selected-row/column/item reveal modes keep hidden labels occupying the same space so
  rows and columns do not shift when `1.`/custom labels appear. Settings keeps scroll mode visible
  in the main Candidate layout area, and the embedded preview passes that flag into Rust layout.
- Candidate/Config palette and renderer-path requirement is implemented for the reachable code-only slice: project
  defaults use WeChat IME green, with
  light green/white and dark green/black palettes. Qingfeng/WindInput upstream themes and variants
  are preserved; Candidate visual code directly ports WindInput/Qingfeng candidate window/view/theme
  tokens for the Rust PoC screenshots and renderer evidence.
- Candidate production typography visual acceptance is green after rejecting the old
  non-overlap-only proof. Rust owns independent candidate, label, and comment sizes plus row height;
  the shared plan budgets full CJK advance without an unbudgeted text offset. Fresh x64/x86 reports
  assert `typography_text_fits=true`, and matching 150% DPI screenshots show complete `水` and `收`
  glyphs beside `~b` and `~d`.
- Candidate default font must be CJK-first: `Microsoft YaHei` first, `Microsoft YaHei UI` and
  `system` only as fallback. Appearance tuning may use Rime/鼠须管 theme lessons, including
  `eosphoros-keytao` Squirrel color schemes, for spacing, label size, corner radius, border, line
  spacing, candidate spacing, light/dark scheme tokens, and highlight tokens, but those values must
  resolve into one typed theme snapshot before rendering.
- The executable queue has been re-cleaned after the 2026-08-24 review: completed/current R3
  FUTURE-GATED duplicates are not active queue items. Completed queue source files are removed from
  `docs/tasks/queue` after their task files are archived. `059-CANDIDATE-LABEL-SLOT-RUST-DRAWING-001`
  through `064-CONFIG-CANDIDATE-PLUGIN-USABILITY-CORRECTION-001` are complete for reachable
  automated Candidate and plugin work; `RELEASE-01` is current and remains gated on
  external/manual evidence.
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

1. Prepare/run generation-drain, installer/UAC, published plugin lifecycle, Narrator/NVDA, and real
   Win7/Win10/Win11 host evidence before any release-readiness claim.
2. Provision the protected release inputs and publish the prepared Rime package/index/signatures as
   immutable GitHub Release assets; then run the real x64/x86 lifecycle through Settings.
3. Execute `RELEASE-01` only after required manual/production evidence
   are green.
4. Continue deeper Stage 4 Rust Config binding work only under an explicit eligible task; the
   current code-only shell cutover is green, while release remains manual/real-host gated.
5. Continue Candidate renderer Rust drawing work only under a later explicit task that replaces more
   of the native adapter while preserving the current label-slot and scroll evidence.
6. Continue shrinking non-Engine product-owned C++ shells only under explicit Rust migration tasks
   with frozen behavior and regression evidence.

## Next Code-Only Queue

`RELEASE-01` remains parked on external evidence. The first eligible code-only task is
`065-CONFIG-CORE-TRANSACTION-CONTRACT-001`, followed by Candidate notification/accessibility,
plugin provenance/data-boundary, repository freshness, real plugin ecosystem build matrix, and
low-resource evidence tasks. Each task is Rust-first: C++ is limited to the direct Fcitx adapter,
upstream native addon, or a thin Windows ABI/renderer adapter; no second Config truth, generic GUI
framework, or permanent protocol dual stack.
