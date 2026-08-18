# Local verification record — 2026-08-17

> Status: historical v1.5 evidence. It is not current v1.6 acceptance and must not be used to skip a Phase gate.

Specification: Frozen v1.5  
Task contract: CHANGE plus local release-readiness verification  
Environment: Windows 10.0.19044, interactive user desktop, PowerShell 7  
Result: local PR, Package, installer and Desktop gates passed; Stable Release gate not eligible

## Verified lineage

- Source commit recorded by the stage: `7efc0a03f4911c25d6106c24f6e8fc62a9c020ad`
- Source tree: dirty; the uncommitted implementation is intentionally not represented as a Stable
  source release.
- Stage: `out/package/stage-7224fe3f8a344d00ac418855f78de5d0/Fcitx5`
- Architecture: x64 product with x64 and x86 TSF DLLs
- Config binary SHA-256: `b74aab3ad9a35f67206ea4ba6a3a4a3aeb51e62e83a76a834164bf7397365b10`

## Automated results

| Gate / case family | Result |
|---|---|
| x64 Release CTest | 44/44 passed |
| x86 Release CTest | 44/44 passed |
| Config complete interaction sweep | 19/19 unique Button HWNDs clicked; Apply 3/3 page branches; Install/Update 2/2; Enable/Disable 2/2; all combo entries and reversible toggles exercised |
| Protocol/package fuzz smoke | passed on x64/x86; malformed, oversized, path and signature cases retained |
| Real Fcitx integrations | Pinyin, Rime, Rime Lua, fcitx5-lua, Chinese Addons and chttrans functional assertions passed |
| Typing robustness | 300 continuous rounds / 900 keys and fixed-seed 4,000-event stateful fuzz passed |
| Portable | real ZIP extract/run/move/run and user-data-preserving upgrade passed |
| Installer | install, x64/x86 registration, same-installer Repair, installed Config interaction sweep, uninstall and prior-state restoration passed |
| Desktop | real tray icon/popup actions, Settings/Diagnostics, engine restart, pause/resume, exit, Notepad `ni -> 你` and absent-engine `abc` fail-open passed |
| Static/policy | MSVC analysis/warnings-as-errors, runtime capability/import audit, secret, license, dependency, locale and UTF-8-without-BOM/LF checks passed |

Desktop evidence is machine-readable at `out/evidence/desktop-verification.json`. The desktop runner
also covers a stale workspace stage: it asks that stage's own Control API to shut down and fails if
the bounded graceful transition does not complete, preventing a different Stable singleton from
silently satisfying the test.

## Unsigned local artifacts

| Artifact | Size | SHA-256 |
|---|---:|---|
| `fcitx5-windows-0.1.0-portable.zip` | 59,692,385 | `7A7F2FD6872AA93F0090CA7755B6FBB46A67D5B3B4480EDEA7BFDC87896D8A56` |
| `fcitx5-windows-0.1.0-setup.exe` | 50,528,177 | `50952B5D64A3A681FF916643FA92BD1B10A6CABEF8B58619CC9ECDA783815B14` |

These are Package-gate artifacts for local testing. They are not Stable public-release artifacts.

## Release blockers that cannot be fabricated locally

- The worktree has no clean reviewed/tagged release commit and no configured Git remote.
- No protected Authenticode certificate/timestamping authority is available.
- The shipped production package keyring is intentionally empty and no signed production package
  repository/service is provisioned; real online refresh/download/update/uninstall therefore fails
  closed by design.
- The required Win7 Legacy VM, Windows 11 primary matrix, Word/Excel, Edge/Chrome, VS Code, Windows
  Terminal, RDP/multi-session, physical multi-DPI/multi-monitor, NVDA/Narrator and legacy/fullscreen/
  protected-application environments do not all exist on this machine.

The Release gate must remain red until these inputs and same-lineage environment results exist. A
mock, local test key, screenshot or unsigned installer cannot close those rows.
