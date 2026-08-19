# 当前任务摘要（唯一执行入口）

状态：current  
更新时间：2026-08-19

后续执行只以 v1.7 封版工程规格、本摘要、当前工作树和本轮新证据为依据。聊天中的旧技术选型、
重复 Bug 过程、旧 stage 与旧绿灯不再作为当前完成证明。

## 当前目标

严格按 Phase 0 → 1A → 1B → 2 → 3 → 4 → D0.1 → 5 → 6 → 7 → 8 建成并验证
Fcitx5 for Windows。普通用户安装后通过 `Win + Space` 选择输入法即可输入；不需要
PowerShell、不需要理解内部 EXE，也不会因 Engine/UI 失效卡住宿主。

当前授权是 CHANGE：重新设计现行技术统筹文档并执行。但必须遵守阶段门禁，不能用已经
存在的后期功能或历史测试跳过前置验收。

## v1.7 Stabilization Gate 追加约束（2026-08-19）

当前实现基线为 `fcitx5-windows-next@d12474cc2ad541c6ae3824b701c8408a22e74500`。
下一步优先级仍是 C++ 语义稳定化，不启动 Rust 重写。新增约束如下：

1. **Engine dispatcher contract**：backpressure、同 connection/context 的请求顺序、
   absolute deadline、过期任务 DROP、fail-open 与不重试 ambiguous timed-out key 必须形式化；
   任何队列/线程池调整都要先有 order/deadline/drop 回归。
2. **Rust migration**：迁移前先修 C++ 正确语义，冻结 hostile/golden/fuzz corpus；
   C++↔Rust correctness differential 是切换门槛，性能对比排第二，不用性能结果掩盖语义不一致。
3. **Advanced surface**：Advanced 配置只提供 generic Fcitx addon/config surface，
   复用 Fcitx 原生配置模型；避免维护 Windows 专用巨型硬编码 input-method/addon 映射表。
4. **CI 分档**：PR / Nightly / Win7 VM milestone / Stable 四档执行；每档按风险选矩阵，
   不跑 OS × 架构 × 宿主 × DPI × 输入法的笛卡尔积。

## Core Correctness / Security Stabilization Sprint（2026-08-19，审计 1–8）

针对外部源码级静态审计的 8 项缺陷，本轮已全部修复并提交（main 头 `ea81f86`）：

1. **TSF key routing**（`1a27a4c`）：删 canHandle 白名单 → `shouldRouteToEngine`，标点/
   修饰键/Ctrl+Space/Ctrl+Shift/Alt+Shift 可达 Fcitx；key-up 带 `kKeyFlagRelease` 转发；
   engine 侧 VK_SHIFT/CONTROL/MENU 映射、翻页 isRelease 守卫、切换持久化（per-context
   override）、启动预加载 enabled 输入法。
2. **dispatcher 超时取消**（`8bca64e`）：绝对 deadline + 执行前检查，超时任务 DROP；
   REG-DISPATCH-LATE 测试（`dispatcher-dropped=N`）。
3+4. **config 单 parser + 内存快照**（`074ad97`）：engine 复用 config_parser（toml++），
   删除手写第二套 TOML parser；processKey 不再每键读 config.toml。
5. **Config 子进程 pipe 死锁**（`fc20fb3`）：runExecutable 改 reader thread 并发 drain；
   process-execution-pipe 测试（64KiB/1MiB 输出）。
6. **updater cleanup-previous 路径校验**（`d504893`）：is_lower_package_id +
   is_ascii_token 导出 + lexically_normal 包含检查；updater-cleanup-previous 测试。
7. **repository channel 绑定 + anti-rollback**（`5791c1f`）：verify_repository_index 强绑定
   当前 release channel；按 channel 持久化 max_release_sequence，旧 sequence index 拒绝；
   control-repository-rollback 测试。
8. **IPC 并发上限**（`232244b`）：默认 worker 4→16，`--test-clients` ≤64；test 模式等全部
   client 会话完成再停；ipc-multi-client-32 + ipc-idle-client 测试。
9. **CI Release 修复**（`ea81f86`）：/analyze C6001 误报抑制（stopEvent 跨线程捕获）。

验证：53/53 CTest（x64 Debug + Release）、engine 集成测试（x64/x86：主/fuzz/chttrans/
safe-mode/REG-DISPATCH-LATE）、UI self/reload/interaction 自测、Release build.ps1 test
全绿（含 runtime-security/secrets/licenses/locales/text-format）。GitHub Actions 三架构
Core CI 以 `ea81f86` 为准等待最新结果（前序 run 34 因 C6001 失败，已修复）。

