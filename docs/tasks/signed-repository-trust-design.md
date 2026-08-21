# Signed repository / package manifest / trusted key design

**Product:** Fcitx5 for Windows Next
**Created from HEAD:** `d557e4809cb26c0697169c49294fff2cd8126061`
**Execution state:** DESIGN / not yet implemented
**User decision:** future official repository trust must not be RSA-based; use PQC-first signing.

## Goal

Define the future package and add-on trust model for online installation,
updates, uninstall-after-update, and repair.

The design must be conservative where it matters:

- no unsigned online package or repository metadata is installable;
- no package can escape its declared manifest paths;
- no rollback is accepted unless explicitly allowed by policy;
- no UI should claim that official online plug-ins are available unless a
  trusted official repository key is installed and verified;
- no private signing key is shipped in the product;
- no offensive or prohibited techniques are introduced.

## Algorithm decision

### Production direction

Use **ML-DSA-65** as the default future signature algorithm for official
repository and package metadata.

Use **SLH-DSA** as a reserved conservative/recovery signature family, not as the
first default for every package. SLH-DSA is useful as an alternate trust path
because it is hash-based and conservative, but its signatures and performance
cost are less attractive for routine package metadata.

Use **BLAKE3** as the default payload/content hash for the future package
manifest format.

Keep **SHA-256** as a compatibility hash for external tooling, SBOM scanners,
mirrors, and migration from existing package metadata.

Do not use BLAKE3 as a signature algorithm.

### Why ML-DSA-65

ML-DSA is the NIST-standardized lattice-based post-quantum digital signature
scheme. For this product, ML-DSA-65 is the best default balance:

- stronger future posture than classical RSA/ECC signatures;
- much smaller and faster than SLH-DSA for routine repository/package metadata;
- appropriate for detached signatures over small canonical metadata files;
- suitable for key rotation and offline release signing workflows.

ML-DSA-44 may be acceptable for some use cases, but the official repository is
a long-lived supply-chain trust root. ML-DSA-65 is the safer default.

ML-DSA-87 is stronger but larger. Reserve it for a future policy decision if
the extra size/cost is justified.

### Non-goals

- Do not add RSA as a new future trust root.
- Do not make RSA the default for official online plug-ins.
- Do not use Ed25519 as the future official repository trust root.
- Do not replace package trust with plain hashes.
- Do not accept signatures from keys that are not present in the trusted
  keyring.
- Do not require SLH-DSA for every first-generation package unless a later
  task explicitly chooses that cost.

### Compatibility posture

The repository currently contains RSA verification code and tests. Treat that
as existing compatibility/technical debt until a task explicitly removes it.

New official repository/package metadata should be designed around ML-DSA-65.
If an old RSA path remains temporarily in tests or migration code, it must not
be documented as the future official repository trust root.

## Trust model

```text
trusted-keys.json
        │
        ├─ trusted ML-DSA repository/package public keys
        ├─ optional SLH-DSA recovery/public keys
        └─ revoked keys

repository/index.json
repository/index.sig.json
        │
        ├─ signed package list
        ├─ channel binding
        ├─ repository generation / anti-rollback counter
        └─ package manifest URL/hash/signature metadata

package/manifest.json
package/manifest.sig.json
        │
        ├─ package id/version/channel
        ├─ package payload paths
        ├─ payload content hashes
        ├─ uninstall ownership policy
        └─ optional compatibility / feature metadata
```

Verification order:

1. Load `trusted-keys.json`.
2. Reject malformed, unknown, duplicate, revoked, or unsupported required keys.
3. Verify `repository/index.sig.json` over canonical `repository/index.json`.
4. Check repository id, channel, generation, and rollback policy.
5. Download only manifest/package URLs referenced by the verified index.
6. Verify `manifest.sig.json` over canonical `manifest.json`.
7. Check package id, channel, version, declared ownership, and hashes.
8. Stage payload into a transaction directory.
9. Re-hash staged payloads against manifest.
10. Atomically activate only after all checks pass.

## Keyring schema v2

`trusted-keys.json` should become an explicit versioned keyring.

```json
{
  "format_version": 2,
  "policy": {
    "official_required_signatures": [
      "mldsa65"
    ],
    "compatibility_hashes": [
      "sha256"
    ],
    "default_payload_hash": "blake3"
  },
  "keys": [
    {
      "key_id": "official-2026-mldsa65",
      "algorithm": "mldsa65",
      "status": "trusted",
      "public_key_base64": "BASE64_ML_DSA_65_PUBLIC_KEY",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2028-01-01T00:00:00Z",
      "scope": ["repository", "package"],
      "channels": ["stable"]
    },
    {
      "key_id": "official-2026-slh-dsa-recovery",
      "algorithm": "slhdsa-sha2-128s",
      "status": "trusted",
      "public_key_base64": "BASE64_SLH_DSA_PUBLIC_KEY",
      "scope": ["repository"],
      "channels": ["stable"]
    },
    {
      "key_id": "official-2026-mldsa65-revoked",
      "algorithm": "mldsa65",
      "status": "revoked",
      "public_key_base64": "BASE64_ML_DSA_65_PUBLIC_KEY",
      "scope": ["repository", "package"],
      "channels": ["stable"]
    }
  ]
}
```

