# ADR 0001: Phase 2 input IPC deadline

- Status: accepted for Phase 2; mandatory review in Phase 3
- Date: 2026-08-17
- Scope: synchronous TSF key request to engine response

## Context

The engineering specification requires bounded input-thread I/O, fail-open behavior, and Phase 2
p50/p95/p99 evidence before fixing the initial 25 ms budget. A missing, stalled, malformed, or
untrusted engine must not freeze its host. Cold startup is handled by launcher warm-up and must not
be hidden by a multi-second key deadline.

The Release benchmark uses the production `PipeClient`, protocol v2, an independent mock-engine
process, 100 warm-up requests, and 2,000 measured requests. Release-validation presets explicitly
compile with `FCITX_DEVELOPMENT_PEER_EXCEPTION=OFF`.

| Architecture | Run | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|---:|
| x64 | 1 | 27.8 us | 39.7 us | 51.9 us | 106.4 us |
| x64 | 2 | 29.6 us | 45.8 us | 54.5 us | 78.2 us |
| x64 | 3 | 30.8 us | 47.0 us | 82.6 us | 121.1 us |
| x86 | 1 | 27.6 us | 37.5 us | 63.8 us | 92.0 us |
| x86 | 2 | 33.1 us | 51.8 us | 84.1 us | 110.2 us |
| x86 | 3 | 35.0 us | 60.2 us | 97.4 us | 947.3 us |

The separate codec measurements were 816.768 ns/op on x64 and 959.793 ns/op on x86. The first
post-build x64 roundtrip run produced a 67 us p95; immediate repeated runs above show that it was
scheduler/warm-system variation rather than a persistent regression.

## Decision

Retain one absolute 25 ms deadline across connect, peer verification, handshake, write, and read.
On expiry, cancel the overlapped operation, disconnect, clear handshake/epoch state, and pass the
key through. A launcher start request may consume only the remainder of that same deadline.

The performance objectives remain p95 <= 5 ms and p99 <= 10 ms. They are trend gates distinct from
the 25 ms safety cutoff. Do not add retries, sleeps, or a larger deadline to improve cold-start
success.

## Consequences

- The Phase 2 mock path has substantial headroom, but does not predict real Fcitx/addon latency.
- Phase 3 must repeat warm and cold measurements with keyboard/table and reconsider this ADR.
- Timeout, delayed response, engine-exit, and reconnect schedules remain deterministic regression
  tests.
- A deadline increase requires a new ADR with host-impact evidence; a performance regression may
  not be made green by silently changing the cutoff.

## References

- Engineering specification sections 4.5, 13.7.1, 13.7.2, 13.7.11, and 13.10.4.
- [Microsoft: synchronous and overlapped I/O](https://learn.microsoft.com/windows/win32/fileio/synchronous-and-asynchronous-i-o)
- [Microsoft: ConnectNamedPipe](https://learn.microsoft.com/windows/win32/api/namedpipeapi/nf-namedpipeapi-connectnamedpipe)
