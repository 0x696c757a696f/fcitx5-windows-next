# ADR 0009: Fcitx Upstream Boundary and Rust Product Plane

Status: accepted

Date: 2026-08-24

## Context

The product goal is to move product-owned Windows logic to Rust while preserving the Fcitx upstream object model and addon ecosystem. Existing Windows ports are not architecture authority for this project. In particular, `fcitx-contrib/fcitx5-windows` is excluded as an architecture reference.

The current upstream signals are:

- `fcitx/fcitx5` defines Fcitx 5 as a generic input method framework. The main repository contains only the keyboard layout engine; language engines live in addons.
- Fcitx 5.1.12 introduced an addon factory mechanism to support fully statically linked Fcitx, so this project must not model addon loading as `Addon == DLL`.
- Fcitx 5.1.10 introduced candidate action API for macOS/Android-style ports, so candidate semantics should stay aligned with upstream `CandidateList`/candidate action ownership.
- Upstream Fcitx text-edit/input-context capability continues to evolve, including the open optional atomic surrounding-text replacement contract and C++23 target discussion in August 2026.

## Decision

Keep the direct Fcitx object manipulation layer in C++ and make it thin. Product-owned logic outside that Fcitx-facing layer moves toward Rust.

The C++ island is limited to direct interaction with:

- `fcitx::Instance`
- `fcitx::InputContext`
- `InputContextManager`
- `FocusGroup`
- `AddonManager`
- `AddonInstance`
- `InputMethodEngine`
- `InputPanel`
- `CandidateList`
- Fcitx config objects
- Fcitx event objects

The Rust product plane owns:

- IPC and wire protocol
- validation and canonicalization
- context/composition ledger
- revision and generation policy
- timeout/deadline/fail-open policy
- diagnostics
- Windows TSF implementation
- Candidate UI product behavior and rendering state
- Config, package/update/control, launcher, provider, and deployment logic

Engine structure:

```text
Rust Engine Core
  protocol / state / validation / revision / generation / policy / IPC
        |
        | narrow C ABI
        v
C++ Fcitx Adapter
  InputContext / Instance / AddonManager / CandidateList
  config and event conversion
        |
        v
upstream Fcitx5 and upstream addons
```

Addon modeling is:

```text
Addon
  static or built-in
  dynamic or package-loaded
```

Addon modeling is not:

```text
Addon == DLL
```

Candidate flow is:

```text
Fcitx CandidateList
  -> candidate/action DTO
  -> Rust Candidate
  -> user-triggered action
  -> Engine C++ adapter
  -> Fcitx upstream candidate action API
```

The Rust protocol must advertise extensible capabilities rather than freezing current Fcitx frontend shape into fixed actions:

```text
TEXT_COMMIT
TEXT_COMMIT_WITH_CURSOR
TEXT_DELETE_SURROUNDING
TEXT_REPLACE_SURROUNDING
CANDIDATE_ACTION
```

Avoid narrow long-term enums such as:

```text
Commit(String)
Delete(i32, u32)
```

when those become the only project-wide action vocabulary.

## Consequences

- Do not rewrite upstream addons as Windows-private Rust implementations.
- Do not bind the whole Fcitx C++ API into Rust.
- Do not make compatibility with any existing Windows port a design goal.
- Track `fcitx/fcitx5` and each addon upstream for semantics.
- Treat `fcitx-contrib/fcitx5-plugins` only as a cross-platform plugin build/dependency reference, not as semantic authority over individual addons.
- Rust migration continues for product-owned Windows code outside the narrow Fcitx adapter.