Validation rules:

- `format_version` must be recognized.
- `key_id` must be stable, lowercase, and unique.
- `algorithm: "mldsa65"` requires the exact ML-DSA-65 public key length used by
  the selected verifier implementation.
- `algorithm: "slhdsa-..."` requires an explicitly supported SLH-DSA parameter
  set.
- `status` is either `trusted` or `revoked`.
- revoked keys may remain in the keyring so old signatures can be rejected with
  a clear reason.
- a key without the required `scope` cannot verify that object type.
- a key without the requested `channel` cannot verify that channel.
- unsupported algorithms are ignored only when they are not required by policy.
- if policy requires `mldsa65`, an object without a valid ML-DSA-65 signature
  fails closed.

## Signature envelope

Do not use a raw fixed-size `.sig` file for the PQC design. PQC signatures have
algorithm-dependent sizes and must carry policy-relevant metadata.

Use a canonical JSON signature envelope:

```json
{
  "format_version": 2,
  "signed_object": "repository-index",
  "canonicalization": "fcitx5-windows-next-json-v1",
  "signatures": [
    {
      "key_id": "official-2026-mldsa65",
      "algorithm": "mldsa65",
      "signature_base64": "BASE64_ML_DSA_65_SIGNATURE"
    }
  ]
}
```

For package manifests:

```json
{
  "format_version": 2,
  "signed_object": "package-manifest",
  "canonicalization": "fcitx5-windows-next-json-v1",
  "signatures": [
    {
      "key_id": "official-2026-mldsa65",
      "algorithm": "mldsa65",
      "signature_base64": "BASE64_ML_DSA_65_SIGNATURE"
    }
  ]
}
```

Optional recovery or future multi-signature example:

```json
{
  "format_version": 2,
  "signed_object": "repository-index",
  "canonicalization": "fcitx5-windows-next-json-v1",
  "signatures": [
    {
      "key_id": "official-2026-mldsa65",
      "algorithm": "mldsa65",
      "signature_base64": "BASE64_ML_DSA_65_SIGNATURE"
    },
    {
      "key_id": "official-2026-slh-dsa-recovery",
      "algorithm": "slhdsa-sha2-128s",
      "signature_base64": "BASE64_SLH_DSA_SIGNATURE"
    }
  ]
}
```

Initial policy:

- ML-DSA-65 required for official repository index and package manifests.
- SLH-DSA optional and used only when policy or recovery tooling requires it.
- RSA and Ed25519 do not satisfy official repository policy.

The signed bytes must be the exact canonical metadata bytes used by the release
tooling and verifier.

Do not sign a separately serialized object that omits security-relevant fields.
The signed metadata must include at least:

- object type: repository index or package manifest;
- format version;
- product id;
- repository id;
- channel;
- package id, when verifying a package manifest;
- package version, when verifying a package manifest;
- repository generation / rollback counter, when verifying an index;
- payload hashes and sizes, when verifying a package manifest.

## Hashing policy

### Required now

Keep the existing package hash behavior until an implementation task changes it
with tests.

### Future default

For the future package manifest format, BLAKE3 is the required/default payload
hash. SHA-256 is retained as a compatibility hash, not as the strategic default.

Verification policy:

1. For new v2 manifests, `blake3` is required for every payload entry.
2. For compatibility during migration, `sha256` may also be present.
3. If both hashes are present, both must match.
4. If `sha256` is absent in a v2 manifest, the package is still valid as long
   as the signed manifest policy and required BLAKE3 hashes pass.
5. Older v1 manifests keep their existing hash policy until explicitly
   migrated.

Future package manifest example:

```json
{
  "payload": [
    {
      "path": "share/fcitx5/addon/example.conf",
      "size": 1234,
      "hashes": {
        "blake3": "HEX_BLAKE3_REQUIRED_DEFAULT",
        "sha256": "HEX_SHA256_COMPATIBILITY"
      }
    }
  ]
}
```

Policy:

- signature proves authenticity;
- hashes prove payload integrity;
- BLAKE3 is the default payload hash for the future manifest format;
- SHA-256 remains a compatibility hash for ecosystem tooling, SBOM output,
  external scanners, mirrors, and old metadata migration.

If the project later removes SHA-256 compatibility from package manifests, that
should be a separate migration task with release notes and backward
compatibility handling.

## Release-side signing rules

The product must contain only public verification material.

Release tooling may provide:

- offline key generation command;
- public key export command;
- repository index signing command;
- package manifest signing command;
- key rotation helper;
- revoked-key update helper.

Release tooling must not:

