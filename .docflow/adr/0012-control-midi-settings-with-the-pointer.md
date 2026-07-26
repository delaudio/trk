---
adr: 0012
title: Control MIDI Settings with the pointer
status: Accepted
date: 2026-07-26
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, midi, overlay]
---

# ADR 0012 — Control MIDI Settings with the pointer

## Context

MIDI Settings renders output-port rows and connect, disconnect, panic, refresh,
and close commands as wrapped text inside one modal region. The application
cannot identify individual rows or actions, and long port lists can displace
the action hints beyond the visible overlay.

## Capability statement

The MIDI Settings renderer keeps its action controls fixed and visible, exposes
each visible output-port row with its absolute list index, and exposes each
action with a typed semantic payload. Primary clicks select ports or invoke the
same action paths as the existing keys; all other modal pointer input is inert.

## User stories / scenarios

- As a pointer user, I want to select an output port by clicking its row.
- As a pointer user, I want distinct visible controls for connect, disconnect,
  panic, refresh, and close.
- As a user with many ports, I want the selected port and actions to remain
  visible.
- As a user with no ports, I want the empty state and actions to remain safe.

## Acceptance criteria

1. Visible port rows are fixed-height targets carrying absolute port-list
   indices, and the selected row remains within the rendered viewport.
2. Connect, Disconnect, Panic, Refresh, and Close are distinct fixed action
   targets and remain visible independently of port count.
3. Primary port clicks select without connecting; primary action clicks use the
   existing keyboard action paths, including Connect on the selected port.
4. The empty state has no port targets; empty rows, overlay chrome, outside
   coordinates, secondary clicks, drags, and invalid payloads are no-ops.
5. Existing MIDI Settings keyboard navigation remains unchanged, with focused
   renderer and application tests covering empty and multiple-port states.

## Out of scope

- MIDI input-port configuration, routing-field editing, or port-list wheel
  scrolling.
- Hover styles, context menus, and changes to MIDI transport semantics.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/todo/0010-midi-settings-pointer-controls.md`
- GitHub issue #258.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-26 | r1 | default-agent | Accepted render-owned virtualized port rows and typed fixed MIDI action targets with modal pointer capture. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-26 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
