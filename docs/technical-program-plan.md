# v1.7 技术统筹与执行计划

状态：current  
更新：2026-08-19

## 交付定义

交付的是可持续输入的 Windows 平台，不是“许多能启动的 EXE”。用户安装后只需用
`Win + Space` 选择 Fcitx5；托盘负责状态/恢复，Settings 负责低频管理，内部 helper
均由受控调用链启动。

```text
Host → TSF → bounded IPC → Launcher/Engine → Fcitx5 → CandidateModel
       ↑ EditSession commit                         ├→ UILess UIElement
       └──────── authoritative result ──────────────└→ on-demand D2D UI
```

候选点击也走同一主链：UI 只发送带身份的 `SelectCandidate` 意图；Engine 校验并改变权威
状态；TSF 通过 EditSession 投影 commit。禁止 SendInput、Hook 或 UI 直接写宿主。

## Phase 执行顺序

| Phase | 主参考 | 本阶段交付 | 退出门禁 |
| --- | --- | --- | --- |
| 0 | `fcitx/fcitx5`、addon upstream、Chewing、McBopomofo、Weasel 定向补充 | pins、许可证、Reference Matrix、Keep/Rewrite/Do-not-inherit | 每个关键设计有首选参考和禁止继承项 |
| 1A | MSVC/CMake/Windows SDK | x86/x64 空壳、统一 build、编码/静态/依赖基础门禁 | clean checkout 的 `dev/test` 确定通过 |
| 1B | Chewing + McBopomofo | TSF → versioned IPC → mock engine → Notepad commit | 双架构注册激活；bounded fail-open；自动 E2E |
| 2 | Chewing；按问题看 WindInput/Moqi | IPC v2、身份/ACL/deadline、Launcher、contract/fuzz/fault | timeout/late/backoff/并发/SYSTEM-LogonUI 确定测试与延迟基线 |
| 3 | `fcitx/fcitx5` core + addon upstream | Fcitx object adapter、capability model、基础 engine | 多 context 隔离；restart epoch；启动/idle/key repeat；surrounding/candidate action capability 基线 |
| 4 | Chewing + McBopomofo；Weasel 病例 | CandidateModel、独立 C++ D2D UI、UILess、strict TOML、DPI/a11y | 真实宿主候选；UI crash 不阻塞；隐藏无 render loop；Win7 import smoke |
| D0.1 | 真实宿主 | 停止加能力，维护者 dogfood；LoL+Vanguard 正规 TSF 路径 | 日常输入稳定；无 Hook/SendInput/注入；故障不拖垮宿主 |
| 5 | Weasel；失败定向看 Rabbit/WindInput/Moqi | 可靠性、安全、Win7/游戏/Office/browser/RDP 兼容 | crash-loop、hijack、Safe Mode、实机矩阵 |
| 6 | v1.6 Config 规则 | WTL Config、真实预览、安装/repair/uninstall/portable、i18n | typed round-trip；headless self-tests；GUI 生命周期一致 |
| 7 | package/provider 契约 | addon/data/theme/translation、签名仓库、事务/回滚/权限分离 | parser fuzz；atomic activation；坏插件 Safe Mode |
| 8 | release gate | identity、签名、SBOM/provenance、系统包管理器、PKG rollback | exact artifacts 与 release checklist 完整 |

不得合并 Phase 来“节省步骤”。提前存在的后期代码可以保留，但只有通过前置契约后才能
成为当前交付证据。

## 状态与进程所有权

- TSF：composition、EditSession commit、UILess 投影；没有 renderer/engine/network。
- Engine：Rust core 拥有 protocol/state/validation/revision/generation/policy/IPC；薄 C++ adapter 只直接操作 Fcitx `InputContext`、`CandidateList`、addon/config/event 对象。
- UI：按需启动，非激活渲染不可变 snapshot；窗口消失不改变输入真相。
- Launcher：per-user/per-session；Engine 监督、UI 按需拉起/退避、托盘与最小健康状态。
- Config/package/updater/deployer：按需，完成后退出。

Launcher 崩溃不能靠新增永久 watchdog 解决。Windows Application Restart 可作补充，但必须
保证下次正常 TSF activation 也能重新建立 launcher。输入线程等待总有硬 deadline，失败
即 passthrough/fail-open。

