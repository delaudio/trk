---
adr: 0017
title: Control supported sampler actions with the pointer
status: Implemented
date: 2026-07-27
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, sampler, waveform]
---

# ADR 0017 — Control supported sampler actions with the pointer

## Context

The sampler already supports ADSR field selection and adjustment, waveform
zoom and pan, and opening the sample browser from the keyboard. Its pointer
path instead relies on broad fixed coordinates, does not expose those actions,
and behaves differently between the compact and large Renoise-inspired
layouts.

## Capability statement

Both sampler layouts render fixed-height, typed pointer targets for each ADSR
field and for visible decrement, increment, zoom, pan, and Browse controls.
Application routing dispatches those payloads through the same methods used by
the existing keyboard bindings.

## User stories / scenarios

- As a pointer user, I want to select a specific envelope field and adjust it
  without cycling through the other fields.
- As a pointer user, I want visible waveform zoom and pan controls whose labels
  describe the action they perform.
- As a pointer user, I want a precise Browse control instead of an undocumented
  broad click area.
- As a user, I do not want sampler borders, metadata, waveform content, or
  empty space to trigger actions.

## Acceptance criteria

1. Every rendered ADSR field exposes an exact typed target carrying its field,
   and primary click selects that field without editing its value.
2. Visible decrement and increment controls call the existing fine envelope
   adjustment method for the currently selected field.
3. Visible zoom-out, zoom-in, pan-left, and pan-right controls call the existing
   waveform methods.
4. A visible Browse control opens the in-app sample browser through the
   existing application method, including when no sample is loaded.
5. Compact and large sampler layouts expose the same semantic actions at
   representative supported dimensions.
6. Only primary button-down events activate controls; secondary clicks, drags,
   stale or mismatched payloads, borders, help text, waveform content, and
   empty space are no-ops.
7. Existing sampler keyboard navigation and adjustment behavior remains
   unchanged.

## Out of scope

- Drag-to-edit envelopes, waveform scrubbing, wheel behavior, hover styling,
  coarse envelope controls, and new sample-processing operations.
- Changing sampler value ranges, step sizes, or browser contents.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/done/2026-07-27-sampler-direct-pointer-controls.md`
- GitHub issue #263.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-27 | r1 | default-agent | Accepted typed direct controls for supported sampler actions. |
| 2026-07-27 | r2 | default-agent | Marked Implemented after PR #297 merged with GitHub Actions CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-27 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
