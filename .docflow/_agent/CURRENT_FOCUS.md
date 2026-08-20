# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/314-variation-history`.
- **Active item:** `.docflow/plan/todo/0023-pattern-variation-history.md` for
  GitHub issue #314.
- **Blockers:** none.
- **Pending integration:** implement persistent snapshots and the rollback
  modal, pass the full gate and Norn Codex review, then merge through CI.

## Last shipped

Issue #313 implementation via PR #334 and Docflow closeout via PR #335, both
with CI green; ADR 0024 is Implemented.

## Next item

Implement ADR 0025 acceptance criteria and integrate issue #314 before
selecting issue #315.
