---
adr: 0030
title: Edit patterns through synchronized piano rolls
status: Accepted
date: 2026-08-20
owner: default-agent
supersedes:
superseded-by:
depends-on: [0001, 0003, 0028]
tags: [piano-roll, patterns, tui, web, editing, midi, automation]
---

# ADR 0030 — Edit patterns through synchronized piano rolls

## Context

The tracker grid is compact and precise for row events, instruments, effects,
and parameter locks, but it makes pitch contours, chord voicings, and note
lengths difficult to inspect horizontally. `PatternCell` already persists an
optional `gate` value, yet the tracker does not expose that field and playback
does not schedule a note release from it. The current Web companion calls its
active-pattern visualization a piano roll, but places tracks rather than
chromatic pitch on the vertical axis and deliberately cannot edit cells.

Issue #319 requires TUI and browser piano rolls that synchronize immediately
with the canonical pattern matrix. A second note store would make tracker,
persistence, playback, undo, and browser state diverge. The capability must
therefore project and mutate `PatternCell` data directly, define one bounded
gate interpretation, preserve collision safety and undo, and extend the
existing loopback action boundary rather than allowing the HTTP thread to
borrow live application state.

The issue also requests browser automation curves. The existing pattern
automation model only targets sample gain, although the MIDI layer can already
emit controller messages. A piano-roll automation lane needs a stable MIDI CC
target and deterministic row event mapping; it must not invent a browser-only
curve or an unpersisted playback path.

## Capability statement

`trk` provides synchronized horizontal piano-roll editors in the terminal and
local Web companion, backed solely by pattern cells and automation lanes, with
lossless gate lengths, velocity and collision-safe note edits, optional
multi-track ghost overlays, and bounded MIDI CC curve editing and playback.

## User stories / scenarios

- As a composer, I want to see pitch vertically and time horizontally, so that
  melodic contours, chord voicings, and note lengths are readable at a glance.
- As a keyboard-first user, I want to insert, delete, move, resize, and change
  velocity without leaving the terminal, so that the alternate view remains a
  complete pattern editor rather than a passive visualization.
- As an arranger, I want other tracks as optional ghost notes, so that I can
  write counter-lines against the active harmony without mutating references.
- As a local Web companion user, I want direct-manipulation note and MIDI CC
  curve edits to enter the same undoable application boundary, so that browser
  and tracker state cannot drift.

## Acceptance criteria

1. `:view roll` enters a dedicated Piano Roll mode for the active pattern and
   track; `Esc` and `:view tracker` return without changing musical data.
   Existing `Tab` track navigation and `F1`/`F2` octave controls remain
   unchanged in tracker modes. The roll renders a chromatic pitch ruler with
   distinct white/black keys, 16/32/64-row horizontal zoom levels, current
   row/pitch cursor, and a live playhead at representative terminal sizes.
2. Pattern cells remain the sole note source. For a note onset, `gate: Some(n)`
   means an explicit duration of `n.max(1)` rows, bounded to `1..=127` and the
   pattern end; `gate: None` preserves legacy sustain until the next note,
   NoteOff/NoteCut, or pattern end. Gate-aware MIDI scheduling starts duration
   after any cell delay and releases no later than a replacing or explicit
   termination event. The tracker Full field layout displays and edits `Gxx`
   so switching views exposes the same note, velocity, and gate values.
3. In Piano Roll mode, arrows move the pitch/time cursor, Space inserts or
   removes the exact cursor note, Shift+Left/Right shrinks or expands its gate,
   digits 1–9 set velocity to `10,20,30,40,50,60,70,80,100%` respectively,
   Alt+arrows move the
   complete source cell by time or pitch, `g` toggles ghost tracks, and
   `[`/`]` select 16/32/64-row zoom. Every edit is one undoable transaction,
   stays within pitch/pattern bounds, refuses an occupied destination without
   data loss, and keeps the canonical tracker cursor synchronized.
4. The TUI roll draws the active track as velocity-scaled gate bars without
   hiding onset or cursor cells. When ghosting is enabled, every non-active
   track note in the visible pitch/time window appears with a distinct dim
   style and never affects selection or mutation. Empty, narrow, low-height,
   first/last-row, and overlapping cross-track inputs render without panic or
   out-of-bounds interaction regions.
5. Web state projects bounded note onset, pitch, track, velocity, and resolved
   gate duration plus MIDI CC automation lanes for the active pattern. The
   self-contained Canvas uses pitch on the vertical axis and time on the
   horizontal axis, distinguishes the selected track from ghost tracks,
   renders gate-width note bars and a playhead, and draws controller curves in
   a separate lane without exposing project paths or a parallel data model.
6. Browser pointer editing can create/select notes, drag a note body to move
   time or pitch, drag its end to resize, delete the selected note, change its
   velocity, and create/update/remove normalized MIDI CC points. Strict edit
   actions include the observed state revision and complete source/target
   coordinates; the loopback server rejects stale, malformed, oversized, or
   out-of-range requests before bounded delivery, and the TUI thread rechecks
   targets and applies the same collision-safe undoable mutations as keyboard
   editing. Existing transport, pattern, mute, and solo actions remain valid.
7. `AutomationTarget::MidiCc` persists a track identity and controller in
   `0..=127`; its finite normalized points stay within pattern rows and
   `0.0..=1.0`. Playback emits deterministic row-ordered MIDI Control Change
   events on the target track channel, respects track audibility and MIDI
   routing filters, and remains backward-compatible with sample-gain lanes and
   projects that omit gates or MIDI CC automation.
8. Unit tests cover gate resolution/scheduling, MIDI CC validation/playback,
   collision-safe note mutations, cursor controls, velocity/zoom/ghost state,
   Web projection and stale-action rejection. Buffer assertions and snapshots
   cover populated/empty/narrow Piano Roll rendering, and a loopback browser
   smoke test proves note and CC edits round-trip into canonical pattern state.
   The complete repository verification gate passes.

## Out of scope

- More than one simultaneous note in the same track and row; chords continue
  to use separate tracks until a polyphonic cell model is decided elsewhere.
- Freehand sub-row curves, interpolation beyond the existing stepped lane
  model, pitch bend, aftertouch, NRPN, or arbitrary controller learn mappings.
- Replacing the tracker grid, changing existing tracker-mode Tab/F-key
  bindings, or making Piano Roll the startup view.
- LAN access, a public HTTP API, WebSocket transport, external browser assets,
  or browser-side direct access to project persistence.
- The tracker-grid-wide ghost-note shortcut planned separately from this
  Piano Roll-specific overlay.

## Open questions

- None.

## References

- `0001-record-architecture-decisions.md`
- `0003-render-owned-interaction-regions.md`
- `0028-expose-a-local-web-companion.md`
- `../plan/todo/0028-synchronized-piano-roll-editing.md`
- GitHub issue #319.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-08-20 | r1 | default-agent | Recorded and accepted synchronized pattern-cell piano rolls, explicit gate scheduling, ghost overlays, safe Web edits, and MIDI CC curves. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-08-20 | Approved autonomous resolution of the prioritized GitHub issue queue in chat. |
