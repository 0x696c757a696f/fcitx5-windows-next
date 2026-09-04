# 081 UI 迁移调查 — src/ui/ui_main.cpp (fcitx5-ui, CANDIDATE-RENDERER-D2D-TINY-SKIA-CUTOVER-001 Slice A 输入)

代码调查（只读）。HEAD `a069c16278efa3301a034c329c0f45a981b7aead`。
目标：为 081 renderer cutover 定位 data/logic 与 rendering 的迁移边界。

---

## ui_main.cpp 的主要职责

单文件 3007 行候选窗口 UI 宿主。编译为 WIN32 可执行 `fcitx5-ui.exe`（CMakeLists.txt:636，
link d2d1/dwrite/shell32/ole32 + `fcitx5::candidate_layout` + `fcitx5_config_core_rust` +
`fcitx5::windows_common` + `fcitx5_protocol_core_rust` + `fcitx5::ipc_client` + `fcitx5::platform`）。
进程内两大半区：

1. **演示/预览/自测宿主**：`wWinMain` 解析命令行 → 决定 demo / 各类 self-test 模式。
2. **生产 presenter 宿主**：后台线程 `servePresentation` 建命名管道等服务 engine 推送
   `KeyResponse` 帧（protocol wire type 4）→ 解帧 → `PostMessageW` 投递到 UI 线程 →
   `update()` 构建模型/几何/窗口 → D2D `paintOnce()`。

代码分层非常规则（C++ 只剩 shell + renderer）：

- Rust 拥有并导出 opaque 状态机与纯函数（candidate-model / presentation / axis-layout /
  render-segments / hit-test / 标签格式化 / 选择 intent / 协议编解码 / config snapshot /
  utf8→utf16 / 当前进程号 / 深色外观 / candidate-select 客户端）。C++ 只做薄 FFI 包装、转译
  （marshal）与调用。
- C++ 真正自有的代码：HWND 生命周期 + WndProc 消息处理、D2D/DWrite/GDI 绘图与测量、
  命名管道服务端连接循环与逐字节解帧（`readExact`/`decodePresentationFrame`）、命令行为参数
  `fcitx5_candidate_parse_command_line_utf16` 的两段式查询拷贝、config snapshot → 原生绘制
  数值的投影（`loadVisualConfig`/`NativeRenderConfig`）、`layout_type` 词表解码 seam
  （`decodeNativeLayoutType`，080 §3 镜像）、线程编排、各 self-test 断言。

## 入口函数

`int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR, int)`（line 2936），流程：

1. `enableDpiAwareness()`（DpiAwarenessContext → 回落 `SetProcessDPIAware`）。
2. `parseCommandLine` → 校验 generation（env `FCITX5_RELEASE_GENERATION` 与
   `currentRuntimeGeneration()` 必须一致，否则退出码 1）。
3. `--candidate-select` 模式：直接 `runCandidateSelectionTest`（另起 peer 进程的 select 客户端，
   退出码 0/66/67）。
4. 其余模式：构造 `CandidateWindow`；`create()`（注册类 + CreateWindowExW + 定时器 + layered
   opacity + D2D/DWrite 资源）→ `paintOnce()`。
5. 分支：demo（合成预览）、interaction/uiless/scroll-expansion/locale/candidate-UX self-test、
   device-loss 模拟、reload-test（`SendMessageW` visual-config-change → 重载重绘）；`selfTest`
   模式直接返回 0。
6. 生产（`hasParentId`）：守护线程等 parent 退出 → `PostMessageW(WM_CLOSE)`。
7. `std::thread(servePresentation, window.handle(), testOnce).detach()` 起管道服务线程。
8. `run()`：`GetMessageW/TranslateMessage/DispatchMessageW` 消息泵直到 `WM_DESTROY`（
   `PostQuitMessage`），返回 wParam。

退出码约定：成功 0；自测失败 2；管道/创建失败 1（66/67 select 专用）。

## 拥有的状态（按职责分组）

单一 `CandidateWindow` 对象（内嵌所有状态，经 `GWLP_USERDATA` 绑到 HWND）+ 3 个文件级结构。

### CandidateWindow 成员（line 2741-2778，按职责分）

