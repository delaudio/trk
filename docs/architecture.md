# Architecture Notes

trk is split into workspace crates so the musical model stays independent from terminal and MIDI backends.

The measured module audit, target shape, and ordered refactor work are maintained in the [Architecture Quality Roadmap](architecture-roadmap.md).

## Crates

- `trk-audio`: post-MVP audio thread lifecycle, backend abstraction, realtime command boundary, and offline audio export primitives. It must not depend on Ratatui or project serialization.
- `trk-ai`: post-MVP AI-assist boundary for local or external proposal providers. It must never contact external services implicitly, and generated edits must be reviewable before they are applied.
- `trk-core`: song model, tracks, patterns, rows, cells, sequence operations, transport math, and playback event scheduling. It must not depend on Ratatui, Crossterm, MIDI, terminal state, audio backends, or filesystem APIs.
- `trk-interop`: post-MVP import/export boundary for MIDI files and future tracker formats. trk native `.trk` files remain canonical.
- `trk-midi`: MIDI messages, separated input/output traits, fake MIDI endpoints for tests, `midir` input/output connections, port listing, panic/all-notes-off, and conversion from core playback events.
- `trk-sampler`: post-MVP WAV loading, preview buffer generation, and sample-to-track assignment metadata. It must stay optional for the MIDI-first playback path.
- `trk-transform`: post-MVP deterministic song and pattern transforms. It must remain pure core-model logic so app and CLI integrations can wrap edits in undoable operations.
- `trk-tui`: Ratatui rendering only. It receives immutable song data and view state from the app layer.
- `trk-app`: CLI parsing, config loading, persistence, terminal lifecycle, input handling, undo/redo, playback runtime, MIDI connection state, and coordination between crates.

Plugin hosting is explicitly deferred in the
[plugin-hosting evaluation](plugin-hosting-evaluation.md). No VST, AU, or CLAP
SDK types should be introduced until a separate architecture review chooses a
host strategy.

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

Terminal input and external runtime results enter mutable app state through the
typed [application event and action boundary](app-events.md). Its synchronous
FIFO dispatcher preserves current ordering while keeping future background-task
events explicit.

Detailed timing assumptions and jitter test limits are tracked in [timing.md](timing.md).

## Dependency Direction And Ownership

Internal crate dependencies point toward stable domain data and away from application and presentation concerns. The allowed graph is:

```text
trk-app -> trk-ai, trk-audio, trk-core, trk-interop,
               trk-midi, trk-sampler, trk-transform, trk-tui
trk-tui -> trk-core, trk-sampler
trk-audio -> trk-sampler -> trk-core
trk-ai -> trk-core
trk-interop -> trk-core
trk-midi -> trk-core
trk-transform -> trk-core
trk-core -> (none)
```

The machine-readable policy and concise ownership statements live in `config/crate-dependency-policy.json`. Run `python3 scripts/check_crate_dependencies.py` locally; CI runs the same check against structured `cargo metadata` output. Every workspace crate must have a policy entry, and adding an internal dependency that is not explicitly allowed fails the check.

Ownership of cross-cutting responsibilities is split as follows:

| Responsibility | Owner | Boundary |
| --- | --- | --- |
| Serializable song data and validation | `trk-core` | Plain domain data; no filesystem, backend, or UI types |
| Native project persistence | `trk-app` | File loading, migration orchestration, and atomic writes |
| External format serialization | `trk-interop` | Imports/exports domain data through `trk-core` |
| Playback semantics | `trk-core` | Deterministic transport math and scheduled events |
| Realtime coordination | `trk-app` | Threads, lifecycle, routing, and status propagation |
| Audio and MIDI I/O | `trk-audio`, `trk-midi` | Backend-specific processing at the workspace edge |
| UI state and input | `trk-app` | Mutable application/view state and input dispatch |
| Rendering | `trk-tui` | Immutable inputs and presentation only |
| Background tasks | `trk-app` | The [task runtime](tasks.md) owns IDs, lifecycle, progress, cooperative cancellation, diagnostics, and typed results |
| External integrations | `trk-ai`, `trk-interop`, `trk-midi`, `trk-audio` | Provider or protocol details stay out of core and TUI |

New feature issues should name the owning crate and any required dependency edges. Put musical invariants and serializable state in core; deterministic transformations in transform; format adapters in interop; protocol and device adapters in MIDI/audio; pure rendering in TUI; and coordination, filesystem access, configuration, or user interaction in app. A feature that does not fit these rules should update the ownership decision before introducing a new edge.

## Persistence

Project files use JSON with a `formatVersion` field and a serialized `Song`. File writes are atomic through a temporary file followed by rename.

The first format version is intentionally close to the internal model. Future migrations should keep DTOs separate if the internal model starts changing faster than the file format.

## Test Boundaries

- Core behavior belongs in `trk-core` unit tests.
- MIDI byte conversion and fake output behavior belong in `trk-midi`.
- Rendering smoke/snapshot-style tests belong in `trk-tui`.
- CLI, config, persistence, input mapping, undo/redo, and playback runtime coordination belong in `trk-app`.

## Rust Module Size Budgets

Run `scripts/check-rust-file-sizes.sh` locally before adding substantial Rust code. The check reports the largest source files and applies per-domain soft and hard limits from `config/rust-file-size-budgets.tsv`:

| Domain | Soft limit | Hard limit |
| --- | ---: | ---: |
| Application | 600 | 1,000 |
| TUI | 500 | 800 |
| Core | 500 | 800 |
| Audio | 500 | 800 |
| Interop | 500 | 800 |
| Other workspace crates | 500 | 800 |

Crossing a soft limit produces a warning so a module split can be planned. Crossing a hard limit fails CI. Existing oversized files are capped at their recorded line counts in `config/rust-file-size-baseline.tsv`; they may shrink but may not grow. Each baseline exception must link to a tracking issue, and obsolete entries fail the check after their file is removed.

A new hard-limit exception requires a documented tracking issue and an explicit baseline entry. Prefer splitting the module instead: the exception is a visible, temporary debt record rather than a higher default limit.
