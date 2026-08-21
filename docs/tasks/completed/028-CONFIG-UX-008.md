# CONFIG-UX-008 Package smoke cleanup

**State:** COMPLETED
**Plan source:** `docs/tasks/settings-uiux-operation-integration-plan.md`

## Goal

Ensure package smoke tests shut down launcher/UI/engine processes they start and leave package output unlocked.

## Acceptance

- Portable/package smoke uses a reliable cleanup path.
- Started launcher/UI/engine processes are stopped in success and failure paths.
- Package gate leaves `out/package` artifacts usable after test completion.
- Relevant package smoke tests pass.
