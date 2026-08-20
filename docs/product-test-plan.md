# Product-wide test plan

This is the risk-based test design for the frozen v1.7 product. It complements the
requirement traceability in `ssdlc-verification-matrix.md` and the control inventory in
`config-ui-test-cases.md`. A row is not accepted merely because code exists: the named gate must
pass against the same artifact lineage. Environment-dependent rows remain open until their real
environment evidence exists.

## Coverage model

| Layer | Purpose | Required examples |
|---|---|---|
| Unit / property | Pure rules, invariants and boundary values | protocol codec, CandidateModel, layout, typed TOML, launcher and deployment models |
| Parser / fuzz | Arbitrary hostile bytes and resource budgets | IPC, package manifest/index/archive paths, TOML validation adapter, stateful typing |
| Integration | Real process and owner boundaries | TSF to engine, launcher lifecycle, Fcitx addons, package transaction, Control API |
| Desktop E2E | Windows behavior that mocks cannot prove | real TSF profile, Notepad commit, tray popup, no-console startup, focus and fail-open |
| Delivery E2E | Actual files given to users | moved Portable ZIP, installer install/repair/uninstall, signed release artifact verification |
| Compatibility | Declared host/platform capability | Win7/10/11, x86/x64, Office/browser/terminal, DPI/multi-monitor/RDP, legacy/fullscreen hosts |

No blanket code-coverage percentage is used. Coverage is complete when every public action,
security boundary, failure transition and declared environment has a test owner and current result.

## User interaction inventory

### Config window

`config-ui-interaction-coverage` opens the actual WTL/Win32 window and uses `BM_CLICK` or the real
control notification. It then enumerates every child `Button` HWND and fails if one was not clicked.

| ID range | Controls and state branches | Expected result |
|---|---|---|
| UI-NAV-001..006 | General, Appearance, Theme, Diagnostics, Repair, Packages | each page becomes selected; only its controls are visible and focusable |
| UI-GEN-001 | Startup off -> on -> off | declared Live/Deferred behavior is consistent; persistence and partial-failure semantics match the typed model |
| UI-GEN-002 | every input-method combo entry | every selection is unambiguous and dirty; selected stable ID round-trips |
| UI-APP-001 | System, Light, Dark | all entries emit selection; saved mode round-trips and changes renderer appearance |
| UI-APP-002 | Vertical, Horizontal, Vertical | radio exclusivity and reversible renderer reflow |
| UI-APP-003 | Scroll off -> on -> off | reversible grid/scroll layout with no stale hit-test coordinates |
| UI-APP-004 | Appearance lifecycle | Live values persist immediately; Deferred groups expose Apply/Cancel; active renderer reloads only committed state |
| UI-THM-001 | every theme combo entry | stable theme ID; invalid/missing theme falls back safely |
| UI-THM-002 | font empty, ASCII, non-ASCII, over-limit via parser boundary | UI becomes dirty; invalid value cannot replace valid config; valid value round-trips |
| UI-THM-003 | Preview | launches the production D2D/DWrite renderer path; closes with parent; matches saved layout contract |
| UI-THM-004 | Theme lifecycle | same Live/Deferred atomic rules as Appearance; Restart-required is explicit when applicable |
| UI-DIA-001 | Restart | bounded engine PID/epoch change; host remains responsive |
| UI-DIA-002 | Diagnostics | structured status is non-empty and contains no S0/S1 input data |
| UI-REP-001 | Repair | invokes owned bootstrap/installer repair; cancellation/failure is actionable and bounded |
| UI-PKG-001 | Refresh | offline, TLS/signature failure and success preserve/replace cache atomically as appropriate |
| UI-PKG-002 | Install and Update branches | signed dependency plan stages and activates once; failure leaves old active version complete |
| UI-PKG-003 | Enable and Disable branches | persisted state and one safe engine restart; bundled rows remain read-only |
| UI-PKG-004 | Uninstall | pending removal, restart/finalize, owned files only, user dictionary retained |

The former 19-button inventory is historical and must be regenerated from the Phase 6 UI after its
v1.7 Live/Deferred/Restart-required redesign. Automation must exercise every reachable button and
semantic branch; a hard-coded count is not acceptance.

