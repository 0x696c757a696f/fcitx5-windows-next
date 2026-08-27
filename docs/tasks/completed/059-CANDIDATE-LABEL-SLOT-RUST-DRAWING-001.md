# Task 059 — Candidate Label Slot Rust Drawing

**Mode:** CHANGE
**Task ID:** `CANDIDATE-LABEL-SLOT-RUST-DRAWING-001`
**Prerequisite:** `R3-01` automated Candidate model/layout/interaction cutover evidence; `REL-01` remains parked on external/manual evidence.
**Evidence class:** Rust Candidate layout/drawing contract, screenshot/golden evidence, and focused CTest/Cargo regression.

## Goal

Implement the newly frozen Candidate ordinal/label UX as a code-only Rust slice:
configurable candidate numbers, stable reserved row/column/cell label slots, selected-scope label reveal, and Rust-owned drawing/layout state.

## Spec Sections

- `docs/spec-v1.8.md` section `0.4` for reference authority and the explicit exclusion of `fcitx-contrib/fcitx5-windows`.
- `docs/spec-v1.8.md` section `5.1` for CandidateModel/renderer separation and label slot semantics.
- `docs/spec-v1.8.md` section `5.2` for Rust-owned Candidate domain plus temporary native renderer adapter.
- `docs/spec-v1.8.md` section `13.3` for Candidate UI Rust migration boundary.
- `docs/spec-v1.8.md` sections `13.9.5` through `13.9.7` for candidate label/theme schema and validation.

## Requirements

- Candidate labels are configurable presentation fields only. Do not move selection key, page size, candidate order, commit key, or plugin candidate action semantics into theme/config-owned label code.
- Support label display modes including always visible, hidden, and selected row/column/item reveal.
- Every horizontal, vertical, and grid candidate cell reserves a label slot before candidate text.
- Label slot width is resolved from the widest label in the current page/grid or an explicit fixed width. Hidden labels still reserve the same slot when `reserve_when_hidden` is enabled.
- Labels align inside the label slot, with right alignment as the default for `1.`, `10.`, custom suffix, and circled-number styles.
- Candidate text, annotation, and emoji start after a fixed gap and must wrap, clip, or ellipsis inside the candidate text/comment area. They must not consume label-slot width or break left/right/top/bottom alignment.
- Drawing/layout state added for this task must be Rust-owned. Existing Win32/D2D/DWrite code may remain only as a window/font/DPI/native adapter until equivalent renderer evidence allows deletion.
- Do not use `fcitx-contrib/fcitx5-windows` as an architecture basis for this work.

## Acceptance

- Rust tests cover label width resolution for `1.`, `10.`, custom prefix/suffix, hidden labels, and selected row/column/item reveal.
- Rust layout/drawing evidence covers horizontal, vertical, and multi-column/grid candidates with stable candidate text origins.
- Screenshot or golden evidence demonstrates at least:
  - vertical list with selected item label reveal;
  - horizontal row with reserved hidden labels;
  - multi-column/grid layout where columns align despite different label/text lengths.
- Source-contract or equivalent guard prevents reintroducing Candidate label/domain ownership in new C++ code.
- Affected x64/x86 Cargo and CTest checks pass, or any unavailable real-host/a11y evidence is recorded as manual-pending without claiming `REL-01`.

## Out Of Scope

- Rewriting Fcitx `CandidateList` or upstream addon candidate semantics.
- Moving selection keys/page size/commit semantics into theme config.
- Completing the full Candidate renderer migration if this label-slot slice can be delivered with a narrower Rust drawing/layout contract.
- Declaring release readiness.
