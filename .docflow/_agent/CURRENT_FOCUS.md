# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** agent/256-command-palette-mouse
- **Active item:** `plan/todo/0008-command-palette-entry-clicks.md` for GitHub
  issue #256.
- **Blockers:** none.
- **Uncommitted work:** none expected. Command palette target, click, and
  scoped-wheel implementation passes focused tests and awaits the full gate,
  Codex-provider review, and push to draft PR #283.

## Last shipped

Issue #255 implementation and Docflow closeout via PRs #281 and #282.

## Next item

Implement and ship `plan/todo/0008-command-palette-entry-clicks.md`.
