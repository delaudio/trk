# Salieri Tracker

Salieri Tracker is a MIDI-first music tracker that runs in the terminal. The current app is a Rust workspace with a Ratatui/Crossterm TUI, pattern editing, sequence playback, project persistence, undo/redo, MIDI output through `midir`, sample inspection and assignment workflows, deterministic transforms, and the first internal audio/AI foundations.

The primary realtime playback path remains MIDI-first for external instruments, but assigned WAV samples can also play through the internal CPAL audio backend on the default output device. Internal audio is still early: sampler playback is intentionally minimal and does not yet include device selection, sustained loop playback, sends, or a full DSP device set.

## Current Capabilities

- Terminal tracker UI with pattern, track, sequence, sampler, MIDI, and help views.
- Pattern editing with keyboard note entry, note-off/note-cut, velocity, instrument, volume, pan, delay, effect columns, row insert/delete, selection copy/cut/paste/delete, undo/redo, and playhead follow.
- Track, pattern, and sequence management, including rename, duplicate, delete, move, mute/solo, pattern length, and arrangement playback.
- MIDI output routing with port listing, connection from the TUI, panic/all-notes-off, channel assignment, logging, and MIDI test-note CLI support.
- MIDI input port listing, command-mode connection, note-on recording into the current pattern, and basic MIDI clock start/continue/stop following.
- Project persistence as JSON `.salieri` files with validation and atomic writes.
- WAV sample loading, waveform inspection, in-app sample browser, external chooser integration, sample-backed instruments, track assignment, replacement, unassignment, unload, cleanup, frame windows, loop-point metadata, and ADSR-style amplitude envelopes.
- Realtime sampler playback for assigned WAV samples through the default CPAL output device.
- Mixer foundations with master gain, per-track audio gain/pan/mute/solo, track-editor display, and offline level metering helpers.
- Minimal native DSP graph with per-track and master gain/pan devices shared by realtime playback and offline export.
- Pattern automation lanes with stepped sample-gain automation observed by realtime playback and offline audio export.
- Deterministic sampler event contracts for routing assigned samples into audio commands.
- Offline audio rendering foundations for sampler preview/event buffers and WAV PCM16 encoding.
- Standard MIDI File format 0 import/export and lossy XRNS import for the supported subsets.
- Deterministic generative transform CLI, currently Euclidean rhythm generation.
- AI-assisted composition boundary with reviewable proposals and explicit apply semantics; no provider contacts external services implicitly.

## Requirements

- Rust stable
- macOS or Linux
- A terminal with alternate-screen support
- A MIDI destination for external instrument playback
- A default audio output device for internal assigned-sample playback

On macOS, use IAC Driver for a virtual MIDI cable. On Linux, use your preferred ALSA/JACK/PipeWire MIDI routing setup.

## Build And Run

```bash
cargo run
```

Open a project file:

```bash
cargo run -- song.salieri
```

Build a release binary:

```bash
cargo build --release
./target/release/salieri
```

Install locally from the workspace:

```bash
cargo install --path crates/salieri-app
salieri
```

## Test And Lint

