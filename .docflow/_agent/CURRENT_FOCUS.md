# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/321-elektron-parameter-workflow`.
- **Active item:** `.docflow/plan/todo/0030-contextual-parameter-pages.md`.
- **Blockers:** none.
- **Pending integration:** implement, verify, review, and integrate issue #321
  through its own pull request.

## Last shipped

Issue #320 implementation PR #348 and closeout PR #349 are merged; issue #320
is closed and `main` is synchronized at `ce9c1d1`.

## Next item

Implement the six contextual parameter pages, canonical row locks,
performance snapshot reload, and instant mute workflow.

## Exit criteria

1. ADR 0032 acceptance criteria are covered by implementation and tests.
2. The complete repository verification gate and Norn Codex review pass.
3. The implementation PR is squash-merged with CI green and issue #321 is
   closed before the separate Docflow closeout.
