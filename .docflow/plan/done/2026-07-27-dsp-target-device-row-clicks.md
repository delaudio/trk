# DSP target and device row clicks

Owning ADR: `../../adr/0015-select-dsp-targets-and-device-rows-with-the-pointer.md`

GitHub issue: #261

## Scope

Expose typed render-owned targets for the Track and Master DSP controls and
each visible device row. Route primary clicks through DSP selection state so
the clicked chain and device become current and the parameter panel refreshes.

Palette interaction, reordering, hover, drag gestures, and parameter adjustment
semantics are unchanged.

## Exit criteria

1. Track and Master controls expose exact typed targets and primary clicks
   select the named chain (ADR AC1).
2. Each rendered device row carries its chain and absolute device index; a
   primary click selects both (ADR AC2).
3. Device selection bounds the parameter cursor and refreshes the parameter
   panel for that device (ADR AC3).
4. Empty space, borders, secondary clicks, drags, invalid payloads, and stale
   indices are no-ops (ADR AC4).
5. Focused renderer and application coverage exercises responsive/clipped
   geometry, empty chains, routing, and parameter refresh (ADR AC5).
6. Existing keyboard DSP navigation tests remain green (ADR AC6).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).

---

Shipped at HEAD `b748451` via
[PR #293](https://github.com/delaudio/trk/pull/293), with GitHub
Actions CI run #314 green and issue #261 closed.
