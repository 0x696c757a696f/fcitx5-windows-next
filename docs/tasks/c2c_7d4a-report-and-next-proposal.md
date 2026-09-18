# c2c_7d4a 结项汇报 + 下一任务申请

> 交付对象：ChatGPT（作为设计/计划大脑）；执行方：Pi 执行代理。
> 本文分两部分：**(A) 结项汇报**、**(B) 下一任务申请**。

---

# A. 结项汇报 — c2c_7d4a（WINDUI-WIN32-RAW-UIA-PROVIDER）

## A0. 状态行（已落档）

```
WIN32 RAW UIA ROOT/TEXT/BUTTON PROVIDER AUTOMATED-GREEN
```

已写入 `docs/tasks/status.md`。**明确不是** "Settings accessibility complete"，**也不是** "Narrator/NVDA green"。

## A1. 目标与达成

**目标**：闭合 Win32 HWND → Raw UIA provider → OS UIA client 链路，使 vendored WindUI 在真实 HWND 上暴露 Root/Window/Text/Button 语义树，可导航、可 SetFocus、可 Invoke，并能由**独立进程**的 OS UIA 客户端验证。

**达成**：全部 9 个里程碑完成，双架构（x86_64 + i686）实测通过。

## A2. 交付物

| 文件 | 内容 |
|---|---|
| `third_party/wind-ui-rust/src/accessibility.rs` | 平台无关语义投影：`AccessibilityRole{Window,Text,Button}`、快照 flatten、focus/invoke 校验 |
| `third_party/wind-ui-rust/src/platform/win32/accessibility.rs` | 三个 COM provider：`ElementProvider`(Simple+Fragment)、`RootProvider`(+FragmentRoot)、`InvokeProvider`(IInvokeProvider)；UI 线程桥；属性/pattern/导航/焦点/取点实现 |
| `third_party/wind-ui-rust/src/platform/win32/mod.rs` | `WM_GETOBJECT` 仅认 `UiaRootObjectId`（谓词 `is_uia_root_object`），借用结束后才 `UiaReturnRawElementProvider` |
| `third_party/wind-ui-rust/examples/uia_smoke.rs` | 双进程真实 HWND OS UIA 冒烟（`--host` / client） |
| `third_party/wind-ui-rust/{app/mod.rs, core.rs, lib.rs, platform/mod.rs, ui/mod.rs, Cargo.toml}` | 语义快照 / action seam / 依赖特性（前序迭代所加，本次核验） |

**关键机制**：
- provider 持久状态仅 `{hwnd, target: Root|Node(id)}`；无 `state_from` 直调、无后台线程、无静态 registry、无 Mutex、无 `Tree` Send/Sync。
- 所有 property/navigation/action 经**同步 `SendMessageW(WM_APP+3)`** 回到拥有 HWND 的 UI 线程；唯一入口在 `wnd_proc` 消息分支。
- 属性只真实提供 Name/ControlType/IsEnabled/IsKeyboardFocusable/HasKeyboardFocus，其余返回空 variant。
- pattern 仅在 Button 且支持 Invoke 时给 `IInvokeProvider`。
- bounds：`GetDpiForWindow()/96` 换算 + `ClientToScreen`，逻辑坐标不回写快照。
- child runtime id = `UiaAppendRuntimeId` + `NodeId.index` + `generation`；Root 走 host 规则返回 `E_NOTIMPL`。
- 失败语义冻结：`Disabled→UIA_E_ELEMENTNOTENABLED`、stale/not-visible→`UIA_E_ELEMENTNOTAVAILABLE`。

## A3. 本次修复的阻塞缺陷（真实运行时崩溃）

前序代码的四个 `no_*_provider()` 助手用 `unsafe { zeroed() }` 构造 COM 接口。windows-rs 0.62 的接口类型是 `NonNull` 包装，`zeroed()` 不合法 → 运行时

```
attempted to zero-initialize type `IRawElementProviderFragment`, which is invalid
thread caused non-unwinding panic. aborting.
```

→ host 进程 abort，UIA 树被打崩（root 退化成 Pane 50033，client 断言失败）。

**修复**：改为 `no_element_error()`（`E_FAIL`）的失败 HRESULT 语义——该 ABI 下空接口不可表达，失败正是 UIA 期望的"无此元素"。同时修掉 `UiaReturnRawElementProvider` 的 `Param<T>` 需传 `&provider`（E0277）。

## A4. 验证证据（可复现）

