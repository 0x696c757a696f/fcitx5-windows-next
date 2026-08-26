# Current Task — RELEASE-01

**Mode:** RELEASE-GATED
**Task ID:** `REL-01-RELEASE-GATE`
**Prerequisite:** `050` plus required external evidence and intended code-only
cutovers.
**Evidence class:** external/manual evidence plus production release artifacts.

## Goal

Keep release readiness parked until production GitHub Release-backed package
assets, signed repository/update metadata, protected signing/key evidence, and
required real-host/manual compatibility evidence are available.

## Current State

Code-only Settings modernization through `058` is complete. This does not make
the project releasable.

## Blocked External Evidence

- Production GitHub Release-backed official add-on package assets.
- Signed repository/update metadata generated from immutable release assets.
- Trusted production key/credential evidence.
- Narrator/NVDA and real Windows 7/10/11 host compatibility evidence.

## Advancement Rule

Do not mark `REL-01` complete from local CTest, package smoke, or screenshot
evidence. Continue only when the missing external evidence exists or when a new
code-only task is explicitly added to PLAN.
