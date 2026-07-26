---
adr: 0007
title: Select composite song slots with the pointer
status: Accepted
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, sequence]
---

# ADR 0007 — Select composite song slots with the pointer

## Context

Medium responsive tracker layouts include a Song Slots panel beside the
pattern grid. The list scrolls around the active sequence position, but
Normal/Edit mouse dispatch has no semantic targets for its rows. Deriving a
slot from pointer coordinates in the application would duplicate the
renderer-owned scrolling geometry established by ADR 0003.

## Capability statement

Every visible row in the composite Song Slots panel is exposed as a semantic
interaction target carrying its absolute sequence position. A primary-button
click selects that sequence position with the same non-playing semantics as
keyboard sequence navigation.

## User stories / scenarios

- As a pointer user, I want to select a visible song slot directly, so that I
  can navigate an arrangement from the composite pattern workspace.
- As a user with a long arrangement, I want a clicked scrolled row to select
  the slot displayed under the pointer.
- As a keyboard user, I want pointer selection to preserve existing sequence
  navigation and playback controls.

## Acceptance criteria

1. Visible composite Song Slot rows carry their absolute sequence positions
   after the centered list offset.
2. A primary-button click updates the sequence cursor to the targeted slot
   while retaining the current Pattern workspace and keyboard selection
   semantics.
3. Selection clicks do not start playback or change transport state.
4. Panel borders, empty rows, drag events, and secondary clicks do not change
   sequence selection.
5. Sequence positions are validated against the current song before mutation,
   and existing keyboard sequence navigation remains unchanged.

## Out of scope

- The dedicated full-screen Sequence view.
- Starting playback, editing sequence contents, reordering slots, drag
  gestures, or context menus.
- The Renoise-style large workspace, which has no composite Song Slots panel.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/todo/0005-composite-song-slot-clicks.md`
- GitHub issue #253.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted render-owned row targets for composite Song Slot selection. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised autonomous implementation of the queued UX issues in chat. |
