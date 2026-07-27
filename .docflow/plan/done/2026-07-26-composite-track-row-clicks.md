# Composite track-row clicks

Owning ADR: `../../adr/0006-select-composite-track-rows.md`

GitHub issue: #252

## Scope

Expose the track rows drawn by the responsive composite Tracks panel as
semantic interaction targets carrying absolute track indices, then select those
targets from Normal/Edit mouse dispatch.

This item does not change the full-screen Tracks view, Renoise-style workspace,
keyboard navigation, mute/solo controls, or track ordering.

## Exit criteria

1. Visible composite rows carry absolute track indices after centered scrolling
   (ADR AC1).
2. Clicking a row selects its track without changing row, field, or digit
   (ADR AC2).
3. Borders and empty rows are no-ops (ADR AC3).
4. Out-of-range semantic payloads are rejected (ADR AC4).
5. Existing keyboard tests remain green (ADR AC5).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).

---

Shipped at HEAD `d2f9b3b` via
[PR #275](https://github.com/delaudio/trk/pull/275), with GitHub
Actions CI run #270 green and issue #252 closed.
