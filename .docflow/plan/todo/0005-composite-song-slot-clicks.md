# Composite song-slot clicks

Owning ADR: `../../adr/0007-select-composite-song-slots.md`

GitHub issue: #253

## Scope

Expose the rows drawn by the responsive composite Song Slots panel as semantic
interaction targets carrying absolute sequence positions, then select those
targets from Normal/Edit primary-click dispatch.

This item does not change the full-screen Sequence view, keyboard navigation,
transport controls, sequence editing, or the Renoise-style large workspace.

## Exit criteria

1. Visible composite rows carry absolute sequence positions after centered
   scrolling (ADR AC1).
2. A primary click selects the sequence cursor while preserving the Pattern
   workspace and existing keyboard semantics (ADR AC2).
3. Selection does not start or otherwise change playback (ADR AC3).
4. Borders, empty rows, drag events, and secondary clicks are no-ops
   (ADR AC4).
5. Out-of-range semantic payloads are rejected and keyboard tests remain
   green (ADR AC5).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).
