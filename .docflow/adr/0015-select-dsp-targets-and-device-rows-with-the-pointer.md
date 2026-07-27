---
adr: 0015
title: Select DSP targets and device rows with the pointer
status: Accepted
date: 2026-07-27
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, dsp, audio]
---

# ADR 0015 — Select DSP targets and device rows with the pointer

## Context

The DSP rack renders separate Track and Master chains plus their visible device
rows, but its mouse path only converts fixed terminal rows into parameter
indices. The renderer owns the actual responsive chain geometry, while the
application owns target, device, and parameter selection state.

## Capability statement

The DSP rack renderer exposes Track and Master controls and each rendered
device row as typed interaction targets. A primary click selects the named
chain or device through application state; selecting a device also bounds the
parameter cursor so the parameter panel immediately reflects that device.

## User stories / scenarios

- As a pointer user, I want to select Track or Master without switching to the
  keyboard.
- As a pointer user, I want to select a visible DSP device and immediately see
  its editable parameters.
- As a user, I do not want empty chain space to change my current selection.

## Acceptance criteria

1. Each rendered Track and Master control exposes an exact typed target and a
   primary click selects that target.
2. Every visible device row exposes an exact typed target carrying both its
   chain and absolute device index.
3. Selecting a device switches to its chain, selects the device, and bounds the
   parameter cursor for the newly selected device.
4. Empty rows, chain borders, secondary clicks, drags, and invalid or stale
   payloads do not change the selected DSP target or device.
5. Renderer geometry tests cover populated, empty, clipped, and representative
   responsive layouts; application tests cover routing and parameter-panel
   refresh.
6. Existing keyboard target, device, and parameter navigation remains
   unchanged.

## Out of scope

- Device palette interaction, device reordering, drag gestures, hover styles,
  and changing parameter adjustment behavior.
- Selecting hidden devices that are not rendered in the current terminal.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/todo/0013-dsp-target-device-row-clicks.md`
- GitHub issue #261.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-27 | r1 | default-agent | Accepted render-owned DSP target and device-row selection. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-27 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