- **窗口/UI 线程**：`HWND window_`；`HWND targetForegroundWindow_`；`DWORD targetForegroundProcessId_`
  （点击归属校验）；定时器常量（`kFocusWatchTimer`=1 前台丢失检查 100ms、
  `kClickGuardTimer`=2 点击抑制 750ms）；两个已注册广播消息号
  （visual-config-changed、candidate-dismiss，全局限 1 的窗口消息）。
- **D2D/DWrite 资源（ComPtr）**：`d2dFactory_`（单线程工厂）、`writeFactory_`（shared）、
  `renderTarget_`（HwndRenderTarget）、`textFormat_`/`labelFormat_`/`annotationFormat_`
  （DWriteTextFormat，按 DPI/字号/locale 重建）。资源持有全在类内 RAII；recreate 即
  `Reset()` + `createDeviceResources()` 重建。
- **候选数据（中间态）**：`candidates_`（`CandidateVisual` = label/reservedLabel/text/comment +
  sourceLabel，UTF-16 已格式化）、`preeditPanel_`/`preeditPanelRect_`/`preeditDividerY_`。
- **几何/layout（本帧结果）**：`itemRects_`（D2D1_RECT_F）、`visibleIndices_`、
  `renderIndices_`（presentation render-plan 输出）、`resolvedPresentationOrientation_`、
  `hasScrollbar_`、`scrollbarTrack_`/`scrollbarThumb_`、`selectionInflateX/Y_`。
- **config（投影后）**：`NativeRenderConfig visualConfig_`（纯数值/颜色/字体族，D2D 友好型）、
  `safeMode_`。
- **模型/状态机句柄**：`candidate::CandidateModel model_`（包 Rust model opaque 句柄 +
  当前 `std::optional<Snapshot>` 镜像）、`void* presentation_`（Rust presentation 句柄）。
- **IPC**：`void* candidateClient_`（Rust candidate-select 客户端 opaque 句柄）、`CaretRect lastCaret_`、
  `std::wstring dwriteLocale_`、`std::string contentLocaleUtf8_`。
- **scroll/点击暂态**：`scrollOverridePx_`（wheel 累计，-1 未用）、`pressedCandidate_`、
  `clickInFlight_`、`interactionTest_`、`capturedTestIntent_`、`fontDpiScale_`。

### 类外文件级状态

- `Metadata`/`CaretRect`/`CandidateRecord`/`KeyResponse`（line 453-493）：管道帧解码后的自有
  镜像 DTO（wire codec 在 Rust，此处只投影）。
- `candidate::Item`/`Snapshot`/`ApplyResult`/`Visibility` + `CandidateModel` 包装（line 495-770）：
  进 Rust model 的输入/输出镜像。
- `candidate::detail` 的 `Fcitx5CandidateModelItem/Snapshot` 等 C ABI 结构（Rust 结构镜像）。

## 窗口生命周期（Create→Show→Update→Paint→Dismiss→Destroy）

1. **create**（line 1185）：`queryCurrentIdentity`（失败即退出）→ 生产模式创建 Rust
   candidate-select 客户端（pipe `engine`）→ `loadVisualConfig(safeMode)`（Rust config-core，
   失败返回 false）→ `RegisterClassW`（类名 `{local_object_prefix}.Candidate`，
   `lpfnWndProc=windowProcedure`，`CS_DROPSHADOW`，类只注册一次，从不 Unregister）→
   `CreateWindowExW(WS_EX_TOOLWINDOW|NOACTIVATE|TOPMOST[|LAYERED], WS_POPUP, ..., this)`
   （`this` 经 `lpCreateParams` 传入）→ 校验 exstyle（无 APPWINDOW、确有 TOOLWINDOW|NOACTIVATE）→
   `SetTimer(kFocusWatchTimer, 100ms)` → `enableNativeWindowEffects`（dwmapi
   DwmSetWindowAttribute 圆角 33）→ `SetLayeredWindowAttributes`（opacity 0.2-1.0 clamp）→
   `createDeviceResources()`（工厂 + 3 个 text format + HwndRenderTarget）。demo 模式
   `ShowWindow(SW_SHOWNOACTIVATE)`。
2. **Show/Place**：`update()` 内 `SetWindowPos(HWND_TOPMOST, x,y,w,h, SWP_NOACTIVATE|SWP_SHOWWINDOW)`
   + render target `Resize` + `InvalidateRect`。无独立的 show 方法；隐藏 =
   `ShowWindow(SW_HIDE)`。
