# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** agent/249-interaction-regions
- **Active item:** `plan/todo/0001-render-owned-interaction-regions.md` for
  GitHub issue #249.
- **Blockers:** the required Lachesi review cannot initialize inside the
  sandbox; unsandboxed execution is pending explicit authorization for code
  egress to a named review provider.
- **Uncommitted work:** issue #249 implementation and multi-size interaction
  region tests; the full local repository gate passes.

## Last shipped

Docflow bootstrap commit `8be2907`.

## Next item

Review and publish `plan/todo/0001-render-owned-interaction-regions.md`, then
start issue #250.
