# RUST-R3-CANDIDATE-POC Rust Candidate UI differential PoC

**State:** MANUAL-PENDING / AUTOMATED-POC-GREEN / CANDIDATE-MODEL-RUST-CUTOVER-GREEN / CANDIDATE-MODEL-HEADER-DELETED / CANDIDATE-INTERACTION-RUST-CUTOVER-GREEN / CANDIDATE-INTERACTION-HEADER-DELETED / CANDIDATE-LAYOUT-RUST-CUTOVER-GREEN / CANDIDATE-LAYOUT-HEADER-DELETED

## Automated evidence completed

- Rust out-of-process Candidate PoC exists and does not use C++ FFI.
- Prohibited behavior evidence is recorded: no hooks, no `SendInput`, no process injection.
- Automated Rust/C++ differential coverage exists for vertical demo and horizontal scroll-demo snapshots.
- Rust PoC has HWND screenshot, MSAA, UIA, DPI, mock-host, layout non-overlap, and layout-driven paint evidence.
- Config Appearance candidate preview is embedded inside `fcitx5-config.exe`, resolves the current theme/config presentation data, and is compared against the real Candidate UI demo by Rust QA.
- Candidate model semantics are Rust-owned in `fcitx5-candidate-core`; the obsolete C++ `candidate_model.h` adapter, old C++ model test, and old model/render perf benches are deleted.
- Candidate layout/render-segment semantics are Rust-owned in `fcitx5-candidate-core`; the obsolete C++ `candidate_layout.h` adapter and old C++ layout test are deleted. The still-C++ Config preview and Candidate UI consumers keep only local temporary Rust ABI adapters until those windows are migrated/cut over.

## Manual-pending evidence

R3 Candidate cannot be archived as fully complete on this machine because the remaining acceptance evidence requires real external hosts and assistive technology:

- real Office/Word host matrix;
- real Chrome/Edge host matrix;
- real VS Code/Terminal/RDP/x86 host coverage beyond mock-host snapshots;
- real Narrator/NVDA screen-reader smoke;
- final product decision on full Rust renderer parity/cutover vs continuing the C++ renderer.

Do not mark these cases passed without actual real-host evidence.

## Must not do

- Do not replace the shipping C++ Candidate UI during this manual-pending PoC state.
- Do not add hooks, `SendInput`, process injection, anti-cheat bypass, credential access, or external exploitation behavior.
