# Current Task — REG-PEER-ID-001 Handle-based IPC peer executable identity

**Mode:** CHANGE
**Task ID:** `REG-PEER-ID-001`

## Goal

Upgrade peer validation from lexical path equality to handle-based executable identity while preserving SID, session, and pipe ACL checks.

## Specification references

- §0.5 item 10
- IPC authentication/security sections
- Phase 2/5
- `REG-PEER-ID-001`

## Required behavior / implementation contract

- Open/inspect the peer executable using Windows handles and compare canonical final identity appropriate to the current trust boundary.
- Include final path/file identity/volume identity as needed to reject reparse/hardlink surprises under the defined install policy.
- Keep SID + Session + DACL checks; file identity is additional evidence, not a replacement.
- Fail closed on unverifiable peer identity without hanging the TSF host.

## Out of scope

- Authenticode policy redesign
- Package updater

## Required validation

- Expected executable passes.
- Different executable with similar lexical path fails.
- Hardlink/reparse/final-path cases in `REG-PEER-ID-001`.
- Installed Program Files path and developer/test path cases.

## Done when

- Security decision no longer relies only on case-insensitive normalized strings.
- Existing legitimate IPC clients still connect.
- No new blocking I/O enters the per-key hot path.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
