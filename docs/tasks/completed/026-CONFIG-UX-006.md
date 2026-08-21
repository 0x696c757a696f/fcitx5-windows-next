# CONFIG-UX-006 Add-ons/package UX completion

**State:** COMPLETED
**Plan source:** `docs/tasks/settings-uiux-operation-integration-plan.md`

## Goal

Make the Add-ons page accurately represent bundled, installed, online, update, disabled, trust-failure, incompatible, and pending-restart states.

## Acceptance

- Package refresh/install-or-update/enable-disable/remove paths are reachable through the modern Settings UI.
- Unsafe actions are disabled with visible localized explanations.
- No fake official downloadable plug-ins are shown without trusted signed repository metadata.
- Interaction tests cover the modern package action paths.
- Visual contracts prove controls/details do not overlap.
