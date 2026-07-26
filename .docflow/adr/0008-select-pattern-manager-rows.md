---
adr: 0008
title: Select pattern manager rows with the pointer
status: Implemented
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, patterns]
---

# ADR 0008 — Select pattern manager rows with the pointer

## Context

The full-screen Patterns view scrolls around the selected pattern but exposes
only keyboard navigation. Reconstructing the visible row offset in application
mouse handling would duplicate renderer-owned geometry and risk selecting a
different pattern from the one displayed under the pointer.

## Capability statement

Every visible pattern row in the full-screen Patterns view is exposed as a
semantic interaction target carrying its absolute pattern index. A primary
click selects it, while a secondary click selects it and returns to the
tracker editor.

## User stories / scenarios

- As a pointer user, I want to select a visible pattern row directly.
- As a pointer user, I want a secondary click to open the selected pattern in
  the tracker.
- As a keyboard user, I want existing navigation and mutations to remain
  unchanged.

## Acceptance criteria

1. Visible pattern rows carry absolute pattern indices after scrolling.
2. A primary click selects the targeted pattern and remains in Patterns.
3. A secondary click selects the targeted pattern and opens the tracker.
4. Borders, headers, footer controls, empty rows, and drag events are no-ops.
5. Invalid indices are rejected and existing keyboard pattern actions remain
   unchanged.

## Out of scope

- Pattern creation, mutation, deletion, reordering, or context menus.
- Pointer interaction in the composite pattern grid or Renoise sidebar.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/done/2026-07-26-pattern-manager-row-clicks.md`
- GitHub issue #254.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted render-owned pattern manager row targets and pointer activation semantics. |
| 2026-07-26 | r2 | default-agent | Marked the capability Implemented after PR #279 merged with CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised autonomous implementation of the queued UX issues in chat. |
| Maintainer | fdg | 2026-07-26 | Authorised autonomous merge and closeout of issue #254 in chat. |
