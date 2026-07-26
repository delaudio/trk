---
adr: 0006
title: Select composite track rows with the pointer
status: Implemented
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, tracks]
---

# ADR 0006 — Select composite track rows with the pointer

## Context

Medium responsive tracker layouts include a Tracks panel beside the pattern
grid. The panel can scroll around the active track, but Normal/Edit mouse
dispatch only interprets pattern-cell targets. A visible track row therefore
cannot be selected directly, and duplicating the panel's centered-scroll
calculation in the application would recreate the geometry drift addressed by
ADR 0003.

## Capability statement

Every visible row in the composite Tracks panel is exposed as a semantic
interaction target carrying its absolute track index. A primary-button click
selects that track while preserving the current tracker row and field.

## User stories / scenarios

- As a pointer user, I want to select a visible track from the composite panel,
  so that I can move between tracks without returning to the pattern grid.
- As a user with many tracks, I want clicks on a scrolled list to select the
  track displayed under the pointer.
- As a keyboard user, I want mouse support to preserve existing tracker
  navigation state and shortcuts.

## Acceptance criteria

1. Visible composite Track rows carry their absolute track indices after the
   centered list offset.
2. A primary-button click selects the targeted track without changing the
   current pattern row, cell field, or edit digit.
3. Panel borders and empty rows do not change the active track.
4. Track indices are validated against the current song before mutation.
5. Existing keyboard track navigation remains unchanged.

## Out of scope

- The dedicated full-screen Tracks view.
- Track reordering, mute/solo controls, drag gestures, or context menus.
- The Renoise-style large workspace, which has no composite Tracks panel.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/done/2026-07-26-composite-track-row-clicks.md`
- GitHub issue #252.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted render-owned track-row targets for the composite tracker layout. |
| 2026-07-26 | r2 | default-agent | Marked the capability Implemented after PR #275 merged with CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised autonomous implementation of the queued UX issues in chat. |
| Maintainer | fdg | 2026-07-26 | Authorised autonomous merge and closeout of issue #252 in chat. |
