---
adr: 0019
title: Compose the transport header by width
status: Accepted
date: 2026-07-27
owner: default-agent
supersedes:
superseded-by:
depends-on: [0014]
tags: [tui, ux, responsive, transport]
---

# ADR 0019 — Compose the transport header by width

## Context

The transport header is currently rendered as one long line. Narrow terminals
clip that line at an arbitrary cell, leaving partial labels such as `Sync: Int`
or `STO` while hiding the playback state, pattern, and row that users need to
orient themselves.

## Capability statement

The transport header selects an explicit composition for the available inner
width. Each composition renders whole priority-ordered segments, always keeps
Play, Stop, BPM, LPB, playback state, pattern, and row at supported widths,
adds synchronization status at medium widths, and exposes the complete status
set at large widths.

## User stories / scenarios

- As a user in a 72-column terminal, I want the core transport and position
  state to remain readable.
- As a user in a 100-column terminal, I want synchronization status without a
  clipped label.
- As a user in a 140-column terminal, I want the complete transport header.
- As a pointer user, I want Play and Stop targets to continue matching their
  rendered symbols in every composition.

## Acceptance criteria

1. At 72 and 80 columns the header renders whole Play, Stop, BPM, LPB,
   playback-state, pattern, and row segments.
2. At 100 columns the header also renders a whole Sync segment.
3. At 140 columns the complete transport header remains available.
4. Optional information is omitted as an entire segment; no selected
   composition exceeds its available inner width.
5. Play and Stop expose distinct exact render-owned targets in every
   composition.
6. Focused rendering tests and snapshots cover 72×24, 80×24, 100×28, and
   140×36.

## Out of scope

- Changing transport behavior, playback state, MIDI routing, or status values.
- Wrapping the header onto multiple lines or making omitted segments
  horizontally scrollable.

## Open questions

- None.

## References

- `0014-control-play-and-stop-with-distinct-pointer-targets.md`
- `../plan/todo/0017-responsive-transport-header.md`
- GitHub issue #265.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-27 | r1 | default-agent | Accepted width-aware atomic transport-header compositions. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-27 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
