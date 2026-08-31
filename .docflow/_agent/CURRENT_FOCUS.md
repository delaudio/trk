# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/322-scale-lock-chord-identifier`.
- **Active item:** `.docflow/plan/todo/0031-scale-lock-and-chord-identification.md`.
- **Blockers:** none.
- **Pending integration:** implement, verify, review, and integrate issue #322
  through its own pull request.

## Last shipped

Issue #321 implementation PR #350 and closeout PR #351 are merged; issue #321
is closed and `main` is synchronized at `c5f804a`.

## Next item

Implement session-only Scale Lock QWERTY entry and real-time chord naming for
issue #322.

## Exit criteria

1. ADR 0033 acceptance criteria are covered by implementation and tests.
2. The complete repository verification gate and Norn Codex review pass.
3. The implementation PR is squash-merged with CI green and issue #322 is
   closed before the separate Docflow closeout.
