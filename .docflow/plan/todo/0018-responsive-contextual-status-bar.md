# Responsive contextual status bar

Owning ADR: `../../adr/0020-compose-contextual-status-hints-by-width.md`

GitHub issue: #266

## Scope

Replace arbitrary clipping of contextual shortcut text with a width-aware
composer of complete priority-ordered segments. Keep command input and
notifications authoritative and add narrow/medium snapshot coverage for every
main view.

## Exit criteria

1. Every main view keeps its mode and first three actions at 72 columns
   (ADR AC1).
2. Wider layouts add only complete priority-ordered segments (ADR AC2).
3. Focused tests prove shortcut labels and delimiters are never partially
   rendered (ADR AC3).
4. Command input and notification rendering continues to replace shortcuts
   (ADR AC4).
5. Aggregate 72- and 100-column snapshots cover all ten main views (ADR AC5).

## Dependencies

- `../../adr/0019-compose-the-transport-header-by-width.md`
- `../done/2026-07-27-responsive-transport-header.md`
- GitHub issue #265 (closed).
