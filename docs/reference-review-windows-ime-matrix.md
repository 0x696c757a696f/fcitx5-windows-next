# 附录：Windows 输入法成熟实现问题对照

状态：supplementary reference（不得覆盖 Chewing 主教材）  
审查日期：2026-08-18  
用途：为 v1.6 的具体预编辑、界面边界、运行进程和故障恢复问题提供补充输入；不复制第三方源码，也不平均分配研究精力。

## 结论先行

六类成熟实现给出的共同规律是：

1. 可编辑的预编辑文本必须由 TSF composition 投影到当前宿主；浮动窗口中的预编辑只能
   是可选展示或兼容回退，不能成为唯一真相。
2. 候选、composition、commit 必须属于同一个带身份的输入会话。失焦、延迟响应和旧窗口
   消息必须用 host/client、context、composition、epoch、revision 校验后才能清理新状态。
3. 后台进程要由一个明确的生命周期所有者恢复，并有最小存活时间、退避和熔断；TSF
   输入线程不能无限等待，也不能在每次按键中制造无界重启风暴。
4. “安装包里有多少 EXE”和“正常输入时常驻多少进程”必须分开统计。部署、配置、字典、
   注册工具按需运行，不应因为文件存在就出现在用户启动流程里。
5. 用户入口应少于内部安全边界。内部 helper 可以分进程，但产品层只暴露“启动”和
   “设置”，卸载走 Windows 已安装应用。

## 对照矩阵

| 项目 | 预编辑 | 候选与主要界面 | 正常输入常驻 EXE | 按需/安装 EXE | 故障恢复经验 |
| --- | --- | --- | ---: | --- | --- |
| Weasel | `inline_preedit=true` 时写入 TSF composition；关闭时可把 composition/preview/preview_all 绘在候选面板 | 候选 UI/UILess 对象在 TSF 宿主内；Server 提供 Rime、IPC、托盘；Deployer 提供设置/部署 | 1：`WeaselServer.exe` | `WeaselDeployer.exe`、`WeaselSetup.exe`、uninstaller | Server 注册 Windows Application Restart；TSF 点击先选择服务端候选，再注入 `VK_SELECT` 令 TSF edit session 取回并提交；失焦终止 composition。优点是路径成熟，缺点是候选渲染仍在宿主进程 |
| windows-chewing-tsf | 预编辑与分段下划线写入 TSF composition；commit 与新 preedit 可在一次 edit transaction 中衔接 | 新版把候选/通知渲染和更新检查移到 `chewing_tip_host.exe`；设置、词库编辑独立 | 1：`chewing_tip_host.exe` | `ChewingPreferences.exe`、`chewing-editor.exe`、`chewing-cli.exe`；`tsfreg.exe` 作为安装 custom action | Host 注册 Application Restart；失焦结束 composition、隐藏候选；显式 Begin/Update/End UIElement。宿主 IPC 与签名校验值得采用，但 JSON 长度/超时仍需加强 |
| WindInput | Service 下发 `CMD_UPDATE_COMPOSITION`，TSF 负责宿主组字；caret 单独上报 | Rust `wind_input` 负责引擎、候选管理、UI、IPC；设置器/portable launcher 未开源 | 1：核心服务 | 未开源设置器、portable launcher、installer | TSF 自动重连并可启动服务；双管道分离同步请求与异步推送。最关键经验是失焦携带 `clientToken`：旧宿主约 100ms 后到达的失焦不得清掉新宿主状态 |
| Moqi | 支持 inline composition；关闭 inline 时在候选窗顶部绘制 preedit 与光标 | 候选窗当前在 TSF 宿主内；`MoqiLauncher.exe` 代理多个 backend；托盘在 Launcher | 2：Launcher + 当前 backend `server.exe` | `SetupHelper.exe`；候选 Preview 是开发工具 | Launcher 注册 Application Restart；backend 崩溃按最小 2 秒存活阈值节流，快速启动崩溃则停止并等待下一次请求懒启动；TSF 可拉起 Launcher。其强杀不可达 Launcher 的做法不采用 |
| win-mcbopomofo | Server 产生 state，Client DLL 在宿主应用 TSF composition 中应用 preedit/commit | Server 拥有引擎状态和自定义候选/tooltip；ConfigApp 独立 | 1：Server | 1：ConfigApp；安装器/脚本另计 | HKCU Run 启动 Server；安装流程先停 Server/Config、注册多架构 DLL、再启动 Server。状态投影边界清晰，但 Windows 10+ 与当前未发布状态不能直接当兼容性证据 |
| PIME/libIME2 | backend 返回 `compositionString`/cursor，TSF 开始、更新、结束 composition；cursor 明确处理 UTF-16 surrogate pair | PIME 候选窗在 TSF 宿主；Launcher 代理 Python/Node/Go 等 backend | 1 + 当前 backend 数量 | 每个输入法 backend/配置工具不同 | 设计规格要求 backend 崩溃或超时后静默恢复且不丢客户端连接；当前 Rust Launcher 提供代理/监控。多运行时模型不适合本项目，但断线重置和 Unicode caret 测试必须采用 |

注：表中的常驻数不含加载在应用进程里的 TSF DLL，也不含 `ctfmon.exe` 和 Windows 自身
进程。闭源或未安装完整成品的项目只统计可验证的开源范围。

## 对本项目的冻结决定

### 预编辑

- 默认且权威的预编辑位置是宿主编辑控件中的 TSF composition。
- Candidate UI 不重复显示预编辑，除非进入经测试的兼容回退模式；回退必须在 UI 上明确
  标识，且仍不能替代 TSF composition 的生命周期。
