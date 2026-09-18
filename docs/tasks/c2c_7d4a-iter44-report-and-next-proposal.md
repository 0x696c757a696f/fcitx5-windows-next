# c2c_7d4a ITERATION 44 结项汇报 + 下一任务申请

> 交付对象：ChatGPT（设计/计划大脑）；执行方：Pi 执行代理。
> (A) 结项汇报（已完成并审计通过）；(B) 下一任务申请（P3，待 ChatGPT 设计）。

---

# A. 结项汇报 — ITERATION 44 / WINDUI-VENDORED-REPLAYABILITY

## A0. 结论

vendored WindUI 的本地差异已**逐字节证明可从「确切上游 commit + 已登记补丁」重建**，恢复点已在隔离分支建立；原脏并发工作树全程只读、未被改动。**审计通过（独立复核并重放成功）**。

## A1. 关键凭证

| 项 | 值 |
|---|---|
| 恢复提交 | `4ead534bb5634598bd09f63da62d56400380fea7` |
| 分支 | `c2c/windui-replayability-iter44`（隔离 worktree：`D:\Documents\GitHub\fcitx5-windows-next-iter44`） |
| 提交信息 | `vendor: make WindUI local delta replayable` |
| 提交路径数 | **14**（全部白名单，无越界） |
| 上游 WindUI pin | `980eb5ef132d2aacc0ae801e7cdc11a433f88206` |
| catch-up patch | `third_party/patches/wind-ui-rust/windui-local-product-delta.patch`，79160 B |
| patch SHA-256 | `26FF6699BCD73145AE9B021820C304DA5348727BD2288D5AC43CEC155B0BA918` |
| replay manifest SHA-256 | `4D4669B16AFE9C7AF1C78A586574EB21220B47DBB6E53E7A61CD20DFDF572F81`（128 行） |

## A2. 重放证明（核心）

- 隔离 worktree 的 vendored WindUI 先恢复为干净 HEAD（125 文件），随后**只经**官方 `pwsh tools/sync-windui.ps1 -Commit 980eb5ef132d2aacc0ae801e7cdc11a433f88206` 重建 → `SYNC_EXIT=0`。
- 重建结果与 Phase 0 原工作树 snapshot 做逐文件 SHA-256 比较：**128 vs 128 文件、零差异** → `CURRENT_SNAPSHOT == SYNC_REPLAY_RESULT`。
- 未在重放后手工补任何文件。排除项仅生成/未跟踪的 `third_party/wind-ui-rust/Cargo.lock`（不在 vendoring 范围 `src/examples/Cargo.toml/README*/LICENSE*`，且主仓为 untracked 生成物，按既定规则不入 patch；两侧一致排除）。
- 审计方独立复现：外部临时目录 clone 上游 → checkout pin（168 路径）→ 按脚本顺序 `git apply` 三个补丁 + 同样 LF 归一化 → `diff -r` 与主树/隔离树的 vendored 内容**零差异**。

## A3. 交付物

1. **catch-up patch**：从「pin + 既有两 patch」到当前 vendored 状态的完整差异（11 个变更路径、`+1826/−12`、无删除、无 mode change、3 个新文件为标准 `--- /dev/null` 形式、patch 内不含 `Cargo.lock`）。语义是「一个补丁补齐全部本地产品差异」，未追溯拆分 Iteration 41/42/43 历史。
2. **同步脚本注册**：`tools/sync-windui.ps1` 的 `$patches` 追加该 patch 为**最后一项**（因其基线正是前两 patch），保留原有「任一登记 patch 缺失即 throw」的 fail-fast，**未**增加「不存在则跳过」容错。
3. **机器检查** `tools/verify-windui-vendor-coverage.ps1`：守「新增本地 WindUI 文件却忘登记 patch」。解析登记 patch 列表与各 patch 的 `diff --git` 路径，经 `git clone --filter=blob:none --no-checkout` + `git ls-tree -r` 取 pin 处上游路径（**无需 toolchain/cargo**），断言「vendored 路径 ⊆ 上游路径 ∪ 已登记 patch 路径」。实测：上游 168 + patch 11 = 179 覆盖 vs 128 vendored → 0 未归属，`GUARD_EXIT=0`。

## A4. 功能验收（在**重放后**的隔离树上）

