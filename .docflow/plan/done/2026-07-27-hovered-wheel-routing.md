# Hovered-region mouse-wheel routing

Owning ADR: `../../adr/0021-route-wheel-input-by-rendered-region.md`

GitHub issue: #267

## Scope

Classify scroll targets from the render-owned interaction map and route
vertical and horizontal wheel events by the region beneath the pointer.
Preserve modal capture and existing bounded navigation behavior.

## Exit criteria

1. Pattern cells and rendered pattern-row content scroll pattern rows while
   borders, headers, and empty padding remain inert (ADR AC1).
2. Every currently scrollable list routes to its own bounded selection
   (ADR AC2).
3. Modal overlays capture wheel events without workspace fall-through
   (ADR AC3).
4. Non-scrollable regions are explicit no-ops (ADR AC4).
5. Horizontal events affect only pattern, clips, and loaded sampler waveform
   targets (ADR AC5).
6. Focused TUI and application tests cover semantic classification and all
   routing classes (ADR AC6).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed).

---

Shipped at HEAD `3b20b85` via
[PR #305](https://github.com/delaudio/salieri-tracker/pull/305), with GitHub
Actions CI run #338 green and issue #267 closed.
