# Stable release checklist

- [ ] Release commit is clean, reviewed, tagged `v<version>`, and matches the requested version.
- [ ] x64/x86 Release package gate, real Fcitx integration, static analysis, fuzz smoke, runtime import,
      secret, dependency, license, locale and UTF-8/LF policies pass.
- [ ] Protected keyring includes current and revoked key records; no private update key is in CI.
- [ ] Signing certificate is valid, protected, timestamping succeeds, and build jobs cannot read it.
- [ ] `release` promotes the recorded tested stage and does not compile source.
- [ ] Installer and portable smoke use the actual final signed artifacts.
- [ ] Interactive desktop gate passed against the exact staged lineage and
      `out/evidence/desktop-verification.json` records tray, Config Apply behavior, real Notepad
      commit, engine restart and engine-absent fail-open.
- [ ] In-use TSF upgrade evidence proves `rename-old -> install-new -> delayed cleanup` and
      generation-specific IPC routing; the upgrade does not forcibly close or inject into TSF host
      applications.
- [ ] TSF activation guard evidence proves stale/broken activation fail-opens, Control reports the
      marker, reset clears it, and ordinary host typing works without logoff/reboot/Safe Mode.
- [ ] Product-function inventory in `docs/ssdlc-verification-matrix.md` has no unowned or
      implicitly-passed row; settings persistence, real/preview renderer parity, long typing,
      scroll candidates, tray recovery and every bundled engine/addon have current evidence.
- [ ] Every applicable case in `docs/product-test-plan.md` has current same-lineage evidence; the
      Config HWND inventory and real tray popup action sweep pass without an untested command.
- [ ] Signed production package repository and non-empty protected keyring pass refresh, install,
      update, disable/enable, uninstall, interrupted activation, rollback and offline scenarios.
- [ ] Compatibility evidence covers the declared Modern and Legacy host/DPI/session matrix; any
      unsupported row is called out as a release blocker or explicit scoped exception.
- [ ] Final manifest hashes, detached CMS signature, Authenticode, SPDX SBOM and provenance verify.
- [ ] WinGet/Chocolatey metadata points at the final installer SHA-256 and records its update owner.
- [ ] Release notes state user-visible changes, compatibility changes, security fixes and known issues.
- [ ] Published assets exactly match the release manifest; previous versions are never overwritten.
- [ ] Rollback target and key/addon/bad-release incident procedures in `SECURITY.md` are usable.
