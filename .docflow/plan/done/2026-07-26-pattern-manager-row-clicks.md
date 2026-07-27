# Pattern manager row clicks

Owning ADR: `../../adr/0008-select-pattern-manager-rows.md`

GitHub issue: #254

## Scope

Expose rendered full-screen Pattern Manager rows as semantic targets carrying
absolute pattern indices. Route primary clicks to selection and secondary
clicks to selection followed by opening the tracker.

Pattern mutations, keyboard controls, the pattern grid, and sidebar pattern
rows are unchanged.

## Exit criteria

1. Scrolled visible rows carry absolute indices (ADR AC1).
2. Primary clicks select and remain in Patterns (ADR AC2).
3. Secondary clicks select and open the tracker (ADR AC3).
4. Non-row geometry and drags are no-ops (ADR AC4).
5. Invalid payloads are rejected and keyboard tests remain green (ADR AC5).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).

---

Shipped at HEAD `08dbdae` via
[PR #279](https://github.com/delaudio/trk/pull/279), with GitHub
Actions CI run #280 green and issue #254 closed.
