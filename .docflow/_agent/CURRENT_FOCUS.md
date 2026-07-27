# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** agent/263-sampler-direct-controls
- **Active item:** `plan/todo/0015-sampler-direct-pointer-controls.md` for
  GitHub issue #263.
- **Blockers:** none.
- **Pending integration:** typed controls, application routing, responsive
  renderer coverage, pointer no-op coverage, and intentional snapshots are
  complete. The full local gate passes; commit, mandatory Codex review, PR, CI,
  and merge remain.

## Last shipped

Issue #262 closeout via PR #296.

## Next item

Implement typed direct sampler controls for issue #263.
