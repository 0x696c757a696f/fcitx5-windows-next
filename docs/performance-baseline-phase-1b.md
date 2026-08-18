# Phase 1B performance baseline

Date: 2026-08-17

This is a trend baseline, not a universal pass/fail threshold. Re-run the same Release binaries
on the same reference machine with:

```powershell
.\tools\benchmark.ps1
```

## Reference machine

- CPU: Intel Core i7-10710U @ 1.10 GHz
- RAM: 15.8 GiB
- OS: Windows 10 IoT Enterprise LTSC, 10.0.19044
- Toolchain: Visual Studio 2022 MSVC 17.14, Windows SDK 10.0.19041.0, CMake 4.4.2
- Samples: 2,000 warm key roundtrips after 100 warm-up operations
- Codec iterations: 200,000 encode/decode pairs

## Results

| Architecture | Codec ns/op | Codec ops/s | Key p50 | Key p95 | Key p99 | Key max |
|---|---:|---:|---:|---:|---:|---:|
| x64 | 1,455.87 | 686,876 | 25.0 us | 50.2 us | 89.5 us | 746.2 us |
| x86 | 2,417.49 | 413,652 | 57.9 us | 107.1 us | 193.0 us | 391.9 us |

The key benchmark uses the production `PipeClient`, the versioned wire codec, a real local named
pipe, and the independent mock-engine process. The 25 ms input deadline remains a fail-open safety
budget; it is not derived from these first measurements and will be reviewed by ADR in Phase 2.

## Trend re-run on 2026-08-18 (v1.6 work tree, protocol v7, Release)

Re-ran `tools/benchmark.ps1` on the same reference machine after the v1.6 candidate-selection
intent, protocol v7 and launcher/tray changes. Same binaries and sample counts as the baseline.

| Architecture | Codec ns/op | Codec ops/s | Key p50 | Key p95 | Key p99 | Key max |
|---|---:|---:|---:|---:|---:|---:|
| x64 | 441.9 | 2,262,710 | 21.6 us | 25.4 us | 40.2 us | 99.1 us |
| x86 | 610.7 | 1,637,590 | 36.2 us | 50.4 us | 61.8 us | 99.8 us |

Additional soak evidence on the same re-run:

- `focus-context-churn` (10,000 identity switches): x64 170.8 ns/switch, x86 211.0 ns/switch,
  stale-context rejection intact (0 stale accepted).
- `handle-leak-soak` (10,000 TSF COM create/destroy cycles): x64 and x86 both
  handle-delta=1, gdi-delta=0, user-delta=0 — no resource leak.
- `tsf-module-activation` x64/x86: COM activation, QueryInterface and teardown pass.

All values remain far below the 100 ms fail-open safety deadline (ADR 0003) and improve on the
2026-08-17 baseline in every measured category. The 100 ms cutoff is distinct from the p95/p99
optimization targets; tail regression continues to be reported by the long-string test.

## Release binary size baseline

| Binary | x64 | x86 |
|---|---:|---:|
| `fcitx5-tsf.dll` | 36,864 bytes | 31,232 bytes |
| `fcitx5-mock-engine.exe` | 29,696 bytes | 26,112 bytes |

The mock engine is a vertical-slice development artifact, not the Phase 3 Fcitx5 engine.
