**Fcitx5 for Windows Next**

**工程规格、架构约束与轻量 SSDLC / DevSecOps**

Codex 执行版 · 个人项目模式

| **定位　这是通用 Fcitx5 Windows 前端/发行平台的执行规格，不是任何单一输入方案的专用设计。晨星键道、Rime、拼音、Mozc、Hangul、码表等都只是可选 addon 或 inputmethod-data。** |
|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|

| **个人项目原则　保留真正降低风险的 SSDLC 活动，去掉组织流程负担。一个人可以完成需求、实现、审查与发布，但关键检查必须自动化、可重复、有记录。** |
|-------------------------------------------------------------------------------------------------------------------------------------------------|

版本：Frozen 1.8 · 2026-08-20 · UI/UX, Branding & Single-TSF-Profile Baseline · Markdown / Codex 执行版

# 0. 执行摘要

**文档版本：v1.8**

**本版重点：**v1.8 继续以 `d12474cc2ad541c6ae3824b701c8408a22e74500`（2026-08-19）作为**可重放的完整源码审计基线**，同时在 2026-08-20 重新核对公开 `main` 的 Config/Candidate 当前代码以吸收其已出现的 UI 实现变化；**不得把 `d12474c...` 误写成永远的“最新 HEAD”**。真正开始实现前必须先记录本地 `git rev-parse HEAD`，若 HEAD 已前进，则只对受影响 subsystem 做增量审计并记录新 SHA。在 v1.7 风险驱动 Rust 迁移原则不变的前提下，正式补齐 **Config UI/UX、Candidate UI/UX、Design System、企鹅品牌/任务栏/TSF 图标、单一 TSF profile 与默认显示名、TSF in-use DLL 更新 / Generation Draining** 的产品规格。Windows 侧只注册并维护 **一个 Fcitx5 TSF profile**；Rime、拼音、Mozc、Hangul、m17n 等输入法/引擎只在 Fcitx5 内部切换，不为每个 engine 创建 Windows TSF profile。TSF DLL、Fcitx Engine、现有 D2D/DWrite Candidate UI、WTL 宿主与最薄的 Windows register/bootstrap 层不为“界面现代化”或 Rust 重写。

**规格冻结规则：**从 v1.8 起再次冻结。只有真实实现、测试失败、平台/工具链行为、安全事件或可验证的产品需求才能解冻；不得因为“Rust/GUI 框架更潮”扩大迁移面。UI 改进必须优先复用现有 WTL/Win32/D2D/DWrite 技术栈，不以换框架作为美观方案。

**目标：**构建一个 Windows 原生 TSF 前端，使 Fcitx5 核心与其 addon 生态在 Windows 上可长期维护地运行，同时保证输入隐私、宿主进程稳定、单一 Windows TSF profile 下的多语言 engine 语义正确、插件隔离、原子且可验证的更新，以及现代、克制、低认知负担的设置与候选体验；Windows 10/11 为持续演进主线，Win7 SP1 作为有明确边界的 Legacy compatibility lane。

## 0.1 最高工程原则：优先级高于本文其他条款

以下原则优先于后文任何历史描述。若后文与本节冲突，以本节为准，并立即删除冲突实现与文档，不保留旧路径。

1. **运行时不保留向后兼容。** 旧协议、旧内部 API、旧配置 schema、旧包格式、废弃 feature flag 和旧实现一旦不再属于当前需求就直接删除。Core runtime 禁止新增 compatibility shim、双协议解析、隐式 migration、legacy adapter、旧代码 fallback；版本不匹配时明确失败并要求组件同版。若公开 Stable 后确有必要转换用户人工维护且不可安全丢失的旧配置，可提供**独立、显式、一次性的离线转换工具/导入动作**：旧格式 parser 不进入正常 runtime、不长期双栈、不自动执行，并保留原文件供用户恢复。部署 rollback 与旧协议兼容是两件事，不受本条禁止。

2. **选择能满足当前需求的最简单实现。** 不做预防性抽象，不为了“以后也许需要”增加配置层、接口层、注册中心、provider、factory、broker 或 manager。真实需求出现后再抽象。

3. **按垂直切片分层推进。** 先跑通最小端到端链路，再逐层增加功能。任何阶段都不得为了尚未实现的未来复杂度拆掉已经正确、可测试、满足当前架构方向的工作链路。

4. **组件保持模块化，关注点分离。** TSF、IPC、launcher、engine、CandidateModel、renderer、管理 UI、package/update 各有清晰 owner、输入输出和失败边界；禁止跨层直接读写内部状态。

5. **优先使用成熟、持续维护、许可证合适的库。** 没有明确的安全、性能、体积、兼容或维护理由，不自行重写成熟基础能力。

6. **先调查项目已有依赖。** 新增包或自研前，先确认 Fcitx5、现有 third_party、Windows API 和当前构建依赖是否已经提供所需能力；不得凭印象假设“库里没有”。

7. **架构决策按长期目标一次做对。** 不接受“先放宿主里以后再拆”“先用同步长阻塞以后再异步”“先 WebView 以后再 D2D”这类已知会被替换的临时架构。允许功能不完整，但已实现部分必须沿最终方向。

8. **Reference-first。** 实现重要 TSF/IPC/UI/installer/compatibility 行为前，先看 Microsoft 官方语义和成熟产品如何解决同一问题，再结合 Fcitx5 现状选最小已验证模式；不从零发明已有成熟解法。

9. **Fcitx5 Core 尽量保持 upstream。** Windows 只实现必要平台层，不重新发明输入法核心或另造一套 Windows-only addon API。

10. **宿主进程中的 fcitx5-tsf.dll 必须极薄。** 复杂逻辑、插件、完整配置解析、候选 renderer、网络和更新全部在进程外。

11. **所有输入状态必须有唯一 owner。** 使用 engine_epoch / context_id / composition_id / revision 等身份判断消息有效性；禁止多个 authoritative 状态副本。

12. **宿主稳定性优先。** Engine、UI、插件、配置或外围进程失败都不能把 Word、Explorer、浏览器、游戏等宿主一起拖垮。

13. **输入隐私边界固定。** 能看到实时输入内容的组件默认没有网络能力；能联网的更新/包管理组件在接口上拿不到实时按键、preedit、candidate、commit history。

14. **不同包类型保持不同信任等级。** Core、addon、inputmethod-data、theme、translation 不混为一类；native DLL 风险最高。

15. **安装/升级使用 staging + verify + atomic activation，并保留有界的 previous-known-good 部署回滚。** 新版本未通过 health check 前不得替换当前可用版本；成功激活后允许保留**至多一个** previous-known-good 程序版本，在明确的稳定窗口/下一次成功更新后清理。**同一 runtime generation 内禁止旧 TSF + 新 Engine、旧协议 + 新协议等混跑；但为处理已被宿主加载、无法安全原地替换的 TSF DLL，部署层允许两个完整 generation 在有界时间内 side-by-side 存活并分别使用 generation-specific IPC，直到旧 generation 客户端自然 drain。** 这不是 runtime 向后兼容：新 generation 不解析旧协议，回滚/排空也不覆盖或回滚不可再生用户数据。不可再生用户数据与程序版本分离。

16. **兼容问题必须沉淀成回归测试。** Office、Chromium、Terminal、老游戏、IMM32/CUAS 等当前宿主兼容问题直接实现当前正确路径并持续测试。Win7 属于 Legacy compatibility lane：保证其声明过的基础能力与安全/严重兼容修复，但不要求获得所有现代 Windows 新视觉/隔离能力；这仍不等于保留旧版内部协议/旧代码兼容层。

17. **性能预算是一等约束。** 输入热路径必须有延迟、CPU、内存、阻塞和队列上限；冷启动和外围功能不得把成本转嫁给每次按键。

18. **复杂度本身是一种风险。** KISS/YAGNI 优先；禁止万能 Manager、预防性 framework、新的无必要常驻进程和无法证明价值的配置开关。

19. **追求 smallest correct result，不追求 smallest diff。** 当前任务若真实需要修改多个调用方、测试或协议两端，就完整修改；但任何额外动作都必须是当前正确结果的必要后果。不能用“少改文件”掩盖不完整实现，也不能以“顺手完善”为理由扩大范围。

20. **任务意图是授权边界。** RESEARCH / REVIEW 默认只读；CHANGE 只允许实现明确请求及其必要后果；RELEASE 只允许完成本次发布所必需的版本、签名、打包和验证动作。不得把“看看”“评估”“审计”自动升级成代码修改。

21. **额外工作必须有 reachable evidence。** 在新增文件、依赖、抽象、配置、测试范围、进程、协议字段或重构前，依次回答：用户明确要求了吗？当前验收必须吗？有什么当前可达代码/数据/平台/部署事实证明必须？不做会让当前任务失败吗？无法给出证据就不做。

22. **证据先于规则，Good Case 与 Bad Case 成对。** 新的硬门禁先有真实失败或可复现 Bad Case，再写最近的合法 Good Case 证明规则没有误伤必要行为；之后才考虑机器化 enforcement。禁止仅凭偏好发明全局 gate。

23. **完成即停止。** 当前验收标准和受影响测试已有足够证据后停止继续搜索、重构、重复审计、扩大测试矩阵或增加 guard。Stable Release / 高风险边界变更按本文明确要求跑完整门禁，不以“再保险一点”为理由无限追加工作。

24. **机器只强制可高置信判断的事实。** 依赖、进程、网络能力、Hook/注入、TSF 禁止依赖、IPC schema、安装权限、敏感日志等可客观检测的变化可以自动 gate；“抽象是否漂亮”“Manager 是否多余”“架构是否优雅”等语义判断由 review + evidence 决定，不造脆弱的元规则系统。

25. **人因工程与可预期性是一等约束。** 输入法应尽可能处于用户注意力边缘：不抢焦点、不制造意外模式、不打断连续输入、不改变既定肌肉记忆。Consistency beats cleverness；稳定、可预测的交互优先于偶尔“更聪明”但不可预测的行为。

26. **系统偏好优先，用户覆盖最少而明确。** 外观默认跟随 Windows 的 Light/Dark、DPI、High Contrast 与可访问性设置；只提供少量高价值覆盖项。不得为系统已经提供的能力再造定时黑夜模式、位置推断、复杂外观规则引擎。

27. **心智负担预算与性能预算同级。** 默认路径不得要求用户理解 TSF、engine、IPC、addon ABI、package source、schema 等内部概念。系统能安全推断、继承、自动完成或提供合理默认值的事情，不要求用户决定；用户界面按用户任务而不是内部模块组织。

28. **配置格式必须少、稳定、严格且只有一个语义来源。** 人工可编辑的 Windows 外壳/主题配置统一使用 TOML 1.0；签名 manifest、lockfile、i18n 等机器数据使用严格 JSON；Fcitx addon 配置继续由 Fcitx 原生配置 API 拥有。禁止同一设置同时存在 TOML/JSON/注册表/GUI 私有缓存四套真相，禁止 include、脚本、表达式、环境变量替换和隐式 migration。

29. **DevSecOps 以风险和证据驱动，不以流程数量驱动。** Security、Privacy、Reliability、Performance、Human Factors、Accessibility、Config/Theme Correctness 与 Supply Chain 都进入同一质量闭环；低风险 PR 只跑受影响门禁，高风险边界变更扩展对应矩阵，Stable Release 才跑完整发布门禁。

30. **Build once, promote the same artifacts。** 每个明确的 release artifact lineage（例如 Modern x64、Legacy Win7 x64）只允许一次受控源码编译；通过测试的产物进入签名、打包、最终 smoke 和发布，不在签名前后重新从源码构建“另一份等价二进制”。若未来同时发布 Modern/Legacy 两条 lineage，它们必须来自同一 source commit 和各自锁定的 toolchain，并在各自 lineage 内 Build Once。签名/打包会改变最终 bytes，因此最终 hash、manifest、attestation 必须针对实际发布字节生成。

31. **工程行为优先确定性与可重放。** 时间、随机数、并发调度、文件系统根目录、IPC endpoint、网络 fixture 等测试敏感依赖必须可注入或受控；禁止把 `Sleep()`、真实公网、真实用户目录和“碰巧的线程时序”当作测试正确性的前提。

32. **边界先有契约，再有实现。** IPC、Control API、CandidateModel、config/theme typed model、package manifest 与 launcher state machine 都必须有机器可验证的当前版本契约；同版本组件必须满足契约测试，breaking change 直接同步修改双方与测试，不保留旧契约兼容层。

33. **Rust 采用以产品边界为驱动，而不是改写 Fcitx 上游对象模型。** 除直接操作 Fcitx5 core/addon 对象的 Engine adapter 留 C++ 外，产品自有 Windows 逻辑默认继续 Rust 化；迁移前必须冻结 contract/corpus，再做 side-by-side differential verification。禁止把整个 Fcitx5 C++ API 绑定进 Rust，禁止 Windows 私有 Rust rewrite upstream addons，禁止把当前 Fcitx frontend API 形状写死成不可扩展 Rust protocol。

## 0.2 现在做什么 / 暂时不做什么

| **现在必须完成**                                        | **后续再做**                                 |
|---------------------------------------------------------|----------------------------------------------|
| TSF → IPC → Fcitx5 → CandidateModel → D2D → Commit 主链 | 插件商店、主题商城、复杂在线服务             |
| x86/x64 TSF、x64 engine、焦点/Composition 生命周期      | ARM64（接口预留，第二阶段以后）              |
| Notepad / Word / Chrome/Edge / VS Code 基本兼容；Dogfood 起加入 League of Legends + Vanguard | 长尾游戏黑名单与特殊兼容模式                 |
| DPI、多屏、候选定位、系统明暗色、字体 fallback、图标/资源与故障恢复 | 高级动画、复杂在线主题能力                    |
| Win7 SP1 Legacy lane 基础能力、Win10/11 Modern lane 完整能力 | 为 Win7 锁死现代 Windows 工具链与新能力       |
| 轻量 SSDLC / DevSecOps：威胁模型、SAST/SCA/Fuzz、配置/主题/人因回归、许可证、SBOM、签名与发布门禁 | CMMI、正式 CAB、每版人工渗透、复杂 IAST 平台 |

## 0.3 Codex 的最高优先级

| **必须遵守　Codex 不得从配置器、插件商店或主题系统开始。首先审计并复用现有 Windows TSF/Fcitx 实现，稳定 “TSF → IPC v2 → Launcher/Engine → Fcitx5 → CandidateModel → 独立 UI → Commit” 主链。参考仓库只能作为行为与实现模式来源；任何历史 Hook/SendInput、长阻塞 IPC、宿主内 WebView、宽松 Pipe ACL、任意 ShellExecute 代理等做法不得因“别人已经这么做”而直接继承。** |
|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|

## 0.4 Codex 参考学习顺序：上游语义与 Windows 病例分离

Codex **不得把所有参考仓库平均通读，也不得选择一个仓库整体照抄**。Fcitx 语义只以 `fcitx/fcitx5` core 和各 addon upstream 为权威；现有 Windows port 只可作为兼容病例或历史实现对照，不是架构依据。已排除的 Windows prototype 不得回到 Engine/Package/Candidate 架构参考链。

1. **`fcitx/fcitx5`：Fcitx core 语义权威。** 重点跟踪 `Instance`、`InputContext`、`AddonManager`、`InputPanel`、`CandidateList`、config/event、candidate action、surrounding-text capability 和 addon factory 机制。
2. **各 addon upstream：插件语义权威。** Rime、Mozc、Hangul、m17n、Keyman、Bamboo、Chinese Addons 等不做 Windows 私有 Rust rewrite；Windows 产品层只适配、打包、隔离和呈现。
3. **`chewing/windows-chewing-tsf`、`openvanilla/win-mcbopomofo`、Weasel 等：Windows TSF/兼容病例教材。** 只学习 TSF lifecycle、UILess、out-of-process UI、host compatibility 和失败病例，不继承其产品架构。
4. **`fcitx-contrib/fcitx5-plugins`：跨平台插件构建与依赖清单参考。** 它不是 addon 语义权威；具体语义仍以各 addon upstream 为准。

专项参考只在对应问题出现后读取：**WindInput** 用于 overlapped IPC/timeout/stale response，**Moqi** 用于 request sequence/launcher/crash lifecycle，**Rabbit** 用于 caret/focus/password/game 用例，**PIME/libIME2/Cassotis** 用于 thin backend、x86/x64 与历史架构对照。

**Phase 默认参考权重：**Phase 1B = Microsoft TSF + Chewing + win-mcbopomofo；Phase 2 = Chewing + win-mcbopomofo，再按问题读取 WindInput/Moqi；Phase 3 = `fcitx/fcitx5` core + addon upstream；Phase 4 = Chewing + win-mcbopomofo + Weasel；Phase 5 = Weasel + Rabbit + WindInput + Moqi + 真实宿主源码/E2E。

**禁止行为：**不要在 Phase 0–4 研究 Moqi 的 AI/云同步、WindInput 的非当前产品功能、主题商店或漂亮设置器；不要因为参考项目使用某 GUI/Hook/消息转发方式就改变本项目冻结技术栈；不要复制 GPL 项目非平凡源码来“加速”，除非先完成许可证决策。

## 0.5 v1.8 实现审计基线：先稳定真实代码，再迁 Rust

**审计对象固定为：**`0x696c757a696f/fcitx5-windows-next@d12474cc2ad541c6ae3824b701c8408a22e74500`。Codex 在执行 v1.8 时不得把后续 `main` 的变化默认为本基线已包含；若更新 audit baseline，必须记录新 SHA、重新跑受影响 contract/regression，并更新本节“已修 / 仍开放”状态。

**HEAD 快照规则：**每个实现任务开始时先记录当前仓库 `HEAD`、工作树状态和与上述审计基线的 diff 范围。若 `HEAD != d12474c...`，先针对受影响目录完成最小增量审计，再执行修改；禁止引用网页缓存中的“Latest commit”文本代替本地/真实 Git SHA。UI/UX 评审尤其以当前 `src/config`、`src/ui`、`src/candidate`、resources/theme schema 的真实代码为准。

### 已在冻结快照中完成、不得重复重写的稳定化成果

- TSF 不再依赖窄按键 whitelist 来阻断 Ctrl+Space/普通标点；未处理按键按 fail-open/passthrough 语义返回。
- Engine dispatcher 使用有界 deadline，排队任务在真正执行前检查过期，避免 caller 已 fail-open 后迟到工作继续修改 runtime。
- Engine config 已从每键磁盘读取移出热路径；不得重新引入 per-key TOML/I/O。
- Config 的 child-process stdout capture 已有并发 drain 实现；应收敛成共享 authoritative process-execution primitive，而不是复制第三份。
- updater cleanup path containment、repository channel binding/anti-rollback 和多 client 并发压力已有实现/测试基础；后续只补状态鲁棒性与 corpus，不从零重写。
- package manifest/archive/signature/deployer 的 staging、路径约束、大小预算、签名/hash 与权限分离是 Keep 项；Rust R1 必须保持或加强这些性质，不允许功能回退。

### 当前 Stabilization Gate：未解决前不扩大产品功能或 Rust 迁移面

1. **UILess / PresentationPolicy：**TSF `BeginUIElement(..., show=false)` 必须能抑制独立 `fcitx5-ui.exe` popup，同时继续提供 `ITfUIElement`/屏幕阅读器语义；`popup_allowed/presentation_mode` 必须成为 context/protocol 一等状态。
2. **单一 Windows TSF Profile：**Windows 侧只注册一个固定 Fcitx5 profile，不为 Rime、拼音、Mozc、Hangul、m17n 等 engine/input method 动态增加 TSF profile。默认 Windows 显示名为 **`Fcitx5`**，TSF/语言栏使用一个固定、语言中立的 Fcitx5 图标；当前 Fcitx input method/group、BCP-47/locale、DWrite language 与候选模式由 Engine/Presentation metadata 表达，不靠新增 Windows profile。
3. **通用 KeyEvent contract：**协议不能长期只依赖 `virtualKey + modifier flags`。在协议 breaking change 中定义 physical/logical key、scan code/extended、release、keyboard-layout/AltGr/dead-key 所需语义，再转换到 Fcitx KeyEvent；避免把 `VK_*` 手写映射扩成平台核心。
4. **Fcitx InputContext capability：**补齐 real-engine 需要的 surrounding-text、delete-surrounding、forward-key、atomic surrounding-text replacement、candidate action 等能力/协商；当前空实现不得作为“通用 Fcitx frontend”完成证据。Rust protocol 必须是可扩展 capability model，不得把当前 Fcitx API 固化成 `Commit(String)` / `Delete(i32,u32)` 这类封闭动作集。
5. **Warmup 无副作用：**禁止通用 warmup 伪造 `n` 或其他真实用户文本键。只允许显式 preload/engine-loading API 或经过测试证明无学习/commit/history/global side effect 的 addon-specific hook。
6. **CandidateModel A→B→A：**composition/revision 顺序必须按 `(engine_epoch, context_id, composition_id, revision)` 作用域判断；不同 context 的 compositionId 不互相比全局大小。
7. **Launcher crash ledger：**SafeMode/crash-window/backoff 的最小 ledger 必须跨 launcher 自身重启保留；只持久化恢复语义所需字段，不把整个内部状态机序列化。
8. **Control process capture：**`fcitx5-control` 不得先 wait child 再读匿名 pipe；Config/Control 收敛到唯一并发 drain + bounded output + timeout/cancel/job containment 实现。
9. **Repository sequence state：**区分 never-initialized 与 expected-but-corrupt/missing；成熟安装的 anti-rollback 状态损坏默认 fail-closed 到显式 repair/reset；写入使用原子发布。
10. **Peer executable identity：**从“规范化路径字符串相等”升级到 handle-based final identity/file-ID 级校验，同时保留 SID + Session + DACL 边界。
11. **Installer machine/user ownership：**管理员安装只拥有 machine artifact/系统注册；per-user startup/session/config 由 user-plane 拥有。跨账户 UAC 安装/卸载必须能清理正确 owner SID，不能依赖 elevated uninstaller 的当前 HKCU。
12. **Register/bootstrap side-effect helper：**production register helper 只消费已验证 product/staged artifact；有副作用的 child timeout 必须能确认终止/完成，不能 UI 报失败后后台继续修改系统。
13. **Candidate locale/config polling：**DWrite locale 不硬编码 `zh-CN`；由 active Fcitx input method / content locale / 文本语言策略决定。配置 reload 以 generation/broadcast 为主，文件时间只做低频 fallback，不按 candidate snapshot 查磁盘 metadata。
14. **Windows hostile-path corpus：**在 Rust R1 前固定 DOS device names、case collision、trailing dot/space、reparse、`..`、separator、控制字符等 corpus；不能把“Rust PathBuf”当作自动安全证明。
15. **Win7/架构宣称：**当前 x86 主要表示 64-bit Windows 上的 32-bit host/TIP 兼容；除非 installer/native engine/VM 真的覆盖 32-bit Windows OS，不得宣传 32-bit OS 支持。Win7 只有通过真实 VM install→register→input→candidate→uninstall 才算 runtime-proven。
16. **TSF in-use DLL 更新 / Generation Draining：**不得尝试覆盖或远程卸载仍被宿主加载的 `fcitx5-tsf.dll`，不得通过杀 Word/Chrome/游戏作为正常升级方案。更新必须支持“旧 DLL rename/留存 + 新 DLL canonical path 激活 + generation-specific runtime/IPC + 旧客户端自然排空 + 有界清理/必要时 reboot delete”。同一 generation 严格同协议，新 Engine 不为旧 TSF 增加 decoder/shim。

### Rust 迁移波次

| 波次 | 组件 | v1.8 决策 |
|---|---|---|
| **R1** | package core / repository / updater / downloader / provider / deployer | **首选 Rust 迁移域。** 先固定 C++ contract/corpus；Modern lane side-by-side Rust → differential → artifact smoke → 单组件 cutover。elevated deployer 必须保持最小；Legacy 若 Rust Win7 PoC 未通过可暂留 C++ lineage。 |
| **R2** | launcher / control / diagnostics / shared process-execution | **第二波。** 先修 crash ledger、child process 与恢复语义，再迁；状态机用强类型/enum 表达，不能把旧逻辑 bug 原样翻译。 |
| **R3** | TSF / Candidate UI / Config | 继续向 Rust shipping implementation 迁移；Windows/COM/D2D/WTL 外壳可阶段性保留为薄 adapter，但产品状态、协议、策略和可测试语义进入 Rust。 |
| **Fcitx C++ island** | `fcitx5-engine.exe` 中直接操作 Fcitx core/addon 对象的 adapter | 仅保留 `fcitx::Instance`、`fcitx::InputContext`、`InputContextManager`、`FocusGroup`、`AddonManager`、`AddonInstance`、`InputMethodEngine`、`InputPanel`、`CandidateList`、Fcitx config/event conversion。Engine 其余 protocol/state/validation/revision/generation/policy/IPC 可 Rust 化。 |

**迁移规则：**每个 Rust 组件必须按“C++ 语义修正 → contract/golden/fuzz corpus → Rust side-by-side → differential test → security/artifact smoke → **行为一致后再做 performance comparison** → cutover → 同批删除旧实现”的顺序推进。不得永久双栈，不用 runtime feature flag 在 C++/Rust 两套实现之间长期切换；不得在 differential 尚未通过时同时改性能算法/异步策略。回滚依赖 previous-known-good artifact，而不是保留两套 runtime implementation。

# 目录

1. 产品边界与兼容目标
2. 总体架构与进程边界
3. TSF Frontend 设计
4. Engine、InputContext 与 IPC
5. Candidate UI、UILess、皮肤与人因工程
6. Fcitx5 Addon 与跨平台策略
7. 包管理、方案、Plum 与更新
8. 安装版、Portable 与系统包管理器
9. i18n 与多语言 Input Profile
10. 隐私与安全架构
11. 个人项目版轻量 SSDLC / DevSecOps
12. 测试、老游戏、反作弊、宿主兼容与人因回归
13. 仓库结构、C++/Rust/WTL 规则、性能复杂度、现代软件工程与构建链
14. 分阶段执行计划与验收标准
15. Codex 执行协议
16. 附录：威胁模型、决策、参考实现、回归用例、Linux/Windows 差异、扩展源码


# 1. 产品边界与兼容目标

## 1.1 产品定义

Fcitx5 for Windows Next 是一个通用 Windows 平台前端与发行层。Windows 平台只负责 TSF、候选 UI、配置/安装/更新/包管理和系统集成；输入算法尽量复用 Fcitx5 upstream 及其 addon。

## 1.2 平台支持矩阵

| **系统**         | **支持级别**    | **要求** |
|------------------|-----------------|----------|
| Windows 7 SP1    | Legacy lane     | 当前发行目标按 **x64 OS + x86/x64 host/TIP** 解释；TSF、基础 D2D/DWrite 候选 UI、基础主题、核心 addon、WTL 配置器基础功能；安全与严重兼容修复；System DPI；不承诺所有现代视觉/隔离能力。32-bit Windows OS 只有独立 installer/native-engine/VM 证据后才可宣称支持 |
| Windows 8 / 8.1  | Legacy lane     | 原则同 Win7；可用能力按 capability detection 启用，不为其新增独立产品分支 |
| Windows 10 1809+ | Full baseline   | 完整功能基线；现代 DPI、安全隔离、管理与视觉能力 |
| Windows 11       | Primary         | 主测试/设计平台；优先验证最新 Windows 行为与能力 |
| ARM64            | Planned         | 接口/包格式预留；主链稳定后实现，不阻塞 x86/x64 |

**兼容策略：一套源码，两条能力 lane，不 fork 产品。** Modern lane 面向 Windows 10/11，允许使用当前受支持的新工具链与新 API；Legacy lane 面向 Win7/8.x，使用明确 pin 的兼容工具链和最小 API 集，只承诺声明过的基础能力。平台差异优先通过 runtime capability detection / `GetProcAddress` 处理；不得维护 `win7/` 与 `modern/` 两套业务实现。

- C++ Legacy release lane 使用仍能 target Win7 的固定 MSVC toolset/SDK；最新 MSVC 不再能直接 target Win7 时，不因此锁死 Modern lane。
- Rust R1/R2 **默认先服务 Modern lane**。Rust 1.78 起普通 `*-pc-windows-*` target 的最低系统基线为 Windows 10；专门的 `i686/x86_64-win7-windows-msvc` 当前为 Tier 3 且不直接分发预编译 target artifacts。某组件若进入 Legacy lane，必须先用独立 PoC 证明 toolchain/std/dependency/min_os/import/VM/CI 成本可接受并通过 ADR；在此之前该组件的 Legacy lineage 可继续使用已经验证的 C++ 实现。
- 若当前阶段一个共同的 Legacy-compatible build 已经同时满足 Modern/Legacy，允许只维护一套发行物；只有真实依赖或工具链价值证明必要时才拆成 Modern/Legacy 两条 artifact lineage。
- 如果未来 Win7 维护成本超过明确收益，直接从支持矩阵删除 Legacy lane；不保留“半支持但没人测”的灰色状态。

## 1.3 非目标

- 不重写 Fcitx5 核心或另造 Windows 专用输入法插件 API。

- 不使用全局键盘 Hook + SendInput 作为正常输入路径。

- 不把 Qt、WebView2、配置器 GUI 或其他管理层/GUI framework 加载进 TSF DLL。

- 不把 Rime/Plum 设计成整个发行版的中心；它只是 addon / provider 之一。

- 不承诺任意第三方 native addon 安全；必须有信任等级和安装提示。

## 1.4 产品差异化与平台价值验收

Fcitx5 for Windows Next 的长期价值不是“另一个 Rime Windows 壳”，而是**一个 Windows frontend 承载多个输入引擎，并共享候选 UI、主题与通用 addon 能力**。

- Rime 是一等公民，但不得成为架构中心或唯一成功标准。
- 在公开 1.0 前，必须至少证明两个真实 engine/input method 共用同一 TSF frontend 与 Candidate UI，其中至少一个不是 Rime；第二个 engine 不得要求重写 Windows frontend、renderer、theme/config/package 基础设施。
- Clipboard、Quick Phrase、Unicode/Emoji、简繁转换等通用能力优先作为 Fcitx/平台级 addon 能力共享，不为每个 engine 重写一份。
- 一套 `theme.toml + assets` 必须能跨支持的 engine 生效；皮肤属于 presentation，不改变输入语义。
- 市场/README 只宣传已经验证的 engine 与能力；产品定位优先描述“Windows 通用输入平台 / 多引擎统一体验”，不以 Rust、WTL、D2D 等实现细节作为主要卖点。

# 2. 总体架构与进程边界

Windows Application
│
│ TSF
▼
┌───────────────────────┐
│ fcitx5-tsf.dll │ x86 / x64
│ COM / TSF / IPC only │ no network, no addon
└──────────┬────────────┘
│ versioned local IPC
▼
┌───────────────────────┐
│ fcitx5-engine.exe │ native OS architecture
│ Fcitx5 + libime │
│ addons + InputContext │
└──────┬─────────┬──────┘
│ │
│ └──────────────► addons
▼
┌───────────────────────┐
│ fcitx5-ui.exe │
│ C++ / Win32 / D2D / DWrite │
└───────────────────────┘

Management plane
┌───────────────────────┐
│ fcitx5-config.exe │ C++ / WTL
└──────────┬────────────┘
▼
┌───────────────────────┐
│ fcitx5-package.exe │ C++ current → Rust R1 / verify/resolve/stage/activate
└──────┬─────────┬──────┘
│ └────► deployer / provider adapters
└──────────────► updater / repositories

## 2.1 信任边界

| **边界**           | **风险**                      | **约束**                                             |
|--------------------|-------------------------------|------------------------------------------------------|
| Host App ↔ TSF DLL | 任何崩溃会伤害宿主            | TSF 极薄；边界 noexcept；不解析用户配置；不加载插件  |
| TSF ↔ Engine       | 版本错配、阻塞、恶意/损坏消息 | 版本握手、长度上限、schema 校验、timeout、generation |
| Engine ↔ Addon     | native code 可读输入、可崩溃  | 只进 engine；签名/来源/ABI 校验；Safe Mode           |
| Engine ↔ UI        | 过期候选、位置/状态串线       | CandidateModel + revision；UI 无输入业务逻辑         |
| Config ↔ Engine    | 配置器意外获得输入数据        | 控制协议不暴露实时输入/历史 API                      |
| Updater ↔ Internet | 供应链攻击                    | 签名 manifest、SHA-256、代码签名、密钥轮换/吊销      |

