# 工程文档入口

状态：current。这里的文档体系以当前合同、当前状态和一项当前任务为中心；旧计划不会授权新的实现工作。

## 先读什么

1. [product-contract.md](product-contract.md)：长期产品、安全、所有权和发布不变量。
2. [current.md](current.md)：当前实现事实、已知外部门禁与下一步边界。
3. [tasks/current.md](tasks/current.md)：唯一正在授权的工作项。
4. [tasks/status.md](tasks/status.md)：按任务 ID 检索的历史执行证据；它不是当前规格。
5. 对应的 [ADR](adr)、[product-test-plan.md](product-test-plan.md)、
   [config-ui-test-cases.md](config-ui-test-cases.md) 和
   [ssdlc-verification-matrix.md](ssdlc-verification-matrix.md)。

`tasks/completed/` 保留完成任务的原始证据。reference、audit 和旧状态记录仅提供上下文，不能单独证明当前行为或 release-ready。

## 文档约定

- `current`：当前可执行事实或任务；日期和证据范围必须明确。
- `contract`：长期不变量；修改时需要对应测试/ADR 评审。
- `evidence`：带提交、产物、环境和结果的记录；代码变更后可能失效。
- `reference` / `historical`：设计输入或审计历史，不授权实现。

新文档优先补充上述权威文件。删除旧文档前，先迁移仍有效的规则并断开现行引用。
