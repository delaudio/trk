# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** agent/257-help-overlay-mouse
- **Active item:** `plan/todo/0009-help-overlay-pointer-controls.md` for GitHub
  issue #257.
- **Blockers:** none.
- **Uncommitted work:** render-owned Help tabs, content and close targets,
  primary-click routing, scoped wheel input, modal outside-click capture, and
  focused renderer/application tests pass the full local gate and await commit,
  Codex-provider review, and push to draft PR #285.

## Last shipped

Issue #256 implementation and Docflow closeout via PRs #283 and #284.

## Next item

Implement and ship `plan/todo/0009-help-overlay-pointer-controls.md`.