## 2.2 输入数据平面与管理平面

```text
输入数据平面（敏感、低延迟、默认禁网）
Host → TSF → IPC → Engine/Fcitx → CandidateModel → UI / Commit

管理平面（低频，可按需联网，但拿不到实时输入）
Config(WTL) → typed Control API → Package/Updater/Repair
```

任何新功能先判断属于哪个平面。需要网络的同步、AI、下载等能力不得直接进入能观察实时输入的 engine/TSF/UI 进程；如未来提供网络功能，应通过显式 Network Broker 和用户动作传递最小必要数据。

# 3. TSF Frontend 设计

## 3.1 DLL 职责白名单

- COM / TSF 注册与激活；ITfTextInputProcessorEx 等必要接口。

- KeyEvent、Focus、ThreadMgr/Context 生命周期。

- Composition、EditSession、Compartment 同步。

- Candidate UIElement / UILess 桥接。

- 最小 IPC client 与 engine handshake。

## 3.2 DLL 禁止事项

- 禁止加载 libime、Rime、Lua、OpenCC、第三方 addon。

- 禁止解析 YAML/JSON/主题/完整用户配置。

- 禁止 HTTP、WinHTTP、WinINet、curl、WebView、更新器和遥测。

- 禁止写入包含 key/preedit/candidate/commit 内容的日志。

- 禁止把 C++ exception、SEH 可恢复异常或未验证 IPC 数据穿出 COM/TSF 边界。

HRESULT SomeTsfEntry(...) noexcept {
try {
return SomeTsfEntryImpl(...);
} catch (...) {
ResetLocalStateWithoutInputLogging();
return E_FAIL;
}
}

## 3.3 32/64 位与进程模型

- x64 Windows 同时提供 x86 TSF DLL 与 x64 TSF DLL。

- 两者通过 IPC 连接同一个原生架构 engine；不为 32 位应用额外运行一份 32 位 Fcitx5。

- 未来 ARM64 使用独立 TSF/engine 产物和包架构字段。

- 同一用户 + Session 只允许一个 active engine，启动过程用命名 mutex + 二次连接检查避免竞争。

## 3.4 宿主进程内代码的爆炸半径

最低要求：

- TSF DLL 不解析 YAML/JSON、主题、词库或第三方数据；复杂 parser 必须出进程。

- 不允许任何第三方 native addon、配置 GUI、Qt、WebView2、Rust GUI/runtime framework、Fcitx/libime 本体进入 TSF DLL。

- 关键边界启用 RAII、异常封口、长度校验、CFG/DEP/ASLR/CET（工具链可用时）；debug/CI 使用 Application Verifier、PageHeap 或 ASan 等发现 UAF/heap corruption。

- Release 前使用 Process Monitor / Process Explorer 或等价工具审计宿主进程实际加载的 DLL 路径；任何从当前目录、Temp、Downloads 或其他非预期用户可写目录解析敏感依赖的行为视为发布阻断。

Windows TSF frontend 与 Linux/Wayland/DBus frontend 的关键差异是：TSF DLL 会进入宿主进程。任何内存破坏、未捕获异常、错误 DLL 依赖、阻塞等待或复杂 UI 初始化，都可能直接影响 Word、Explorer、浏览器、游戏等宿主。故 TSF DLL 的安全严重度按“宿主进程代码”评估，而不是按普通辅助进程评估。

- 4. Engine、InputContext 与 IPC

# 4. Engine、InputContext 与 IPC

## 4.1 InputContext 是状态中心

ITfContext A → Fcitx InputContext A
ITfContext B → Fcitx InputContext B

ContextState = {
engine_epoch,
session_id,
context_id,
generation,
composition_id,
revision,
focus_state,
input_mode
}

| **禁止　不得使用 CurrentWindow / CurrentComposition / CurrentCandidate 这种单全局状态模型。多个应用、多个 context、RDP/多 session 都会让这种模型失效。** |
|----------------------------------------------------------------------------------------------------------------------------------------------------------|

## 4.2 IPC 协议最低字段

Header {
magic
protocol_version
message_type
payload_length
request_id
engine_epoch
session_id
context_id
composition_id
revision
}
Payload { validated structured data }

**协议版本规则：** client/server 必须使用当前相同协议版本。版本不一致直接拒绝连接并要求同版组件；不得增加旧版本 decoder、兼容 shim、双写/双读协议或自动 migration。协议 breaking change 时直接删除旧协议代码并同步升级所有项目内组件。

**Dispatcher / backpressure contract：**

- 多个 TSF host/process/thread 可以并发成为 producer，但 Fcitx Instance 只能由唯一 engine execution context 调用。
- 入队必须有 bounded capacity 与 absolute deadline；队列满或预算不足时优先 fail-open/passthrough，不得无界堆积。
- request 在真正执行 Fcitx 前再次检查 deadline/generation/context；已过期请求 DROP，禁止产生 composition/learning/history 副作用。
- 同一 `(session, context)` 的输入事件保持必要顺序；不同 context 只共享 Fcitx 明确定义的全局状态，不人为制造跨 context composition ordering。
- 不以 `Sleep()`/线程时序碰巧正确作为测试；使用 deterministic fake clock、queue saturation 与 late-work corpus。

- 握手失败时优雅拒绝，不尝试“按旧 struct 猜着解析”。

- 所有消息有最大长度；所有字符串显式编码；所有数组长度校验。

- Named Pipe DACL 绑定当前 logon SID；namespace 包含 User SID + Session ID。

- Engine 重启必须增加 engine_epoch，旧 epoch 的所有响应自动作废。

- 输入热路径有严格 timeout；engine 不响应时 fail-open / passthrough，不允许无限阻塞宿主输入线程。

## 4.3 Fcitx 调用线程模型

Many TSF clients
↓
IPC dispatcher
↓
Fcitx event queue
↓
Single authoritative Fcitx event loop
↓
InputContext A/B/C/...

| **原因　不要让 Word、Chrome、VS Code 的多个 IPC 线程同时直接调用同一个 Fcitx instance。并发只存在于边缘，核心状态统一进入 Fcitx 自己的事件循环。** |
|----------------------------------------------------------------------------------------------------------------------------------------------------|

## 4.4 故障恢复

engine crash
↓
invalidate engine_epoch
↓
cancel/end current composition safely
↓
hide candidate UI
↓
restart engine
↓
next key starts a new session

- 不要恢复半截 composition。

- 短时间连续崩溃达到阈值后进入 Safe Mode，禁用第三方 addon、Lua 扩展和用户主题。

- Watchdog 只能重启 engine，不能无限 crash-loop。

## 4.5 IPC v2：有界等待、请求关联与对端认证

现有参考实现暴露了三类真实问题：gaboolic/fcitx5-windows 的早期 pipe 协议字段不足且同步阻塞；WindInput 已实现 overlapped I/O、timeout、circuit breaker 与 stale-response 处理；Moqi 使用 seq_num 解决同步响应与异步通知交错；windows-chewing-tsf 则提供 SID/ACL 与 server 身份校验思路。IPC v2 必须吸收这些经验。

> Header {
> magic; protocol_version;
> message_type; payload_length; request_id; response_to;
> engine_epoch; session_id; context_id; composition_id; revision;
> }
> Payload { validated structured data }

强制要求：

- 输入热路径不得出现秒级等待。所有读写必须有明确 deadline；timeout 后 fail-open / passthrough，断开或重建连接，不能让宿主输入线程无限等 engine。

- request_id/response_to 必须显式关联响应；迟到的旧响应不得被下一次请求误收。旧 engine_epoch/context/composition/revision 的异步消息一律丢弃。

- Pipe namespace 以 User SID + Session ID 为身份基础，不以可碰撞的用户名作为安全标识；CreateNamedPipe 使用显式 DACL/SACL，限制当前用户/必要系统主体，并拒绝网络访问。

- 客户端连接后应验证 server PID，核对预期 executable path；公开签名发行版进一步使用 WinVerifyTrust/Authenticode 验证 peer。Debug/dev 构建可允许受控例外，但必须显式标记。

- I/O 可以并发接受多个 TSF client；Fcitx 核心状态变更仍通过 dispatcher 进入单一 authoritative event loop，不能让多个 pipe worker 线程直接并发调用同一个 Fcitx Instance。

## 4.6 Launcher / Engine 生命周期状态机

fcitx5-engine.exe 是 per-user / per-session 进程，不设计成 Session 0 Windows Service。推荐一个极小的 fcitx5-launcher.exe 负责启动、监督、预热和状态机；TSF 只负责快速连接、按需请求启动和安全失败。

> LauncherState =
> Normal \| UserStopped \| Updating \| Uninstalling \|
> CrashBackoff \| SafeMode
>
> TSF connect -> launcher/engine ready ? use : bounded start request -> retry -> fail-open

- 登录后可预启动并 warm-up Fcitx/addon/词典缓存，降低第一键冷启动成本；warm-up 走内部初始化，不伪造真实用户输入。

- engine crash 后增加 engine_epoch，取消/结束当前 composition、隐藏 candidate；launcher 按运行时长和 crash count 决定立即重启、退避或进入 Safe Mode。

- 短时间连续 startup-crash 禁止高速 respawn；采用 backoff/circuit-breaker，必要时等待下一个真实输入需求再 lazy restart。

- Installer/Updater 设置 Updating/Uninstalling 状态，TSF 在此期间不得“帮忙”把刚停止的 engine 拉起；Portable 主动停止使用 UserStopped marker，避免下一键自动复活。

- 如果 TSF 激活在 SYSTEM/LogonUI/secure desktop 上下文，不得启动普通用户 engine 或建立错误用户数据目录；安全降级或等待正常用户 session 的 launcher。

- launcher 本身故障时，下一次正常 TSF activation 仍必须能够重新建立 launcher/engine；Config 提供第三层 Repair/Health Check。

- 5. Candidate UI 与皮肤

# 5. Candidate UI 与皮肤

## 5.1 CandidateModel 与 Renderer 分离

CandidateModel
├─ preedit
├─ aux_up / aux_down
├─ candidates\[\]
│ ├─ id
│ ├─ label
│ ├─ text
│ └─ comment
├─ selected
├─ page
├─ total
└─ visibility
│
├─ Direct2D Renderer
└─ TSF UILess UIElement

- **Engine/Fcitx 是 authoritative candidate state 的唯一 owner。** `fcitx5-ui.exe` 与 TSF UIElement 只持有带 `engine_epoch/context_id/composition_id/revision` 的 immutable snapshot/cache；不得在 renderer/TSF 侧形成第二套 selected/page/candidate 真相。点击或键盘选择只作为 intent 发回 engine，由 engine 产生新的 snapshot 或 commit。

- 候选窗可见性不能只由 preedit 是否为空决定；预测候选可能在空 preedit 时存在。

- 皮肤只控制 measure/layout/paint/animation/hit-test geometry，不控制候选语义与 commit。

- 候选窗不抢焦点、不进 Alt+Tab，处理 WS_EX_NOACTIVATE 等窗口属性。

- 候选定位必须处理无效 caret、TS_E_NOLAYOUT、last-valid rect、work-area clamp、上下方位置锁定。

## 5.2 渲染技术

| **部分** | **选择** | **理由** |
|----------|----------|----------|
| 候选窗 | **C++ + Win32 + Direct2D + DirectWrite** | Windows 原生、Win7→Win11 工具链/调试经验最成熟；低常驻开销；Unicode/字体 fallback、DPI、device loss 与皮肤渲染可精确控制 |
| 配置器 | **C++ + WTL/ATL + Win32** | Windows-only、轻量、成熟、可无人值守 CMake/MSVC 构建；不进入输入热路径；复杂视觉仅做必要 owner-draw/D2D |
| 候选预览 | 复用真实 Candidate renderer 的 preview host / presentation path | 避免 WTL 再实现第二套候选渲染，保证配置预览与真实皮肤一致 |
| WebView2 / Web UI | 不作为默认依赖 | 配置器无需浏览器 runtime；未来只有明确、不可替代的管理 UI 需求才重新评估 |
| 其他 GUI framework | 当前不采用 | 现有 WTL + 原生 renderer 已满足 Windows-only、Legacy lane、CI/CD 与美观需求；禁止无证据重复换框架 |

Candidate renderer 的内存安全主要通过**进程隔离 + RAII + smart COM pointer + bounded IPC + fuzz/ASan/Verifier + 明确 state owner**保证。不得为了“语言更安全”引入需要自行维护 Tier-3 Legacy toolchain 的复杂度，除非真实缺陷数据证明收益高于成本。

## 5.3 皮肤包

theme-package/
├─ manifest.json
├─ theme.toml
├─ assets/
├─ light/
└─ dark/

- 允许字体、DIP 尺寸、padding、margin、radius、border、shadow、background、nine-patch、横/竖排、highlight、label/comment、light/dark、简单动画。

- 默认不允许 skin.lua / skin.js / DLL / WASM 等任意可执行主题；主题只允许受限数据与资源。

- 所有尺寸使用逻辑单位 DIP；Win10+ 使用 Per-Monitor V2，Win7 使用其当前受支持的 System DPI 路径。

- Direct2D device loss 必须可重建；睡眠/显卡重置后候选 UI 不得永久空白。

## 5.4 UILess、无障碍与独立 UI 进程

UILess 不是“以后补的无障碍特性”，而是 Windows TSF、游戏、屏幕阅读器和某些宿主兼容性的共同能力。CandidateModel 必须同时支持自绘 UI 与 ITfCandidateListUIElement 等 TSF UIElement 表达。

- 最终候选窗口不放在 TSF DLL 内。采用独立 `fcitx5-ui.exe`：C++ + Win32 + Direct2D + DirectWrite；TSF/engine 只发送 candidate snapshot、caret/layout 与语义事件。

- 候选/tooltip renderer 崩溃或卡顿不得阻塞 key processing；Presentation channel 在协议层与 Input/Control message 区分，必要时可独立队列或独立连接。

- 为 NVDA/屏幕阅读器提供稳定的 composition、candidate count、selected candidate、candidate change 语义；不能只保证“肉眼看到 popup”。

- 参考 win-mcbopomofo / windows-chewing-tsf 的 out-of-process candidate 思路；参考 Rabbit/WindInput/Weasel 的 layout、caret 和 D2D 经验，但不复制其 Hook/SendInput 或宿主内 UI 历史实现。

## 5.5 人因工程：低打扰、可预期与低认知负担

输入法属于高频、低容错、长期依赖肌肉记忆的软件。正常体验应尽量是“想输入 → 输入 → 候选 → 选择 → 继续工作”，而不是要求用户持续判断内部模式、服务状态或实现细节。

### 5.5.1 注意力与焦点

- Candidate UI MUST 不抢焦点、不激活宿主外窗口、不进入 Alt+Tab；普通状态变化不得弹阻塞对话框。
- 更新下载、词库扫描、插件检查、缓存预热等后台活动 MUST 安静进行，不因内部维护任务打断实时输入。
- 正在输入、游戏或全屏时，更新程序 MUST NOT 主动重启 engine；激活更新应等待明确安全边界或用户主动动作。
- 用户发起的长操作需要“进行中 → 成功/失败”的明确状态；普通成功不额外弹窗。
- 模式切换提示可短暂显示 `中` / `A` 等状态，但 SHOULD 自动消失，不长期占据注意力。

### 5.5.2 可预期性与肌肉记忆

- 同一状态、同一按键 MUST 得到稳定语义；不得靠模糊 heuristic 随机改变快捷键、候选选择、提交或模式切换方式。
- Space、Enter、Esc、Backspace、数字选词、翻页和中英切换一旦成为当前产品规范，变更视为 UX breaking change，不能由普通 refactor 顺手改变。
- Application Compatibility Profile 可以改变行为，但规则必须明确、稳定、可测试；不得“猜测用户意图”后偷偷改变键位。
- 鼠标 hover MUST NOT 静默改变 keyboard-selected candidate；真正点击后才改变选择/提交语义。
- 无有效 Windows text context 时，IME MUST fail-open，不截获 WASD、Space、Shift、Ctrl、数字键等游戏输入。

### 5.5.3 错误恢复与反馈

- 用户误入 composition、误选模式或候选时，应尽可能通过一次 Esc/Backspace 或一个明确动作恢复；不可把取消动作带入新的隐式模式。
- 已被 IME 接管的按键必须快速产生可理解反馈：composition 变化、candidate 变化、commit 或 fail-open；禁止“按了但不知道是否收到”的悬空状态。
- 一次 IPC timeout、旧 generation 或迟到 response MUST NOT 污染后续正常输入。
- 对用户展示的错误信息必须包含下一步动作，例如“输入服务未运行 → 重新启动”；HRESULT/Win32 错误码放 Details。
- 可取消的长操作必须真正支持取消；没有真实取消语义就不要画假的 Cancel 按钮。

### 5.5.4 候选窗视觉稳定性

- 一个 composition 周期内，候选窗 SHOULD 尽量锁定在 caret 上方或下方，避免因为候选宽度变化在上下方向频繁跳动。
- Candidate UI 不得遮住 caret 和正在输入的核心文本；空间不足时按 work area 选择上方/下方并 clamp。
- 候选内容变化不得导致无意义的 layout shift；窗口宽度/高度应有稳定策略和最大值。
- selected candidate 的视觉层级必须明显高于普通候选；主文字 > index > annotation/comment。
- 不能仅靠颜色表达 selected / broken / warning / enabled 等状态；需要形状、文字、图标或可访问语义中的至少一种额外通道。
- 动画默认极少；任何动画不得成为输入反馈延迟的一部分。隐藏 Candidate UI 后 MUST 不持续 render loop。

### 5.5.5 配置器的人因原则

- 首次安装后应能直接切换到 Fcitx5 并输入；不得要求新用户先理解 addon、profile、IPC、Rime deploy 或包源。
- 默认页只放高频设置；ABI、日志、IPC、调试、数据目录、实验项放 Advanced。
- 软件能安全判断并完成的内部动作不要转嫁给用户。例如启用插件需要重启 engine 时，在安全边界自动完成，而不是询问用户“是否重启内部服务”。
- 可逆且低风险的 toggle/theme/font 选择通常即时生效；复杂多字段编辑可以使用明确 Apply/Cancel。
- 只有真正不可逆或高损失操作需要确认，例如删除不可再生用户词库。
- 配置器关键任务必须可全键盘完成，Tab 顺序稳定，focus indicator 明显。

### 5.5.6 Cognitive Load Budget / 心智负担预算

目标不是让用户学会 Fcitx5 的内部架构，而是让用户尽量不需要意识到这些内部概念。设计评审必须回答：**用户为了完成当前目标，需要知道多少与目标本身无关的信息？**

#### A. 零配置与渐进披露

- 安装完成后 MUST 能直接选择 Fcitx5 并开始输入；首次启动不得强制展示 onboarding wizard。
- 只有确实存在不可推断的必要信息时才询问用户；主题、字体、候选方向、包源、日志级别等都使用合理默认值。
- 设置界面只允许三层信息级别：`Basic`、`Advanced`、`Diagnostics`。不得继续增加 Expert / Developer / More Advanced 等层级。
- Basic 只展示高频、用户能直接理解的决定；内部实现参数不得通过“高级设置”逃逸成用户责任。
- 首页 SHOULD 控制在约 8–12 个主要控制项以内；单一视觉分组 SHOULD 控制在约 5–7 个核心概念以内。超过时视为认知负担报警线，需要证明无法通过分组、继承或删除配置项解决。

#### B. Recognition over recall

- 用户不需要记 theme ID、addon ID、字体完整名称、路径、快捷键语法或内部服务名；GUI 应使用 picker、列表、搜索、预览和友好显示名。
- 稳定内部 ID 与显示名称分离。UI 显示 `Rime / 小狼毫 / 晨星键道` 等友好名称；依赖、路径、注册和协议只使用稳定 ID。
- 一个设置只应有一个主要入口。不得为了“方便”在多个页面复制同一个 authoritative toggle。
- 相互依赖的设置必须显式表现。例如关闭候选注释后，与 annotation 相关的视觉控件应禁用或折叠，不能留下“能改但没效果”的控件。

#### C. 默认、继承与自动完成

- 能继承就不要求重复选择。Annotation 字体默认继承 Candidate；相对字号优先使用 scale，不要求用户维护多个近似值。
- 软件能安全完成的内部动作自行完成。例如启用 addon 后若必须重启 engine，应在安全边界自动完成，不询问“是否重启内部服务”。
- 用户选择的是目标，例如“启用插件”“更换主题”“重新输入”，不是实现步骤，例如“重启 renderer”“刷新 IPC”“部署 backend”。
- 网络源、mirror、缓存目录、内部 timeout 等存在可靠默认策略时不得暴露给普通用户。

#### D. 可逆性、确认与中断预算

- 可逆低风险操作默认直接执行并支持 Reset/Undo；不得弹确认框。
- 只有不可逆或高损失动作允许 modal confirmation，例如删除不可再生用户词库。
- 普通成功使用 inline state 或静默完成，不弹“操作成功”窗口。
- 提示等级只允许：`Inline`、`Non-blocking`、`Modal`。Modal 只用于无法继续或明显数据损失风险。
- 后台维护、更新检查、主题扫描和缓存工作不得通过 tray icon 闪烁、toast 洪泛或频繁 spinner 消耗注意力。

#### E. 设置生效模型

所有设置只属于三种生命周期：

```text
Live
  安全地立即生效，例如字体、颜色、布局、主题预览

Deferred
  当前 composition / 输入事务结束后生效

Restart-required
  只有技术上确实无法热更新的组件才允许
```

不得每个设置自行发明一套“应用 / 保存 / 重载 / 重启”语义。GUI 必须向用户表达结果，而不是内部机制。

#### F. 错误与帮助文本

- 普通错误信息固定回答三件事：`发生了什么`、`是否影响输入`、`下一步怎么办`。技术错误码放 Details。
- 不用报错来教育用户。能通过 picker、enum、range、disable state 防止无效输入的地方，不允许先接受任意字符串再报错。
- 术语必须统一；`输入法`、`输入方案`、`插件`、`主题`、`用户词库`、`更新` 等一旦确定，不在不同页面改叫 schema/module/provider/profile。
- 面向普通用户的文案禁止暴露 `TSF`、`IPC`、`ABI`、`HRESULT`、`Named Pipe` 等术语，除非位于 Diagnostics/Details。

#### G. 配置项准入规则

新增用户可见设置前至少满足一项：

1. 存在真实、常见且合理的不同用户偏好；
2. 解决当前支持平台或应用的真实兼容问题；
3. 明显影响高频输入体验，且系统无法可靠自动决定。

如果只是方便开发、暴露内部实现、预防未来需求或“高级用户也许想调”，默认不增加设置。Direct2D antialias mode、IPC timeout、buffer size、worker count、内部 cache 参数等不属于普通设置。

### 5.5.7 人因验收指标

至少记录并持续回归：

- 首次安装到第一次成功输入所需的用户动作数；
- 中英切换、候选翻页、取消 composition 的操作一致性；
- engine/UI crash 后继续输入所需的额外动作数；
- 候选窗在连续输入中的位置跳动次数；
- 100% / 125% / 150% / 200% / 300% DPI 下布局稳定性；
- Light / Dark / High Contrast 下可读性；
- keyboard-only 与 Narrator/NVDA 下完成关键任务的可行性；
- p95/p99 输入反馈延迟，而不是只看平均值；
- 首次启动是否出现非必要 wizard / modal；
- 完成“换主题、改候选字号、改横竖排、启用/禁用输入法”是否需要理解内部组件名；
- Basic 页面主要控制项数量和单组概念数量是否超过认知负担预算；
- 同一个 authoritative setting 是否在多个页面重复出现；
- 可逆操作是否出现不必要确认框；
- 普通用户路径中是否暴露 TSF / IPC / ABI / backend / package source 等内部术语。


### 5.5.8 Config UI/UX：现代原生设置器，而不是传统属性页

当前 HEAD 已具备 WTL/Win32 窗口、左侧导航、D2D 背景/卡片、DPI 处理与 production Candidate Preview 基础；**不得把“现代化”理解为继续给传统 Win32 表单换背景色、加圆角。** Config 的视觉目标是“现代、原生、克制、密度适中”，避免 VC6/Windows 2000 属性页感，也避免网页化、毛玻璃堆叠和过度动画。

**技术边界固定：**

```text
WTL / Win32
  → window lifecycle / message routing
  → native dialogs / text edit / accessibility bridge
  → hosting

D2D / DWrite Settings Surface
  → NavigationItem
  → SettingRow / SettingSection
  → Toggle
  → SegmentedControl
  → Slider
  → ThemeCard
  → InputMethodCard
  → InlineBanner / StatusBadge
  → CandidatePreview
```

- WTL 继续作为窗口和系统集成框架；**不得为了美观切换到 Tauri/Slint/Qt/WebView。**
- 普通 `STATIC + EDIT + COMBOBOX + CHECKBOX` 的“标签-控件表格”不得成为主要视觉语言。真正需要文本输入、系统选择器、复杂文本编辑或成熟 accessibility/IME 语义的地方继续使用 native HWND；D2D 层只负责确有视觉收益的 Navigation、SettingRow、Toggle、SegmentedControl、Slider、ThemeCard、InputMethodCard、Banner、Preview 等有限组件，**不是再造一个通用 GUI framework**。
- 所有自绘交互组件 MUST 有完整 keyboard/focus 语义和 UI Automation 暴露：Name/Role/State，以及适用的 Invoke、Toggle、Selection、RangeValue 等 pattern。若某控件无法在 Narrator/NVDA/keyboard-only 下达到 native control 等价可用性，则优先退回 native HWND，而不是牺牲无障碍换外观。
- 自绘组件的 hit target、focus ring、disabled/hover/pressed state、DPI pixel snapping 必须由统一 Design Tokens/组件实现拥有；不得每页复制一套鼠标命中与 focus 绘制。
- Config 自身必须支持 `System / Light / Dark / High Contrast`，不能出现“候选框能跟随深色，但设置器本身永远浅色”的状态。
- 不做巨大顶栏、24px 以上夸张圆角、毛玻璃、卡片套卡片和无意义动画；目标参考 Windows 原生工具的清晰层级，而不是网页 dashboard。

**基础 Design Tokens（初始值，可经视觉回归 ADR 微调）：**

```text
spacing:        4 / 8 / 12 / 16 / 24 DIP
page padding:   24 DIP
control height: 32–36 DIP
corner radius:  6–8 DIP
icon:           16 / 20 / 24 DIP
page title:     24 DIP semibold
section title:  16 DIP semibold
setting title:  14 DIP medium
description:    12 DIP regular
body:           14 DIP regular
```

字体优先使用当前 Windows UI 字体策略（Modern 优先 Segoe UI Variable，fallback Segoe UI/System），通过 DirectWrite fallback 支持 CJK；不得在不同页面随意发明字号、间距和字重。

Design Tokens、icon metrics、component state colors 与 typography 必须只有一个 typed source of truth；C++ renderer/WTL host/测试读取同一模型。视觉回归采用少量 canonical screenshots + 结构断言（尺寸、层级、focus/contrast/a11y），**不得把所有 UI 变更做成脆弱的全屏 pixel-perfect gate**。

**设置行模式：**

```text
候选布局
根据当前输入内容选择候选排列方式                 [ 自动 | 横排 | 竖排 ]

候选字号
控制候选主文字大小                              ─────●──── 16

阴影
增强候选框与背景之间的层次                      [ toggle ]
```

每一项必须具有 `title + optional description + control + accessible name/state`，而不是裸 `Label: [Control]`。

### 5.5.9 Config 信息架构与渐进披露

用户看到的一级导航按任务组织，推荐基线：

```text
输入法
外观
快捷键
插件与扩展

────────
更新
诊断与修复
```

- `Theme` 不再与 `Appearance` 作为两个平级一级页面；主题、字体、布局、候选视觉统一属于“外观”。
- `Diagnostics` 与 `Repair` 合并成“诊断与修复”，内部再分状态、日志、Health Check、技术详情。
- 若“常规”只包含少量杂项，不创建垃圾桶式 General 页面；把设置放进其真正 owner 页面。
- Advanced 必须允许高级用户进入通用 Fcitx addon/input-method 配置视图，优先根据 Fcitx metadata/config schema 动态生成，不维护巨大的“用户意图 → addon ID”硬编码映射表。
- 普通用户看到友好名称；addon ID、ABI、路径、profile GUID、IPC 等只在 Advanced/Diagnostics 的明确技术视图出现。

### 5.5.10 Appearance 页面与 Live Preview

Appearance 是最能体现产品完成度的页面，第一屏 SHOULD 包含：

```text
候选框实时预览
[ production renderer preview ]

主题
[ System ] [ Light ] [ Dark ] [ 已安装主题 … ]

候选布局
[ 自动 ] [ 横排 ] [ 竖排 ]

文字大小
────────●──────── 16

字体
[ 跟随系统 / 选择字体 ]

高级外观 >
```

- Preview 必须复用 production Candidate renderer/layout/theme model；不得维护第二套“看起来差不多”的模拟 renderer。
- 当前 `fcitx5-ui.exe --demo` 可作为阶段性实现；最终 SHOULD 支持嵌入/托管 production renderer 或通过明确 PreviewHost 协议实现 inline live preview。
- 字体、主题、布局、圆角、阴影等可逆外观变化默认 `Live`；不得强迫用户反复 `Apply → Preview → 修改 → Apply`。
- `max width`、`scroll cell width`、padding、row/column gap 等 renderer 工程参数不得默认暴露。普通用户只看到主题、字号、字体、布局等高价值选项；高级参数折叠到“高级外观”，完整能力仍可由 `theme.toml` 提供。
- ThemeCard/输入法卡片/Toggle/SegmentedControl 等组件必须复用 Design System，不允许每页自行 owner-draw 一套近似控件。

### 5.5.11 Candidate UI/UX：高频、低干扰、稳定优先

Candidate UI 的目标不是“第一眼惊艳”，而是**长时间输入时几乎感觉不到 UI 在打扰**。

默认视觉：

- 轻量边框、轻阴影、约 6–8 DIP 圆角；
- 主文字 > selected state > label/index > annotation/comment；
- selected candidate 使用背景块 + 对比/字重等至少两个通道，不使用夸张高亮；
- 不添加无意义的心形、更多菜单、动画徽章等 chrome；
- 鼠标 hover 不改变 keyboard-selected candidate；点击才改变选择/提交。

新增布局模式：

```text
orientation = auto | horizontal | vertical
```

`auto` 为默认。解析策略由 presentation 层依据 active Fcitx input method、candidate/comment 内容、writing mode 与可用 work area 决定；用户显式 horizontal/vertical override 始终优先。Auto 不得随机抖动，同一 composition 内布局切换必须有稳定规则。

**候选窗口尺寸稳定性：**

- 一个 composition 内采用 width hysteresis：窗口可以立即增长，但 SHOULD 不因短候选立即缩小；composition 结束后再 reset，或使用经测试的 decay policy。
- 上/下定位在同一 composition 内 SHOULD 尽量稳定；只有 work area 不允许时才翻转。
- 普通 candidate 更新不得重新解析主题、字体资产或执行磁盘 metadata query。
- Scroll layout 现有稳定 viewport/column alignment 等行为属于 Keep 项；label column alignment、DWrite ellipsis、DPI/work-area clamp 不得因 UI 重构退化。


## 5.6 外观、主题、字体与视觉资源

外观能力应给普通用户“零配置可用”，给高级用户“少量明确控制权”。外观错误永远不能阻止输入主链。

