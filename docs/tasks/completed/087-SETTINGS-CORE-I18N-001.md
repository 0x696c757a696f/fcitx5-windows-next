# 087 — Complete WindUI Settings localization

**Status:** completed / automated and visual evidence green. REL-01 remains a
separate manual-pending release gate.

## Goal

Finish consistent localization of every product-owned visible string in the
shipping WindUI Settings surface for English, Simplified Chinese, Traditional
Chinese, Japanese, Korean, Vietnamese, Thai, and Sinhala.

## Scope

- Reuse `locales/*.json` as the only translation source; add the required keys
  to all eight catalogs with strict JSON and identical key sets.
- Localize remaining window/title/search/sidebar and navigation descriptions,
  Input Methods, Appearance and theme controls, Shortcuts, shared Settings
  buttons/status/footer/toasts/dialogs, Updates, Diagnostics/Repair, and the
  add-on/plugin manager's product-owned labels and operation messages.
- Keep upstream/plugin-provided names and metadata as source content; localize
  surrounding product UI without corrupting or guessing upstream terminology.
- Use English fallback for missing catalog entries and keep user data, plugin
  identifiers, protocol values, and persisted configuration locale-neutral.
- Ensure accessible names/help text use the same localized source as visible
  controls. Do not claim Narrator/NVDA evidence from screenshots or tests.
- Add Rust tests for catalog key coverage and UI string resolution. Preserve
  explicit source/literal exceptions only for identifiers, hotkeys, and
  upstream-provided content; no user-facing Simplified-Chinese literals may
  remain silently embedded in the WindUI Settings construction path.
- Build the real Settings executable and capture/review all six pages in every
  locale at default and minimum supported window sizes. Verify long labels,
  dialogs/toasts where screenshot automation supports them, scrolling, no
  clipping/overlap, and correct locale selection without mutating real user
  settings.

## Out of scope

- WindUI vendor changes, new localization framework/dependency, translation of
  external plugin metadata, candidate rendering behavior, and release-host or
  screen-reader claims.
- Production Authenticode/UAC/online repository evidence tracked under REL-01.

## Acceptance

- All eight locale catalogs are valid strict UTF-8 JSON with matching keys;
  missing translations have an explicit English fallback.
- All product-owned visible text on the six Settings pages and shared shell is
  localized; locale-neutral IDs and upstream content remain unchanged.
- Rust tests cover the localized UI contract and every translation lookup used
  by the shipping shell.
- x64 and x86 affected Rust lanes pass where supported.
- Real-window screenshot evidence covers each page and locale at default and
  minimum sizes, with visual review documenting any remaining non-blocking
  limitation. No screenshot is represented as screen-reader or release proof.
- Safe Rust remains `#![forbid(unsafe_code)]` unless code is an explicitly
  narrow native/ABI boundary.

## Required environment

- PowerShell 7: `D:\Program Files\PowerShell\7\pwsh.exe`
- Toolchain root: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains`
- Fast tools: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast`
- Pinned Cargo: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home\bin\cargo.exe`
- Keep `CARGO_TARGET_DIR` inside this worktree.

## Completion evidence

- All eight locale catalogs have matching keys and pass strict JSON validation.
- Config UI tests pass 40/40 on x64 and 40/40 on x86; the shipping Settings
  executable builds on x64; rustfmt and `git diff --check` pass.
- Captured and visually reviewed 96 real-window screenshots under
  `out/evidence/task-087-locales` (six pages × eight locales × 1040×700 and
  900×620). Preview locale selection did not save or change the user's setting.
- Screenshot review caught and corrected narrow-locale layout compression:
  candidate controls stack below translated labels; plugin actions use one
  column, long catalog rows ellipsize with a full-name tooltip, and plugin
  details scroll independently. Contact sheets cover every page/locale/size.
- Shortcuts, Updates, and Diagnostics/Repair remain explicitly unavailable
  placeholder pages; this task localizes those states but does not implement
  their product behavior. No toast/dialog action was invoked because doing so
  would execute a plugin operation; internal plugin errors are localized and
  covered by Rust regression tests.
- Screenshots do not establish Narrator/NVDA, accessibility-host, online
  signed-plugin/Rime lifecycle, release signing/UAC, Windows 7, constrained
  host, CI, or publication evidence. Those remain tracked separately under
  REL-01.
