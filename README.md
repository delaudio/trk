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

## CLI

```bash
salieri [OPTIONS] [FILE]
salieri --list-midi-outputs
salieri --midi-test-output NAME_OR_INDEX [OPTIONS]
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
```

`--midi-test-output` accepts either a port index or a port name. Configured MIDI output names are normalized, so `IAC Driver`, `IAC Driver Bus 1`, and `IAC Driver (Bus 1)` can match the same CoreMIDI port when available.

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
Space           Play/stop
Shift+Space     Play pattern from start
Enter           Play from current row
Shift+Enter     Play sequence from selected position
F8              Stop
L               Toggle pattern loop
F4              MIDI settings
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

## Command Mode Examples

```text
:write
:write song.salieri
:saveas song.salieri
:wq
:q!
:bpm 140
:lpb 4
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
:play pattern
:play sequence 0
:stop
```

## Project Files

Salieri saves JSON projects with the `.salieri` extension. The file contains a `formatVersion` and a serializable song model so projects can be versioned in Git.

The app tracks dirty state. Quitting with unsaved changes prompts for save, discard, or cancel.