主题是候选框完整视觉样式的 owner，**可以定义字体、字号、fallback、颜色、横/竖排、序号外观、间距、边框、圆角、阴影和资源**。限制的是主题不能改变输入语义，而不是限制主题的视觉能力。用户在 `config.toml` 中的显式视觉 override 可以覆盖主题。

### 5.6.1 系统主题与优先级

默认提供且只需要提供：

```text
Theme preference:
  System   # DEFAULT
  Light
  Dark
```

- `System` MUST 实时跟随 Windows 当前 Light/Dark 偏好。
- 当前不实现按时间、日落、地理位置自动切换；Windows 已经是系统偏好的 source of truth。
- High Contrast / Contrast Theme MUST 高于普通 Light/Dark 和自定义主题。
- 用户自定义主题不得覆盖最低可读性、DPI、无障碍和输入可用性要求。

统一优先级：

```text
Accessibility requirement
        >
Explicit user override
        >
Theme/package recommendation
        >
Application default
        >
Windows/system fallback
```

### 5.6.2 字体策略与作用域

只定义少量字体 surface：

```text
UI
Candidate
Annotation
Monospace
```

- `UI`：WTL 配置器、管理页、状态 UI。
- `Candidate`：候选主文字。
- `Annotation`：候选注释、编码、辅助信息。
- `Monospace`：日志、调试、技术字段。
- Host application 中的 TSF composition/preedit 字体由宿主拥有，Fcitx 不强行覆盖 Word/Chrome/Terminal 的文本字体。

字体选择是“偏好”而不是强制：

```text
preferred font
  ↓ missing font / glyph
user fallback 1
  ↓
user fallback 2
  ↓
DirectWrite system font fallback
```

- Candidate renderer MUST 使用 DirectWrite 的字体 fallback 能力；不得自己维护语言→字体的巨大硬编码表。
- 字体未安装、字体文件损坏或缺 glyph 时必须自动 fallback，不能导致方框、空白、Candidate UI 失败或输入失效。
- 配置器可以显示“首选字体未安装 / 当前实际使用字体”，但不能阻止使用。
- 字号用逻辑尺寸并结合 DPI/系统文本缩放；禁止固定像素字号。
- Annotation/Index 等 SHOULD 使用相对比例而不是各自维护大量独立字号开关。
- 配置器 SHOULD 提供一次性字体预览，例如中文、繁體、日本語、한글、Latin、扩展汉字与 emoji，帮助用户在保存前发现缺字。

示例配置只表达策略；正式 schema 见 13.9：

```toml
[fonts.ui]
families = ["system"]

[fonts.candidate]
families = ["system"]

[fonts.annotation]
families = ["inherit"]

[fonts.monospace]
families = ["system"]
```

### 5.6.3 图标与资源格式

第一版格式策略：

| 格式 | 级别 | 用途 |
|---|---|---|
| ICO | MUST | EXE、安装器、卸载器、托盘、快捷方式、文件关联等 Windows icon semantics |
| PNG | MUST | 普通 UI、输入方案、插件、主题位图资源 |
| SVG | SHOULD | 配置器和主题矢量资源；不得成为 Win7 候选热路径的硬运行时依赖 |
| BMP | MAY | 只为现有 Windows/Fcitx 资源或明确必要场景 |
| JPEG | NO by default | 不作为 UI 图标格式 |
| GIF/APNG/animated SVG | NO | 当前不支持可执行/动态主题资源 |
| WebP | NO by default | 当前无明确收益，不增加 decoder 依赖 |

Windows 程序 ICO SHOULD 是 multi-resolution icon，覆盖常用系统尺寸；不要求为每个尺寸维护独立文件。

### 5.6.4 Symbolic Icon 与 Brand Icon

区分两类图标：

```text
SymbolicIcon
→ 由系统/主题提供 foreground color
→ 自动适应 Light/Dark/High Contrast

BrandIcon
→ 保持输入方案/插件品牌颜色
→ 仍必须保证背景可辨识
```

设置、删除、搜索、警告、重启等通用操作优先 symbolic icon；输入方案/logo 可使用 brand icon。

状态不能只靠 icon 表达。至少保证 `icon + text` 或等价 accessible name，例如“Rime — 已启用”。


### 5.6.4A 产品图标、任务栏图标、TSF 图标与默认名称

图标分成**产品图标**与**TSF profile 图标**，不得为了展示不同 engine 在 Windows 语言栏制造多个品牌/语言图标。

#### A. Product / Taskbar / Start Menu：Penguin-first

`fcitx5-config.exe`、开始菜单入口、任务栏、安装器品牌页使用统一的 **Fcitx5 for Windows Next Product Icon**。默认品牌方向冻结为 **Penguin-first / 小企鹅优先**：使用简化、原创、几何化的小企鹅 silhouette 作为核心识别，不再以抽象 `F5`、键盘、Windows 窗格或语言字符作为默认主标识。

若采用/改编上游 Fcitx 已有企鹅资产，必须先确认许可证与品牌使用条件；否则设计原创企鹅，不直接复制 Tux 或第三方吉祥物。

视觉要求：

- 大尺寸 Product Icon 可以使用完整/半身企鹅；小尺寸必须抽象成强轮廓，保留最少但稳定的企鹅识别特征。
- 风格“友好但不过度卡通”：避免大眼萌系、表情包、3D 高光、复杂渐变和细碎装饰。
- 主体优先黑/白 + 一个受品牌 token 管理的点缀色；最终颜色以 Light/Dark/High Contrast 实机评审为准。
- 轮廓在 16/20/24/32/48/64/128/256 px 均检查；16/20/24 px 允许单独手工简化/像素对齐，不要求把 256px master 机械缩放。
- 不在任务栏图标上叠 `中`、`A`、`拼`、国旗等模式状态；模式状态属于输入 UI，不污染 product brand。
- Config/开始菜单 shortcut 使用稳定 AppUserModelID（建议产品级命名如 `Fcitx5.Windows.Settings`，最终值由资源/安装规范唯一拥有），跨版本不得无故变化，避免任务栏 pin / notification identity 分裂。
- `fcitx5-config.exe` 是主要普通桌面可见应用，正常进入任务栏/Alt+Tab。
- `fcitx5-ui.exe` Candidate window 必须 no-activate / no-taskbar / no-Alt+Tab。
- launcher、engine、downloader、provider、deployer、updater 以及未来 Rust R1/R2 后台组件默认不得创建任务栏/Alt+Tab surface、空白 console 或默认 EXE 图标窗口；文件属性/Task Manager 可以嵌入同一 product-family icon。
- 安装器、卸载器、Repair 若有可见窗口，使用同一 Product Icon；不得出现 Windows 默认空白 EXE 图标或每个 helper 自己一套 logo。
- **默认不新增常驻 tray icon。** Windows 已有输入法/语言栏入口；只有真实用户研究证明存在高频 tray 快捷需求时再增加，且 tray 只作为可选 shell surface，不承载不可替代状态、不闪烁催更新。

#### B. 单一 TSF Profile：Micro Penguin

Windows 语言栏/输入法切换器只显示一个固定的 **Fcitx5 TSF Icon**。默认仍使用企鹅视觉，但不是把完整 Product Icon 硬缩到 16px，而是使用同一 silhouette family 的 **micro penguin glyph**。

- 优先为 16/20/24 px 单独设计/检查，宁可减少细节，也不要糊成黑白噪点。
- **必须语言中立**：禁止把 `中`、`拼`、`日`、`한`、国旗或 Rime/Mozc logo 作为固定 TSF 图标。
- TSF icon 不随当前 Fcitx input method 动态换图，避免 Windows shell 缓存、profile identity 与内部状态耦合。
- 不在 TSF icon 内写 `F`、`5`、`Fcitx` 等小字；文字 identity 由 Windows picker 的 `Fcitx5` 提供。
- Light/Dark/High Contrast 下必须可辨认；High Contrast 可以退化到同轮廓的单色版本。
- 资源损坏时 fallback 到内置最小 Fcitx glyph，不影响 TIP 注册/激活。

#### C. 默认用户可见名称

```text
Windows 输入法选择器 / TSF profile:  Fcitx5
开始菜单 / Settings 应用:           Fcitx5 for Windows Next
Config 窗口标题:                    Fcitx5 for Windows Next
候选/内部状态:                      需要时显示当前 Fcitx input method 名称，但不改变 Windows profile 名称
```

禁止 Windows profile 默认叫 `Fcitx5 Pinyin`、`Fcitx5 Rime`、`Fcitx5 Chinese`、`Fcitx5 Mozc`。产品名称、FileDescription、shortcut label、AppUserModelID 与 i18n string 必须由统一 resource manifest 拥有，不在多个 EXE 中各自硬编码。


### 5.6.5 资源 fallback 与安全

统一资源 fallback：

```text
explicit user asset
      ↓ invalid/missing
current theme/package asset
      ↓
built-in Fcitx asset
      ↓
system/default fallback
```

- 图标、字体、主题资源损坏 MUST NOT 导致候选窗或设置器无法启动。
- Theme/package 内资源只接受包内相对路径或 `builtin:*` 标识；禁止 `..` 逃逸、绝对路径、UNC、远程 URL。
- 网络资源不允许由 Candidate renderer 直接获取。
- 图片 decoder 输入必须有 encoded size、decoded dimension、pixel count 和内存上限，防止压缩炸弹/恶意尺寸。
- 初始预算建议：单 asset encoded ≤ 8 MiB、单边尺寸 ≤ 4096 px、decoded pixels ≤ 16 MP；后续依据真实主题需求和 benchmark 调整。
- SVG 解析仅允许数据格式能力，不允许脚本、外部网络资源或可执行扩展。

### 5.6.6 DPI、缓存与运行时成本

所有视觉尺寸以 DIP 表达。例如 16 DIP 图标：

```text
100% → 16 px
125% → 20 px
150% → 24 px
200% → 32 px
```

- Asset cache key 至少包含 `(asset_id, logical_size, dpi, appearance)`。
- DPI、主题或资源版本变化时才 invalidate；每次候选更新不得重新 decode 图标或重新解析 SVG。
- Win7 若缺少现代 SVG/Composition 能力，可在加载阶段 rasterize/cached bitmap；这属于当前平台实现差异，不保留项目旧版本 fallback。
- 多显示器跨 DPI 移动时，候选 UI 必须保持物理可读尺寸和位置连续性，不能突然模糊或倍增。

### 5.6.7 允许暴露的外观参数

第一版只允许少量高价值设置：

- System / Light / Dark；
- UI / Candidate / Annotation / Monospace 字体及有限 fallback chain；
- Candidate 基础字号或整体 scale；
- 主题选择；
- 必要的候选布局、间距、圆角、阴影、透明度等主题级参数；
- 图标大小随主题/scale 统一控制。

不为每个按钮、页面、插件单独提供字体/颜色/字号开关；不增加定时主题切换、复杂条件规则、任意脚本或动态网络主题。

- 6. Fcitx5 Addon 与跨平台策略

- 7. 包管理、方案、Plum 与更新

- 8. 安装版、Portable 与系统包管理器

- 9\. i18n 与多语言输入法 Profile

- 10\. 隐私与安全架构

## 5.7 候选框定制：布局、字体、颜色与序号

候选框必须允许高级用户定制，但“显示样式”和“输入语义”严格分离。Renderer 可以改变排列、字体、颜色、序号外观、间距和装饰；真正哪个键选择哪个候选、候选顺序、commit 语义仍由 Fcitx/输入方案决定，主题和外观配置不得修改。

### 5.7.1 第一版正式支持的候选布局

- `vertical`：纵向逐候选排列；默认。
- `horizontal`：横向连续排列；空间不足时按当前页面整体布局，不偷偷改变候选顺序。
- 一个 composition 周期内 SHOULD 保持 orientation 和候选窗口锚点稳定。
- 当前不实现瀑布流、任意网格、主题脚本计算布局；出现明确需求后再扩。

用户可配置或由主题提供默认值的几何项：

```text
orientation
max_width_dip
padding_x_dip / padding_y_dip
item_padding_x_dip / item_padding_y_dip
row_gap_dip / column_gap_dip
border_width_dip
corner_radius_dip
shadow
opacity
```

所有值都有合理范围；0/负数/极大值不得制造不可见、超屏或超大分配。窗口最终仍受 work-area clamp、DPI 和 accessibility 约束。

### 5.7.2 排版

候选框至少区分四种文本角色：

```text
preedit
candidate.label
candidate.text
candidate.comment
```

- `candidate.text` 使用 Candidate 字体策略；`comment` 默认继承 Annotation；label 默认继承 Candidate 再乘相对 scale。
- 用户可以为 Candidate / Annotation 指定有序字体族列表；缺字继续走 DirectWrite system fallback。
- 支持字号、weight、相对 scale，但不允许每一个候选项单独设置字体。
- 行高、baseline 与 CJK/Latin/emoji 混排必须由 DirectWrite 测量结果驱动，不用固定字符宽度猜测。

### 5.7.3 颜色与状态

只暴露语义色，不暴露“第几个矩形”的任意 paint 参数。第一版语义色至少包括：

```text
background
border
preedit_text
label_text
candidate_text
comment_text
selected_background
selected_label_text
selected_candidate_text
selected_comment_text
shadow
```

颜色格式统一为 `#RRGGBB` 或 `#RRGGBBAA`。High Contrast/Contrast Theme 有权覆盖全部普通颜色。选中态不得只靠微小色差表达。

### 5.7.4 序号 / Label 显示

CandidateModel 中的 `label` 是 engine/Fcitx 给出的选择标签，是唯一语义来源。Renderer 不得自行改变“按 1 选择第一候选”之类的输入语义。

允许用户只修改 label 的视觉呈现：

```text
visible = true | false
style = plain | dot | paren | bracket | circled
font_scale = 0.85
gap_dip = 4
```

例如 engine label 为 `1` 时可显示为 `1`、`1.`、`(1)`、`[1]`、`①`；底层选择键仍然是 engine 定义的 `1`。非数字 label 使用不适用的 presentation style 时保持原 label 的可读表示，不改变语义。

### 5.7.5 每页候选数与输入语义的 owner

`page_size`、候选排序、快捷键、翻页键等属于 Fcitx/输入方案配置，不属于 `theme.toml`。配置器可以把这些设置暴露给用户，但必须通过 Fcitx Config API 修改 authoritative engine configuration；不得复制一份到 Windows appearance config。

这避免：

```text
theme.toml page_size = 7
Fcitx page_size = 5
→ UI 与 engine 对候选页理解分裂
```

### 5.7.6 主题默认值与用户覆盖

最终视觉值按以下顺序求值：

```text
Windows accessibility override
        >
user config.toml sparse override
        >
selected theme.toml (light/dark branch)
        >
built-in defaults
        >
DirectWrite/Windows fallback
```

用户配置只保存真正覆盖的字段；没有覆盖的字段继续继承主题。Reset 删除对应 override，而不是把当前主题值复制进用户配置。

# 6. Fcitx5 Addon 与跨平台策略

## 6.1 平台边界

Fcitx5 Core
│
┌──────────────┼──────────────┐
│ │ │
libime Rime Lua / Mozc / ...
│
inputmethod-data
──────────────── Platform boundary ────────────────
Linux macOS Windows
X11/Wayland macOS frontend TSF frontend
Linux UI macOS UI Windows D2D UI
config tools macOS config Windows WTL config

- 尽量复用 Input Method / Module addon；Frontend / UI / OS integration 平台化。

- Windows 不另造专用 plugin API；仅为平台必要能力提供 adapter。

- 所有 native addon 只加载进 engine，不进入 TSF DLL。

- 第一阶段只保证少量核心 addon；兼容性确认后再扩大插件集合。

## 6.2 Addon 信任等级与状态

| **类别**                      | **风险**               | **默认策略**                         |
|-------------------------------|------------------------|--------------------------------------|
| Official                      | 已由项目 CI 构建/验证  | 允许安装；仍做 ABI/签名/hash 检查    |
| Verified Community            | 来源可追踪、构建可验证 | 安装前显示来源/权限风险              |
| Local / Unverified native DLL | 任意代码、可读取输入   | 默认阻止或强警告；不进入自动仓库     |
| 数据包 / 词库 / 主题          | 风险较低但仍可损坏/DoS | schema/大小/路径校验；不赋予执行能力 |

- 插件状态至少：installed / enabled / disabled / pending_update / pending_remove / broken / quarantined。

- 不要热卸载 native DLL。更新/删除通过 engine restart 完成。

- 包 manifest 必须含 core_api、addon_abi、architecture、min_os、dependencies、license、source_commit。

# 7. 包管理、方案、Plum 与更新

## 7.1 统一包类型

| **type**         | **示例**                               | **是否执行代码**             |
|------------------|----------------------------------------|------------------------------|
| core             | TSF/engine/UI/config/package service   | 是                           |
| addon            | fcitx5-rime、Lua、Mozc、chinese-addons | 是                           |
| inputmethod-data | Fcitx table、Rime schema、码表、词典   | 默认否；含脚本时升级风险等级 |
| theme            | 候选主题与资源                         | 否                           |
| translation      | UI 语言包                              | 否                           |

## 7.2 Package Manager 是独立核心，不是配置 GUI

```text
fcitx5-package.exe / package core (C++)
├─ resolve dependencies
├─ download
├─ verify signatures/hashes
├─ stage
├─ install/activate
├─ remove
├─ repair
└─ packages.lock

fcitx5-config.exe (C++ / WTL)
└─ only UI + user intent via typed Control API
```

Package core 处理网络、archive、manifest、路径与事务状态，属于高风险外部输入边界，因此必须比普通 GUI 接受更严格的 parser fuzz、路径规范化、签名/哈希验证、权限与 crash-consistency 测试。GUI 不拥有 resolver、signature、transaction 或 update policy。下载器与 updater 同样不得接触实时输入数据。

### 7.2.1 Package / Deployer 权限边界

默认不新增常驻高权限 service。下载、解析仓库、依赖求解、hash/signature 验证与 staging 在普通用户权限下完成；只有确实需要写 `%ProgramFiles%`、系统级注册或其他管理员资源时，才调用极小、可审计的 `fcitx5-deployer.exe` / `fcitx5-register.exe` 经 UAC 执行必要动作。

- **联网组件不得同时长期持有管理员权限。** `package/downloader` 负责取得并验证 immutable staged artifact；提权组件只消费已经验证、路径固定、内容 hash 已知的本地 artifact。
- deployer 不负责下载、仓库解析、任意 URL、主题解析或 addon 业务逻辑；只执行明确白名单的 install/activate/remove/register 操作。
- 提权前后都重新验证 artifact identity / hash；临时目录、用户可写目录和路径参数必须防止替换、junction/reparse-point/path traversal 等 TOCTOU/路径攻击。
- per-user 安装若不需要管理员权限，则整个事务保持 unelevated。Portable 默认不因“方便”请求管理员权限。
- 未来只有真实部署需求证明 UAC helper 不够时，才重新评估 Windows Service；不得预防性增加高权限 daemon.

## 7.3 更新事务

Resolve
↓
Download
↓
Verify signed manifest + SHA-256 + code signature
↓
Stage to new version directory
↓
Compatibility check
↓
Atomic activation
↓
Restart engine / re-register when needed
↓
Health check
├─ pass → Commit + mark previous-known-good
└─ fail → Discard staged build / keep current active

- 使用版本目录，避免原地覆盖仍被宿主进程加载的 DLL。

- `packages.lock` 记录当前实际安装组合，用于问题复现和一致性检查；**不承担旧组件兼容**。部署层可以另外记录当前 active 与至多一个 previous-known-good artifact identity，用于整套程序版本回退。

- previous-known-good 只用于处理“新版本已经通过安装 health check、但真实使用后发现严重回归”的 bad release。回滚必须整套切回同一 release lineage；**同一 generation 内**禁止旧 TSF + 新 engine、旧 schema parser + 新 config 等混跑。更新 drain 期间允许 N 与 N+1 两个完整 generation 并存，但它们必须由 generation-specific IPC 隔离。

- 程序版本与不可再生用户数据严格分离。派生缓存/索引格式变化时直接删除并重建；不可再生用户数据格式应尽量保持简单稳定。程序更新不得为了方便而破坏性重写旧用户配置；若 Stable 版本确需 breaking config change，显式转换必须保留原文件，使 deployment rollback 不依赖运行时双 schema。

- 更新 manifest 设计 key_id / key rotation / revoked keys，不能只有一个永久在线签名密钥。

### 7.3.1 TSF DLL 正被宿主加载时：Generation Side-by-Side / Drain

`fcitx5-tsf.dll` 会被 Word、Chrome、VS Code、游戏等宿主进程加载；正常升级不得假定可以原地覆盖文件，也不得注入/远程 `FreeLibrary`、强杀宿主或要求每次更新都注销 Windows。

**部署模型：**

```text
generation N
  old TSF image already loaded by existing hosts
  runtime/N/engine + ui + protocol N
           │
           └─ generation-specific IPC endpoint

generation N+1
  new fcitx5-tsf.dll at canonical registration path
  runtime/N+1/engine + ui + protocol N+1
           │
           └─ generation-specific IPC endpoint
```

更新事务必须遵守：

1. 验证/stage 完整 N+1 artifact；不得先碰当前 TSF。
2. 若 canonical `fcitx5-tsf.dll` 正在使用，使用同目录安全 rename/版本化留存旧文件，再把 N+1 DLL 原子放到 canonical path；已加载旧 image 的宿主继续运行，之后新启动/重新加载的宿主得到新 DLL。
3. **每个 TSF build 在编译/打包时绑定自己的 runtime generation/protocol identity。** N 只连接 N，N+1 只连接 N+1；pipe endpoint/handshake 中包含 generation/build identity，禁止误连。
4. Launcher 可以在 drain 窗口内同时监督 N 与 N+1 两个完整 runtime，但不得把 N TSF 接到 N+1 Engine，也不得让 N+1 Engine 解析 protocol N。
5. 旧 generation 无 client 后退出并清理；旧 DLL 能删则立即删，仍被占用则记录 pending cleanup，最终允许使用 Windows reboot-time delete 机制作为兜底。
6. previous-known-good 与 draining generation 是不同概念：前者用于 bad-release rollback，后者用于处理 in-use module 生命周期。正常状态最多保留 current + 一个 draining/previous generation，不允许无限堆积。
7. 更新 UI 不要求用户关闭全部应用；只有注册/文件系统异常导致无法安全激活时才提示重启/注销，并给出明确原因。
8. Restart Manager MAY 用于诊断/可选“立即完成更新”，但不得把自动关闭用户文档、浏览器或游戏作为默认升级路径。

**目录建议（实现可按现有 installer 结构调整，语义必须保持）：**

```text
Fcitx5/
  tsf/
    fcitx5-tsf.dll
    fcitx5-tsf.old.<generation>.dll
  runtime/
    <generation-N>/
    <generation-N+1>/
  deployment-state.json   # active / previous / draining identity
```

该设计是部署期 side-by-side，不是运行时旧协议兼容；因此不违反 0.1 的“删除旧协议/旧 parser”原则。

## 7.4 Rime 与东风破（Plum）

- Rime 是 addon；Rime schema 是 inputmethod-data；Plum 是 Rime provider，不是全局包管理器。

- GUI 可提供“方案管理”，普通用户不需要理解 Plum 命令行。

- 调用 Plum 时必须显式传入当前 Rime user directory；禁止自己猜目录。

- 任意 Git/URL 来源必须显示来源与信任提示；含 Lua/可执行脚本的方案提升风险等级。

# 8. 安装版、Portable 与系统包管理器

## 8.1 安装版

- Installer 推荐 Inno Setup；不让配置器或 package GUI 自己实现完整安装器。

- TSF 注册/注销/修复由专用 C++ fcitx5-register.exe 完成。

- 安装器负责文件、UAC、卸载项、版本目录；register 工具负责 COM/TSF profile。

- 卸载前禁用 profile、停止 engine、注销 TSF、安排已锁定旧 DLL 的后续清理。

## 8.2 Portable / Self-contained

| **语义　Windows 输入法不可能做到真正“零注册绿色版”。Portable 仅意味着程序与用户数据可放在同一目录、可注销后搬走；使用期间仍需注册 TSF。** |
|-------------------------------------------------------------------------------------------------------------------------------------------|

Fcitx5-Portable/
├─ bin/
├─ tsf/x86/
├─ tsf/x64/
├─ plugins/
├─ themes/
├─ locales/
├─ data/
└─ portable.flag

- portable.flag 只改变路径解析，不分叉业务代码。

- 检测 RegisteredPath != CurrentPath 时提示“修复输入法注册”，不要静默改系统状态。

- 同一用户默认只允许一个 Active Stable frontend；多个 portable 副本不能争同一 CLSID/Profile。

- Stable/Beta/Nightly 若允许并存，必须使用独立 CLSID、Profile GUID、IPC namespace。

## 8.3 Chocolatey / winget / Scoop

- Chocolatey/winget 只包装官方签名 installer，不重复实现 TSF 注册逻辑。

- 安装时记录 update_owner = builtin / chocolatey / winget / enterprise / manual。

- 若 update_owner=chocolatey，则 Core 不自更新；Config 显示由 Chocolatey 管理，用户执行 choco upgrade fcitx5。

- Addon、inputmethod-data、theme、translation 仍可由 Fcitx Package Manager 独立更新。

- Scoop 更适合作为后续 self-contained 分发渠道，但仍需明确 TSF 注册步骤。

# 9. i18n、单一 Windows TSF Profile 与内部多语言

## 9.1 配置器 i18n

locales/
├─ en-US.json
├─ zh-CN.json
├─ zh-TW.json
├─ ja-JP.json
├─ ko-KR.json
└─ ...

UI code: i18n("plugin.install")

- 翻译文件外置，翻译者不需要懂 C++/WTL。

- 切换语言可重启配置器，不要求所有控件实时热切换。

- 候选 renderer 必须支持 Unicode shaping、字体替补链、emoji、combining marks、CJK Extension；不能假定“只有中文”。

## 9.2 单一 Windows TSF Profile + Fcitx 内部多语言

Windows 侧固定注册一个 TSF profile，用户可见显示名为 **`Fcitx5`**，使用固定语言中立 TSF icon。Rime、拼音、Mozc、Hangul、Chewing、m17n 等只作为 Fcitx input method/group 在 Engine 内切换，**不为每个 engine 注册第二个 Windows TSF profile**。

数据模型仍必须保留：

```text
Windows profile identity  # 单一稳定 GUID/CLSID/LANGID registration
current Fcitx input method id
Fcitx group
BCP-47 / content locale
capabilities
presentation locale
```

- Windows profile identity 与 Fcitx input method identity 分离；前者稳定，后者可高频切换。
- 候选 DWrite locale、字体 fallback、标点/输入语义等需要语言信息时，从当前 Fcitx input method metadata / content locale 获取，不能因为 Windows profile 固定而永久硬编码 `zh-CN`。
- Windows 语言栏只表达“Fcitx5 已激活”，不承担展示当前内部 engine 的职责。
- 当前 engine 名称可在 Config、候选模式提示或 Fcitx 菜单显示，但不得通过动态重注册 TSF profile 实现。
- 这是明确的产品取舍：Windows shell 中保持一个干净、稳定的 Fcitx5 identity，牺牲“每个内部 engine 都作为独立 Windows language profile 出现”的系统级呈现。
- TSF language profile 在 Windows API 层仍必须绑定一个实际注册语言/LANGID；**“单一 profile”不等于 Windows 存在真正 language-neutral LANGID。** v1 首选继续沿用当前已验证的 registration language，除非 ADR + 真实安装/切换测试证明需要改变。该 LANGID 只表示 Windows shell registration identity，不得被 Engine/renderer误当成当前输入内容语言。

# 10. 隐私与安全架构

## 10.1 核心隐私不变量

| **Privacy Invariant　能看到实时输入数据的组件默认不能访问 Internet；能访问 Internet 的组件在接口上拿不到实时按键、preedit、candidate、commit 历史。** |
|-------------------------------------------------------------------------------------------------------------------------------------------------------|

| **组件**                   | **可见输入内容**        | **网络** | **说明**                |
|----------------------------|-------------------------|----------|-------------------------|
| fcitx5-tsf.dll             | 必要的当前 context 数据 | 禁止     | 不包含网络栈/更新逻辑   |
| fcitx5-engine.exe          | 是                      | 禁止     | Fcitx/addon 输入平面    |
| fcitx5-ui.exe              | 候选展示数据            | 禁止     | 只渲染 CandidateModel   |
| fcitx5-config.exe          | 默认否                  | 可按需   | 控制 API 不暴露实时输入 |
| updater/package downloader | 否                      | 允许     | 只处理包和元数据        |

## 10.2 敏感输入

- Password / PIN / secure/sensitive context：不学习、不预测、不记录、不写日志、不调用联网服务；必要时 ASCII passthrough。

- 用户词库视为敏感数据：放用户专属数据目录，使用正确 ACL；不自动放 Documents/OneDrive。

- Release 日志只能记录事件类型、长度、数量、latency、context id 等元信息，不能记录原文。

- 完整 crash dump 可能含输入内容：默认不自动上传；用户主动生成时明确提示。

## 10.3 现代 Windows 隔离

- Win8+ 可评估 AppContainer/受限 token 作为 engine/addon 的额外隔离层，但不得阻塞主链开发。

- Win7 使用较弱的 legacy hardening：禁止网络代码、受限权限、Firewall outbound deny（如部署策略允许）。

- 不要把“声明 network=false”当安全边界；真正边界应尽可能由 OS 权限执行。

## 10.4 供应链

- 依赖 pin 到 tag/commit/hash，记录 source、license、build flags、compiler。

- 生成 SPDX 或 CycloneDX SBOM。

- Release artifact、manifest、native addon 使用可验证签名；包内容使用 SHA-256。

- 使用安全 DLL 搜索路径；禁止依赖当前目录/PATH 随机解析 addon 依赖。

- 依赖漏洞用 SCA 扫描；高危漏洞进入 release gate。

## 10.5 Windows 专属加载与本地数据边界

Windows 发行版必须把“可执行代码目录”和“用户可写数据目录”分离，防止用户数据目录被意外提升为 native code 搜索路径。

- %ProgramFiles% 下 Core/官方 native addon 默认普通用户不可写；%LOCALAPPDATA% 保存配置、词库、cache、Rime user data。Portable 模式也要逻辑区分 program 与 data，即使物理上同一根目录。

- LoadLibraryEx 使用绝对路径或受控目录 + LOAD_LIBRARY_SEARCH\_\*；Win7 无相关 API/补丁时使用当前 Win7 支持路径，绝不把当前工作目录/PATH 当默认 addon 搜索策略。

- 用户词库、历史和个性化数据按敏感数据处理；默认本地、正确 ACL、可关闭学习/清空；必要数据可用 CurrentUser DPAPI，但不要为了加密破坏上游词库兼容。

- 11. 个人项目版 SSDLC

- 12. 测试与兼容矩阵

# 11. 个人项目版轻量 SSDLC / DevSecOps

| **范围控制　本项目采用轻量、风险驱动的 SSDLC / DevSecOps。目标是把安全、隐私、可靠性、性能、人因、可访问性、配置正确性和供应链质量持续接入开发与发布，而不是模拟大型组织流程。一个人可以自审、自批准、自发布，但关键证据必须自动化、可重复、可追踪。** |
|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|

本节借鉴 NIST SSDF 的“把安全实践嵌入现有 SDLC”思想，但不把本项目变成合规项目。要求只保留对桌面输入法真实攻击面、可靠性和维护成本有价值的活动。

## 11.1 一个统一的质量生命周期，而不是七套流程

每个需求/变更先判断它影响哪些质量属性，再执行对应检查。质量属性共享同一 issue、commit、CI 与 release evidence，不建立独立官僚流程。

