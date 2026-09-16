# Fcitx5 for Windows Next

[![Core CI](https://github.com/0x696c757a696f/fcitx5-windows-next/actions/workflows/core.yml/badge.svg)](https://github.com/0x696c757a696f/fcitx5-windows-next/actions/workflows/core.yml)
[![Developer Preview](https://img.shields.io/badge/status-Developer%20Preview-orange.svg)](#status)
[![Windows](https://img.shields.io/badge/platform-Windows-0078D4.svg)](#build)
[![Rust + native adapters](https://img.shields.io/badge/implementation-Rust%20%2B%20native%20adapters-informational.svg)](#architecture)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)

[English](README.md) | [简体中文](README.zh-CN.md)

![Fcitx5 for Windows icon preview](resources/icons/fcitx5-icons-preview.png)

**Fcitx5, natively integrated with Windows.**

A native Windows frontend for Fcitx5, built around TSF, supervised Fcitx processes, a native candidate UI, and Rust-owned Windows product logic.

> [!WARNING]
> **Developer Preview:** This project is for development and evaluation. Local builds and automated tests do not by themselves establish release readiness.

## Why this project

- Windows sees one product-level TSF input-method entry instead of a separate TSF profile for every Fcitx addon.
- Input-method behavior comes from Fcitx5 Core and its addon ecosystem, including Rime, Pinyin, Mozc, and other Fcitx input methods.
- Windows-specific TSF, candidate, settings, launcher, package, and update logic is being consolidated in Rust.
- The input hot path stays separate from network download and update boundaries.

Fcitx5 Core remains the source of truth for input-method and addon behavior. This project is a Windows frontend and distribution layer, not a rewrite of Fcitx5.

## What you get

- Windows TSF integration
- Fcitx5 Core and addon runtime
- Native candidate window
- Settings application
- Launcher and process supervision
- Package, plugin, and update infrastructure
- Portable and installer packaging pipeline

The repository contains the local and automated foundations for transactional package activation and rollback. It does not claim that a public official online plugin catalog is already available.

## Architecture

```text
Windows applications
        │
        ▼
fcitx5-tsf.dll
Windows TSF / composition boundary
        │
        ▼
fcitx5-launcher.exe
process supervision
        │
        ├────────────► fcitx5-ui.exe
        │              candidate window
        ▼
fcitx5-engine.exe
Fcitx5 Core + addons
```

Settings, package management, updates, and downloads live outside the live input path. They do not receive live keystrokes, preedit text, candidate state, or commit history from input sessions.

## Status

The current codebase has substantial automated and local verification, and a current-HEAD portable package can be built locally. It is still behind the release evidence gates. Do not infer the following from local tests alone:

- real-host typing, accessibility, or UAC/install lifecycle;
- protected production signing or publication;
- the online package/plugin lifecycle;
- the declared Windows compatibility matrix;
- CI and release evidence.

The declared Windows compatibility matrix is not yet release-qualified across the required real-host environments. See [`docs/host-compatibility-matrix-2026-08-20.md`](docs/host-compatibility-matrix-2026-08-20.md) for the evidence record.

## Build

Requirements:

- Windows with Visual Studio 2022 C++ x86/x64 tools and ATL (`Microsoft.VisualStudio.Component.VC.ATL`)
- CMake 3.29 or newer
- PowerShell 7

The repository bootstrap manages the pinned Rust and supporting toolchains; contributors do not need to install the pinned versions by hand.

From the repository root:

```powershell
./tools/build.ps1 bootstrap
./tools/build.ps1 dev -Architecture x64
./tools/build.ps1 test -Architecture x64
```

To produce a local release-shaped package:

```powershell
./tools/build.ps1 package -Architecture all -Configuration Release
```

This produces local package artifacts; it is not equivalent to the protected signing and publication release gate.

## Repository layout

- `rust/` — Windows product logic and shipping Rust components
- `native-engine/` and `src/` — Fcitx-facing and native adapter boundaries
- `resources/` — generated original branding and product resources
- `tools/` — bootstrap, build, test, package, and release tooling
- `docs/` — contracts, verification matrices, and engineering evidence

## Documentation

- [`docs/current.md`](docs/current.md) — current implementation and evidence truth
- [`docs/product-contract.md`](docs/product-contract.md) — product and architecture contract
- [`docs/tasks/current.md`](docs/tasks/current.md) — current bounded task
- [`SECURITY.md`](SECURITY.md) — security model and reporting
- [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) — dependency notices
- [`AGENTS.md`](AGENTS.md) — contributor instructions

## License

The root project is released under **GNU GPL version 3 or any later version**; see [`LICENSE`](LICENSE). Some reusable Rust crates declare `LGPL-2.1-or-later`, and third-party components retain their own terms. The applicable license for each component is defined by its license files and [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