- protocol 的 caret 使用 UTF-8 byte offset 传输，进入 TSF 前必须验证边界并转换为 UTF-16
  code-unit offset；孤立 continuation byte、越界、surrogate pair 和 emoji 都要有回归。
- commit、旧 composition 结束和新 composition 开始必须防重入；同步终止回调不能清掉刚
  建立的新 composition。

### 界面与进程

正常运行不固定要求 UI 常驻：

```text
应用进程
  └─ fcitx5-tsf.dll       TSF composition、commit、限时 IPC、UILess 元数据

用户会话
  ├─ fcitx5-launcher.exe  生命周期所有者、托盘、状态、退避/熔断
  ├─ fcitx5-engine.exe    Fcitx5/Rime/addon 和 CandidateModel 真相
  └─ fcitx5-ui.exe        仅实际显示时按需运行；D2D/DWrite，不获取键盘焦点
```

按需界面：

- `fcitx5-config.exe`：设置、插件、诊断、预览；关闭即退出；
- package/downloader/deployer/provider/updater：内部 helper，只由 Config/Control 调用；
- register：安装器/portable 注册辅助；
- libime/rime 数据工具：内部工具，不是用户入口。

当前 stage 有 28 个 `.exe` 文件，但只有 3 个顶层文件，其中 `Unregister Fcitx5.exe`
会造成误导。新的产品入口收敛为：

- `Start Fcitx5.exe`；
- `Fcitx5 Settings.exe`；
- 卸载/修复通过 Windows“已安装的应用”或安装器完成。

内部 EXE 数量不是产品启动步骤，也不得在普通用户文档里让用户逐个选择。

### 故障恢复

| 故障 | 所有者 | 自动动作 | 用户可见结果 | 禁止行为 |
| --- | --- | --- | --- | --- |
| Engine 退出 | Launcher | 指数退避重启；达到阈值进入 Safe Mode | 托盘显示恢复/安全模式；输入暂时直通 | TSF 每键无限重启、无退避循环 |
| UI 退出 | Launcher | 有候选需求时按独立预算重启；反复崩溃时暂停视觉 UI | composition/commit 与 UILess 继续；托盘可重试 | UI 永久常驻、UI 崩溃拖死 Engine |
| Launcher 崩溃 | TSF activation + Windows Application Restart（补充） | 下次正常 activation 重建 Launcher，再恢复 Engine；UI 仍按需 | 短暂直通后恢复 | 只依赖 Application Restart 或增加永久 watchdog |
| TSF 宿主退出 | Windows/宿主 | OS 卸载 DLL；Engine 按 context 过期并隐藏候选 | 其他应用不受影响 | 按 PID 通配清掉新 context |
| Config/helper 退出 | 用户/调用者 | 不常驻、不自动循环；事务未提交则回滚 | 用户重开设置或重试 | 后台静默反复弹窗 |
| IPC 超时/身份不符 | TSF/Launcher | 取消 I/O、断线、丢弃旧 revision、输入直通 | 宿主不卡死 | 在输入线程无限等待或接受旧响应 |

## 采用与拒绝

采用：TSF composition、UIElement 生命周期、Application Restart 作为补充、带身份失焦、异步推送、
最小存活阈值、退避/熔断、按需配置工具、Unicode caret 测试、候选点击经过 TSF edit
session 完成提交。

拒绝：候选 UI 加载在任意宿主进程、TSF 强杀进程、全局无 SID/session 管道、无界 JSON、
每键无界启动服务、浮动 preedit 作为唯一状态、Weasel 的 SendInput 提交方式、把安装 helper
暴露成普通用户启动步骤。

## 来源

- [Weasel](https://github.com/fxliang/weasel)
- [windows-chewing-tsf](https://github.com/chewing/windows-chewing-tsf)
- [WindInput](https://github.com/huanfeng/WindInput)
- [Moqi](https://github.com/gaboolic/moqi-im-windows)
- [win-mcbopomofo](https://github.com/openvanilla/win-mcbopomofo)
- [PIME](https://github.com/EasyIME/PIME) 与 [libIME2](https://github.com/EasyIME/libIME2)

固定提交和许可证状态见 [`reference-baseline.md`](reference-baseline.md)。

## 新增病例：Weasel `d73f6295…` 托盘刷新不得阻塞 IPC pipe（2026-08-18 核对）

该 commit 把托盘刷新从"IPC 处理路径内同步执行"改为显式异步状态机：

- `WeaselTrayIcon::RequestRefresh()`：互斥锁下合并 `m_pending_state`，只 `PostMessage`
  `WM_WEASEL_SERVICE_NOTIFY` 一次（`m_refresh_pending` 去重），不直接做任何窗口/托盘 I/O；
- `WeaselTrayIcon::ApplyRefresh()`：在目标窗口消息循环中执行真正的 `Refresh(state)`，完成后
  清 `m_refresh_in_progress` 并 `notify_all`；
- `WeaselTrayIcon::DisableRefresh()`：停用刷新前等待进行中的 Apply 结束（`m_state_cv.wait`）。

根因：托盘 `Shell_NotifyIcon` 调用可能慢或阻塞，若在 IPC 回调线程上同步执行会卡住
`WeaselServer` 的 pipe 服务。对本项目的直接意义：launcher 的托盘与 engine IPC 必须分属
不同职责，托盘状态变化只能走异步通知，不能在输入热路径或 engine IPC 线程上同步做
`Shell_NotifyIcon`/窗口 I/O；本项目的 `tray_icon.cpp` 沿用同一异步原则。
