# 候选窗对齐与卷轴（scroll）模式导航 —— 最近改动说明

> 覆盖提交：`6718aad` → `290ae49` → `bd48781` → `3581606` → `69a57aa` →（`a938eaf` 已撤回）→ `35432aa`（Revert）→ `f3c472d`，以及 2026-08-19 实测后的工作树修正
> 文档日期：2026-08-19　·　提交基线：`f3c472d`

## 1. 背景与目标

为 v1.6 工程规格的候选窗补齐**搜狗式导航**体验：

- label 列宽统一对齐（不再是参差不齐的左缘）；
- 数字行 `,`/`.`、`-`/`=`、`[`/`]` 翻页；
- `;`/`'` 直接选择第 2/3 个候选；
- `←`/`→` 在高亮之间移动焦点（不提交）；
- **卷轴模式**（`scroll_mode=true` 且输入法提供真 bulk 候选）下按当前布局方向滚动：横向是按行展开的网格，纵向是按列展开的网格；数字键始终选择当前高亮行/列里的 label 对应候选。

`f3c472d` 后继续实测发现四类问题：数字标签与实际选择不是同一套索引；Enter/Space 缺少“提交文字等于高亮文字”的回归断言；每次按 `+`/`-` 都会把目标行顶到六行视口的第一行；`+`/`-`、`,`/`.`、`[`/`]` 等行移动键会被 Windows frontend 处理后又继续交给 Fcitx 处理，造成 UI snapshot 与最终提交候选错位。当前工作树按 v1.6 规格修正：候选 label 由 engine snapshot 唯一提供，UI 不再生成选择标签；卷轴行移动键在 engine 产出新 snapshot 后立即消费并返回。

## 2. 当前行为（用户视角）

### 2.1 普通分页（非 bulk / 未开卷轴）

| 按键 | 行为 |
|---|---|
| `,` / `[` / `-` | 上一页 |
| `.` / `]` / `=` | 下一页 |
| `↓` / `↑` | Fcitx 默认：下一页 / 上一页 |
| `;` / `'` | 选择第 2 / 第 3 个候选 |
| `←` / `→` | 焦点左移 / 右移（不提交） |
| `1`-`9` | 选择 engine/Fcitx label 对应候选 |

### 2.2 卷轴模式（`scroll_mode=true` + bulk 候选，`totalSize() >= 0`）

| 按键 | 行为 |
|---|---|
| 横向 `↓` / `↑` | 下一排 / 上一排，**保持列位置**（第 3 列 → 下一排第 3 列） |
| 纵向 `↓` / `↑` | 在当前列内上一/下一候选，不切到前后列 |
| 横向 `←` / `→` | 焦点在当前行内左移 / 右移 |
| 纵向 `←` / `→` | 前一列 / 后一列，保持当前行号 |
| `+` / `-`（及 `.` `,` `]` `[` `=`） | 横向切上一/下一排行首；纵向切上一/下一列列首 |
| `1`-`N` | 横向选择高亮行第 1 到第 N 个候选；纵向选择当前列第 1 到第 N 行候选；N 为 `candidate.page_size` |
| `;` / `'` | 选择当前行/列第 2 / 第 3 个候选 |
| Enter / Space | 提交**当前高亮**的候选 |

- 横向卷轴显示 `candidate.page_size × 6 行`；纵向卷轴显示 `6 列 × candidate.page_size 行`。
- `candidate.page_size` 在设置里显示为“每行/每列候选数”：横向表示每行候选数，纵向表示每列候选数，范围 1–9。
- `candidate.scroll_cell_width_dip` 控制卷轴模式单个候选格子的目标宽度（默认 96 DIP，范围
  40–160）。这对应 fcitx5-macos webpanel 的 `ScrollCellWidth` 思路：卷轴格子宽度有边界，长候选
  用 DWrite 单行省略显示，不能把整张候选窗撑成超宽横幅，也不能把短候选压成半个字形。
- 横向六行视口在内部移动时保持稳定；只有高亮跨过六行边界时才翻屏，最后不足六行时向前回填以保持窗口高度。纵向按列分组，最后一列不足时仍只显示该列。
- 末行/末列不足 N 个时，不存在的数字标签不会选择越界候选。
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
  - `scroll = scroll_mode && bulk && totalSize() >= 0`（启用卷轴且为真 bulk）；
  - `dimension = candidate.page_size`（横向为每行数，纵向为每列数）；
  - `scrollNext = nextPage || (scroll && Down)`；`scrollPrev = prevPage || (scroll && Up)`。
- **scroll 分支独立于 `toPageable()`**（`f3c472d` 修复）：只要是真 bulk，`↓`/`↑`/`+`/`-` 都进入滚动分支并移动高亮——即使列表没有 pageable 也不会落入 Fcitx 默认翻页。
  - 目标行宽为 `list->size()`（Fcitx page size）：`upDown` 保持列，`+`/`-` 落目标行首。
  - 越界：有 pageable 且还有页则翻页；无更多页则 clamp 到最后一个候选。
  - 结果写入 `selectedOverride[key]` 后 `filterAndAccept()`，立即 `collectResult()` 返回；不能再进入后续普通输入路径，否则 override 会被清掉且同一按键可能被 Fcitx 二次处理。
