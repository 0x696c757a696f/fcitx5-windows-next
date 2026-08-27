# Fcitx5 for Windows Next

[![Core CI](https://github.com/0x696c757a696f/fcitx5-windows-next/actions/workflows/core.yml/badge.svg)](https://github.com/0x696c757a696f/fcitx5-windows-next/actions/workflows/core.yml)
[![Developer Preview](https://img.shields.io/badge/status-Developer%20Preview-orange.svg)](#maturity-and-scope)
[![Windows](https://img.shields.io/badge/platform-Windows-0078D4.svg)](#requirements)
[![Rust + C++ adapter](https://img.shields.io/badge/implementation-Rust%20%2B%20C%2B%2B%20Fcitx%20adapter-informational.svg)](#architecture)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)

[English](README.md) | [简体中文](README.zh-CN.md)

![Fcitx5 for Windows icon preview](resources/icons/fcitx5-icons-preview.png)

**A native Windows frontend and distribution layer for Fcitx5.**

Fcitx5 for Windows Next brings Fcitx5 input methods to the Windows Text Services Framework (TSF), with a supervised process boundary, a native candidate experience, and an evolving management surface for configuration, packages, plugins, and themes.

> [!WARNING]
> **Developer Preview:** This repository is actively evolving and is not a production-ready release. Real-host TSF, accessibility, installer/UAC, online plugin lifecycle, production signing, and Windows-version matrix evidence remain release gates.

## What is here

- Fcitx5 Core remains authoritative for input-method and addon semantics. Pinyin, Rime, Mozc, and other input methods are Fcitx addons rather than separate Windows TSF profiles.
- The shipping TSF boundary is Rust, with bounded IPC and fail-soft behavior around the input path.
- Candidate data, layout, interaction, themes, and preview contracts are Rust-owned; the candidate UI is shown on demand and does not commit text directly to host applications.
- Rust configuration, package, update, control, launcher, and policy cores provide the product-owned foundations. Settings migration and transaction work continue in the task queue.
- Plugin and theme management models are under active development. A public official online catalog and complete install, update, rollback, and uninstall lifecycle are not yet available.

## Architecture

```text
Windows host
    |
    v
fcitx5-tsf.dll       Rust TSF boundary; bounded IPC; fail-soft behavior
    |
    v
fcitx5-launcher.exe  lifecycle and process supervision
    |
    v
fcitx5-engine.exe    Fcitx5 Core and direct addon/config integration
    |
    v
fcitx5-ui.exe        on-demand candidate window
```

Product-owned Windows state, validation, operations, parsing, package/update logic, candidate semantics, and UI-domain behavior default to Rust. C++ is intentionally limited to direct Fcitx object integration and thin native/Windows adapter seams. The old authoritative C++ TSF implementation has been removed.

The input plane has no network capability. Network access belongs to an isolated downloader boundary, which cannot receive preedit text, candidates, key events, or commit history. See [`SECURITY.md`](SECURITY.md) for the defensive security model and reporting route.

## Maturity and scope

This is a technical preview for development and evaluation. Automated tests and local artifact checks do not establish release readiness. The repository does not currently claim:

- a production Authenticode or repository-signing program;
- a published official plugin catalog or online plugin lifecycle;
- completed Narrator or NVDA evidence;
- a validated Windows 7, Windows 10, or Windows 11 real-host matrix;
- installer, upgrade, uninstall, or UAC readiness;
- native ARM64 Fcitx engine support.

## Requirements

- Windows development host
- Visual Studio 2022 with MSVC C++ x86/x64 tools and ATL (`Microsoft.VisualStudio.Component.VC.ATL`)
- CMake 3.29 or newer
- PowerShell 7
- The repository-prepared Rust toolchain

Run `./tools/build.ps1 bootstrap` to prepare and manage the repository's local toolchains under `out/toolchains`. Generated build and package outputs are not checked-in release assets.

## Build and test

From the repository root in PowerShell 7:

```powershell
./tools/build.ps1 bootstrap
./tools/build.ps1 dev
./tools/build.ps1 test
```

Select an architecture when needed:

```powershell
./tools/build.ps1 dev -Architecture x64
./tools/build.ps1 test -Architecture x86
```

The release-shaped package command requires all supported native build lanes and Release configuration:

```powershell
./tools/build.ps1 package -Architecture all -Configuration Release
```

`test` runs the configured CTest and policy gates. CI also has an ARM64 cross-build/static-check lane; that lane is not a native Fcitx engine support promise.

## Project map

| Path | Purpose |
| --- | --- |
| `rust/` | Rust product cores, CLIs, TSF, Config, Candidate, package, and policy code |
| `src/` | Native Windows and direct Fcitx integration adapters |
| `protocol/` | Thin protocol marshalling boundary |
| `tools/` | Bootstrap, build, packaging, verification, and audit scripts |
| `docs/` | Engineering specification and task evidence |
| `resources/` | Icons and other project resources |

The engineering queue and current implementation truth are in [`docs/tasks/current.md`](docs/tasks/current.md), [`docs/tasks/PLAN.md`](docs/tasks/PLAN.md), and [`docs/current.md`](docs/current.md). Read [`AGENTS.md`](AGENTS.md) before changing code. Security reporting is described in [`SECURITY.md`](SECURITY.md), and dependency notices are in [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

The root project is released under **GNU GPL version 3 or any later version**; see [`LICENSE`](LICENSE). Some reusable Rust crates declare `LGPL-2.1-or-later`, and third-party components retain their own terms. The applicable license for each component is defined by its file and [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
