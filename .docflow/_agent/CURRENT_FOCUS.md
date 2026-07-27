# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** agent/267-hovered-wheel-routing
- **Active item:** implement `plan/todo/0019-hovered-wheel-routing.md` for
  GitHub issue #267.
- **Blockers:** none.
- **Pending integration:** semantic scroll-target classification,
  coordinate-based vertical/horizontal routing, loaded-waveform targeting,
  modal capture, and exact rendered-content tests are implemented. Codex
  finding `164be98a98636f7b` was remediated by excluding panel chrome and empty
  padding; full gate, final Codex review, PR, CI, merge, and closeout remain.

## Last shipped

Issue #266 closeout via PR #304.

## Next item

Implement, verify, review, push, merge, and close issue #267.
