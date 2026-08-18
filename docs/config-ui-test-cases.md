# Config UI complete interaction test cases

This is the control-level acceptance specification for `fcitx5-config.exe`. The automated
interaction sweep invokes real Win32 command notifications while suppressing only external or
destructive effects. Those effects are verified through their lower-level contract and the named
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
| CFG-VIS-06 | Click Preview | preview process uses the real D2D/DWrite CandidateModel renderer and exits with its parent | interaction sweep wiring + renderer self-test + Desktop visual parity |
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

`config-ui-interaction-coverage` must click every command button and trigger every selection/edit
notification on a safe synthetic model. It enumerates the real child HWND tree after the sweep and
fails if any non-zero-ID `Button` was not clicked. The current inventory is 19 unique Button HWNDs;
any shared Apply button is exercised on every Deferred owning page and the shared Install/Update button
is exercised through both semantic branches. All input-method/theme/appearance combo entries are
selected, checkboxes are toggled in both directions, both orientation radios are selected, and the
font edit receives empty, ASCII and non-ASCII edit notifications. This makes adding a button without
adding a test a reproducible failing case instead of a documentation convention.

Lower-level tests then prove persistence and transaction semantics. Desktop gates prove Shell, COM,
UAC and real host integration. A feature is accepted only when all applicable layers pass for the
same artifact lineage.

The Desktop gate also invokes every actionable notification-menu item through its real Shell popup:
Restart, Pause, Resume, Settings, Diagnostics and Exit. The disabled status row and separators are
asserted as menu structure, not treated as clickable actions.
