# 高分屏（Hi-DPI）支持方案

状态：plan（2026-08-18）
范围：config 设置窗口 + 候选窗审计结论 + launcher 托盘

## 1. 现状审计

| 组件 | 感知模式 | 现状 | 结论 |
| --- | --- | --- | --- |
| 候选窗（fcitx5-ui） | PMv2（`SetProcessDpiAwarenessContext(-4)`） | 每次快照按 caret 所在显示器 `dpi/96` 重建字体格式（`fontDpiScale_`），布局输入（itemPadding / maxWidth / geometry）全部乘 `scale`；D2D 按物理像素渲染 | ✅ 已成熟 |
| 设置窗口（fcitx5-config） | PMv2（`enableDpiAwareness()`） | 控件坐标全部固定物理像素（`add(... 12, 82, 180, 34 ...)`）；字体 `CreateFontIndirect` 未按 DPI 缩放；无 `WM_DPICHANGED` 处理 | ❌ 150%/200% 屏界面偏小、控件拥挤、跨显示器拖动错位 |
| 托盘/启动器 | 系统菜单 | 菜单与图标由系统按 DPI 处理 | ✅ 无问题 |

## 2. 成熟方案（Windows 官方共识，weasel 设置窗口同款）

### 2.1 感知模式
- 已用 Per-Monitor V2：每显示器独立 DPI；窗口跨屏移动时收到 `WM_DPICHANGED`。

### 2.2 手动 DPI 缩放（手建 Win32 控件的标准做法）
- 窗口创建/DPICHANGED 时取 `scale = GetDpiForWindow(m_hWnd) / 96.0f`。
- 所有控件坐标与尺寸乘以 `scale`（等价于对话框 DLU 的自动换算）。
- 字体：`LOGFONTW::lfHeight` 乘 `scale`（`CreateFontIndirect`）。
- 窗口初始尺寸（当前 1010×650）乘 `scale`。

### 2.3 WM_DPICHANGED
- 处理 `WM_DPICHANGED`：wParam 低 16 位为新 DPI；lParam 为建议新窗口矩形。
- 按新 `scale` 重排控件（销毁重建子控件或统一重算坐标），`SetWindowPos` 应用建议矩形。
- 关键：**跨屏拖动不错位、不模糊**。

### 2.4 候选窗增强（可选）
- 已按 caret DPI 重建，无需改动；若窗口跨屏移动且无新快照，可在 `WM_DPICHANGED` 里触发一次按新 DPI 重排（低优先级）。

## 3. 实施步骤（config，预估工作量）

1. `ConfigWindow` 增加 `dpiScale_` 成员与 `reflowControls()`：
   - `add()` 内部按 `dpiScale_` 缩放坐标（改一处，全窗口生效）。
   - 字体创建按 `dpiScale_` 缩放 `lfHeight`。
   - `ResizeClient(1010 * scale, 650 * scale)`。
2. 消息映射加 `MESSAGE_HANDLER(WM_DPICHANGED, onDpiChanged)`：
   - 更新 `dpiScale_` → 销毁重建子控件（`onCreate` 抽出可重入的建控件逻辑）→ `SetWindowPos` 应用新矩形 → `InvalidateRect` 整窗重绘。
3. 交互自测扩展：`--ui-contract-test` 校验在 96/120/144/192 DPI 下控件可见与布局不越界（可选）。

## 4. 验收

- 150%（120 DPI）、200%（144 DPI 或 192 DPI）显示器上 config 界面控件清晰、布局正常、文字不截断。
- 窗口从 100% 显示器拖到 200% 显示器，界面即时按新 DPI 重排，无错位/模糊。
- 候选窗在 150%/200% 屏跟随 caret 缩放（既有行为，回归确认）。
