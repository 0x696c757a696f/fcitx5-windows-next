# App-path UIA bounding-rect: handoff playbook (Iteration 50 candidate)

Frozen criterion ④ requires every asserted control's `BoundingRectangle` to be non-empty and fully
inside the real window rect. That clause is the **only** unmet item of Iteration 48; everything else
(navigation Invoke, semantics, `Flow → 7 → Apply` round-trip, Config Core read-back, stale
`0x80040201`, 16-case execution) passes on both architectures.

## Established facts (do not re-derive)

1. The vendored provider repair is landed and proven **cross-process** by `examples/uia_smoke` on x64
   and x86 (exit 0): a cross-process OS UIA client reads `VT_ARRAY | VT_R8` with four finite values
   for a windui node, a non-Button clickable row exposes a non-null Invoke pattern that fires exactly
   one host state change, Button Invoke still works, stale elements still report `0x80040201`.
   The cross-process nature is verified in source: the client spawns the host as a child process
   (`std::process::Command::new(&exe).arg("--host").spawn()`, `examples/uia_smoke.rs:111-113`), so
   SAFEARRAY marshalling for this provider demonstrably works in principle — the gap is specific to
   the Settings window/provider delivery, not to cross-process arrays in general.
2. On the **Settings application path** the same client, reading the same property id, gets an
   unwritten VARIANT, while other properties of the *same elements* decode exactly right:
   - `30003` ControlType → `vt=0x0003`, `data1=0xc370` = **50032** (`UIA_WindowControlTypeId`) — correct
   - `30005` Name → `vt=0x0008` (`VT_BSTR`) — correct
   - `30020` BoundingRectangle → `vt=0x0003` with garbage `data1` (`0x902d0`, `0x7ffc00000000`, ...)
     and `GetCurrentPropertyValueEx(ignoreDefaultValue=TRUE)` → `vt=0x000D`
   Evidence: `docs/tasks/evidence/iteration-48-49/VARIANT-decode-proof.txt`.
3. `windows-rs`'s `VARIANT::default()` is `core::mem::zeroed()` → `vt = VT_EMPTY (0)`, **not** `VT_I4`.
   So the `vt=0x0003` garbage cannot be the provider's `_ => VARIANT::default()` fallback arm; it is
   synthesised on the UIA side.
4. Instrumented traces on both provider rect error branches (`scaled_screen_rect`'s `ClientToScreen`
   failure and the rect arm's error paths, including `double_array_variant`) **never fired** on the
   Settings path, so the provider neither errored nor reported unavailability there.
5. Rect is the **first** property read for each element, so staleness/timing is excluded. The window
   rect from `GetWindowRect` is real (e.g. `25,25 → 1079,762`).
6. The client's COM apartment is not the difference: the fixture client uses the same
   `CoInitializeEx(NULL, COINIT_MULTITHREADED)` and succeeds.

## Next-session playbook (in order, cheapest first)

1. **Isolate client vs provider with a standard Win32 control.** Read property `30020` through the
   same raw-COM driver path for a window of a stock application (a class like `Notepad` or
   `#32770`). If that also returns `vt=0x0003`, the defect is in the *client/marshaling* path used by
   the driver, not in windui; if it returns `VT_ARRAY | VT_R8`, the defect is specific to windui's
   server-side provider on the Settings window.
2. **Read the fixture's root element rect.** Extend `examples/uia_smoke.rs`'s client to read
   `CurrentBoundingRectangle()` for the **root** element as well as for the button. If the root is
   garbage there too, the root/`UiaHostProviderFromHwnd` path is the suspect and the app-path failure
   for its 66 elements follows from it.
3. **Compare provider registration between the two windows.** In `platform/win32/mod.rs`, diff the
   `WM_GETOBJECT` arm behaviour for the Settings window vs the fixture host window: which object id is
   answered, whether `UiaReturnRawElementProvider` is called with a server-side provider, and whether
   `ProviderOptions_ServerSideProvider` is advertised. A registration difference would explain why one
   window marshals arrays and the other does not.
4. **UIA-level probe.** Inspect the Settings window with a UIA client that reports raw property
   variants (an Inspect-style tool, or `IUIAutomationElement::GetCurrentPropertyValueEx` with
   `ignoreDefaultValue = FALSE`) to see what UIA believes the provider answered.
5. Fix, then re-run the acceptance: `pwsh tools/verify-product.ps1 pr -Architecture x64` (89 tests,
   includes the eight `interactive-uia` lanes) and the 16-case matrix
   (`docs/tasks/evidence/iteration-48-49/` shows the runner recipe), and only then write the green
   status line.

## Guardrails

- Never weaken criterion ④: the assertion is blocking in `rust/config-qa/src/main.rs` and must stay so.
- Any change to `third_party/wind-ui-rust` requires the fourth-patch discipline: regenerate
  `third_party/patches/wind-ui-rust/win32-uia-bounds-invoke.patch` with git (`--output=`, no BOM/CR),
  keep it registered last in `tools/sync-windui.ps1`, run
  `tools/verify-windui-vendor-coverage.ps1` (exit 0) and prove the fixed-pin
  `980eb5ef132d2aacc0ae801e7cdc11a433f88206` replay is byte-identical.
- Root `Cargo.lock` must stay unchanged and no dependency may be added (this is why the driver is
  hand-rolled raw COM rather than using the `windows` crate).
