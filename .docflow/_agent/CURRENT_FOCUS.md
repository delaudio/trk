# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** agent/260-transport-click-targets
- **Active item:** `plan/todo/0012-distinct-transport-click-targets.md` for
  GitHub issue #260.
- **Blockers:** none.
- **Uncommitted work:** ADR 0014 and plan 0012 claim distinct render-owned Play
  and Stop targets, existing start/stop intent routing, and inert Record/header
  chrome before implementation and validation.

## Last shipped

Issue #259 implementation and Docflow closeout via PRs #289 and #290.

## Next item

Implement and ship `plan/todo/0012-distinct-transport-click-targets.md`.
