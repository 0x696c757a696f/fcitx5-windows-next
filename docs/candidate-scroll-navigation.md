# 候选窗对齐与卷轴（scroll）模式导航 —— 最近改动说明

> 覆盖提交：`6718aad` → `290ae49` → `bd48781` → `3581606` → `69a57aa` →（`a938eaf` 已撤回）→ `35432aa`（Revert）→ `f3c472d`
> 文档日期：2026-08-19　·　当前基线：`f3c472d`

## 1. 背景与目标

为 v1.6 工程规格的候选窗补齐**搜狗式导航**体验：

- label 列宽统一对齐（不再是参差不齐的左缘）；
- 数字行 `,`/`.`、`-`/`=`、`[`/`]` 翻页；
- `;`/`'` 直接选择第 2/3 个候选；
- `←`/`→` 在高亮之间移动焦点（不提交）；
- **卷轴模式**（`scroll_mode=true` 且输入法提供真 bulk 候选）下，`↓`/`↑` 按**行**滚动（保持列），`+`/`-` 跳**行首**，视口滚动跟随高亮，翻屏有明确边界语义。

用户在实测中反馈的两个问题已修复：① 滚动后高亮不动（"第二排显示成第一排"）；② 移动高亮后 Enter/Space 提交的不是高亮候选。

## 2. 当前行为（用户视角）

### 2.1 普通分页（非 bulk / 未开卷轴）

| 按键 | 行为 |
|---|---|
| `,` / `[` / `-` | 上一页 |
| `.` / `]` / `=` | 下一页 |
| `↓` / `↑` | Fcitx 默认：下一页 / 上一页 |
| `;` / `'` | 选择第 2 / 第 3 个候选 |
| `←` / `→` | 焦点左移 / 右移（不提交） |
| `1`-`9` | 选择对应候选（Fcitx 默认） |

### 2.2 卷轴模式（`scroll_mode=true` + bulk 候选，`totalSize() >= 0`）

| 按键 | 行为 |
|---|---|
| `↓` / `↑` | 下一排 / 上一排，**保持列位置**（第 3 列 → 下一排第 3 列） |
| `+` / `-`（及 `.` `,` `]` `[` `=`） | 下一排 / 上一排，落**行首**（第 1 列） |
| 到第 6 排后再按 `↓` / `+` | 翻屏到新屏第 1 排（`↓` 保持列；`+` 行首）；无更多页时**保持原位** |
| `-` / `↑` | 回上一屏（同样的列/行首规则） |
| `←` / `→` | 焦点在 6 列网格内左右移动 |
| `;` / `'` | 选择第 2 / 第 3 个候选 |
| Enter / Space | 提交**当前高亮**的候选 |

- 视口滚动跟随：高亮所在排滚动后成为视口首排（`firstRow = selectedRow`）。
- 候选少于 6 时网格部分填充；候选不足一行时按行内移动并 clamp 到最后一个候选。
- 边界语义：越界且有更多页 → 翻屏（新屏第 1 排，列/行首规则如上）；无更多页 → 高亮停在末尾/原位，不回跳第 1 行。

### 2.3 对齐

- 渲染前测量最宽的可见 label，所有行的 text 起点统一 = `bounds.left + labelColumnWidth`；comment 在 text 之后 +4px。选中行同样对齐（不再因背景色导致文字错位）。

## 3. 技术实现

### 3.1 engine —— `src/engine/fcitx_runtime.cpp`

**按键符号（keyFromRequest）**
- `VK_SHIFT/CONTROL/MENU` → `Shift_L/Control_L/Alt_L`；
- `VK_OEM_PLUS/MINUS` → `plus/equal/underscore/minus`（按 Shift）；
- `VK_OEM_COMMA/PERIOD/1/7/4/6` → `comma/period/semicolon/apostrophe/bracketleft/bracketright`（无 Shift）及 `<` `>` `:` `"` `{` `}`（Shift）。

**候选键处理（processKey 的翻页块，`page_keys_done:` 标签）**
- 判定：
  - `nextPage`：`=` `+` `.` `]`；
  - `prevPage`：`-` `_` `,` `[`；
  - `upDown`：`Up` / `Down`；
  - `scroll = bulk && totalSize() >= 0`（真 bulk）；
  - `scrollNext = nextPage || (scroll && Down)`；`scrollPrev = prevPage || (scroll && Up)`。
- **scroll 分支独立于 `toPageable()`**（`f3c472d` 修复）：只要是真 bulk，`↓`/`↑`/`+`/`-` 都进入滚动分支并移动高亮——即使列表没有 pageable 也不会落入 Fcitx 默认翻页。
  - 目标行计算：`upDown` → `cursor ± 6`（保持列）；`+`/`-` → `(cursor/6)*6 ± 6`（行首）。
  - 越界：有 pageable 且还有页 → 翻页后落新屏第 1 排（`↓`/`↑` 保持列 `cursor%6`，`+`/`-` 行首 `0`）；无更多页 → clamp 到 `available-1`（不回跳第 1 行）。
  - 结果写入 `selectedOverride[key]` 并 `event.filter()`。
