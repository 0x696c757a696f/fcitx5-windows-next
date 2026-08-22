# Windows Chewing TSF 全面工程复盘

状态：v1.6 Phase 0 主教材审阅；行为/工程模式参考，不作为源码依赖
审阅日期：2026-08-18
固定版本：[`342ead0c0b445ec376fbd6ffb3b105e78c499419`](https://github.com/chewing/windows-chewing-tsf/tree/342ead0c0b445ec376fbd6ffb3b105e78c499419)
许可证：GPL-3.0-or-later；本项目只重建通用行为和测试，不复制其实现代码。

## RUST-R3-TSF-POC refresh

审阅日期：2026-08-22
固定版本：[`a6cf2c55ca1009d6aeac894760a59ba459ec676d`](https://codeberg.org/chewing/windows-chewing-tsf/src/commit/a6cf2c55ca1009d6aeac894760a59ba459ec676d)
读取范围：`tip/src/com.rs`、`tip/src/text_service/mod.rs`、`tip/src/text_service/edit_session.rs`、`tip/src/text_service/key_event.rs`、`tip/src/text_service/ui_elements/candidate_list.rs`、`crates/chewing_tip_core/src/ipc/messages.rs`、`crates/chewing_tip_core/src/ipc/values.rs`、`Cargo.toml`。
许可证处理：GPL-3.0-or-later；本项目只记录行为、测试场景和边界设计，不复制非平凡源码。

R3 Rust TSF PoC 中应采用的行为/测试场景：

- Rust `windows-rs` TSF DLL 可以直接实现 `ITfTextInputProcessorEx`、key/thread/focus/composition sinks、display attribute provider 和 `ITfCandidateListUIElement`，不需要 C++ FFI 作为长期依赖。
- `Activate` 内部初始化失败时应记录错误并让宿主 fail-open；不能让 TSF manager 因一次失败后无法重新初始化。当前 PoC 用 panic-to-HRESULT 和 key callback fail-open smoke 覆盖第一层，后续差分需要覆盖真实 sink advise/unadvise 路径。
- `Deactivate` 必须结束 candidate UIElement、unadvise thread/key/compartment/function sinks，并释放 thread manager 相关状态；这应成为 Rust PoC 的 COM refcount/sink cleanup 差分项。
- `OnTestKeyDown`、`OnKeyDown`、`OnTestKeyUp`、`OnKeyUp` 分离成 typed IPC messages 是正确方向，但本项目必须保留现有 versioned binary IPC、epoch/context/composition/revision、peer identity 和 deadline，不采用 Chewing 的 JSON/varlink wire format。
- UILess `BeginUIElement` 返回 `show=false` 时仍保留 candidate metadata 和 keyboard selection semantics；这应进入 Candidate/TSF 共享 corpus，不能只测试 popup 可见性。
- composition commit + preedit update 尽量在同一个 edit session 内完成，减少 Cicero/host 状态撕裂；这应进入 Rust TSF composition differential。
- key callback 期间的 document/focus change 需要重入保护；不能按进程名写宽 quirk，必须有 host-matrix regression。

R3 Rust TSF PoC 中明确不采用：

- 不复制 GPL 源码，不把 Chewing-specific engine/config/update model 移入本项目。
- 不采用 release `panic = "abort"` 作为宿主进程安全策略；本项目 TSF DLL 的 COM 回调仍必须捕获 panic 并返回 HRESULT/fail-open。
- 不引入 IMM32 patch/quirk 作为默认路径；只有真实 host evidence 和单独 gate 才能加入窄兼容修复。
- 不用 Chewing 的 JSON/varlink IPC 替换本项目已冻结的 TSF ↔ Engine protocol。

`EasyIME/libIME2` 对照版本 [`717b1901a417667405399cfbf25b25664efcf0e4`](https://github.com/EasyIME/libIME2/tree/717b1901a417667405399cfbf25b25664efcf0e4) 仅作为 secondary TSF framework reference。可借鉴 sink RAII、edit-session/composition helper、compartment/preserved-key 病例；不可作为本项目 Rust TSF 或 out-of-process UI 架构底座，因为它的候选窗/engine 抽象更偏宿主进程内框架。

## 结论

Windows Chewing 最重要的经验不是某个控件，而是把 TSF 生命周期、候选
`ITfUIElement` 和独立 UI host 组成一条有始有终的状态链。当前项目保留自身更强的
有界二进制 IPC、Fcitx 单一状态所有权、C++ D2D/DWrite renderer 和 launcher
监督模型，同时采用其经过生产验证的焦点、组合、安装迁移和 host 恢复经验。

鼠标候选选择不能直接照搬：审阅版本的候选 host 只处理绘制和窗口位置消息，没有
鼠标点击提交路径。因此本项目必须自己实现命中、选择意图、过期状态拒绝和真实桌面
点击验证。

## 架构对照

| 领域 | Windows Chewing 做法 | 本项目决策 | 状态 |
|---|---|---|---|
| TSF DLL | `ITfTextInputProcessorEx` 加线程、按键、composition、compartment 等 sinks | DLL 继续极薄；完整遵守 COM/TSF 生命周期 | 采用模式 |
| 焦点 | 订阅 `ITfThreadMgrEventSink`、`ITfThreadFocusSink`；失焦结束 composition 并隐藏候选；按键期间抑制 Excel 的伪焦点切换 | 同样成对订阅；增加焦点丢失回归测试；不做无测试的应用白名单 | 候选实现，待本轮重验 |
| 候选语义 | TSF 侧拥有 `ITfCandidateListUIElement`，调用 Begin/Update/End；独立 host 接收 Show/Hide | TSF 保留 UILess 元数据；engine 是候选状态唯一所有者；UI 只收 revision snapshot | 采用并收紧 |
| 候选交互 | 键盘方向和 Enter 映射为明确动作；审阅版本外部候选窗无鼠标处理 | 鼠标必须发送带 context/composition/revision/candidate-id 的语义选择动作，不直接提交字符串 | 必须完善 |
| UI 进程 | `chewing_tip_host.exe` 独立绘制，Windows subsystem，无常驻控制台 | `fcitx5-ui.exe` 独立 C++ D2D/DWrite、无黑框，仅有展示需求时运行 | 冻结目标，待按需生命周期验收 |
| host 恢复 | ping/connect 失败时启动 host；host 注册 Windows Application Restart | launcher 监督；TSF activation 可重建 launcher；Application Restart 只是补充；bounded fail-open | 待 Phase 2/4 验收 |
| IPC 命名 | 用户 SID 哈希命名 | SID 加 logon session 命名 | 冻结目标 |
| IPC ACL | 用户、SYSTEM、Administrators、AppContainer ACL；低完整性标签 | 显式 DACL、同用户/会话、远程拒绝、双向 peer PID/image 验证 | 冻结目标 |
| IPC 鉴别 | client 用 WinVerifyTrust 验证 server；Release 失败关闭，Debug 放行 | 不因 Debug 放行；验证预期映像/安装根和身份；所有帧有大小、关联、epoch/revision | 冻结目标 |
| IPC 编码 | NUL 结尾 JSON/varlink 风格 | 显式小端二进制、最大帧、请求关联、读写 deadline、晚响应拒绝 | 不采用其编码 |
| 配置 | 注册表配置，焦点时检查并重载 | 原子 TOML、schema/defaults、Control API；按 Live/Deferred/Restart-required 生效 | 保留 v1.6 方案 |
| 兼容性 | 对具体宿主设置窄 `Quirk`；Excel 首次按键 document 切换有重入防护 | capability 优先；只有带可复现测试和问题编号的窄 quirk 才可加入 | 采用规则 |
| 注册 | x86/x64 DLL、profile/category、安装时 enable；卸载避免误删语言最后一个 layout | x86/x64 一致注册、obsolete profile 清理、事务回滚；补“最后布局不受损”测试 | 部分采用 |
| 安装升级 | WiX major upgrade；升级前显式停止 host；兼容旧 NSIS 卸载 | Phase 6 验证安装/repair/uninstall/portable；升级前停止进程并验证文件可替换 | 历史实现，待 Phase 6 重验 |
| 更新 | 仅检查和通知，下载/安装交给发布页及 UAC | 有签名仓库的 downloader/deployer/updater，staging、原子切换、回滚 | 不照搬功能范围 |
| 发布 | unsigned artifact 送 SignPath，再发布签名 MSI 和 SHA-256 | build once、test exact stage、protected signing、SBOM/provenance、exact-artifact promotion | Phase 8 冻结目标，当前红灯 |

## Keep / Rewrite / Do-not-inherit

### Keep（保留行为与测试模式）

- 事务式 Activate/Deactivate 与 sink 成对管理；
- TSF composition 和 UIElement Begin/Update/End 的完整生命周期；
- thread/document/context focus 的明确 teardown，以及按键回调中的重入防护；
- 非激活 D2D/DWrite renderer host、DPI/device-loss 处理；
- x86/x64 注册、升级前停止 host、Application Restart 作为补充恢复；
- host-specific quirk 必须有真实复现、窄范围和回归测试。

### Rewrite（按 v1.6 边界重写）

- JSON/varlink 风格 IPC 改为当前 versioned binary contract、deadline、request correlation、
  epoch/context/composition/revision 与双向 peer verification；
- renderer host 改为只消费 Engine CandidateModel 的不可变 snapshot，并仅按需运行；
- 点击选择改为 UI → Engine 语义 intent → TSF EditSession，不借助键盘注入；
- 配置改为 strict TOML typed model 与 Live/Deferred/Restart-required，不复制注册表模型；
- Launcher 恢复同时依赖正常 TSF activation 的可重建路径，不只依赖系统重启注册。

### Do-not-inherit（明确禁止）

- GPL 非平凡源码复制；
- NUL 分隔/无明确当前 schema 的协议、无界同步读写或 Debug trust bypass；
- SID-only pipe namespace、宽 AppContainer 权限、仅验证“任意系统信任签名”；
- 输入按键热路径中的进程冷启动等待；
- WebView2 Config、默认允许降级、未经项目门禁验证的 release workflow；
- 永久常驻 UI、Hook、SendInput、宿主注入或把浮动 preedit 当权威状态。

## 已吸收的具体经验

1. `ActivateEx` 对所有 sink 注册必须是事务性的；任一步失败就撤销前面步骤。
2. `Deactivate` 必须先结束候选 UIElement、断开会话，再成对 Unadvise，最后释放
   `ITfThreadMgr`。不能让 COM 引用环或 host 中残留候选。
3. document focus 变为空、context 被 pop、composition 被外部终止、thread focus
   丢失都属于候选关闭条件；不能只等下一次按键刷新。
4. 键盘回调期间发生的 document 切换可能是宿主内部行为。焦点清理需要重入保护，
   Excel 等兼容性必须由实机用例决定，不能按进程名猜。
5. TSF UIElement 的 Begin/Update/End 与可见候选窗口 Show/Hide 必须对应；无界面模式
   仍需要正确的候选元数据。
6. 独立 UI host 必须是 Windows GUI subsystem，并且能被 launcher/系统恢复；用户不应
   手动开 PowerShell 才能输入。
7. 安装器升级前应先让 host 正常退出，避免 DLL/EXE 被占用；反安装不能破坏其他语言
   和输入法的布局状态。
8. 兼容性例外必须非常窄，并附带宿主、原因、回归测试和移除条件。
9. code signing 应在构建测试之后对同一 artifact 完成，不能签名后重新打包不同内容。

## 明确不继承的风险

- 审阅版本候选窗口没有鼠标点击确认，所以不能作为本项目点击完成的证据。
- NUL 分隔 JSON receiver 未体现本项目要求的严格最大帧和逐操作 deadline；输入热路径
  不使用无界读取。
- pipe 名只含用户 SID，未区分同一用户的多个 logon session；本项目继续包含 session。
- server 端主要依赖 ACL，client 验证的是系统信任签名而非固定发布者/预期路径；本项目
  双向验证具体 peer。
- AppContainer 获得广泛 pipe 权限不直接继承；能力必须按调用者和真实场景最小化。
- Debug 模式不能绕过 peer trust，否则测试环境与发布安全语义不一致。
- TSF 按键路径临时启动 host 可能叠加连接和进程启动延迟；本项目由 launcher 预热，
  超时后立即 fail-open。
- `AllowDowngrades=yes` 不适合作为默认企业更新策略；降级只能走显式 rollback policy。
- 当前 `release.yml` 的 tag trigger 与 job 条件、tag 拼接存在明显不一致，不能照搬；
  本项目发布门禁必须真实跑过 tag 场景。
- 审阅 CI 未显式展示足够的行为测试、fuzz、SCA、SBOM、provenance 和桌面矩阵；它是
  实现参考，不是本项目质量门禁的上限。
- 配置器使用 WebView2 的取舍与本项目 Win7、包体和攻击面约束不一致。

## 对当前故障的落地结果

| 故障 | 根因 | 采用的成熟模式 | 自动化证据 |
|---|---|---|---|
| 候选无法点击 | 窗口曾返回 `HTTRANSPARENT` 且无 mouse handlers；当前临时提交用了禁用的 SendInput | 非激活 hit-test 可保留；提交必须改为语义 intent → Engine → TSF | 现有 focused tests 仅为 historical，合规测试待建 |
| 切走后候选不消失 | TSF 曾未订阅线程/文档焦点生命周期，UI 只能等新 snapshot | focus/composition/context 明确 End/Hide | 候选实现，需双架构和桌面重验 |
| 打多后卡住 | 点击无完成动作、presentation 缺少确定撤销，状态可长期悬挂 | 有界点击 guard、失焦撤销、IPC 断开重连、stale revision 拒绝 | 还需真实桌面长输入/点击压力验证 |

## 后续必须完成的验证

1. 删除临时键盘注入并实现 UI 到 engine 的语义 `SelectCandidate` 动作，校验
   engine epoch、context、composition、revision 和 candidate id。
2. 在真实 Notepad 中点击第一项、非第一项和卷轴模式远端候选，核对最终提交文字。
3. 在候选显示时切换窗口、关闭文档、终止 engine/UI、重启 launcher，逐项验证候选消失
   且宿主不挂起。
4. 长输入、100 次候选点击、焦点 churn、迟到响应和无 engine 场景均设置硬超时。
5. 在 Excel/Word/浏览器/VS Code/Terminal/RDP 兼容矩阵验证焦点重入保护，再决定是否
   需要应用专用 quirk。
6. 对安装、repair、升级、卸载补充“先停 host”“文件无锁”“不破坏其他输入布局”证据。

## 代码级核对（2026-08-18，pin `342ead0c…`）

按规格 0.4 的"看相关代码实现"要求，在固定 commit 上直接阅读以下实现文件，核对结论：

- `crates/chewing_tip_core/src/ipc/named_pipe.rs`：pipe 名 = `\.\pipe\chewing.` + **user SID 的 FNV hash**（`FnvHasher` + `token_user_sid`）；SDDL 为 `O:<SID> D:(A;;;;;OW)(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;AC)(A;;GA;;;<user SID>) S:(ML;;NX;;;LW)` —— 确认 owner 默认权限移除、SYSTEM/BA/AppContainer/用户 GA、**低完整性 no-execute-up 标签**；`connect_and_attest` 用 `WinVerifyTrust`（`WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT`）验证 server 签名，**Debug 构建放行**（`cfg!(debug_assertions)`）。
- `crates/chewing_tip_core/src/sandbox.rs`：`OpenThreadToken` 优先、失败回退 `OpenProcessToken`，取 `TokenUser` SID 字符串；不缓存，每次获取。
- `crates/chewing_tip_host/src/text_service/chewing.rs`：`on_test_keydown` 是**纯查询**（只读 editor 状态、不推进输入）；Alt/Ctrl 修饰键 bypass（视为应用快捷键）；readonly/inactive context 忽略；Ctrl+Shift 数字仅用户词库/符号输入时接管；非组合态 Space 不接管（留给应用）。
- `crates/chewing_tip_host/src/text_service/keyevent.rs`：VK + scancode + 256 字节 key_state → X11 keycode/keysym 映射（Set 1 scancode 表 + Dvorak/Colemak/QGMLWY 反向映射）；Shift/Ctrl/Alt/CapsLock/NumLock/Super 状态从 key_state 位提取。
- `crates/chewing_tip_host/src/ui_elements/candidate_list.rs`：D2D/DWrite/DirectComposition swap chain、`WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOREDIRECTIONBITMAP | WS_POPUP` + `CS_IME`；`WM_WINDOWPOSCHANGING` 里按 `get_dpi_for_point` 计算 client rect 并 `clamp_point_to_monitor`（DPI/多屏 clamp 在窗口类中完成）。
- `crates/chewing_tip_host/src/main.rs`：启动即 `RegisterApplicationRestart`；NamedPipe listener 创建失败时通过 pipe path 存在性判断"已有实例"；IPC listener 独立线程，主 UI loop 独立。

对本项目的影响：`windows-keyboard`/TSF 按键状态提取与 keyevent.rs 思路一致；IPC 采用本项目更强的 versioned binary + request correlation + session namespace + 双向 peer 验证，不采用 SID-hash-only 命名与 Debug 放行；候选 DPI clamp 复用其 monitor clamp 行为模式。
