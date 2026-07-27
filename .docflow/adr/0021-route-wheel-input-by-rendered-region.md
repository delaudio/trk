---
adr: 0021
title: Route wheel input by rendered region
status: Accepted
date: 2026-07-27
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, mouse, scrolling]
---

# ADR 0021 — Route wheel input by rendered region

## Context

Mouse-wheel behavior is currently selected from the application mode. A
composite tracker workspace contains several independently meaningful panels,
so the same wheel event can move the pattern cursor even when the pointer is
over the track list, sequence list, inspector, header, or another
non-scrollable region.

## Capability statement

The render-owned interaction map classifies semantic scroll targets. Wheel
handling hit-tests the pointer coordinates, routes vertical input only to the
hovered scroll target, and routes horizontal input only to targets that
support a horizontal axis. Modal overlays capture wheel input and prevent
fall-through to covered workspace regions.

## User stories / scenarios

- As a tracker user, I want the wheel over the pattern grid to move pattern
  rows.
- As a composite-layout user, I want the wheel over track and sequence lists
  to move those lists without changing the pattern row.
- As an overlay user, I do not want wheel input outside overlay content to
  mutate the covered workspace.
- As a horizontal-wheel user, I want track or waveform panning only where a
  horizontal axis is supported.

## Acceptance criteria

1. Pattern cells and rendered pattern-row content route vertical wheel input
   to pattern rows; the panel border, header, and empty padding are no-ops.
2. Track, sequence, pattern-manager, browser, DSP, clip, and MIDI list regions
   route vertical wheel input to their own bounded selections.
3. Help, command palette, MIDI settings, and confirmation overlays capture
   wheel input and never fall through to covered workspace regions.
4. Header, status, inspector, and other non-scrollable regions are no-ops.
5. Horizontal wheel input affects only the pattern grid, clip grid, and loaded
   sampler waveform.
6. Focused interaction-map and application tests cover routing, overlay
   capture, non-scrollable no-ops, and horizontal restrictions.

## Out of scope

- Adding scroll state to panels that are not currently scrollable.
- Changing keyboard navigation increments or wheel sensitivity.
- Smooth scrolling, inertial scrolling, touch gestures, or hover styling.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/todo/0019-hovered-wheel-routing.md`
- GitHub issue #249 (closed).
- GitHub issue #267.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-27 | r1 | default-agent | Accepted render-owned semantic scroll targets and coordinate-based wheel routing. |
| 2026-07-27 | r2 | default-agent | Restricted wheel targets to rendered content rows and explicitly excluded chrome and empty padding. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-27 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
