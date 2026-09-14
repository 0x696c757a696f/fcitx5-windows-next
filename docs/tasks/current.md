# STAB-002 — Settings Control Truth Inventory

Status: current

This is the sole active task. The governing product rules are in
[`../product-contract.md`](../product-contract.md), current repository facts are in
[`../current.md`](../current.md), and prior evidence is searchable in `status.md`.

Goal:
Inventory every visible shipping Settings control.

Scope:
rust/config-poc
config-core
control/package boundary only as needed

No production behavior changes.

For every control record:
- semantic_id
- page
- visible label
- backing authority
- ConfigField / backend action
- PreviewPolicy
- CommitPolicy
- persistence
- current state:
  - FullyBound
  - ReadOnlyStatus
  - ExplicitlyUnavailable
  - Fake/Demo
- tests

Done when:
Every shipping-visible control is classified.
No code behavior is changed.
