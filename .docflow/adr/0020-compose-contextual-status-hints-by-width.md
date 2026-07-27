---
adr: 0020
title: Compose contextual status hints by width
status: Implemented
date: 2026-07-27
owner: default-agent
supersedes:
superseded-by:
depends-on: [0019]
tags: [tui, ux, responsive, status, shortcuts]
---

# ADR 0020 — Compose contextual status hints by width

## Context

The contextual status bar currently renders each view's shortcuts as one long
string. Ratatui clips that string at the terminal boundary, which leaves
partial shortcuts and hides high-priority actions at common widths.

## Capability statement

Each main view owns a priority-ordered set of complete status segments. The
renderer composes only segments that fit the available width, preserving the
mode plus the three most relevant actions at 72 columns and progressively
adding lower-priority actions at wider widths. Command input and notifications
continue to replace shortcut hints.

## User stories / scenarios

- As a user in a 72-column terminal, I want the current mode and the three most
  useful actions for the active view to remain readable.
- As a user in a wider terminal, I want additional shortcuts without partial
  labels at the right edge.
- As a command-mode user, I want my input to take precedence over passive
  shortcut hints.
- As a user receiving feedback, I want notifications to take precedence over
  passive shortcut hints.

## Acceptance criteria

1. Every main view renders its mode and first three priority actions in full at
   72 columns.
2. Additional priority-ordered actions are appended only when each complete
   segment fits the available width.
3. Shortcut compositions never render a partial delimiter, key, or label.
4. Command input and notifications replace shortcut compositions.
5. Narrow- and medium-width snapshots cover Pattern, Sequence, Clips, Tracks,
   Patterns, Sampler, DSP Rack, Sample Browser, Project Browser, and AI Chat.

## Out of scope

- Changing shortcuts, command behavior, notification duration, or Help
  overlay content.
- Wrapping shortcut hints onto multiple lines or making the status bar
  horizontally scrollable.
- Truncating command input or notification messages into atomic segments.

## Open questions

- None.

## References

- `0019-compose-the-transport-header-by-width.md`
- `../plan/done/2026-07-27-responsive-contextual-status-bar.md`
- GitHub issue #266.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-27 | r1 | default-agent | Accepted priority-ordered atomic status segments for every main view. |
| 2026-07-27 | r2 | default-agent | Marked Implemented after PR #303 merged with GitHub Actions CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-27 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