```bash
cargo fmt --all -- --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

The GitHub Actions workflow runs the same checks on pushes to `main` and on pull requests.

Timing assumptions and jitter test limits are documented in [docs/timing.md](docs/timing.md).

## CLI

```bash
salieri [OPTIONS] [FILE]
salieri --list-midi-outputs
salieri --list-midi-inputs
salieri --midi-test-output NAME_OR_INDEX [OPTIONS]
salieri transform euclidean INPUT OUTPUT [OPTIONS]
salieri sample inspect FILE [OPTIONS]
salieri export audio INPUT OUTPUT [OPTIONS]
salieri --help
salieri --version
```

Useful options:

```bash
salieri --config config/iac-driver.toml
salieri --log-level debug
salieri --midi-log salieri-midi.log
salieri --list-midi-outputs
salieri --list-midi-inputs
salieri --midi-test-output 0 --midi-test-channel 1 --midi-test-note 60
salieri --midi-test-output "IAC Driver Bus 1" --midi-test-duration-ms 500
salieri sample inspect kick.wav --format text --buckets 64
salieri transform euclidean input.salieri output.salieri --pattern 1 --track 1 --steps 16 --pulses 5 --pitch 36
salieri export audio song.salieri song.wav --pattern 1
salieri export audio song.salieri song.wav --sequence --sample-rate 48000 --channels 2
```

`--midi-test-output` accepts either a port index or a port name. Configured MIDI output and input names are normalized, so `IAC Driver`, `IAC Driver Bus 1`, and `IAC Driver (Bus 1)` can match the same CoreMIDI port when available.

## macOS IAC Driver Setup

1. Open **Audio MIDI Setup**.
2. Choose **Window > Show MIDI Studio**.
3. Open **IAC Driver**.
4. Enable **Device is online**.
5. Add or select **Bus 1**.
6. Run:

```bash
salieri --list-midi-outputs
```

Expected output:

```text
0: IAC Driver Bus 1
```

Run Salieri with the included IAC config:

```bash
salieri --config config/iac-driver.toml --midi-log salieri-midi.log
```

Or copy the relevant settings into `~/.config/salieri/config.toml`:

```toml
[midi]
default_output = "IAC Driver Bus 1"
default_input = "IAC Driver Bus 1"
log_file = "salieri-midi.log"
```

## Ableton Live Routing

1. Enable IAC Driver in macOS as described above.
2. In **Live > Settings > Link, Tempo & MIDI**, enable **Track** for `IAC Driver (Bus 1)` under **Input Ports**.
3. Create a MIDI track.
4. Set **MIDI From** to `IAC Driver (Bus 1)`.
5. Choose either **All Channels** or the channel used by the Salieri track.
6. Arm the track or set monitor to **In**.
7. Load an instrument on the track.
8. Press `Space` in Salieri to play.

If Live shows MIDI activity but you hear nothing, check the track monitor, arming state, channel filter, and whether an instrument is loaded.

## Renoise Routing

1. Open Renoise MIDI preferences.
2. Select `IAC Driver (Bus 1)` as an input device.
3. Use the Renoise MIDI monitor to confirm incoming note events.
4. Assign the input to the current track or instrument as needed.

Renoise is also useful as a MIDI monitor because it shows the raw note-on and note-off events arriving from Salieri.

## MIDI Debugging

List ports:

```bash
salieri --list-midi-outputs
salieri --list-midi-inputs
```

Send a single note outside the TUI:

```bash
salieri --config config/iac-driver.toml --midi-test-output 0 --midi-test-channel 1 --midi-test-note 60
```

Log all MIDI messages sent by Salieri:

```bash
salieri --config config/iac-driver.toml --midi-log salieri-midi.log
tail -f salieri-midi.log
```

Example log lines:

```text
75160ms NOTE_OFF ch=1 note=61 velocity=0 bytes=80 3D 00
75160ms NOTE_ON ch=1 note=63 velocity=127 bytes=90 3F 7F
```

MIDI channel numbers in the log are user-facing `1..16`. Raw status bytes use the MIDI wire format, where channel 1 note-on is `0x90`.

MIDI input recording and transport sync are documented in [docs/midi-input-sync.md](docs/midi-input-sync.md).

## TUI Commands

Global:

```text
H or ?          Help
q               Quit
Ctrl+S          Save
Ctrl+Shift+S    Save As prompt with current path
Space           Play/stop
Shift+Space     Play pattern from start
Enter           Play from current row
Shift+Enter     Play sequence from selected position
F8              Stop
L               Toggle pattern loop
F4              MIDI settings
F7              Sequence view
F9              Tracks view
F10             Patterns view
F11             Sampler view
:               Command mode
```

Help:

```text
H or ?          Open help
Up/Down, j/k    Scroll help
PageUp/PageDown Scroll help by a larger step
Home/End        Jump to top/bottom of help
Esc, q, ?       Close help
```

Navigation:

```text
Arrows          Move cursor
h/j/k/l         Vim-style move, when enabled
Tab             Next track
Shift+Tab       Previous track
PageUp/PageDown Jump rows
Home/End        First/last row
gg/G            First/last row
```

Editing:

```text
i               Edit mode
Esc             Normal mode
z s x d c v...  Insert notes from computer keyboard
o               Note off
.               Note cut
Hex digits      Edit VEL/INST/VOL/PAN/DLY/FX value fields
F1 or -         Octave down
F2 or +         Octave up
Delete          Clear current cell or selection
Insert          Insert row
Ctrl+Delete     Delete row
Ctrl+Z          Undo
Ctrl+Y          Redo
```

Tracks, patterns, and sequence:

```text
Ctrl+T          New track
D               Duplicate track
{ / }           Move track left/right
M / S           Mute/solo current track
N / P / X       New, duplicate, delete pattern
[ / ]           Previous/next pattern
F3              Rename pattern command
F6              Pattern length command
A               Add current pattern to sequence
Y               Duplicate sequence position
R               Remove sequence position
T               Set sequence position to current pattern
< / >           Move sequence position up/down
```

## Command Mode Examples

```text
:write
:write song.salieri
:saveas song.salieri
:wq
:q!
:bpm 140
:lpb 4
:fx D 20
:fx R 04
:fx clear
:cell instrument 01
:cell volume 40
:cell pan 7f
:cell delay 20
:cell effect R 04
:cell volume clear
:midi outputs
:midi connect 0
:midi disconnect
:midi panic
:midi-input ports
:midi-input connect 0
:midi-input record on
:midi-input clock on
:midi-input disconnect
:track new Acid
:track rename Bass
:track channel 2 10
:track duplicate 2
:track move 2 3
:pattern new
:pattern duplicate
:pattern rename Intro
:pattern length 128
:sequence add
:sequence set 0 2
:sequence move 1 0
:play pattern
:play sequence 0
:sample view path/to/sample.wav
:sample browse path/to/samples
:sample choose path/to/samples
:sample assign
:sample replace 2
:sample unassign 2
:sample start 1200
:sample end 48000
:sample loop 2400 12000
:sample loop off
:sample envelope 0.005 0.040 0.800 0.080
:sample unload
:sample cleanup
:sample assignments
:automation sample-gain 4 0.750
:automation sample-gain clear 4
:mixer gain 2 0.750
:mixer pan 2 -0.250
:mixer mute 2
:mixer solo 2
:mixer master 0.900
:dsp track 2 gain 0.500
:dsp track 2 pan -0.250
:dsp master gain 0.800
:dsp track 2 clear
:ai propose sparse bass sketch
:ai show
:ai accept
:ai reject
:stop
```

## Tracker Columns

Pattern cells render as:

```text
NOTE VEL IN VOL PN DL FX
C-4  64 01 40 7F 20 R04
```

`NOTE` and `VEL` continue to drive MIDI note playback. `INST`, `VOL`, `PN`, and `DL` are optional tracker metadata columns for richer sampler-backed playback: `INST` selects a sample-backed instrument for that cell, `VOL` scales sampler gain, `PN` overrides mixer pan for the sampler event, and `DL` offsets the event within the row. `FX` stores the first tracker command; delay (`Dxx`) and retrigger (`Rxx`) remain supported, and `DL` takes precedence over `Dxx` when both are present.

Move horizontally through sub-columns with Left/Right. In edit mode, type two hex digits on value columns to enter a value. Command mode can also edit the current cell with `:cell instrument|volume|pan|delay|effect VALUE` and clear fields with `:cell FIELD clear`.

## Samples And Audio

Salieri can inspect and load WAV samples, render waveform overviews, and persist sample references in `.salieri` projects. Supported sample loading currently covers 16-bit PCM and 32-bit float RIFF/WAVE files.

Inside the tracker:

```text
F11                         Open sampler view
:sample view PATH           Load a WAV reference for waveform inspection
:sample browse [DIR]        Open the in-app sample browser
:sample choose [DIR]        Launch a configured external chooser
:sample assign [TRACK]      Assign the loaded sample to a track
:sample replace [TRACK]     Replace a track assignment
:sample unassign [TRACK]    Remove a track assignment
:sample start FRAME|clear   Set or clear the sample start frame
:sample end FRAME|clear     Set or clear the sample end frame
:sample loop START END|off  Set or clear loop-point metadata
:sample envelope A D S R    Set attack/decay/sustain/release
:sample settings            Show playback settings for the loaded sample
:sample cleanup             Remove unused sample references
```

Assigned samples are routed into the internal realtime audio command boundary during playback and rendered through the default CPAL output device. Samples are sliced by start/end frame, shaped by the configured envelope, and prepared for the output sample rate and channel count before playback/export. Loop points are persisted and displayed but are not yet rendered as sustained loop playback. See [docs/sampler.md](docs/sampler.md), [docs/audio-engine.md](docs/audio-engine.md), and [docs/audio-export.md](docs/audio-export.md).

## Generative And AI Foundations

Deterministic transforms live in `salieri-transform` and can be used from the CLI today:

```bash
salieri transform euclidean input.salieri output.salieri --pattern 1 --track 1 --steps 16 --pulses 5 --rotation 0 --pitch 36 --velocity 100
```

AI-assisted composition lives behind `salieri-ai` and is available in the TUI through the local deterministic provider: `:ai propose PROMPT`, `:ai show`, `:ai accept`, and `:ai reject`. Proposals are previewed as touched cells before application, and accepted proposals use the normal undo stack. No network provider is invoked implicitly. See [docs/generative-transforms.md](docs/generative-transforms.md) and [docs/ai-assisted-edits.md](docs/ai-assisted-edits.md).

## Interoperability

`salieri-interop` supports a narrow Standard MIDI File format 0 subset for import/export, plus XRNS inspection and a minimal lossy XRNS importer for the constrained subset documented below. Legacy module formats such as MOD, XM, IT, and S3M remain explicit unsupported song-import formats; the current probe is limited to metadata/effect diagnostics and MOD sample extraction.

```bash
salieri import xrns input.xrns output.salieri
```

See [docs/interoperability.md](docs/interoperability.md).

## Audio Export

Assigned-sample playback can be rendered offline to WAV PCM16:

```bash
salieri export audio input.salieri output.wav --pattern 1
salieri export audio input.salieri output.wav --sequence --sample-rate 48000 --channels 2
```

The exporter renders sampler events only. MIDI-only external instruments are not captured in the WAV file. Tracker instrument/volume/pan/delay columns, stepped sample-gain automation, mixer gain/pan, and native DSP gain/pan chains are applied through the same sampler event path used by realtime playback. See [docs/audio-export.md](docs/audio-export.md), [docs/automation.md](docs/automation.md), and [docs/mixer.md](docs/mixer.md).

## Project Files

Salieri saves JSON projects with the `.salieri` extension. The file contains a `formatVersion` and a serializable song model so projects can be versioned in Git.

The app tracks dirty state. Quitting with unsaved changes prompts for save, discard, or cancel.

## Roadmap Gaps

Salieri is not yet a full Renoise-class workstation. The largest missing product areas are keyzones/velocity layers, sustained sampler loop playback and choking, multiple effect columns with DSP device parameter mapping, a broader DSP device set, sends/routing, graphical mixer and automation views, richer MIDI input mapping/quantization, and external-provider AI integration.
