# Phase 1B acceptance evidence

Status: **accepted on 2026-08-17**.

## Implemented vertical slice

- Both x86 and x64 build an in-process TSF COM DLL with the four required exports.
- The text service implements `ITfTextInputProcessorEx` and `ITfKeyEventSink`.
- `OnTestKeyDown` is local and read-only; `OnKeyDown` performs a bounded IPC request and commits
  returned text through an edit session.
- Protocol v2 has a fixed little-endian header, explicit version, request correlation, architecture
  handshake, context ID, bounded frame size, and strict rejection of malformed/truncated frames.
- A separate mock-engine process maps `A` through `Z` to lowercase commits.
- The IPC client uses overlapped I/O and one absolute 25 ms deadline. Missing and stalled engines
  fail open, leaving the key unhandled.
- The TSF DLL has no engine, UI, Fcitx, WebView, networking, or addon dependency.

## Automated evidence

Run:

```powershell
.\tools\build.ps1 clean
.\tools\build.ps1 test
.\tools\benchmark.ps1
```

The test command builds x86 and x64 with warnings-as-errors and MSVC `/analyze`, then runs:

- protocol roundtrip, every truncation boundary, and version-mismatch rejection;
- real child-process named-pipe hello + key + commit roundtrip;
- DLL class factory -> TextService key callback -> IPC -> mock engine -> edit-session `SetText`
  automated E2E (only the OS-issued registered-TIP client ID is represented by a test double);
- missing-engine and accepted-but-stalled-engine bounded fail-open paths;
- exact DLL exports, COM class factory activation, and `ITfTextInputProcessorEx` construction;
- paired secret-scanner and license-checker self-tests plus dependency inventory.
- repository text policy: strict UTF-8 without BOM and LF-only line endings.

The initial benchmark and binary-size measurements are recorded in
`docs/performance-baseline-phase-1b.md`.

## Elevated desktop E2E result

TSF and in-process COM registration write machine-level registry keys. The test used:

```powershell
.\tools\register-dev.ps1 register
.\tools\run-mock-engine.ps1 start
```

Observed result:

- OS: Windows 10 IoT Enterprise LTSC 10.0.19044
- Host: 64-bit `C:\Windows\System32\notepad.exe`
- Profile: **Fcitx5 for Windows Next (Development)**
- Input: physical `A` key
- Commit: lowercase `a`, as specified by the Phase 1B mock engine
- Result: Notepad remained responsive; x64 and x86 COM registry views both point to their expected
  Debug DLLs
- x86 evidence: automated DLL lifecycle and full key-to-edit-session E2E pass; no separate 32-bit
  Notepad host was available

This proves the required real `TSF -> IPC -> mock engine -> edit session` desktop path. The mock
engine's lowercase-only behavior is intentional and does not define final Shift/case semantics.

To remove the development registration after testing, run from elevated PowerShell:

```powershell
.\tools\register-dev.ps1 unregister
```
