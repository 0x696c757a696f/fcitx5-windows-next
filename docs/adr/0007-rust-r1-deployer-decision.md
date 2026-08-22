# ADR 0007: Conditional Rust R1 elevated deployer decision

## Status

Proposed / decision-gated.

## Context

`RUST-R1-05` is intentionally conditional. The deployer is an elevated boundary,
so the Rust migration rule is stricter than for unelevated package/provider
tools: migration must not enlarge the privileged operation set, and it needs
installer/UAC, packaged update, and Legacy/toolchain evidence before cutover.

The current shipping baseline is the minimal C++ `fcitx5-deployer.exe`.

## Current privileged operation set

The elevated deployer currently exposes only:

- `--version`
- `--self-test`
- `--activate LOCAL_ARCHIVE SHA256 TRANSACTION_ID`

The activation path is bounded to:

1. Refuse non-elevated activation.
2. Verify the deployer is running from `Program Files/Fcitx5/bin/fcitx5-deployer.exe`.
3. Accept only a 64-hex SHA-256 and a safe single-component transaction id.
4. Read trusted keys from the protected install root.
5. Create a protected `.transactions/<transaction_id>` directory.
6. Copy the caller-provided archive using an exclusive, non-reparse input handle.
7. Enforce the 128 MiB artifact budget.
8. Re-hash the protected copy across the elevation boundary.
9. Stage and activate the already validated package through package-core.
10. Remove the protected transaction directory on success or failure.

The deployer does not download, parse repositories, resolve dependencies, launch
arbitrary commands, fetch URLs, inspect live input data, or run as a service.

## Decision gate

Do not cut over the deployer to Rust until the following evidence exists:

- Real cross-account UAC install/update/uninstall evidence for the current
  installer/register/bootstrap ownership model.
- Modern packaged install/update smoke using the exact artifact lineage that
  would contain the Rust deployer.
- A differential operation corpus for the privileged operation list above.
- Confirmation that the Rust artifact/toolchain lane is acceptable for the
  intended supported OS/architecture set, or an explicit Legacy lineage decision.

## Current decision

Keep the minimal C++ deployer as the shipping authoritative implementation for
now. This is not a permanent exemption from Rust; it is a privilege-boundary
stop condition until the missing evidence is available.

## Consequences

- No new privileged Rust binary is introduced in this gate.
- No duplicated long-term deployer business logic is added.
- Future Rust deployer work must start from this operation list and provide
  differential evidence before cutover.
