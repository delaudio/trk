# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/314-docflow-closeout`.
- **Active item:** close out merged GitHub issue #314 and ADR 0025.
- **Blockers:** none.
- **Pending integration:** publish the plan move, Implemented status, INDEX,
  and shipped audit trail through a closeout PR.

## Last shipped

Issue #314 implementation via PR #336 with CI green; issue #314 is closed and
implementation squash `ff4512f` is on `main`.

## Next item

After the #314 closeout PR merges, claim issue #315 and formalize its owning
decision and plan item before implementation.
