# Generative Transforms

Generative composition is post-MVP. The first foundation is `trk-transform`, a pure library crate that mutates `trk-core::Song` data deterministically.

Current transform:

- Euclidean rhythm generation;
- deterministic pattern application to one track;
- scriptable CLI application to `.trk` project files;
- touched-cell report so app integrations can wrap the mutation in undo/redo snapshots;
- a persistent pattern-variation snapshot when a CLI Euclidean transform changes the song.

The crate intentionally has no terminal, MIDI, filesystem, or random dependencies. Project files remain normal `.trk` JSON after a transform is applied.

The CLI exposes scriptable commands such as:

```bash
trk transform euclidean input.trk output.trk --pattern 1 --track 1 --steps 16 --pulses 5 --pitch 36 --velocity 100
```

No-op transforms do not add variation entries. The resulting project can browse
and restore recorded generated patterns from the TUI with `v` in normal mode.
Future in-app wiring should call the same transform functions through the normal
command handler so every generated edit remains undoable.
