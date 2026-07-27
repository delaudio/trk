# Command palette entry clicks

Owning ADR: `../../adr/0010-activate-command-palette-entries.md`

GitHub issue: #256

## Scope

Expose the command palette result-list area and visible entries as semantic
targets. Route primary entry clicks through selection and existing execution,
and scope wheel selection changes to the rendered result-list area.

Query editing, other overlays, the DSP palette, secondary clicks, and drag
gestures are unchanged.

## Exit criteria

1. Scrolled visible entries carry absolute result indices and fixed-height
   row geometry (ADR AC1, AC5).
2. Primary clicks select and execute enabled entries through the existing
   execution path (ADR AC2).
3. Disabled entries select without executing or closing the palette (ADR AC3).
4. Wheel events move selection only over the result list and clamp at its
   bounds (ADR AC4).
5. Borders, search/help rows, empty results, outside clicks, secondary clicks,
   drags, and invalid payloads are no-ops (ADR AC5).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).

---

Shipped at HEAD `41860c6` via
[PR #283](https://github.com/delaudio/trk/pull/283), with GitHub
Actions CI run #290 green and issue #256 closed.
