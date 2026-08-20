# Pattern variation history

Owning ADR: `../../adr/0025-browse-and-restore-persistent-pattern-variations.md`

GitHub issue: #314

## Scope

Add a bounded project-level variation history, persist it inside the existing
`.trk` envelope, record successful AI applications and Euclidean transforms,
and provide the normal-mode `v` modal for inspecting and restoring snapshots.
Keep ordinary undo/redo independent, make restore undoable, retain uppercase
`V` selection and `Ctrl+v` paste, and do not introduce sidecar storage or a
branching history graph.

## Exit criteria

1. The 64-entry bounded variation model, monotonic ids, metadata, snapshot
   validation, and active marker satisfy ADR AC1.
2. Legacy projects and history-bearing projects load and save atomically with
   invalid history rejected (ADR AC2).
3. Successful AI application and Euclidean CLI output record versions while
   rejected, failed, cancelled, and no-op paths do not (ADR AC3).
4. The `v` modal, keyboard navigation, entry metadata, active badge, empty
   state, dismissal, uppercase `V` selection, and `Ctrl+v` compatibility
   satisfy ADR AC4.
5. Restore is pattern-scoped, undoable, dirty-tracked, cursor-aware, and active
   state is reconciled after later edits, undo, and redo (ADR AC5).
6. Focused model, persistence, app, key-routing, and render snapshot tests plus
   the complete repository gate pass (ADR AC6).
7. Norn review with the Codex provider has no unresolved in-scope findings
   before publication.

## Dependencies

- `../../adr/0001-record-architecture-decisions.md`
- `../../adr/0024-select-ai-proposal-engines-at-runtime.md`
- Maintainer approval to execute issue #314 autonomously.
