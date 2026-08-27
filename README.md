# Fcitx5 for Windows Next

[![Core](https://github.com/0x696c757a696f/fcitx5-windows-next/actions/workflows/core.yml/badge.svg)](https://github.com/0x696c757a696f/fcitx5-windows-next/actions/workflows/core.yml)
[![Developer Preview](https://img.shields.io/badge/status-Developer%20Preview-orange.svg)](#maturity-and-scope)
[![Windows](https://img.shields.io/badge/platform-Windows-0078D4.svg)](#requirements)
[![Rust + C++ adapter](https://img.shields.io/badge/implementation-Rust%20%2B%20C%2B%2B%20Fcitx%20adapter-informational.svg)](#architecture)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](https://spdx.org/licenses/GPL-3.0-or-later.html)

![Fcitx5 for Windows icon preview](resources/icons/fcitx5-icons-preview.png)

**A native Windows frontend and distribution layer for Fcitx5.**

**面向 Windows 的原生 Fcitx5 前端与发行层。** 目标是让用户在 Windows 中选择一个 Fcitx5 输入法配置，就能自然输入中文和其他语言，而无需手动管理后台进程。

> [!WARNING]
> **Developer Preview / 开发者预览版**
>
> This repository is actively evolving and is not a production-ready release. Real-host TSF, accessibility, installer/UAC, published package-repository, Authenticode, and Windows-version matrix evidence are still release gates.
>
> 本仓库仍在快速演进，尚非生产就绪版本。真实宿主 TSF、无障碍、安装器/UAC、已发布的软件包仓库、Authenticode 签名和 Windows 版本矩阵仍是发布门槛。

## What it is

Fcitx5 for Windows Next connects the Windows Text Services Framework (TSF) to the upstream Fcitx5 engine through a small, supervised process architecture. Fcitx5 Core remains authoritative for input-method and addon semantics; Windows-specific product state and operations are increasingly Rust-owned.

它把 Windows TSF 与上游 Fcitx5 引擎连接起来，并通过受监督的进程边界隔离输入路径、候选窗口和管理工具。Fcitx5 Core 是输入法与 addon 语义的权威来源；Windows 产品状态与操作逐步由 Rust 负责。

## Architecture

```text
Windows host
    |
    v
fcitx5-tsf.dll       Rust TSF boundary; bounded IPC; fail-open behavior
    |
    v
fcitx5-launcher.exe  lifecycle and process supervision
    |
    v
fcitx5-engine.exe    Fcitx5 Core, InputContext, addon/config integration
    |
    v
fcitx5-ui.exe        on-demand candidate window
```

- **Fcitx5 Core is the authority.** Pinyin, Rime, Mozc, and other input methods are internal Fcitx addons, not separate Windows TSF profiles.
- **Rust owns product logic.** Current Rust components cover the TSF, configuration core/backend, candidate model/layout/interaction, protocol, launcher, package/update/control, and related policy surfaces.
- **C++ is intentionally narrow.** The durable C++ island is the direct Fcitx object integration and thin Windows/native adapters. The shipping TSF is Rust; the old shipping C++ TSF has been removed.
- **Candidate UI is on demand.** Candidate data comes from the engine model; the UI does not commit text directly to host applications.

## Current capabilities

Implemented or under active automated verification:

- x86 and x64 TSF artifacts and registration paths;
- supervised launcher, engine, and candidate UI processes;
- versioned IPC with deadlines and fail-open handling;
- Rust-owned candidate model, layout, interaction, themes, and embedded Config preview contracts;
- Rust Config backend and the shipping `fcitx5-config.exe` Settings shell, with typed `Current`/`Draft`/`Defaults` transaction work;
- package, repository, update, and diagnostics primitives with ML-DSA-65 v2 package verification;
- CTest, Rust tests, source-contract, dependency, license, locale, and runtime-security checks.

插件、主题、翻译和输入法数据的模型与管理入口正在建设中。仓库包含受审查的包输入和本地验证路径，但**不代表官方在线插件仓库已经公开可用，也不代表完整在线安装/更新生命周期已经完成**。

## Maturity and scope

This is a development repository and a technical preview, not a promise of release readiness. In particular, this README does not claim:

- production Authenticode or repository signing;
- a published official plugin catalog or online plugin lifecycle;
- completed Narrator/NVDA evidence;
- a validated Windows 7, Windows 10, or Windows 11 real-host support matrix;
- UAC, installer, upgrade, or uninstall readiness;
- ARM64 as a current native Fcitx engine support claim.

代码中的自动化报告、预览截图和本地包生成结果不等于真实宿主或生产发布证据。安全边界、已知限制和漏洞报告方式见 [`SECURITY.md`](SECURITY.md)。

## Requirements

- Windows development host
- Visual Studio 2022 with MSVC C++ x86/x64 tools and the ATL component (`Microsoft.VisualStudio.Component.VC.ATL`)
- CMake 3.29 or newer
- PowerShell 7
- Rust toolchain supplied/prepared by the repository scripts

The repository uses pinned local toolchains under `out/toolchains` and `out/toolchains/fast`. Do not treat generated package artifacts as checked-in release assets.

## Build and test

Run these commands from the repository root in PowerShell 7:

```powershell
./tools/build.ps1 bootstrap
./tools/build.ps1 dev
./tools/build.ps1 test
```

`dev` and `test` target x64 and x86 by default. To select one architecture:

```powershell
./tools/build.ps1 dev -Architecture x64
./tools/build.ps1 test -Architecture x86
```

For a local release-shaped package and portable smoke checks:

```powershell
./tools/build.ps1 package -Architecture all -Configuration Release
```

`test` runs the configured CTest and policy gates. CI additionally exercises an ARM64 cross-build lane for static checks; that lane is not a current native Fcitx engine support promise.

## Project map

- `rust/` — Rust product cores, CLIs, TSF, Config, Candidate, package, and policy code
- `src/` — native Windows and direct Fcitx integration adapters
- `protocol/` — thin protocol marshalling boundary
- `tools/` — bootstrap, build, packaging, verification, and audit scripts
- `docs/` — engineering specification and task evidence
- [`SECURITY.md`](SECURITY.md) — security reporting and defensive boundaries
- [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) — dependency and notice inventory

Engineering queue and current truth: [`docs/tasks/current.md`](docs/tasks/current.md), [`docs/tasks/PLAN.md`](docs/tasks/PLAN.md), and [`docs/current.md`](docs/current.md). Before changing code, read [`AGENTS.md`](AGENTS.md). Security-sensitive reports should follow [`SECURITY.md`](SECURITY.md); third-party licensing details are in [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

The project release metadata declares **GPL-3.0-or-later**. Third-party components may carry their own licenses; use the generated license inventory for the complete record.

项目发行元数据声明许可证为 **GPL-3.0-or-later**。第三方组件可能使用各自许可证，完整记录以构建检查生成的许可证清单为准。
