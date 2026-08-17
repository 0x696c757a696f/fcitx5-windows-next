# Security policy

Report suspected vulnerabilities privately through GitHub Security Advisories. Do not include real
input text, user dictionaries, signing material, or private repository credentials in a public issue.
Supported Stable builds receive security fixes; Beta and Nightly are diagnostic channels and may be
replaced by a newer build.

## Maintainer incident runbook

1. Preserve the affected version, source commit, final hashes, signatures, repository index, logs that
   do not contain input data, and update-owner state. Stop the affected release/repository channel.
2. For a compromised package/update key, mark the key revoked in the trusted keyring, rotate to an
   offline replacement, republish signed metadata, and publish an advisory. Never silently replace
   bytes under an existing version.
3. For a malicious or broken addon, remove it from new repository indexes, retain evidence, mark the
   installed package quarantined/broken where applicable, and keep user data separate from removal.
4. For a bad Core release, stop distribution and use the complete previous-known-good deployment.
   Never mix old TSF, engine, UI, schema, or updater components.
5. For a signing-certificate incident, revoke the certificate with its issuer, remove it from the
   protected release environment, replace it, and reissue under a new version after the full gate.
6. State scope, affected versions/hashes, mitigations, recovery instructions, and whether input data
   could have been exposed. Close only after signed fixed artifacts and repository metadata verify.

The input plane (`fcitx5-tsf.dll`, engine, candidate UI) has no network capability. Only the isolated
downloader owns network access, and it cannot receive preedit, candidates, key events, or commit history.