| Requirement family | 主要问题 | 主要验证位置 |
|---|---|---|
| `SR-*` Security / Privacy | IPC、ACL、签名、插件、DLL loading、敏感输入、日志 | threat model、SAST/SCA、fuzz、security regression、release audit |
| `REL-*` Reliability | crash、restart、stale state、原子写入、失败语义 | unit/integration/E2E、fault injection、soak |
| `NFR-P*` Performance | 热路径 latency、CPU、内存、队列、启动、资源泄漏 | `tests/perf/`、trend、release baseline |
| `NFR-HF*` Human Factors | 心智负担、焦点、肌肉记忆、低打扰、恢复成本 | deterministic UI tests + short human checklist |
| `NFR-A11Y*` Accessibility | keyboard-only、High Contrast、UILess、screen reader semantics | automation + NVDA/Narrator smoke |
| `NFR-CFG*` Config / Theme | strict parse、inherit/reset、原子写、权限、资源安全 | unit/golden/invalid corpus/fuzz |
| `NFR-VIS*` Visual | 横/竖排、字体 fallback、DPI、layout、Light/Dark | layout golden + renderer smoke |
| `SC-*` Supply Chain | dependency、license、CI workflow、toolchain、SBOM、signing、release provenance | dependency/license review、workflow audit、package/release gate |

**规则：**不要为了有编号而复制同一要求。例如“engine 不联网”仍以 `SR-001` 为唯一权威要求，人因/DevSecOps 表只引用它，不再建立第二份等价规则。

## 11.2 生命周期最小活动

| 阶段 | 当前个人项目必须做 | 轻量产物 |
|---|---|---|
| 需求 | 写不可接受失败模式；给安全/性能/人因/配置等需求稳定 ID | 本规格 + `security/requirements.md` |
| 设计 | 新增/改变信任边界时做 STRIDE；重大 architecture/security decision 写短 ADR | `security/threat-model.md` + 必要 ADR |
| 实现 | smallest correct vertical slice；secure coding；依赖/许可证先查；Good/Bad Case | code + focused tests |
| PR CI | 受影响 build/test、SAST/SCA/secret/license/workflow checks；高风险变更追加对应 fuzz/integration | required checks |
| Main/Nightly | 更重的 fuzz、perf trend、Win7/host smoke、layout/a11y regression、OpenSSF-style supply-chain self-check | trend / nightly artifacts |
| Package | clean runner 构建 C++ Core/WTL Config 与真实 installer/portable/package metadata；不接触 production signing key | unsigned staging artifacts |
| Release | 从已验证 artifacts 签名/打包/最终 smoke；SBOM、hash、manifest、provenance/attestation、渠道发布 | signed immutable release set |
| 维护 | 漏洞入口、bad release、签名密钥/插件吊销、安全公告、依赖更新 | `SECURITY.md` + incident runbook |

## 11.3 当前明确不要求

- CMMI/IDEAL、CAB/change board、专职安全团队式审批流。
- 所有 PR 跑 Win7 + Office + 全游戏 + installer 的完整矩阵。
- 每个版本都做第三方渗透测试；公开 1.0/重大信任边界变更再按风险安排。
- 固定采购 IAST/DAST/SIEM 平台；本项目优先 SAST、SCA、Fuzz、E2E、权限/IPC/更新测试。
- 为了“合规”生成无人阅读的大量模板文档。
- 把主观 UX 判断全部做成 screenshot pixel-diff gate；只自动化可稳定判断的结构/语义事实。

## 11.4 安全需求基线（必须进入验收）

| ID | Requirement |
|---|---|
| SR-001 | TSF DLL MUST NOT initiate network communication. |
| SR-002 | Production logs MUST NOT contain raw keystrokes, preedit, candidate text, commit text or clipboard content. |
| SR-003 | Engine failure MUST NOT crash or indefinitely block the host application. |
| SR-004 | Malformed or version-mismatched IPC MUST NOT crash TSF or Engine. |
| SR-005 | Untrusted native addon MUST NOT be silently installed or auto-enabled. |
| SR-006 | User data MUST remain independent from program replacement/reinstall. |
| SR-007 | Package activation MUST be verified, staged and atomic; failed activation MUST leave the previous active install untouched. |
| SR-008 | Config/theme corruption MUST fall back safely instead of causing a crash loop. |
| SR-009 | Sensitive input contexts MUST disable learning/logging and external-data features. |
| SR-010 | Win7 support MUST be continuously verified by import checks + smoke tests if advertised. |
| SR-011 | Game / anti-cheat compatibility MUST NOT use process injection, game-memory access, graphics/input API hooking, kernel drivers, process hiding, obfuscation, or anti-cheat bypass techniques. |
| SR-012 | If a game or anti-cheat blocks the IME, the product MUST fail safely by degrading or disabling affected features; it MUST NOT attempt to evade or circumvent the protection mechanism. |
| SR-013 | TSF IPC operations MUST have bounded deadlines; timeout or engine unavailability MUST fail open instead of blocking the host input thread. |
| SR-014 | TSF MUST authenticate/attest the local engine endpoint using per-user/session namespace and ACL; stable signed releases SHOULD also verify expected server path and Authenticode trust. |
| SR-015 | TSF running as SYSTEM/LogonUI/secure-desktop context MUST NOT spawn or bind to a normal user engine/data directory. |
| SR-016 | Candidate UI failure MUST NOT block input processing; UILess candidate semantics MUST remain available where the host requests them. |
| SR-017 | Core/native DLL resolution MUST NOT fall back to untrusted current-directory or arbitrary PATH locations. |
| SR-018 | Untrusted PR/fork workflows MUST NOT receive production signing credentials, release tokens or equivalent high-value secrets. |
| SR-019 | Stable release artifacts MUST be traceable to one source commit, pinned dependency/toolchain set and one verified build lineage; release must not silently rebuild different binaries after approval. |
| SR-020 | Third-party CI actions/tools used in privileged release/security jobs MUST be pinned to immutable versions/hashes where the ecosystem supports it; workflow token permissions MUST be least-privilege. |

## 11.5 数据分类与隐私检查

只定义足够指导接口/日志/诊断的四类，不建设复杂 DLP 系统：

```text
Class S0 Secret/Input
  raw key, preedit, commit, candidate content, clipboard/current selection

Class S1 Sensitive User Data
  user dictionary, history, personalization, private config content

Class S2 Operational
  version, component state, context/revision id, latency, error category, crash metadata

Class S3 Public
  package metadata, public build version, license, source commit
```

- 新 IPC/API/log field 在 review 时必须能说清属于哪类。
- `S0` 默认不得落盘、不得进入网络组件、不得进入 production log/telemetry。
- `S1` 必须有明确 owner、ACL、retention/delete 语义；默认不进入云同步目录。
- crash dump 视为可能包含 `S0/S1`；默认不自动上传，用户主动导出时明确提示。
- 诊断包 SHOULD 先生成最小 operational report，再由用户显式选择是否附加更敏感材料。

## 11.6 轻量 STRIDE 做法

- 只在新增/修改信任边界、权限、网络能力、插件执行边界、签名/更新边界时更新 threat model；小字体/颜色改动不重做 STRIDE。
- 每个边界回答：Spoofing / Tampering / Repudiation / Information Disclosure / DoS / Elevation。
- 每个发现只要求：风险说明、当前缓解、对应测试/验证。
- 如果风险不能通过代码解决，必须变成明确产品边界/用户提示，不用模糊“最佳努力”掩盖。

## 11.7 CI / DevSecOps 最低工具组合

| 类别 | 最低实现 | Gate |
|---|---|---|
| Compile hardening | MSVC warnings、`/sdl`、CFG/DEP/ASLR/CET（可用时） | Core warning/build failure 阻断 |
| SAST | MSVC `/analyze` + clang-tidy；可选 CodeQL | 新 high/critical 或明确安全边界缺陷不得进入 stable |
| SCA | Dependabot/OSV/GitHub dependency review 等 | 已知高危且当前可利用的依赖需修复或有明确例外 |
| Secret scan | GitHub secret scanning 或等价本地扫描 | secret/token/key 命中阻断 |
| License | C++ dependencies + WTL/ATL + copied-source/asset license inventory | 未知/冲突 license 不得进入默认发行 |
| Fuzz | IPC decoder、manifest、TOML adapter/validation、package/archive path | crash/OOB/无限分配/路径逃逸阻断 |
| SBOM | SPDX 或 CycloneDX | 每个 Stable Release 附带 |
| Workflow security | Action/tool pin、least permissions、fork/PR secret isolation | release/security workflow 违规阻断 |
| Signing | Authenticode / package signature / SHA-256 | 未签/验签失败对象不得作为 stable 激活 |
| Host/DLL hardening | PE import + DLL path audit、Verifier/PageHeap/ASan（适用） | UAF/heap corruption/不可信 DLL path 阻断 stable |
| Supply-chain posture | 可选 OpenSSF Scorecard/等价自检 | 初期 informational；真实高风险 finding 转 issue/gate |
| Artifact provenance | source commit + lock/tool versions + final artifact hashes；平台可用时附 attestation | Stable 必须可追溯 |

OpenSSF Scorecard 可以帮助发现 workflow token 权限、依赖更新、危险工作流等供应链问题，但它是**检查器，不是架构 authority**；低分项必须结合本项目真实风险判断，禁止为了追分引入无用流程。

## 11.8 Risk-based Gate：PR 不跑全世界

先根据变更内容自动/人工归类：

| 风险 | 典型变更 | 必跑 |
|---|---|---|
| Low | docs、纯测试数据、无边界纯函数 | affected build/test + basic secret/license |
| Medium | Candidate UI、config/theme、WTL GUI、普通 addon/data | affected unit/integration；C++ compile/analyze checks；config/theme invalid corpus；layout/a11y/cognitive checks（相关时） |
| High | TSF/COM、IPC schema/ACL、launcher、native addon loader、archive/package、updater、installer privilege、signing、network、sensitive logging | x86/x64 build + focused integration/fuzz + threat-model delta + relevant security regression；合入 main 后扩大 nightly |
| Release-critical | 发布 workflow、签名、package manifest、release identity、update key | protected release path + full release gate |

**禁止两个极端：**一个 typo 跑 3 小时矩阵；一个 IPC/签名边界改动只跑 unit test。

## 11.9 CI 分层

### PR Gate（快、确定、无生产秘密）

- x86/x64 Core affected build；项目自有代码 warnings-as-errors。
- affected unit/integration/regression。
- SAST/SCA/dependency diff/secret/license checks。
- parser/schema 受影响时跑 invalid corpus + targeted fuzz smoke。
- config/theme/UI 受影响时跑 deterministic merge/layout/accessibility semantics tests。
- PR/fork job 不得获取 signing certificate、release token、update signing key。

### Main / Nightly（慢检查放这里）

- 更长 fuzz/soak、handle leak、perf trend。
- Phase 1–4：Win7 只按变更相关性执行 `min_os` / PE import / capability-detection 检查；Phase 5 起，Win7 VM 与代表性 host smoke 才进入 Main/Nightly 轮转。DPI、多屏、High Contrast、UILess/NVDA smoke 按对应能力出现阶段和可用环境轮转。
- 安装器/portable 的非签名 smoke。
- OpenSSF Scorecard/供应链 posture、dependency freshness 可定期运行，不阻塞普通开发除非转化为明确 blocker。

### Package Gate（真实交付物，但无 production signing）

- 从 clean checkout + pinned toolchain 构建 Core。
- 使用锁定的 MSVC/Windows SDK/CMake/WTL 与已获准的 Rust/Cargo 工具链无人值守构建真实交付 targets；**不允许依赖任何 IDE Publish 动作或手工 cargo release 步骤**。
- 生成真实 installer/portable/package metadata，验证文件布局、ACL、uninstall/repair/portable move。
- package job 必须能在干净 Windows runner 无人工点击完成；任何需要 IDE、桌面焦点、UI Automation 或开发机隐含状态的构建步骤都不得进入 release critical path。

### Stable Release Gate（权限最小、证据链完整）

推荐流水线：

```text
source commit + locked dependencies/tool versions
    ↓
one controlled build
    ↓
Core tests / package smoke / security checks
    ↓
sign payload binaries
    ↓
package installer / portable
    ↓
sign final signable containers
    ↓
release smoke against signed artifacts
    ↓
final SHA-256 + manifest + SBOM + provenance/attestation
    ↓
publish exact tested artifacts
```

- **禁止 release job 再重新编译一份源码。** 需要重编即产生新 build lineage，必须重新走对应验证。
- Signing job 使用独立 protected environment/runner；只拿签名需要的 secrets 和 artifacts，不拿开发无关权限。
- 最终 SHA-256/manifest 针对签名后的实际发布 bytes。
- 至少一条 smoke 必须安装/运行实际 signed installer/portable，而不是 build tree。
- 发布后立即能从 release metadata 找到 source commit、版本、SBOM、hash、签名信息。

## 11.10 GitHub Actions / CI 供应链规则

- `.github/workflows` 视为安全敏感代码；改变 `permissions`、secrets、release trigger、artifact flow 的 PR 按 High / Release-critical 处理。
- `GITHUB_TOKEN` 默认 read-only；每个 job 只开启实际需要 scope。
- 第三方 Action 在 privileged workflow SHOULD pin 到完整 immutable commit SHA，并由 Dependabot/人工更新；不使用漂移的 `@main` / `@master` / 未验证 `latest`。
- `pull_request_target`、workflow chaining、artifact-from-untrusted-PR 等高风险模式默认禁止，除非有当前不可替代需求和专门 threat model/test。
- 从网络下载的 build tool、WTL archive、code generator 等必须固定版本并验证来源/hash/signature；不能“下载最新版然后 release”。
- 缓存只能加速，不成为 source of truth。cache miss 必须能完整重建；cache hit 不能绕过 hash/lock 验证。

## 11.11 C++ / WTL + Rust 双工具链规则

正式发行链只允许可脚本化、可在 clean runner 无人值守执行的工具链。C++/WTL/Inno 继续作为 Windows Core 基线；v1.7 允许 Rust R1/R2 进入独立 management/security-domain 组件，但 **IDE 不是任何语言的构建依赖**。

- 所有 C++/WTL/Rust targets 都必须由顶层 `build.ps1` 驱动；开发者不需要手工打开 Visual Studio、WTL wizard、IDE designer 或执行单独“cargo release”仪式。
- C++ toolset、Windows SDK、CMake、WTL、MSYS2/Fcitx source pins 与第三方 dependencies 必须 pin；Rust 新增 `rust-toolchain.toml` 精确 channel/toolchain、提交 `Cargo.lock`，CI/release 使用 `cargo --locked`/等价 frozen resolution。
- Rust workspace **只在第一个真实 R1 target 的同一 PR 创建**，禁止提前搭空 workspace。crate 以实际进程/安全边界组织，不为共享几行 helper 制造 C++↔Rust FFI。
- TSF DLL 不链接 Rust staticlib/dylib，不引入 Rust panic/runtime/allocator 边界；Fcitx Engine 不为调用 Rust 重包 Fcitx C++ API。C++↔Rust 优先以既有 versioned IPC/wire contract 隔离。
- Rust protocol codec 独立实现同一 language-neutral wire spec，并与 C++ codec 共享 golden/invalid/fuzz corpus；不得把 C++ protocol 库经 FFI 拉进 Rust，也不得悄悄定义第二套协议语义。
- R1/R2 Rust 代码使用 `#![deny(unsafe_op_in_unsafe_fn)]`；`unsafe` 只允许在最小 Win32/FFI platform adapter，并要求邻近 safety comment/test。业务层 package/path/repository/state machine 默认 safe Rust；不追求不现实的“仓库 0 unsafe”数字。
- Candidate UI、launcher/package/updater 当前 C++ 代码在迁移前继续使用 RAII、有界容器、smart COM/HANDLE owner、显式长度和成熟 parser；**发现 C++ bug 不等于获得 rewrite 授权**。
- Config 只调用 typed Control API，不自行实现第二套 TOML schema、签名验证、package resolver、TSF registration semantics 或输入历史读取。
- clean runner → bootstrap pinned toolchains → build → tests → package 必须无人工点击、无 UI Automation 构建、无无限等待、无 retry-to-green。

### Cargo / Rust DevSecOps 必须与 C++ 同级

Rust 进入正式 artifact 前必须同时完成：

- `cargo-deny` 或等价机制覆盖 advisories / licenses / sources / bans；禁止未审查 git/path dependency、漂移 branch dependency 与未知许可证。
- resolved Cargo dependency graph 合并进 release SPDX SBOM 与 `THIRD_PARTY_NOTICES`；不能只保留手工 `third_party/dependencies.json` 而让 crates 成为盲区。
- secret/source policy scanner 覆盖 `*.rs`；runtime PE import/network-domain/min-OS 检查继续作用于 Rust 产物。
- 对 package/archive/path/parser 使用同一 hostile corpus 与 fuzz/property tests；Rust 通过编译器检查不替代路径、回滚、权限、签名等逻辑测试。
- MSYS2/Fcitx 与 Cargo 都进入 reproducibility inventory：实时 `pacman -Syu` 或 registry resolution 只能在被 lock/attest 的策略内发生，不能让第二套语言工具链降低原有可重现性。

### Legacy lane

Rust R1/R2 默认 Modern-first。只有某个具体 Rust target 通过独立 Win7 PoC（Tier-3 std/toolchain 构建、crate `min_os`、PE import、x86/x64 所需架构、Win7 VM smoke、CI 成本）并记录 ADR 后，才允许替换该组件的 Legacy C++ lineage。**不能因为 Rust 有 Win7 target triple 就把整个 Legacy 产品链绑到 Tier-3 target。**

## 11.12 许可证与来源治理

仓库至少维护：

```text
LICENSES/
THIRD_PARTY_NOTICES.md
third_party/manifest.*      # 或等价机器可读 inventory
```

- 自有文件使用 SPDX identifier；不同目录允许不同兼容许可证，但必须清晰。
- Fcitx5/libime/Rime/OpenCC、WTL/ATL、图片/字体/主题 asset、复制的代码片段都进入 inventory；R1/R2 引入的 Rust crates 必须从 `Cargo.lock`/resolved graph 自动进入 lock/SBOM/license inventory。
- 参考 GPL 项目（如 Weasel/Chewing）默认只借行为、设计、测试场景；复制非平凡源码前必须先确认目标项目许可策略和 attribution/源码义务。
- Release SBOM 与 `THIRD_PARTY_NOTICES` 应来自实际 build/package dependency set，不维护一份与构建脱节的手工名单。
- “许可证未知”视为 dependency 不可发布，而不是以后再补。

## 11.13 安全事件响应：个人项目也必须能处理坏版本

不建设 SOC，只保留四个最现实 playbook：

1. **Compromised signing/update key**：停止发布/更新 → 吊销/撤换 key → 更新 trusted-key set → 发布 advisory → 重新签发可信版本。
2. **Malicious/compromised addon**：仓库下架/quarantine → revoke package/version → Safe Mode/repair 可禁用 → 通知受影响用户。
3. **Bad release**：停止渠道分发 → 保留证据/commit/hash → 发布修复版；不得通过静默替换同版本二进制掩盖事故。
4. **Privacy/security bug**：最小化日志/证据 → 修复 + regression → 必要时 CVE/advisory；不得在公开 issue 粘贴用户输入数据。

`SECURITY.md` 至少写清 vulnerability reporting channel、supported versions、response expectations、是否接受加密报告。`security/incident-response.md` 保留上面四个短 runbook 即可。

## 11.14 证据阶梯与成对反例

个人项目的规则新增顺序固定为：**观察/报告 → 最小反例 → 最近 Good Case → 可复现测试 → 必要时机器门禁**。不能看到一次可疑实现就立刻造复杂静态分析器。

| 规则 | Bad Case：应拒绝 | Good Case：应允许 |
|---|---|---|
| 输入域禁网 | `engine.exe` 为预测主动访问公网 | `updater/package` 下载已签名包，且接口拿不到实时输入 |
| 依赖最小化 | 现有库已有能力却为了几十行逻辑新增大型 runtime | 当前依赖确实不能正确满足需求，且成熟库比自研更小、更安全 |
| TSF 极薄 | 把 TOML/Rime/Lua/WebView2/Config GUI/Rust GUI framework 拉进宿主 | Windows TSF/COM API + 项目极薄 C++ IPC client |
| 测试范围 | docs typo 强制跑完整 Win7+游戏+installer | IPC/signing/installer change 执行对应高风险矩阵 |
| 无 runtime migration | runtime 为 schema 变化保留旧 parser/migrator 或自动双栈迁移 | cache 直接重建；不可再生数据保持稳定，必要时提供显式离线 export/import/convert 且保留原文件 |
| 无 Hook/注入 | 游戏兼容添加 global hook / SendInput replay | 正式 TSF/UILess + regression test |
| 心智负担 | 为内部 timeout/IPC 参数增加普通用户 checkbox | 软件自动选择安全默认；Diagnostics 只展示状态 |
| CI secret | fork PR 可读取签名证书/token | PR 无 secrets；独立 protected signing job 只接收已验证 artifacts |
| Build once | Release 测完后再次从源码编译“发布版” | 同一 build artifacts 经签名/打包后做最终 release smoke |
| toolchain drift | MSVC/WTL 使用 latest 漂移、IDE 手工构建或未锁依赖后直接 release | pin toolchain/dependency + clean-runner unattended build + per-lineage Build Once |

**本项目最高原则覆盖通用工具默认值。** Scorecard/NIST/CI template 是参考，不因为某工具给分或建议就引入不符合本项目边界的 runtime migration framework、telemetry、复杂审批或额外 runtime。

# 12. 测试与兼容矩阵

## 12.1 测试分层

| **层**        | **内容**                                                                    |
|---------------|-----------------------------------------------------------------------------|
| Unit          | CandidateModel、状态机、generation、路径解析、包 resolver、config parse/reset |
| Parser/Fuzz   | IPC、manifest、theme、config、translation、package metadata                 |
| Integration   | TSF↔Engine handshake、engine restart、addon loading、package transaction    |
| E2E           | 真实应用中输入、候选、commit、切焦点、崩溃恢复、DPI、多屏                   |
| Release smoke | 安装、原子替换失败保护、卸载、Portable 修复、Chocolatey/winget wrapper            |

## 12.2 固定 Windows 应用矩阵

| **类别** | **最低验证对象**       | **重点**                                 |
|----------|------------------------|------------------------------------------|
| 基础     | Notepad                | 标准输入、composition、candidate、commit |
| Office   | Word + Excel           | 复杂 TSF、行尾、候选位置、焦点/鼠标      |
| Browser  | Edge/Chrome            | Chromium text controls、IME state        |
| Electron | VS Code                | 编辑器/内置终端                          |
| Terminal | Windows Terminal       | ConPTY、SSH/tmux 场景后续扩展            |
| Shell    | Explorer / Search      | 系统 UI、输入法切换                      |
| 架构     | 一个 32-bit app        | x86 TSF DLL                              |
| 权限     | 管理员运行应用         | 完整性级别/签名问题                      |
| 显示     | 100/125/150/200%、双屏 | DPI、跨屏、work area                     |
| 会话     | RDP 或多 Session       | SID/Session IPC 隔离                     |

## 12.3 Win7 Legacy compatibility lane

Win7 采用“**Day 1 不选死路，Phase 5 起持续验证 Legacy lane**”策略。它是明确支持的 legacy 产品能力，但不再与 Windows 10/11 承诺完全一致的新 UI/安全隔离能力。

- Phase 1–4 持续检查：C++ Core 不硬导入 Win10-only API；Rust/第三方依赖记录 `min_os`；平台差异优先 runtime capability detection；不得选择无法隔离且会让 Win7 基础主链必然失效的依赖。
- Phase 1–4 不要求每个普通变更都启动 Win7 VM；与 Win7 import、D2D/DWrite、WTL/ATL、TSF/installer 或未来新工具链相关的变更应提前跑专项检查。
- Phase 5 起，只要发布页面仍写“支持 Windows 7 SP1”，Win7 **按实际发行 OS/host 架构**执行 PE import check、VM smoke、TSF 注册/输入、Candidate UI、基础 Config 与代表性宿主场景。当前基线的 `x86 TSF` 首先表示 x64 Windows 上 32-bit host compatibility；32-bit Windows OS 不得由此自动推导为受支持。
- C++ Legacy lane 使用明确 pin 的 Win7-compatible MSVC toolset/SDK；Modern lane 可以升级最新受支持工具链，不让 Win7 冻结整个项目。
- Rust R1/R2 默认不替换 Legacy runtime。某个独立组件只有通过 Tier-3 Win7 target 的 std/toolchain、crate min_os、PE import、VM、CI 时间与维护成本 PoC 并记录 ADR 后，才允许替换其 Legacy C++ lineage；TSF/Engine/Candidate UI/WTL Config 不因该 PoC 自动获得迁移授权。
- 同一源码树禁止复制成 `win7/` 与 `modern/` 两套业务实现。平台差异优先 `GetProcAddress`/capability detection；必要的编译期 cfg 只停留在 platform adapter。
- Legacy lane 可以缺少 Win11 backdrop、现代 composition/动画、AppContainer 等增强能力，但 TSF 输入、候选可见性、基础主题/字体、配置、修复与安全更新必须符合发布声明。
- Win7 native addon 可以按 `min_os` 分级；一个 modern-only addon 不得拖累整个 Core。
- 未来若无法在可接受成本下维持安全可测的 Win7 lane，发布一个明确的最后支持版本并从下一主版本移除；不保留无人维护的半支持状态。

## 12.4 游戏与反作弊兼容性

定位　目标是让 Fcitx5 作为正常 Windows IME 与游戏/反作弊共存，降低误报、阻止和宿主崩溃概率。该当前支持能力不得包含任何反检测、绕过反作弊或模拟外挂行为。

### 核心规则

- 仅使用 Windows 正规 TSF 输入路径。不得为了兼容游戏增加全局低级键盘 Hook、DirectInput/Raw Input Hook、SendInput 回放主路径或游戏进程注入。

- 不得读取、扫描或写入游戏进程内存；不得对游戏使用 OpenProcess 的 VM_READ / VM_WRITE / VM_OPERATION 等能力；不得远程线程注入、DLL proxy/sideload、句柄劫持或内核驱动。

- 不得 Hook Direct3D、DXGI、OpenGL、Vulkan 等图形 API 绘制候选 UI。Candidate UI 只能由独立 fcitx5-ui.exe 以普通 Windows 窗口/TSF UI 机制呈现；必要时使用 UILess 或直接隐藏。

- 不得使用随机进程名、进程隐藏、UPX/自定义 packer、控制流混淆、自修改代码等“降低可见性”的手段。所有 release 二进制使用稳定文件名、版本资源、发行者信息和数字签名。

- 不得探测反作弊组件后改变行为以规避检测。Game Compatibility Mode 只能由正常应用兼容配置触发，用于关闭动画/复杂 UI/非必要插件等功能降级。

- 如果某游戏或反作弊阻止 IME DLL/UI，视为兼容性失败：记录最小诊断信息、允许用户禁用该应用中的 Fcitx5 或切换系统输入法，并向上游/厂商报告；禁止尝试绕过。

### 推荐的 Game Compatibility Mode

- 保持 TSF KeyEvent / Composition / EditSession 主路径不变，只关闭非必要能力。

- 可按用户配置的应用 profile 禁用自定义候选动画、复杂主题、第三方 addon 或候选窗，优先使用最简单的 TSF/UILess 行为。

- 无有效文本 InputContext 时完全 passthrough，不能吞 WASD、Space、Shift、Ctrl、数字键等游戏操作键。

- x86 老游戏必须通过 x86 TSF DLL 测试；全屏/分辨率切换时重点验证 candidate placement、焦点和 Composition 生命周期。

### 反作弊兼容测试

| **测试项**       | **验证**                                                                             | **失败标准**                                |
|------------------|--------------------------------------------------------------------------------------|---------------------------------------------|
| 静态行为审计     | 检查 imports / 代码路径：无游戏内存 API、远程注入、图形/输入 Hook、驱动、packer/混淆 | 发现任何非 TSF 必需的外挂式能力             |
| 代码签名/身份    | TSF、engine、UI、launcher 使用稳定名称和版本资源；公开发行使用有效数字签名           | unsigned/身份漂移进入 stable                |
| 老 x86 游戏      | 聊天/文本框能输入；非文本状态 WASD 等完全 passthrough；Alt+Enter/全屏切换后状态正常  | 误吃游戏键、候选窗抢焦点、宿主崩溃          |
| 受保护游戏 smoke | 在维护者合法可测试的代表性反作弊游戏中仅进行正常输入 smoke；观察是否被阻止/踢出/崩溃 | 发现 IME 主动注入、绕过保护或破坏游戏完整性 |
| 阻止后的行为     | 反作弊拒绝加载/显示时，IME 安全失效并可切换其他输入法；诊断不含用户输入内容          | 尝试绕过、无限重试、导致游戏崩溃或卡死      |

说明　不能承诺所有反作弊产品都会接受第三方 IME。Windows 正规 TSF 加载、数字签名和最小宿主 DLL 能显著降低可疑行为面；若保护产品仍阻止模块，应把它当作兼容性问题，而不是实现“反检测”。

## 12.5 Legacy / 老游戏 / IMM32-CUAS 兼容

正式实现仍以 TSF 为唯一 authoritative frontend。老 Win32/游戏兼容通过 Windows 的 TSF/IMM32/CUAS 行为和宿主实现来验证，不引入全局 Hook、DirectInput/RawInput Hook 或 SendInput 作为隐藏兼容路径。

| **场景**                   | **必须验证**                                                                        | **主要参考**                                     |
|----------------------------|-------------------------------------------------------------------------------------|--------------------------------------------------|
| 老 x86 Win32/IMM32 程序    | 验证 Windows 自身 CUAS/IMM32→TSF 系统行为是否能把文本输入正确桥接到 TSF；记录 WM_IME\_\* / HIMC 行为 | ImeStudy、ImeModePersistence、Microsoft DXUT IME |
| DirectDraw/D3D8/D3D9 全屏  | Alt+Enter、exclusive/windowed、分辨率切换、候选位置、焦点恢复                       | Microsoft DXUT IME + 实机 smoke                  |
| DirectInput/Raw Input 游戏 | 无文本 context 时 WASD/Space/Shift/数字键完全 passthrough                           | TSF context + Game Compatibility Mode            |
| 自绘聊天框/无 caret        | TS_E_NOLAYOUT/无效 rect、last-valid/安全位置策略，不无限查询目标进程                    | Rabbit/Weasel 作为测试用例来源                   |
| 高权限/反作弊              | 被阻止时安全失效；不注入、不读游戏内存、不图形 Hook、不规避检测                     | Anti-Cheat Friendly Build                        |

## 12.6 宿主端兼容性源码与回归

IME 的 bug 不一定只在 IME 一侧。对 Chrome/Edge、Firefox、Windows Terminal、Electron/VS Code 等问题，必须同时检查宿主的 TSF TextStore/IME handler 行为；兼容修复应沉淀成可重放的回调序列或 E2E。

| **宿主/类别**    | **审计重点**                                                                        |
|------------------|-------------------------------------------------------------------------------------|
| Chromium/Edge    | TSFTextStore、lock/composition/edit session、ACP range 行为                         |
| Firefox          | TSFTextStore / WinIMEHandler / TSFStaticSink，TSF 与 IMM 双路径历史兼容             |
| Windows Terminal | ConPTY/terminal TSF、composition/caret、节流与韩文/箭头键等真实回归                 |
| Word/Excel       | 真实 Office E2E；重点关注重复 OnTestKeyDown、行尾/焦点/同步 composition termination |
| Electron/VS Code | Chromium 基础上增加编辑器/terminal 组合场景                                         |

- 13. 仓库结构与编码规则

## 12.7 测试范围、真实产物与隔离环境

