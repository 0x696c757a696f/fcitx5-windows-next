# Phase 6 acceptance

Date: 2026-08-17  
Specification: Frozen v1.5  
Scope: WTL Config, typed Control API, registration/repair, i18n, installer, Portable

## Evidence

- WTL 10.1.0 package is pinned by URL, size, and SHA-256; ATL is an explicit
  checked Visual Studio component. No IDE publish or GUI automation is used.
- x64 and x86 Release builds each pass 29/29 CTest cases with MSVC analysis.
- Config exposes `--self-test`, `--check-resources`, and `--check-i18n`; the
  corresponding headless tests pass.
- Config edits presentation only through `fcitx5-control.exe`; the typed TOML
  round-trip test proves unrelated geometry and color fields are preserved.
- Candidate preview launches the production C++ D2D/DWrite UI and feeds a fixed
  synthetic response through `CandidateModel` and the normal paint path.
- Runtime security, secret scan, license inventory, dependency inventory, strict
  locale JSON, and UTF-8-without-BOM/LF checks pass for the package gate.
- Real Fcitx engine normal/safe-mode startup and key-repeat checks pass for x64
  and x86 test clients against the single staged engine.
- The package gate produces the Inno installer and portable ZIP from the tested
  Release artifacts without rebuilding after tests.
- `tools/test-portable.ps1` extracts the real ZIP, runs Config and Control, moves
  the full tree, reruns both, and verifies `data_root` follows the moved tree.
- `tools/test-installer.ps1` completes elevated install, x64/x86 registration
  status, same-installer repair, and uninstall, then restores both development
  TSF registrations. The 2026-08-17 run passed.

## Artifacts from the accepted run

- `fcitx5-windows-0.1.0-portable.zip`
  - SHA-256: `F213B1712BF9242E4917222A55F89369CD63FF96335501B2332D2E1551BFF43F`
- `fcitx5-windows-0.1.0-setup.exe`
  - SHA-256: `4740C8356134E35E5F6E7F454E5AC4E2EDB77EA395B09A18C3F35546767FD16A`

These are unsigned package-gate artifacts, not Stable release artifacts.
Production signing, SBOM/provenance, channel identity, system-package metadata,
and previous-known-good deployment rollback remain Phase 8.
