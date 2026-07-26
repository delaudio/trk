---
adr: 0009
title: Select and play sequence editor rows with the pointer
status: Implemented
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, sequence, playback]
---

# ADR 0009 — Select and play sequence editor rows with the pointer

## Context

The full-screen Sequence view scrolls around its sequence cursor and exposes
keyboard selection and playback, but its rendered rows have no pointer
targets. The composite Song Slots panel already exposes selection-only targets;
reusing that panel-specific semantic would blur the full-screen view's
secondary-click playback behavior.

## Capability statement

Every visible sequence row in the full-screen Sequence view is exposed as a
dedicated semantic interaction target carrying its absolute sequence position.
A primary click selects it, while a secondary click selects it and starts
sequence playback from that position.

## User stories / scenarios

- As a pointer user, I want to select a visible sequence position directly.
- As a pointer user, I want a secondary click to start playback from the row
  under the pointer.
- As a user with a long arrangement, I want pointer mapping to remain correct
  after the Sequence view scrolls.
- As a keyboard user, I want existing sequence editing and playback controls
  to remain unchanged.

## Acceptance criteria

1. Visible full-screen Sequence rows carry absolute sequence positions after
   scrolling and remain one rendered line at narrow widths.
2. A primary click selects the targeted position and remains in Sequence.
3. A secondary click selects the targeted position and starts sequence
   playback from it while remaining in Sequence.
4. Borders, headers, footer controls, empty rows, and drag events are no-ops.
5. Invalid positions are rejected and existing keyboard sequence actions
   remain unchanged.

## Out of scope

- Sequence insertion, removal, duplication, reordering, or context menus.
- Pointer behavior in the composite Song Slots panel or Clip Launcher.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/done/2026-07-26-sequence-editor-row-clicks.md`
- GitHub issue #255.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted dedicated full-screen Sequence row targets with selection and playback activation semantics. |
| 2026-07-26 | r2 | default-agent | Required fixed-height narrow rendering and explicit Sequence focus retention during playback activation. |
| 2026-07-26 | r3 | default-agent | Marked the capability Implemented after PR #281 merged with CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised autonomous implementation of the queued UX issues in chat. |
| Maintainer | fdg | 2026-07-26 | Authorised autonomous merge and closeout of issue #255 in chat. |
