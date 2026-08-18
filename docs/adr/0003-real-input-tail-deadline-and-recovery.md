# ADR 0003: Real-input tail deadline and connection-scoped recovery

- Status: accepted; supersedes the established-context deadline in ADR 0002
- Date: 2026-08-17
- Scope: synchronous TSF key request to native Fcitx5 engine

## Context

ADR 0002 retained a 25 ms established-context deadline after measuring a short Pinyin sequence.
Longer stateful testing exposed tail latency that the short sample did not cover. On the reference
machine, an x64 Release run measured a 20.805 ms first key and a 14.943 ms warm key before a later
context-switching key exceeded 25 ms. An x86 Debug long-string run exceeded 25 ms in round 46.
At 25 ms, a 4,000-event x64 stateful fuzz run had 10 transport timeouts.

The timeout also exposed a correctness defect: the client discarded its revision state while the
engine retained the old process/context state. Every request after reconnect was then rejected as
stale, producing a permanent input stall. A process ID is not a sufficient cleanup key because one
host process can own multiple TSF connections.

The audited mature implementations use much wider bounded waits: PIME uses 2,000 ms pipe I/O and
the current Moqi source uses an 8,000 ms reply ceiling while consuming printable keys on RPC
failure. Those values demonstrate the need to accommodate real backend tails, but are too wide for
this project's host-protection target.

## Decision

- Use one 100 ms absolute client deadline for both new and established input contexts.
- Limit the engine dispatcher operation to 75 ms, reserving the remaining client budget for frame
  encoding and pipe return.
- Assign every verified pipe connection a server-side connection ID. Fcitx InputContexts,
  revisions, and compositions are keyed by process ID, connection ID, and TSF context ID.
- On disconnect, remove only that connection's contexts. A reconnect starts clean even when the
  engine epoch is unchanged; another TSF connection in the same host process is not disturbed.
- Keep timeout behavior fail-open. Do not retry an ambiguous timed-out key because the engine may
  already have processed it.
- Gate the path with a 900-key repeated `ha + Space` smoke and a deterministic 4,000-event,
  16-context stateful typing fuzz. The fuzz must recover after every transport timeout and fails if
  the timeout rate exceeds one percent.

Release results after the decision, on the same reference machine and with the real Pinyin engine:

| Architecture | Samples | p50 | p95 | p99 | max | Stateful fuzz |
|---|---:|---:|---:|---:|---:|---:|
| x64 | 900 | 4.259 ms | 10.701 ms | 12.055 ms | 15.431 ms | 4,000 events, 0 timeouts |
| x86 | 900 | 4.593 ms | 11.123 ms | 12.675 ms | 23.632 ms | 4,000 events, 0 timeouts |

The p95/p99 values exceed the initial NFR-P001 objectives and therefore remain an optimization
target. They no longer cross the host-protection cutoff or poison the following connection state.

## Consequences

- A hung engine still cannot block a host indefinitely; the bound remains far below the audited
  mature implementations and is not a cold-start allowance measured in seconds.
- The 100 ms safety cutoff is distinct from the p95/p99 performance objectives. Tail regression is
  reported by the long-string test rather than hidden by the larger cutoff.
- A single slow key may fail open, but it cannot poison all later input through stale revision
  state.
- The measured p95/p99 regression remains visible even though the safety cutoff is now usable.

## References

- Engineering specification sections 4.5 and 13.7.1–13.7.2.
- `out/research/pime/PIMETextService/PIMEClient.cpp`
- `out/research/moqi/MoqiTextService/MoqiClient.cpp`