- **受影响测试优先。** 普通 CHANGE 先运行直接受影响的 unit/integration/regression；只有信任边界、TSF 生命周期、IPC、installer、package transaction、Win7 API 基线等高风险变化才扩展到对应矩阵。Stable Release 再执行完整 release gate。
- **测试实际交付物。** 至少一条 release smoke 必须针对真正生成的 installer/ZIP/签名 manifest 运行，而不是只测试源码目录或未打包 build tree。
- **保持测试环境隔离。** IPC、package、Portable、config、update 测试使用临时用户数据根目录/临时 pipe namespace/临时 install root，不读取开发者真实词库、历史或配置；能离线完成的测试默认断网。
- **fake boundary 优先。** TSF↔Engine 可使用确定性的 fake engine/pipe server 测协议、timeout、stale reply、crash；网络更新测试优先本地 fixture server。不要让核心回归依赖真实公网、真实云 API 或人工输入。
- **禁止 flaky-by-retry。** 测试失败不能通过增加 sleep、扩大 retry 次数或反复重跑“修绿”；先找竞态/时序根因。
- **停止规则适用于测试。** 已有足够证据覆盖当前变更后停止扩大矩阵；但不能用停止规则跳过本文明确列为 MUST 的 release/security gate。

## 12.8 人因、外观与资源回归

自动化能客观判断的部分进入回归：

- Candidate window 不 activation、不抢 focus、不进入 Alt+Tab；
- Light/Dark 系统切换后的 live update；
- High Contrast 覆盖普通主题；
- 字体缺失、缺 glyph、emoji/CJK fallback；
- 损坏 PNG/SVG/ICO、超尺寸资源、path traversal 被拒绝且不会影响输入主链；
- 多 DPI / 多显示器移动与 asset cache invalidation；
- keyboard-only 配置路径；
- CandidateModel 的 selected/count/text 与 UILess/accessibility 语义同源；
- renderer crash / theme corruption / missing font 时恢复内置默认外观；
- 隐藏 UI 无持续 repaint/busy loop。

需要人工观察的 UX 不造脆弱自动 gate；维护短 checklist 和可复现场景即可，例如候选窗是否明显抖动、视觉层级是否清楚、错误信息是否可行动、首次使用是否需要理解内部概念。

## 12.9 配置格式与候选定制回归

至少覆盖：

- `config.toml` / `theme.toml` 当前 `format_version` 正常解析；runtime 遇到版本不匹配明确拒绝且不自动运行 migration；
- unknown key、duplicate key、wrong type、NaN/Inf、不合法 enum、越界 DIP/opacity/font scale、超长字符串必须报精确错误；
- GUI 写配置前完整校验，临时文件写入失败不得破坏原配置；
- 手工写坏 TOML 时输入主链仍可用内置默认值，原坏文件不被“猜测修复”；
- vertical/horizontal、label plain/dot/paren/bracket/circled、字体 fallback、颜色与 Light/Dark 分支；
- user sparse override > theme，Reset 后恢复继承而不是固化旧主题值；
- High Contrast 覆盖用户/主题普通颜色；
- theme 不能改变 candidate order、commit、selection key、page semantics；
- theme asset path traversal、远程 URL、过大资源和非法 SVG 被拒绝；
- config/theme parser fuzz 针对项目自己的 validation/adapter 层，不重复 fuzz 第三方 parser 内部实现，除非出现真实 crash 证据。

# 13. 仓库结构与编码规则

## 13.1 推荐仓库布局

```text
/
├─ CMakeLists.txt
├─ CMakePresets.json
├─ cmake/
├─ src/
│  ├─ tsf/                 # Rust TSF target + host-matrix gated adapter/corpus
│  ├─ engine/              # thin C++ Fcitx adapter + Rust Engine Product Core
│  ├─ ui/                  # Candidate UI product logic migrates Rust; Win32/D2D adapter may remain during cutover
│  ├─ config/              # Config product logic migrates Rust; native Windows adapter may remain during cutover
│  ├─ launcher/            # Rust state/policy core + temporary Windows shell adapter
│  ├─ control/             # Rust command/schema/policy core + temporary Windows shell adapter
│  ├─ package/             # Rust package/repository/update authority with temporary adapters
│  ├─ updater/             # Rust migration source/baseline
│  ├─ common/
│  ├─ register/            # minimal C++ Windows helper
│  └─ bootstrap/           # minimal C++ Windows helper
├─ rust/                   # only created together with first real R1 target
│  ├─ Cargo.toml           # minimal workspace
│  ├─ Cargo.lock
│  ├─ rust-toolchain.toml  # or repository-root equivalent
│  └─ crates/
│     ├─ package-core/     # example R1; actual crates follow process/security boundaries
│     ├─ repository/
│     └─ updater/
├─ protocol/               # language-neutral current wire contract + golden corpus
├─ schemas/
├─ themes/
├─ locales/
├─ installer/inno/
├─ packaging/
│  ├─ chocolatey/
│  └─ winget/
├─ tests/
│  ├─ unit/
│  ├─ integration/
│  ├─ differential/       # C++ ↔ Rust migration corpus when R1 starts
│  ├─ fuzz/
│  ├─ perf/
│  └─ e2e/
├─ security/
│  ├─ requirements.md
│  ├─ threat-model.md
│  └─ incident-response.md
├─ LICENSES/
├─ THIRD_PARTY_NOTICES.md
├─ docs/
│  └─ adr/
├─ third_party/
├─ tools/
└─ .github/workflows/
```

Rust workspace 承载产品自有 Rust authority；CMake/build.ps1 仍是顶层 Windows product build orchestrator。共享协议靠 wire contract/golden corpus，跨语言业务逻辑不靠巨型 FFI bridge。某个 C++ 实现完成 cutover 后应在同批次删除，不在 `src/` 与 `rust/` 永久维护两份 authoritative 实现。直接操作 Fcitx core/addon 对象的 C++ adapter 是长期边界，不是待绑定进 Rust 的 TODO。

## 13.2 C++ 规则

- TSF 边界函数 noexcept；COM 生命周期使用 RAII/smart COM pointer；AdviseSink cookie 统一 RAII 管理。

- 解析外部输入前校验长度和版本；避免把裸 C++ struct 直接作为 IPC wire format。

- 优先 deterministic ownership；跨线程状态通过队列/消息传递，不共享可变全局对象。

- 打开 /W4、/WX（可按第三方代码隔离）、/sdl、DEP/ASLR/CFG/CET 等工具链可用的安全选项。

- 任何新增依赖必须说明：用途、许可证、最小 Windows、二进制体积、是否可访问网络/输入、是否支持目标工具链。

## 13.3 Config / Candidate UI / Rust 迁移边界

### Config — Rust product logic + native adapter during migration

- Config 的 typed state、validation、command orchestration、preview state、package/config protocol 继续迁 Rust；native HWND/WTL/D2D 只作为 Windows adapter 或 spike 对照存在。
- 若继续 WTL 路线，WTL 只用于 `fcitx5-config.exe` / 必要 management windows；它不是 Candidate renderer，也不进入 TSF/engine 输入热路径。
- Config UI 采用任务导向的原生/轻量自绘 Settings Surface：EDIT/复杂文本输入/系统 dialog 等可保持 native HWND；NavigationItem、SettingRow、Toggle、SegmentedControl、Slider、ThemeCard、InputMethodCard、Banner、Preview 等由小型可复用 component layer按需实现。该层只服务本产品，不演化成通用 GUI framework；每个自绘交互组件必须提供 keyboard/focus/UI Automation/High Contrast 等价语义。
- Config 不读取实时按键/preedit/commit history；网络操作通过 package/updater domain；配置语义通过 typed Control API/共享 schema 获得。
- 候选皮肤预览不得重写第二套 renderer。调用真实 Candidate renderer 的 synthetic preview path，只发送固定测试 CandidateModel/theme snapshot。
- Config/Control 的 child-process execution 必须只有一个 authoritative primitive：并发 drain stdout/stderr、bounded output、timeout、取消/Job containment；禁止复制“先 wait child 再读 pipe”的实现。
- Config Rust cutover 必须先有 UIA、DPI、High Contrast、keyboard-only、typed API、startup/perf 和 Legacy/Win7 决策证据；不得长期保留 WTL shipping、Rust PoC 和第三套 UI。

### Candidate UI — Rust product logic + Win32/D2D adapter during migration

- Candidate model/layout/interaction、freshness/order、presentation policy、snapshot DTO 和用户 action intent 继续保持 Rust authority；Win32 + Direct2D + DirectWrite renderer 可阶段性保留为 adapter。
- CandidateModel/theme/layout 与 renderer 分层；renderer 不拥有 authoritative candidate state。
- Candidate semantics 跟随 upstream `CandidateList` 与 candidate action API；Rust 层不私有发明删除/忘记/固定候选等插件语义。
- 对 IPC/theme/assets 先校验长度、数量、维度、路径与资源预算；Device loss、DPI、多屏、font fallback、High Contrast、RDP/低性能降级有可重放测试。
- Rust D2D renderer cutover 前必须已有 screenshot/golden/perf/a11y corpus；不能以 Rust rewrite 掩盖 UILess、A→B→A、locale、reload 等语义问题。

### Rust R1 — 不可信数据与更新域

优先迁移 `package core / repository / updater / downloader / provider`，以及在满足工具链/权限证据时的最小 deployer。原因是这些组件集中处理网络、manifest、archive、路径、签名/hash、版本/anti-rollback、staging/activation/rollback，Rust 的强类型 ownership 与 enum/state 表达有实际安全收益。

迁移前必须固定：package ID/path 类型、hostile Windows path corpus、manifest/repository canonicalization、signature/hash vectors、anti-rollback corruption semantics、atomic activation/previous-known-good、elevated/non-elevated privilege boundary。Rust implementation 必须与 C++ baseline 跑 differential corpus，不能以“Rust memory-safe”替代这些逻辑验证。

Generation Draining 的 Windows-specific 文件语义（in-use DLL rename、atomic move、file identity、pending cleanup/reboot delete）必须封装在极小 platform adapter 中；Rust 业务层使用强类型 generation/deployment state，但不得把 `PathBuf` 或 ownership 当成 Windows 文件更新正确性的自动证明。必要 Win32 调用允许最小 `unsafe`，要求 safety comment + hostile/in-use-file tests。

### Rust R2 — 状态机与管理工具

在 R1 稳定后再评估 `launcher / control / diagnostics / shared process-execution`。进入 R2 前先在 C++ 中修好并测试：launcher crash ledger 跨自身重启、process capture 并发 drain/cancel、session/SID ownership、repair semantics。Rust 的 `enum`/强类型用于减少非法状态，但不得把旧状态机的逻辑错误逐行翻译。

### 明确禁止的错误 Rust 化

- 不为 Rust Engine 建立完整 Fcitx C++ API binding；Rust 不持有 `fcitx::*` object 指针，不模拟 Fcitx inheritance/vtable。
- 不做 Windows 私有 Rust rewrite upstream addons；Windows 产品只决定 package/static/dynamic 部署、隔离、配置和 UI 呈现。
- 不把插件模型写死为动态 DLL；必须支持 static/built-in 与 dynamic/package-loaded。
- 不把当前 Fcitx frontend API 形状冻结成永久 Rust ABI；Engine protocol 必须 versioned、capability-aware、extensible。
- register/bootstrap 可保持最薄 Windows adapter；安全性靠输入 artifact identity、权限边界和可确认 child lifecycle，不靠语言替换叙事。
- 不把 Rust 行数比例、crate 数量、`unsafe` 百分比作为产品/工程 KPI。

## 13.4 Anti-Cheat Friendly Build 规则

- Release 不使用 UPX/第三方可执行压缩器、运行时 unpack、自修改代码或随机化模块/进程名。

- 禁止在 game directory 投放代理 DLL（如 d3d9.dll、dxgi.dll、dsound.dll）或通过图形 API 注入候选 UI。

- TSF DLL 依赖最小化，并通过安全 DLL 搜索路径加载依赖；公开发行保持可追溯签名、SBOM 和 source commit。

- 兼容 bug 的修复不得以“让反作弊看不到/识别不到 Fcitx5”为验收标准；验收标准只能是使用受支持 Windows API 后正常工作。

## 13.5 统一构建入口、Modern/Legacy 工具链与可追溯发布

对开发者/Codex 只暴露一个顶层入口，避免 CMake、Inno、SignTool、SBOM 和渠道命令散落：

```powershell
./tools/build.ps1 dev
./tools/build.ps1 test
./tools/build.ps1 package
./tools/build.ps1 release
```

- `dev`：构建当前 Phase 已存在的 C++ targets 与已经获准的 Rust R1/R2 targets；Config 仍纳入同一顶层 build 入口。
- `test`：运行 affected unit/integration/fuzz smoke、SAST/SCA、必要 x86/x64 / min_os checks。
- `package`：在干净 Windows runner 从锁定工具链无人值守构建全部真实 delivery artifacts，再生成 installer/portable/package metadata；无 production signing secret。
- `release`：提升同一 artifact lineage 的已验证产物，进行签名、最终安装/卸载 smoke、hash、SBOM、manifest、provenance/attestation 和渠道发布；不得在通过验证后偷偷重编。

### 13.5.1 CMake + WTL + Cargo clean-runner 模型

```text
clean Windows runner
  ↓
bootstrap/check pinned MSVC + Windows SDK + CMake + Ninja/Generator
  ↓
restore/vendor verified WTL + C++ dependencies
  ↓
if approved Rust targets exist:
  verify rust-toolchain.toml + Cargo.lock
  cargo metadata/deny --locked
  ↓
CMake/build.ps1 orchestrated build
  TSF / engine / ui / config + current C++ targets
  Rust R1/R2 targets where cutover is active
  ↓
unit / integration / self-test / PE min_os checks
  ↓
package
```

强制规则：

- **不存在 IDE Publish 步骤。** build/package/release 不需要 Visual Studio GUI、WTL wizard、UI Automation、桌面焦点或手工点击。
- CMake presets、toolset/SDK、WTL version、第三方 dependency lock/inventory 必须进入版本控制或机器可读 inventory。
- `tools/bootstrap.ps1` 可以安装/检查声明过的 build components，但不得依赖“作者电脑碰巧已经装好”。
- 网络下载的 tool/dependency 必须固定版本与来源，并在可行时校验 hash/signature；cache miss 后仍可从声明过的 source 重建。
- Rust/其他工具链必须由当前真实 target 触发并接入同一个 `build.ps1`；R1/R2 已获准的 Rust target 不得新增第二套手工 build ritual。

### 13.5.2 Modern / Legacy toolchain lanes

- Modern lane 面向 Windows 10/11，允许采用当前受支持的新 MSVC toolset/SDK 与 API。
- Legacy lane 面向 Win7/8.x，使用 pin 的 Win7-compatible MSVC toolset/SDK；实际第三方 dependency set 必须经过 Legacy CI。
- 只有真实需要时才产出 Modern/Legacy 两套 binaries。若一个 Legacy-compatible build 已满足所有当前需求，保持单一 lineage，避免预防性双倍维护。
- 一旦产生两条 release lineage，它们必须来自同一 source commit；每条 lineage 独立 Build Once、测试、签名、hash、SBOM，不允许把两条不同构建的文件混进同一声明不清的包。

### 13.5.3 Build Once / Promote Same Artifacts

Release lineage 固定为：

```text
source commit + dependency locks + exact toolchain/target
        ↓
controlled build ONCE
        ↓
test / SAST / SCA / fuzz / package smoke
        ↓
sign payload binaries
        ↓
create installer/portable
        ↓
sign final signable package
        ↓
release smoke
        ↓
final hashes + SBOM + manifest + provenance
        ↓
publish exact files
```

- Release flags/target 不同就代表不同 artifact lineage，必须从该 lineage 的唯一 controlled build 开始重新验证；不能“测试版编一份、发布版再编一份”。
- SignTool 对 PE/installer 的签名会改变 bytes；final SHA-256 必须在所有最终签名完成后生成。
- SBOM 描述实际发布依赖；release manifest 指向最终 signed artifact hash。
- 平台若支持 artifact attestation/provenance，Stable SHOULD 生成；它不替代 Authenticode/package signature。

### 13.5.4 Core CI 与 Packaging CI 解耦

- 日常 Core CI 可以只构建当前变更影响的 C++ 与 Rust targets；Package Gate 必须构建实际交付集合，包括 Config/installer metadata，并对 Cargo/C++ 两套 resolved dependency 生成统一 SBOM。
- Config GUI 只是 artifact，不拥有 Core build graph、config schema、package resolver 或 signing policy。
- hosted runner 原则上即可完成完整构建；只有签名硬件/证书等真实安全需求才使用专用 runner，不允许因为 GUI build 需要桌面 session 而引入 self-hosted“宠物机”。

## 13.6 Windows 技术栈决策：Fcitx C++ island + Rust product plane

**v1.8 当前选择冻结为：**

| 组件 | 当前/目标技术 | v1.8 决策 |
|---|---|---|
| `fcitx5-tsf.dll` | Rust / windows-rs / COM / TSF | **Rust product component。** 宿主内仍保持最小依赖、panic containment、fail-open、无 Fcitx/libime/addon 链接。 |
| `fcitx5-engine.exe` | Rust Engine Core + thin C++ Fcitx adapter | **Mixed by boundary。** Rust 拥有 protocol/state/validation/revision/generation/policy/IPC；C++ 只直接操作 Fcitx core/addon 对象。 |
| `fcitx5-ui.exe` | Rust target with Win32/D2D/DWrite adaptation allowed during migration | **Rust product component。** Candidate UI 产品语义、snapshot、layout policy 和 rendering state 继续迁 Rust；保留薄 Windows adapter 直到 cutover evidence 足够。 |
| `fcitx5-config.exe` | Rust target with native Windows adapter allowed during migration | **Rust product component。** typed settings、command orchestration、preview state、validation 和 package/config protocol 迁 Rust；不为 GUI 框架重写而丢失 native/a11y 约束。 |
| package / repository / updater / downloader / provider | C++ current → **Rust R1** | **首选迁移域。** Modern-first；contract + hostile corpus + differential + package smoke 后逐组件 cutover。 |
| elevated deployer | 最小 C++ current；Rust R1 可选 | 权限面必须最小；只有 Modern/Legacy toolchain 与 artifact identity 证据通过才迁。 |
| launcher / control / diagnostics / process-exec | C++ current → **Rust R2** | 第二波；先修 crash ledger、pipe/cancel/session 状态语义，再 side-by-side differential。 |
| register / bootstrap | Rust policy + 最小 Windows adapter | 产品策略迁 Rust；Win32 registry/elevation/DllRegisterServer 调用层保持最薄并可继续收缩。 |
| installer | Inno Setup | 成熟 installer；明确 machine/user ownership，不让 elevated uninstall 依赖“当前 HKCU”。 |

Rust 的目标是让产品自有 Windows 逻辑成为 memory-safe、typed、可差分验证的实现，而不是重写 Fcitx 上游。Modern lane 可以继续采用 Rust R1/R2/R3；Legacy lane 对每个组件单独证明 Rust target 的工具链和依赖可维护性，未证明则保留该组件 Legacy adapter lineage。两条 lineage 必须来自同一 source commit，并在各自 lineage 内 Build Once。

### 13.6.0 Fcitx 上游边界硬规则

1. Fcitx5 core object model 不做 Rust rewrite。
2. Upstream addons 不做 Windows 私有 Rust rewrite。
3. 直接操作 Fcitx-facing object 的层保持薄 C++ adapter。
4. 其余产品逻辑继续 Rust 化。
5. Addon 抽象必须支持 static/built-in 与 dynamic/package-loaded，不得假设 `Addon == DLL`。
6. Candidate semantics 尽量跟随 Fcitx `CandidateList` 和 upstream candidate action API；Rust Candidate 层只负责传输、状态、UI 与用户触发 intent。
7. Engine protocol 使用 capability model：`TEXT_COMMIT`、`TEXT_COMMIT_WITH_CURSOR`、`TEXT_DELETE_SURROUNDING`、`TEXT_REPLACE_SURROUNDING`、`CANDIDATE_ACTION` 等可扩展能力，不把当前 Fcitx frontend API 固定成封闭动作 enum。

### 13.6.1 皮肤与美观策略

- 候选皮肤由 `theme.toml + assets` 定义，renderer 使用现有 D2D/DWrite；第三方主题纯数据，不执行 JS/DLL/Slint code。
- WTL Config 默认使用系统原生控件与固定 spacing/type scale/design tokens；只对确实影响产品观感的区域做 owner-draw/D2D。
- Config 候选预览走真实 Candidate renderer 的 synthetic preview path，避免预览与实际候选窗漂移。
- 后期如需要 fade/selection transition 等动画，优先评估 DirectComposition；不因皮肤能力迁移整个 renderer。

### 13.6.2 Rust cutover 触发与停止条件

每个 R1/R2 组件只有同时满足以下条件才允许切换正式实现：

1. 当前 C++ 行为语义已修正，已知逻辑 bug 不作为 Rust rewrite 的输入；
2. contract/golden/hostile/fuzz corpus 足以描述关键行为；
3. Rust implementation 与 C++ baseline differential tests 通过；
4. clean runner、签名、package、install/repair/update/rollback smoke 通过；
5. Cargo license/advisory/source/SBOM/min-OS gate 接入；
6. Modern/Legacy artifact lineage 决策明确；
7. cutover 后在同批次删除被替换的 C++ authoritative implementation。

如果 Rust 版本在体积、启动、Win7维护、依赖供应链或可诊断性上明显恶化且没有相称安全收益，应停止迁移或回到 previous-known-good artifact；**不得因为已经投入迁移成本而继续扩大 Rust 化。**

## 13.7 非功能工程约束：性能、效率与复杂度

本节是与安全需求同级的工程约束。目标不是追求极端 benchmark，而是防止输入法随着功能增加逐步变成高延迟、高常驻、高耦合、难以审计的系统。

规范词：

- **MUST**：违反即不能进入对应 Phase 的完成标准或 stable release。
- **SHOULD**：默认必须满足；如不满足，需要在 issue/ADR 中记录原因、影响和后续处理。
- **MAY**：可选能力，不得反过来增加核心热路径复杂度。
- 下列数字是**初始工程预算**，不是对外产品 SLA。性能 baseline 随对应真实 target 首次出现的 Phase 建立，并使用固定参考机器和 Release build 测量；后续可用 ADR 调整，但不得为了让退化“通过”而静默放宽。

### 13.7.1 性能测量规则

- 冷启动、热启动、基础 keyboard/table、重型 addon（例如 Rime/Mozc）必须分开测量，不得混成一个平均值。
- 延迟报告至少记录 p50 / p95 / p99；只报平均值无效。
- 热路径 benchmark 至少先 warm-up，再采样；机器、Windows 版本、CPU、电源模式、构建类型、addon 组合必须记录。
- Benchmark 默认使用签名关闭与符号保留不影响逻辑的 Release 优化构建；Debug 数据不能作为性能验收依据。
- CI 中先记录趋势，主链稳定后再逐步升级为门禁。任何 p95 延迟退化 >15%、常驻内存退化 >20%、安装体积退化 >20% 的变更，SHOULD 给出原因。

### 13.7.2 输入热路径延迟预算

| ID | 约束 | 初始预算 |
|---|---|---|
| NFR-P001 | Warm key 从 TSF 发出到得到“handled/commit/composition state”结果 | p95 ≤ 5 ms，p99 ≤ 10 ms |
| NFR-P002 | TSF 同步等待 engine 的绝对 deadline | MUST bounded；`25 ms` 为初始 engineering budget，Phase 2 记录 p50/p95/p99 后通过 ADR 固化或调整正式上限；超时 fail-open / passthrough |
| NFR-P003 | `OnTestKeyDown/Up` | SHOULD 尽量本地/只读判断；不得执行持久状态变更、磁盘 I/O、进程启动 |
| NFR-P004 | CandidateModel 更新到可见候选完成一帧 | p95 ≤ 16 ms，p99 ≤ 33 ms |
| NFR-P005 | Focus/Profile 激活热路径 | p95 ≤ 20 ms；不得同步做词库扫描、完整配置解析或 updater 检查 |
| NFR-P006 | 基础 engine 冷启动 Ready | SHOULD ≤ 1 s，超过 2 s 必须记录并依赖预热而不是放宽按键 timeout |
| NFR-P007 | 输入吞吐 | MUST 连续承受 60 Hz key repeat 无队列累积；synthetic burst 200 events/s × 2 s 不崩溃、不无限增长 |

关键原则：**冷启动慢用 launcher + warm-up 解决，不能用“把 KeyDown timeout 改成几秒”解决。**

### 13.7.3 CPU、内存、进程、线程与句柄预算

以下是基础安装、无重型第三方 addon 时的初始预算；词典 mmap/file cache 与明确标注的 addon 内存单独统计。

| 组件 | 初始预算 |
|---|---|
| `fcitx5-tsf.dll` | 每宿主激活后的增量 private bytes SHOULD ≤ 4 MiB，MUST ≤ 8 MiB |
| `fcitx5-launcher.exe` | idle private bytes SHOULD ≤ 16 MiB |
| `fcitx5-ui.exe` | idle/隐藏 SHOULD ≤ 32 MiB；隐藏时不得持续重绘 |
| `fcitx5-engine.exe` | 基础 keyboard/table 配置 SHOULD ≤ 64 MiB，不含明确统计的 mmap dictionary / optional addon |
| 全部后台组件 idle CPU | 60 秒平均 SHOULD < 0.2%（参考机）；不得 busy-wait |

强制规则：

- TSF DLL MUST NOT 创建“每个 InputContext 一个线程”或“每个按键一个线程”。
- TSF 默认不得有轮询线程；若异步 IPC 最终需要 lazy worker，MUST 每宿主进程最多一个、可安全卸载、无空闲轮询。
- Engine 的并发 I/O 可以多线程，但 Fcitx 状态变更必须回到单一 authoritative event loop；禁止 thread-per-request。
- Launcher 不得演化成常驻大服务；只承担生命周期、状态、恢复、最小健康检查。
- UI 隐藏时不跑 60 FPS render loop，不为动画保活 GPU/CPU。
- 10,000 次 focus/context/create-destroy + 输入循环后，HANDLE、GDI object、COM sink subscription 不得持续净增长；资源泄漏是 release blocker。
- 空闲状态不得使用 <250 ms 的周期轮询；SHOULD 事件驱动。确有短周期 timer 时，只允许在短暂活动状态存在并及时撤销。

### 13.7.4 IPC、队列、内存分配与背压

- Hot-path request（key/focus/caret 等）SHOULD ≤ 4 KiB。
- Candidate snapshot 正常 SHOULD ≤ 64 KiB，只发送**当前可见页/必要元数据**，不得每键发送完整词库或全量历史候选。
- 单个 hot-path frame MUST ≤ 256 KiB；control frame MUST ≤ 1 MiB。大文件、词库、插件包禁止直接塞 IPC，应使用受验证的文件路径/包事务。
- 所有来自 IPC/文件/插件的数据长度在分配前校验；禁止 `resize(remote_length)` 这类无上限分配。
- 每连接队列必须有上限；建议默认 ≤256 个待处理消息。达到上限时优先 coalesce/drop stale caret/candidate/revision 消息，而不是继续吃内存。
- `request_id`、`engine_epoch`、`context_id`、`composition_id`、`revision` 必须用于丢弃迟到响应和过期 UI 更新。
- 不允许在持有全局/Context mutex 时做阻塞 IPC、`RequestEditSession`、跨进程 `SendMessage` 或用户代码 callback。
- 不允许依赖“请求与响应永远严格 FIFO”作为正确性条件。
- 对 UTF-16（TSF）与 UTF-8（Fcitx）转换要有明确边界；同一字符串不得在一条热路径中反复 UTF-8↔UTF-16 来回转换。
- 热路径 SHOULD 避免不必要 heap churn；CandidateModel 使用有所有权的 snapshot/move 或复用 buffer。不得为了微优化引入不安全裸指针生命周期。

### 13.7.5 磁盘 I/O、日志、词库与缓存

- Key/TestKey/Composition 热路径 MUST NOT 做同步磁盘 I/O、目录扫描、配置解析、更新检查或网络请求。
- 用户学习数据不得每个按键 `fsync`；采用批量/idle/shutdown flush，并保证 crash 后最多丢失有限未刷新的学习数据，而不是损坏数据库。
- 配置、词库索引、包数据库写入使用 temp → validate → atomic replace；禁止直接 truncate 正式文件后再慢慢重写。
- Release 日志默认轮转并有硬上限；建议单文件 ≤10 MiB、保留 ≤3 份。不得无界增长。
- 包下载 cache、构建 cache、词库 cache 必须各自声明上限和清理策略；不得把 `%LOCALAPPDATA%` 当无限空间。
- Core idle 不得周期性扫描整个 plugin/theme/schema 目录。变更通过显式 reload、文件通知或低频管理平面操作触发。
- 大词典 SHOULD 优先 mmap/只读共享/按需索引，避免每个 InputContext 复制一份。

### 13.7.6 Candidate UI 与图形性能约束

- `fcitx5-ui.exe` 使用 C++ + Win32 + Direct2D + DirectWrite；正常候选路径不依赖 WebView2/Chromium/Qt/通用 GUI framework。
- DirectWrite font/text format、D2D brush/bitmap 等可复用对象要缓存；不得每次 `WM_PAINT` 全量重建资源。
- Device loss 时重建 device-dependent resource；不得因此重启整个 engine。
- UI 更新采用 revision/snapshot；同一 revision 重复消息不重绘。
- 候选窗隐藏后停止动画和 render timer。
- 阴影、透明、动画属于 MAY；当它们使 p95 render 超预算、影响 Win7/远程桌面/游戏兼容时必须可关闭或自动降级。
- 低性能/Legacy/Game 模式优先正确输入与低延迟，不保证高级动画。

### 13.7.7 零遗留兼容与删除策略

- 项目只维护**当前架构和当前格式**。内部 API、IPC、package schema、配置字段、feature flag 一旦废弃必须从代码、测试、文档、样例和 CI 中一起删除。
- 禁止 `legacy/compat/v1/v2-old/deprecated` 一类长期目录或分支。若名字只是当前平台概念（例如 Windows legacy application/IMM32）则必须明确它指宿主类型，而不是项目旧实现。
- 禁止为了读取旧配置保留旧 parser。未知/过期 schema 明确报错或重置可再生配置；不可再生数据必须在设计时尽量保持稳定，避免频繁改格式。
- 禁止旧新 IPC 双栈、双写文件、双 package manifest、旧字段 alias、deprecated API wrapper。
- 删除功能时删除其配置、文档、测试、资源、依赖和死代码，不留关闭状态的 feature flag。
- Feature flag 只允许用于**当前同时存在且都受支持**的产品行为（例如 Safe Mode / Game profile），不得用于拖延删除旧实现。
- 任何 PR 若在 Core runtime 新增 `legacy`、`compat`、`migration`、`deprecated`、`fallback` 语义的项目自有代码，默认视为设计审查失败；除非它描述的是当前明确支持的外部平台行为且没有保留项目旧实现，或是经明确授权、与 runtime 隔离的一次性离线用户数据转换工具。

### 13.7.8 代码复杂度预算

对**新增的项目自有代码**使用以下默认门槛；第三方/upstream/generated code 不强制套用，但不得用它规避自有代码质量要求。

| 项目 | 默认约束 |
|---|---|
| Cyclomatic Complexity | 每函数 SHOULD ≤ 10；>15 必须重构或在 review note 中解释 |
| 函数长度 | SHOULD ≤ 80 个逻辑行；TSF/COM boundary handler SHOULD ≤ 40 行 |
| 嵌套深度 | SHOULD ≤ 4 层 |
| 单源码文件 | SHOULD ≤ 1,200 行；generated/vendor 除外 |
| 单个 Codex 实现批次 | SHOULD < 800 行净新增自有代码；>1,500 行应拆分，导入 upstream/生成代码除外 |
| 公共接口 | 最小化；没有第二个真实调用方时不要为了“未来”先造通用框架 |

复杂度原则：

- KISS / YAGNI 优先于“看起来很架构化”。
- 禁止 `ManagerManager` 式万能对象。一个类/模块应有明确 owner 和失败边界。
- 禁止为了统一而把 TSF、engine、UI、package service 抽象成一个巨型 cross-platform interface。
- 两处相似代码不一定立即抽象；出现第三个稳定用例再考虑公共抽象。
- 禁止循环模块依赖；CMake target dependency 必须形成 DAG。
- 禁止跨模块直接访问内部 global state；通过窄接口/消息传递。
- 除显式 process-wide owner（例如 engine instance/launcher state）外，禁止可变全局 singleton。
- 异常不得作为常规分支控制流；TSF/COM 边界不得让异常/panic/abort 穿出。
- 不要为了减少几次 allocation 引入 UAF/悬垂引用风险；安全和确定 ownership 优先于纳秒级优化。

