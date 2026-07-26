# Render-owned interaction regions

Owning ADR: `../../adr/0003-render-owned-interaction-regions.md`

GitHub issue: #249

## Scope

Introduce the semantic interaction-map types, populate the map from the current
frame's top-level and managed-panel layout calculations, retain the latest map
in the application, and add representative multi-size tests.

This item does not migrate existing mouse behaviour; each interactive surface
is queued as a separate GitHub issue.

## Exit criteria

1. The frame exposes stable semantic identifiers and rectangles (ADR AC1).
2. Managed-panel regions reuse the rendered responsive layout (ADR AC2).
3. The application retains the latest map without TUI-owned mutation logic
   (ADR AC3).
4. Reverse-order hit testing is covered by tests (ADR AC4).
5. Region tests pass at 72×24, 100×28, and 140×36, with existing snapshots and
   keyboard behaviour unchanged (ADR AC5).

## Dependencies

- None.

---

Shipped at HEAD `952f834` via
[PR #269](https://github.com/delaudio/salieri-tracker/pull/269).
