# Architecture Notes

Salieri is split into workspace crates so the musical model stays independent from terminal and MIDI backends.

## Crates

- `salieri-audio`: post-MVP audio thread lifecycle, backend abstraction, and realtime command boundary. It must not depend on Ratatui or project serialization.
- `salieri-core`: song model, tracks, patterns, rows, cells, sequence operations, transport math, and playback event scheduling. It must not depend on Ratatui, Crossterm, MIDI, terminal state, audio backends, or filesystem APIs.
- `salieri-midi`: MIDI messages, fake output for tests, `midir` output connections, port listing, panic/all-notes-off, and conversion from core playback events.
- `salieri-sampler`: post-MVP WAV loading, preview buffer generation, and sample-to-track assignment metadata. It must stay optional for the MIDI-first playback path.
- `salieri-tui`: Ratatui rendering only. It receives immutable song data and view state from the app layer.
- `salieri-app`: CLI parsing, config loading, persistence, terminal lifecycle, input handling, undo/redo, playback runtime, MIDI connection state, and coordination between crates.

## Runtime Shape

The TUI render loop and sequencer are separate concerns:

```text
Terminal input -> App command handling -> Song/App state
                                   |
                                   v
                            Playback runtime -> MIDI output
                                   |
                                   v
                            Playhead updates -> TUI render state
```

The playback runtime owns timing and MIDI emission. The TUI polls playback updates and renders the latest known playhead position.

Detailed timing assumptions and jitter test limits are tracked in [timing.md](timing.md).

## Persistence

Project files use JSON with a `formatVersion` field and a serialized `Song`. File writes are atomic through a temporary file followed by rename.

The first format version is intentionally close to the internal model. Future migrations should keep DTOs separate if the internal model starts changing faster than the file format.

## Test Boundaries

- Core behavior belongs in `salieri-core` unit tests.
- MIDI byte conversion and fake output behavior belong in `salieri-midi`.
- Rendering smoke/snapshot-style tests belong in `salieri-tui`.
- CLI, config, persistence, input mapping, undo/redo, and playback runtime coordination belong in `salieri-app`.
