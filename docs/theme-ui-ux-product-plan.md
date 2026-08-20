# 主题配置与 UI/UX 统筹方案

状态：current  
更新：2026-08-20  
适用范围：Candidate UI、Config、Plugin Manager、Theme Library、Advanced Fcitx config surface

## 1. 产品原则

本项目的主题和设置系统不是“把所有字段摆出来”。目标是让普通用户不用读配置文件也能把输入法调舒服，
让高级用户能迁移、导入、复用既有 Fcitx/Rime/Weasel/Squirrel 风格资产，同时不破坏 v1.7 的输入语义、
安全边界和可验证发布链。

四个角色的共同结论：

| 角色 | 关注点 | 设计要求 |
| --- | --- | --- |
| CTO | 可维护、可扩展、可测试 | schema-first；Theme/Config/Package 统一走 typed control surface；预览复用生产 renderer；避免巨型硬编码映射表 |
| CISO | 第三方主题和插件不可成为代码执行入口 | theme.toml strict parse；未知字段拒绝；脚本/网络默认禁止；资源路径限制在主题目录；包源、签名、SBOM、license gate 保持强制 |
| 产品经理 | 用户能理解并安全试错 | 基础页放常用设置；高级页动态渲染 Fcitx typed metadata；每个设置标明 Live/Deferred/Restart-required；一键恢复默认 |
| 市场经理 | 看起来像现代产品而不是工程样机 | 默认主题精致、DPI 正确、浅/深色完整；提供主题库、预览、导入向导；用“小企鹅 + 当前方案”建立品牌识别 |

## 2. 参考输入的取舍

参考来源只塑造产品形态，不授权复制非平凡实现：

- fcitx-contrib theme 文档：借鉴 Fcitx 主题以声明式配置和图片资产表达外观的方向；
- Fcitx5 Android theme designer：借鉴可视化编辑、即时预览、字段分组；
- Weasel customization 与 Rime CustomizationGuide：借鉴 patch/override 和迁移习惯，但 Windows Next 不把 YAML 手写作为普通用户主路径；
- Squirrel/Rime See Me：借鉴“读入既有主题并生成配置”的导入体验；
- fcitx5-macos config：借鉴单 OS 输入源、内部输入法状态、Advanced 动态配置和插件管理器结构。

Windows Next 的取舍是：普通路径走 GUI 和 typed schema；高级路径可以导入/导出文本配置，但必须经过验证、预览和回滚。

## 3. 信息架构

Config 应逐步形成五个稳定区域：

1. **概览**
   - 当前状态：Running / Paused / Safe Mode / Engine unavailable；
   - 当前内部输入方案：Pinyin、Rime、Mozc 等；
   - 快捷入口：重启 engine、打开诊断、修复注册、打开用户数据目录。

2. **输入**
   - 候选数、候选方向、滚动模式、一行/一列候选数；
   - 常用快捷键说明和冲突提示；
   - 输入法组和内部方案管理，后续来自 generic Fcitx input-method surface。

3. **外观**
   - 字体、字号、候选窗最大宽度、滚动单元宽度、圆角、阴影、透明度；
   - 浅色/深色主题选择；
   - 生产 Candidate renderer 的 synthetic preview，不维护第二套近似预览。

4. **主题库**
   - builtin themes + user themes；
   - theme detail：元数据、浅/深色分支、可编辑字段、安全策略；
   - 导入向导：Fcitx theme、Weasel/Squirrel/Rime 风格配置先进入 staging，验证后生成安全 theme.toml。

5. **插件与高级**
   - 插件列表、详情、依赖、权限、签名/manifest SHA；
   - generic Fcitx addon/config renderer；
   - 不维护 Windows 专用巨型 Pinyin/Rime/Mozc 硬编码设置表。

## 4. 主题模型

主题系统采用三层模型：

```text
Theme Package / User Theme
        ↓ strict parse
ThemeRecord metadata + Theme schema
        ↓ safe editor surface
Config UI / Theme Preview / Candidate Renderer
```

ThemeRecord 至少包含：

- `id`
- `source`：builtin / user / package
- `name`
- `version`
- `license`
- `description`
- `has_light_branch`
- `has_dark_branch`
- `editable_fields`
- `security`

主题编辑器不得直接编辑任意文件路径。所有资产引用都必须限制在主题目录内；不允许脚本、网络请求、
外部程序或注册表动作。未知字段默认拒绝，后续若需要兼容旧格式，通过 importer 显式转换。

## 5. 人因工程与默认值

候选窗默认应让“读、选、输”三件事都轻：