3. **Update**（line 2052）：消息线程收到 `kSnapshotMessage`（payload 为 new 出来的
   `KeyResponse*`）→ `update()`：
   `applyContentLocale` → 组装 `candidate::Snapshot` → `model_.apply`（Rust dedupe/stale 判定，
   stale/invalid 直接 return）→ `presentation_apply`（2/3=duplicate/stale 直接 return）→
   DPI 变化时重建 format → UTF-8→UTF-16 转成 `CandidateVisual` 列表 → `render_plan` 得
   `renderIndices_` → 可见性/空/无 caret/`!popupAllowed` → dismiss/hide → 取 foreground 进程
   id → monitor work-area → `resolve_orientation`（Rust，含量词首/屏幕边/宽度启发式）→
   scroll 标签保留宽度测量（DWrite）→ 文本测量（DWrite TextLayout metrics，逐候选/逐段）→
   automatic 水平 → 超限回落垂直 → `axis_layout`（Rust，得窗口矩形+item 矩形+placement+
   滚动视口）→ `stable_window_width`（Rust 宽度滞后）→ 本地算 preedit 块/窗口 clamp →
   组 `itemRects_`/`visibleIndices_`/scrollbar rect → SetWindowPos → InvalidateRect。
4. **Paint**（line 1733 `paintOnce`）：`BeginPaint/EndPaint` 包夹（WM_PAINT 分支）。
   `createDeviceResources` 幂等 → `BeginDraw` → 高对比（`SystemParametersInfoW` +
   `GetSysColor`，override 全色板）→ Clear → 每帧 CreateSolidColorBrush×9 → preedit 面板 + 分隔线
   → `renderSegments`（Rust：label/text/comment 子矩形，fallback 两行"你/呢"演示数据）→ 滚动
   分页分隔线 → 选中圆角高亮 + DrawTextW×段（clip 不换行防行间重叠）→ scrollbar 轨道/滑块 →
   边框圆角 → `EndDraw`；`D2DERR_RECREATE_TARGET` → `Reset()`+重建。WM_PRINT/PRINTCLIENT →
   GDI 兜底 `paintToDeviceContext`（CreateFontW 单行 DrawTextW）；interaction-test 模式额外
   `paintTestSurfaceOverlay` 把同一帧画到真实 HDC（自动化截屏用）。
5. **Dismiss/Hide**：`hidePopup()` = SW_HIDE + 释放 capture + 清 pressed + KillTimer 点击守卫；
   `dismissPresentation()` = hidePopup + Rust `presentation_reset` + `model_.reset` + 清全部
   本帧缓冲 + 清 foreground 归属。触发：candidate-dismiss 广播（同 contextId 且同进程）/
   focus-watch 计时器发现前台进程已变 / model 陈旧 / 合成结束（visibility=hidden）。
6. **Destroy**：`WM_DESTROY` → `PostQuitMessage(0)` → `run()` 退出。无显式
   `DestroyWindow`/`UnregisterClass`；EXE 退出即回收。`~CandidateWindow`（析构）：
   destroy Rust candidate client + Rust presentation。Window 对象不显式析构（run() 返回后
   wWinMain 栈对象回收）。

## 主要 C++ 类型

- `detail::Fcitx5Candidate*` ABI 结构簇（line 36-260）：Rust C-ABI 输入/输出布局镜像
  （LayoutPoint/Size/Rect、RenderItemInput/Output、AxisLayoutInput/Output+ItemOutput、
  PresentationUpdate/Output/RenderPlan、Utf8/Utf16 借用视图、SelectionIntent、CommandLine、
  CommonUtf8ToWide）。
- `detail::Fcitx5CandidateModelItem/Snapshot/ScrollLabel`（line 525-560）：model ABI。
- `CandidateWindow final`（line 1172-2778）：唯一大类型 = 窗口+D2D+模型+config 所有权根。
- `candidate::CandidateModel final`（line 667）：Rust model opaque 句柄的 RAII 壳 +
  `std::optional<Snapshot> current_` 镜像。
