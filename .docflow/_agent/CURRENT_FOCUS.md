# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/rename-trk-docflow-closeout`
- **Active item:** integrate the completed plan/ADR closeout after implementation
  PR #310 merged with CI green and issue #309 closed.
- **Blockers:** none.
- **Pending integration:** the Docflow closeout commit and PR only; product
  implementation is already on `main`.

## Last shipped

Issue #309 implementation via PR #310, with CI green and ADR 0023 Implemented.

## Next item

Commit and integrate the Docflow closeout, then return to `main` and audit the
open GitHub issue queue.