| 项目 | x86_64 | i686 |
|---|---|---|
| vendored WindUI full tests | 2 suites / **910 passed** / 0 failed | 2 suites / **910 passed** / 0 failed |
| 双进程 OS UIA smoke | **exit=0** | **exit=0** |
| fcitx5-config-poc 构建 | 0 error | 0 error |
| fcitx5-config-poc tests | **66 passed** | **66 passed** |
| fcitx5-candidate-core tests | **116 passed** | — |

smoke 实测输出（两架构一致）：

```
uia_smoke OK: root=Window, 9 descendant(s) [(50037) | 系统(50010) | 系统(50011) | 最小化(50000) | 最大化(50000) | 关闭(50000) | Accessibility smoke(50020) | Invoked 0(50020) | Invoke me(50000)], Invoke produced Invoked 1
stale element reported: 元素无法使用 (0x80040201)
```

即：root=Window；枚举到静态/动态 Text 与 Button；**永久 hidden 按钮未被暴露**；Invoke 后动态标签变 `Invoked 1`；宿主关闭后 stale 查询返回 `UIA_E_ELEMENTNOTAVAILABLE`。

`git diff --check` 干净；未整体重排 `platform/win32/mod.rs`。

> 注：独立审计方复跑时读到 802/830（其构建的 exe 时间戳早于最后一次源码编辑），我方测得 910；两方均为 **0 failed**。测试计数差异不影响目标达成（计数不在验收标准内）。

## A5. 边界（明确未做，非本目标范围）

未改 `rust/config-poc` 产品 UI；未做 Config HWND smoke；未实现 Selection/SelectionItem、Toggle、ComboBox、Value/Text patterns、UIA 事件通知、Narrator/NVDA；未验证真实朗读器与多语言 Name/bounds。

## A6. ⚠️ 第三方代码与上游同步风险（重要）

### 事实

`third_party/wind-ui-rust` 是 `huanfeng/wind-ui-rust` 的**扁平 vendored 副本**（作为 Rust path dependency）。上游同步机制是 `tools/sync-windui.ps1`，其补丁列表**硬编码**：

```powershell
$patches = @(
  (Join-Path $patchDir 'win32-window-user-data.patch'),
  (Join-Path $patchDir 'win32-tray-unaligned.patch')
)
# 缺任一 patch 直接 throw
```

### 风险

本次与前序迭代对 vendored 树的改动（`accessibility.rs`、`platform/win32/accessibility.rs`、`app/mod.rs`、`core.rs`、`lib.rs`、`platform/mod.rs`、`platform/win32/mod.rs`、`ui/mod.rs`、`Cargo.toml`、`examples/uia_smoke.rs`）**都不在补丁列表内**。执行 `sync-windui.ps1` 会：

1. 拉取上游 commit 并把 vendored 树重置为上游状态；
2. 只重放那 2 个旧补丁；
3. → **本次全部 UIA 工作丢失**（且脚本不会报错，因为它的 patch 校验只覆盖已登记的两个）。

**这不是"改了第三方就不能同步"，而是"改动未登记为补丁，同步会静默丢弃"** —— 属于必须优先偿还的债务。

### 缓解（已在下一任务申请中列为 P1）

把 vendored 树相对上游的全部差异导出为补丁（含新增文件），放进 `third_party/patches/wind-ui-rust/`，并注册进 `sync-windui.ps1` 的 `$patches`；随后用一次 `sync-windui.ps1 --Latest`（或同版本重建）验证"重置 + 重放全部补丁"能回到当前状态。

### 附带风险：全部工作未提交

当前 30+ 个文件（含前序 Codex 的 UIA/本地化/滚动工作在制品）**全部未提交**。本会话内已发生过一次子代理误 `git checkout --` 清掉他人改动的事故。**建议在任何后续任务前先提交或至少打 tag/分支**。

---

# B. 下一任务申请

> 供 ChatGPT 设计计划。以下为建议的优先级、范围、验收标准与约束；请据此产出可执行计划后由 Pi 执行。

## B1. 背景（给设计者的现状摘要）

- WindUI 的 Win32 raw UIA provider 已由**跨进程 OS UIA 客户端**证明可用（Root/Text/Button、Invoke、SetFocus、stale 语义）。状态行：`WIN32 RAW UIA ROOT/TEXT/BUTTON PROVIDER AUTOMATED-GREEN`。
- 该 provider 尚未接到**真实产品界面**。产品侧 Settings 是 `rust/config-poc`（Rust），Candidate 窗口宿主是 `rust/candidate-core`。
- 仓库规则：新 Windows 产品代码默认 Rust；C++ 仅限 Fcitx 适配岛与必要 Win32/COM seam；纯 safe crate 默认 `#![forbid(unsafe_code)]`，FFI/Win32 文件需 `#![deny(unsafe_op_in_unsafe_fn)]` + 窄 unsafe + SAFETY 注释。
- 并发写作者存在（另一 Codex 会话在同一工作树工作）；任务需自带"先提交/隔离"前置。

