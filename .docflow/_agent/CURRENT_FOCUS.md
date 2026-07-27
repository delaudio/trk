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
- **Pending integration:** the render-to-dispatch harness and four-size matrix
  now cover pattern cells, scrolled composite lists, scrolled browsers, Help,
  scrolled DSP lists/palette, sampler controls, and adjacent no-ops. The new
  tests exposed and fixed missing DSP device/parameter virtualization. Full
  gate, Codex review, PR, CI, and merge remain.

## Last shipped

Issue #267 Docflow closeout via PR #306.

## Next item

Implement the cross-size matrix for issue #268, then close out its Docflow
records after merge.