### 13.7.9 依赖与二进制复杂度约束

- TSF DLL SHOULD 只依赖 Windows 系统库、项目自有最小 common/ipc；禁止第三方 GUI、Web runtime、脚本 runtime、输入引擎或 plugin runtime 进入宿主。
- 每个新增第三方依赖必须记录：用途、版本/commit/hash、license、min Windows、架构、二进制体积、是否联网、是否处理输入、替代方案。
- Release build 禁止在线下载“latest dependency”；依赖必须 pin/lock。
- 一个库如果只是为了 20~50 行可维护功能而引入数 MiB runtime/复杂供应链，默认不接受。
- TSF/UI/launcher 的最终 binary size 在 Phase 2/4 建 baseline；之后单组件增长 >20% SHOULD 说明原因。
- Core installer（不含可选大词库/插件/debug symbols）SHOULD 维持轻量；初始目标 ≤80 MiB，超过时必须给出构成分析，而不是静默接受。
- PDB/debug symbols 与用户安装包分离，但 stable release SHOULD 保留对应 symbols 供本地 crash diagnosis。

### 13.7.10 Windows API、路径与当前平台支持约束

- Windows API 边界使用 Unicode/Wide API；源码 UTF-8。必须测试非 ASCII 用户名、安装目录、词库名、方案名。
- 不假设 `MAX_PATH` 足够；能使用长路径安全方式时使用，Win7 当前支持路径单独测试。
- 不依赖 Current Working Directory 解析 DLL、配置、资源或插件。
- Win10+ API 用 capability detection；Win7 目标二进制不得因 import table 硬引用现代 API 而无法加载。
- 跨进程查询必须有 timeout；目标应用被挂起/无响应不能拖死输入法。
- 不读取目标进程内存来识别应用，不因游戏兼容申请 `PROCESS_VM_READ`；按 executable identity 使用受限的 process query API 即可。

### 13.7.11 可靠性与失败语义

明确“什么时候 fail-open、什么时候 fail-closed”：

- Engine 不可用、IPC timeout、UI 崩溃：输入可用性优先，**fail-open/passthrough**，不冻结宿主。
- IPC peer 身份/签名验证失败、包签名失败：安全优先，**fail-closed**，不得连接/激活不可信对象。
- Candidate UI 不可用：保持 composition/commit 与 UILess 语义，视觉 UI 可缺失。
- 第三方 addon 崩溃：隔离到 engine，触发 crash accounting / Safe Mode，不把 TSF 宿主带走。
- 配置损坏：保留原坏文件用于用户修复/诊断，运行时使用 compiled safe default 或最后一个当前 schema 的有效内存 snapshot；不得猜测修复、自动重写坏文件、保留旧 schema parser，也不得 crash-loop。
- 用户明确 `UserStopped` / 正在 `Updating` / `Uninstalling`：TSF 不得自作主张重新拉起 engine。
- crash restart 必须 exponential/backoff 或等价节流；禁止每秒几十/几百次 respawn。

### 13.7.12 额外硬约束：默认禁止事项

这些规则用于防止个人项目和 AI 辅助开发在功能增长后失控。除非当前明确需求和可测证据证明必须，否则默认禁止。

**基础设施与依赖**

- 不自写密码学、签名验证、TLS、HTTP client、JSON/YAML parser、Unicode shaping、压缩/解压、semver、SQLite/数据库引擎等已有成熟实现的基础能力。优先使用 Windows/Fcitx/现有依赖或成熟维护库。
- 新依赖必须先证明现有依赖无法满足；“我不知道现有库有没有”不是加包理由。
- 不同时引入两个解决同一问题的库。一个能力只能有一个默认实现。
- 不引入仅用于 syntactic sugar、几十行辅助代码或一次性功能的大 runtime。
- 不把 package manager、updater、config UI 的依赖带入 TSF/engine 热路径。

**运行时与进程模型**

- 默认长期常驻的项目进程只允许 launcher、engine，以及实际需要显示时的 UI；config/package/updater/deployer 使用完即退出。新增常驻进程必须有明确不可替代的信任边界理由。
- 不使用 Windows Service/Session 0 承载 per-user 输入 engine，除非未来有新的当前需求明确要求。
- 不使用 thread-per-key、thread-per-request、thread-per-context；禁止无界线程池和无界任务队列。
- shutdown/restart 必须可取消、有 deadline，不依赖“等一会应该就结束”。
- 不用 `Sleep()`、固定延时或循环 retry 作为同步/竞态正确性机制；使用 event、condition、overlapped I/O、明确状态机或系统通知。
- 不允许隐藏后台 polling、自动扫描、自动联网、自动上传或定期唤醒；所有后台活动必须有事件来源和可解释生命周期。

**TSF / COM / DLL 边界**

- TSF DLL 不执行 shell 命令，不提供 `ShellExecute` 代理，不启动任意 URL/程序，不读取剪贴板，不访问网络。
- DLL/进程边界不得传递需要跨模块释放的 C++ owning object、STL container、异常或 allocator ownership。跨进程使用显式 wire format；跨 DLL 若未来确有需要，优先稳定 C ABI/POD 或同工具链且明确所有权的窄接口。
- COM sink/cookie、HANDLE、HWND、HDC、D2D/DWrite 对象都必须有明确 RAII owner；不得依赖进程退出“顺便释放”。
- TSF callback 中禁止加载 DLL、扫描目录、解析完整配置、启动 updater、执行插件代码或等待长任务。
- Release 的 TSF DLL 禁止 debug backdoor、测试 pipe、隐藏诊断命令和动态脚本入口。

**配置与状态**

- 配置只为真实用户可理解的行为存在。不得为内部实现细节暴露开关，不增加“也许以后会调”的参数。
- 同一状态只能有一个 source of truth；禁止 registry、JSON、engine memory、TSF compartment 多处互相同步同一个 authoritative 值。其余只能是派生/缓存。
- 可再生 cache/index 损坏或格式变化直接删除重建，不写修复器或格式迁移器。
- 不可再生用户数据必须与程序版本分离，并尽量使用简单稳定的数据表示；如果未来必须 breaking change，先重新设计当前格式，而不是提前埋 migration framework。
- 配置写入、package 激活、词库元数据更新必须原子；进程崩溃后只能看到旧完整状态或新完整状态。

**文件系统、安装与供应链**

- Program Files 下的 core/native addon 目录普通用户不可写；用户数据目录不得成为默认 native DLL 搜索路径。
- 解包所有 archive/package 时必须防 `..`、绝对路径、UNC、drive-prefix、symlink/reparse-point 逃逸和覆盖安装目录外文件。
- 临时文件必须位于受控 temp/staging 目录，使用不可预测名称/安全创建语义；验证完成前不得从 temp 直接执行下载的 native code。
- 不从当前工作目录、PATH、Downloads、用户可写插件目录隐式加载 core DLL。
- Release 依赖、工具链和生成器版本全部 pin；禁止 release 时拉取 `latest`。

**网络与隐私**

- Core/TSF/engine/UI 默认零网络。需要网络的新功能必须是独立 broker，并先定义它能看到的最小数据。
- 默认无遥测、无自动 crash dump 上传、无输入内容日志。未来若增加统计，必须 opt-in 且与输入内容彻底分离。
- Clipboard、当前选中文本、历史 commit 都视为敏感输入；没有当前明确功能就不读取。

**测试与可诊断性**

- 测试不得用无限 retry、扩大 timeout 或跳过断言来“修复 flake”。出现 flake 先定位竞态/环境依赖。
- 单元测试不得替代真实 TSF E2E；涉及 COM lifecycle、focus、DPI、游戏、Terminal、安装注册的行为必须有对应 integration/E2E。
- 兼容 bug 必须保留最小可重放回归用例；修复后删除只为旧错误路径存在的临时代码。
- 错误处理使用明确 error/status，不通过解析日志字符串判断业务状态。
- 每个后台进程必须支持 `--version`/基本 health/status 或等价可诊断能力，但 release 不暴露可执行任意命令的调试 RPC。

**代码与构建**

- 自有 C/C++ 代码 warning-free；项目代码默认 warnings-as-errors，第三方 vendor 单独隔离。
- 禁止新增未解释的 `#pragma warning(disable)`、裸 `reinterpret_cast`、所有权不明的 raw `new/delete`、可避免的全局可变状态。
- 安全边界 parser 必须有 fuzz target；解析器新增字段必须有长度/数量/深度上限。
- 生成代码与 vendor code 必须与手写代码目录分离；不手工修改生成文件。
- CMake target 应小而明确；禁止一个 target 链接“以后可能用”的所有库。
- Build/CI 不依赖开发机个人路径、注册表偶然状态或已安装但未声明的 SDK。

### 13.7.13 性能与复杂度的 CI / Release Gate

`tests/perf/` **随对应真实 target 出现逐步建立，不预建空 benchmark**：

```text
Phase 1B  key_roundtrip_bench / ipc_codec_bench
Phase 3   engine_startup_bench / idle_resource_probe
Phase 4   candidate_model_bench / candidate_render_bench
Phase 5   focus_context_churn / handle_leak_soak
```

若某 target 尚不存在，对应 benchmark 不得仅为“基础设施完整”而提前创建；target 首次进入当前 Phase 后，应同时建立可重复 baseline。

Release checklist 至少记录：

- x86/x64 TSF binary size；
- TSF/launcher/engine/ui idle private bytes；
- 60 秒 idle CPU；
- warm key p50/p95/p99；
- candidate render p95/p99；
- engine cold/warm startup；
- 10k context/focus soak 后 handle/GDI object delta；
- installer/core package size。

个人项目不要求建立大型性能实验室。允许只维护 1 台固定参考机 + 1 台低端/Win7 VM 作为趋势基线，但**必须用相同脚本重复测**，而不是凭“感觉没有变慢”。

## 13.8 AI/Codex 代码生成附加约束

Codex 的目标是 **smallest correct result**：当前结果必须完整正确，但不把一次任务升级成未来平台建设。代码量、文件数和 diff 大小只是报警信号，不是完成度指标。

### 13.8.1 Task Contract：先判断当前意图

| 模式 | 默认授权 | 禁止自动升级的行为 |
|---|---|---|
| RESEARCH | 读仓库、读参考实现、形成证据和建议 | 不修改产品代码，不“顺手实现” |
| REVIEW | 只读审查、指出问题、给修复建议 | 不改文件、不格式化、不重构 |
| CHANGE | 实现用户明确要求 + 当前正确结果的必要后果 | 不扩展到邻近优化、未来 feature、无关清理 |
| RELEASE | 完成本次 release 所需版本、构建、测试、签名、SBOM、打包/发布准备 | 不借 release 顺手重构产品 |

没有显式标签时按用户自然语言推断；不确定时选择更窄的权限，而不是更宽。用户明确要求修改时不因“保守”拒绝必要修改。

### 13.8.2 Stop Ladder：任何额外动作前

对当前请求之外的每个新增动作依次问：

1. 用户明确要求了吗？
2. 当前 requested result 的正确性/验收必须依赖它吗？
3. 有什么**当前可达**代码、调用方、数据、平台行为、部署状态或测试失败证明它必须做？
4. 如果不做，当前验收会失败吗？

四问都无法给出肯定证据：**停止，不做。** “以后可能”“顺手”“最佳实践一般建议”“也许更灵活”都不是 reachable evidence。

### 13.8.3 代码生成范围

- 未经明确需求，不新增 daemon、service、broker、database、RPC framework、DI container、plugin abstraction 或新的序列化技术。
- 能用已有 Windows/Fcitx primitive 解决时，不再造一套相同抽象。
- 不允许为了一个 bug 同时重构三个无关模块。
- 不复制参考仓库的兼容 hack，除非先写出该 hack 对应的失败用例并证明当前 Windows 路径确实需要。
- 从 GPL/LGPL/BSD/MIT 等参考仓库复制非平凡代码前必须确认许可证兼容、保留必要 attribution；优先借设计与测试思路，除非明确决定复用代码。
- 禁止以“暂时 workaround、以后再换”为理由提交已知会被替换的架构路径。若当前需求无法在目标架构上正确完成，应缩小功能范围，而不是加入临时实现。
- 不引入隐藏的旧实现兼容路径：Safe Mode、插件 quarantine、当前应用模式等必须有可诊断状态，但日志不得包含输入内容。协议版本不一致直接拒绝，不做协议降级。
- 删除死代码和已废弃路径优先于保留“以后可能用”。同一功能只允许一个 authoritative 实现。
- 不为了满足“少改代码”留下半套调用链；必要调用方、测试和两端协议属于 smallest correct result，应一次改完整。

### 13.8.4 自动 Gate 的边界

优先自动检查这些**高置信、客观**事件：新增第三方依赖；新增常驻进程/服务；新增网络能力；新增 global hook/SendInput/注入/进程内存访问；TSF DLL 新增非白名单依赖；修改 IPC schema；提升 installer 权限；新增敏感日志字段；改变 DLL 搜索目录；引入无界 timeout/queue。

不要为了自动化去判断“这个 abstraction 是否多余”“这个类名是不是 Manager”“这个架构是否够优雅”等上下文语义。先靠 review、Bad/Good Case 和真实失败证据；只有能够稳定机械判定时才升级为 gate。

### 13.8.5 完成与停止

- 当前验收标准、受影响测试、必要安全检查已有足够证据后停止。
- 禁止为了“更放心”无限追加 web/repo 搜索、第二套实现、无关重构、全矩阵重复运行或给 guard 再加 guard。
- Stable Release、签名、SBOM、Win7 advertised support、IPC/parser fuzz 等本文明确要求的门禁不属于“多余工作”，按阶段执行。
- 如果一次实验/抽象被证据证明收益不足或增加错误面，优先整套删除，而不是继续叠兼容层、feature flag 和补丁。

每个 batch 结束只需回答：当前验收是否满足？是否增加热路径工作、常驻内存/线程、新信任边界或新第三方依赖？若有，证据/benchmark/必要性是什么？满足后停止。


## 13.9 配置文件格式、所有权与严格解析规则

### 13.9.1 格式决策：用户只承担一个配置文件

从用户心智模型看，第一版只有两类人类可编辑文件：

```text
普通用户：config.toml
主题作者：theme.toml + assets/
```

其他 JSON 文件属于构建、打包、运行时机器数据，不应要求普通用户理解或维护。

| 数据 | 格式 | 人工维护者 | Owner / Writer | 说明 |
|---|---|---|---|---|
| Windows 用户设置 | **TOML 1.0** `config.toml` | 普通用户可选 | Windows config core | 唯一普通用户配置；GUI 是主要入口 |
| 候选主题 | **TOML 1.0** `theme.toml` | 主题作者 | theme tooling | 主题视觉定义 + 主题元数据；字体/颜色/布局均可定义 |
| Package manifest | **严格 JSON** `manifest.json` | **不人工维护** | package tooling 生成 | 签名、哈希、安装元数据；主题包从 `theme.toml` 生成相关 manifest 字段 |
| Package lock | **严格 JSON** `packages.lock` | **不人工维护** | package service only | 机器生成 |
| i18n | **严格 JSON** `locales/*.json` | 翻译维护者 | translation tooling | 程序资源，不属于用户设置 |
| Fcitx/addon 配置 | **Fcitx 原生配置 API/格式** | 高级用户/插件自身 | Fcitx Core / addon | Windows 层不镜像、不复制 TOML |
| IPC | 当前二进制结构化协议 | 无 | TSF/engine/UI | 不使用 TOML/JSON 作为按键热路径协议 |

**不使用 YAML 作为本项目自己的配置格式。** TOML 覆盖人类编辑需求，严格 JSON 覆盖机器数据。禁止同时保留 `theme.yaml` / `theme.toml` 或多套等价配置语言。

**主题作者不应同时维护 `theme.toml` 和 `manifest.json` 的重复字段。** 主题的 `id/name/version/license/description` 等 authoring metadata 以 `theme.toml` 为 source of truth；打包工具生成用于安装/签名的 `manifest.json`。

### 13.9.2 文件位置与职责

安装版默认：

```text
%LOCALAPPDATA%\Fcitx5\
├─ config.toml
├─ themes/
├─ data/
├─ cache/
└─ logs/
```

Portable：

```text
<portable-root>\data\
├─ config.toml
├─ themes/
├─ data/
├─ cache/
└─ logs/
```

- `config.toml` 只保存 Windows frontend / candidate / appearance / management UI 等 Windows 层用户偏好。
- Fcitx Core、addon、Rime 等已有 authoritative 配置继续通过它们自己的 Config API/目录管理；禁止把相同 setting 再复制进 `config.toml`。
- 注册 TSF、CLSID、安装路径、update owner 等系统集成状态放其应有的系统/installer owner，不为了“所有东西都在 TOML”强行镜像。
- WTL GUI 是编辑前端，不拥有配置语义。它通过 typed Control API 或共享 typed config implementation 读取/写入；不得另写一套 GUI 私有 TOML 规则。
- TSF DLL 永远不读取这些文件。

### 13.9.3 版本规则：运行时精确匹配，不做隐式迁移

所有本项目 TOML/JSON 顶层格式都有整数 `format_version`：

```toml
format_version = 1
```

规则：

- parser 只接受当前精确版本；
- 旧版本、新版本、缺失版本都明确拒绝；
- 不猜 schema、不自动补字段、不在 runtime 自动跑 migration、不让 runtime 同时读两版；
- 被拒绝的用户文件保持原样，程序使用内置安全默认值继续提供输入主链，并在配置器显示可行动错误；
- 派生 cache 可直接删除重建；不可再生用户数据不放进会频繁 breaking 的 Windows shell schema。
- 公开 Stable 后若 breaking change 会让真实用户人工配置无法继续使用，可以提供**用户显式触发的离线 Convert/Import**：读取旧文件、完整校验、写出新的当前格式文件，但原文件必须保留；转换代码不得进入 engine/TSF/runtime parser，也不得形成长期 v1/v2 双栈。

“使用内置默认值继续运行”是配置损坏时的 fail-safe；显式离线转换是用户数据导入工具；两者都不是 runtime 旧格式兼容层。

### 13.9.4 TOML 语义约束

- UTF-8；writer 输出 UTF-8 无 BOM。
- key 使用小写 `snake_case`；枚举值使用文档指定的固定小写字符串。
- unknown key = error；duplicate key = error；wrong type = error。
- 不支持 `include`、继承指令、环境变量替换、模板表达式、Lua/JS、命令执行、远程 URL、任意文件读取。
- 缺失字段表示“继承下一层 / 使用默认”，不发明 `null` 魔法值。
- Reset = 删除该 override key/table；不得把继承值复制进用户文件。
- 不做宽松 coercion，例如字符串数字自动转数字、`yes/1` 自动当 bool。
- 所有数值字段都有 schema 范围；超范围是错误，不静默 clamp 后保存。Renderer 的 OS work-area clamp 属于运行时保护，不修改配置语义。
- 第一版不要求 live file watcher。GUI 原子保存后显式通知 reload；手工编辑在显式 Reload/相关进程重启后生效。
- GUI 可以 canonicalize/rewrite TOML；**不承诺保留用户手写注释原位置**，避免为了 comment-preserving AST 增加复杂依赖。

### 13.9.5 `config.toml` v1 正式结构

配置是 sparse override。未出现的键继承主题/默认：

```toml
format_version = 1

[appearance]
mode = "system"                 # system | light | dark
theme = "builtin:default"

[candidate]
orientation = "vertical"        # vertical | horizontal
max_width_dip = 720.0
opacity = 1.0

[candidate.geometry]
padding_x_dip = 8.0
padding_y_dip = 6.0
item_padding_x_dip = 6.0
item_padding_y_dip = 4.0
row_gap_dip = 2.0
column_gap_dip = 8.0
border_width_dip = 1.0
corner_radius_dip = 8.0
shadow = true

[candidate.label]
visible = true
style = "dot"                   # plain | dot | paren | bracket | circled
font_scale = 0.85
gap_dip = 4.0

[fonts.ui]
families = ["system"]

[fonts.candidate]
families = ["LXGW WenKai", "Microsoft YaHei", "system"]
size_dip = 16.0
weight = 400

[fonts.annotation]
families = ["inherit"]
scale = 0.80

[fonts.monospace]
families = ["Cascadia Mono", "Consolas", "system"]

[candidate.colors]
background = "#202124F2"
border = "#5F6368FF"
preedit_text = "#FFFFFFFF"
label_text = "#BDC1C6FF"
candidate_text = "#FFFFFFFF"
comment_text = "#BDC1C6FF"
selected_background = "#8AB4F8FF"
selected_label_text = "#202124FF"
selected_candidate_text = "#202124FF"
selected_comment_text = "#303134FF"
shadow = "#00000066"
```

说明：

- `system` / `inherit` 是 schema 保留 token，不是字体文件路径。
- `families` 是有序 preference chain，最后进入 DirectWrite system fallback。
- 颜色只接受 `#RRGGBB` / `#RRGGBBAA`；不接受 CSS 名称、HSL、表达式或 `rgba()`，避免多套 parser。
- GUI 可以只暴露高价值字段；高级用户手改 TOML 能使用完整 v1 schema。
- `page_size`、selection keys、candidate order、commit key **不在这里**；它们由 Fcitx Config API 拥有。

### 13.9.6 `theme.toml` v1 正式结构

主题作者只维护：

```text
my-theme/
├─ theme.toml
└─ assets/
   ├─ logo.png
   └─ ...
```

打包时由 tooling 生成机器使用的 `manifest.json`；主题作者不维护两份元数据。

`theme.toml` 同时包含主题 authoring metadata 和候选视觉默认值：

```toml
format_version = 1

[theme]
id = "example.clean"
name = "Example Clean"
version = "1.0.0"
license = "MIT"
description = "A clean Fcitx5 candidate theme"

[common.candidate]
orientation = "vertical"
max_width_dip = 720.0
opacity = 1.0

[common.candidate.geometry]
padding_x_dip = 8.0
padding_y_dip = 6.0
corner_radius_dip = 8.0

[common.fonts.candidate]
families = ["LXGW WenKai", "Microsoft YaHei", "system"]
size_dip = 16.0
weight = 400

[common.fonts.annotation]
families = ["inherit"]
scale = 0.80

[common.candidate.label]
visible = true
style = "dot"
font_scale = 0.85

[light.candidate.colors]
background = "#FFFFFFFF"
candidate_text = "#202124FF"
selected_background = "#D2E3FCFF"
selected_candidate_text = "#174EA6FF"

[dark.candidate.colors]
background = "#202124F2"
candidate_text = "#FFFFFFFF"
selected_background = "#8AB4F8FF"
selected_candidate_text = "#202124FF"
```

规则：

- Theme **可以**设置 candidate layout、字体/fallback/字号、颜色、label/序号外观、geometry、shadow、opacity 和 assets。
- Theme 不能出现 `page_size`、selection key、commit、network、command、addon、engine option；这些属于输入语义或其他 owner。
- `theme.*` authoring metadata 是主题作者唯一元数据 source of truth；打包生成的 `manifest.json` 不反向成为作者配置。
- 合并顺序固定为 `common → active appearance branch → user override → accessibility override`。
- 不支持主题之间 `extends` / include；要复用就复制/生成完整主题，避免形成第二套依赖系统。
- asset 只能是 package-relative path 或 `builtin:*`，并受 5.6 的尺寸、路径、SVG 安全限制。

### 13.9.7 v1 数值与枚举范围

第一版 validation 使用明确范围，GUI 也使用同一 typed schema 生成控件边界：

| 字段 | v1 范围 / 枚举 |
|---|---|
| `appearance.mode` | `system | light | dark` |
| `candidate.orientation` | `vertical | horizontal` |
| `candidate.max_width_dip` | 160–2048 DIP |
| `candidate.opacity` | 0.20–1.00 |
| padding / gap | 0–64 DIP |
| `border_width_dip` | 0–8 DIP |
| `corner_radius_dip` | 0–64 DIP |
| Candidate `size_dip` | 8–72 DIP |
| font `weight` | 100–900，100 步进 |
| annotation `scale` | 0.50–1.50 |
| label `font_scale` | 0.50–1.50 |
| label `style` | `plain | dot | paren | bracket | circled` |
| font families | 1–8 entries |
| color | `#RRGGBB` 或 `#RRGGBBAA` |

- UI 不应该允许产生 schema 外的值。
- 手工配置超范围直接报错，不自动改成最近合法值。
- 若真实主题证明某个范围不足，修改**当前 v1 schema 和测试**即可；不因此保留旧范围分支或兼容代码。
- High Contrast、work-area clamp、DPI rounding 等系统级运行时保护仍可覆盖最终 paint 结果，但不回写用户配置。

### 13.9.8 JSON 文件规则

机器数据统一使用严格 JSON：

- UTF-8；writer 可用确定性字段顺序便于 diff，但字段顺序不具有语义。
- 禁止 comment、trailing comma、NaN、Infinity、重复 key。
- manifest 签名针对**确切 manifest bytes 的 hash / detached signature**，不自造 JSON canonicalization。
- `packages.lock` 只由 package service 写；用户手改后仍必须完整验证。
- i18n 第一版优先 flat `string key -> string value`；不在翻译字符串里执行 HTML/JS/命令。未来真的需要复杂复数/MessageFormat 时再基于真实语言需求引入成熟实现。

主题包的 `manifest.json` 示例（**由打包工具根据 `theme.toml` 和构建信息生成，不要求主题作者手写**）：

```json
{
  "format_version": 1,
  "type": "theme",
  "id": "example.clean",
  "version": "1.0.0",
  "entry": "theme.toml",
  "license": "MIT",
  "source_commit": "..."
}
```

### 13.9.9 大小与资源上限

初始安全预算：

```text
config.toml       <= 256 KiB
theme.toml        <= 512 KiB
manifest.json     <= 256 KiB
packages.lock     <= 2 MiB
single locale     <= 2 MiB
font family list  <= 8 entries
font family name  <= 128 UTF-8 bytes
theme/package id  <= 64 ASCII chars
```

这些是输入验证预算，不是旧版兼容承诺；需要扩大时先用真实资源证明必要性。

### 13.9.10 ID、路径与原子写入

- package/theme/input-method 等本项目 ID 使用稳定 ASCII 标识；建议 `[a-z0-9][a-z0-9._-]{0,63}`。显示名与 ID 分离，显示名可以 Unicode。
- 配置里的普通视觉资源不得接受绝对路径、UNC、`..`、ADS、device path、URL。
- 保存固定为：typed model → 全量 validate → 同目录临时文件 → flush/close → 原子 replace。失败时原文件保持不变。
- 不维护“旧格式 last-known-good schema”。配置损坏时用 compiled default 继续运行；诊断指出文件/行列/字段。
- 不在 Registry 镜像 `config.toml` 的视觉值，也不在 Config GUI 私有缓存保存第二份 authoritative 设置。

### 13.9.11 Parser / library 决策

实现前先检查现有依赖是否已经有合格 TOML/JSON parser。若没有：

- 选择成熟、维护活跃、许可证适合、能在目标 MSVC/Win7 构建链工作的 TOML 1.0 parser；
- JSON 同样优先复用现有成熟 parser；
- **禁止自写 TOML、JSON、Unicode escape、数字 parser。**

Parser 只负责语法；项目 typed validation 负责 enum、范围、ID、路径、颜色、字体列表和主题权限。第三方 parser 更换不得改变配置语义。

### 13.9.12 单一 source of truth

```text
appearance/candidate visual override -> config.toml typed model
theme defaults                        -> active theme.toml typed model
Fcitx behavior/page/key binding       -> Fcitx Config API
package metadata                      -> manifest.json
installed package set                 -> packages.lock
Windows registration                  -> installer/register owner
```

GUI、日志、诊断页只能读取这些 owner，不维护影子副本。

### 13.9.13 配置器信息架构、Design System 与设置 ownership

普通设置按用户任务组织，而不是按内部进程/模块组织。v1.8 推荐一级导航：

```text
输入法
外观
快捷键
插件与扩展

────────
更新
诊断与修复
```

信息层级仍然只有 `Basic / Advanced / Diagnostics`，但它们是**渐进披露层级**，不是必须直接显示给用户的三个大标签页。

- `输入法`：当前 Fcitx input methods、启用/禁用、排序、默认项；使用卡片/列表，不暴露 TSF profile GUID。
- `外观`：production Candidate Preview、主题、System/Light/Dark、Auto/Horizontal/Vertical、字号、字体；Theme library 作为该页子区域，不作为平级一级导航。
- `快捷键`：面向用户任务组织；具体 Fcitx key binding 由 Fcitx Config API authoritative ownership。
- `插件与扩展`：普通用户显示友好 addon/input-method 信息；Advanced 提供 generic Fcitx metadata/config schema view，避免巨大硬编码映射。
- `更新`：channel、版本、检查/安装、previous-known-good 状态；网络/签名技术字段只在 Details。
- `诊断与修复`：状态、Health Check、日志、Repair、技术详情；不拆成 Diagnostics/Repair 两个一级页面。

**视觉实现规则：**

- WTL/Win32 负责 window/message/native integration；D2D/DWrite Settings Surface 负责主要视觉组件。
- 建立单一 `DesignTokens` 与可复用 `SettingRow/Section/Toggle/SegmentedControl/Slider/ThemeCard/InputMethodCard/Banner/StatusBadge/CandidatePreview`；禁止每页手工造近似控件。
- Config 自身 MUST 支持 System/Light/Dark/High Contrast。
- 不创建 `TSF`、`IPC`、`Renderer`、`Launcher` 等普通设置页面。
- 每个 setting 只有一个 authoritative owner 和一个主要 UI 入口。其他页面需要引用时使用导航/摘要，不复制第二个可编辑控件。
- `config.toml` 是 sparse override；普通用户不需要打开它。GUI 使用 picker、preview、enum、slider 等受约束控件生成合法配置。
- 主题、字体、颜色、横竖/Auto 布局等外观项默认 Live；Reset 的语义是删除 override，重新继承当前主题。
- `max_width`、`scroll_cell_width`、padding/gap 等 renderer 工程值不出现在 Basic；只有真实用户需求时进入高级外观。
- 不实现首次使用 wizard，除非未来出现无法从系统/默认值可靠得到的必填信息。
- 设置搜索第一版可以不实现，但 setting 应具有稳定 ID、显示名称、关键词和页面归属，避免未来为了搜索重构整个配置模型。
- Windows shell 只显示一个 `Fcitx5` TSF profile；Config 中切换 Rime/Mozc/拼音等不得触发 TSF profile 注册/注销。


## 13.10 现代软件工程实践：确定性、契约、小批次与可诊断性

本节只加入能实际减少返工、竞态、发布漂移和维护成本的工程实践。它们不是独立流程，而是现有 `build.ps1`、测试、CI、ADR、Release Gate 和 Codex Task Contract 的执行方式。

### 13.10.1 近 Hermetic / 可复现构建

本项目不承诺在所有 Windows SDK/MSVC toolchain、签名时间戳场景下达到理论上的 bit-for-bit hermetic build，但必须尽量消除“开发机偶然状态”。

- MSVC toolset、Windows SDK、CMake/生成器、WTL、第三方依赖、代码生成器必须 pin 到明确版本；Release 记录实际版本与 hash。
- `tools/bootstrap.ps1` 或等价入口负责检查/准备声明过的工具，不依赖开发者手工设置的 PATH、注册表偶然状态、IDE 插件或未声明 SDK。
- 使用 `CMakePresets.json` / `CMakeUserPresets.json` 的职责分离：仓库提交共享 preset；个人机器差异只进入不提交的 user preset。
- CI 从 clean checkout 构建；cache 只作为加速，不是依赖来源。删除 cache 后仍必须能完整构建。
- 构建默认禁止访问公网拉取 `latest`。确需下载的工具/依赖必须版本固定并验证 hash/signature。
- Release metadata 至少记录 source commit、submodule/dependency lock、toolchain versions、build flags、target architecture 与最终 artifact hash。
- 对无法做到 byte-for-byte reproducible 的 PE/签名产物，不伪称“可复现”；目标是 **reproducible inputs + traceable lineage + same-artifact promotion**。