## B2. 候选任务（按优先级）

### P1 — 把 vendored WindUI 改动固化为可重放补丁（债务偿还，最高优先）

**为什么现在做**：不做则一次上游同步即静默清空 UIA 工作；且这是后续任何 wind-ui 工作的前置。

**范围**：
1. 生成差异补丁（相对 vendored 基线 commit）覆盖全部改动文件与**新增文件**（`accessibility.rs`、`platform/win32/accessibility.rs`、`examples/uia_smoke.rs`、`Cargo.lock` 视需要）。
2. 存入 `third_party/patches/wind-ui-rust/`，命名遵循现有风格。
3. 注册进 `tools/sync-windui.ps1` 的 `$patches`（保持其"缺补丁即 throw"的失败即停语义）。
4. 验证：在**干净副本**上重放"上游 checkout + 全部补丁"，结果与当前工作树一致（文件级 diff 为空），且 `cargo test -p windui`（双架构）与 `uia_smoke`（双架构）仍绿。

**验收**：`sync-windui.ps1` 的补丁列表覆盖全部本地改动；模拟同步（不推到远端）后工作树与现状一致；双架构测试与 smoke 绿。

**风险**：新增文件补丁需 `/dev/null` 形式；`Cargo.lock` 是否纳入需决策（我倾向纳入以免依赖漂移）。

### P2 — 提交/固化当前工作树

**范围**：把当前 30+ 文件的在制品提交（或至少分支+tag），把 Codex 的 UIA/本地化/滚动工作与本次修复分开或合并成有意义的历史。

**验收**：`git status` 干净或在受控分支；无未跟踪的源码文件散落。

### P3 — 把已证明的 provider 接到真实 Settings（C2C 原文所指的"下一阶段"）

**范围**（沿用前一阶段边界，不扩大）：
- 让真实 shipping Config 窗口的**候选布局 / 页大小按钮**由该 provider 暴露；
- 多语言 Name 与 bounds/截断检查；
- 不允许引入 Selection/Toggle/ComboBox/Value/Text patterns、UIA 事件通知、Narrator/NVDA（留给再下一阶段）。

**验收建议**：真实 Config 窗口的双进程 OS UIA smoke（同 `uia_smoke` 手法：ElementFromHandle + 枚举 + Invoke + stale），x64/x86 双通过；且不破坏现有 Settings 视觉/交互测试。

**前置**：需要明确"真实 Config 窗口"当前由谁创建（Rust `config-poc` 还是 WTL 遗留 shell），以及其可测窗口标题/坐标。

### P4 — Candidate UI 遗留项复核（低优先，未验证）

本会话内有两项候选 UI 修复**已落地但未提交、未做真实宿主验证**：
- 选中项 fallback 撞色（`selected_candidate_text` 默认值曾与 `selected_background` 同色）；
- 竖排测量/渲染不一致导致首字后裁切（`VERTICAL_GLYPH_STEP_RATIO` + grapheme cluster 化）。

第三项（预览画布 `Contain` 拉伸导致比例脱节）的目标文件 `settings_preview.rs` **已被前序改动删除**，修复随之失效 —— 需先确认预览功能现在由谁承载（`theme_tokens.rs`？`config-poc` 内部？）再决定是否重做。

**验收建议**：三项各自的最小像素回归（"完整字形可见"）+ 真实宿主截图。

## B3. 约束（给设计者）

1. 不得改动 Fcitx C++ 适配岛以外的新 C++ 产品逻辑。
2. unsafe 边界规则（见 B1）必须保持；新增 FFI 文件需显式例外并带 SAFETY。
3. 不得为本任务引入新的第三方依赖，除非有明确记录与理由。
4. 每个任务需给出：可机器验证的验收命令、失败即停的阻塞条件、以及"不做什么"的边界。
5. 任何"无法自动验证"的项必须显式标为 MANUAL-PENDING，不得记为绿。
6. 工作树当前存在并发写作者痕迹，任务开始前须先做提交/隔离决策（见 P2）。

## B4. 需要设计者决策的开放问题

1. `Cargo.lock` 是否纳入 wind-ui 补丁集？
2. P3 的"真实 Config 窗口"当前归属（Rust 还是 WTL 遗留 shell），以及其可自动化的窗口识别方式。
3. P4 的预览功能现状（谁承载渲染、原 `settings_preview.rs` 的替代者），决定第三项是否重做。
4. 是否需要为 UIA provider 增加"事件通知"作为独立后续阶段（RAII 事件、Narrator 依赖它）。
