---
adr: 0011
title: Control the Help overlay with the pointer
status: Implemented
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, help, overlay]
---

# ADR 0011 — Control the Help overlay with the pointer

## Context

The Help overlay renders tab labels and scrollable content, but its interaction
map exposes only the whole modal rectangle. The application therefore cannot
distinguish tabs, content, or a close action. Wheel input also changes Help
scroll whenever Help mode is active, including outside the rendered content.

## Capability statement

The Help renderer exposes each visible tab, the scrollable content area, and an
explicit close control as semantic interaction targets. Primary clicks switch
tabs or close Help, wheel input scrolls only over the content target, and Help
mode captures all other pointer clicks as no-ops.

## User stories / scenarios

- As a pointer user, I want to open any Help page by clicking its tab.
- As a pointer user, I want to scroll Help only while the pointer is over its
  content.
- As a pointer user, I want a visible close control that returns me to the view
  from which I opened Help.
- As a user, I want clicks outside the modal to leave both Help and the
  underlying workspace unchanged.

## Acceptance criteria

1. Basics, Editing, Sampler, MIDI, and Commands have distinct fixed-height
   rendered targets carrying stable tab indices.
2. A primary click on a tab activates that page and resets its scroll offset.
3. The visible close control has its own target and releases the existing Help
   focus capture through the same path as keyboard close.
4. Wheel events change Help scroll only when their coordinates hit the rendered
   content target.
5. Overlay borders, hints, outside coordinates, secondary clicks, drags, and
   invalid payloads are no-ops; existing keyboard Help navigation is unchanged.

## Out of scope

- Hover styling, drag scrolling, clickable help commands, or scrollbars.
- Mouse behavior in command palette, MIDI settings, or confirmation overlays.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/done/2026-07-26-help-overlay-pointer-controls.md`
- GitHub issue #257.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted render-owned Help tab, content, and close targets with modal pointer capture and scoped scrolling. |
| 2026-07-26 | r2 | default-agent | Marked Implemented after PR #285 merged with GitHub Actions CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
