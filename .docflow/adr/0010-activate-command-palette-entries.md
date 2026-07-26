---
adr: 0010
title: Select and activate command palette entries with the pointer
status: Implemented
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, command-palette, overlay]
---

# ADR 0010 — Select and activate command palette entries with the pointer

## Context

The command palette is a centered, scrolling overlay, but pointer input cannot
select or execute its visible entries. Mapping coordinates in the application
would duplicate the renderer's centered overlay geometry and visible start
offset. Existing wheel handling also moves palette selection without checking
whether the pointer is over the result list.

## Capability statement

The command palette renderer exposes its result-list area and every visible
entry as semantic interaction targets. Entry targets carry absolute result
indices. A primary click selects the entry and executes it only when enabled;
wheel events move selection only while the pointer is over the result list.

## User stories / scenarios

- As a pointer user, I want to click an enabled palette result to execute it.
- As a pointer user, I want a disabled result to become selected without
  closing the palette or executing an action.
- As a user with many results, I want clicks to select the visible entry after
  scrolling.
- As a pointer user, I want wheel navigation limited to the palette results,
  so scrolling elsewhere is harmless.

## Acceptance criteria

1. Visible palette entries carry absolute result indices after the renderer's
   current visible start offset.
2. A primary click selects an enabled entry and invokes the same execution
   path as Enter.
3. A primary click selects a disabled entry, leaves the palette open, and does
   not execute it.
4. Wheel events over the result-list target move and clamp selection; wheel
   events elsewhere do not change palette selection.
5. Entry rows remain one rendered line at narrow widths, while overlay
   borders, non-entry rows, empty results, secondary clicks, and drags are
   no-ops.

## Out of scope

- Query text editing via pointer, hover states, context menus, or scrollbars.
- Pointer behavior in the DSP device palette or other overlays.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/done/2026-07-26-command-palette-entry-clicks.md`
- GitHub issue #256.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted render-owned palette result geometry, absolute entry targets, scoped wheel navigation, and primary-click execution semantics. |
| 2026-07-26 | r2 | default-agent | Marked Implemented after PR #283 merged with GitHub Actions CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised autonomous implementation of the queued UX issues in chat. |
| Maintainer | fdg | 2026-07-26 | Authorised autonomous merge and closeout of GitHub issue #256 in chat. |