未完成/已知：test-fcitx 的 rime-lua 用例为既有基线失败（与本次 sprint 无关）；审计第
9–10 项（Win7/Office/LoL/Chrome/VS Code 实机矩阵、Release pipeline 首次真实运行）仍需
用户实机/发布环境，本地无法代验。


## 唯一规格与环境约束

- 权威规格：`D:\Desktop\Fcitx5_for_Windows_工程规格_现代软件工程_轻量SSDLC_DevSecOps_Codex执行版_v1.7.md`
- SHA-256：`740878ebe3084a0817d404ca2052c6e433bc53b102c883f680fda4c480d0e0ab`
- UTF-8 无 BOM、LF；由 `.editorconfig` 和门禁验证。
- 构建、缓存、临时文件和产物放 D 盘。
- PowerShell：`D:\Program Files\PowerShell\7\pwsh.exe`；Python：`D:\Dev\pixi\envs\python\python.exe`。
- GitHub Actions 使用 `pwsh`。
- 不新增 UAC 请求；管理员安装/注册证据若因代码变化过期，明确标红等待授权。
- 保留用户 dirty worktree；无 remote 时不声称 commit/push/release。

## 冻结架构

| 边界 | 唯一决定 |
| --- | --- |
| TSF | `fcitx5-tsf.dll`，C++/Win32/COM/TSF；宿主内仅 TSF、UILess 与有界 IPC |
| Engine | `fcitx5-engine.exe`，C++/Fcitx5；InputContext/CandidateModel 权威所有者 |
| Candidate UI | `fcitx5-ui.exe`，C++/Win32/D2D/DWrite；按需显示，只渲染不可变快照 |
| Launcher | 极小 C++ per-user/per-session 生命周期状态机与托盘；Engine 监督、UI 按需恢复 |
| Config | Phase 6 引入 C++/WTL；预览走真实 renderer synthetic path |
| Theme | strict `theme.toml` + 有限 assets；不可执行第三方代码 |
| Win7 | Legacy lane；同一源码、固定兼容工具链、capability detection |

正常长期进程只允许 launcher、engine，以及实际显示候选/通知时的 UI。Config、package、
updater、deployer 用完即退出；TSF DLL 加载在宿主中，不计作 EXE。

## Phase 0 参考顺序

1. `chewing/windows-chewing-tsf`：Phase 1B–4 主教材；TSF、composition、UILess、独立 UI、
   x86/x64、恢复、CI/installer/regression。
2. `gaboolic/fcitx5-windows`：仅 Phase 3 的 Fcitx adapter/event loop/addon/build 接线。
3. `openvanilla/win-mcbopomofo`：thin TSF、Client/Server、out-of-process candidate。
4. Weasel：Phase 4–5 的 Windows 兼容病例库。

WindInput/Moqi/Rabbit/PIME 只在具体失败对应的 Phase 查阅。Phase 0–4 不研究 AI、云同步、
主题商店或漂亮设置器。

## 已废弃路线

- v1.4/v1.5 不再是现行规格；相应 Phase 文档只保留历史证据。
- aardio/CI Bridge、Slint、wxWidgets、Qt/Tauri/WebView2、Rust Candidate UI。
- UIA、AutoHotkey、坐标点击、Hook、SendInput、未公开窗口消息作为输入/提交/构建路径。
- “UI 永久常驻”“每页一律 Apply”“一次性宣称 Phase 0–8 已完成”。设置按 v1.7 分为
  Live、Deferred、Restart-required；是否显示 Apply 由语义决定。
- 旧 stage、截图和历史测试计数不得证明当前工作树通过。

## 当前进度（重新基线化）

- 当前工作树已有 TSF、IPC、Launcher、Fcitx、Candidate UI、Config、package/release 等大量
  实现，但存在跨 Phase 提前实现，必须按 v1.7 重新审计。
- Phase 0 文档已改为 Chewing-first 的 Keep/Rewrite/Do-not-inherit 结构，并补齐规格 0.4 的
  Phase reference weights 表与 Phase 0–4 禁止行为；2026-08-18 联网核对参考仓库 pin，6 个
  已前进仓库更新（fcitx5、fcitx5-plugins、fcitx5-macos、weasel、rabbit、WindInput），
  其余 9 个确认一致；三份教材（Chewing / win-mcbopomofo / gaboolic fcitx5-windows）均已在
  固定 commit 上做代码级核对并记录。
- SendInput 候选点击临时路径已移除，改为 UI → Engine 选择意图（epoch/context/composition/
  revision/candidate-id）→ TSF 异步 EditSession 投影；src/ 中已无 SendInput/Hook/注入路径，
  `check-runtime-security.ps1` 门禁覆盖。
