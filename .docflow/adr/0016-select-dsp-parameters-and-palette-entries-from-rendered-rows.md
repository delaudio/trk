---
adr: 0016
title: Select DSP parameters and palette entries from rendered rows
status: Accepted
date: 2026-07-27
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, dsp, audio, palette]
---

# ADR 0016 — Select DSP parameters and palette entries from rendered rows

## Context

DSP parameter clicks and device-palette clicks convert absolute terminal rows
with fixed first-row constants. The parameter panel moves when terminal height
changes, and the palette renders a centered window whose first absolute entry
changes with selection. The renderer already owns both layouts and their
visible ranges.

## Capability statement

The DSP renderer exposes each visible parameter row and device-palette entry as
a fixed-height typed interaction target carrying its absolute index. Pointer
input selects or activates only those payloads; application code no longer
derives DSP indices from terminal coordinates.

## User stories / scenarios

- As a pointer user, I want a parameter click to select the row I can see at
  any terminal height.
- As a pointer user, I want right-click adjustment to affect the parameter I
  clicked.
- As a pointer user, I want a scrolled palette click to assign the visible
  device type I chose.
- As a user, I do not want borders, help text, or empty panel space to trigger
  DSP actions.

## Acceptance criteria

1. Every rendered parameter row exposes a one-line target carrying its absolute
   parameter index at representative terminal heights.
2. Primary click selects the payload parameter; secondary click first selects
   it and then uses the existing positive adjustment action.
3. Every visible palette entry exposes a one-line target carrying its absolute
   device index after centered scrolling, and primary click assigns it through
   the existing palette action.
4. Parameter help rows, empty panels, palette/chain borders, blank space,
   drags, invalid payloads, and stale indices are no-ops.
5. Fixed DSP parameter and palette first-row conversion constants and helpers
   are removed.
6. Existing keyboard DSP parameter and palette navigation remains unchanged.

## Out of scope

- Parameter value drag gestures, wheel adjustment, hover styling, device
  reordering, and changes to parameter step sizes.
- Changing palette contents or keyboard shortcuts.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/todo/0014-dsp-parameter-palette-row-clicks.md`
- GitHub issue #262.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-27 | r1 | default-agent | Accepted render-owned DSP parameter and palette-row targets. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-27 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