- 普通分页（非 scroll）：`=/-.,[]` 走 pageable 翻页（`next/prev`），翻页后 `selectedOverride=0`（高亮跳页首）。
- 卷轴中的数字键和 `;`/`'` 通过 `rowSelectionTarget` 按高亮行选择；普通分页仍使用 Fcitx 页内语义。
- `←`/`→`：焦点 `±1` 并 clamp 到 `[0, bounded-1]`，写 override，不提交。
- **Enter/Space（`f3c472d` 修复）**：若该上下文存在 override，提交**高亮候选**（`candidate.select`），`filterAndAccept` 并清 override——避免提交落到 Fcitx cursor（仍停在第 1 个）上。

**选中覆盖（selectedOverride）**
- `Impl` 成员：`unordered_map<ClientContextKey, optional<uint32_t>>`，按"客户端上下文 + 按键"分别记忆焦点；
- `collectResult`：`cursor = selectedOverride[key]`（存在时覆盖），再校验 `0 <= cursor < size` 后写 `selectedCandidate`；
- `forgetConnection`：清理该上下文的 override 与输入法切换状态。

**bulk 分页状态**
- `collectResult`：`candidatePageSize = list->size()`（页内候选数），`candidatePage = pageable ? pageable->currentPage() : 0`；
- 真 bulk 时候选总数 `totalSize()` 参与 `kMaxCandidates` 截断（`bounded`）。
- 真 bulk 的选择 label 由 engine snapshot 生成，仅高亮行/列带 `1..N` label；`rowSelectionColumn` 与 `rowSelectionTarget` 保证显示标签和按键选择互相对应。UI 不修改 label。
- TSF inline preedit 保持 Fcitx5 engine/addon 提供的 preedit 语义；拼音长串出现按音节/分段的空白属于 Fcitx5 显示行为，不在 Windows UI 层二次改写。

### 3.2 UI —— `src/ui/ui_main.cpp` / `src/ui/candidate_layout.cpp`

- **scrollEligible**：`scroll_mode && candidateBulk && candidatePageSize > 0`；
- **scrollExpanded**：第一页不立即展开；只有同一 composition 已展开过、`candidatePage > 0`，或全局高亮 `selectedCandidate >= candidatePageSize` 时才展开。这样普通打字仍是普通一行/一列候选，继续按滚动/翻页键后才进入卷轴视口。
- **scrollMode_ = scrollEligible && scrollExpanded_**：展开后横向按 `candidatePageSize × 6 行` 网格渲染；纵向按 `6 列 × candidatePageSize 行` 网格渲染；UI 只渲染 engine snapshot 的 label。
- **对齐**：paint 前遍历可见候选测最宽 label → `labelColumnWidth`；`drawTextAt(text, bounds.left + labelColumnWidth)`；comment 起点再 +4px。横向卷轴里，无 label 的非高亮行也预留同一 label 起点，保证候选文字和 `1. 2. 3.` 那一行按列对齐；纵向卷轴里，非当前列不预留 label，避免短候选被挤成窄条。
- **视口跟随**：按六行分组计算 viewport；组内移动不改变 `firstVisible`，末屏向前回填，避免窗口高度变化。
- **宽度**：横向 scroll 布局按普通候选行测宽，避免无意义拉长；纵向 scroll 是列主序，优先保留每列自然宽度，最多 6 列，只有超过工作区宽度时才减少可见列或裁剪单个超宽列。
- **卷轴 cell 绘制（2026-08-20 修正）**：`scroll_cell_width_dip`
  是单个候选格子的最大 viewport 宽度，不是固定宽度；短候选按自然宽度收缩，长候选在该上限内省略。label、text、comment 都走单行 DWrite trimming；长词省略，短词完整显示。

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

提交表只描述历史演进。`f3c472d` 文档曾声称问题已全部修复，但没有覆盖数字标签一致性和稳定视口；本轮工作树修正以实测和 v1.6 规格为准。

## 5. 验证情况

- `ctest --test-dir out/build/windows-x64-dev -C Debug`：**54/54 全绿**；
- 受影响的 candidate navigation/layout/UI 测试：**7/7 全绿**；新增 `candidate-navigation-contract` 覆盖 label 与数字键双向映射，layout contract 覆盖六行内不跳视口；
- `fcitx5_engine_integration_test.exe out/stage/fcitx5/bin/fcitx5-engine.exe`：exit=0（含导航块 `;`/`'`/`←`/`→` 断言）；
- `fcitx5-ui --self-test --reload-test` / `--interaction-self-test` / `--scroll-demo`：exit=0（demo 验证 6×6 布局与视口跟随、行起点对齐）；
- 真实 Fcitx x64 主路径与 4000 次 typing fuzz 通过；新增断言验证数字 2、Enter、Space 的提交文字与其 label/高亮文字一致。完整脚本随后在既有 `--chttrans` 场景的 Ctrl+Shift 输入法切换断言失败，和候选断言分开记录，尚未据此宣称全量 acceptance 通过。

## 6. 已知限制与待确认

- **Rime 伪 bulk**：`totalSize() == -1` → `candidateBulk=false` → 不进卷轴（普通分页）。这是 fcitx 侧 Rime addon 不提供真 bulk API 所致，非前端可解。
- **单页 bulk 不展开卷轴**：候选不足一行（≤ pageSize）时仍是普通分页布局。
- 默认候选主题保持语义不变，只收紧 padding/font size，让高 DPI 下普通竖排页不显得过分松散。
- **集成测试环境 pinyin 词典极简**（每音节 ≤7 候选、一页）：scroll/翻页无法在 CI 断言，靠 demo、代码审查与用户实测。
- 待用户确认：卷轴内的逐排滚动、翻屏边界、Enter/Space 提交高亮这三项的实际手感。
