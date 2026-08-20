---
adr: 0025
title: Browse and restore persistent pattern variations
status: Accepted
date: 2026-08-20
owner: default-agent
supersedes:
superseded-by:
depends-on: [0001, 0024]
tags: [history, patterns, persistence, tui, transforms, ai]
---

# ADR 0025 — Browse and restore persistent pattern variations

## Context

`trk` has bounded in-memory undo and redo for linear editing, but generated
pattern takes are not durable or directly comparable. Applying an AI proposal
or a Euclidean transform changes the song without retaining the action
description, affected pattern and track, timestamp, or a restorable pattern
snapshot. Closing the application therefore loses the creative lineage, while
reaching an older take through repeated undo is fragile and session-local.

The project format already provides an atomically replaced JSON envelope around
the portable song. Variation history is project workflow metadata rather than
musical song content, so it belongs in that envelope instead of inside `Song`.
The existing undo transaction remains the authority for ordinary linear edits
and must also make an explicit historical restore undoable.

The plain `v` key currently starts a visual selection, while `Ctrl+v` pastes.
The requested history browser takes lowercase `v`; starting a visual selection
moves to uppercase `V`, and clipboard paste remains on `Ctrl+v`.

## Capability statement

`trk` records bounded, descriptive snapshots after successful generative
pattern changes, persists them with the project, presents them in an in-TUI
history browser, and restores a selected pattern take immediately through the
normal undoable mutation boundary.

## User stories / scenarios

- As a musician exploring variations, I want every successful generated take
  recorded with its prompt or transform parameters, so that I can compare the
  creative path later.
- As a tracker user, I want a keyboard-driven history browser, so that I can
  inspect and restore a take without walking backward through unrelated edits.
- As a project owner, I want variation history saved in the project file, so
  that reopening the project preserves both the takes and the active marker.
- As an editor, I want a restore to participate in undo, so that an accidental
  rollback is reversible.

## Acceptance criteria

1. The project model represents a variation with a monotonic version id, Unix
   timestamp, non-empty description, source kind, pattern index, optional track
   index, and a complete snapshot of the affected pattern. History stores the
   next id, an optional active id, and at most 64 retained entries, dropping the
   oldest entries without reusing ids. The fixed cap avoids a new configuration
   surface while bounding project-file growth.
2. Variation history is an optional, backward-compatible field of the `.trk`
   project envelope. Loading validates every snapshot and active reference;
   saving preserves song and history through the existing atomic replacement
   path, while legacy projects without history still load unchanged.
3. Successfully applying an AI proposal records its resulting pattern with the
   proposal prompt as description. The Euclidean CLI transform records its
   resulting pattern and parameters. Rejected, cancelled, failed, or no-op
   operations create no version, and future generative transforms use the same
   recording boundary.
4. In normal mode, plain `v` opens a double-bordered history modal; Up/Down
   changes selection, Enter restores the selected version, and Esc or `v`
   closes without mutation. Entries show version, time, description,
   pattern/track context, and an `[ACTIVE]` badge; an empty history renders an
   explicit empty state. Uppercase `V` continues to start a visual selection,
   and `Ctrl+v` continues to paste.
5. Restore replaces only the recorded pattern through the existing undo
   transaction, selects that pattern and track when possible, marks the project
   dirty, closes the modal, and sets the requested active version. Restoring a
   snapshot already identical to the live pattern is an explicit no-op that
   leaves the active marker and undo stack unchanged. After ordinary edits, undo, or redo,
   the active id is reconciled to the newest retained entry whose snapshot
   exactly equals its live affected pattern, or cleared when none matches;
   undoing a restore reinstates the prior song and reconciles the marker.
6. Automated tests cover bounded recording and ids, legacy and history-bearing
   project round trips, invalid persisted history, AI and Euclidean recording,
   no-op/failure exclusion, modal navigation and dismissal, restore plus undo,
   active-marker invalidation, clipboard compatibility, and TUI snapshots.

## Out of scope

- A branching version graph, merges, or comparison diffs between takes.
- User-authored renaming or deletion of individual historical entries.
- Recording every manual cell edit, transport change, or mixer adjustment.
- Persisting the general undo/redo stack.
- External history directories or sidecar files.

## Open questions

- None.

## References

- `0001-record-architecture-decisions.md`
- `0024-select-ai-proposal-engines-at-runtime.md`
- `../plan/todo/0023-pattern-variation-history.md`
- GitHub issue #314.
- [grain history manager](https://github.com/delaudio/grain/blob/c609fcd5a05d8862d88819690a051f5df13238be/src/history/manager.rs)
- [grain history record](https://github.com/delaudio/grain/blob/c609fcd5a05d8862d88819690a051f5df13238be/src/history/record.rs)

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-08-20 | r1 | default-agent | Recorded and accepted persistent, bounded pattern variation snapshots and undoable TUI restore. |
| 2026-08-20 | r2 | default-agent | Fixed retention at 64 entries, defined active-marker reconciliation across undo/redo, and pinned prior-art references after Norn review. |
| 2026-08-20 | r3 | default-agent | Corrected the existing key contract: lowercase v opens history, uppercase V starts selection, and Ctrl+v pastes. |
| 2026-08-20 | r4 | default-agent | Required song-aware persisted-history validation and made identical-snapshot restore an explicit no-op after implementation review. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-08-20 | Approved autonomous resolution of the prioritized GitHub issue queue in chat. |