### 13.10.2 边界契约测试（Contract Testing）

以下边界必须有当前版本契约测试：

```text
TSF client ↔ Engine IPC
Engine ↔ CandidateModel / Presentation
Config GUI ↔ typed Control API
config.toml/theme.toml ↔ typed model
Package manifest ↔ package service
Launcher commands ↔ launcher state machine
```

要求：

- 契约测试验证字段、长度、enum、错误码、deadline、身份/epoch/revision 与非法输入行为，而不只验证 happy path。
- client/server 的契约 fixture 必须来自同一个 authoritative schema/model；禁止两边手写“看起来一样”的 magic numbers/enum 表。
- breaking change 直接同步修改 producer、consumer、fixtures 和 tests；旧 contract fixture 删除，不建立 v1/v2 长期双栈。
- Control API 的 Config smoke 必须验证真实调用与错误映射；优先用 `--self-test`/mock service 等 headless 路径，不把“GUI 按钮能点击”当作契约测试。

### 13.10.3 Property-based / Model-based Testing

对状态空间大、例子测试容易漏边界的模块，优先增加属性/模型测试；不要求全项目使用。

优先目标：

- IPC codec：`decode(encode(x)) == x`（合法域）、任意 bytes 不 crash/OOB/无限分配。
- config/theme merge：相同输入确定得到相同 snapshot；Reset 删除 override 后恢复下一层继承；High Contrast 始终拥有最高视觉优先级。
- CandidateModel：selected index 永远在合法范围；revision 单调规则不被旧消息逆转；候选顺序不会被 renderer 改写。
- launcher state machine：非法状态转移被拒绝；`Updating/UserStopped/Uninstalling` 不会被 TSF 按键错误拉起 engine。
- package transaction：任一阶段失败后 active install 保持旧完整版本或新完整版本，不出现半激活状态。

Model-based test 使用一个极小 reference model 描述状态转移，不复制生产实现的内部算法，否则测试与实现会一起错。

### 13.10.4 确定性并发、虚拟时间与取消

- 新的 timeout/backoff/debounce/idle flush 逻辑 SHOULD 通过可注入 clock/timer abstraction 测试；测试不得真实等待几秒钟。
- IPC timeout、launcher backoff、candidate debounce 等测试使用 fake/virtual clock 推进时间。
- 并发测试优先用 event/barrier/latch 明确制造交错；禁止依靠 `Sleep(50)` “希望线程刚好跑到那里”。
- cancellation、shutdown、timeout 后的 late response 都必须有明确 owner 和终态。
- 发现竞态必须保留最小可重放 schedule/fixture；不能用增加 retry 次数掩盖。

### 13.10.5 故障注入与 Crash-Consistency Testing

只对真实边界做小规模、确定性的 fault injection，不建设大型 chaos 平台。

至少可注入：

```text
pipe connect/read/write timeout
partial/truncated IPC frame
engine exits between request/response
ui.exe exits during composition
config temp-write/flush/replace failure
package download/verify/stage/activate failure
invalid signature/hash
out-of-disk / access denied（可可靠模拟时）
D2D device loss
```

验收重点不是“错误被捕获”，而是**失败后的系统状态**：宿主不冻结、输入 fail-open、旧 revision 不复活、配置不损坏、active install 不半更新、不会 crash-loop。

### 13.10.6 Schema / Codegen 单一来源

- 一个协议/manifest/config enum 只能有一个 authoritative definition。
- 如果现有成熟工具能够从 schema 安全生成 C++/测试 fixture，可使用 codegen；generated code 与手写代码分目录，生成器版本固定，CI 检查“重新生成后 git diff 为空”。
- 如果引入 codegen 比共享 typed definition 更复杂，则不为了“现代化”强行 codegen；优先最简单的单一来源。
- WTL Config 不维护第二套 enum/schema；通过 Control API 获得 typed metadata 或消费构建生成的只读描述。
- 生成文件禁止手工修改；修改 source schema 后重新生成并更新测试。

### 13.10.7 Trunk-based、小批次与主干可用

个人项目默认采用轻量 trunk-based workflow：

- `main` 始终应处于可构建、核心测试通过的状态。
- feature/fix branch SHOULD 短命、小批次；完成当前 vertical slice 后尽快合并，不维持长期 `develop` / `integration` 分支。
- 一个 change 尽量只承担一个可说明的目的；安全关键重构与无关 UI 改动不要混在同一个 PR/commit batch。
- 不使用长期 feature flag 隐藏半成品架构。只有当前同时受支持的真实产品模式才能保留 flag。
- 大功能通过多个仍保持主链正确的垂直切片交付，而不是创建数月后一次性合并的大分支。

个人项目不强制模拟多人 PR 审批；即使直接提交，也使用相同 affected tests / Gate Impact / self-review 规则。

### 13.10.8 本地可观测性与诊断，不等于遥测

默认仍然无遥测。可观测性首先服务本机故障定位：

- 结构化日志使用稳定 event id / component / severity / correlation id；禁止通过解析自然语言日志驱动业务逻辑。
- IPC request_id、engine_epoch、context_id 可用于关联事件，但 S0/S1 输入内容不得进入 production 日志。
- 每个后台组件提供最小 `--version` / health/status 或等价 Control API。
- Diagnostics 可生成**用户主动请求的本地诊断包**：版本、组件状态、匿名化配置摘要、安装/签名状态、性能计数、日志；默认不包含 raw key/preedit/candidate/commit/user dictionary。
- 诊断包生成前列出包含内容；不自动上传。
- crash/incident 修复后，优先新增 regression test/event，而不是增加更多常驻 logging。

### 13.10.9 版本与发布纪律

- Core 同一发行集合中的 TSF/engine/ui/launcher/config/package 使用一个产品版本，避免组件版本组合爆炸。
- `protocol_version`、`format_version`、`addon_abi` 是独立技术版本，不能拿产品版本替代。
- 本项目不承诺内部协议/配置 schema 向后兼容；版本不匹配按现有规则明确拒绝。
- Stable/Beta/Nightly 的 identity、CLSID/Profile/IPC namespace 若允许并存必须明确分离。
- 每个 Stable Release 有简短 changelog：用户可见变化、兼容性变化、安全修复、已知问题；不把 commit dump 当 release notes。
- 发布后发现严重回归时按 bad-release runbook 处理，不在原 tag 下静默替换二进制。

### 13.10.10 依赖更新与技术债纪律

- Dependabot/等价机器人可以提出更新，但**不盲目自动合并**进入 release；先看 changelog、license、min Windows、ABI/API、二进制体积与受影响测试。
- 安全更新按风险及时处理；普通依赖采用固定节奏批量 review，避免每天产生噪声 PR。
- 未使用依赖、过期 feature flag、死代码在确认无当前需求后直接删除。
- 技术债必须写成具体可验证问题，例如“`OnTestKeyDown` 重复路径缺 regression”，不接受“以后重构一下”这种无边界 TODO。
- TODO/FIXME 若涉及 security/correctness/release blocker，应关联 issue/ADR；普通局部 TODO 不强制建流程。

### 13.10.11 格式化、静态规则与工程入口

- 仓库提供 `.editorconfig`；C/C++ 使用固定版本/规则的 `clang-format`，项目自有代码统一格式，vendor/generated 不做无意义重排。
- `clang-tidy` / `/analyze` 只启用项目当前能稳定维护的规则集；新增规则先通过 Good/Bad Case 验证，禁止一次打开数百条噪声 warning。
- 公共 build/test/lint/package/release 操作只能从少量顶层入口调用，底层工具命令可以存在但文档不要求贡献者记忆十几套调用方式。
- 新开发机目标：clone 后运行 bootstrap/check + `build.ps1 dev/test` 即能得到确定结果；不得依赖“作者电脑上已经装过某东西”。

### 13.10.12 不采用的伪指标与过度工程

当前**不设置**以下硬 KPI：

- 全仓代码覆盖率必须 90%/100%；
- 每个函数必须有 unit test；
- mutation score 必须达到某百分比；
- 所有模块都必须 property-based test；
- 每个 commit 都跑完整 Win7 + Office + 游戏 + installer 矩阵；
- 为个人项目增加强制双人审批、CAB、复杂 release train。

覆盖率可以作为“哪里完全没测”的观察信号，但不能替代边界、状态机、失败语义和真实 E2E 的风险判断。


### 13.10.13 跨语言 contract 与 differential migration

- 同一 IPC/wire/package/repository contract 只有一个语言无关 source of truth；C++/Rust parser 可以各自实现，但不得各自定义字段默认值、宽松解析或 canonicalization。
- Rust 迁移必须保存 C++ baseline vectors：正常包、损坏/截断、签名错误、路径 hostile corpus、sequence rollback/corruption、child process 大输出/挂死、launcher crash sequence。
- Differential test 只用于迁移期证明行为一致；cutover 后保留 golden/contract corpus，删除旧 C++ implementation 与永久 dual-runtime switch。
- 如果旧 C++ 行为本身是 bug，先修改 contract + C++ + regression，再让 Rust 对齐**新正确行为**；禁止把 bug compatibility 当迁移成功。

# 14. 分阶段执行计划与验收标准

| **执行策略　每个阶段完成后先达到验收标准再进入下一阶段。Codex 不得主动跨阶段堆功能。个人项目允许阶段内快速迭代，但架构边界不能临时绕过。** |
|--------------------------------------------------------------------------------------------------------------------------------------------|

## 14.0 v1.7 现有实现 Stabilization Sprint（优先于新增 Phase 功能）

当前仓库已经跨越多个历史 Phase；因此 v1.7 **不是要求把代码删除回 Phase 1 再重做**。先对冻结快照完成下列有序修复，全部以 regression/contract 证明，再继续产品功能或 Rust R1：

1. UILess PresentationPolicy 跨 TSF→Engine/UI 生效；新增 `REG-UILESS-001`。
2. CandidateModel A→B→A context/composition identity 修正；新增 `REG-CTX-002`。
3. KeyEvent protocol 扩展为多布局/AltGr/dead-key 可表达模型；新增 `REG-KEY-INTL-001`，protocol breaking change 同批更新双方，不留 v7/v8 双栈。
4. 单一 Windows `Fcitx5` TSF profile：固定 profile GUID/显示名/icon；内部 Fcitx IM/group + BCP-47/content locale 动态切换，不增加第二个 Windows profile；新增 `REG-PROFILE-001` 与 `REG-BRAND-001`。
5. Fcitx surrounding/delete/forward capability；新增 `REG-FCITX-CAP-001`。
6. 删除 synthetic `n` warmup 或改为无副作用 preload；新增 `REG-WARMUP-001`。
7. Launcher 最小 crash ledger 持久化；新增 `REG-LAUNCHER-LEDGER-001`。
8. Config/Control 统一 process-execution primitive；新增 `REG-PROC-PIPE-001`。
9. repository max-sequence 原子/抗损坏状态；新增 `REG-REPO-STATE-001`。
10. pipe peer handle-based executable identity；新增 `REG-PEER-ID-001`。
11. Installer machine/user owner SID 与跨账户 UAC uninstall；新增 `REG-INSTALL-UAC-001`。
12. register/bootstrap validated artifact + side-effect timeout containment；加入 installer/repair E2E。
13. Candidate locale/config-generation reload；加入非 zh-CN internal engine/content-locale visual/golden test。
14. Windows hostile path corpus；新增 `REG-PKG-WINPATH-001`。
15. Win7 VM 与 LoL/Vanguard/Office/Chrome/VS Code 真实宿主证据按对应 Gate 落地。

16. Config Visual System：建立 DesignTokens + reusable D2D Settings components；Config 自身 System/Light/Dark/High Contrast；导航收敛为用户任务，不再扩散 Theme/Repair 等平级工程页面；新增 `REG-CONFIG-VISUAL-001`。
17. Appearance Live Preview：production Candidate renderer 的 inline/live preview；外观 Live 生效；工程参数渐进披露；新增 `REG-CONFIG-LIVE-001`。
18. Candidate UX：加入 `auto/horizontal/vertical` 与 composition-scope width hysteresis，保留现有 scroll viewport/label alignment；新增 `REG-CAND-STABLE-001` / `REG-CAND-AUTO-001`。
19. Branding：统一 Product Icon、固定语言中立 TSF Icon 与 Windows 显示名 `Fcitx5`；验证多分辨率 ICO、Light/Dark/HC 和 shell picker，不把 engine logo/语言字符写进 TSF identity；新增 `REG-BRAND-001`。

**停止条件：**上述当前可达缺陷/契约完成并有证据后停止 stabilization，不借机重写无关模块。Rust R1 从第 14 项 corpus 和 package contract 完成之后开始；R2 必须等第 7–8 项语义先在 C++ 基线通过。

| **阶段**                                       | **Codex 任务** | **完成标准** |
|------------------------------------------------|----------------|----------------|
| Phase 0：参考基线与源码审计 | 固定参考仓库/commit；建立 Reference Matrix。Fcitx 语义以 `fcitx/fcitx5` core 与 addon upstream 为权威；Windows 侧审计 `windows-chewing-tsf` 的 TSF/UILess/out-of-process UI/build/test，并用 `win-mcbopomofo` 校对 thin client/server 边界；Weasel 只建立兼容病例索引。对每个 subsystem 记录 Keep / Rewrite / Do-not-inherit。 | `reference-baseline.md` 入仓；明确“Fcitx upstream 语义权威、Chewing/WinMcBopomofo/Weasel 为 Windows 病例教材、现有 Windows port 不作架构权威”；每个关键设计能指出首选参考和禁止继承项；不得仅凭 README 或旧记忆实现，也不得平均通读所有参考仓。 |
| Phase 1A：最小构建基线 | 建立 CMake、x86/x64 toolchain、统一 `build.ps1`、`.editorconfig`/clang-format、CMakePresets、clean-build/bootstrap；建立最小 PR CI 与 warnings/secret/license/SCA 基线；把现有 SR/NFR/SC 需求作为约束登记，但**不预建尚无真实 target 的 fuzz/property/model framework**。 | clean checkout 可构建 x86/x64 C++ Core 空壳；`build.ps1 dev/test` 有确定结果；基础 dependency/license/secret checks 可运行。完成即进入 1B，不继续装饰 CI。 |
| Phase 1B：First Vertical Slice | 以 `windows-chewing-tsf` 为 TSF 主参考、`win-mcbopomofo` 为 thin client/server 次参考，复用成熟 TSF 最小骨架，建立最小 versioned IPC 与 mock engine，只实现足够完成一次端到端文本输入/commit 的字段和行为；先以 Notepad 跑通真实 `TSF → IPC → mock engine → Commit`。 | x86/x64 TSF 可注册/激活；Notepad 中最小输入/commit 成功；engine 不存在/失败时宿主不崩、不无限等待；主链已有第一个可自动回归的 E2E。 |
| Phase 2：IPC / KeyEvent + Launcher 硬化 | 在已工作的主链上固定 request/response、epoch/context/composition/revision、SID+Session、DACL、peer identity、bounded I/O；把 Windows KeyEvent 从简单 VK 扩展为能表达 physical/logical key、scan/extended、release、layout/AltGr/dead-key 的语言中立 contract。Launcher 先用 C++ 修正并固化 crash ledger/backoff/SafeMode 语义；**R2 此时不得先重写。** | 错误/截断协议安全拒绝；32-client stress/idle 无积压；timeout/late work/backoff 确定；AltGr/dead-key/layout golden 通过；peer identity 为 handle-based；launcher 自身重启不能清零 crash storm；`25 ms` 仍仅为经测量固化的 bounded budget。 |
| Phase 3：Fcitx5 / libime + 通用 InputContext / 单一 TSF Profile | 以 `fcitx/fcitx5` core 与 addon upstream 为语义权威，建立 thin C++ Fcitx adapter + Rust Engine Product Core；补 surrounding text、delete surrounding、atomic replacement、candidate action、forward key 等真实 engine/addon 所需 capability；Windows 侧维持单一稳定 `Fcitx5` TSF profile，Fcitx input method/group 与 BCP-47/content locale 作为内部动态状态。通用 warmup 禁止伪造用户文本键。 | Notepad composition/commit；多 context 不串状态；engine restart epoch 正确；Rime/拼音及至少一个非中文 engine 可在同一 Windows `Fcitx5` profile 内切换，DWrite locale/输入语义随当前 Fcitx metadata 正确变化；surrounding/delete/replace/action/forward capability contract tests 通过；warmup 无副作用。 |
| Phase 4：CandidateModel + 独立 C++ D2D UI + UILess | 保留 C++ Win32/D2D/DWrite renderer；把 UILess `popup_allowed/presentation_mode` 做成跨进程 context policy；修正 `(epoch, context, composition, revision)` 作用域和 A→B→A；DWrite locale 跟 active Fcitx input method/content locale/文本语言走；配置 reload 由 generation/broadcast 驱动、文件时间仅低频 fallback。继续 DPI、多屏、device loss、无障碍、theme/golden。**不因 Rust 可写 D2D 而重写 renderer。** | Word/UILess host 返回 `show=false` 时独立 popup 不显示而 UIElement/a11y 仍可用；A→B→A 不误丢合法 snapshot；非 zh-CN internal engine/content locale 字体/locale 正确；125/150/200%/多屏/device-loss/High Contrast 通过；隐藏时无持续 render loop/输入频率磁盘 polling。 |
| **Milestone D0.1：First Usable / Dogfood Build** | **停止增加平台能力，打出一个供维护者日常使用的 Developer Preview。** 允许开发安装脚本/手工安装；不要求插件商店、自动更新、Plum GUI、winget/Chocolatey 或完整 package repository。把 League of Legends + Vanguard 提前作为一等真实宿主：只验证正规 TSF/Windows 系统兼容路径，不为游戏新增 Hook/SendInput/注入/规避。 | 能在日常 Windows 环境持续使用：x86/x64、真实 Fcitx5、真实候选 UI、基本字体/主题/横竖排；Notepad/Word/Chrome/VS Code 可输入；LoL 游戏内聊天至少完成 composition/candidate/commit、Alt+Tab 恢复和控制键 passthrough smoke，Vanguard 正常且无特殊规避代码；engine/UI 故障不拖垮宿主。达到后先 dogfood 和修真实问题，再进入 Phase 5。 |
| Phase 5：可靠性、安全与兼容 + R2 前置 | 基于真实宿主补 Weasel/Rabbit/Moqi/WindInput/host-side regression；完成 launcher crash ledger、Control/Config authoritative process execution、peer identity、register/bootstrap side-effect containment、LoL/Vanguard、老游戏、RDP、Win7 Legacy VM。全部 C++ 语义稳定后，才允许按 R2 对 launcher/control/diagnostics 做 side-by-side Rust PoC。 | engine/UI crash 不杀宿主；launcher 自身重启不能绕过 SafeMode；64KiB/1MiB child output/hang/cancel 测试通过；Win7 VM + Office/terminal/老 x86/LoL 核心矩阵通过；R2 若启动必须 differential 通过且未引入 anti-cheat/Legacy 回归。 |
| Phase 6：Config UI/UX、安装与 i18n | 保持 WTL/Win32 宿主并引入**小型、产品专用** D2D/DWrite Settings Component System；Config 只走 typed API；production Candidate Preview inline/live；System/Light/Dark/High Contrast；按“输入法/外观/快捷键/插件与扩展/更新/诊断与修复”组织。自绘交互控件同步实现 keyboard/focus/UI Automation；无法做到等价 a11y 的控件退回 native HWND。完成 Penguin-first Product Icon、固定 micro-penguin TSF icon/name、稳定 AppUserModelID 与统一 resource pipeline。Inno/Control 明确 per-machine program/system registration 与 per-user startup/session/config owner。 | Config 不再呈现传统属性页/裸表单观感；Design Tokens/组件复用、DPI/keyboard/UIA/Narrator/NVDA/theme smoke 通过；Windows 输入法列表只出现一个名为 `Fcitx5` 的固定 profile/icon；Config 是正常任务栏应用，Candidate/Launcher/Engine/后台 helpers 无额外 taskbar/Alt+Tab surface；默认无额外常驻 tray icon；切换内部 engine 不新增 profile；跨账户 UAC uninstall 正确。 |
| Phase 7：Package / Addon / Provider + Rust R1 | 先以现有 C++ package/repository/update 实现和 hostile corpus 固定正确行为，再为 Modern lane 逐组件实现 Rust R1：package core/repository/updater/downloader/provider，deployer 仅在权限/Legacy toolchain 证据充分时迁。加入 Cargo lock/advisory/license/source/SBOM gates；同一 wire/manifest/path corpus 做 C++↔Rust differential。实现 TSF in-use DLL 的 generation-specific staging/activation/drain，不把 Rust 迁移与协议兼容混在一起。 | staging+verify+atomic activation/previous-known-good 不回退；DOS device/case/reparse/path corpus、签名/hash/rollback/corrupt-sequence、archive budgets fuzz 通过；`REG-UPDATE-TSF-001/002` 通过；Rust artifact 在 clean runner/package/update smoke 通过；cutover 后删除旧 C++ authoritative implementation；Legacy 未证明 Rust Win7 时继续其 C++ lineage。 |
| Phase 8：分发与公开发布 | Stable/Beta/Nightly identity；签名、统一 C++/Cargo SBOM、release gate、Chocolatey/winget；每条 Modern/Legacy artifact lineage Build Once。验证 mixed-toolchain provenance、previous-known-good、key rotation/revocation；不得在 signing job 重编 C++ 或 Rust。 | 最终发布 bytes 的 hash/signature/SBOM/provenance 对得上 source commit + locked MSVC/MSYS2/Cargo toolchains；坏 release 可整套回退；系统包管理器与 builtin updater 不抢 Core 更新权；Rust/C++ release 依赖与许可证均无 inventory blind spot。 |

## 14.1 Phase 0–4 / Dogfood 前严格禁止

- 不要先做漂亮配置器。

- 不要先做插件商店或 Rime/Plum GUI。

- 不要先做 WebView2 高级皮肤。

- 不要用 Hook/SendInput 绕过 TSF 难点以“先跑起来”。

- 不要把 Fcitx/libime 链接进 TSF DLL。

- 不要为了 Win7 把现代 API/工具链全部禁用；Win7 是 Legacy lane，Windows 10/11 是 Modern lane，同源代码按 capability/toolchain target 验证，不复制业务实现。

# 15. Codex 执行协议

## 15.0 先确定任务授权

Codex 在读取/执行用户请求后先内部归类为 RESEARCH、REVIEW、CHANGE 或 RELEASE，并据此限制动作范围。不得因为发现邻近问题自动把 RESEARCH/REVIEW 变成 CHANGE，也不得因为 CHANGE 涉及构建文件就自动变成 RELEASE。

**Smallest correct result = 用户明确要求 + 当前正确结果不可缺少的后果。** “改动最小”不是目标；“范围有证据”才是目标。

## 15.1 开始任何实现前

0\. 明确当前 Task Contract（RESEARCH / REVIEW / CHANGE / RELEASE），先确定允许做什么、完成标准是什么。

1\. 先查“Reference Implementation Matrix”和 0.4：对当前问题先看官方规范，再读当前 Phase 的主教材，最后才读专项参考与本仓实现；不能只凭模型记忆重写，也不得平均通读所有仓库。对参考代码的 commit、许可证、读取范围和可借/不可借结论必须记录。

2. 检查当前仓库现状，不覆盖已有正确实现；先列出与本规格冲突的地方。

3. 核对当前 `fcitx/fcitx5`、各 addon upstream、Windows TSF 文档，以及只作为构建/依赖清单参考的 `fcitx5-plugins`；外部 API/ABI 不凭旧记忆硬写，不使用现有 Windows port 作为 Engine/Package 架构依据。

4. 先搜索本仓现有依赖、Fcitx5/upstream API 与 Windows API 是否已经提供所需能力；确认不能满足后才考虑新增依赖或自研。

5. 只选择当前 Phase 所需的最小依赖。新增依赖前记录用途、维护状态、许可证、Modern/Legacy min_os、体积、风险与“为什么现有依赖不够”；C++ GUI 依赖需记录 MSVC/ATL/WTL 约束。若任务属于 R1/R2 Rust 迁移，还必须记录 target tier/toolchain distribution/MSRV、Cargo dependency graph、Legacy 影响与 cutover/rollback 计划。

6. 为当前变更明确至少一个可自动验证的完成标准。任何额外动作先过 Stop Ladder；没有 reachable evidence 就不加入任务。

## 15.2 每个实现批次

1. 做 smallest correct vertical slice：把当前正确结果所需的调用方/协议两端/测试改完整，但不同时重构无关模块；优先让端到端主链保持可运行。

2. 不得提交已知会被未来目标架构替换的“临时实现”。功能可以少，但已经实现的路径必须沿最终架构。

3. 同步增加/更新单元、集成或回归测试；兼容 bug 修复必须留下测试。

4. 运行**受影响范围**的构建、测试和安全检查；parser/protocol 变更运行 fuzz corpus。只有当前变更/Phase/Release 明确要求时才扩到完整矩阵。

5. 更新必要的 ADR / threat-model / package schema 文档；小 UI 改动不需要额外安全文档。

6. 提交信息写清楚“为什么”和风险，不只写“fix”。

7. 对照 NFR 检查本批次是否增加热路径工作、常驻内存/线程、IPC payload、二进制体积或新依赖；有明显增加时记录 benchmark/理由。

8. 若本批次增加或修改用户可见设置/流程，执行 Cognitive Load Review：用户是否被要求理解新的内部概念、是否出现重复设置入口、是否能自动推断、是否增加不必要确认/弹窗、是否违反 Basic/Advanced/Diagnostics 三层结构。

9. 对当前 diff 做 Gate Impact 判断：是否修改 TSF/IPC/ACL/network/signing/installer/package parser/CI workflow/WTL或其他 toolchain dependency/license/user-visible config。只触发与当前风险相符的额外门禁；Release-critical 变化不得混在普通 UI PR 中静默通过。

10. 若修改边界协议/状态机/timeout/并发/config merge/package transaction，检查是否应增加 contract/property/model/fault-injection test；若测试依赖真实 Sleep、真实公网、真实用户目录或偶然线程时序，优先改成可注入/确定性 fixture。

## 15.3 Codex 必须停止并报告的情况

- 需要把复杂逻辑或第三方 DLL 加进 TSF 宿主进程才能继续。

- 需要让 engine 获得公网访问权限才能实现某功能。

- 发现实现试图兼容旧 IPC/旧 schema/旧内部 API，而不是删除旧路径并要求同版组件。

- 需要更改不可再生用户数据格式，却没有先证明变更必要、定义显式导出/备份策略或避免格式变更。

- 某依赖无法确认许可证、来源或最小 Windows，却准备进入默认发行包。

- 为解决一个应用兼容问题必须引入全局 Hook/SendInput 主路径。

- 需要在 Core runtime 新增 compatibility shim、自动 migration、旧协议 parser、deprecated wrapper 或“先这样以后再换”的临时架构才能继续。独立、显式、一次性的离线用户配置转换若符合 0.1/13.9.3 的边界，不属于此停止条件。

- 需要新增通用 abstraction/config/provider/manager，但当前只有一个真实调用场景且现有依赖已经能完成需求。

- 想做的额外工作无法通过 Stop Ladder：没有用户要求、没有当前验收必要性、也没有可达代码/数据/平台证据。

- 想新增机器化 guard，但目前只能依赖主观架构判断，无法给出稳定的 Bad Case + 最近 Good Case。

- 想新增普通用户设置，但它只是暴露内部实现参数、系统已有可靠默认、没有真实用户偏好/兼容证据，或会要求用户理解 TSF/IPC/ABI/engine 等内部概念。

- Release/security workflow 需要把 signing secret 暴露给 PR/fork、需要使用漂移的第三方 action/latest toolchain、或需要在通过测试后重新从源码构建发布二进制。

- 任一正式构建步骤只能靠 IDE/人工点击/UI Automation、开发机隐含状态、在线拉取 latest 或无界 retry 才能成功。

- 想把 TSF DLL、Fcitx Engine、现有 Candidate UI 或 WTL Config 改成 Rust，只因为语言偏好/统一技术栈/提高 Rust 比例，而没有 13.6.2 所要求的真实安全、维护或产品证据。

- 当前验收已经满足，只剩“再搜一点、再重构一点、再跑一遍全部测试、顺便清理一下”等无新证据工作；此时应结束当前 batch。

## 15.4 完成判定：Enough Evidence → Stop

一个 batch 完成只需要证明：

1. 当前 Task Contract 没有越权；
2. 明确验收标准已满足；
3. 受影响测试通过，必要的安全/性能检查有结果；
4. 没有留下当前任务导致的已知 broken caller / 半套协议 / 未处理失败路径；
5. 新依赖、新权限、新信任边界、新热路径成本如果存在，都有当前必要性证据。

满足后停止。不要为了制造“更完整的工作痕迹”增加第二轮无目标审计或无关代码。

## 15.5 个人项目的完成记录

| **低负担做法　不需要写长报告。每个 milestone 保留：一个 issue/notes 文件、对应 commit、测试结果、需要时一条 ADR。一个人自己完成 review checklist 即可。** |
|-----------------------------------------------------------------------------------------------------------------------------------------------------------|

# 附录 A. 威胁模型基线

| **资产/边界**                  | **主要威胁**                                   | **最低缓解**                                                                          |
|--------------------------------|------------------------------------------------|---------------------------------------------------------------------------------------|
| Keystrokes / preedit           | 信息泄露                                       | 输入平面禁网；日志禁止原文；敏感 context 不学习                                       |
| User dictionary                | 本地窃取/云同步泄露                            | 用户 ACL、默认本地目录、不自动同步、必要时 DPAPI                                      |
| TSF ↔ Engine IPC               | 冒充/篡改/DoS                                  | SID ACL、session namespace、版本/长度/schema、timeout                                 |
| Native Addon                   | 任意代码/泄露/崩溃                             | 信任等级、签名/来源/ABI、engine-only、Safe Mode                                       |
| Update channel                 | 供应链/版本回退攻击                                | signed manifest、hash、key rotation、版本策略、atomic activation                      |
| Theme/Data package             | parser DoS/path traversal                      | schema/大小限制、路径规范化、无任意代码                                               |
| Crash dump/log                 | 无意泄露输入                                   | 默认最小诊断、不自动上传、显式用户操作                                                |
| Portable registration          | 路径悬挂/多副本冲突                            | 路径检查、repair、single active frontend                                              |
| Game / anti-cheat boundary     | 合法 IME 被阻止/误报；兼容修复误引入外挂式行为 | TSF-only、稳定签名二进制；禁止 Hook/注入/内存读写/驱动/混淆；被阻止时安全降级，不绕过 |
| Host App ↔ TSF DLL             | 内存破坏/异常/阻塞拖垮宿主                     | 极薄 TSF、边界封口、bounded wait、Verifier/ASan、DLL path audit                       |
| TSF ↔ Launcher/Engine identity | 本地假冒 pipe server / 跨 session 串线         | SID+Session namespace、DACL、PID/path/签名验证                                        |
| SYSTEM/LogonUI activation      | 错误启动用户 engine/数据泄露                   | 识别安全桌面/系统上下文，不 spawn normal user engine                                  |
| Candidate/UI process           | UI 卡死阻塞输入/无障碍丢失                     | 独立 ui.exe、presentation 与 input 解耦、UILess 表达                              |
| CI / Release workflow          | secret 泄露、恶意 Action、artifact 替换、重编译漂移 | least privilege、immutable pins、PR 无签名秘密、Build Once、protected signing |
| C++/WTL toolchain             | 版本漂移、header/dependency 供应链、min_os 破坏、不可追溯 | pinned toolsets/dependencies、inventory、Legacy min_os CI、clean-runner build |

