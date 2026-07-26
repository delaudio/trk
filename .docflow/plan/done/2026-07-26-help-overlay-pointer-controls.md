# Help overlay pointer controls

Owning ADR: `../../adr/0011-control-the-help-overlay-with-the-pointer.md`

GitHub issue: #257

## Scope

Separate the Help overlay's fixed controls from its scrollable body, expose
render-owned targets for every visible tab, the body, and the close control,
and route primary clicks and wheel events through those targets.

Help keyboard navigation, clickable commands, drag scrolling, secondary clicks,
and other overlays are unchanged.

## Exit criteria

1. All five fixed-height tab labels expose stable indexed targets without
   wrapping into adjacent rows (ADR AC1).
2. Primary tab clicks select the page and reset Help scroll (ADR AC2).
3. The explicit visible close target releases Help focus capture and restores
   the underlying focused view (ADR AC3).
4. Wheel events scroll only over the Help content target (ADR AC4).
5. Borders, hints, outside clicks, secondary clicks, drags, and invalid tab
   payloads are no-ops, while existing Help key tests remain green (ADR AC5).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).

---

Shipped at HEAD `baf39b6` via
[PR #285](https://github.com/delaudio/salieri-tracker/pull/285), with GitHub
Actions CI run #295 green and issue #257 closed.
