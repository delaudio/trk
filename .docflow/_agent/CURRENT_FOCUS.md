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
- **Uncommitted work:** distinct one-cell Play and Stop targets now route
  primary clicks through existing start/stop intents; Record, header chrome,
  secondary clicks, drags, and invalid payloads are inert. Focused tests and
  the full local gate are green before implementation commit and Codex review.

## Last shipped

Issue #259 implementation and Docflow closeout via PRs #289 and #290.

## Next item

Implement and ship `plan/todo/0012-distinct-transport-click-targets.md`.
