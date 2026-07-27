---
adr: 0014
title: Control Play and Stop with distinct pointer targets
status: Implemented
date: 2026-07-27
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, input, transport, playback]
---

# ADR 0014 — Control Play and Stop with distinct pointer targets

## Context

The header renders separate Play, Stop, and Record symbols, but application
mouse routing treats the whole left header strip as one playback toggle. The
renderer owns the exact symbol positions and clipping behavior, while existing
transport intents already own start and idempotent stop semantics.

## Capability statement

The header renderer exposes Play and Stop as distinct one-cell typed targets
matching the visible symbols. Primary clicks route through the existing start
and stop paths; Record and all other header coordinates remain inert.

## User stories / scenarios

- As a pointer user, I want Play to start pattern playback without toggling it
  off when it is already active.
- As a pointer user, I want Stop to stop playback safely even when it is already
  stopped.
- As a user, I do not want the visible Record symbol to pretend to be Play.

## Acceptance criteria

1. The visible Play symbol has a one-cell typed target at every supported
   terminal width where the symbol is rendered.
2. The visible Stop symbol has a separate one-cell typed target at every
   supported terminal width where the symbol is rendered.
3. Primary Play and Stop clicks use the existing pattern-start and stop intent
   paths respectively, including idempotent Stop behavior.
4. Record, brackets, gaps, other header coordinates, secondary clicks, drags,
   and invalid payloads do not change playback.
5. Renderer and application tests assert Play, Stop, and Record independently
   while existing keyboard transport behavior remains unchanged.

## Out of scope

- Recording, transport hover styles, and changing keyboard shortcuts.
- Making BPM, LPB, loop, MIDI-map, or other header fields interactive.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/done/2026-07-27-distinct-transport-click-targets.md`
- GitHub issue #260.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-27 | r1 | default-agent | Accepted distinct render-owned Play and Stop targets routed through existing transport intents. |
| 2026-07-27 | r2 | default-agent | Marked Implemented after PR #291 merged with GitHub Actions CI green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-27 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
