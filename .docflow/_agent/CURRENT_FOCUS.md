# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/316-docflow-closeout`.
- **Active item:** closeout of
  `.docflow/plan/done/2026-08-20-external-editor-hot-reload.md`.
- **Blockers:** none.
- **Pending integration:** merge the documentation closeout through CI, then
  clear operational context before selecting the next open issue.

## Last shipped

Issue #316 implementation via PR #340 with CI green; ADR 0027 implementation
squash is `ffa18b0` and the GitHub issue is closed.

## Next item

After the closeout merges, begin a fresh-context session, reread the repository
handoff and live GitHub issue queue, and select the highest-priority open issue.
