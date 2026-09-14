# Config UI complete interaction test cases

This is the semantic, control-level acceptance specification for `fcitx5-config.exe`. Automation
may use a platform adapter to invoke production controls, but its authority is the visible
semantic action and result—not a Win32 notification, HWND count, or a particular UI toolkit.
External or destructive effects are verified through their lower-level contract and the named
desktop/package gate. A visible control that has no row here is a test-design defect.

## Navigation and common behavior

| ID | Interaction | Expected result | Automated evidence |
|---|---|---|---|
| CFG-NAV-01..06 | Click General, Appearance, Theme, Diagnostics, Repair and Packages | selected navigation style, page title and page-specific controls change; hidden-page controls are not visible | `config-ui-interaction-coverage`, `config-ui-behavior-contract` |
| CFG-KEY-01 | Tab/Shift+Tab through the visible page; Space activates checks/buttons; arrows change radio/combo/list selection; Enter applies | logical focus order, visible focus, no keyboard trap, no hidden control receives focus | Desktop accessibility gate |
| CFG-DPI-01 | Open at 100/150/200%, resize/minimize/restore | no clipping/overlap; native text remains readable; card decoration does not cover controls | Compatibility visual evidence |
| CFG-I18N-01 | Run English and Simplified Chinese locales | every key exists, no blank/truncated action label | `config-ui-i18n-check`, Compatibility visual evidence |

## General

| ID | Interaction | Expected result | Automated evidence |
|---|---|---|---|
| CFG-GEN-01 | Click Startup | dirty state appears | interaction sweep |
| CFG-GEN-02 | Select every available input method | dirty state appears; selection remains unambiguous | interaction sweep + real engine list test |
| CFG-GEN-03 | Exercise declared setting lifecycle | Live commits immediately; Deferred applies atomically or reports failure; Restart-required gives a bounded action | interaction sweep + Control round-trip + Desktop |
| CFG-GEN-04 | Close/reopen after committed change | selected settings reload; cancelled Deferred edits do not | Portable round-trip + Desktop |

## Appearance and Theme

| ID | Interaction | Expected result | Automated evidence |
|---|---|---|---|
| CFG-VIS-01 | Select System, Light and Dark | each selection follows its declared lifecycle and persists after commit/restart | interaction sweep + Control/Portable round-trip |
| CFG-VIS-02 | Click Vertical then Horizontal | radio buttons remain mutually exclusive; renderer orientation reflows after committed change | interaction sweep + candidate live-config reflow |
| CFG-VIS-03 | Toggle Scroll mode on/off | dirty/saved state is correct; grid/scroll rendering changes without stale candidate coordinates | interaction sweep + layout/render contracts |
| CFG-VIS-04 | Select each theme | valid theme ID persists; missing/invalid theme falls back safely | interaction sweep + TOML/theme tests |
| CFG-VIS-05 | Focus/edit/blur font, including empty, long and non-ASCII names | dirty state; empty/invalid input is rejected without corrupting previous config; valid font persists | interaction sweep + config parser boundary tests |
| CFG-VIS-06 | Open Preview | preview uses the production CandidateModel/layout/render contract and closes with its parent | interaction sweep wiring + renderer self-test + Desktop visual parity |
| CFG-VIS-07 | Exercise Live/Deferred paths on Appearance and Theme | saved state is reported only after atomic TOML write; active candidate reloads only committed values | interaction sweep + Control atomic write + live-config reflow |

## Diagnostics and Repair

| ID | Interaction | Expected result | Automated evidence |
|---|---|---|---|
| CFG-DIAG-01 | Click Restart engine | bounded generation change; TSF host remains alive; status is explicit | interaction sweep wiring + Desktop PID-change test |
| CFG-DIAG-02 | Click Diagnostics | non-empty structured health data; no typed/candidate/user-dictionary content | interaction sweep + Control schema/status tests |
| CFG-REPAIR-01 | Click Repair | invokes the product bootstrap repair path, handles UAC cancellation, restores owned files/profile only | interaction sweep wiring + same-installer repair test |

## Packages

| ID | Interaction | Expected result | Automated evidence |
|---|---|---|---|
| CFG-PKG-01 | Click Refresh online/offline/bad TLS/bad signature | signed metadata replaces cache atomically only on success; useful offline/error status remains | interaction sweep wiring + downloader/repository fixture tests |
| CFG-PKG-02 | Select bundled component | Rime, Rime Lua, Fcitx Lua, Chinese Addons and Chttrans appear as bundled; action buttons are disabled | UI behavior contract + Portable package list |
| CFG-PKG-03 | Select available component and click Install | dependency plan, download, hash/signature, staging, activation and restart complete; failure rolls back | interaction sweep wiring + signed package transaction test |
| CFG-PKG-04 | Select older installed component and click Update | exact target version activates and previous-known-good remains rollbackable | interaction sweep wiring + package transaction/rollback test |
| CFG-PKG-05 | Click Disable and Enable | persisted state changes and engine restarts; protected/bundled component rules remain enforced | interaction sweep wiring + package state contract |
| CFG-PKG-06 | Click Uninstall | restart-safe pending removal/finalization; user data follows manifest ownership and is not broadly deleted | interaction sweep wiring + package removal contract |
| CFG-PKG-07 | Double-click rapidly or change selection during an operation | one bounded transaction, buttons reflect busy state, no duplicate activation | Desktop/package concurrency gate |

## Negative, recovery and security cases

| ID | Case | Expected result | Gate |
|---|---|---|---|
| CFG-NEG-01 | Invalid/truncated/oversized TOML | page reports error and preserves last known-good file | PR |
| CFG-NEG-02 | read-only/full disk/interrupted atomic write | no partial config; prior configuration remains loadable | PR + Package |
| CFG-NEG-03 | launcher/engine/UI missing or crashing | Config remains responsive; diagnostics explain the state; restart is bounded | PR + Desktop |
| CFG-NEG-04 | malformed repository/archive/path collision/zip bomb/bad key | reject before activation; no path escape or executable input enters the input plane | PR + Package |
| CFG-NEG-05 | 100 repeated page/control sweeps and preview open/close | no hang; bounded handle/GDI/USER growth | Compatibility soak |
| CFG-NEG-06 | screen reader, High Contrast, keyboard-only and 200% DPI | names, roles, focus and contrast are usable | Compatibility accessibility gate |

## Coverage rule

`config-ui-interaction-coverage` must exercise every reachable visible semantic action and each
selection/edit branch on a safe synthetic model. It must derive its inventory from the production
accessibility/control semantics and fail when an actionable control lacks coverage; it must not use
a hard-coded HWND or button count as acceptance. Any shared Apply action is exercised on every
Deferred-owning page, and Install/Update is exercised through both semantic branches. Input-method,
theme and appearance choices are selected, toggles run both ways, orientations are selected, and
font input covers empty, ASCII and non-ASCII values. This makes an untested user action a
reproducible failure without binding the contract to WTL, Win32, or a particular renderer.

Lower-level tests then prove persistence and transaction semantics. Desktop gates prove Shell, COM,
UAC and real host integration. A feature is accepted only when all applicable layers pass for the
same artifact lineage.

The Desktop gate also invokes every actionable notification-menu item through its real Shell popup:
Restart, Pause, Resume, Settings, Diagnostics and Exit. The disabled status row and separators are
asserted as menu structure, not treated as clickable actions.