Engine dispatcher 的队列语义必须作为 contract 管理：同一 connection/context 内保持请求
顺序；队列压力只允许按明确 backpressure 策略拒绝或 fail-open；超过 absolute deadline
且尚未执行的任务必须 DROP；已超时但可能执行过的 key 不重试，避免双提交或双翻页。
性能优化不能改变这些 order/deadline/drop 语义。

## 更新与 generation draining

`fcitx5-tsf.dll` 更新不得靠强杀或强制关闭所有加载 TSF DLL 的宿主进程解决。TSF DLL 是
宿主内组件，可能被 Word、浏览器、编辑器、Explorer、游戏和企业软件长期加载；产品不注入、
不远程 `FreeLibrary`，也不把 Restart Manager 关闭宿主作为默认升级路径。

采用 ADR 0005 的 deployment-level generation side-by-side draining：

- 同一 generation 内组件严格同版，IPC protocol 可以 breaking change；
- 更新事务允许多个完整 generation 暂时并存，但每个 generation 使用独立 runtime 目录和
  generation-specific IPC endpoint；
- TSF 文件更新使用 `rename-old -> install-new -> delayed cleanup`：旧宿主继续使用
  `fcitx5-tsf.old.<generation>.<id>.dll`，新宿主从注册路径加载新的 `fcitx5-tsf.dll`；
- 当前 `fcitx5-tsf.dll` 旁必须有 `fcitx5-tsf.generation` sidecar，DLL 优先读取该文件确定
  自身 generation，再 fallback 到安装根 `current.json`；避免更新窗口中新 DLL 读旧 current
  或旧 DLL 读新 current；
- 旧 TSF 和旧 Engine/UI/runtime 成组自然 drain，不要求新 Engine 理解旧 protocol；
- IPC/local object 名包含 `.Generation.<generation>`；launcher/engine/UI/TSF 都按 generation
  路由，旧 TSF 从 `fcitx5-tsf.old.<generation>...dll` 解析 generation；
- `current.json` 记录 active/previous runtime generation 和 build id；只有
  `runtime\<generation>` 存在时 updater 才能发布为 current；
- named-pipe 连接遇到 `ERROR_PIPE_BUSY` 时只允许在已有 absolute deadline 内等待可用实例；
  超过 deadline 必须 fail-open，不得无限阻塞宿主输入线程；
- 旧 DLL/旧 runtime 删除失败时记录 pending cleanup，后续 launcher/updater 重试，最终可用
  `MoveFileEx(..., NULL, MOVEFILE_DELAY_UNTIL_REBOOT)` 延迟删除。
- 每个 TSF generation 还有用户级 activation guard：激活失败或上次激活未收尾时，TSF 在
  宿主内 fail-open，不注册 key sink、不连接 engine、不吃键；`fcitx5-control` 暴露状态和
  reset，repair 可清除。这个 guard 不是 Windows 安全模式。

这精确定义 v1.7 的版本原则：**磁盘上不保留永久 runtime protocol compatibility；更新期间
允许多个完整 generation 暂时并存并由 IPC generation 隔离。**

## 设置生效语义

- `Live`：安全、可逆的视觉或低风险设置立即生效并持久化；
- `Deferred`：复杂编辑在明确 Apply/Cancel 后提交；
- `Restart-required`：只对确需重启的边界显示结果和动作。

Config 不强制每页出现没有语义的 Apply。预览使用生产 CandidateModel/theme/layout 和真实
renderer synthetic host，不维护第二套近似 UI。

Advanced 页面只暴露 generic Fcitx addon/config surface：读取 Fcitx 原生 addon/config
描述并以受限、类型化方式呈现；Windows 层不维护巨型硬编码 input-method/addon 配置映射。
确有高价值的一线设置可以提升为普通页面，但必须仍以 Fcitx 原生配置为单一语义来源。
输入法分组、addon 动态配置、typed option renderer、插件管理器、数据/词库/短语管理等均
通过 engine/control 暴露，不从 TSF DLL、Candidate UI 或包管理代码直接加载 Fcitx addon。

