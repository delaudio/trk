---
adr: 0003
title: Expose render-owned interaction regions
status: Implemented
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: []
tags: [tui, ux, input, architecture]
---

# ADR 0003 — Expose render-owned interaction regions

## Context

Salieri renders responsive terminal layouts at several width and height
breakpoints. Mouse input is currently interpreted by application handlers that
duplicate parts of that geometry as fixed terminal coordinates. The duplicated
values can drift from the rectangles that Ratatui actually renders, causing
visible controls to miss clicks or unrelated controls to receive them.

The TUI crate owns layout calculation and must remain independent from
application mutation logic. The application crate owns input dispatch and must
be able to translate a pointer event into the same intent used by the keyboard.
These constraints require a small read-only contract between rendering and
input handling.

## Capability statement

Each rendered frame exposes an ordered map of semantic interaction regions whose
rectangles come from the same calculations used to draw that frame. The TUI
describes what was rendered and where; the application decides what a semantic
region does. More specific regions take precedence over containing regions so
overlays, items, and controls can safely refine a larger panel.

## User stories / scenarios

- As a pointer user, I want clicks to follow the visible layout after resizing,
  so that the selected control is the one under the pointer.
- As a TUI contributor, I want rendering and hit-testing to share geometry, so
  that new responsive layouts do not require duplicated coordinate constants.
- As an application contributor, I want semantic region identifiers rather
  than renderer callbacks, so that input still flows through typed application
  actions.

## Acceptance criteria

1. A rendered frame can expose zero or more semantic interaction regions with
   stable identifiers and Ratatui rectangles.
2. Region rectangles are derived from the layout calculations used for the same
   frame, including responsive managed panels.
3. The application can retain the most recently rendered interaction map
   without moving mutation logic into the TUI crate.
4. Hit-testing prefers the most recently registered matching region, allowing
   a specific control or overlay to override a containing panel.
5. Tests verify representative region bounds at 72×24, 100×28, and 140×36
   without changing existing visual snapshots or keyboard behaviour.

## Out of scope

- Migrating every existing mouse handler in the same change.
- Defining application commands or mutations inside `salieri-tui`.
- Hover styling, drag gestures, double-click timing, or contextual menus.
- Implementing currently decorative controls.

## Open questions

- None.

## References

- GitHub issue #249.
- `../../crates/salieri-tui/src/render.rs`
- `../../crates/salieri-tui/src/layout.rs`
- `../../crates/salieri-app/src/app/key_handling.rs`

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted the render-owned interaction-region contract for issue #249. |
| 2026-07-26 | r2 | default-agent | Marked the capability Implemented after PR #269 merged with CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised implementation of the queued UX issues in chat. |
| Maintainer | fdg | 2026-07-26 | Authorised autonomous merge and closeout of issue #249 in chat. |
