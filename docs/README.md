# 工程文档入口

状态：current
更新：2026-08-25

这里是 Fcitx5 for Windows Next 的工程导航。当前执行入口是 v1.8 queue：
`docs/tasks/PLAN.md`、`docs/tasks/current.md`、`docs/tasks/status.md` 与
`docs/current.md`。

旧聊天记录、旧 stage、旧测试绿灯、v1.5/v1.6/v1.7 的局部结论都不能单独证明当前实现完成。

## 必读顺序

1. [current.md](current.md)
   当前真实架构、语言边界、红灯和下一步。

2. [spec-v1.8.md](spec-v1.8.md)
   当前长期规格和工程/产品评审基线。若它与任务队列冲突，以任务队列的当前解释为准。

3. [tasks/PLAN.md](tasks/PLAN.md)、[tasks/current.md](tasks/current.md)、
   [tasks/status.md](tasks/status.md)
   自动执行队列、当前授权任务、执行证据和 manual-pending/blocker 记录。

4. [technical-program-plan.md](technical-program-plan.md)
   历史路线和运行拓扑参考；不能覆盖当前 Rust-first queue。

5. [reference-baseline.md](reference-baseline.md)
   Phase 0 参考仓库、pin、许可证边界和“能借鉴什么 / 不能继承什么”。

6. [reference-review-windows-chewing-tsf.md](reference-review-windows-chewing-tsf.md)
   Windows TSF 主教材审计。

7. [reference-review-win-mcbopomofo.md](reference-review-win-mcbopomofo.md)
   thin client/server 输入法结构参考。

8. [macos-config-reference.md](macos-config-reference.md)
   fcitx5-macos、fcitx-contrib 和 Rime 相关配置能力的产品化参考。

9. [theme-ui-ux-product-plan.md](theme-ui-ux-product-plan.md)
   主题、插件管理器、候选窗、高分屏、可视编辑器和导入器的统筹方案。

10. [product-test-plan.md](product-test-plan.md) 与
   [ssdlc-verification-matrix.md](ssdlc-verification-matrix.md)
   测试、SSDLC、发布和证据门禁。

11. [adr](adr)
   已接受的架构决策。修改对应边界前先读相关 ADR。

## 外部权威规格

外部桌面规格/评审文件只能作为设计输入。仓库内当前执行权威是
`docs/tasks/PLAN.md`、`docs/tasks/current.md`、`docs/tasks/status.md`、`docs/current.md`
和 `docs/spec-v1.8.md`。

## 文档状态约定

- `current`：现行执行依据。
- `evidence`：带提交、产物、环境或截图标识的验证记录；相关代码变化后可能过期。
- `reference`：设计输入和问题定向参考，不授权复制源码或跳过门禁。
- `historical`：保留审计价值，但不能证明当前实现已经完成。
- `obsolete`：已废弃，禁止继续实施。

## 当前冻结边界

- TSF：shipping Rust；宿主内只做 TSF、UILess、EditSession 和有界 IPC；release-ready 仍需真实 host matrix。
- Engine：直接 Fcitx5 core/addon object adapter 保留 C++；产品 protocol/state/IPC/policy 继续 Rust 化。
- Candidate UI：domain/model/layout/interaction Rust-owned；当前 Win32/D2D/DWrite renderer/window 是临时 adapter。
- Launcher/Control：Rust core owns product state/policy；剩余 C++ shell 继续收缩。
- Config：当前 C++/WTL/ATL shell 只是临时 shipping adapter；`CONFIG-RUST-CUTOVER-001` 将执行 Rust shipping cutover。
- Theme：strict `theme.toml` + 有限 assets；第三方主题不可执行代码。
- Package/Updater/Downloader/Provider/Deployer/Register/Bootstrap：Rust-owned 或 Rust CLI cutover；不复活旧 C++ authority。

## 已废弃路线

以下内容不得重新作为实现路线：

- v1.4/v1.5 作为现行规格；
- aardio/CI Bridge、无证据的重 GUI runtime、WebView2/Tauri/Qt 作为默认设置器路线；
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
