# Phase 2 threat-model delta

Date: 2026-08-17  
Applies to: protocol v2, named pipes, peer verification, launcher, and mock engine

## Changed trust boundaries

Phase 2 adds two local IPC boundaries: TSF client to engine and TSF/config caller to launcher. The
TSF host is untrusted for availability, and a process with the same user token is not automatically
trusted as the expected product binary. Launcher state is authoritative for whether demand start is
allowed. The mock engine remains a development/test component and is not a production fallback.

| Threat | Required control | Evidence |
|---|---|---|
| Cross-user or cross-session pipe collision | Endpoint contains SID and Session ID; server DACL permits current SID and SYSTEM; handles are non-inheriting | `runtime-identity-contract` inspects namespace and DACL/SACL |
| Remote pipe access | Every server instance uses `PIPE_REJECT_REMOTE_CLIENTS` | engine and launcher implementation |
| Local fake server | Client resolves server PID, SID, session, and exact executable path; mismatch fails closed | `ipc-late-response-reconnect` fake-server case |
| Client self-declaration spoofing | Server resolves client PID from the pipe and compares token SID/session; engine compares actual PID with hello PID | roundtrip and multi-client tests |
| SYSTEM, Session 0, or secure desktop starts user engine | One pure launch policy rejects service accounts, Session 0, secure desktop, or missing SID | synthetic cases in `runtime-identity-contract` |
| Stalled or truncated peer freezes host | All client hot-path I/O shares one 25 ms absolute deadline; server body/write operations are bounded | missing/stalled/truncated contract fixtures |
| Old response matches a new request | `request_id/response_to`, epoch, session, context, composition, and revision are checked; timeout disconnects the pipe | delayed-response, abrupt-exit, new-epoch reconnect test |
| Restart storm | Virtual-clock state model uses exponential backoff and Safe Mode after three startup crashes | `launcher-state-model` |
| User/update stop marker is bypassed | `UserStopped`, `Updating`, and `Uninstalling` are atomically persisted before transition and suppress demand start | state-store reload and launcher lifecycle tests |
| Abrupt launcher exit or orphan engine | Engine is assigned to a kill-on-close job; normal stop uses a named event and bounded wait | launcher lifecycle integration |
| Development exception reaches a release build | CMake default is OFF; only `windows-*-dev` presets opt in; release-validation presets force OFF | x64/x86 release builds |

## Residual risks and later gates

- Public signed releases must add Authenticode/WinVerifyTrust verification in the release phase.
  Phase 2 verifies installed-image path identity; the explicitly compiled development exception is
  same-user/session only and is never the default.
- The current binary is a mock engine. Addon isolation and Safe Mode disabling of real addons become
  testable only after Phase 3/5.
- Win7 VM behavior and modern-token edge cases remain compatibility-matrix work; no unsupported
  modern API is intentionally required by the TSF hot path.
- State files are availability controls, not a privilege boundary. Invalid persisted state fails
  conservatively to `UserStopped` and is not silently rewritten.
