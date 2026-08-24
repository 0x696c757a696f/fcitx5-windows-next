# TSF Profile Boundary

Date: 2026-08-24

This repository exposes exactly one Windows TSF language profile for the product:
`Fcitx5`.

## Product Profile

- Text service CLSID: `3a21b9e2-4f47-4c36-8bfa-91d7d3b3e901`
- Language profile GUID: `6c2ac726-7703-4b65-89af-a77e9e0da102`
- Windows profile display name: `Fcitx5`
- Windows profile count: `1`

Internal input methods such as Pinyin, Rime, Mozc, Hangul, m17n, Keyman, and
Bamboo are Fcitx/addon state inside the product. They must not become separate Windows TSF profiles.

## Ownership

- `rust/tsf-poc/src/lib.rs` owns COM registration, TSF profile registration,
  single-profile identity, and best-effort cleanup of obsolete profile GUIDs.
- `rust/register-core/src/lib.rs` owns product TSF DLL path, operation, elevation,
  registry-status, and registration-export policy only. It must not enumerate
  internal input methods or create per-engine TSF profile data.
- `installer/fcitx5-windows.iss` installs the x64/x86 TSF DLL generations and
  invokes the register helpers. It must not carry a profile list or per-engine
  profile registration.
- `tests/support/tsf_test_identity.h` mirrors the release identity for automated
  tests and may keep legacy dynamic-profile ledger parsing only to prove old
  data is cleaned.

## Legacy Cleanup

Obsolete profile GUIDs and `tsf-profile-ledger.tsv` are cleanup inputs only.
They are allowed to drive unregister/delete behavior, but they must never drive new `RegisterProfile` calls or product UI profile creation.

Dynamic multi-profile machinery is retired. Reintroducing it requires a new
explicit product decision and a new migration task.
