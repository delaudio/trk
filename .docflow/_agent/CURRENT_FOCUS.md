# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/313-ai-engine-selector`.
- **Active item:** `.docflow/plan/todo/0022-runtime-ai-engine-selection.md`
  for GitHub issue #313.
- **Blockers:** none.
- **Pending integration:** implementation is complete with the full local gate
  green; Norn Codex review and PR #334 integration remain.

## Last shipped

Issue #309 implementation via PR #310 and Docflow closeout via PR #311, both
with CI green; ADR 0023 is Implemented.

## Next item

Implement ADR 0024 acceptance criteria, run the full gate and Norn Codex review,
then integrate issue #313 before selecting issue #314.
