# ADR 0005: TSF DLL 更新采用 generation draining

- Status: Accepted under v1.7 stabilization
- Date: 2026-08-20
- Scope: installed `fcitx5-tsf.dll`, updater/deployer/launcher runtime layout, IPC versioning

## Context

`fcitx5-tsf.dll` is loaded into arbitrary host processes such as Office, browsers, editors,
Explorer, games, and enterprise applications. Treating an in-use TSF DLL like an ordinary helper
binary would make upgrades depend on closing or killing a large set of unrelated applications.
That violates the product boundary: the input method must not inject into, force-unload from, or
control host processes merely to update itself.

Microsoft documents a supported in-use DLL update pattern: rename the old DLL on the same volume,
copy the new DLL back to the original path, then schedule deletion of the renamed DLL with
`MoveFileEx(..., NULL, MOVEFILE_DELAY_UNTIL_REBOOT)` if immediate cleanup is not possible. Existing
processes keep using the old loaded image; future loads use the new file at the original path.
Microsoft also notes that DLLs using global state or service protocols must be prepared for this
mixed state, otherwise a full restart is required.

This conflicts with the product rule that runtime IPC does not keep old protocol compatibility.
During an update, an old TSF DLL may remain loaded in Word while a newly started browser loads the
new TSF DLL. If there is only one upgraded engine that only accepts the new protocol, the old host
immediately loses input.

## Decision

Use deployment-level generation side-by-side draining.

Same-generation components must remain strictly compatible and same-lineage. Runtime protocol
compatibility across generations is not preserved. Instead, each installed generation owns its own
runtime directory, process generation, and IPC namespace:

```text
Program Files\Fcitx5\
  current.json                  # active generation and build metadata

  tsf\
    x64\fcitx5-tsf.dll          # current generation TSF entrypoint
    x64\fcitx5-tsf.old.<gen>.<id>.dll
    x86\fcitx5-tsf.dll
    x86\fcitx5-tsf.old.<gen>.<id>.dll

  runtime\
    00000041\
      bin\fcitx5-engine.exe
      bin\fcitx5-ui.exe
      lib\...
      share\...
    00000042\
      bin\fcitx5-engine.exe
      bin\fcitx5-ui.exe
      lib\...
      share\...

  management\
    fcitx5-control.exe
    fcitx5-updater.exe
    fcitx5-deployer.exe
    fcitx5-provider.exe
```

The TSF update transaction is:

1. Verify the new TSF DLL, runtime generation, manifest, hashes, signatures, and minimum OS/import
   policy before touching the active install.
2. Stage the new generation under `runtime\<generation>` and write durable metadata.
3. Rename each installed `fcitx5-tsf.dll` to
   `fcitx5-tsf.old.<generation>.<unique-id>.dll` on the same volume, without
   `MOVEFILE_COPY_ALLOWED`.
4. Place the new `fcitx5-tsf.dll` at the original TSF registration path and write the adjacent
   `fcitx5-tsf.generation` sidecar for that architecture.
5. Atomically advance `current.json` only after the new generation passes activation/health checks.
6. Attempt to delete old TSF DLLs and old runtime generations when their clients drain. If deletion
   fails with sharing/lock errors, record pending cleanup and retry on launcher/updater startup.
   Final fallback is `MOVEFILE_DELAY_UNTIL_REBOOT`.

TSF handshake and launcher routing must include at least:

```text
release_generation
protocol_version
architecture
build_id
```

The IPC endpoint must include the release generation, for example:

```text
\\.\pipe\Fcitx5\<sid-or-session>\generation-00000042\...
```

Therefore:

```text
old host TSF 41  <-> engine/UI/runtime 41
new host TSF 42  <-> engine/UI/runtime 42
```

There is no permanent dual stack inside one engine. Multiple complete generations may coexist only
as a deployment drain state.

Implementation seam:

- IPC and local object names include `.Generation.<generation>` so same-user/session processes from
  different generations cannot accidentally share `engine`, `launcher`, candidate notification, or
  lifecycle objects.
- `current.json` records the active runtime generation, previous generation, and build id. Publishing
  a generation is refused unless `runtime\<generation>` exists.
- `fcitx5-updater` owns low-level helpers for TSF rename/install and old-TSF cleanup. The full
  installed update transaction must compose these helpers with package activation, current.json
  publication, activation health, rollback, and later cleanup evidence.
- Old renamed TSF DLLs parse their generation from `fcitx5-tsf.old.<generation>...dll` and route
  engine/launcher IPC to that generation. The current `fcitx5-tsf.dll` first reads the adjacent
  `fcitx5-tsf.generation` sidecar, then falls back to `current.json` or an explicitly supplied
  generation context. This avoids a race where a newly copied DLL observes the old global current
  generation, or an old loaded DLL observes the new one.

Each TSF generation also has a per-user activation guard. The guard is not Windows Safe Mode and
does not unregister the TIP. If a previous activation attempt is left unfinished or an activation
step fails, the TSF DLL must fail open inside the host process: return successful activation, avoid
advising the key sink or connecting to the engine, and leave all keys to the host editor. Control
and repair tooling must expose the guard status and a user-scoped reset path. This prevents a bad
TSF generation from turning ordinary applications into recovery blockers that require logoff,
reboot, or Safe Mode deletion.

Do not force `FreeLibrary`, inject into host processes, or remotely unload TSF DLLs. Do not make
Restart Manager shutdown of host applications the default update path. Restart Manager may be used
only for diagnostics, explicit user-visible remediation, or cases where a full installer action
cannot proceed without closing management-owned processes.

## Consequences

- In-use TSF updates no longer require killing Word, Chrome, VS Code, Explorer, games, or other
  host processes.
- Breaking IPC changes remain allowed because compatibility is scoped to a generation.
- Launcher, control, updater, and TSF need a real generation seam. This is a deep module boundary:
  callers route by generation rather than interpreting arbitrary protocol compatibility.
- Disk layout and cleanup become more complex. The release gate must prove old generations are
  bounded, discoverable, and eventually removable.
- Rollback becomes cleaner: failed activation can switch `current.json` back to the previous
  generation while old hosts continue on the old runtime.
- Uninstall must remove current generation files, unregister TSF profiles, and either delete or
  schedule deletion of old generation artifacts without treating still-loaded host processes as
  failures.

## Required verification

- Unit contract for TSF rename-old/install-new path generation and delayed-delete fallback.
- Unit/control contract for the TSF activation guard: disabled marker, stale activation detection,
  status reporting, reset, and fail-open activation.
- Launcher/control contract proving generation-specific pipe endpoints and peer identity checks.
- Runtime metadata contract proving `current.json` only advances to an existing runtime generation
  and preserves previous generation metadata for rollback/draining.
- Upgrade smoke with a simulated locked old TSF DLL: new file appears at the registered path, old
  file is renamed, cleanup is pending, and no host process termination is attempted.
- Mixed-generation protocol test: generation 41 TSF cannot bind to generation 42 engine and instead
  routes to generation 41 or fails open if its generation has been explicitly retired.
- Rollback test: failed generation 42 health check restores `current.json` to generation 41 without
  overwriting generation 41 runtime files.
- Stable desktop evidence: upgrade while a TSF host remains open, then verify old host continues
  input and newly started host uses the new generation.

## References

- [Microsoft: Dynamic-Link Library Updates](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-updates)
- [Microsoft: MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw)
- [Microsoft: Restart Manager](https://learn.microsoft.com/en-us/windows/win32/rstmgr/restart-manager-portal)
