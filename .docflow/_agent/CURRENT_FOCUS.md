# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/318-docflow-closeout`.
- **Active item:** `.docflow/plan/done/2026-08-20-terminal-aware-audio-heatmaps.md`.
- **Blockers:** none.
- **Pending integration:** squash-merge the Docflow closeout after CI is green.

## Last shipped

Issue #318 implementation via PR #344 with CI run `32372743179` green; the
GitHub issue is closed and implementation squash `287460b` is on `main`.

## Next item

Merge this closeout, clear operational context as requested, then select issue
#319 as the next open item in priority order.

## Exit criteria

1. The completed plan lives under `plan/done/` with the implementation SHA,
   PR, green CI run, and closed issue recorded.
2. ADR 0029 and the generated index report `Implemented`.
3. The closeout passes Docflow audit, Norn review, PR CI, and squash merge.
