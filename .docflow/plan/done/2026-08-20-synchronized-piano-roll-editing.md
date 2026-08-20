# Synchronized Piano Roll editing

Owning ADR: `../../adr/0030-edit-patterns-through-synchronized-piano-rolls.md`

GitHub issue: #319

## Scope

Implement one canonical pattern-cell Piano Roll mode in the TUI and upgrade the
local Web companion to the same pitch/time projection and bounded undoable edit
boundary. Define visible/editable gate rows and MIDI scheduling, add collision-
safe keyboard and browser note operations, ghost-track overlays, persisted MIDI
CC automation curves and playback, responsive rendering, documentation, and
deterministic coverage. Keep same-row single-note-per-track semantics, existing
tracker bindings, loopback security/lifecycle, and stepped automation.

## Exit criteria

1. View routing, responsive pitch/time layout, zoom, cursor, and playhead meet
   ADR AC1.
2. Canonical gate semantics, tracker-field parity, and MIDI release scheduling
   meet ADR AC2.
3. Undoable insert/delete/move/resize/velocity controls and collision handling
   meet ADR AC3.
4. Active/ghost track TUI rendering and bounded interaction geometry meet ADR
   AC4.
5. Web state, Canvas note bars, ghost tracks, playhead, and CC curve projection
   meet ADR AC5.
6. Revision-bound strict Web note/velocity/gate/CC actions and TUI-thread
   application meet ADR AC6.
7. Persisted MIDI CC targets, validation, scheduling, routing, and backward
   compatibility meet ADR AC7.
8. Unit, buffer, snapshot, loopback round-trip, the complete repository
   verification gate, and Norn review satisfy ADR AC8.

## Dependencies

- `../../adr/0001-record-architecture-decisions.md`
- `../../adr/0003-render-owned-interaction-regions.md`
- `../../adr/0028-expose-a-local-web-companion.md`
- Maintainer approval to execute issue #319 autonomously.

---

Shipped at HEAD `7c3a0cb0b72d1a272bcec46b52e318806d152996` via
[PR #346](https://github.com/delaudio/trk/pull/346), with GitHub Actions CI
run `32383583128` green and issue #319 closed.
