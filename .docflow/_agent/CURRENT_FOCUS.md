# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `main` is authoritative once this state-sync change lands; the
  source branch exists only to satisfy PR-based integration.
- **Active item:** none; the plan queue is empty.
- **Blockers:** none.
- **Pending integration:** none after this state-sync change lands.

## Last shipped

Issue #309 implementation via PR #310 and Docflow closeout via PR #311, both
with CI green; ADR 0023 is Implemented.

## Next item

Audit the open GitHub issue queue and select the next smallest actionable item.