- `NativeRenderColors`（line 772）/`NativeRenderConfig`（line 785）：config 投影后的绘制参数
  （D2D1_COLOR_F + float dip 数值 + 字体族 vector + 枚举）。`CandidateVisual`（line 847）：
  UTF-16 候选行。`KeyResponse`/`CandidateRecord`/`CaretRect`/`Metadata`（line 453）：
  帧投影 DTO。`ParsedCommandLine`（line 407）命令行。`candidate::Snapshot/Item`（line 500）。
- 文件级：`HandleDestroyer`（loadVisualConfig 内 RAII 局部，line 1113）；`Point/Size/Rect/
  Orientation/Placement/RenderItemInput/RenderItemSegments/CandidateSelectionIntent`（line 263-407，
  纯几何/意图值类型）。
- Win32/COM 对象均为 Microsoft::WRL::ComPtr 或裸 HWND/HANDLE/void* 句柄（看下一节）。

## 外部依赖（Rust FFI 调用）

ui_main.cpp 中 `extern "C"` 声明（均在 `namespace detail` 或文件内联），Rust 侧实现于
`rust/candidate-core`、`rust/windows-common-core`、`rust/config-core`、`rust/protocol-core`。

candidate-core（Rust 状态机 + 纯函数，共 27 个声明，文件内 `detail` 前缀两处声明同函数重复但签名一致）：
- `fcitx5_candidate_model_create/destroy/reset/apply/current` — 候选模型状态机
- `fcitx5_candidate_presentation_create/destroy/reset/apply/current/render_plan/set_placement/
  stable_window_width/resolve_orientation` — presentation 状态机（选中/滚动展开/宽度滞后/方向）
- `fcitx5_candidate_axis_layout` — 三轴几何（页面/滚动/换行+书写方向+placement）
- `fcitx5_candidate_render_segments` — 单候选 label/text/comment 子矩形
- `fcitx5_candidate_hit_test`、`fcitx5_candidate_selection_intent`、
  `fcitx5_candidate_format_label_utf16`、`fcitx5_candidate_parse_command_line_utf16`、
  `fcitx5_candidate_default_dwrite_locale_utf16`、`fcitx5_candidate_content_locale_valid_utf8/
  content_locale_or_default_utf16`、`fcitx5_candidate_locale_prefers_compact_horizontal_utf8`、
  `fcitx5_candidate_scroll_label_policy`

windows-common-core：
- `fcitx5_windows_common_utf8_to_wide_utf16`（多处文本转码）
- `fcitx5_windows_common_current_process_id`（interaction-test 归属）
- `fcitx5_windows_common_system_uses_dark_appearance`
- `fcitx5_windows_common_candidate_select_client_create_utf16/select/destroy`（声明在
  src/ipc/candidate_select_client.h，select 点击投递）

config-core（声明在 src/config/config_snapshot_ffi.h）：
- `fcitx5_config_snapshot_load_visual_utf16/destroy/view/candidate_label_at/font_family_at/
  candidate_color_at` — resolved visual 快照；常量 kFcitx5ConfigFont{UI,Candidate,Annotation,
  Monospace}。Fcitx5ConfigSnapshot 含大量已折旧兼容字段（candidateOrientation/ScrollMode 等
  "legacy projection"，注释标明 cutover 后删）。

protocol-core（声明在 protocol/protocol_ffi.h）：
- `fcitx5_protocol_core_decode_header`、`fcitx5_protocol_core_decode_key_response` — 帧解码
  （FcitxMetadataC/FcitxBytesC/FcitxKeyResponseC/FcitxCandidateRecordC；decode_key_response 两段式：
  先探需量再填 arena）。

C++ include 进来的 C++ 依赖头：
- `src/ipc/peer_verification.h` — `ipc::verifyPipeClient(HANDLE, RuntimeIdentity, ProcessIdentity*)`
- `src/platform/pipe_security.h` — `PipeSecurity::create`/attributes（SECURITY_ATTRIBUTES）
- `src/platform/runtime_identity.h` — `queryCurrentIdentity`/`makeLocalEndpointName`/
  `pathsReferToSameFile`/`currentRuntimeGeneration`
- `fcitx5_windows/release_identity.h`（cmake 生成）— `kReleaseIdentity.local_object_prefix`、
  `.data_directory`

## Win32/COM/TSF 依赖（逐个调用）

Win32（调 user32/gdi32/kernel32/dwmapi，来自 grep 统计）：

