# 工程文档入口

状态：current  
更新：2026-08-18

本仓库只有一条现行执行路线：冻结的 v1.6 工程规格。阅读顺序：

1. [`current-task-summary.md`](current-task-summary.md)：当前目标、红灯和下一步；
2. [`reference-baseline.md`](reference-baseline.md)：Phase 0 的固定来源、权重和许可证边界；
3. [`reference-review-windows-chewing-tsf.md`](reference-review-windows-chewing-tsf.md)：主教材审计；
4. [`reference-review-win-mcbopomofo.md`](reference-review-win-mcbopomofo.md)：thin Client/Server 次教材审计；
5. [`technical-program-plan.md`](technical-program-plan.md)：严格 Phase 0 → 8 的执行顺序；
6. [`adr/0004-runtime-topology-preedit-and-recovery.md`](adr/0004-runtime-topology-preedit-and-recovery.md)：运行拓扑和状态所有权；
7. [`product-test-plan.md`](product-test-plan.md) 与 [`ssdlc-verification-matrix.md`](ssdlc-verification-matrix.md)：测试与质量门禁；
8. `phase-*-acceptance.md`：历史或当前阶段证据，不能反向修改路线。

外部唯一权威规格：

`D:\Downloads\64Gram Desktop\Fcitx5_for_Windows_工程规格_现代软件工程_轻量SSDLC_DevSecOps_Codex执行版_v1_6.md`

SHA-256：`ED652A385C9F3F7DFC710B0BF905F7129D831894FBD50297BB0278509574615F`

## 文档状态

- `current`：现行执行依据；
- `evidence`：带提交/产物/环境标识的验证记录；代码变化后可能过期；
- `reference`：问题定向的设计输入，不授权复制源码；
- `historical`：保留审计价值，但不得证明当前实现完成；
- `obsolete`：已废弃，禁止继续实施。

以下路线统一标记为 `obsolete`：v1.4/v1.5 作为现行规格、aardio、Slint、wxWidgets、Qt、
Tauri、WebView2、Rust Candidate UI、UI Automation/坐标点击、Hook/SendInput 输入或候选提交
路径，以及“所有内部 EXE 都让用户手工启动”。WTL Config、C++ D2D/DWrite Candidate UI
属于 v1.6 冻结路线，不在废弃范围。

广泛项目对照只放在
[`reference-review-windows-ime-matrix.md`](reference-review-windows-ime-matrix.md) 附录中。
它不得稀释 Phase 0 的优先级：Chewing 是主教材，gaboolic 仅用于 Fcitx 接线，
win-mcbopomofo 用于 thin client/server，Weasel 是兼容病例库。
