# Sequence editor row clicks

Owning ADR: `../../adr/0009-select-sequence-editor-rows.md`

GitHub issue: #255

## Scope

Expose rendered full-screen Sequence rows as dedicated semantic targets
carrying absolute sequence positions. Route primary clicks to selection and
secondary clicks to selection followed by sequence playback from that
position.

Sequence mutations, keyboard controls, the composite Song Slots panel, and the
Clip Launcher are unchanged.

## Exit criteria

1. Scrolled visible rows carry absolute sequence positions and stay aligned
   with one-line targets at narrow widths (ADR AC1).
2. Primary clicks select and remain in Sequence (ADR AC2).
3. Secondary clicks select and start playback at the clicked position
   without leaving Sequence (ADR AC3).
4. Non-row geometry and drag events are no-ops (ADR AC4).
5. Invalid payloads are rejected and keyboard tests remain green (ADR AC5).

Renderer coverage must include a narrow-width regression proving every
registered target aligns with the sequence position displayed on that row.
Mouse-routing coverage must assert both primary and secondary clicks retain
the full-screen Sequence view.

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).
