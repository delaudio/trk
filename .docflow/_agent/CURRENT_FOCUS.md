# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/316-external-editor`.
- **Active item:** `.docflow/plan/todo/0025-external-editor-hot-reload.md` for
  GitHub issue #316.
- **Blockers:** none.
- **Pending integration:** claim the accepted editor/hot-reload contract, then
  implement, verify, review, and merge it through CI.

## Last shipped

Issue #315 implementation via PR #338 and Docflow closeout via PR #339, both
with CI green; ADR 0026 is Implemented.

## Next item

Implement ADR 0027 acceptance criteria and integrate issue #316 before
selecting issue #317.
