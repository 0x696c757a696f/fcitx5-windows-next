# 工程文档入口

状态：current  
更新：2026-08-20

这里是 Fcitx5 for Windows Next 的工程导航。当前只有一条执行路线：冻结的 v1.7 规格。
旧聊天记录、旧 stage、旧测试绿灯、v1.5/v1.6 的局部结论都不能单独证明当前实现完成。

## 必读顺序

1. [current-task-summary.md](current-task-summary.md)
   当前唯一执行入口：目标、环境约束、红灯、下一步、哪些证据已经过期。

2. [technical-program-plan.md](technical-program-plan.md)
   v1.7 的 Phase 0 → 8 路线、运行拓扑、dispatcher 语义、Rust 迁移门槛、
   Advanced/config surface 和 generation draining。

3. [reference-baseline.md](reference-baseline.md)
   Phase 0 参考仓库、pin、许可证边界和“能借鉴什么 / 不能继承什么”。

4. [reference-review-windows-chewing-tsf.md](reference-review-windows-chewing-tsf.md)
   Windows TSF 主教材审计。

5. [reference-review-win-mcbopomofo.md](reference-review-win-mcbopomofo.md)
   thin client/server 输入法结构参考。

6. [macos-config-reference.md](macos-config-reference.md)
   fcitx5-macos、fcitx-contrib 和 Rime 相关配置能力的产品化参考。

7. [theme-ui-ux-product-plan.md](theme-ui-ux-product-plan.md)
   主题、插件管理器、候选窗、高分屏、可视编辑器和导入器的统筹方案。

8. [product-test-plan.md](product-test-plan.md) 与
   [ssdlc-verification-matrix.md](ssdlc-verification-matrix.md)
   测试、SSDLC、发布和证据门禁。

9. [adr](adr)
   已接受的架构决策。修改对应边界前先读相关 ADR。

10. `phase-*-acceptance.md`
    阶段验收记录。它们是证据，不是反向修改路线的授权。

## 外部权威规格

唯一现行外部规格：

`D:\Desktop\Fcitx5_for_Windows_工程规格_现代软件工程_轻量SSDLC_DevSecOps_Codex执行版_v1.7.md`

SHA-256：

`740878ebe3084a0817d404ca2052c6e433bc53b102c883f680fda4c480d0e0ab`

如果仓库文档和这份规格冲突，先以
[current-task-summary.md](current-task-summary.md) 中已经 reconciliation 的内容为准；未
reconciliation 的冲突要标红，不要靠猜测继续执行。

## 文档状态约定

- `current`：现行执行依据。
- `evidence`：带提交、产物、环境或截图标识的验证记录；相关代码变化后可能过期。
- `reference`：设计输入和问题定向参考，不授权复制源码或跳过门禁。
- `historical`：保留审计价值，但不能证明当前实现已经完成。
- `obsolete`：已废弃，禁止继续实施。

## 当前冻结边界

- TSF：C++/Win32/COM/TSF；宿主内只做 TSF、UILess、EditSession 和有界 IPC。
- Engine：C++/Fcitx5；InputContext、CandidateModel、输入语义和配置权威所有者。
- Candidate UI：C++/Win32/D2D/DWrite；按需渲染不可变快照。
- Launcher：per-user/per-session；负责托盘、Engine/UI 生命周期和恢复。
- Config：C++/WTL/ATL；只通过 typed Control/config API 变更状态。
- Theme：strict `theme.toml` + 有限 assets；第三方主题不可执行代码。
- Package/Updater：签名仓库、事务、回滚、generation-aware draining。

## 已废弃路线

以下内容不得重新作为实现路线：

- v1.4/v1.5 作为现行规格；
- aardio/CI Bridge、Slint、wxWidgets、Qt、Tauri、WebView2、Rust Candidate UI；
- UI Automation、AutoHotkey、坐标点击、Hook、SendInput、注入或未公开窗口消息作为输入/
  提交/构建路径；
- TSF DLL 加载 Fcitx addon、Rime、Lua、下载器、解压器或 GUI；
- 把 Pinyin、Rime、Mozc 或后续插件注册为多个 Windows TSF profile；
- 用 Restart Manager 默认关闭所有加载输入法 DLL 的宿主来升级；
- 用历史测试计数、旧安装截图或旧 stage 证明当前 Phase 完成。

## 写新文档的规则

- 先标状态和日期。
- 写清楚适用范围，不要让局部实验看起来像全局决策。
- 任何完成声明都要带当前提交、构建配置、架构和测试/桌面证据。
- 参考文档要写“借鉴点”和“禁止继承项”。
- 如果改动触及 TSF、Engine、Candidate UI、Launcher、package 或 updater 边界，优先写 ADR。