- 进程/启动：`SetProcessDpiAwarenessContext`（动态 GetProcAddress+LoadLibraryW user32，
  handle -4）、`SetProcessDPIAware`（回落）、`SetEnvironmentVariableW`（generation 校验）、
  `OpenProcess(SYNCHRONIZE)`/`WaitForSingleObject`/`CloseHandle`（parent 存活看门狗线程）。
- 窗口类/消息：`RegisterClassW`、`CreateWindowExW`、`LoadCursorW(IDC_ARROW)`、
  `RegisterWindowMessageW`×2（自定义广播）、`GetMessageW`/`TranslateMessage`/`DispatchMessageW`、
  `PostQuitMessage`、`DefWindowProcW`、`SetWindowLongPtrW(GWLP_USERDATA)`×1/`GetWindowLongPtrW`
  （含 GWL_EXSTYLE 读）、`PostMessageW`×4、`SendMessageW`×7（WndProc 外自测/重载触发）。
- 窗口操作：`ShowWindow(SW_SHOWNOACTIVATE/SW_HIDE)`、`SetWindowPos(HWND_TOPMOST)`、
  `InvalidateRect`、`RedrawWindow`、`IsWindowVisible`、`SetWindowPos`（DPI-changed 按建议 rect 重置、
  render target SetDpi 96）。
- 前台/点击/焦点：`GetForegroundWindow`、`GetWindowThreadProcessId`、`SetCapture`/
  `GetCapture`/`ReleaseCapture`、`SetTimer`/`KillTimer`（100ms 焦点监视 + 750ms 点击守卫）、
  `ClientToScreen`（自测坐标）。
- 屏幕/监控：`MonitorFromPoint`、`GetMonitorInfoW`、`GetSystemMetrics(SM_CXSCREEN)`（自测）、
  `GetWindowRect`（自测取宽）、`GetClientRect`。
- 绘制（GDI 兜底路径 WM_PRINT / interaction 截屏 + 高对比查询）：`BeginPaint`/`EndPaint`/
  `GetDC`/`ReleaseDC`、`GetSysColor`×12、`SystemParametersInfoW(SPI_GETHIGHCONTRAST)`×2、
  `CreateSolidBrush`/`DeleteObject`、`CreateFontW`/`SelectObject`（HFONT/HGDIOBJ 手工 RAII，
  用后还原再删）、`FillRect`、`DrawTextW`、`SetBkMode`、`SetTextColor`、
  `CompareStringOrdinal`×5（locale 相等判断）。
- 管道服务端（后台线程）：`CreateNamedPipeW`（PIPE_ACCESS_INBOUND|REJECT_REMOTE_CLIENTS）、
  `ConnectNamedPipe`、`DisconnectNamedPipe`、`ReadFile`（阻塞逐字节读，`readExact` 循环）、
  `GetLastError`（ERROR_PIPE_CONNECTED 处理）。
- DWM：`LoadLibraryW("dwmapi.dll")`/`GetProcAddress`/`FreeLibrary` +
  `DwmSetWindowAttribute(33, DWMWA_WINDOW_CORNER_PREFERENCE round)`（动态绑定）。

COM/D2D/DWrite（wrl/client.h `ComPtr` RAII，接口调用）：
- `D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED)` → `CreateHwndRenderTarget`；
  render target：`BeginDraw`/`EndDraw`/`Clear`/`CreateSolidColorBrush`×9/`DrawTextW`/
  `DrawLine`/`FillRoundedRectangle`/`DrawRoundedRectangle`/`GetSize`/`Resize`/`SetDpi`/`Reset`。
- `DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, IDWriteFactory)` →
  `CreateTextFormat`×3（SetWordWrapping NO_WRAP、SetTrimming、SetTextAlignment）/
  `CreateTextLayout`×4（测量 metrics）→ `CreateEllipsisTrimmingSign`/`GetMetrics`。
- 无 TSF/COM 注册；无 ole32 直接使用（链接但未见调用）。`WS_EX_LAYERED`+`SetLayeredWindowAttributes`
  （非 interactionTest）。

## 线程模型

- 主线程（wWinMain 所在）：窗口创建、消息泵、全部 `update()`/`paintOnce()`/WndProc。
  所有 HWND/D2D/DWrite/模型访问都在这条线程。
