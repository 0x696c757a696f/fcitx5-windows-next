# WindUI UIA `BoundingRectangle` repair plan (blocker for Iteration 48 / P3)

Status: **BLOCKER for the P3 Settings real-HWND UIA slice.** Produced because the approved plan
pre-authorizes exactly this outcome: "如果真实 Settings 暴露出 provider framework defect，停止 P3，
返回单独的 WindUI repair plan" — and because the goal constraint forbids patching vendored WindUI
inside P3.

## 1. Symptom (measured, reproducible twice)

A cross-process UIA client reading `UIA_BoundingRectanglePropertyId` (30020) from the real Settings
window gets a **non-array VARIANT** instead of `VT_ARRAY | VT_R8`:

```
{"result":"FAIL","reason":"BoundingRectangle VARIANT vt=0x0003 data1=0x306fe (expected 0x2005);
 GetCurrentPropertyValueEx(ignoreDefault=TRUE) -> vt=0x0003"}
{"result":"FAIL","reason":"BoundingRectangle VARIANT vt=0x0003 data1=0x7a0b70 (expected 0x2005);
 GetCurrentPropertyValueEx(ignoreDefault=TRUE) -> vt=0x0003"}
```

- `vt = 0x0003` = `VT_I4`, not `VT_ARRAY | VT_R8` (0x2005) → the value is not a rect.
- `data1` differs between runs (0x306fe, 0x7a0b70) → stack/heap garbage, i.e. **the VARIANT is never
  written**; the value is not even a stable integer.
- Same result via `GetCurrentPropertyValue` *and* `GetCurrentPropertyValueEx(ignoreDefaultValue=TRUE)`
  → the property is genuinely unsupported, not merely defaulted.
- The read target was the **root** element of a real window whose `GetWindowRect` is real
  (`left 176, top 176, right 1230, bottom 913` = 1054×737 physical px), so the window itself is fine.
- Independent corroboration: a separately written PowerShell probe (its own SAFEARRAY reader) saw
  `{0,0,0,0}` for **all 42** enumerated descendants of the same window.
- The ABI of the driver's own raw-COM path is *not* the cause: the same call path correctly reads
  `UIA_ControlTypePropertyId` (30003) as `VT_I4` and `UIA_NamePropertyId` (30005) as `VT_BSTR`, and
  `FindAll(CreateTrueCondition, TreeScope_Descendants)` works (42 elements).

## 2. Root cause (source-level, read in the vendored crate)

`third_party/wind-ui-rust/src/platform/win32/accessibility.rs`, `fn property_value` (~lines 286-296):

```rust
let value = match property_id {
    id if id == UIA_NamePropertyId.0 => bstr_variant(&node.name),
    id if id == UIA_ControlTypePropertyId.0 => i32_variant(control_type(node.role)),
    id if id == UIA_IsEnabledPropertyId.0 => bool_variant(node.enabled),
    id if id == UIA_IsKeyboardFocusablePropertyId.0 => bool_variant(node.focusable),
    id if id == UIA_HasKeyboardFocusPropertyId.0 => bool_variant(node.focused),
    _ => VARIANT::default(),
};
```

There is **no arm for `UIA_BoundingRectanglePropertyId`**, so every rect request falls into
`VARIANT::default()`, and the cross-process client receives an unwritten VARIANT.

The fragment method *is* implemented — `IRawElementProviderFragment::BoundingRectangle()` at ~lines
492 and 558 delegates to `scaled_screen_rect(state, node)` (~line 366), which computes
`node.bounds` scaled by the window DPI and converted client→screen. But that fragment-level answer
is not delivered to a cross-process UIA client through the property path, which is what clients
(`IUIAutomationElement::CurrentBoundingRectangle`, and any rect-based assertion) actually use.

Secondary finding (not a rect issue, but relevant to the same slice): sidebar navigation items are
exposed as **ControlType 50020 (Text)**, not 50000 (Button). That is a classification choice in the
platform-neutral projection; it does not block the slice if assertions accept Text for nav entries.

## 3. Proposed repair (vendored, separate task — NOT part of P3)

1. Add a rect→VARIANT helper in the win32 provider, e.g.
   `fn double_array_variant(rect: UiaRect) -> VARIANT` building a `VT_ARRAY | VT_R8` SAFEARRAY of
   four doubles in UIA's documented order `[left, top, width, height]` (allocated with
   `SafeArrayCreateVector(VT_R8, 0, 4)` + `SafeArrayPutElement`, ownership transferred to UIA).
2. Add the missing arm:
   `id if id == UIA_BoundingRectanglePropertyId.0 => double_array_variant(scaled_screen_rect(state, node)?),`
3. Unit tests (in the vendored crate): assert the property returns `VT_ARRAY | VT_R8` with 4
   elements, and that its values match the fragment `BoundingRectangle()` for the same node under
   both 96 dpi and a scaled DPI, including a non-origin window position.
4. Re-run the vendored suite on x64 and x86 (recorded baseline before the change: 910 passed,
   2 suites) and `examples/uia_smoke.rs` (which already asserts a rect inside the window rect).
