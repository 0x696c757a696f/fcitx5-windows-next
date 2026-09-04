# 080 layout naming + full-glyph contract (frozen)

**Decision date:** 2026-09-04
**Sources:** fcitx5-macos#164 (scroll mode 卷轴模式), rimeinn/rabbit (stacked/flow/vertical_text).

## 1. Candidate layout type (unified setting)

Replace the current `orientation` (`automatic`/`horizontal`/`vertical`) + `scroll_mode` (bool)
pair (and the 6-way `CandidateLayoutMode`) with ONE `layout_type` setting. Naming mirrors the two
references while keeping the existing capability:

| `layout_type` key | 中文 label | 实现语义 | 旧值映射 |
|---|---|---|---|
| `automatic` | 自动 | caret + 显示器工作区自动选纵排/横排 | `orientation=automatic, scroll=false` |
| `stacked` | 纵排 | 候选逐行纵向列表（每行一个候选，文字横排） | `orientation=vertical, scroll=false` |
| `flow` | 横排 | 候选横向排列，超宽时多行分页 | `orientation=horizontal, scroll=false` |
| `scroll` | 卷轴 | 固定网格（6×N 或 N×6），带滚动条 | `scroll_mode=true`（方向由 grid 形状决定） |
| `vertical_text` | 竖排文字 | 文字自上而下竖排，列从左→右或右→左（可配置） | 新增，对应 rabbit `vertical_text` + `vertical_text_left_to_right` |

`scroll` 模式的网格形状（6×N 列优先 vs N×6 行优先）由 `page_size` + 现有 scroll 方向语义决定，
不新增单独的方向设置。

## 2. Full-glyph visibility contract (全字形可见性)

候选字必须完整显示，禁止裁切/重叠/截断：

- CJK 字形：完整 advance + DirectWrite 边距（预算完整字形宽度，含 comment 相邻）。
- emoji：彩色 emoji / 彩色字形回退完整渲染，不裁切。
- 注释（comment）：与候选文本并列时完整可见，不重叠、不截断。
- 所有 `layout_type`（automatic/stacked/flow/scroll）× HiDPI（100/125/150/200%）× light/dark 组合下成立。
- 渲染尺寸计算用完整字形预算（`cjk_text_rect_keeps_full_glyph_budget_beside_comment` 语义），
  不依赖 `non-overlap` 假阳性。

## 3. HiDPI contract

- 候选窗口在 100/125/150/200% DPI 下正确缩放（字体、间距、圆角、滚动条、边框、选中区域）。
- 移除 C++ `createDeviceResources` 的 `SetDpi(96,96)` 硬编码；Rust 渲染器用实际 DPI scale
  （caret/monitor dpi）驱动几何 + 字体。
- golden 截图覆盖四档 DPI。

## 4. Config setting surface

- Config 设置项显示为：`自动 / 纵排 / 横排 / 卷轴 / 竖排文字`（radio/segmented），key 用 `layout_type`。
- 中文 label 与实现语义严格一致：“纵排”= 纵向列表（非竖排文字），“横排”= 横向流式，
  “卷轴”= 固定网格 + 滚动条，“竖排文字”= 文字自上而下（列从左→右或右→左可配）。
- 旧配置迁移：`orientation=vertical`→`stacked`，`horizontal`→`flow`，`scroll_mode=true`→`scroll`。

## 5. Acceptance

- `rust/config-core` 设置项 `layout_type` + 中文 label 冻结；旧值迁移测试绿。
- `rust/candidate-core` 四种布局（stacked/flow/scroll/vertical_text）+ 四档 DPI 的 golden 截图，候选字
  （CJK/emoji/comment）完整可见。
- 渲染迁移（080）后等价视觉差分通过。