- `servePresentation` 线程（`std::thread(...).detach()`，line 3005）：阻塞管道服务循环
  （单实例管道，逐连接逐帧读）。与 UI 线程唯一的同步机制 = `PostMessageW(kSnapshotMessage,
  reinterpret_cast<LPARAM>(new KeyResponse))`（投递即移交所有权，失败 `return` 导致消息泄漏，
  但进程级退出的退出路径下可忽略——本文件无所有权表/引用计数）。
  解码/管道状态全部只在此线程。peer 校验成功才进帧循环；校验引擎路径用
  `pathsReferToSameFile(peer.executablePath, engine)`。
- parent 看门狗线程（`std::thread(...).detach()`，line 2998）：`WaitForSingleObject(parent)`
  → `PostMessageW(WM_CLOSE)`。
- 无 mutex/atomic/临界区。跨线程对象只有一个：KeyResponse 帧所有权经消息传递；HWND 句柄值
  跨线程使用（PostMessage target 合法）。
- 两个计时器驱动 UI 内逻辑：focus-watch（100ms）与 click-guard（750ms），全部主线程回调。

## 可以直接迁 Rust 的部分

1. **帧解码层**：`readExact`+`decodePresentationFrame`+帧长度校验（type 4、64B header、
   256KiB 上限、两段式 decode）— 纯字节→结构逻辑，Rust protocol-core 已有权威 codec；
   C++ 侧只是把 arena 拷贝成 std::string。直接换成 Rust 的 key-response 解码 + String。
2. **KeyResponse → Snapshot 组装**（`update()` 前段）+ `CandidateModel` 壳全部逻辑 —
   本就是 Rust model 的镜像；C++ 镜像类型（`Snapshot`/`Item`/`Metadata`）可整体替换成
   Rust `candidate_model`/`presentation` 的 native 调用。
3. **方向/宽度/scroll 决策编排**（`update()` 中段：resolve_orientation、宽度回落、
   stable_window_width、label 保留宽度策略、scroll_label_policy）— 逻辑已在 Rust；
   C++ 只做参数 marshalling。
4. **配置投影**：`loadVisualConfig` 中 `decodeNativeLayoutType`（080 §3 seam）、label 样式/
   preedit 样式词表、`nativeColor` 解析 — 与 config-core 的 `decode_candidate_layout_options`
   重复。Rust 侧已有权威实现；投影层只需要拿 resolved 数值。
5. **纯计算/枚举转换层**：`toRust`/`fromRust`/`placementFromRust`、`renderSegments`、
   `hitTestCandidate`、`makeCandidateSelectionIntent`、`utf8ToWide`、`formatCandidateLabel` —
   全部只是把 Rust 结果搬进 C++ 值；迁后直接消失。
6. **自测**（7 个 self-test 方法 + demo 数据）：全部通过公开 `update()` 路径驱动真实
   presentation/model；逻辑在 Rust，C++ 只有 KeyResponse 构造与断言。Rust 侧可直接用
   `presentation`/`model` 状态断言；interaction/uiless 两测试已按窗口可见性断言，需保留
   HWND 探针。
7. **presentation 状态机的所有查询**（selected/scrollMode/columns/placement，封装在
   `presentationState()`）— Rust 原样。

## 暂时不适合迁 Rust 的部分（必须保留 C++ 的理由）

1. **HWND + WndProc + 消息泵**（含 SetWindowLongPtrW GWLP_USERDATA 绑定、WM_NCCREATE this
   注入、WM_PAINT/WM_PRINT/WM_TIMER/WM_DPICHANGED/WM_SETTINGCHANGE/THEMECHANGED/
   SYSCOLORCHANGE/WM_MOUSEACTIVATE/WM_LBUTTON*/WM_MOUSEWHEEL/WM_CANCELMODE/
   CAPTURECHANGED/NCHITTEST 分支）：迁 Rust 必须整窗迁移（windui 或 win32 Rust 封装），
   与现有 C++ D2D renderer 无法共存——这正是 081 后续 slice C 的边界，不是 slice A 可单独
   动的部分。081 计划本身在 slice C 迁。
