# External editor and project hot reload

Owning ADR: `../../adr/0027-edit-and-hot-reload-projects-externally.md`

GitHub issue: #316

## Scope

Add a normal-mode `e` handoff to the configured external editor, including an
unnamed-project scratch flow, and a bounded watcher for the current named
project. Reuse atomic project persistence and terminal suspension, guard dirty
local state, preserve active transport, and report invalid external writes
without repeated errors.

## Exit criteria

1. Key routing, terminal suspension, shell-free editor resolution/argv parsing,
   distinct path passing, and diagnostics satisfy ADR AC1–AC2.
2. Clean named-path editing, dirty/unnamed exclusive scratch adoption, and
   missing-path refusal satisfy ADR AC3.
3. Portable signatures, bounded polling, and internal-save suppression satisfy
   ADR AC4.
4. Reload state replacement, clamping, undo cleanup, and transport preservation
   satisfy ADR AC5.
5. Dirty conflicts and every invalid filesystem/project/editor path satisfy ADR
   AC6.
6. Status messages, focused cross-platform tests, terminal recovery checks, and
   the complete repository gate satisfy ADR AC7.
7. Norn review with the Codex provider has no unresolved actionable findings
   before publication.

## Dependencies

- `../../adr/0001-record-architecture-decisions.md`
- Maintainer approval to execute issue #316 autonomously.
