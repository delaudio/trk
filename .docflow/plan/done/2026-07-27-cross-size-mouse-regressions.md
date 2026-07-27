# Cross-size mouse interaction regressions

Owning ADR: `../../adr/0022-verify-pointer-dispatch-against-rendered-geometry.md`

GitHub issue: #268

## Scope

Add a test rendering harness that captures the application's real interaction
map and dispatches mouse events from rendered regions across responsive
terminal sizes. Cover representative pattern, composite, browser, overlay,
DSP, and sampler targets, including scrolled content and outside-region
no-ops.

## Exit criteria

1. The matrix covers 72×24, 80×24, 100×28, and 140×36 (ADR AC1).
2. Pattern cells, composite panels, browsers, overlays, DSP, and sampler
   controls are exercised (ADR AC2).
3. Covered scrollable lists use non-zero visible start offsets (ADR AC3).
4. Every target class includes an immediately-outside no-op assertion
   (ADR AC4).
5. All click points are derived from the rendered interaction map and fail
   when renderer and dispatcher disagree (ADR AC5).
6. The full repository validation gate and Codex review are green.

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issues #249 and #267 (closed).

---

Shipped at HEAD `76fe0f1` via
[PR #307](https://github.com/delaudio/trk/pull/307), with GitHub
Actions CI run #342 green and issue #268 closed.
