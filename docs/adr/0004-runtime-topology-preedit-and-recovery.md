# ADR-0004：运行拓扑、预编辑所有权与分层恢复

- 状态：Accepted under v1.6
- 日期：2026-08-18
- 取代：宿主内自绘候选、UI/Engine 自行 commit、永久常驻 UI、额外 watchdog、浮动 preedit 作为唯一真相、SendInput/Hook 提交以及要求用户启动内部 EXE 的方案

## 背景

候选无法点击、焦点切换残留、长输入卡住和进程退出后无法恢复，都是状态所有权和生命周期未闭合的系统问题，不能靠追加窗口消息修好。Phase 0 主依据是 Chewing 的 TSF/UILess/独立 renderer 实践，McBopomofo 校对 client/server 边界；Weasel 只提供兼容病例。

## 决策

1. `fcitx5-tsf.dll` 是唯一宿主内组件，拥有 TSF composition、EditSession commit 和 UILess UIElement 投影；只做有界 IPC。
2. `fcitx5-engine.exe` 拥有 InputContext、CandidateModel、选择语义、epoch 与 revision。
3. `fcitx5-ui.exe` 只渲染不可变 snapshot，窗口不激活；仅在确有候选/通知时运行。
4. 点击产生带 engine epoch、context、composition、revision、candidate id 的选择意图。Engine 校验并产生权威结果，TSF 通过异步 EditSession 投影；UI 不写宿主。
5. 默认 preedit 是宿主内 TSF composition。浮动 preedit 仅为经过宿主验证的兼容回退，不能成为状态真相。
6. `fcitx5-launcher.exe` 是 per-user/per-session 生命周期 owner：监督 Engine，按需拉起 UI，对二者分别计数、退避和熔断。不得新增常驻 watchdog。
7. Launcher 可注册 Windows Application Restart，但下次正常 TSF activation 也必须能重建 launcher；输入线程在恢复期间 bounded fail-open。
8. focus/hide/selection 都校验原 context 身份；晚到旧 context、epoch 或 revision 必须丢弃。
9. 产品只暴露 Start、Settings 与 Windows 标准卸载/修复入口。内部 helper 不要求用户选择。

## 运行形态

```text
Host process: fcitx5-tsf.dll
User session: fcitx5-launcher.exe + fcitx5-engine.exe
On demand:    fcitx5-ui.exe, fcitx5-config.exe, package/updater/deployer helpers
```

Application Restart 是恢复手段之一，不是唯一保证；Job Object 也不得造成 Launcher 一退出就永久失去输入恢复能力。

## 后果与验证

获得宿主最小化、状态单一、UI 故障不阻塞、UILess 可用和反作弊友好。代价是选择结果需异步回到 TSF EditSession，Launcher/UI 懒启动和身份状态机必须用 virtual clock、fault injection 和真实宿主验证。

- `REG-UI-001`：UI crash/device loss 不阻塞 composition/commit，UILess 语义仍正确；
- Engine/UI/Launcher 分别退出后按策略恢复，无风暴、无旧窗口、无句柄持续增长；
- Notepad/Office/browser/VS Code 的 inline preedit、caret、click commit、focus loss；
- emoji/surrogate pair、空 preedit、commit+new-preedit、外部 composition termination；
- 旧 host 失焦、旧 epoch/revision/candidate id 全部拒绝；
- 静态/运行证据证明无 Hook、SendInput、注入或游戏规避路径。
