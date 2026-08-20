# trk

> A MIDI-first music tracker and composition environment for the terminal.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/delaudio/trk/actions/workflows/ci.yml/badge.svg)](https://github.com/delaudio/trk/actions/workflows/ci.yml)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux-lightgrey.svg)]()

`trk` is a fast, responsive terminal tracker built with **Rust**, **Ratatui**, and **Crossterm**. It combines pattern and sequence editing, persistent `.trk` JSON projects, realtime MIDI I/O, WAV sample playback, native audio DSP, row parameter locks, deterministic generative transforms, and reviewable AI-assisted composition edits directly in your shell.

---

## Table of Contents

- [Overview & Architecture](#overview--architecture)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Key Features](#key-features)
- [TUI Commands & Keybindings](#tui-commands--keybindings)
  - [Global Shortcuts](#global-shortcuts)
  - [Navigation & Editing](#navigation--editing)
  - [Tracks, Patterns & Sequence](#tracks-patterns--sequence)
  - [Mouse Controls](#mouse-controls)
  - [Layout Commands](#layout-commands)
- [Command Mode Reference (`:`)](#command-mode-reference-)
- [Tracker Mechanics & DSP](#tracker-mechanics--dsp)
  - [Cell Format](#cell-format)
  - [Tracker FX vs Native DSP vs Parameter Locks](#tracker-fx-vs-native-dsp-vs-parameter-locks)
- [MIDI & Routing Setup](#midi--routing-setup)
  - [macOS IAC Driver Setup](#macos-iac-driver-setup)
  - [Ableton Live Routing](#ableton-live-routing)
  - [Renoise Routing](#renoise-routing)
  - [MIDI Debugging](#midi-debugging)
- [CLI & Interoperability](#cli--interoperability)
  - [CLI Syntax & Options](#cli-syntax--options)
  - [Import & Export Formats](#import--export-formats)
  - [Audio Export & Stems](#audio-export--stems)
  - [Generative & AI Tools](#generative--ai-tools)
- [Development & Testing](#development--testing)
- [Roadmap Gaps](#roadmap-gaps)
- [License](#license)

---

## Overview & Architecture

`trk` is designed as a modular Rust workspace:

```text
               ┌──────────────────────────────────────────────┐
               │           trk-app (CLI & Main Loop)          │
               └──────────────────────┬───────────────────────┘
                                      │
               ┌──────────────────────┴───────────────────────┐
               │              trk-tui (Ratatui UI)            │
               └───────┬──────────────┬──────────────┬────────┘
                       │              │              │
        ┌──────────────┴───┐   ┌──────┴─────────┐  ┌─┴────────────┐
        │  trk-core        │   │  trk-audio     │  │  trk-midi    │
        │  (Song Model &   │   │  (CPAL Engine &│  │  (CoreMIDI & │
        │   Pattern Data)  │   │   DSP Chains)  │  │   ALSA I/O)  │
        └───────┬──────────┘   └──────┬─────────┘  └──────────────┘
                │                     │
  ┌─────────────┼──────────────┐      │
  │             │              │      │
┌─┴──────────┐ ┌┴───────────┐ ┌┴──────┴───────┐
│ trk-ai     │ │ trk-trans- │ │ trk-sampler   │
│ (Proposal  │ │ form       │ │ (WAV Samples, │
│  Engine)   │ │ (Euclidean)│ │  Looping)     │
└────────────┘ └────────────┘ └───────────────┘
```

External hardware or software synthesizers are controlled in realtime via MIDI. Assigned WAV samples play through the internal CPAL backend and share the exact same mixer and per-track DSP chain as offline audio export.

---

## Installation

### Homebrew (macOS)

Install `trk` via the official Homebrew tap:

```bash
brew install delaudio/tap/trk
```

See [`docs/HOMEBREW.md`](docs/HOMEBREW.md) for tap bootstrap and release details.

### From Source (Cargo)

Build and install locally using Cargo:

```bash
git clone https://github.com/delaudio/trk.git
cd trk
cargo install --path crates/trk-app
```

---

## Quick Start

Launch `trk` in interactive mode:

```bash
trk
```

Open a project file or the bundled foundation fixture:

```bash
trk song.trk
trk fixtures/projects/foundations.trk
```

Inside the TUI:
- Press **`H`** or **`?`** to open contextual help overlays.
- Press **`Ctrl+P`** for the command palette.
- Import a MIDI file directly via the palette or command line:
  ```text
  :midi import /path/to/song.mid
  ```

---

## Key Features

### 🎛️ Terminal Workspace & Editing
- **Multi-View Workspace**: Pattern, Sequence, Clips, Tracks, Patterns, Sampler, DSP Rack, Sample Browser, Project Browser, MIDI Settings, AI Chat, command palette, and help views.
- **Pattern Editing**: Synchronized tracker and horizontal Piano Roll views with note gates, ghost notes, velocity, instrument, volume, pan, delay, effect columns, row insert/delete, copy/cut/paste, undo/redo transaction history, persistent generated-variation snapshots, and playhead follow.
- **Annotations**: Project, pattern-row, lyric, and cue text annotations that persist without affecting playback.

### 🎹 MIDI & Audio
- **MIDI Output Routing**: Port listing, TUI connection, panic/all-notes-off, channel assignment, logging, and test-note CLI tool.
- **MIDI Input & Sync**: Port listing, note-on recording into active patterns, and MIDI clock start/continue/stop follow.
- **Sample Engine**: 16-bit PCM & 32-bit float WAV sample loading, terminal-aware waveform heatmaps, sample browser, multi-mode looping (one-shot, forward, backward, ping-pong, reverse), and ADSR amplitude envelopes.
- **DSP Chain**: Native per-track and master DSP rack (gain, pan, filter, delay, reverb, drive, bitcrusher, chorus, flanger, phaser, compressor, gate, limiter) shared by realtime playback and offline export.
- **Parameter Locks**: Row-scoped parameter overrides for sampler, mixer, and DSP parameters.

### 🔄 Interoperability & Export
- **File Support**: Standard MIDI File (SMF 0/1) import/export, XRNS inspection & import, MusicXML `score-partwise` interchange.
- **Audio Bouncing**: Render sequence or pattern selections to WAV PCM16, generate stems with manifests, or export Strudel browser live-coding sketches.

### 🤖 Generative & AI Tools
- **Generative Transforms**: Deterministic CLI algorithms (such as Euclidean rhythm generation).
- **AI Composition**: Reviewable local proposal engine (`:ai propose`, `:ai show`, `:ai accept`, `:ai reject`) with zero implicit network calls.

---

## TUI Commands & Keybindings

### Global Shortcuts

| Key / Shortcut | Action |
| :--- | :--- |
| `H` or `?` | Toggle contextual help overlay |
| `q` | Quit application (prompts if unsaved changes exist) |
| `Ctrl+S` | Save current project |
| `Ctrl+Shift+S` | Save As prompt |
| `Space` | Toggle play / stop |
| `Shift+Space` | Play pattern from start |
| `Enter` | Play from current row |
| `Shift+Enter` | Play sequence from selected position |
| `F8` | Stop playback |
| `L` | Toggle pattern loop |
| `F4` | Open MIDI settings |
| `F7` | Song slot / Sequence view |
| `F9` | Tracks view |
| `F10` | Patterns view |
| `Ctrl+J` | Sampler view |
| `Ctrl+P` | Command palette |
| `:` | Enter command mode |
| `:view roll` | Open the synchronized Piano Roll editor (`Esc` returns) |

### Navigation & Editing

| Key / Shortcut | Action |
| :--- | :--- |
| `Arrow Keys` / `h j k l` | Move cursor in grid |
| `Tab` / `Shift+Tab` | Next / previous track |
| `PageUp` / `PageDown` | Scroll rows by page |
| `Home` / `End` / `gg` / `G` | Jump to first / last row |
| `i` / `Esc` | Enter edit mode / normal mode |
| `z s x d c v...` | Play / enter notes from computer keyboard |
| `o` / `.` | Note-off / note-cut |
| `F1` / `F2` (or `-` / `+`) | Octave down / up |
| `Delete` | Clear current cell or active selection |
| `Insert` / `Ctrl+Delete` | Insert row / delete row |
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo |
| `v` (normal mode) | Browse and restore generated pattern variations |
| `V` (normal mode) | Start visual selection |
| `t` (normal mode) | Open live DSP calibration; arrows adjust, `r` resets, `t`/`Esc` closes |
| `e` (normal mode) | Edit the current project in `$EDITOR` and reload it on exit |
| `b` (normal mode) | Open the local browser companion visualizer ([details](docs/web-companion.md)) |

### Tracks, Patterns & Sequence

| Key / Shortcut | Action |
| :--- | :--- |
| `Ctrl+T` | Create new track |
| `D` | Duplicate current track |
| `{` / `}` | Move track left / right |
| `M` / `S` | Toggle mute / solo on current track |
| `N` / `P` / `X` | New pattern / duplicate pattern / delete pattern |
| `[` / `]` | Select previous / next pattern |
| `F3` / `F6` | Rename pattern / change pattern length |
| `A` / `Y` / `R` / `T` | Song slot: Add / Duplicate / Remove / Set pattern |
| `<` / `>` | Move selected song slot up / down |

### Mouse Controls

- **Left-Click**: Select or activate tracker cells, list rows, tabs, browser entries, overlays, sampler controls, or DSP parameters.
- **Right-Click**: Open a Pattern Manager row in tracker, play from a Sequence row, or assign a Sample Browser entry to the active track.
- **Scroll Wheel**: Scroll list/grid under pointer (horizontal wheel targets pattern grid, clip grid, or waveform).

### Layout Commands

```text
:layout compact|balanced|studio
:layout fields full|note|instrument|fx|note-instrument|note-fx|instrument-fx
:layout show|hide|toggle tracks|sequence|inspector|track-desk
:layout resize tracks|inspector|track-desk +/-N
```

---

## Command Mode Reference (`:`)

```text
# File Management
:write [PATH]                       Save project file
:saveas PATH                        Save project to a new path
:wq / :q!                           Save & quit / force quit

# Song & Tempo
:bpm BPM                            Set tempo (e.g. :bpm 140)
:lpb LPB                            Set lines per beat (e.g. :lpb 4)

# Tracker Cell & Effects
:fx CMD VAL                         Set FX1 command (e.g. :fx D 20, :fx R 04, :fx clear)
:fx2 CMD VAL                        Set FX2 command
:cell FIELD VAL                     Edit cell (field: instrument, volume, pan, delay, effect)

# MIDI Controls
:midi outputs / :midi connect ID    List & connect MIDI output
:midi panic                         Send All-Notes-Off / Reset
:midi import /path/to/song.mid      Import Standard MIDI file
:midi-input connect ID              Connect MIDI input device
:midi-input record on               Enable MIDI note recording

# Tracks & Patterns
:track new NAME / :track rename     Create / rename track
:track channel CH PORT              Set track MIDI channel
:pattern new / :pattern duplicate   Manage patterns
:pattern length LEN                 Set pattern length (e.g. :pattern length 64)

# Sampler & Audio
:web                                Open local browser companion
:sample view PATH                   Load WAV sample for inspection
:sample browse [DIR]                Open in-app sample browser
:sample assign [TRACK]              Assign loaded sample to track
:sample mode MODE                   Set playback mode (one-shot, forward, backward, pingpong, reverse)
:sample envelope A D S R            Set ADSR envelope
:sample render-selection PATH       Bounce selected cells to WAV sample

# Mixer & DSP Rack
:mixer gain TRACK VAL               Set track audio gain
:mixer pan TRACK VAL                Set track pan
:mixer master VAL                   Set master gain
:dsp track TRACK DEVICE ARGS...     Add/configure DSP device on track
:dsp master DEVICE ARGS...          Add/configure DSP device on master
:plock PARAM VAL                    Set parameter lock on current row

# AI & Guidance
:ai guidance apply NAME             Apply local guidance preset
:ai propose PROMPT                  Generate reviewable AI proposal
:ai show / :ai accept / :ai reject  Review and apply/discard proposal
```

---

## Tracker Mechanics & DSP

### Cell Format

Pattern cells are displayed as:

```text
NOTE VEL IN VOL PN DL FX
C-4  64 01 40 7F 20 R04
```

- **`NOTE` / `VEL`**: Drive MIDI note playback and sample trigger velocity.
- **`IN`**: Selects sample-backed instrument assignment.
- **`VOL`**: Scales sampler gain.
- **`PN`**: Overrides track mixer pan for the sample event.
- **`DL`**: Micro-delay offset within the row.
- **`FX`**: Tracker command (e.g., `Dxx` for row delay, `Rxx` for retrigger).

### Tracker FX vs Native DSP vs Parameter Locks

- **Tracker FX columns**: Per-cell commands stored directly in the pattern grid (`:fx` and `:fx2`).
- **Native DSP chains**: Audio device chains running per-track or on master (`:dsp` or DSP Rack view).
- **Parameter Locks**: Row-level parameter overrides (`:plock` or DSP Rack parameter editor).

```text
:sample view ~/Music/Samples/kick.wav
:sample assign 1
:dsp track 1 filter lowpass 2000 0.250 0.000 0.500
:dsp master reverb 0.500 20 2.500 0.250
:plock dsp track filter-cutoff 1200
```

---

## MIDI & Routing Setup

### macOS IAC Driver Setup

1. Open **Audio MIDI Setup** on macOS.
2. Choose **Window > Show MIDI Studio**.
3. Double-click **IAC Driver** and check **Device is online**.
4. Run `trk --list-midi-outputs` to confirm availability (`0: IAC Driver Bus 1`).
5. Launch `trk` with IAC config:
   ```bash
   trk --config config/iac-driver.toml --midi-log trk-midi.log
   ```

### Ableton Live Routing

1. Enable IAC Driver in Audio MIDI Setup.
2. In Live's **Preferences > Link, Tempo & MIDI**, enable **Track** for `IAC Driver (Bus 1)` under Input Ports.
3. Create a MIDI track in Live, set **MIDI From** to `IAC Driver (Bus 1)`, and arm the track.

### Renoise Routing

1. Open Renoise MIDI preferences and set `IAC Driver (Bus 1)` as input device.
2. Use Renoise MIDI monitor to inspect incoming raw note-on/note-off events.

### MIDI Debugging

```bash
trk --list-midi-outputs
trk --list-midi-inputs
trk --config config/iac-driver.toml --midi-test-output 0 --midi-test-channel 1 --midi-test-note 60
trk --config config/iac-driver.toml --midi-log trk-midi.log
tail -f trk-midi.log
```

---

## CLI & Interoperability

### CLI Syntax & Options

```bash
trk [OPTIONS] [FILE]
trk --list-midi-outputs
trk --list-midi-inputs
trk --midi-test-output NAME_OR_INDEX [OPTIONS]
trk transform euclidean INPUT OUTPUT [OPTIONS]
trk sample inspect FILE [OPTIONS]
trk import midi INPUT.mid OUTPUT.trk
trk import xrns INPUT OUTPUT [OPTIONS]
trk import musicxml INPUT.musicxml OUTPUT.trk
trk export plan INPUT [OUTPUT.json] [OPTIONS]
trk export audio INPUT OUTPUT.wav [OPTIONS]
trk export stems INPUT OUT_DIR [OPTIONS]
trk export strudel INPUT [OUTPUT.js] [OPTIONS]
trk export musicxml INPUT [OUTPUT.musicxml] [OPTIONS]
trk validate roundtrip INPUT [OUTPUT] [--format text|json]
trk analyze INPUT [OUTPUT] [--format text|json]
trk compare LEFT RIGHT [OUTPUT] [--format text|json]
```

### Import & Export Formats

- **Standard MIDI Files**: Import SMF 0/1, export SMF 0. Long MIDI files are automatically segmented into 64-row patterns.
- **XRNS & MusicXML**: Inspection & import for Renoise `.xrns` archives, plus `score-partwise` MusicXML interchange.
- **Strudel Export**: Export deterministic browser live-coding sketches preserving tempo, notes, velocity, and pattern arrangement.

### Audio Export & Stems

Bounce sample-backed songs to WAV PCM16 or multi-track stem archives:

```bash
trk export audio input.trk output.wav --sequence --sample-rate 48000 --channels 2
trk export stems input.trk stems/ --sequence
trk export plan input.trk plan.json --sequence
```

### Generative & AI Tools

Run Euclidean rhythm generation on target pattern tracks:

```bash
trk transform euclidean input.trk output.trk --pattern 1 --track 1 --steps 16 --pulses 5 --pitch 36
```

Generate style analysis or critique reports:

```bash
trk report project song.trk reports/project.md
trk report critique song.trk reports/critique.md
trk analyze song.trk reports/style.json --format json
```

---

## Development & Testing

Run code quality and test suites locally:

```bash
cargo fmt --all -- --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
scripts/test-rust-file-sizes.sh
scripts/check-rust-file-sizes.sh --top 12
python3 scripts/test_check_crate_dependencies.py
python3 scripts/check_crate_dependencies.py
```

See [`docs/timing.md`](docs/timing.md) for timing budget contracts and jitter test tolerances.

---

## Roadmap Gaps

`trk` is actively evolving and is not yet a full Renoise replacement. Current roadmap focus areas include:
- Sampler keyzones, velocity layers, and sample streaming.
- Voice allocation and choking groups.
- Plugin hosting (VST3 / AU / CLAP).
- Tracker effect column features beyond FX1/FX2.
- Realtime audio device selection in UI.
- Graphical automation and broader mixer metering views.

---

## License

This project is licensed under the [MIT License](LICENSE).
