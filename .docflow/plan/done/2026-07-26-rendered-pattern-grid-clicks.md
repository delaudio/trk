# Rendered pattern-grid clicks

Owning ADR: `../../adr/0004-select-rendered-pattern-cells.md`

GitHub issue: #250

## Scope

Expose the visible pattern cells as semantic interaction targets derived from
the same standard and Renoise-style grid geometry used for the current frame,
then make `trk-app` select the targeted absolute row and track.

The standard Full layout intentionally gains its missing trailing separator
column so rendered cells, track headers, viewport calculations, and hit regions
all use the same 28-column width. Responsive snapshots protect this alignment.

This item does not migrate headers, gutters, side panels, overlays, wheel
behaviour, field/digit selection, or other mouse surfaces.

## Exit criteria

1. Visible pattern-cell targets reuse the rendered grid geometry at 72×24,
   100×28, and 140×36 (ADR AC3).
2. Cell targets carry their absolute row and track after viewport offsets
   without introducing application mutations in `trk-tui` (ADR AC1).
3. Left clicks on a rendered cell select that row and track.
4. Headers, gutters, side panels, empty grid space, and clicks outside the grid
   do not move the cell cursor.
5. The fixed tracker-grid coordinate constants are removed from
   `trk-app`, while keyboard behaviour remains unchanged.

## Dependencies

- `../done/2026-07-26-render-owned-interaction-regions.md`
- `../../adr/0003-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).

---

Shipped at HEAD `7312620` via
[PR #271](https://github.com/delaudio/trk/pull/271), with GitHub
Actions CI run #262 green and issue #250 closed.