2. **D2D/DWrite 绘制与测量**：`paintOnce`（D2D）+ `paintToDeviceContext`（GDI fallback，
   WM_PRINT/自动化截屏路径）+ TextLayout 测量。设计上就是 C++ 绘图栈；tiny-skia cutover
   替换成 Rust 渲染器前保留。注意 WM_PRINT/GDI 兜底与 interaction-test 截屏管道是 Rust
   renderer 必须复刻的契约（080 acceptance 里有"visual+DPI evidence"）。
3. **高对比模式探测 + GetSysColor 取样**：读系统状态直接取 COLORREF，与 D2D 目标紧耦合；
   属于渲染器的一部分。
4. **管道服务端线程**（create/accept/verify/读帧）：当前实现跨 C++ 线程 + Rust 编解码。
   迁 Rust 时若保留 named-pipe 方案应整段进 Rust（重连/校验/安全属性），因为 pipe security/
   identity 验证全在 platform/ipc C++ 库——除非已有对应 Rust seam。081 无此 slice，留待
   引擎侧 IPC cutover。
5. **D2D 资源生命周期策略**：懒建工厂/按 DPI+config 重建 format、`D2DERR_RECREATE_TARGET`
   自愈、幂等 `createDeviceResources` — 属于 renderer 内部。

## 建议迁移边界（按职责拆分）

现况已近于"纯净 C++ shell + 纯 Rust 状态机"，因此边界可按 081 计划 + 本文件天然接缝切成：

```
rust/candidate-core/
  renderer.rs          # slice B：tiny-skia 绘制（paintOnce 等价），输入 =
                       #   RenderFrame{候选行(已格式化 UTF-16/像素 rect)、选择下标、scrollbar、
                       #   colors、dip 尺寸}
  window.rs            # slice C：HWND 生命周期 + 消息泵 + WndProc 分发（D2D 外全部消息）
  wm_dispatch.rs       # slice C：WM_* → 事件 enum（paint/print/click/dismiss/config-changed/
                       #   dpi/timer/wheel），与绘制解耦
  frame_decode.rs      # slice A：pipe 帧 → KeyResponse（取代 readExact+decodePresentationFrame）
ui_main.cpp → 薄 main()：        # slice C/D 后删：参数解析(cmdline 已 Rust) + 自测路由 +
                                 #   windows-common utf8 桥，不再拥有窗口/绘制
```

明确切片顺序（对照 081）：

1. **Slice A（本调查服务对象）**：把 `paintOnce` 的 D2D 代码提成可测纯绘制函数（输入输出
   结构化，不动 HWND 生命周期）—— C++ 侧重构，为 slice B 提供等价参照。
2. **Slice B**：Rust `renderer.rs` 产出同一组 rects/颜色/字体（从 snapshot ABI +
   axis_layout 结果直接驱动），双轨差分（golden/像素对比，命名 Rust cutover 任务删除）。
3. **Slice C**：Rust `window.rs` 整窗迁移（类注册、CreateWindowExW、消息分发、timer、
   layered、dpi、mouse 交互、前台归属校验、点击 intent→candidate-select 客户端）；此步删
   WndProc + GWLP_USERDATA 绑定。
4. **Slice D**：删 `ui_main.cpp` + `fcitx5_ui` C++ target；测试切到 Rust 权威（081 frozen
   acceptance 各条逐一映射）。遗留验证契约：WM_PRINT/GDI 路径（若产品仍需 printer 兜底）、
   interaction self-test 的 HWND 可见性断言、reload/device-loss 触发路径。

关键前置约束（迁移时勿破坏）：
- 单窗口不可激活（MA_NOACTIVATE/WS_EX_NOACTIVATE），点击不抢前台，归属 = 最后一次
  foreground 进程 id + focus-watch 轮询。
- `SetLayeredWindowAttributes` 只在非 interactionTest；interaction 模式开真实 HDC 截屏路径。
- 宽度滞后/方向自动决定是 presentation（Rust）契约，C++ 侧逻辑只在被删的 update() 编排层。
- Rust 状态机被 C++ 通过 opaque 句柄轮流调用（apply/current/render_plan 同帧多次查询）；
  迁 Rust 内部化后这些全部变成内部字段，无 ABI 磨损。
- renderer 每帧 9 个 brush + 每帧创建 TextLayout 做测量：tiny-skia 参照实现要保留
  clip/不换行/行间不重叠 的绘制语义（注释明示防重叠是故意行为）。