- 普通分页（非 scroll）：`=/-.,[]` 走 pageable 翻页（`next/prev`），翻页后 `selectedOverride=0`（高亮跳页首）。
- `;`/`'`：`candidate.select(&context)`，`filterAndAccept`，清理 override。
- `←`/`→`：焦点 `±1` 并 clamp 到 `[0, bounded-1]`，写 override，不提交。
- **Enter/Space（`f3c472d` 修复）**：若该上下文存在 override，提交**高亮候选**（`candidate.select`），`filterAndAccept` 并清 override——避免提交落到 Fcitx cursor（仍停在第 1 个）上。

**选中覆盖（selectedOverride）**
- `Impl` 成员：`unordered_map<ClientContextKey, optional<uint32_t>>`，按"客户端上下文 + 按键"分别记忆焦点；
- `collectResult`：`cursor = selectedOverride[key]`（存在时覆盖），再校验 `0 <= cursor < size` 后写 `selectedCandidate`；
- `forgetConnection`：清理该上下文的 override 与输入法切换状态。

**bulk 分页状态**
- `collectResult`：`candidatePageSize = list->size()`（页内候选数），`candidatePage = pageable ? pageable->currentPage() : 0`；
- 真 bulk 时候选总数 `totalSize()` 参与 `kMaxCandidates` 截断（`bounded`）。

### 3.2 UI —— `src/ui/ui_main.cpp` / `src/ui/candidate_layout.cpp`

- **scrollEligible**：`scroll_mode && candidateBulk && candidatePageSize > 0`；
- **scrollExpanded**：首次进入 composition 时要求 `candidatePage > 0`；翻页页号变化时保持/关闭（`1 → 0` 视为关闭）。单页 bulk（candidatePage 恒 0）不展开卷轴——这是已知限制（见 §5）。
- **scrollMode_ = scrollEligible && scrollExpanded_**：开启时按 6 列网格渲染、行 label 只在选中行显示、行高选中行加高、视口滚动跟随。
- **对齐**：paint 前遍历可见候选测最宽 label → `labelColumnWidth`；`drawTextAt(text, bounds.left + labelColumnWidth)`；comment 起点再 +4px。
- **视口跟随**：`candidate_layout.cpp` 的 scroll 布局 `firstRow = min(selectedRow, rows - visibleRows)`（高亮排滚出视口时整窗滚动，当前排变首排）。

### 3.3 键路由（TSF 层，前期已定）

- `shouldRouteToEngine` 只过滤 Win+*/F1-F24/左 Alt（除 Alt+Shift），其余按键全部路由给 Fcitx 由 engine 决定；key-up 带 `kKeyFlagRelease`。

## 4. 提交演进（含一次撤回）

| 提交 | 内容 | 状态 |
|---|---|---|
| `6718aad` | label 列宽统一对齐；`;`/`'` 选择、`←`/`→` 焦点、`,`/`.`/`[`/`]` 翻页；OEM 键映射 | 保留 |
| `290ae49` | 翻页键在卷轴内移动**一行**并落行首 | 保留 |
| `bd48781` | 卷轴未展开（candidatePage=0）时先翻页展开，再行滚动 | 保留 |
| `3581606` | `↓`/`↑` 卷轴内按**行**滚动并**保持列** | 保留 |
| `69a57aa` | 翻屏边界：新屏第 1 排保持列/行首；无更多页保持原位 | 保留 |
| `a938eaf` | 展开放宽（有 bulk 即开 6×6）+ 高亮必动（含 clamp） | **已撤回**（用户实测"没用"，且改变了拼音显示布局） |
| `35432aa` | Revert `a938eaf` | 保留 |
| `f3c472d` | scroll 分支移出 `toPageable()`（bulk 无 pageable 也滚动高亮）；Enter/Space 提交高亮候选 | 保留（当前基线） |

## 5. 验证情况

- `ctest --test-dir out/build/windows-x64-dev -C Debug`：**53/53 全绿**；
- `tools/build.ps1 test -x64 -Release`：runtime-security / secrets / licenses / locales / text-format 全过；
- `fcitx5_engine_integration_test.exe out/stage/fcitx5/bin/fcitx5-engine.exe`：exit=0（含导航块 `;`/`'`/`←`/`→` 断言）；
- `fcitx5-ui --self-test --reload-test` / `--interaction-self-test` / `--scroll-demo`：exit=0（demo 验证 6×6 布局与视口跟随、行起点对齐）；
- 用户实测：已修复"第二排显示成第一排"（滚动分支不再落入 Fcitx 默认翻页）与"Enter/Space 提交错误"（提交高亮候选）；待用户确认最终滚动/翻屏手感。

## 6. 已知限制与待确认

- **Rime 伪 bulk**：`totalSize() == -1` → `candidateBulk=false` → 不进卷轴（普通分页）。这是 fcitx 侧 Rime addon 不提供真 bulk API 所致，非前端可解。
- **单页 bulk 不展开卷轴**：UI 首次要求 `candidatePage > 0`；候选不足一页（≤ pageSize）时仍是普通分页布局（此前尝试放宽会改变拼音显示布局，已撤回）。
- **集成测试环境 pinyin 词典极简**（每音节 ≤7 候选、一页）：scroll/翻页无法在 CI 断言，靠 demo、代码审查与用户实测。
- 待用户确认：卷轴内的逐排滚动、翻屏边界、Enter/Space 提交高亮这三项的实际手感。
