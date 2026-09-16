# REL-01 — Current-head Build-Once evidence

Status: `AUTOMATED-GREEN / MANUAL-PENDING`

## Build identity

- Source HEAD: `6862ca3722f42d970fd98592620c975baf3e10be`
- Source tree: clean at staging time (`source_tree_clean=true`)
- Command: `tools/build.ps1 package -Architecture all -Configuration Release -Version 0.1.0 -Channel stable`
- Build model: one tested `x64-with-x86-tsf` lineage; no signing-stage rebuild
- Stage: `D:\Documents\GitHub\fcitx5-windows-next\out\package\stage-c8dc35adfa7e48e696476f35044dda2e\Fcitx5`
- Stage pointer: `out\package\current-stage.txt`

The stage manifest records:

```text
source_commit     = 6862ca3722f42d970fd98592620c975baf3e10be
source_tree_clean = true
architecture      = x64-with-x86-tsf
version           = 0.1.0
channel           = stable
```

## Automated results

- x64 Release configure/build: passed.
- x86 Release configure/build: passed.
- Real Fcitx acceptance clients: x64 and x86 baseline, typing-fuzz, `chttrans`, safe-mode, and Rime/Lua lifecycle: passed.
- Typing fuzz: 4,000 iterations per architecture, zero recovered failures.
- Runtime security audit, secret scan, license inventory, locale validation, text-format validation, and production-signing-credential separation: passed.
- Release package generation: passed; Inno Setup produced the setup executable from the tested stage.
- Portable ZIP move, launch, and user-data-preserving upgrade self-tests: passed.
- Final package gate: passed using the tested Release artifacts.

## Artifact hashes

| Artifact | Size | SHA-256 |
| --- | ---: | --- |
| `out\package\artifacts\fcitx5-windows-0.1.0-portable.zip` | 134,858,638 bytes | `BDB3421C6475DBF3061AB727E8705F36F7932E61C355F82718B8673EC379D71B` |
| `out\package\artifacts\fcitx5-windows-0.1.0-setup.exe` | 107,823,850 bytes | `F0D579A16C3DB585FF5DD10A9A25EED86FE69912DD9865B6030E70F9947C75FC` |
| stage `Fcitx5\manifest.json` | — | `6146F803F01D8F2513322E800BC6DA5AC4782BF22905D61DE0A26BFB00A78526` |

These are local staged artifacts for inspection. They are not a production-signed release and do not prove Authenticode, timestamp, UAC, or publication readiness.

## Evidence still required

The following remain `MANUAL-PENDING` or `CI-UNVERIFIABLE` and are not inferred from this build:

- production Authenticode certificate, protected private key, timestamp, package signing, and final publication;
- elevated install, repair, update, rollback, uninstall, and cross-account UAC behavior;
- real typing-time Candidate UI visual/interaction behavior, UIA, Narrator/NVDA, accessibility, DPI, font fallback, and device-loss evidence;
- Windows 7 and complete Windows 10/11 host matrix;
- real 2-core/4-GB, low-storage, offline, and constrained-network runs;
- online signed Rime/plugin repository lifecycle (discover, download, verify, install, enable/disable, update, repair, rollback, uninstall);
- GitHub Actions results.

No `tools/build.ps1 release` run was claimed because production signing credentials and protected release key material are unavailable in this environment.
