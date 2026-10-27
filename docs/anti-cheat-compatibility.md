# Games and anti-cheat

Like every TSF input method, Fcitx5 for Windows Next works by Windows
loading `fcitx5-tsf.dll` into each app that takes text input, games included.
The DLL then talks to the engine (`fcitx5-engine.exe`, started by
`fcitx5-launcher.exe`) over a per-user named pipe, and the candidate window is
drawn by `fcitx5-ui.exe`, a separate process.

Anti-cheat systems (EAC, BattlEye, Vanguard, ACE and others) watch which
modules load into the game and what the game process opens. This project does
not try to hide from or get around them. It aims to be a well-behaved,
identifiable module that degrades to plain keyboard input when a game does not
allow it.

## What can happen

| Situation | Effect | What the IME does |
| --- | --- | --- |
| The anti-cheat refuses unsigned modules | The DLL never loads; the game gets plain keyboard input | Nothing to do in code; Authenticode signing helps (below) |
| The game may not open the engine pipe (access denied) | No Chinese input in that game | Keys pass through unchanged for 10 minutes, then it tries once more. It does not start the launcher from inside the game |
| The game may not start processes | No launcher start from the game | The start fails fast and keys pass through; the launcher that starts at sign-in serves the game instead |
| Exclusive fullscreen | The candidate window (another process) cannot draw over the game | Games that draw their own IME UI get candidates through `ITfCandidateListUIElement`; otherwise use borderless windowed mode |
| You want the IME off in one game | | List the game in `tsf-excluded-apps.txt` (below) |

The DLL never hooks input, writes to other processes or creates remote threads;
`tools/check-runtime-security.ps1` fails the build if such calls appear in
the source. It does not touch the network either.

## Turning the IME off for one game

Create `tsf-excluded-apps.txt` in the data folder:

- installed: `%LOCALAPPDATA%\Fcitx5\tsf-excluded-apps.txt`
- portable: `data\tsf-excluded-apps.txt` next to `portable.flag`

Put one executable name per line (a full path also works; only the file name
counts, case does not matter). Lines starting with `#` are comments.

```text
# Valorant
VALORANT-Win64-Shipping.exe
# Apex Legends
r5apex.exe
```

In a listed app the input method stays selected but handles no keys, so the
game receives every key as typed, as with an English keyboard. The list is
read when the game starts; restart the game after editing it. To type Chinese
in such a game, switch to another input method with Win+Space.

## Code signing

Several anti-cheat systems block or flag modules without a valid Authenticode
signature, and some also check the publisher. Signed releases (through
SignPath) make the DLL identifiable to them and to Windows; a signature does
not by itself put the DLL on any anti-cheat allow list. Until releases are
signed, a game that rejects unsigned modules will simply not load the DLL, and
input in that game falls back to plain keys.

## Runtime dependencies

The DLL links the VC runtime statically and uses only the Universal CRT that
ships with Windows 10 and later, so it does not need the Visual C++
redistributable and cannot bind to an older `vcruntime140.dll` that a game
ships beside its executable. The runtime security check fails the build if the
DLL imports `VCRUNTIME140*.dll` or `MSVCP140*.dll`.
