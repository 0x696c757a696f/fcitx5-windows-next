# ADR 0003: WTL configuration frontend

- Status: Accepted
- Date: 2026-08-17
- Decider: Project owner
- Governing specification: Frozen v1.6 (decision retained)

## Context

Phase 6 requires an on-demand native settings application that participates in
the same unattended CMake/MSVC build and Windows 7 Legacy lane as the core. The
original aardio option failed bounded IDE-side publication probes and has no
supported standalone headless publisher. wxWidgets and Slint add GUI/runtime
stacks that are not needed by this Windows-only management surface.

## Decision

`fcitx5-config.exe` uses C++ with WTL 10.1/ATL and native Win32 controls. WTL is
restored from the pinned NuGet package published for the official WTL 10.01
release and verified by size and SHA-256. ATL is an explicit Visual Studio Build
Tools component checked by bootstrap. No IDE wizard, designer, GUI automation,
or publish step is part of the build.

| Component | Implementation |
|---|---|
| TSF | C++ / Win32 / COM / TSF |
| Engine | C++ / Fcitx5 |
| Candidate UI | C++ / Win32 / Direct2D / DirectWrite |
| Skin system | Strict TOML + bounded assets |
| Config | C++ / WTL / ATL / Win32 |
| Candidate preview | Production Candidate renderer with synthetic data |
| Launcher/package/updater | C++ |
| Animation | DirectComposition, optional and deferred |

Config calls only the typed Control/config API. It does not parse a second
configuration schema, observe live input, own package resolution, or perform
network operations.

## Options considered

| Option | Result |
|---|---|
| WTL/ATL | Selected: small native Windows-only management UI and controlled Legacy lane |
| Pure Win32 | Rejected for Config: duplicates layout/control lifecycle plumbing |
| wxWidgets | Rejected: cross-platform Config is not a current requirement |
| Slint/Rust | Rejected: not a formal runtime dependency under the v1.6 Legacy policy |
| aardio | Rejected: deterministic unattended publication was not demonstrated |

## Consequences

- ATL is a declared build component; clean runners fail with an actionable error
  if it is absent.
- WTL is confined to Config/necessary management windows and never enters TSF,
  engine, or Candidate rendering hot paths.
- The first surface remains Basic appearance/settings plus Diagnostics and Repair.
- Theme preview must reuse the production renderer instead of forking paint logic.
- Reconsideration requires the evidence listed in specification v1.6 section 13.6.2.
