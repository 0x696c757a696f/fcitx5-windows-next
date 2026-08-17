# Stable release checklist

- [ ] Release commit is clean, reviewed, tagged `v<version>`, and matches the requested version.
- [ ] x64/x86 Release package gate, real Fcitx integration, static analysis, fuzz smoke, runtime import,
      secret, dependency, license, locale and UTF-8/LF policies pass.
- [ ] Protected keyring includes current and revoked key records; no private update key is in CI.
- [ ] Signing certificate is valid, protected, timestamping succeeds, and build jobs cannot read it.
- [ ] `release` promotes the recorded tested stage and does not compile source.
- [ ] Installer and portable smoke use the actual final signed artifacts.
- [ ] Final manifest hashes, detached CMS signature, Authenticode, SPDX SBOM and provenance verify.
- [ ] WinGet/Chocolatey metadata points at the final installer SHA-256 and records its update owner.
- [ ] Release notes state user-visible changes, compatibility changes, security fixes and known issues.
- [ ] Published assets exactly match the release manifest; previous versions are never overwritten.
- [ ] Rollback target and key/addon/bad-release incident procedures in `SECURITY.md` are usable.
