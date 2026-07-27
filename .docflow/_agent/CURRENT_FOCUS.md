# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/rename-trk`
- **Active item:** propose the hard-cutover product and repository rename to
  `trk` in ADR 0023. No implementation starts until the ADR is Accepted and a
  queue item is created.
- **Blockers:** none.
- **Pending integration:** ADR proposal plus the previously verified README and
  interoperability updates are uncommitted.

## Last shipped

Issue #268 implementation and Docflow closeout via PRs #307 and #309.

## Next item

Accept ADR 0023, queue the rename, then execute the verified hard cutover.
