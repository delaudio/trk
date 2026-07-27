# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** agent/268-cross-size-mouse-regressions
- **Active item:** implement
  `plan/todo/0020-cross-size-mouse-regressions.md` for GitHub issue #268.
- **Blockers:** none.
- **Pending integration:** audit found the application tests use synthetic
  fixed hitboxes while renderer tests do not dispatch input. Add a render-to-
  dispatch test harness and a four-size interaction matrix before gate,
  Codex review, PR, CI, and merge.

## Last shipped

Issue #267 Docflow closeout via PR #306.

## Next item

Implement the cross-size matrix for issue #268, then close out its Docflow
records after merge.
