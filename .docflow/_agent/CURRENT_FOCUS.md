# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/320-strudel-live-coding`.
- **Active item:** `.docflow/plan/todo/0029-strudel-mini-notation-live-coding.md`.
- **Blockers:** none.
- **Pending integration:** claim the work in a draft PR, implement and verify
  the mini-notation and live-coding capability, then squash-merge with CI green.

## Last shipped

Issue #319 and its Docflow closeout are merged; ADR 0030 is Implemented and
the repository is synchronized on `main` at `7c87ccf`.

## Next item

Implement issue #320 from accepted ADR 0031 and queued plan item 0029.

## Exit criteria

1. Parser and evaluator cover the accepted mini-notation subset and scale
   mapping with deterministic tests.
2. Atomic and live TUI workflows mutate canonical cells safely and update
   active playback without a transport stop.
3. Full verification, Norn review, PR CI, squash merge, issue closure, and the
   separate Docflow closeout are complete.
