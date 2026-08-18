# Phase 8 acceptance

> Status: historical v1.5 evidence; Phase 8 is not currently accepted under v1.6.

Date: 2026-08-17  
Specification: Frozen v1.5  
Scope: Release identities, update ownership, rollback, signing, SBOM and distribution
Status: release machinery implemented; Stable publication blocked by external release authority

## Evidence

- Stable, Beta and Nightly generate distinct CLSID/profile, pipe/object, data, registry and AppId
  identities from one source tree. Identity tests prevent cross-channel collisions.
- Installer records `builtin`, `chocolatey`, `winget`, `enterprise`, or `manual` ownership. The built-in
  updater refuses Core activation unless it is the recorded owner; addon/data/theme updates remain
  independently managed.
- Core activation records pending health, keeps at most one complete previous-known-good version,
  rolls back the complete artifact set, and removes the previous version only after a healthy stable
  state. Unit tests cover first activation, second activation, rollback, cleanup and external-owner
  refusal.
- `package` compiles one x64-with-x86-TSF lineage, runs its tests, and records its source commit.
  `release` only promotes that stage: it injects the protected public keyring, Authenticode-signs PE
  files, packages, signs the installer, and never invokes CMake/MSBuild.
- The release gate produces final SHA-256 hashes, a detached signed manifest, SPDX 2.3 SBOM from the
  actual staged files/dependency inventory, SLSA-shaped provenance, WinGet metadata and Chocolatey
  metadata. Final smoke rechecks hashes, Authenticode, SBOM and the detached CMS signature.
- The privileged workflow is manual/tag-bound, keeps build and signing jobs separate, gives the build
  job no secrets, uses a protected `release` environment, grants least permissions, and pins every
  third-party Action to an immutable commit.

Production signing credentials and a non-empty trusted package-key set are intentionally external
release authority. Their absence fails the release command closed; development/package gates never
invent production keys.
