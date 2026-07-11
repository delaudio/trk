# Generative Transforms

Generative composition is post-MVP. The first foundation is `salieri-transform`, a pure library crate that mutates `salieri-core::Song` data deterministically.

Current transform:

- Euclidean rhythm generation;
- deterministic pattern application to one track;
- scriptable CLI application to `.salieri` project files;
- touched-cell report so app integrations can wrap the mutation in undo/redo snapshots.

The crate intentionally has no terminal, MIDI, filesystem, or random dependencies. Project files remain normal `.salieri` JSON after a transform is applied.

The CLI exposes scriptable commands such as:

```bash
salieri transform euclidean input.salieri output.salieri --pattern 1 --track 1 --steps 16 --pulses 5 --pitch 36 --velocity 100
```

Future in-app wiring should call the same transform functions through the normal command handler so every generated edit remains undoable.
