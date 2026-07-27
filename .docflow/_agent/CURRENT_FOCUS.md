# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** agent/264-disabled-workspace-controls
- **Active item:** `plan/todo/0016-disabled-workspace-affordances.md` for
  GitHub issue #264.
- **Blockers:** none.
- **Pending integration:** active/enabled/disabled tab states, shared disabled
  styling, placeholder audit, non-mutating click coverage, style/interaction
  coverage, and intentional snapshots are complete. The full local gate passes;
  commit, mandatory Codex review, PR, CI, and merge remain.

## Last shipped

Issue #263 closeout via PR #298.

## Next item

Implement explicit disabled workspace affordances for issue #264.
