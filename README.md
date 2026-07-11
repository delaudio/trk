# Salieri Tracker

Salieri Tracker is a MIDI-first music tracker that runs in the terminal. The current MVP is a Rust workspace with a Ratatui/Crossterm TUI, pattern editing, sequence playback, project persistence, undo/redo, and MIDI output through `midir`.

The app does not include an internal audio engine yet. It sends MIDI notes to a DAW, a virtual MIDI bus, or an external synth.

## Requirements

- Rust stable
- macOS or Linux
- A terminal with alternate-screen support
- A MIDI destination for playback

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
salieri --midi-test-output NAME_OR_INDEX [OPTIONS]
salieri analyze INPUT [--format json|markdown] [--output PATH]
salieri compare LEFT RIGHT [--format json|markdown] [--output PATH]
salieri --help
salieri --version
```

Useful options:

```bash
salieri --config config/iac-driver.toml
salieri --log-level debug
salieri --midi-log salieri-midi.log
salieri --list-midi-outputs
salieri --midi-test-output 0 --midi-test-channel 1 --midi-test-note 60
salieri --midi-test-output "IAC Driver Bus 1" --midi-test-duration-ms 500
salieri analyze song.salieri --format markdown --output song-analysis.md
salieri analyze song.salieri --format json --output song-profile.json
salieri compare song-a.salieri song-b.salieri --format markdown --output comparison.md
salieri compare song-a.salieri song-profile.json --format json --output comparison.json
salieri stems scan ./audio/stems stems.json
salieri interop validate-midi song.salieri
salieri render-chain song.salieri render-chain.json --sample-rate 48000 --channels 2 --bit-depth 24
salieri transform humanize song.salieri song-human.salieri --pattern 1 --track 1 --seed 42 --velocity 8 --delay 32
salieri transform humanize song.salieri song-human.salieri --pattern 1 --seed 42 --dry-run
salieri transform variation song.salieri song-var.salieri --pattern 1 --seed 7 --thin 10 --fill 5 --transpose 12 --name "Lead Variation"
```

`--midi-test-output` accepts either a port index or a port name. Configured MIDI output names are normalized, so `IAC Driver`, `IAC Driver Bus 1`, and `IAC Driver (Bus 1)` can match the same CoreMIDI port when available.

`analyze` produces deterministic, local project profiles: track role guesses, note density, rhythm and pitch-class profiles, pitch range, sequence energy, scene energy, and generation guidance. `compare` accepts either `.salieri` projects or JSON profiles produced by `analyze`.

`stems scan` walks a local folder and writes a stable JSON manifest for supported audio files (`wav`, `aif`, `aiff`, `flac`, `mp3`, `ogg`, `m4a`). Entries include source path, display name, role, group, order, file size, and modified time. Projects may reference a manifest and individual stem entries as optional metadata; missing manifests warn when a project is opened but do not make the project invalid.

`interop validate-midi` exports the selected project subset to Standard MIDI, imports it back, and reports preserved/lost note counts. The interop crate supports MIDI format 0 and format 1 note import, pattern/sequence/clip/scene MIDI export targets, plus explicit lightweight MusicXML and Renoise Song.xml subset helpers. `.salieri` remains the canonical lossless format.

`render-chain` writes a versioned JSON plan for future render workers and audio-engine integrations. It describes source project metadata, render format, tracker MIDI tracks, optional external stem references, mix defaults, master metadata, and target output paths without loading plugins or starting a realtime backend.

`transform humanize` changes note velocities and writes tracker delay commands (`Dxx`) on note cells. Playback interprets `D` as a sub-row offset, so humanize keeps notes on their original rows while shifting their timing inside those rows. `transform variation` duplicates a source pattern, then applies deterministic thin/fill/transpose operations to the duplicate. Use `--seed` for repeatable output and `--dry-run` to print the summary without writing the output project.

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
F11             Clip/session view
:               Command mode
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

Clip/session view:

```text
Arrows          Move scene or track selection
Enter           Launch selected clip
Shift+Enter     Launch selected scene
Space           Play/stop selected scene
F11 or Esc      Return to pattern view
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
:midi outputs
:midi connect 0
:midi disconnect
:midi panic
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
:transform humanize seed 42 velocity 8 delay 32
:transform variation seed 7 thin 10 fill 5 transpose 12 name Lead Variation
:play pattern
:play sequence 0
:play clip 1
:play scene 1
:stop
```

## Project Files

Salieri saves JSON projects with the `.salieri` extension. The file contains a `formatVersion` and a serializable song model so projects can be versioned in Git.

The app tracks dirty state. Quitting with unsaved changes prompts for save, discard, or cancel.
