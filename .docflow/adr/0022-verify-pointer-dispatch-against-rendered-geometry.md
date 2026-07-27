---
adr: 0022
title: Verify pointer dispatch against rendered geometry
status: Accepted
date: 2026-07-27
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, mouse, testing, responsive]
---

# ADR 0022 — Verify pointer dispatch against rendered geometry

## Context

Application mouse tests commonly register synthetic interaction regions at
fixed coordinates, while renderer tests inspect generated regions without
dispatching an input event. Either layer can therefore change independently
and leave a mismatch between visible controls and application behavior
undetected, especially across responsive terminal sizes and scrolled lists.

## Capability statement

Mouse regression tests render the real application state into a test terminal,
derive click coordinates from the resulting interaction map, dispatch events
through the application handler, and verify both the intended action and
inert behavior immediately outside each target. The same contracts run at the
supported compact, medium, and large terminal sizes.

## User stories / scenarios

- As a mouse user, I want a visible control to activate at every supported
  terminal size.
- As a user of a scrolled list, I want the clicked visible row to address its
  absolute item rather than a stale screen-relative index.
- As a maintainer, I want CI to fail when rendered geometry and pointer
  dispatch diverge.

## Acceptance criteria

1. Integration tests render and dispatch at 72×24, 80×24, 100×28, and 140×36.
2. Pattern cells, composite panels, browsers, overlays, DSP controls, and
   sampler controls are represented across the cross-size matrix.
3. Every scrollable list covered by the matrix includes a case with a non-zero
   visible start offset.
4. Each tested target verifies that a click immediately outside its rendered
   region is inert.
5. Click coordinates come from the renderer's interaction map rather than
   duplicated layout constants.

## Out of scope

- Replacing focused unit tests for individual mouse actions.
- Pixel or color snapshot coverage.
- Mouse support for controls that do not yet expose interaction regions.
- End-to-end tests against a real terminal emulator.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/todo/0020-cross-size-mouse-regressions.md`
- GitHub issue #268.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-27 | r1 | default-agent | Accepted render-derived, cross-size pointer regression contracts. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-27 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