- 横排：候选编号和候选文本基线对齐；候选间距稳定；不会随每个字过度跳动；
- 竖排：编号、文本、注释形成清晰列；宽度按最长候选和注释受限增长，并受工作区 clamp；
- 滚动模式：长度接近普通候选窗，不因 scroll expansion 撑成巨条；
- DPI：所有 DIP 字段在当前 monitor DPI 下缩放；跨屏时重排不模糊；
- 可访问性：UILess 与可视候选使用同一 CandidateModel；高对比/深色主题不丢失选中状态。

“漂亮”不是后期贴皮。默认主题必须作为回归资产管理：每次改布局、字体、DPI、滚动模式，都要有
contract 或视觉 smoke 覆盖。

## 6. 插件管理器与 Advanced surface

插件管理器不是下载器外壳，而是受信任变更控制台：

- repository 是唯一在线来源；
- 每个包显示 type、title、summary、version、architecture、dependencies、permissions、
  source commit、manifest SHA 和状态；
- 安装/更新/删除使用事务、回滚和 generation-aware restart/reload；
- native addon 卸载需要明确 restart-required；
- input-method data/theme/translation 尽量 live/deferred 生效；
- package detail 暴露 `config_surface`，Config 以此决定展示 theme editor、input-method data、
  generic addon config 或普通组件详情。

Advanced 只通过 engine/control 访问 Fcitx typed config metadata。TSF DLL、Candidate UI、
package parser 都不加载 Fcitx addon，也不把 addon schema 硬编码进 Windows UI。

## 7. 安全和发布门禁

主题/插件相关改动必须进 v1.7 的 SSDLC 门禁：

- TOML/YAML/import parser fuzz；
- path traversal、绝对路径、UNC、symlink/reparse point 策略测试；
- SCA、license、SBOM、source policy；
- package signature、anti-rollback、channel binding；
- no script/network in theme gate；
- UI/Config 不请求 UAC；安装/修复注册才进入明确 UAC 流程；
- Win7 Legacy lane 只接受逐组件证明，不因主题系统引入现代 runtime 依赖。

## 8. 分阶段落地

| 阶段 | 交付 | 验证 |
| --- | --- | --- |
| Now | control 暴露 theme list/detail；Config Theme 页消费主题库并显示安全详情；package detail 暴露 config surface；外观常用字段可 round-trip | unit + integration + schema + package gate |
| Phase 6 polish | Config 外观页重排、主题库页、生产 renderer preview、高 DPI 视觉证据 | config UI contract + visual smoke + real DPI matrix |
| Phase 7 | theme package 安装/启用/禁用/回滚；importer staging | package transaction + importer hostile corpus |
| Advanced R1 | addon/config metadata read-only 浏览 | engine/control contract + unknown option fallback |
| Advanced R2 | typed set/reset/apply；输入法组管理 | differential against Fcitx config behavior + rollback |

## 9. 当前实现映射

当前代码已进入 Now 阶段：

- `fcitx5-control --themes-list`：列出 builtin 和 user themes；
- `fcitx5-control --themes-detail ID`：返回主题元数据、浅/深色分支、可编辑字段和安全策略；
- Config Theme 页消费 `--themes-list/detail`，展示主题库、来源、版本、许可证、分支、安全策略，
  并把选中的 theme id 写入 `appearance.theme`；
- `fcitx5-control --packages-detail ID`：返回 permissions、dependencies、manifest SHA、source commit 和 config surface；
- Config 外观页已暴露候选宽度、滚动单元宽度、字号、圆角、阴影等常用字段。

下一步不应该从“继续加硬编码控件”开始，而应该补嵌入式 renderer preview seam 和 importer staging。
当前 Preview 按钮已经启动生产 `fcitx5-ui.exe --demo` 并读取真实 visual config/theme；后续若要做
嵌入式预览，应把生产 renderer 作为独立预览 surface 接入，而不是在 Config 里重画第二套候选窗。
这会让后续导入 Weasel/Squirrel/Rime 风格主题时，有地方预览、验证、回滚，而不是直接污染用户配置。

## 10. 参考链接

- <https://fcitx-contrib.github.io/docs/theme/>
- <https://fcitx5-android.github.io/theme-designer/>
- <https://github.com/rime/weasel/wiki/Weasel-%E5%AE%9A%E5%88%B6%E5%8C%96>
- <https://github.com/rime/home/wiki/CustomizationGuide>
- <https://gjrobert.github.io/Rime-See-Me-squirrel/>
- <https://github.com/GJRobert/Rime-See-Me-squirrel/blob/master/README.md>
