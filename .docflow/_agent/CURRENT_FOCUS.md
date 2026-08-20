# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/317-web-companion`.
- **Active item:** `.docflow/plan/todo/0026-local-web-companion.md`.
- **Blockers:** none.
- **Pending integration:** publish the accepted decision and plan in a draft PR,
  then implement, review, verify, and squash-merge issue #317.

## Last shipped

Issue #316 implementation via PR #340 and Docflow closeout via PR #341, both
with CI green; ADR 0027 is Implemented and the GitHub issue is closed.

## Next item

Implement ADR 0028 through the issue #317 PR, close its Docflow state after the
implementation merge, then clear operational context before selecting #318.
