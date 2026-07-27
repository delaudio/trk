# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/rename-trk`
- **Active item:** implement plan item
  `plan/todo/0021-rename-product-and-repository-to-trk.md` under Accepted ADR
  0023.
- **Blockers:** none.
- **Pending integration:** verified hard-cutover implementation is ready for
  commit and PR integration in `delaudio/trk`.

## Last shipped

Issue #268 implementation and Docflow closeout via PRs #307 and #309.

## Next item

Commit and push the verified implementation, open the PR, wait for CI, and
squash-merge before closing issue #309 and completing Docflow closeout.
