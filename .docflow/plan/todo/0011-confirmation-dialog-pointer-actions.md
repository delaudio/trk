# Confirmation dialog pointer actions

Owning ADR: `../../adr/0013-activate-confirmation-dialog-actions-with-the-pointer.md`

GitHub issue: #259

## Scope

Expose fixed action targets for the visible dirty-quit and destructive
confirmation choices. Route primary clicks back through the equivalent
existing dialog-key handling and capture all other modal pointer input.

Dialog semantics, keyboard behavior, hover, and other overlays are unchanged.

## Exit criteria

1. Quit Save, Don't Save, and Cancel labels expose distinct fixed-height typed
   targets (ADR AC1).
2. Non-quit Confirm and Cancel labels expose distinct fixed-height typed
   targets (ADR AC2).
3. Primary clicks use the existing equivalent dialog-key path (ADR AC3).
4. Text, chrome, gaps, outside clicks, secondary clicks, drags, and invalid
   payloads are no-ops (ADR AC4).
5. Quit, delete, renderer geometry, and unchanged keyboard tests pass the full
   repository gate (ADR AC5).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).