### v1.7 mandatory regressions

| ID | Scenario | Required result |
| --- | --- | --- |
| REG-UI-001 | terminate `fcitx5-ui.exe` or inject device loss | input/composition continues, UILess semantics remain correct, UI recovers only on demand |
| REG-LOL-001 | League of Legends + Vanguard chat, Alt+Tab and control-key passthrough | composition/candidate/commit work through TSF; no Hook, SendInput, injection or evasion |
| REG-PREEDIT-001 | emoji/surrogate caret, empty preedit, commit+new-preedit, external termination | TSF composition offsets/lifecycle stay valid; no stale candidate |
| REG-LIFE-001 | separately kill Engine, UI and Launcher | bounded fail-open/recovery, no restart storm, no manual PowerShell |

### Notification-area menu

The Stable gate obtains the real Shell icon rectangle, opens the production popup and invokes its
actual menu item rectangles.

| ID | Menu action | Expected result |
|---|---|---|
| TRAY-001 | Settings | Config process opens without a console |
| TRAY-002 | Diagnostics | Config opens directly on Diagnostics |
| TRAY-003 | Restart service | engine PID and epoch change; ready returns within deadline |
| TRAY-004 | Pause | state becomes UserStopped and typing is not swallowed |
| TRAY-005 | Resume | launcher/engine become ready again |
| TRAY-006 | Exit | launcher and engine exit; no restart loop |
| TRAY-007 | status row and separators | correct disabled/structural roles; never treated as actions |

### Entry points and installer

| ID | Action | Expected result |
|---|---|---|
| ENTRY-001 | `Start Fcitx5.exe` | GUI-subsystem launch, tray and engine ready, no persistent terminal |
| ENTRY-002 | `Fcitx5 Settings.exe` | Config opens directly and exits cleanly |
| ENTRY-003 | `Unregister Fcitx5.exe` | only owned profile/CLSID registrations are removed |
| INST-001 | silent install | exact x64/x86 registrations and startup owner are present |
| INST-002 | rerun same installer | repair restores owned files/registration without rebuilding |
| INST-003 | silent uninstall | service stops, profiles/files are removed, declared user data is retained |
| INST-004 | upgrade while old `fcitx5-tsf.dll` is loaded by a host | old DLL is renamed, new DLL is installed at the registered path, old host is not killed, cleanup is retried or scheduled |
| INST-005 | mixed-generation TSF clients during upgrade drain | old TSF routes only to its old generation IPC/runtime, new TSF routes only to the new generation, no protocol compatibility shim is required; `current.json` advances only to an existing runtime generation |
| INST-006 | stale/broken TSF activation after update | TSF activation returns success but fail-opens, no key sink is advised, host keys pass through, Control reports and resets the guard |
| PORT-001 | extract, run, move, run | data root follows the moved tree and system drive is not used as fallback |

## Phase and component cases

