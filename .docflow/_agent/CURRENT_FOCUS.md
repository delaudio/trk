# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/315-dsp-calibration`.
- **Active item:** `.docflow/plan/todo/0024-dsp-calibration-modal.md` for
  GitHub issue #315.
- **Blockers:** none.
- **Pending integration:** implement callback-safe audible calibration and live
  metering plus the `t` modal, pass the full gate and Norn Codex review, then
  merge through CI.

## Last shipped

Issue #314 implementation via PR #336 and Docflow closeout via PR #337, both
with CI green; ADR 0025 is Implemented.

## Next item

Implement ADR 0026 acceptance criteria and integrate issue #315 before
selecting issue #316.
