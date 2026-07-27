---
adr: 0013
title: Activate confirmation dialog actions with the pointer
status: Implemented
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, dialog, overlay]
---

# ADR 0013 — Activate confirmation dialog actions with the pointer

## Context

Quit and destructive confirmation dialogs render button-like choices but expose
only their whole modal rectangles. Keyboard handling already owns all save,
quit, delete, open-project, and cancel semantics; duplicating those operations
in mouse routing would risk behavioral drift.

## Capability statement

Confirmation renderers expose every visible choice as a typed semantic target.
A primary click maps that target to the same dialog-choice key path already
used by keyboard input, while the modal captures all other pointer clicks as
no-ops.

## User stories / scenarios

- As a pointer user, I want to Save, Don't Save, or Cancel a dirty quit.
- As a pointer user, I want to Confirm or Cancel a destructive action.
- As a user, I want clicks outside a confirmation to leave it open and avoid
  changing the project.

## Acceptance criteria

1. Dirty-quit Save, Don't Save, and Cancel choices have distinct fixed-height
   targets matching their visible labels.
2. Confirm and Cancel choices in non-quit confirmation dialogs have distinct
   fixed-height targets matching their visible labels.
3. Primary action clicks route through the equivalent existing dialog key path,
   preserving save, force-quit, delete, open-project, and cancel behavior.
4. Dialog text, borders, gaps, outside coordinates, secondary clicks, drags,
   and invalid payloads are no-ops.
5. Existing keyboard dialog behavior remains unchanged, with focused renderer
   and application tests for quit and destructive confirmations.

## Out of scope

- Hover styles, default-button focus, double-click behavior, or new dialogs.
- Changing save, delete, open-project, or quit semantics.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/done/2026-07-27-confirmation-dialog-pointer-actions.md`
- GitHub issue #259.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted typed render-owned confirmation targets routed through existing dialog key choices. |
| 2026-07-27 | r2 | default-agent | Marked Implemented after PR #289 merged with GitHub Actions CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
