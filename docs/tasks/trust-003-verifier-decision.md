# TRUST-003 verifier implementation decision

**Task:** TRUST-003 ML-DSA-65 detached signature verification
**Decision date:** 2026-08-21
**Decision:** use pinned `mldsa-native` 2.0.0 as the product runtime ML-DSA-65 verifier.

## Decision

Fcitx5 for Windows Next verifies v2 repository index and package manifest
signatures with a verify-only `mldsa-native` ML-DSA-65 build.

The product runtime configuration:

- fixes `MLD_CONFIG_PARAMETER_SET` to `65`;
- uses a product namespace prefix;
- disables keypair generation;
- disables signing APIs;
- disables randomized APIs;
- disables assembly/native backends for portability across supported Windows
  hosts.

Unit tests build a separate test-only configuration that exposes deterministic
key generation and signing so verifier fixtures can be generated in memory.
That test-only library is not linked by product binaries.

## Why not Windows CNG as the only verifier

The local Windows SDK exposes `BCRYPT_MLDSA_ALGORITHM`, but the current runtime
probe:

```text
BCryptOpenAlgorithmProvider("ML-DSA") => 0xC0000225
```

fails on this host. Microsoft's public ML-DSA CNG documentation also scopes
support to newer Windows releases than this project currently targets.

Therefore CNG may become an optional future fast path, but it cannot be the
only official repository verifier for this release line.

## Why not a larger crypto framework

`liboqs`, Botan, or another broad cryptographic framework would introduce a
larger dependency surface than this task needs. TRUST-003 needs only detached
ML-DSA-65 public-key verification over already-canonical metadata bytes.

The pinned `mldsa-native` source is smaller and purpose-built for ML-DSA.
Release signing, private-key handling, and key rotation tooling remain separate
release-side work and are not shipped in the runtime.

## Dependency pin

- Package: `mldsa-native`
- Version: `2.0.0`
- Source: `https://github.com/pq-code-package/mldsa-native/archive/refs/tags/v2.0.0.zip`
- SHA-256: `C201A7A8467AFA5B072CA3007CF91EBB55DD027A8F1DC876ADCAE3A6CCC8B158`
- Runtime license choice: compatible permissive license path from the upstream
  SPDX expression `Apache-2.0 OR ISC OR MIT`.

The dependency is fetched by `tools/prepare-package-dependencies.ps1` into
`out/toolchains` and is not downloaded during normal builds.

## Non-goals

- No private key is committed or shipped.
- No release signing command is added by TRUST-003.
- No online add-on installation is enabled merely because the verifier exists.
- No permanent CNG/third-party verifier dual-stack is introduced.
