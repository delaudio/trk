# Strudel mini-notation live coding

Owning ADR: `../../adr/0031-live-code-patterns-with-mini-notation.md`

GitHub issue: #320

## Scope

Add a reusable bounded mini-notation parser and deterministic pattern
evaluator, then integrate it with the active tracker pattern through an atomic
`:strudel` command and a dedicated live editing bar. Map layers to consecutive
tracks, scale degrees to MIDI pitches, and accepted live edits to canonical
cells and future playback rows. Keep the existing Strudel export unchanged;
do not execute JavaScript or add sub-row event storage.

## Exit criteria

1. Typed parsing, bounded errors, and every requested structural operator meet
   ADR AC1.
2. Deterministic note, velocity, gate, instrument, layer, and bounds behavior
   meet ADR AC2.
3. Note-name and scale-degree quantization meet ADR AC3.
4. Atomic command mutation, rollback, undo, and result reporting meet ADR AC4.
5. Transactional live editing, canonical preview, error, one-step accept, and
   cell plus playback-schedule rollback semantics meet ADR AC5.
6. Playback applies live updates at a row boundary without a transport stop or
   audio-runtime reconstruction, satisfying ADR AC6.
7. Unit, application, runtime tests, the complete repository verification
   gate, and Norn review satisfy ADR AC7.

## Dependencies

- `../../adr/0001-record-architecture-decisions.md`
- `../../adr/0025-browse-and-restore-persistent-pattern-variations.md`
- Maintainer approval to execute issue #320 autonomously.