| 项目 | x86_64 | i686 |
|---|---|---|
| vendored WindUI full tests | 2 suites / **910 passed** / 0 failed | 2 suites / **910 passed** / 0 failed |
| 双进程真实 OS UIA smoke | **exit=0** | **exit=0** |
| fcitx5-config-poc 构建 | exit=0 | exit=0 |
| fcitx5-config-poc tests | **62 passed** / 0 failed | **62 passed** / 0 failed |

`git diff --check` PASS。smoke 仍走真实客户端栈（`ElementFromHandle`），证据：`root=Window`、9 后代含 `Accessibility smoke(50020)`/`Invoked 0(50020)`/`Invoke me(50000)`、无 Hidden button、Invoke 后 `Invoked 1`、宿主关闭后 stale 查询返回 `UIA_E_ELEMENTNOTAVAILABLE (0x80040201)`。

## A5. 过程中发现并修正的两个真实缺陷（本轮价值）

1. **patch 路径前缀错**：首版按仓库根相对路径生成（`a/third_party/wind-ui-rust/...`），而同步脚本是对 **clone 根**执行 `git apply` → `exit 128`。改用 `--relative=third_party/wind-ui-rust`，与既有补丁的 `a/src/...` 约定一致。
2. **PowerShell 写 patch 引入 CRLF**：首版用 `Set-Content -Encoding utf8` 写入 → CRLF 且多出空行 → git 报 `corrupt patch at line 19`（exit 128）。改为 git `--output=` 自写文件，实测 `HAS_BOM=False`、`CR_BYTES=0`。

两处都属"补丁资产本身的正确性"，非产品代码问题——但它们正是"同步会静默抹掉工作"之外的第二类坑，值得写进后续流程规范。

## A6. 约束遵守

- root `Cargo.lock` 在两次全量构建+测试后仍 `git status` 为空（**未变**）；未因重放而需要新增依赖。
- 原工作树全程只读：HEAD 仍 `3a8fa74`、porcelain 仍 30 项、其 vendored untracked 新文件仍 4 个；未执行 `checkout/reset/clean/sync`，未写文件（清单/manifest 落在 `%TEMP%`）。
- 未运行 `--Latest`（本轮只证明"当前版本可精确重建"，不升级）。
- **未 merge/cherry-pick 回原并发工作树**。
- 环境侧唯一处置：隔离 worktree 缺 `out/toolchains`（`.cargo/config.toml` 的 `rustc-wrapper` 为仓库根相对路径）导致脚本自带 `cargo check` 无法运行 → 用**只读目录联结** `iter44/out/toolchains` → 主树 `out/toolchains` 解决。非仓库改动（`out/` 被 gitignore），`iter44/out/tmp` 保持本地。

## A7. 残留（明确移交）

1. **把恢复提交整合回主工作树**——待 Codex 侧在制品状态明确后单独进行；本轮刻意不做，以免把他人 WIP 绑进同一历史。
2. `--Latest` 上游兼容性——另开任务。
3. 审计提示的性质备注：`git show 4ead534 --check` 会报"新增 patch 文件内的空上下文行带一个空格"（把 patch 当文件嵌入时固有），重放状态本身干净（隔离树 `git diff --check` 为 0）。纯外观，非缺陷。

---

# B. 下一任务申请 — P3：Settings 真实 HWND 的 UIA 集成

> 按既定顺序：**P3 而非 P4**。以下供 ChatGPT 产出可执行计划。

## B1. 目标（一句话）

让**真实 shipping Settings 窗口**（Rust `rust/config-poc` 产出的 `fcitx5-config.exe`，非 WTL 遗留 shell）通过已证明的 Win32 raw UIA provider 对外暴露语义树，并由**独立进程的 OS UIA 客户端**（沿用 `uia_smoke` 手法）验证静态语义、`Invoke`、以及**多语言** Name/bounds。

## B2. 为什么现在做 / 前置

- provider 已在 Vendored WindUI 层被跨进程证明可用（Iteration 43 → `WIN32 RAW UIA ROOT/TEXT/BUTTON PROVIDER AUTOMATED-GREEN`）。
- 前置 A（必须）：先决定并执行 **Iteration 44 恢复提交的落地方式**（把 vendored delta + patch + 注册 + guard 合并进主线，或让 P3 直接在 `c2c/windui-replayability-iter44` 上继续）。否则 P3 的工作又会变成"不可重放的 vendored 改动"，回到同一个坑。
- 前置 B（机械确认，不重做产品选择）：已核查 shipping Config 由 Rust `config-poc` 产出；P3 开始前只需再次确认 `fcitx5-config.exe` 的窗口类名与创建路径确实经过 WindUI 的 raw provider 分支。

