# Task 062 - Candidate Production Vertical Typography

**Mode:** CODE
**Task ID:** `CANDIDATE-PRODUCTION-VERTICAL-TYPOGRAPHY-001`
**Prerequisite:** `CANDIDATE-MICROSOFT-YAHEI-RUST-TEXT-RENDERER-001`
**Evidence class:** `AUTOMATED`

## Goal

Close the gap between a DirectWrite engineering screenshot and a normal Windows IME Candidate
window by freezing a production-like five-candidate vertical typography and density corpus.

## Specification references

- Section 5.1 CandidateModel/renderer separation
- Section 13.3 Candidate Rust product logic and renderer boundary
- Sections 13.9.5 through 13.9.7 typed Candidate theme fields

## Required behavior / implementation contract

- Reuse the existing Rust WindInput/Qingfeng visual plan and vendored windui DirectWrite renderer.
  Do not add another renderer, GUI framework, or C++ drawing/domain owner.
- Candidate, label, comment font sizes and vertical row height are named Rust-owned theme tokens;
  the paint path consumes those tokens instead of independent raw constants.
- Add a production vertical visual corpus with five simultaneously visible Chinese candidates,
  `1.` through `5.` labels, a selected first candidate, and comments on at least two candidates.
- Labels remain in a fixed right-aligned slot. Candidate and comment origins, baselines, and row
  bounds do not move between selected and unselected rows and do not overlap.
- The Candidate text rectangle must fit the complete estimated CJK glyph advance. Rectangle
  non-overlap alone is not sufficient visual evidence.
- At the 150% DPI visual baseline, default Candidate text is approximately 33 physical px and label
  text approximately 27 physical px, rendered through Microsoft YaHei/YaHei UI plus DirectWrite
  fallback.
- Preserve the existing selected-scope vertical/horizontal/grid evidence and WeChat-green light/dark
  themes; this task adds production typography evidence rather than deleting configurability.

## Required validation

- `cargo fmt --all -- --check`
- x64/x86 `fcitx5-candidate-core` tests
- x64/x86 production vertical typography screenshot CTest
- x64/x86 `source-contract`
- Fresh screenshot/report inspection for five candidates, five shown labels, comments, typed
  typography metrics, stable origins, no overlap, `typography_text_fits=true`, and visibly complete
  `水`/`收` glyphs beside comments

## Done when

- The five-candidate screenshot has normal Windows IME density and readable CJK text.
- The report records the typography tokens, five visible candidates/labels, stable geometry, and
  the actual Microsoft YaHei text face.
- The source contract prevents regression to paint-local typography constants or a one-item visual
  proof.
