# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** agent/267-docflow-closeout
- **Active item:** close out
  `plan/done/2026-07-27-hovered-wheel-routing.md` after GitHub issue #267
  shipped.
- **Blockers:** none.
- **Pending integration:** implementation squash `3b20b85` is on `main`, PR
  #305 and CI #338 are green, issue #267 is closed, and final implementation
  Codex review `run-1785165122597923000` was clean. The atomic Docflow
  move/status/index/worklog closeout still needs gate, Codex review, PR, CI,
  and merge.

## Last shipped

Issue #267 implementation via PR #305.

## Next item

Audit and implement GitHub issue #268 after the #267 Docflow closeout merges.