# 附录 B. 决策记录与禁止事项

| **Decision**       | **当前选择**                                                                    | **重新评估触发条件**                                                  |
|--------------------|---------------------------------------------------------------------------------|-----------------------------------------------------------------------|
| Windows Frontend   | 正式 TSF                                                                        | 仅在 Microsoft 平台模型发生根本变化时                                 |
| Candidate Renderer | 独立 ui.exe：C++ / Win32 / Direct2D / DirectWrite；TSF 仅提供 UIElement/布局桥接 | 仅在 D2D/DWrite 出现无法解决的兼容/性能/无障碍问题，或 Legacy lane 结束且 Rust PoC 有明确收益时重新评估 |
| Config UI          | C++ + WTL/ATL + Win32（Phase 6 后进入）                                          | WTL 无法满足已验证的产品/无障碍需求，且替换收益明显高于局部扩展成本    |
| Core Update        | 版本目录 + 事务切换                                                             | 未来统一系统包管理有更强原子机制                                      |
| Plugin API         | Fcitx addon API                                                                 | upstream 明确提供新的跨平台 ABI                                       |
| Portable           | Self-contained + TSF 注册                                                       | Windows 提供真正免注册 IME 模型                                       |
| Win7               | Legacy compatibility lane；同源代码 + pinned Legacy toolchains                 | 若安全可测的维护成本不可接受，则明确结束支持并删除 lane，不保留半支持 |
| IPC                | versioned binary IPC v2 + request_id/epoch + bounded timeout + peer attestation | 只有在上游提供成熟跨平台 IPC abstraction 时重新评估                   |
| Engine lifecycle   | per-user/session launcher + warmup + backoff + SafeMode                         | Windows 提供更合适的官方 per-user text-service host 模型              |

- 外观默认跟随系统：System / Light / Dark；High Contrast 优先级最高。
- 字体按 UI / Candidate / Annotation / Monospace 四类 surface 管理，Candidate 字体缺 glyph 必须交给 DirectWrite/system fallback。
- ICO/PNG 为基础视觉资源；SVG 为可选矢量资源，不进入 Win7 候选热路径硬依赖。
- Theme/字体/图标故障不能影响输入主链；坏资源回退到内置默认。
- 人因工程采用“低打扰、可预测、保护肌肉记忆、错误便宜恢复、Consistency beats cleverness”。

## 明确禁止

- 禁止为 Light/Dark 再造按时间、地理位置、日落计算的自动切换系统。
- 禁止主题执行 Lua/JS/WASM/DLL/动态脚本、访问网络或从远程 URL 拉取 Candidate 资源。
- 禁止为了字体偏好牺牲 glyph 可显示性；首选字体缺字必须 fallback。
- 禁止把每个窗口/按钮做成独立字体、颜色、字号配置项。
- 禁止动画 GIF/APNG/animated SVG 进入当前主题能力。
- 禁止依赖图标或颜色单独表达关键状态。

- 把 libime/Rime/Lua/主题解析放进 tsf.dll。

- 使用全局键盘 Hook + SendInput 作为默认输入实现。

- 让主题直接执行 Lua/JS/WASM/DLL 等任意代码。

- 让 updater 或 package downloader 订阅实时输入数据。

- 直接原地覆盖正在被宿主加载的 TSF DLL。

- 把 user-data 和 program versions 放在同一个程序激活事务里。

- 无签名/无来源的 native addon 静默安装。

- 为了“兼容”而吞掉异常、无限重试或让输入线程无限等待。

# 附录 C. 上游参考清单

Codex 实现前应优先核对以下 upstream / 官方文档的当前状态。这里列的是参考方向，不把本文档中的版本记忆当作最新事实。

- **Fcitx5:** [<u>https://github.com/fcitx/fcitx5</u>](https://github.com/fcitx/fcitx5)

- **Fcitx5 cross-platform plugins:** [<u>https://github.com/fcitx-contrib/fcitx5-plugins</u>](https://github.com/fcitx-contrib/fcitx5-plugins) — 只作为跨平台插件构建与依赖清单参考，插件语义仍以各 addon upstream 为准。

- **Rust Win7 MSVC target:** [<u>https://doc.rust-lang.org/rustc/platform-support/win7-windows-msvc.html</u>](https://doc.rust-lang.org/rustc/platform-support/win7-windows-msvc.html)

- **Microsoft C++ supported platforms / Win7 targeting:** [<u>https://learn.microsoft.com/cpp/overview/supported-platforms-visual-cpp</u>](https://learn.microsoft.com/cpp/overview/supported-platforms-visual-cpp)

- **WTL:** [<u>https://wtl.sourceforge.io/</u>](https://wtl.sourceforge.io/)

- **windows-rs Direct2D bindings:** [<u>https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Graphics/Direct2D/</u>](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Graphics/Direct2D/)

- **Fcitx5 macOS:** [<u>https://github.com/fcitx/fcitx5-macos</u>](https://github.com/fcitx/fcitx5-macos)

- **Rime Weasel:** [<u>https://github.com/rime/weasel</u>](https://github.com/rime/weasel)

- **Rabbit / 玉兔毫:** [<u>https://github.com/rimeinn/rabbit</u>](https://github.com/rimeinn/rabbit)

- **Rime Plum / 东风破:** [<u>https://github.com/rime/plum</u>](https://github.com/rime/plum)

- **Microsoft TSF documentation:** [<u>https://learn.microsoft.com/windows/win32/tsf/text-services-framework</u>](https://learn.microsoft.com/windows/win32/tsf/text-services-framework)

- **Windows IME requirements:** [<u>https://learn.microsoft.com/windows/apps/develop/input/input-method-editor-requirements</u>](https://learn.microsoft.com/windows/apps/develop/input/input-method-editor-requirements)

- **Windows DLL security:** [<u>https://learn.microsoft.com/windows/win32/dlls/dynamic-link-library-security</u>](https://learn.microsoft.com/windows/win32/dlls/dynamic-link-library-security)

- **GitHub Actions secure use:** `https://docs.github.com/actions/reference/security/secure-use` — release/security workflow 的 immutable action pin、least privilege 等以当前官方文档为准。

- **NIST SSDF:** `https://csrc.nist.gov/projects/ssdf` — 仅作为把安全实践嵌入 SDLC 的框架参考，不把本项目变成合规流程。

- **OpenSSF Scorecard:** `https://github.com/ossf/scorecard` — 供应链 posture 自检参考，不以分数代替风险判断。

- Easy Anti-Cheat Player Support: https://www.easy.ac/support/articles

- BattlEye FAQ: https://www.battleye.com/support/faq/

## C.1 补充专项参考

- ImeModePersistence: https://github.com/mangokingTW/ImeModePersistence
- NVDA: https://github.com/nvaccess/nvda
- KeyMagic 3: https://github.com/thantthet/keymagic-3
- TypeDuck Windows: https://github.com/TypeDuck-HK/TypeDuck-Windows
- VKey: https://github.com/phatMT97/VKey
- ime-rs: https://github.com/saschanaz/ime-rs
- Chromium TSF implementation: https://chromium.googlesource.com/chromium/src/+/main/ui/base/ime/win/
- Firefox Windows TSF implementation: https://searchfox.org/mozilla-central/source/widget/windows
- Windows Terminal: https://github.com/microsoft/terminal
- Stop That Shit（AI/Codex scope-control 参考，不作为运行时依赖）: https://github.com/lennney/stop-that-shit

Microsoft DXUT/IME 行为以 Microsoft Learn / Windows classic DirectX sample 的当前版本为准；不要把第三方镜像当规范来源。

# 附录 D. Reference Implementation Matrix

**全局学习优先级：**Fcitx 语义以 `fcitx/fcitx5` 和各 addon upstream 为权威；Windows IME/TSF 经验以 Microsoft TSF 文档、`windows-chewing-tsf`、`win-mcbopomofo`、Weasel 等作为病例教材。现有 Windows port 不作为 Engine/Package/Candidate 架构权威。Codex 先按 Phase 读取必要部分，WindInput/Moqi/Rabbit/PIME/Cassotis 等只为当前问题补盲，不要求全仓通读。

**参考不是继承授权。** 每次借鉴都必须在 `reference-baseline.md` 或当前 Task Contract 记录：`问题 → 参考 commit/文件 → 借用的行为/模式 → 明确不继承的历史设计 → 许可证处理`。若只能通过复制大量 GPL 源码才能实现，应停止并重新设计边界。

| **问题域**                | **首选参考**                          | **次参考**          | **使用规则**                                                         |
|---------------------------|---------------------------------------|---------------------|----------------------------------------------------------------------|
| TSF 规范/基础语义         | Microsoft SampleIME                   | Weasel / Chewing    | 先按官方模型判断，再吸收实战 workaround。                            |
| Fcitx5 core/addon 语义    | `fcitx/fcitx5` + 各 addon upstream    | fcitx5-plugins 构建清单 | Fcitx object 操作留薄 C++ adapter；产品协议/状态/策略/IPC 迁 Rust；不重写 upstream addons。 |
| TSF 生命周期/Composition  | windows-chewing-tsf                    | Weasel / Khiin       | Chewing 为主教材；Weasel 主要补真实 callback/host 病例。              |
| IPC ACL / peer security   | windows-chewing-tsf                   | Moqi                | SID+Session、显式 ACL、server PID/path/signature。                   |
| IPC timeout/recovery      | WindInput                             | Moqi / Cassotis     | overlapped timeout、circuit breaker、stale response、bounded start。 |
| request sequence/异步通知 | Moqi                                  | WindInput           | 必须有 request_id/response_to；不能依赖 FIFO。                       |
| Launcher/crash lifecycle  | Moqi                                  | PIME / WindInput    | generation、restart storm、warmup、Updating/UserStopped。            |
| x86/x64 → 单 engine       | Cassotis                              | Moqi / PIME         | x86/x64 TSF 共用 native architecture engine。                        |
| Candidate out-of-process  | windows-chewing-tsf + win-mcbopomofo | WindInput / Rabbit  | Chewing 看现代 D2D/UILess，McBopomofo 看 Client/Server 边界；最终 UI 迁出宿主。 |
| Caret/focus/password      | Rabbit                                | Weasel              | 作为兼容测试库，不复制 Hook 输入路径。                               |
| 老游戏/IMM32/CUAS         | ImeStudy + Microsoft DXUT IME         | Yong / Weasel       | 仍以 TSF 为 authoritative，不引入隐藏 Hook 兼容路径。                |
| 内部多语言/单一 TSF Profile | Keyman / NavilIME                     | Microsoft SampleIME | 避免 zh-CN 单一假设。                                                |
| 构建/签名/发布            | windows-chewing-tsf                   | Moqi / PIME         | 优先学习其完整 Windows 产品构建/回归纪律；只采用当前风险所需部分。    |
| 宿主端 TSF                | Chromium / Firefox / Windows Terminal | 真实 E2E            | 问题必须同时检查 host TextStore/IME handler。                        |

## D.1 新增专项首选参考

| 问题域 | 首选参考 | 次参考 | 使用规则 |
|---|---|---|---|
| IMM32/CUAS/老游戏状态桥接 | ImeStudy + ImeModePersistence + Microsoft DXUT IME | Weasel/Yong | 研究系统兼容层与宿主行为，不引入隐藏 Hook 兼容路径。 |
| UILess / 无障碍 | windows-chewing-tsf + win-mcbopomofo | NVDA | CandidateModel 必须同时服务 popup 与 UIElement/屏幕阅读器语义。 |
| per-app/game policy | VKey 的产品策略 | Weasel/Rabbit 用例 | 只借 soft/hard exclude 与用户控制；禁止复制 Hook/input replay。 |
| 跨平台 core/frontend 分层 | Fcitx upstream + KeyMagic/Mozc | - | Windows 专属能力停在 platform layer。 |
| 产品化部署 | TypeDuck/Moqi | Chewing/PIME | 检查 per-user data、silent install、协议测试、升级生命周期。 |

# 附录 E. 源码审计得到的强制回归用例

| **ID**        | **触发场景**                                           | **必须结果**                                      | **来源**                |
|---------------|--------------------------------------------------------|---------------------------------------------------|-------------------------|
| REG-TSF-001   | 同一次物理按键出现多次 OnTestKeyDown，再出现 OnKeyDown | 不能重复送入 engine/重复 commit                   | Weasel                  |
| REG-TSF-002   | 宿主仅出现非典型 Test/KeyDown/KeyUp 序列               | 状态机可收敛，不残留 composing                    | Weasel                  |
| REG-COMP-001  | 宿主同步终止 composition                               | 不能把正常 EndComposition 误判为外部 abort        | Weasel                  |
| REG-IPC-001   | 请求 timeout 后旧 response 迟到                        | 旧响应不得匹配下一请求                            | WindInput → request_id  |
| REG-IPC-002   | 同步 response 与异步 notification 交错                 | 按 seq/request_id 正确关联                        | Moqi                    |
| REG-IPC-003   | engine/launcher 未启动、启动失败、冷启动很慢           | TSF bounded wait + fail-open；后台 warmup         | Moqi/Cassotis/WindInput |
| REG-CRASH-001 | backend 启动后立即崩溃                                 | 不得高速 respawn；进入 backoff/SafeMode           | Moqi                    |
| REG-CTX-001   | 焦点快速跨窗口切换，旧候选晚到                         | revision/context/epoch 丢弃 stale state           | Rabbit                  |
| REG-SEC-001   | password/sensitive control                             | 不学习、不日志、不预测；查询不能无限阻塞          | Rabbit + TSF InputScope |
| REG-DLL-001   | 宿主加载 TSF 后 DLL 搜索                               | 无 CurrentDir/Temp/Downloads 非预期依赖           | ProcMon/PE audit        |
| REG-GAME-001  | 全屏/Alt+Enter/x86 老游戏聊天框                        | 文本时可输入，游戏控制键 passthrough，无注入/Hook | DXUT/ImeStudy           |
| REG-LOL-001   | League of Legends + Vanguard：游戏内聊天、Alt+Tab、窗口/无边框 smoke | composition/candidate/commit 可用；QWER/WASD 等非文本控制键 passthrough；Vanguard 正常；无 Hook/SendInput/注入/规避 | 真实 E2E + Weasel 历史病例 |
| REG-UILESS-001 | 宿主 `BeginUIElement` 返回 `show=false`，Engine 仍有候选 | 独立 popup 不显示；UIElement candidate count/selection/change 与屏幕阅读器语义继续更新 | v1.7 `d12474c` 审计 |
| REG-CTX-002 | A(ctx1,comp1) → B(ctx2,comp2) → A(ctx1,comp1,new revision) | A 的合法新 snapshot 不因 B 的 compositionId 更大而被判 stale | v1.7 `d12474c` 审计 |
| REG-KEY-INTL-001 | AltGr、dead key、extended/scancode、非 US layout、key-up | language-neutral KeyEvent 正确送 Fcitx；未处理键无重复/吞键 | v1.7 `d12474c` 审计 |
| REG-PROFILE-001 | 在单一 Windows `Fcitx5` TSF profile 内从中文 engine 切到 ja-JP/其他真实 engine | Windows shell 仍只有同一 Fcitx5 profile；Fcitx IM/group 与 BCP-47/content locale 更新；DWrite locale/输入语义随当前 engine 变化；不动态注册第二个 TSF profile | v1.8 单一 TSF profile 决策 |
| REG-FCITX-CAP-001 | engine 请求 surrounding/delete-surrounding/forward-key | capability 声明与实际行为一致；不空实现静默吞掉 | v1.7 `d12474c` 审计 |
| REG-WARMUP-001 | engine preload/warmup | 不伪造用户文本键；无 commit/learning/history/global state 副作用 | v1.7 `d12474c` 审计 |
| REG-LAUNCHER-LEDGER-001 | engine 连续崩溃进入 SafeMode 后重启 launcher | crash-window/SafeMode 不被清零；健康窗口后按定义恢复 | v1.7 `d12474c` 审计 |
| REG-PROC-PIPE-001 | child 输出 64KiB/1MiB、挂死、早退 | parent 并发 drain、有界输出、可取消/回收；无 pipe 互等 120s | v1.7 `d12474c` 审计 |
| REG-REPO-STATE-001 | 已接受高 sequence 后 sequence-state 被截断/删除/损坏 | 不退化成 sequence=0；fail-closed 到显式 repair/reset；原子写入 | v1.7 `d12474c` 审计 |
| REG-PEER-ID-001 | 期望路径字符串相同/不同但涉及 hardlink/reparse/final path | peer 校验基于打开 handle 的真实 file identity + SID + Session | v1.7 `d12474c` 审计 |
| REG-INSTALL-UAC-001 | 标准用户用另一管理员账号凭据安装并随后卸载 | machine artifact 正确移除；原用户 startup/session state 不残留、不误删管理员 HKCU | v1.7 `d12474c` 审计 |
| REG-PKG-WINPATH-001 | CON/PRN/AUX/NUL/COM1/LPT1、trailing dot/space、case collision、reparse、`..` | Windows hostile path corpus 按唯一 policy 接受/拒绝；C++/Rust 迁移期结果完全一致 | v1.7 `d12474c` 审计 |
| REG-RUST-DIFF-001 | R1/R2 Rust side-by-side 与冻结 C++ contract/corpus | 对正确 contract 行为一致；旧 C++ 已知 bug 不作为 compatibility；cutover 后无永久 dual stack | v1.7 Rust migration |
| REG-UPDATE-TSF-001 | Word/Chrome 等保持加载 generation N TSF 时安装 generation N+1 | 旧宿主继续用 N；新宿主使用 N+1；N/N+1 只连各自 engine；无强杀/注入/旧协议 decoder；旧 generation drain 后可清理/重启兜底 | v1.8 generation draining |
| REG-UPDATE-TSF-002 | 更新过程中 N+1 health check 失败或 N+1 engine 崩溃 | current/previous identity 可安全回退；N 宿主仍可输入；不可再生用户数据不回滚；不产生 mixed-generation runtime | v1.8 generation draining |
| REG-CONFIG-A11Y-001 | keyboard-only + Narrator/NVDA 操作自绘 Navigation/Toggle/Segmented/Slider/ThemeCard | focus 顺序稳定、focus ring 可见；Name/Role/State 与 Invoke/Toggle/Selection/RangeValue 等适用 UIA pattern 正确；High Contrast 不丢关键状态 | v1.8 D2D Settings a11y |
| REG-CONFIG-VISUAL-001 | Config 在 100/125/150/200% DPI + Light/Dark/High Contrast | 页面/SettingRow/Toggle/Segmented/Slider/卡片使用统一 tokens，不退化成裸 Win32 属性表；无裁切/错位 | v1.8 UI/UX |
| REG-CONFIG-LIVE-001 | 修改主题/字号/Auto/横竖布局 | production Candidate Preview 同步 Live 更新；不要求反复 Apply→Preview；Reset 恢复继承 | v1.8 UI/UX |
| REG-CAND-STABLE-001 | 同一 composition 候选宽度长→短→长变化 | width hysteresis/稳定策略避免无意义频繁缩放；composition 结束后正确 reset | v1.8 UI/UX |
| REG-CAND-AUTO-001 | 中文短候选、长 annotation、非中文 engine、屏幕边缘 | Auto layout 按稳定规则选择横/竖；同一 composition 不随机抖动；显式 override 始终优先 | v1.8 UI/UX |
| REG-BRAND-001 | Shell/任务栏/开始菜单/TSF picker 在常用 DPI 与 Light/Dark/HC | Product icon 与 TSF icon 同一视觉家族；TSF icon 语言中立；Windows 用户可见 profile 名固定为 `Fcitx5`；内部 engine 切换不换 TSF identity | v1.8 branding |
| REG-UI-001    | ui.exe 崩溃或 device loss                              | 输入链继续；可重启 UI；UILess 语义仍正确          | Chewing/win-mcbopomofo  |

# 附录 F. Linux Fcitx5 与 Windows TSF 的安全模型差异

| **维度**      | **Linux Fcitx5**                         | **Windows Fcitx5**                       | **工程结论**                                    |
|---------------|------------------------------------------|------------------------------------------|-------------------------------------------------|
| Frontend 位置 | daemon / Wayland/X11/DBus/IM module 为主 | TSF in-process DLL 进入宿主              | Windows frontend 的崩溃与内存错误影响宿主进程。 |
| 主要 IPC      | DBus/socket/Wayland protocol             | TSF DLL ↔ per-user engine Named Pipe     | 必须建模协议版本、ACL、peer identity、timeout。 |
| 插件加载      | Fcitx daemon/addon                       | 仍应只进入 engine                        | 禁止 addon 进入 TSF DLL。                       |
| 动态库安全    | ELF loader/RPATH/distro package          | PE/DLL search order/Authenticode         | 需要 DLL hijack 测试和显式安全搜索路径。        |
| 权限/隔离     | Unix uid/permissions/sandbox             | ACL/Integrity/AppContainer/token/session | 按 SID+Session 隔离；Win7 做 legacy hardening。 |
| 候选 UI       | compositor/client UI                     | TSF UIElement + native popup + UILess    | 独立 UI 进程并保持 accessibility semantics。    |
| 旧应用兼容    | XIM/GTK/Qt/Wayland 路径差异              | IMM32/CUAS/TSF host 差异                 | 老游戏需单独回归，不复制 Linux threat model。   |

# 附录 G. 扩展参考源码与专项用途

参考库按“补盲区”使用，不要求 Codex 全部通读。实现某问题前先按 0.4 / Reference Implementation Matrix 读取当前 Phase 主教材的相关文件，再按本表补专项证据；没有当前失败或验收需求时不要扩大到邻近项目功能。

| 类别 | 项目 | 主要用途 |
|---|---|---|
| Legacy / 游戏 | `mangokingTW/ImeModePersistence` | IMM32↔TSF/CUAS、跨进程状态、SendMessageTimeout、UIPI/高完整性与游戏轮询风险 |
| Legacy / 游戏 | Microsoft DXUT `ImeUi` / “Using an Input Method Editor in a Game” | DirectX 全屏游戏作为宿主时的 composition/candidate/IMM 行为 |
| 无障碍 | NVDA | composition/candidate/selected candidate 的屏幕阅读器语义；验证 UILess 不只是视觉 popup |
| 多平台工程 | `thantthet/keymagic-3` | platform-independent core 与 Windows TSF frontend 分层、CI/发行工程 |
| 产品化 IPC | `TypeDuck-HK/TypeDuck-Windows` | Moqi 模型产品化后的 per-user data、silent install、协议/部署测试 |
| 游戏策略 | `phatMT97/VKey` | per-app 配置、soft/hard exclude、fullscreen game 产品策略；**不复制其 Hook 路线** |
| Rust TSF 实验 | `saschanaz/ime-rs` | windows-rs/COM/TSF binding 与 Rust 可行性研究，不作为当前重写理由 |
| 实验架构 | `llavon-ime/ime-windows-frontend` | Input/Control 与 Presentation 通道分离、service-process candidate snapshot 思路 |
| 实验架构 | `n0pd/azki` | thin TSF + 独立 backend 的小型现代实现，仅作设计对照 |

宿主端源码同样属于一等参考：Chromium `TSFTextStore`、Firefox `TSFTextStore/WinIMEHandler`、Windows Terminal IME/TSF 路径。真实兼容 bug 必须允许同时审计 IME 与 host 两端。

# 附录 H. `stop-that-shit` 完整仓库审计后的可采纳规则

本项目只吸收其**执行纪律与验证模式**，不引入该仓库作为运行时/构建依赖，也不照搬其领域默认策略。审计范围覆盖当前 `main` 的核心 decision/controller/state/runtime、Codex/OpenCode adapter 与 classifier、hooks/plugin、schema、CLI/release/eval scripts、全部测试与 case/eval fixtures、CI/模板/发布清单，以及提交演化；机器生成 validator 和 SVG 静态资源只做生成链/结构核对，不把生成代码或图标当架构逻辑逐行解释。

## H.1 直接吸收

| 模式 | 本项目落地 |
|---|---|
| Smallest correct result | 完整满足当前需求及必要后果，但拒绝无证据扩展；不以 diff/LOC 最小化代替正确性。 |
| Intent boundary | RESEARCH / REVIEW / CHANGE / RELEASE 四种 Task Contract，默认不向更高写权限自动升级。 |
| Stop Ladder | 额外动作必须有用户要求、当前验收必要性或 reachable evidence。 |
| Evidence ladder | 观察/报告 → Bad Case → Good Case → 可复现测试 → 必要时机器 gate。 |
| Paired cases | 每个新硬门禁尽量保存最近 Good Case，防止规则把合法必要行为一起拦掉。 |
| High-confidence enforcement | 自动化只阻止客观可检测的架构/安全事件，不尝试把所有设计判断编码成规则引擎。 |
| Completion stop | 当前验收与必要测试已有足够证据就结束 batch，禁止无限 review/search/test。 |
| Delete failed complexity | 实验性框架/抽象被证据否定后整套删除，不叠 compatibility layer 和 feature flag。 |
| Thin host adapters | 强化 TSF/管理 UI/平台 adapter 薄化，authoritative domain logic 只有一份。 |
| Artifact smoke | 测实际打包产物和隔离工作区，而不是只测源码树。 |
| Null-result honesty | 没测量就不声称“更快/更安全”；没有改善就记录没有改善。 |

## H.2 明确不照搬

- **不照搬 hash 默认拒绝策略。** 本项目的软件供应链明确需要 SHA-256、签名 manifest、Authenticode/SBOM。
- **不照搬 runtime migration framework。** 本项目最高原则明确不在正常运行路径保留旧格式 parser/双栈/自动迁移；格式设计要稳定，可再生数据直接重建，不可再生数据优先通过稳定格式解决。公开 Stable 后确有必要时，只允许与 runtime 隔离、用户显式触发、保留原文件的一次性离线转换。
- **不复制 compatibility facade。** 旧模块路径/旧接口不保留薄 shim；调用方同批次更新后删除旧入口。
- **不复制 fallback state write 路径。** Windows 特殊失败不能靠第二套 copy/unlink 等实现长期兜底；要么使用当前唯一正确原子路径，要么明确失败。
- **不把 runtime audit sidecar 放入输入热路径。** 输入数据平面优先隐私、延迟和简单性；必要 telemetry 只在 build/test/control plane 使用，且不记录输入内容。
- **不复制 agent-budget / session-authority 插件体系。** 当前个人项目没有真实需求，不为约束 Codex 再开发一个复杂 Codex 插件。先用本规格 + AGENTS.md + CI 的客观 gate。
- **不复制大型 live-model eval 网格。** 只借 paired Bad/Good regression 思想；Fcitx 工程测试重点是真实 Windows/TSF/IPC/package 行为。

## H.3 对我们额外得到的教训

1. **规则也会制造技术债。** 每加一个 guard 都增加误伤和维护面，所以 gate 自己也要有 Good Case 和删除条件。
2. **一次事实只维护一个 source of truth。** 版本号、协议版本、支持平台、包版本、feature availability 尽量从机器可读源生成/引用；不要在多份 README/文档手抄同一个会变化的事实。
3. **参考项目的成功与失败都要学。** 看到成熟项目删除某套复杂 workflow，比只看最终 happy path 更有价值。
4. **测试必须证明“不误伤”。** 安全/复杂度门禁只验证 Bad Case 会导致越来越保守；Good Case 才能证明工程仍能完成合法工作。
5. **约束 Codex 的首选方式是缩小任务和提供证据，不是造更多基础设施。** 如果 `AGENTS.md + 本规格 + focused CI` 能达到目标，就不要再写一套 agent control runtime。

# 附录 I. `fcitx5-windows-next@d12474c` 实现审计与 Rust Migration Map

## I.1 冻结审计范围

- Repository：`0x696c757a696f/fcitx5-windows-next`
- Audit commit：`d12474cc2ad541c6ae3824b701c8408a22e74500`
- 范围：自有 TSF、protocol/IPC、launcher、Fcitx engine/InputContext、CandidateModel/UI、WTL Config/Control、package/repository/updater/downloader/deployer、register/bootstrap、Inno installer、unit/integration/perf/fuzz、CI/release/security/dependency/SBOM/toolchain scripts。
- Fcitx5、WTL、miniz、toml++ 等第三方上游不在“逐行自有源码审计”范围；继续按 pinned dependency/SCA/license/SBOM 边界治理。

## I.2 Keep / Fix First / Rust R1 / Rust R2

| 子系统 | 结论 | 理由 |
|---|---|---|
| TSF DLL / TSF lifecycle | **Rust product component + host-matrix gate** | Shipping Rust TSF is allowed; 宿主内仍必须最小依赖、panic containment、fail-open、real-host matrix。 |
| IPC client/wire | **Rust product component** | wire/protocol/transport policy、request/session/peer/deadline 属产品边界，继续迁 Rust；C++ 只可作过渡 adapter。 |
| Fcitx Engine/InputContext | **Thin C++ Fcitx adapter + Rust Engine Core** | 直接操作 Fcitx object 留 C++；protocol/state/validation/revision/generation/policy/IPC 进入 Rust。 |
| CandidateModel/UI | **Rust product component** | Candidate semantics 跟随 upstream `CandidateList`/candidate action；Rust 负责 DTO、snapshot、layout policy、UI 状态和用户 intent。 |
| Config | **Rust product component** | typed settings、validation、command orchestration、preview state 迁 Rust；Windows native/a11y adapter 可阶段性保留。 |
| package/repository/update | **Rust R1** | 不可信数据、archive/path、签名、anti-rollback、事务状态最集中；Rust收益最高。 |
| downloader/provider | **Rust R1** | 网络/外部输入边界，且与实时输入域隔离。 |
| elevated deployer | **R1 optional** | 代码必须最小；只有权限/Win7 toolchain/installer evidence 满足才迁。 |
| launcher | **Rust R2** | 状态机适合强类型；先修并冻结 crash ledger/recovery contract。 |
| control/diagnostics/process execution | **Rust R2** | CLI/JSON/child process/repair 状态适合 Rust；先统一 C++ process-exec 行为。 |
| register/bootstrap | **Thin Windows adapter + Rust policy where useful** | 极薄 Windows system layer 可阶段性保留；artifact identity、operation policy、timeout lifecycle 等产品语义继续收敛到 Rust。 |

## I.3 Rust migration 不是 rewrite project

Rust 迁移按组件小批次进行。一个 PR/batch 只迁一个明确边界及其必要调用方/测试；不同时重构 protocol、GUI、installer 和 package 业务。迁移期可以 side-by-side 产出测试二进制，但正式 runtime 只保留一个 authoritative implementation。Rust 对外不改变产品身份、配置格式或用户目录结构，除非当前 contract 本身需要 breaking change；此时先做 contract change，再迁移实现。

## I.4 Rust 安全/供应链最低门槛

- pinned Rust toolchain；`Cargo.lock` committed；release `--locked`。
- advisories/licenses/sources/bans 自动 gate；crate 来源和 license 进入统一 dependency inventory。
- Cargo resolved dependency 与 C++/MSYS2 dependencies 一起进入 SPDX SBOM、THIRD_PARTY_NOTICES、provenance。
- `.rs` 进入禁止能力/source security scan；Rust PE 进入同一 import/network/min-OS/Authenticode checks。
- unsafe 只允许最小 platform/FFI adapter；业务 parser/path/state 默认 safe Rust。
- Modern-first。Legacy Win7 只有单组件 PoC 通过后才允许 Rust lineage；不牺牲 Core/TSF/输入可用性来提高 Rust 覆盖率。

# 最后的执行判断

| **如果只能记住一件事　先把 Windows TSF ↔ Fcitx5 与 Presentation/Profile/Package 边界做得小、清楚、可失败、可测试；先修语义再迁 Rust，并让同一条证据链贯穿 C++/Rust 代码→CI→真实包→签名→发布。官方规范决定语义，成熟项目提供兼容经验；安全、性能、人因、配置和供应链只在真实风险处加 Gate。边界正确且发布可追溯，插件/主题才能安全扩展；边界或证据链错误，功能越多，宿主崩溃、隐私泄露和供应链技术债越严重。** |
|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
