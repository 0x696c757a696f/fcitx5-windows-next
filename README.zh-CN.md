# Fcitx5 for Windows Next

[![Core CI](https://github.com/0x696c757a696f/fcitx5-windows-next/actions/workflows/core.yml/badge.svg)](https://github.com/0x696c757a696f/fcitx5-windows-next/actions/workflows/core.yml)
[![开发者预览版](https://img.shields.io/badge/status-Developer%20Preview-orange.svg)](#当前状态)
[![Windows](https://img.shields.io/badge/platform-Windows-0078D4.svg)](#构建)
[![Rust + 原生适配器](https://img.shields.io/badge/implementation-Rust%20%2B%20native%20adapters-informational.svg)](#架构)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)

[English](README.md) | [简体中文](README.zh-CN.md)

![Fcitx5 for Windows 图标预览](resources/icons/fcitx5-icons-preview.png)

**让 Fcitx5 真正融入 Windows。**

面向 Windows 原生体验构建的 Fcitx5 前端：通过 TSF 接入系统，运行真正的 Fcitx5 Core 与插件，并提供原生候选窗口、设置与软件包管理能力。

> [!WARNING]
> **开发者预览版：** 本项目用于开发和评估。仅凭本地构建和自动化测试，不能证明已经达到发行条件。

## 它解决什么问题

- Windows 看到的是一个产品级 TSF 输入法入口，而不是每个 Fcitx addon 都单独注册一个 TSF 配置文件。
- 输入法本身的行为来自 Fcitx5 Core 及其 addon 生态，包括 Rime、拼音、Mozc 和其他 Fcitx 输入法。
- Windows 特有的 TSF、候选窗口、设置、启动器、软件包和更新逻辑正在向 Rust 收敛。
- 输入热路径与网络下载、更新边界分离。

输入法行为和 addon 语义仍由 Fcitx5 Core 决定。本项目是 Windows 前端和发行层，不是重写 Fcitx5。

## 目前包含

- Windows TSF 集成
- Fcitx5 Core 与 addon 运行时
- 原生候选窗口
- 设置应用
- 启动器与进程监督
- 软件包、插件与更新基础设施
- Portable 与安装器构建管线

仓库包含事务式软件包激活与回滚的本地和自动化基础，但不声称官方公开在线插件目录已经可用。

## 架构

```text
Windows 应用
        │
        ▼
fcitx5-tsf.dll
Windows TSF / 组合边界
        │
        ▼
fcitx5-launcher.exe
进程监督
        │
        ├────────────► fcitx5-ui.exe
        │              候选窗口
        ▼
fcitx5-engine.exe
Fcitx5 Core + addons
```

设置、软件包管理、更新和下载位于实时输入路径之外，不接收输入会话中的真实按键、预编辑文本、候选状态或提交历史。

## 当前状态

当前代码库已经有较充分的自动化和本地验证，也可以在本地构建当前 HEAD 的 portable 软件包；但项目仍未通过发行证据门槛。不能仅凭本地测试推断以下事项已经完成：

- 真实宿主打字、无障碍或安装/UAC 生命周期；
- 受保护的生产签名或发布流程；
- 在线软件包/插件生命周期；
- 声明中的 Windows 兼容性矩阵；
- CI 和发行证据。

声明的 Windows 兼容性矩阵仍需在要求的真实宿主环境中完成发行级验证。具体证据记录见 [`docs/host-compatibility-matrix-2026-08-20.md`](docs/host-compatibility-matrix-2026-08-20.md)。

## 构建

环境要求：

- Windows，以及 Visual Studio 2022 C++ x86/x64 工具链和 ATL 组件（`Microsoft.VisualStudio.Component.VC.ATL`）
- CMake 3.29 或更高版本
- PowerShell 7

仓库的 bootstrap 会管理固定版本的 Rust 和配套工具链；贡献者不需要手动安装这些固定版本。

在仓库根目录执行：

```powershell
./tools/build.ps1 bootstrap
./tools/build.ps1 dev -Architecture x64
./tools/build.ps1 test -Architecture x64
```

生成本地发行形态软件包：

```powershell
./tools/build.ps1 package -Architecture all -Configuration Release
```

这会生成本地发行形态构件，但不等同于受保护的生产签名与发布流程。

## 仓库结构

- `rust/` — Windows 产品逻辑和随产品发布的 Rust 组件
- `native-engine/` 与 `src/` — Fcitx 对接及原生适配边界
- `resources/` — 自动生成的原创品牌和产品资源
- `tools/` — bootstrap、构建、测试、打包与发行工具
- `docs/` — 合同、验证矩阵和工程证据

## 文档

- [`docs/current.md`](docs/current.md) — 当前实现和证据事实
- [`docs/product-contract.md`](docs/product-contract.md) — 产品与架构合同
- [`docs/tasks/current.md`](docs/tasks/current.md) — 当前边界明确的任务
- [`SECURITY.md`](SECURITY.md) — 安全模型与报告方式
- [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) — 依赖声明
- [`AGENTS.md`](AGENTS.md) — 贡献者说明

## 许可证

根项目采用 **GNU GPL version 3 或任何更高版本**，详见 [`LICENSE`](LICENSE)。部分可复用 Rust crate 采用 `LGPL-2.1-or-later`，第三方组件继续遵守各自许可证。每个组件适用的许可证以其许可证文件和 [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) 为准。
