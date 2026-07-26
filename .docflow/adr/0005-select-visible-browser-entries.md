---
adr: 0005
title: Select visible browser entries with the pointer
status: Implemented
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, browser]
---

# ADR 0005 — Select visible browser entries with the pointer

## Context

The sample and project browsers scroll their rendered lists to keep the current
selection visible. Mouse handlers independently convert a terminal row to a
zero-based entry index, ignoring that rendered offset. After scrolling, a click
can therefore select or assign a different entry from the one under the
pointer. The grouped Renoise demo browser also inserts section headings that do
not correspond to entries.

ADR 0003 provides a render-owned interaction map. Browser rows can use that map
to carry absolute entry indices while application code remains responsible for
selection, preview, activation, and sample assignment.

## Capability statement

Every rendered sample or project entry row is exposed as a semantic interaction
target carrying its absolute entry index. Pointer selection and activation use
that target, so scroll offsets and non-entry rows cannot change the intended
entry.

## User stories / scenarios

- As a sample-browser user, I want a click after scrolling to select the sample
  visibly under the pointer.
- As a project-browser user, I want a click after scrolling to select the
  visible project rather than a same-row entry from the start of the list.
- As a sample-browser user, I want right-click assignment to use the entry I
  clicked, not the previous cursor.

## Acceptance criteria

1. Visible sample and project entry targets carry their absolute indices after
   non-zero viewport offsets.
2. Primary-button clicks select the targeted entry and retain existing preview
   or project activation behaviour.
3. Secondary-button clicks assign the targeted supported sample to the current
   track.
4. Panel borders, directory headers, grouped section headings, and empty list
   rows do not change selection or activate an entry.
5. Tests cover both browsers with lists longer than the viewport and non-zero
   rendered offsets.

## Out of scope

- Pointer-driven scrollbar dragging or wheel behaviour.
- Changing browser sorting, filtering, directory navigation, or preview logic.
- Redesigning the grouped Renoise demo browser.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/done/2026-07-26-visible-browser-entry-clicks.md`
- GitHub issue #251.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted render-owned entry targets for sample and project browser clicks. |
| 2026-07-26 | r2 | default-agent | Marked the capability Implemented after PR #273 merged with CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised autonomous implementation of the queued UX issues in chat. |
| Maintainer | fdg | 2026-07-26 | Authorised autonomous merge and closeout of issue #251 in chat. |