| Area | Positive cases | Negative / recovery cases | Gate |
|---|---|---|---|
| Reference baseline | pinned source/commit/license and keep/refactor/reject record | missing/unpinned/unknown-license reference | PR |
| Build/toolchain | x64+x86 warnings-as-errors, analysis, UTF-8/LF, declared dependencies | missing tool, cache removal, undeclared IDE/manual step | PR |
| TSF/COM | register, activate, TestKey/KeyDown, composition and commit | duplicate TestKey, orphan KeyUp, sync termination, sensitive scope, missing engine, stale activation guard, in-use DLL upgrade drain | PR + Stable |
| IPC | legal codec round-trip, multi-client, correct peer | truncation, oversized/unknown enum/version, wrong peer, timeout, late/stale response | PR + Nightly |
| Launcher | normal start, warm ready, restart, stop/resume | immediate crash loop, Updating/Uninstalling/UserStopped demand start, secure desktop/session 0 | PR + Stable |
| Fcitx engine | Pinyin and Rime preedit/candidate/commit; two contexts; x86+x64 clients | restart epoch, stale context/revision, UI absent, burst/key-repeat backlog | Nightly |
| Addons | Rime Lua, fcitx5-lua and chttrans execute functional assertions; Chinese Addons produce real Pinyin | missing/ABI mismatch/broken optional addon enters Safe Mode | Nightly |
| Candidate UI | vertical/horizontal/scroll, empty/large list, labels/comments, caret clamp, High Contrast | stale snapshot, invalid selected index, D2D device loss, missing font/theme/UI crash | PR + Stable |
| Config/TOML | sparse override merge/reset, all enum/range boundaries, atomic write and reload | unknown/duplicate/wrong type/version, NaN/Inf, over-limit, interrupted/read-only/full-disk write | PR + Nightly |
| Package repository | trusted signed index/package, dependency resolution, install/update/state/remove/rollback | bad/revoked key, hash mismatch, downgrade/cycle, offline/TLS error, interrupted transaction | PR + Nightly + Stable |
| Archive/filesystem | declared files extract inside staging | traversal, absolute/UNC/ADS, case collision, symlink/reparse, zip bomb, undeclared file | PR + Nightly |
| Deployment/update | atomic activation, health mark, one previous-known-good, owner-aware update, generation-specific IPC/runtime drain | activation/health failure, external owner, rollback, cleanup interruption, locked old TSF DLL | PR + Stable |
| Security/privacy | input processes have no network import; structured operational diagnostics | raw key/preedit/candidate/commit logging, unsafe DLL path, hook/injection/memory-access capability | PR + Stable |
| Distribution | same-artifact promotion, hashes, SBOM, provenance, system metadata | dirty/untagged source, unsigned bytes, empty keyring, artifact/hash lineage mismatch | Stable |

## Real typing and robustness

| ID | Scenario | Pass condition |
|---|---|---|
| TYPE-001 | real Notepad, active registered profile, type `ni`, press Space | commits U+4F60 and Notepad remains responsive |
| TYPE-002 | engine absent, type `abc` | host receives exactly `abc` within the bounded fail-open path |
| TYPE-003 | 300 continuous composition rounds / 900 keys | no hang, unbounded queue, duplicate commit or stale candidate |
| TYPE-004 | fixed-seed 4,000-event stateful typing fuzz | state converges; keys are committed/composing/fail-open, never lost silently |
| TYPE-005 | 200 events/s for two seconds and 60 Hz repeat | no accumulated backlog and latency remains within recorded budget |
| TYPE-006 | 10,000 focus/context create/destroy cycles | stale snapshots rejected; HANDLE/GDI/USER delta remains zero |

## Compatibility matrix

Each cell records OS build, architecture, host version, DPI/monitor/session, artifact manifest hash,
result and evidence timestamp. Pure layout tests do not close a real environment row.

| Environment | Required scenarios |
|---|---|
| Windows 11 primary | Notepad, Word, Excel, Edge/Chrome, VS Code, Terminal, Explorer/Search; 100/125/150/200/300%; multi-monitor; High Contrast; keyboard-only; Narrator/NVDA |
| Windows 10 baseline | same core host set; install/repair/uninstall; RDP or second interactive session |
| Windows 7 SP1 Legacy | pinned toolchain/import check, x86+x64 TSF, Notepad, basic Candidate/Config/Repair, System DPI, KB2533623 loader path |
| Legacy/fullscreen | x86 IMM32/CUAS host, DDraw/D3D8/D3D9 windowed/fullscreen/Alt+Enter, no-text WASD/Space/Ctrl passthrough |
| Protected application | normal signed TSF smoke or safe refusal; no hook, injection, process-memory access or evasion |

## Evidence and stop rule

Every evidence record contains source commit, dirty state, configuration, architecture, artifact
stage and manifest SHA-256, OS/host identity, test IDs, timestamps and pass/fail details without S0/S1
content. A downstream stage invalidates when its binary lineage changes.

The PR, Nightly, Win7 VM milestone and Stable gates may pass independently. Stable completion
additionally requires the production signed repository/keyring, protected Authenticode authority,
clean tagged source and the targeted real compatibility matrix. The matrix is risk-selected rather
than a Cartesian product of every OS, architecture, host, DPI and input method. Missing external
evidence is reported as a blocker; it is never replaced by a mock, screenshot or prose assertion.