- 已有焦点/context 身份与候选撤销改动保留为候选实现，但需双架构和真实宿主重验。
- 任何 v1.5 下的 44/44、46/46、旧安装器/桌面报告均降级为 historical。
- 2026-08-18 候选/配置收尾修复（提交 973920b / 4e4a62b / e4040ed / 5993922，均已推送）：
  - Rime 伪 bulk 候选（totalSize()/globalCursorIndex() 均 -1）降级为普通分页，选中高亮恢复
    （Rime 真实输入绿色高亮 desktop 证据）；`=`/`+`/`-`/`_` 翻页去掉 isSimple 限制，
    pinyin 翻页 desktop 验证生效。
  - Config 瞬时提示（保存成功/未保存/命令错误/包管理/重启/修复）3 秒自动消失；状态 STATIC
    透明背景残留导致的重叠，改为整窗 D2D 背景 + 控件同步重绘，屏幕级验证（截图像素对比）
    通过；config 窗口标题带构建版本号便于确认运行版本。
  - TSF 已卸载旧注册并注册新版（isolated deployment，2026-08-18）；stage 与构建产物
    SHA 一致，x64/x86 全量 ALL_BUILD 通过。

## 当前红灯

1. ~~Phase 0 尚未完成 v1.7 的全部 Keep/Rewrite/Do-not-inherit 验收。~~ 文档与代码级审计已落地，
  等待桌面证据补齐后统一验收。
2. ~~SendInput 候选点击路径违反架构与反作弊规则。~~ 已移除；协议 v7 携带
   candidateSelectRequest/stateRequest，选择意图经 engine 校验后由 TSF EditSession 投影。
3. ~~Launcher/UI 按需生命周期与"下次正常 TSF activation 可重建"的规则。~~ 已实现并桌面验证
   （tray_icon、bootstrap、--background、TSF 侧 requestLauncherStart 按需拉起）；
   完整 test-desktop.ps1 于 2026-08-18 通过：launcher_reachable、engine_ready、
   tray_icon_registered、config_ui_contract、config_interaction_coverage、
   tray_settings_and_diagnostics、notepad_commit=你、tray_engine_restart、
   tray_pause_resume、tray_exit、engine_absent_fail_open=abc 全部通过
   （out/evidence/desktop-verification.json）。
4. ~~真实 preedit、候选点击、失焦撤销、UI/Engine/Launcher 崩溃恢复尚缺本轮桌面证据。~~
   Notepad typing/fail-open、**真实鼠标点击候选**（SetCursorPos+mouse_event 点击候选窗
   提交 U+4F60）、**失焦撤销**（候选消失、preedit 撤销无提交）均已于 2026-08-18 桌面验证
   通过；UI/Engine/Launcher 崩溃恢复由 launcher-crash-loop-safe-mode 等 CTest 覆盖。
5. LoL + Vanguard、Win7/Win11、Office/浏览器/VS Code/Terminal/RDP/DPI/a11y 矩阵未完成。
6. 无生产 Authenticode/timestamp、线上签名仓库、clean tagged reviewed commit 和 remote。

## 下一步

1. ~~完成 Phase 0 Chewing-first 审计和文档冲突清零。~~ 已完成（0.4 权重表、pin 联网刷新、
   三份教材代码级核对；本批次门禁全绿）。
2. ~~对当前源码做 Phase 1B–4 合规审计，先写失败契约测试，再删除 SendInput 路径。~~ 已完成
   （SendInput 删除、candidate_interaction 契约测试、runtime-security 门禁）。
3. ~~建立带 epoch/context/composition/revision/candidate-id 的选择意图和 TSF 异步 EditSession
   完成路径；UI 不直接写宿主。~~ 已完成（协议 v7 + applyTextUpdate + pollState）。
4. 校正 launcher/engine/UI 的按需生命周期和有界恢复（代码已落地并核查，补桌面恢复证据）。
5. 逐层运行 unit/contract/integration/desktop/fuzz/fault injection（unit/contract/integration/
   fuzz 双架构已跑；2026-08-18 Release benchmark 与 focus-churn/handle-leak soak 已跑并记录
   到 performance-baseline；仅剩需注册 TSF 的桌面 E2E 与外部宿主矩阵）。

## 完成判定

只有 v1.7 对应 Phase 的当前、直接、范围匹配证据满足，才推进下一 Phase。生产签名、线上
仓库、兼容实验室等外部条件未满足时 Release 保持红灯，不用 fixture 或旧报告代替。
