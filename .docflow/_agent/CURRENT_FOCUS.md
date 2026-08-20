# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/315-docflow-closeout`.
- **Active item:** close out `.docflow/plan/done/2026-08-20-dsp-calibration-modal.md`
  after GitHub issue #315 shipped.
- **Blockers:** none.
- **Pending integration:** merge the documentation-only closeout through CI.

## Last shipped

Issue #315 implementation via PR #338 with CI green; ADR 0026 is implemented
and its Docflow closeout is pending integration.

## Next item

Inspect and claim GitHub issue #316 after the #315 closeout merges.
