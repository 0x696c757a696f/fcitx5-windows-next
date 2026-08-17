# Phase 2 acceptance record

Date: 2026-08-17  
Status: accepted

## Delivered contract

- Protocol v2 uses an explicit 64-byte little-endian header with message type, bounded payload,
  request/response correlation, engine epoch, session, context, composition, and revision.
- Hot frames are capped at 256 KiB and launcher/control frames at 1 MiB. Typed decoders require
  exact payload consumption and reject unknown enums, malformed status values, truncation, and
  trailing data.
- Engine and launcher endpoints are per SID/session with explicit protected DACL, mandatory-label
  SACL, non-inheriting handles, and remote-client rejection.
- Both sides resolve peer process identity from the pipe. Exact executable policy is fail-closed;
  the same-user/session exception exists only in explicit development presets.
- The mock engine uses four fixed workers, not thread-per-client/request. Launcher keeps an
  overlapped standby control instance and serializes lifecycle changes through one state owner.
- Launcher implements Normal, UserStopped, Updating, Uninstalling, CrashBackoff, and SafeMode;
  marker states are atomically persisted before transition. Engine shutdown is event-driven with a
  two-second deadline and forced termination only as a last resort.

## Acceptance evidence

| Specification result | Evidence | Result |
|---|---|---|
| Stable mock-engine roundtrip | x86/x64 key-to-commit and TSF E2E | Pass |
| Malformed/truncated protocol safely rejected | exhaustive truncation, enum/length cases, 20,000 arbitrary-byte fuzz smoke | Pass |
| Legal codec values roundtrip | fixed-seed 10,000 request/response property cases | Pass |
| Multiple clients connect concurrently | four barrier-synchronized clients and four fixed workers | Pass |
| SYSTEM/LogonUI does not launch user engine | injected service, Session 0, and secure-desktop policy cases | Pass |
| Timeout and late response have deterministic terminal state | stalled server, delayed response, disconnect, and new-epoch reconnect | Pass |
| Engine exits between request and response | explicit abrupt-close schedule followed by clean reconnect | Pass |
| Backoff/Safe Mode is deterministic | virtual clock: 250 ms, 500 ms, third crash Safe Mode, stable reset | Pass |
| Stop/update/uninstall cannot be demand-started | state model plus persisted UserStopped reload | Pass |
| Launcher/engine lifecycle | start, ready, key, user stop, suppression, resume, graceful shutdown; 20 repeated passes | Pass |
| Security policy | fake server path rejected; DACL/SACL inspected; PID/SID/session checked | Pass |
| Input wait is bounded and measured | ADR 0001; x64/x86 p50/p95/p99 Release data | Pass |

The final x64 and x86 Debug gates each passed 13 tests under MSVC `/analyze`, warnings-as-errors,
secret self-test/scan, license self-test/inventory, dependency scan, and UTF-8-without-BOM/LF check.
Release-validation builds passed for x64 and x86 with the development peer exception disabled.

## Phase 2 baselines

Reference machine: Intel Core i7-10710U, 15.8 GiB RAM, Windows 10 IoT Enterprise LTSC
10.0.19044, MSVC 19.44/VS 17.14, Windows SDK 10.0.19041, Release optimization.

| Architecture | Codec ns/op | Representative key p50/p95/p99 |
|---|---:|---:|
| x64 | 816.768 | 29.6 / 45.8 / 54.5 us |
| x86 | 959.793 | 33.1 / 51.8 / 84.1 us |

x64 launcher idle measurement over 60.01 seconds: 2,080,768 private bytes and 0.00% normalized
CPU, below the 16 MiB and 0.2% initial budgets.

| Release binary | x64 | x86 |
|---|---:|---:|
| `fcitx5-tsf.dll` | 55,296 bytes | 45,056 bytes |
| `fcitx5-launcher.exe` | 54,784 bytes | 46,080 bytes |
| `fcitx5-mock-engine.exe` | 43,520 bytes | 37,376 bytes |

TSF and mock-engine sizes grew more than 20% from the deliberately minimal Phase 1B slice because
Phase 2 added the full identity metadata, peer authentication, bounded launcher client, security
descriptor code, fixed concurrent workers, and cancellation paths. The absolute sizes remain small;
the next meaningful engine baseline is Phase 3 with real Fcitx.

## Scope boundary

This acceptance does not claim Fcitx/libime integration, real composition, candidate UI, installer,
package management, or public release readiness. Those remain Phase 3 and later work.
