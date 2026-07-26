# Visible browser-entry clicks

Owning ADR: `../../adr/0005-select-visible-browser-entries.md`

GitHub issue: #251

## Scope

Expose the sample and project entries actually drawn in the current frame as
semantic interaction targets carrying absolute entry indices, then make browser
mouse handlers select and activate only those targets.

This item includes grouped project-browser section headings as non-interactive
rows. It does not change browser wheel behaviour, directory navigation, list
ordering, preview logic, or scrollbar behaviour.

## Exit criteria

1. Rendered sample and project entry rows carry absolute entry indices after
   non-zero viewport offsets (ADR AC1).
2. Primary clicks select the targeted entry and retain existing activation
   behaviour (ADR AC2).
3. A sample-browser secondary click assigns the targeted supported sample
   rather than the stale cursor entry (ADR AC3).
4. Borders, headers, section headings, and empty rows do not mutate selection
   (ADR AC4).
5. Long-list tests cover both browsers with non-zero offsets (ADR AC5).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).

---

Shipped at HEAD `af5e899` via
[PR #273](https://github.com/delaudio/salieri-tracker/pull/273), with GitHub
Actions CI run #266 green and issue #251 closed.