- generate production private keys during normal builds;
- commit private keys;
- copy private keys into `out/package`;
- allow unsigned package publication by default;
- silently sign with a test key for official channels.

## Verifier implementation policy

The first implementation task must decide the verifier source before enabling
official online installation:

- prefer a small, audited, verify-only PQC implementation if Windows CNG support
  is not uniformly available on supported systems;
- if using Windows CNG, tests must prove the required ML-DSA parameter set works
  on the minimum supported Windows version;
- private-key signing code should remain release-tooling-only;
- product runtime should need only public-key verification.

Do not show official downloadable add-ons in Settings until repository and
package ML-DSA verification are both green.

## UI/product behavior

Settings/Add-ons must reflect trust state honestly:

- If no trusted official repository key exists, show:
  `Official add-on repository is not configured yet.`
- Do not show fake downloadable official plug-ins.
- If repository metadata signature fails, show a localized trust error.
- If package manifest signature fails, block install/update.
- If a key is revoked, show a localized revoked-key error.
- If rollback is detected, show a localized rollback-protection error.

All dialogs and errors must go through localization resources.

## Implementation slices

### TRUST-001 — PQC keyring parser

Files likely involved:

- `src/package/package_core.h`
- `src/package/package_core.cpp`
- `tests/unit/package_core_test.cpp`
- `security/trusted-keys.template.json`

Acceptance:

- parser accepts `format_version: 2`;
- parser accepts ML-DSA-65 trusted/revoked keys;
- parser optionally accepts one explicitly chosen SLH-DSA recovery key type;
- parser rejects wrong public-key length for each supported algorithm;
- parser rejects duplicate key ids;
- parser rejects unsupported algorithms required by policy;
- existing compatibility tests remain green until old RSA paths are explicitly
  removed.

### TRUST-002 — PQC signature envelope parser

Files likely involved:

- `src/package/package_core.h`
- `src/package/package_core.cpp`
- `tests/unit/package_core_test.cpp`
- repository/package integration tests.

Acceptance:

- parses `index.sig.json` and `manifest.sig.json`;
- validates `format_version`, `signed_object`, `canonicalization`,
  `key_id`, `algorithm`, and base64 signature fields;
- rejects missing required ML-DSA-65 signature;
- rejects unknown required algorithms;
- rejects duplicate signatures from the same key;
- rejects signature envelopes for the wrong object type.

### TRUST-003 — ML-DSA-65 detached signature verification

Files likely involved:

- `src/package/package_core.h`
- `src/package/package_core.cpp`
- `src/package/repository.cpp`
- `src/package/package_archive.cpp`
- `tests/unit/package_core_test.cpp`
- `tests/unit/control_repository_rollback_test.cpp`
- `tests/integration/control_package_integration_test.cpp`

Acceptance:

- repository index verifies with ML-DSA-65;
- package manifest verifies with ML-DSA-65;
- wrong signature fails closed;
- revoked key fails closed;
- wrong key id fails closed;
- channel mismatch fails closed;
- rollback still fails closed.

### TRUST-004 — BLAKE3-default manifest hashes

Files likely involved:

- manifest parser/writer;
- package staging/verification;
- package tests;
- release tooling.

Acceptance:

- v2 manifests require BLAKE3 for every payload;
- SHA-256 remains optional compatibility;
- if both BLAKE3 and SHA-256 are present, both must match;
- corrupted payload fails before activation;
- old v1 package hash behavior remains covered until migration/removal.

### TRUST-005 — release-side PQC signing fixtures

Files likely involved:

- `tools/release.ps1`
- `tools/stage-package.ps1`
- `tools/test-release-artifacts.ps1`
- test fixtures.

Acceptance:

- test repository fixture can be signed deterministically;
- official package build does not embed private keys;
- unsigned release artifacts fail package verification;
- generated `out/package` contains only public trusted keys.

### TRUST-006 — signed repository UI state

Files likely involved:

- Config Add-ons/Updates UI files;
- localization files;
- package/control integration boundary tests.

Acceptance:

- no official online plug-ins are displayed without trusted repository metadata;
- signature, rollback, revoked-key, and missing-key failures are localized;
- no controls or text overlap at required DPI and minimum window sizes.

## Blockers

Implementation should stop and ask for product/security decision if:

- an official repository URL is selected but no official trusted public key is
  available;
- the verifier would require shipping a large new crypto dependency without
  review;
- Windows CNG is selected but the required ML-DSA parameter set does not work on
  the minimum supported Windows version;
- canonical metadata serialization cannot be made deterministic;
- existing package format contradicts the signed metadata requirements.

## Recommended next task

Create `TRUST-001` as the next explicit task only after the current Config UI
task is green or the user explicitly pauses Config work.

The smallest safe first implementation is:

1. add PQC keyring policy parsing;
2. add ML-DSA-65 public key validation;
3. add signature envelope parsing tests;
4. keep old RSA code untouched unless needed for compatibility;
5. do not enable online official package installation until ML-DSA repository
   and package signature verification are both green.
