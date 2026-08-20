# Contextual parameter pages

Owning ADR: `../../adr/0032-control-tracks-through-contextual-parameter-pages.md`

GitHub issue: #321

## Scope

Add the six-page, eight-slot parameter surface using canonical sampler, mixer,
DSP, and algorithm bindings; render and edit row-local parameter locks through
keyboard and pointer interactions; route repeated page presses to existing deep
editors; and add session-only song snapshots, beat-boundary reload, and the
first eight instant track-mute shortcuts. Unsupported bindings remain explicit
and inert rather than introducing placeholder sound state.

## Exit criteria

1. Typed pages, stable slots, shortcuts, and repeated-page deep routing satisfy
   ADR AC1 and AC6.
2. Canonical dynamic bindings and explicit disabled behavior satisfy ADR AC2.
3. Responsive grid, values, meters, lock LEDs/tags, and render-owned pointer
   targets satisfy ADR AC3 and AC5.
4. Descriptor-aware fine/coarse adjustment, canonical lock upsert/removal, and
   undo behavior satisfy ADR AC4.
5. Complete session snapshot and immediate/next-beat uninterrupted restore
   satisfy ADR AC7.
6. The first eight instant mute shortcuts satisfy ADR AC8.
7. Focused model, application, scheduler, render, and snapshot coverage plus
   the complete repository verification gate and Norn review satisfy ADR AC9.

## Dependencies

- `../../adr/0003-expose-render-owned-interaction-regions.md`
- `../../adr/0015-select-dsp-targets-and-device-rows-with-the-pointer.md`
- `../../adr/0016-select-dsp-parameters-and-palette-entries-from-rendered-rows.md`
- `../../adr/0017-control-supported-sampler-actions-with-the-pointer.md`
- `../../adr/0025-browse-and-restore-persistent-pattern-variations.md`
- `../../adr/0031-live-code-patterns-with-mini-notation.md`
- Maintainer approval to execute issue #321 autonomously.
