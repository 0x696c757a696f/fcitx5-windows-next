# win-mcbopomofo thin client/server 审阅

状态：v1.6 Phase 0 次教材审阅  
日期：2026-08-18  
固定提交：[`06a570a780c088f329cf8532cc558d670a73c5f6`](https://github.com/openvanilla/win-mcbopomofo/tree/06a570a780c088f329cf8532cc558d670a73c5f6)  
许可证：GPL-3.0；只采用行为/边界/测试经验，不复制非平凡源码。

## 可验证拓扑

- `McBopomofoTIP_v2.dll`：宿主内 TSF activation、key sink、composition、display attribute、
  caret geometry 与 UIElement；
- `McBopomofoServer.exe`：唯一长期 EXE，拥有 engine/state、tray、Candidate/Tooltip HWND；
- `McBopomofoConfig.exe`：按需设置器；
- `src/Common`：Named Pipe 与序列化。

Server 产出 `StateUpdatePayload`，Client 在 `CStateEditSession` 中投影 commit/preedit/caret 和
UIElement。自定义 popup 在 Server，TSF 文档访问仍在 Client。当前项目应学习这个所有权
分割，而不是照搬它的协议或单 context 限制。

## Keep

- engine state 是真相，UI 是状态投影；commit 也是显式状态迁移；
- Client 负责宿主 TSF composition/EditSession，Server 不直接写宿主；
- Candidate 数据同时服务 TSF UIElement 与自定义 popup；无 popup 时仍保留 UILess；
- caret/range geometry 由宿主内 Client 查询并传给进程外 popup；
- direct commit、candidate-only state、empty/reset 都有明确 UIElement/窗口 teardown；
- `SetSelection` 先调用 Server 的语义 `SELECT_CANDIDATE`，再由 Client 把新 state 应用到
  TSF EditSession，而不是模拟按键。

## Rewrite

- 其同步文本 Named Pipe 改为 v1.6 versioned binary IPC、硬大小上限、request correlation、
  deadline/cancel、SID+Session 与 peer verification；
- 单一 `InputController` 改为 per-client/per-context InputContext；所有消息带 epoch/context/
  composition/revision/candidate-id；
- Server 内 engine+popup 拆成独立 Engine 与按需 `fcitx5-ui.exe`，避免 UI 故障影响引擎；
- UI 点击不能在 UI 内 apply state；应由 Engine 选择，随后唤醒对应 TSF Client 拉取并通过
  EditSession 投影结果；
- Windows 10+、x64/x86/ARM64 的构建结论不能证明本项目 Win7 Legacy lane。

## Do-not-inherit

- 无协议版本、文本 `stoi/stoull` 解析、1 秒同步 `WaitNamedPipe`、默认 Pipe ACL；
- 全局固定 pipe、缺少 session/context/revision 的 candidate selection；
- 单 engine context 导致多应用状态串线；
- Server 自身同时拥有 engine 和 popup 的故障域；
- GDI fallback、注册表/INI Config 或 Server 自动更新作为本项目默认路线；
- 任何 GPL 实现代码复制。

## 对当前实现的直接约束

当前 UI 的 SendInput 点击必须删除。最终链路是：

```text
fcitx5-ui.exe
  → SelectCandidate(epoch, target_pid, context, composition, revision, candidate_id)
fcitx5-engine.exe
  → 校验并调用 Fcitx CandidateWord::select(InputContext)
  → 保存新的 authoritative state / commit
  → 通知对应 TSF client 有结果可取
fcitx5-tsf.dll
  → 通过已认证 IPC 拉取结果
  → RequestEditSession 应用 commit/preedit/UIElement
```

通知只能是无数据 wake-up；权威结果必须从已认证 IPC 获取。晚到、重复、非前台或身份不匹配
的选择全部拒绝。

## 代码级核对（2026-08-18，pin `06a570a7…`）

按规格 0.4 的"看相关代码实现"要求，在固定 commit 上直接阅读以下实现文件，核对结论：

- `src/Common/NamedPipe.cpp`：`NamedPipeServer::ServerLoop` 用
  `ConvertStringSecurityDescriptorToSecurityDescriptorA("D:(A;;GA;;;WD)(A;;GA;;;AC)S:(ML;;NW;;;LW)", ...)`
  —— **Everyone(WD) + AppContainer(AC) 全权限、低完整性 no-write-up**；`CreateNamedPipeA` 单线程
  `ConnectNamedPipe` 串行服务、同步 `ReadFile`/`WriteFile` 无 deadline；`NamedPipeClient::Call` 用
  `WaitNamedPipeA(..., 1000)` 1 秒等待后 `CreateFileA`，同步往返。
- `src/Common/Ipc.cpp`：文本 message 请求/响应（无 versioned 二进制帧、无 request correlation、
  无 session/context/revision 身份）。
- `src/Client/StateEditSession.cpp`：Server 的 `StateUpdatePayload` 在 TSF `CStateEditSession`
  EditSession 中投影 —— display attribute（`GUID_PROP_ATTRIBUTE`）、composition、commit、caret
  均由 Client 在宿主内完成；Server 不直接写宿主文档。
- `src/Client/TsfUiElement.cpp` + `src/Server/CandidateWindow.*`：TSF UIElement 元数据与
  Server 内自定义 popup 共用同一候选状态；popup 非激活显示。

对本项目的影响：确认"engine 状态是真相、TSF 投影、UIElement 与 popup 同源"的边界与 ADR-0004 一致；
本项目将同步文本 pipe 替换为 versioned binary IPC + deadline + SID+Session + peer 验证，不继承
1 秒 `WaitNamedPipeA`、WD 全权限 ACL 与无身份选择。
