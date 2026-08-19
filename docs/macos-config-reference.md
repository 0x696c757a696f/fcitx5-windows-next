# fcitx5-macos config/reference notes for Windows Next

Status: current design reference  
Date: 2026-08-19  
Scope: status item, input-method management, plugin manager, generic addon/config UI

This note records the parts of `fcitx5-macos` and the published
`fcitx-contrib` documentation that should shape Windows Next. It is a design
reference, not acceptance evidence and not a license to copy platform-specific
implementation.

## System profile and status item

macOS exposes Fcitx as one platform input source. Pinyin, Shuangpin, Rime,
Mozc and other engines are managed inside Fcitx groups/input methods.
Windows Next follows the same product boundary: register one TSF profile,
`Fcitx5 for Windows Next`, and show the active internal input method through
launcher/tray/config status.

Useful macOS behavior:

- the menu/status item uses a stable-width penguin/status label rather than
  exposing every internal engine as an OS input source;
- the active label comes from Fcitx input-method state, preferring
  `subModeLabel`, then the input-method label/name;
- the status menu can list Fcitx groups/current-group input methods for
  internal switching.

Windows mapping:

- keep one TSF profile and clean obsolete dynamic profiles during
  repair/uninstall;
- query current input method from the engine over IPC;
- tray tooltip/menu displays product state plus the current internal scheme;
- later group/input-method switching belongs in Config/control, not TSF
  registration.

## GUI configuration capabilities in fcitx5-macos

The macOS configuration app is broad, but its important architectural shape is
small: most pages are wrappers around Fcitx's native typed config metadata.

### Input Methods page

Observed capabilities:

- list input-method groups;
- add, remove and rename groups;
- set keyboard layout for a group;
- add/remove/reorder input methods inside a group;
- set per-input-method keyboard layout;
- open the selected input method's own config through
  `fcitx://config/inputmethod/<name>`;
- add available input methods grouped by language, with a current-language
  filter and a short list of popular engines;
- import custom table input methods when the pinyin addon is present.

Windows Next target:

- expose group/current-group/input-method list through control/engine JSON;
- apply group and input-method changes through engine-owned commands;
- never use TSF profile GUIDs as internal input-method selectors;
- auto-add newly installed input methods only after package activation exposes
  descriptor-provided `input_methods`, and only when this will not disturb a
  user-managed multi-group setup.

### Advanced page

Observed capabilities:

- list addon categories and addons from Fcitx;
- selecting an addon points a config manager at
  `fcitx://config/addon/<id>`;
- the detail pane is dynamically rendered from typed config metadata;
- a data-manager entry sits alongside addon configuration.

Windows Next target:

- add a generic addon/config surface instead of hardcoding Pinyin/Rime/Mozc
  option tables;
- start with read/list/get config commands, then add typed set/reset once the
  schema and failure modes are covered by tests;
- keep all addon/config access in the engine process; TSF, candidate UI and
  package code must not load Fcitx addons directly.

### Typed option rendering

The macOS renderer maps Fcitx metadata to widgets:

- `Boolean`
- `Color`
- `Enum`
- `Integer`
- `Key`
- `String`
- annotated strings: application input-method binding, CSS, enum-like string,
  image path, font, JavaScript plugin, Vim mode
- `External`
- `List|...`
- `Entries...`
- nested `Children` groups
- fallback unknown option display

Windows Next target:

- implement the renderer incrementally; start with Boolean, Enum, Integer,
  Key, String, List/Entries and Children;
- show unsupported `External` entries safely with an explicit unavailable
  action until a Windows-specific handler exists;
- keep undo/redo/reset/reload semantics at the config-manager layer.

### Plugin Manager

Observed capabilities:

- installed plugins are read from descriptors;
- available plugins are derived from an official catalog minus installed
  plugins;
- updates compare native and data versions separately;
- install resolves dependencies, downloads native/data artifacts and reloads
  Fcitx;
- uninstall removes descriptor-declared files and restarts because native
  plugin binaries may be loaded;
- after installing plugins, input methods declared by descriptors may be added
  automatically when there is only one group.

Windows Next target:

- the signed package repository remains the only online source of truth;
- repository/package metadata must carry type, title, summary, version,
  architecture, dependencies, trust/release information and state;
- Config should expose package type visibly: addon, input-method data, theme,
  translation, core or component;
- install/update/enable/disable/remove flows continue to use the package
  transaction layer and restart/reload engine through control;
- do not embed a permanent Windows hardcoded official-plugin map.

### Data and specialized managers

Observed capabilities:

- import/export data from Fcitx Android/macOS formats;
- import Squirrel/Hamster/Rime-style user data;
- open user data, font, theme and plugin folders;
- edit custom phrases;
- manage dictionaries;
- edit quick phrases.

Windows Next target:

- treat these as later, specialized panes behind the same Advanced/config
  model;
- Rime user data import must preserve the Fcitx Rime user directory boundary
  and avoid sharing LevelDB databases with another frontend;
- dictionary/custom phrase/quick phrase editors should operate on declared
  user-data locations, never on signed package payload directories.

## Rime-specific constraints from published docs

The public docs describe Rime as an internal Fcitx input method with a user
directory, deployment/sync actions and behavior settings such as preedit mode,
shared input state, switch-input-method behavior, user data directory, deploy
shortcut and sync shortcut. They also warn against sharing the Rime user
directory/LevelDB with another frontend.

Windows Next implications:

- Rime must be installed/enabled as package/addon/data, not registered as a
  separate TSF profile;
- deploy/sync should be exposed as engine/config actions, not shell scripts
  wired into TSF;
- switch behavior must be owned by Fcitx/Rime config so key routing remains
  semantic and testable;
- cloud/network helpers must respect dispatcher deadline/backpressure rules and
  must not block real-time key handling.

## Referenced sources

- Local source reference: `out/reference/fcitx5-macos/src/config/Advanced.swift`
- Local source reference: `out/reference/fcitx5-macos/src/config/InputMethod.swift`
- Local source reference: `out/reference/fcitx5-macos/src/config/OptionView.swift`
- Local source reference: `out/reference/fcitx5-macos/src/config/PluginManager.swift`
- Local source reference: `out/reference/fcitx5-macos/src/config/DataManager.swift`
- Public docs: <https://fcitx-contrib.github.io/docs/>
- Rime docs: <https://fcitx-contrib.github.io/docs/im/rime.html>
- Pinyin/Shuangpin docs: <https://fcitx-contrib.github.io/docs/im/pinyin.html>
