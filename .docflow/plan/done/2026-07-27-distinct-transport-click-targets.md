# Distinct transport click targets

Owning ADR: `../../adr/0014-control-play-and-stop-with-distinct-pointer-targets.md`

GitHub issue: #260

## Scope

Expose one-cell typed targets for the rendered Play and Stop symbols, and route
primary clicks through the existing pattern-start and stop actions. Remove the
broad header playback toggle so Record and surrounding chrome are inert.

Recording, other header controls, hover, secondary clicks, and drag gestures
are unchanged.

## Exit criteria

1. Play and Stop expose separate one-cell targets matching visible symbols at
   representative supported widths (ADR AC1–AC2).
2. Primary clicks call existing start-pattern and stop paths, with Stop safe
   when already stopped (ADR AC3).
3. Record, chrome, gaps, other header coordinates, secondary clicks, drags, and
   invalid payloads are no-ops (ADR AC4).
4. Renderer and application coverage distinguishes all three transport symbols
   and the full repository gate passes (ADR AC5).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).

---

Shipped at HEAD `c0b3ff4` via
[PR #291](https://github.com/delaudio/trk/pull/291), with GitHub
Actions CI run #310 green and issue #260 closed.
