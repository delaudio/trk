---
adr: 0004
title: Select rendered pattern cells with the pointer
status: Accepted
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, tracker]
---

# ADR 0004 — Select rendered pattern cells with the pointer

## Context

ADR 0003 established a read-only interaction map whose rectangles are owned by
the renderer. Pattern-grid clicks still used fixed application coordinates,
however, so responsive layouts, viewport offsets, gutters, and side panels
could disagree with the selected row and track.

Pattern cells also need a stable semantic payload. The TUI must describe which
absolute row and track occupy a rendered rectangle without taking ownership of
application mutation, while the application must reject non-cell targets and
out-of-bounds payloads.

During implementation, the existing standard Full layout exposed a one-column
drift: headers and viewport calculations used 28 columns per track while cell
content emitted 27. Full cells therefore need one trailing separator column so
rendering, clipping, and pointer geometry share the same width.

## Capability statement

Every visible pattern cell is exposed as a semantic interaction target carrying
its absolute row and track. A primary-button click selects that cell through
application input handling, using the geometry and viewport resolved for the
rendered frame.

## User stories / scenarios

- As a pointer user, I want to click the visible tracker cell I intend to edit,
  so that selection remains accurate after resizing or scrolling.
- As a keyboard user, I want pointer support to preserve existing navigation and
  edit behaviour.
- As a contributor, I want cell rendering and hit-testing to share one width,
  so that track boundaries cannot select an adjacent cell.

## Acceptance criteria

1. Standard and Renoise-style layouts expose visible cell targets carrying the
   absolute row and track after viewport offsets.
2. A primary-button click on a cell selects its row and track; clicks on
   headers, gutters, side panels, empty grid space, and outside the grid do not
   move the cell cursor.
3. Cell target geometry is verified at 72×24, 100×28, and 140×36.
4. Full-layout cell content, headers, viewport calculations, and interaction
   regions use the same 28-column track width, including the trailing separator
   column.
5. Existing keyboard behaviour remains unchanged and responsive visual output
   is protected by updated snapshots.

## Out of scope

- Selecting an individual field or digit within a pattern cell.
- Drag selection, hover styling, double-click actions, and contextual menus.
- Migrating headers, gutters, sidebars, overlays, or wheel gestures.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/todo/0002-rendered-pattern-grid-clicks.md`
- GitHub issue #250.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted the rendered pattern-cell selection contract and documented the intentional Full-layout alignment. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised autonomous implementation of the queued UX issues in chat. |