TSF input profile 采用单入口原则：Windows 语言栏只出现一个产品级 profile
`Fcitx5 for Windows Next`，不把 Pinyin、Rime、Mozc 或后续插件拆成多个 Windows 输入法。
插件/配置工具通过 Fcitx engine 的 addon/config surface 管理可用输入法与默认输入法；
repair/uninstall 仍读取旧 ledger/obsolete GUID 清理曾经注册过的动态 profile，避免语言栏
留下“幽灵键盘”。TSF DLL 不加载 Fcitx addon、Rime、Lua 或配置 GUI，也不把 profile GUID
当作 engine 选择信号。

## Rust 迁移约束

Rust 迁移按 risk-driven 组件推进。每个组件必须先在 C++ 中修正语义并冻结 contract、
hostile corpus、golden corpus 和 fuzz seeds，再做 C++↔Rust correctness differential。
只有 differential 通过后才比较性能、资源和包体积；performance second，不能用更快但语义
不一致的 Rust 实现替换现有 C++。切换按单组件 cutover 完成，同批删除旧实现，不保留永久
双栈。

Fcitx 边界采用 ADR 0009：Fcitx5 core object model 和 upstream addons 不 Rust 重写；直接
操作 `Instance`、`InputContext`、`AddonManager`、`CandidateList`、Fcitx config/event 的
层保持薄 C++ adapter；其余产品逻辑继续 Rust 化。Addon 抽象必须支持 static/built-in 与
dynamic/package-loaded，不假设 `Addon == DLL`。Engine protocol 必须使用可扩展 capability
model，例如 `TEXT_COMMIT`、`TEXT_COMMIT_WITH_CURSOR`、`TEXT_DELETE_SURROUNDING`、
`TEXT_REPLACE_SURROUNDING`、`CANDIDATE_ACTION`。

## 本轮执行队列

1. 清除 v1.5/旧技术路线和虚假完成叙述，完成 Phase 0 文档门禁。
2. 建立 `no-hook-sendinput` 静态门禁和候选交互 RED tests。
3. 实现 UI → Engine 语义选择 → TSF EditSession commit；验证旧 revision/失焦/重复点击。
4. 修正 UI 按需启动、Engine/UI 独立故障预算和 Launcher 再激活恢复。
5. 覆盖 preedit UTF-8→UTF-16 caret、emoji、commit+new-preedit、UILess 与焦点 churn。
6. 双架构 focused/full gates，再做无需 UAC 的 Desktop/Package；需重注册证据保持待办。
7. 达到 D0.1 后再扩大 Phase 5；Config/package/release 按 6/7/8 顺序重新验收。

## 质量门禁

| 风险 | 必须有的证据 |
| --- | --- |
| preedit/caret | UTF-8 边界/UTF-16 offset 单测；TSF fake session；emoji/焦点真实宿主 |
| 候选点击 | hit/identity/property；UI↔Engine↔TSF contract；Notepad 点击提交 |
| UILess | CandidateModel 同源；UIElement Begin/Update/End；NVDA/Narrator/host smoke |
| 生命周期 | virtual clock；crash-loop fixture；逐一 kill Engine/UI/Launcher 后恢复 |
| 配置 | typed parser/model；Live/Deferred/Restart round-trip；真实 renderer 预览 |
| package | schema/signature/path fuzz；install/update/remove/rollback；离线/损坏/占用 |
| 反作弊 | import/static scan；LoL/Vanguard smoke；无 Hook/SendInput/注入/规避 |

每项高风险行为至少需要最低层确定测试、跨进程契约/集成以及对应真实宿主或故障证据。
历史报告只表明曾通过，受影响代码改变后立即过期。

## 明确不做

- 不更换冻结技术栈，不从 Config/商店/主题开始；
- 不复制 GPL 非平凡实现，除非先做许可证决策；
- 不采用 Hook、SendInput、UIA、坐标点击或宿主内 WebView；
- 不让 TSF 下载、解压、加载 addon 或自绘候选；
- 不让 UI 拥有 CandidateModel 或直接 commit；
- 不把 UI 永久常驻，也不新增 watchdog；
- 不在无新授权时触发 UAC；
- 不在生产签名/仓库/兼容证据缺失时宣布 Phase 8 完成。
