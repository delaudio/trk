# Sampler direct pointer controls

Owning ADR: `../../adr/0017-control-supported-sampler-actions-with-the-pointer.md`

GitHub issue: #263

## Scope

Expose typed render-owned targets for ADSR field selection, fine decrement and
increment, waveform zoom and pan, and Browse in both compact and large sampler
layouts. Route primary clicks through the methods already used by sampler
keyboard controls.

Drag editing, wheel behavior, coarse adjustment, waveform scrubbing, and new
sampler operations are unchanged.

## Exit criteria

1. Each ADSR field carries its semantic field and primary click selects it
   without mutation (ADR AC1).
2. Visible decrement and increment targets use existing fine envelope
   adjustment (ADR AC2).
3. Visible zoom and pan targets use existing waveform actions (ADR AC3).
4. Browse is visible and opens the in-app sample browser with or without a
   loaded sample (ADR AC4).
5. Renderer tests cover compact, large, loaded, empty, and clipped layouts
   with exact target geometry (ADR AC5).
6. Application tests cover each action plus secondary, drag, invalid, stale,
   border, help, waveform, and empty-space no-ops (ADR AC6).
7. Existing sampler keyboard tests remain green (ADR AC7).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).

---

Shipped at HEAD `2275f54` via
[PR #297](https://github.com/delaudio/salieri-tracker/pull/297), with GitHub
Actions CI run #322 green and issue #263 closed.
