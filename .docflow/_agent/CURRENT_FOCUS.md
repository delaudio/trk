# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/319-piano-roll`.
- **Active item:** `.docflow/plan/todo/0028-synchronized-piano-roll-editing.md`.
- **Blockers:** none.
- **Pending integration:** publish the accepted decision and plan in a draft
  PR, then implement, review, verify, and squash-merge issue #319.

## Last shipped

Issue #318 implementation and Docflow closeout merged through PRs #344 and
#345 with CI green; ADR 0029 is Implemented and the GitHub issue is closed.

## Next item

Implement ADR 0030 through the issue #319 PR, close its Docflow state after the
implementation merge, then clear operational context before the next issue.

## Exit criteria

1. Canonical gate rows and MIDI CC automation persist and schedule without
   breaking legacy projects.
2. TUI Piano Roll rendering and keyboard edits round-trip through pattern
   cells with undo and collision safety.
3. Web Canvas projection and revision-bound note/CC edits round-trip through
   the bounded TUI-thread action bridge.
4. Ghost overlays, responsive snapshots, loopback smoke coverage, repository
   gate, Docflow audit, and Norn Codex review pass before publication.
