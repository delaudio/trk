# DSP parameter and palette row clicks

Owning ADR: `../../adr/0016-select-dsp-parameters-and-palette-entries-from-rendered-rows.md`

GitHub issue: #262

## Scope

Expose typed render-owned targets for each visible DSP parameter row and
device-palette entry. Route primary parameter selection, secondary parameter
adjustment, and primary palette assignment through their absolute payload
indices.

Drag adjustment, wheel behavior, palette contents, and keyboard shortcuts are
unchanged.

## Exit criteria

1. Parameter rows expose absolute one-line targets at multiple terminal heights
   (ADR AC1).
2. Primary and secondary clicks select the payload parameter, with secondary
   click using the existing positive adjustment (ADR AC2).
3. Scrolled palette rows carry absolute device indices and assign the clicked
   type through the existing action (ADR AC3).
4. Help, empty space, borders, drags, invalid payloads, and stale indices are
   no-ops (ADR AC4).
5. Fixed first-row constants/helpers are removed and focused renderer plus
   application tests pass (ADR AC5).
6. Existing keyboard DSP tests remain green (ADR AC6).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).