5. **Vendoring evidence is mandatory** because the change touches `third_party/wind-ui-rust`:
   - regenerate `third_party/patches/wind-ui-rust/windui-local-product-delta.patch` (written by
     `git … --output=`, never by `Set-Content` — CRLF/BOM corrupts it), keep it registered as the
     **last** entry of `tools/sync-windui.ps1`'s `$patches`;
   - `pwsh tools/verify-windui-vendor-coverage.ps1` must exit 0;
   - a clean `pwsh tools/sync-windui.ps1 -Commit 980eb5ef132d2aacc0ae801e7cdc11a433f88206` replay
     must be **byte-identical** to the working tree (128 files);
   - the record the patch SHA-256 in the task/status evidence and update the Iteration 44 report's
     manifest values.
6. Then re-run the P3 matrix (16 cases) with the rect assertions enabled.

## 4. Alternatives considered (and why they are not taken inside P3)

- **Relax the driver to skip rect assertions.** Rejected: the frozen success criterion ④ explicitly
  requires every asserted control's `BoundingRectangle` to be non-empty and inside the window rect.
  Dropping it would silently weaken frozen acceptance.
- **Read rects through the fragment interface from the client.** Not possible: clients reach
  providers only through the UIA property/pattern surface.
- **Have the driver compute rects from the app's layout model.** Rejected: that would be a
  screenshot/layout-model proxy, not UIA evidence, and the criterion is about UIA bounds.

## 5. Consequence for Iteration 48

Tasks 4, 5 and 6 (driver, closed loop, 16-case matrix) cannot be marked green while the property is
unserved. The driver itself is implemented and already proves, on a real window: real HWND discovery
by class, root ControlType 50032, a 42-element descendant tree, named localized controls, and
`FindAll`/property plumbing — but every rect assertion fails at the same place. Options for the user:

- **A. Authorize the vendored repair above as its own task** (patch + replay proof), then resume P3.
- **B. Accept a reduced P3 scope explicitly**: keep the semantic assertions (names, roles, enabled,
  focusable, twin uniqueness, closed loop, stale HRESULT) and record
  `VISIBLE-TEXT-TRUNCATION / BOUNDING-RECT EVIDENCE: MANUAL-PENDING` — never green.
- **C. Stop P3** and fold the repair plan into a later WindUI task.

## 6. Second gap in the same slice: the Appearance nav item has no Invoke pattern

Measured with the corrected driver (see below):

```
TRACE about-to-invoke-nav
{"result":"FAIL","reason":"GetCurrentPattern Invoke \"Appearance\" HRESULT 0x00000000"}
```

`0x00000000` is `S_OK`; the driver fails here because the returned pattern interface is **NULL**, which
by COM/UIA convention means *pattern not supported*.

- The sidebar nav element (localized `nav.appearance`, `Appearance` for en-US / `外观` for zh-CN) is
  projected with `ControlType = 50020` (**Text**), not `50000` (Button).
- `GetCurrentPattern(UIA_InvokePatternId)` on it returns `S_OK` + null → the nav row cannot be
  activated through UIA at all. (Before the driver's vtable fix it returned `0x80070057`
  `E_INVALIDARG`, because `GetCurrentPattern` was declared at vtable index 12 — `GetCachedPropertyValue`
  — instead of the real index 16. That was a **driver** bug and is fixed; the null-pattern result is
  the provider's behaviour.)
- The vendored provider imports only `UIA_InvokePatternId` and implements no alternative activation
  pattern (`LegacyIAccessible` / `DoDefaultAction` are absent), so there is **no other UIA path** to
  switch pages.
- Consequence: a UIA client stays on the default Input Methods page forever, the Candidate surface
  never materializes in the tree, and success criteria ④ and ⑤ (which require invoking the localized
  Appearance entry and then the candidate controls) cannot be reached.

### Proposed repair (add to the same vendored task)

7. Make the navigation rows activatable through UIA: either project a nav row as an element with the
   **Invoke** pattern (role Button, or the nav row's clickable container marked invoke-capable in the
   platform-neutral projection) while keeping its accessible Name, or add the
   `LegacyIAccessible`/`DoDefaultAction` pattern. Add provider tests asserting
   `GetPatternProvider(UIA_InvokePatternId)` returns a usable interface for a nav row and that
   invoking it switches the page.
8. Re-run the same vendoring evidence chain as §3.5 (patch regeneration, coverage guard exit 0,
   byte-identical replay) — one combined patch covering both gaps is acceptable and preferred.

### Driver-side bugs found and fixed during this investigation (for the record)

- `IUIAutomationElement` vtable: `GetCurrentPattern` moved from index 12 to the correct index 16.
- `BoundingRectangle` parse no longer silently returns zeros on a VARIANT type mismatch; it now
  reports `vt`/`data1` and is read via both `GetCurrentPropertyValue` and
  `GetCurrentPropertyValueEx(ignoreDefaultValue=TRUE)`.
- Rect assertions were moved to the end of the run so one execution yields the full evidence chain
  (nav invoke → semantics → Flow/7/Apply → readback → stale) before reporting rect failures.
