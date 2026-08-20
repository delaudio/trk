# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/317-docflow-closeout`.
- **Active item:** `.docflow/plan/done/2026-08-20-local-web-companion.md`.
- **Blockers:** none.
- **Pending integration:** commit, review, and squash-merge the Docflow closeout
  for the already shipped issue #317 implementation.

## Last shipped

Issue #317 implementation via PR #342 with CI run `32368367097` green;
implementation squash `9e8ba14` is on `main` and the GitHub issue is closed.

## Next item

Merge this closeout, then clear operational context before inspecting or
starting issue #318.

## Exit criteria

1. Move plan 0026 to `plan/done` with the implementation SHA, PR, CI run, and
   closed issue recorded.
2. Advance ADR 0028 to Implemented, point it at the shipped plan, and regenerate
   the INDEX entry.
3. Append the durable closeout worklog and keep this live snapshot accurate.
4. Pass Docflow audit, Norn Codex review, and the closeout PR CI before squash
   merge.