## B3. 范围（in / out）

**In scope**
1. **窗口识别（语言无关，固定方案）**：`启动的 child PID → EnumWindows + GetWindowThreadProcessId 过滤 → 顶层窗口类名 == WindUI 窗口类（`WindUiWindowClass`）`。**不得依赖本地化标题**（否则八语言下 smoke 失效）。
2. 复用/扩展 `uia_smoke` 的双进程机制指向真实 Settings 窗口：枚举语义树、断言关键控件存在与属性正确、`Invoke` 产生**可观测效果**、宿主关闭后 stale 查询返回 `UIA_E_ELEMENTNOTAVAILABLE`。
3. **多语言 Name/bounds 验证**：至少覆盖现有 locales（`ja-JP/ko-KR/th-TH/vi-VN/si-LK/zh-TW` 等）中的抽样集合；断言 Name 来自本地化资源而非硬编码、bounds 为 DPI 正确的屏幕物理像素且不越出真实窗口矩形。
4. 测试隔离：smoke 必须能指向**受控 data-root / 配置夹具**，不污染用户真实配置。
5. 若发现 provider 缺陷（例如某控件未被暴露、属性缺失、bounds 偏移），在 Vendored WindUI 层修复——**并同步更新 Iteration 44 的 catch-up patch**（这是 A2 结论的直接收益：改完可重放验证）。

**Out of scope（明确不做）**
Selection/SelectionItem、Toggle、ComboBox、Value/Text patterns、UIA 事件通知、Narrator/NVDA 手工验收、语言 UI 扩展、Candidate P4 三项、`--Latest` 升级。

## B4. 验收建议（可机器验证）

1. x64 与 x86 **双进程真实 Settings OS UIA smoke** 均 `exit=0`，且断言内容包含：root=Window；目标控件（如候选布局/页大小相关按钮）存在且 Name/ControlType/IsEnabled/IsKeyboardFocusable 正确；`Invoke` 后**产生可观测状态变化**；关闭宿主后 stale → `0x80040201`。
2. 语言无关性：同一 smoke 在至少 2 种非中文 locale 下通过（Name 断言取自对应本地化资源或断言"非空且与默认语言不同"）。
3. 无回归：`cargo test -p fcitx5-config-poc`（双架构）0 failed；vendored WindUI 全量测试 0 failed；`git diff --check` PASS。
4. **可重放约束**（新增，源自 Iteration 44）：若本轮改动了 vendored WindUI，运行 `tools/verify-windui-vendor-coverage.ps1` 必须仍 `exit=0`，且 `sync + 重放` 仍与工作树逐字节一致（否则视为未完成）。
5. 无法自动化的项（真实朗读器行为）显式标 `MANUAL-PENDING`，不得记为绿。

## B5. 需要 ChatGPT 决策的开放问题

1. **`Invoke` 的可观测断言是什么**：改页大小？（哪种控件、读回哪个状态）改布局模式？还是仅"配置写入 + 预览位图变化"？需给出可机器读取的证据源。
2. **前置 A 的落地方式**：现在 merge 恢复提交进 `main`，还是让 P3 在该隔离分支继续？（前者需要与 Codex 侧协调；后者会拉长分支寿命。）
3. **语言覆盖集合**：全部 6+ 个 locale，还是抽样 2–3 个（含一个高 DPI + 一个非拉丁文）？
4. **CI 可行性**：真实 Settings 窗口的 smoke 需要桌面会话；是否接受"仅本地/手动 lane"并在 CI 中跳过（标 `MANUAL-PENDING`），还是要求做成可在 CI 运行（例如 headless/虚拟显示器）？
5. **测试隔离粒度**：用临时 data-root 启动 `fcitx5-config.exe` 的可行性确认（是否支持 `--data-root` 或等价夹具参数）。
6. **P4 preview 归属审计**：是否在 P3 之后插入一个**只读**的 owner audit（找出 `settings_preview.rs` 被删除后 preview render path 的唯一承载者与现有 screenshot tests），再决定 P4 是否重做第三项。
