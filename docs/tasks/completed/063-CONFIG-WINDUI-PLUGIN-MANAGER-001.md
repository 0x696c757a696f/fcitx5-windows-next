# Task 063 - Config WindUI Plugin Manager

**Mode:** CODE
**Task ID:** `CONFIG-WINDUI-PLUGIN-MANAGER-001`
**Evidence class:** `AUTOMATED`; real online repository lifecycle remains `MANUAL-PENDING`

## Completed behavior

- Displays all 21 top-level entries from the pinned `fcitx5-plugins` commit.
- Reads authoritative package and repository state from strict bounded `--packages-list` JSON.
- Runs refresh, install, update, state, remove, and repair through catalog-restricted typed Control
  arguments and the shared bounded Rust process executor.
- Uses `windui::App::channel` so package work runs off the UI thread and signal updates return to it.
- Rereads package state after successful mutation; errors never become local fake success.
- Keeps unsupported Windows artifacts disabled in a scrollable catalog with fixed detail/actions.
- Revalidated Candidate 150% typography with complete rows 4/5 CJK glyph visibility.

## Evidence

- x64/x86 Config, Candidate, and process-execution Rust tests.
- x64/x86 source contract and Candidate typography screenshot CTest.
- x64/x86 plugin-page screenshots.
- Adjusted strict clippy, text/dependency/license checks, and `git diff --check`.

Production signed repository artifacts, real online lifecycle, release signing, installer/UAC,
Narrator/NVDA, generation drain, and real-host matrix remain external/manual evidence.
