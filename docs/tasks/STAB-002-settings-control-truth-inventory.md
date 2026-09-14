# STAB-002 — Settings Control Truth Inventory

Status: verified source inventory at `fdf66b89dd36aa3ff1481d0bd8082f74fbc8cc90`.

Scope is the visible Rust Settings surface in `rust/config-poc/src/main.rs`.
This document records source-level binding truth. It is not a desktop-host,
accessibility, package-network, or release-evidence claim.

State meanings:

- `FullyBound`: visible action reaches its declared Rust authority.
- `ReadOnlyStatus`: visible state from an authority, with no direct mutation.
- `ExplicitlyUnavailable`: visible page/control tells the user it is not bound.
- `Fake/Demo`: visual-local signal, sample data, or toast only; no product authority.

| semantic_id | page | visible label | backing authority | ConfigField / backend action | PreviewPolicy | CommitPolicy / persistence | state | source tests |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `settings.search` | sidebar | 搜索设置... | none | none | none | none | Fake/Demo | UI construction only |
| `input.default-engine` | 输入 | 默认输入法 | local `input_method` signal | none | local selection only | none | Fake/Demo | UI construction only |
| `input.shortcut-tags` | 输入 | 添加键位... and removable shortcut chips | toast callback | none | none | no Config Core write | Fake/Demo | UI construction only |
| `input.shortcut-summary` | 输入 | 中英切换, 简繁切换, 全半角, 标点切换 | literal labels | none | none | none | ReadOnlyStatus | UI construction only |
| `candidate.layout-mode` | 外观 / 候选窗口 | 自动, 纵排, 横排, 卷轴, 竖排文字 | `WindUiConfigAdapter` → `ConfigCore` | `ConfigEdit::CandidateLayoutType` | Draft immediately changes the displayed sample only | Deferred: Apply atomically persists; Cancel restores Current; Reset restores Defaults | FullyBound | `candidate_layout_uses_frozen_names_and_migrated_semantics`, `snapshot_layout_type_decodes_and_ui_commits_through_config_core`, `windui_candidate_adapter_uses_one_draft_for_preview_cancel_reset_and_apply` |
| `candidate.scroll-direction` | 外观 / 候选窗口 | 横向卷轴, 纵向卷轴 | `WindUiConfigAdapter` → `ConfigCore` | `ConfigEdit::CandidateScrollDirection` | Draft sample updates | Deferred / atomic Apply | FullyBound | `only_relevant_direction_survives_the_typed_config_round_trip` |
| `candidate.page-size` | 外观 / 候选窗口 | 1 through 9 | `WindUiConfigAdapter` → `ConfigCore` | `ConfigEdit::CandidatePageSize` | Draft sample slot visibility updates | Deferred / atomic Apply | FullyBound | `candidate_page_size_is_authoritative_and_strictly_bounded`, `windui_candidate_adapter_uses_one_draft_for_preview_cancel_reset_and_apply` |
| `candidate.vertical-text-column-direction` | 外观 / 候选窗口 | 从右到左, 从左到右 | `WindUiConfigAdapter` → `ConfigCore` | `ConfigEdit::CandidateVerticalTextColumnDirection` | Draft sample updates | Deferred / atomic Apply | FullyBound | `only_relevant_direction_survives_the_typed_config_round_trip` |
| `candidate.preview` | 外观 / 候选窗口 | 生产候选预览 | `WindUiConfigAdapter` → `ConfigCore` → shipping CandidateModel/layout/resolved-theme/renderer | typed `ConfigSnapshot` | Draft directly renders production pixels for CJK, Latin, punctuation, emoji, comments, selection, and every supported layout | Deferred: the same Draft is atomically persisted by Apply; Cancel restores Current | FullyBound | `settings_preview::tests`, `visual_draft_tests`, `windui_candidate_adapter_uses_one_draft_for_preview_cancel_reset_and_apply` |
| `candidate.apply` | 外观 / 候选窗口 | 应用 | `ConfigCore` | `WindUiConfigAdapter::apply` | Draft remains visible | atomic Current write plus last-known-good path | FullyBound | `windui_candidate_adapter_uses_one_draft_for_preview_cancel_reset_and_apply` |
| `candidate.cancel` | 外观 / 候选窗口 | 取消 | `ConfigCore` | `WindUiConfigAdapter::cancel` | restores Current sample | no write | FullyBound | `windui_candidate_adapter_uses_one_draft_for_preview_cancel_reset_and_apply` |
| `candidate.reset` | 外观 / 候选窗口 | 重置 | `ConfigCore` | resets layout type, directions and page size fields | Defaults sample becomes visible | no write until Apply | FullyBound | `windui_candidate_adapter_uses_one_draft_for_preview_cancel_reset_and_apply` |
| `theme.mode` | 外观 / 主题 | 跟随系统, 浅色, 深色 | `WindUiConfigAdapter` → `ConfigCore` | `ConfigEdit::AppearanceMode` | The same Draft selects the resolved preview theme; high contrast remains a host override | Deferred / atomic Apply | FullyBound | `visual_draft_tests`, `settings_preview::tests` |
| `theme.accent` | 外观 / 主题 | 微信绿, 竹青, 墨绿 | local `accent_pick` signal | none | none | none | Fake/Demo |
| `theme.window-shadow` | 外观 / 主题 | 窗口投影 | local `window_shadow` signal | none | none | none | Fake/Demo |
| `appearance.ui-font-size` | 外观 / 排版 | 界面字号 | local `ui_font_size` signal | none | none | none | Fake/Demo |
| `appearance.ui-scale` | 外观 / 排版 | 界面缩放 | local `ui_scale` signal | none | none | none | Fake/Demo |
| `appearance.compact` | 外观 / 排版 | 紧凑模式 | local `compact` signal | none | none | none | Fake/Demo |
| `package.catalog-selection` | 插件与扩展 | plugin catalog rows | fixed `FCITX5_PLUGIN_CATALOG` plus `PluginManagerSnapshot` | selection only | detail panel/status updates | no write | FullyBound | `pinned_plugin_catalog_is_complete_and_unique` |
| `package.repository-status` | 插件与扩展 | 官方仓库 · ... | `fcitx5-control --packages-list` parsed into `PluginManagerSnapshot` | `PluginOperation::List` | status refreshes after operation | read-only | ReadOnlyStatus | `installed_plugin_actions_follow_control_state` |
| `package.refresh` | 插件与扩展 | 刷新插件目录 | `fcitx5-control` package boundary | `--packages-refresh` then `--packages-list` | status refreshes | Control owns networking, verification and persistence | FullyBound | `plugin_control_arguments_are_fixed_and_catalog_bounded` |
| `package.install` | 插件与扩展 | 安装 | `fcitx5-control` package boundary | `--packages-install <catalog-id>` | status refreshes | Control owns verified repository, transaction and persistence | FullyBound when enabled | `plugin_control_arguments_are_fixed_and_catalog_bounded`, `installed_plugin_actions_follow_control_state` |
| `package.update` | 插件与扩展 | 更新 | `fcitx5-control` package boundary | `--packages-update <catalog-id>` | status refreshes | Control owns verified repository, transaction and persistence | FullyBound when enabled | `plugin_control_arguments_are_fixed_and_catalog_bounded`, `installed_plugin_actions_follow_control_state` |
| `package.toggle` | 插件与扩展 | 启用/禁用 | `fcitx5-control` package boundary | `--packages-state <catalog-id> enabled/disabled` | status refreshes | Control owns package state persistence | FullyBound when enabled | `plugin_control_arguments_are_fixed_and_catalog_bounded`, `installed_plugin_actions_follow_control_state` |
| `package.remove` | 插件与扩展 | 卸载 | `fcitx5-control` package boundary | `--packages-remove <catalog-id>` | status refreshes | Control owns transactional removal | FullyBound when enabled | `plugin_control_arguments_are_fixed_and_catalog_bounded`, `installed_plugin_actions_follow_control_state` |
| `package.repair` | 插件与扩展 | 修复 | `fcitx5-control` package boundary | `--packages-repair` then `--packages-list` | status refreshes | Control owns repair and persistence | FullyBound | `plugin_control_arguments_are_fixed_and_catalog_bounded` |
| `shortcuts.page` | 按键 | 此页会绑定对应 Rust 配置模型与 Control API。 | none | none | none | none | ExplicitlyUnavailable | `windui_nav_placeholder` source path |
| `updates.page` | 更新 | 此页会绑定对应 Rust 配置模型与 Control API。 | none | none | none | none | ExplicitlyUnavailable | `windui_nav_placeholder` source path |
| `diagnostics.page` | 诊断与修复 | 此页会绑定对应 Rust 配置模型与 Control API。 | none | none | none | none | ExplicitlyUnavailable | `windui_nav_placeholder` source path |
| `settings.ready-status` | footer | 配置已就绪 | literal status label | none | none | none | Fake/Demo | UI construction only |

## Important boundary

The plugin buttons are source-bound to `fcitx5-control`, but this inventory
does not establish that an online repository has a compatible Windows Rime
package, or that any package action passed on a real host. Those remain
separate lifecycle evidence.

The candidate preview is source-bound to the shipping renderer. This is not a
claim of real-host UIA, Narrator, or desktop visual evidence; those remain
MANUAL-PENDING under the release host-evidence gate.
