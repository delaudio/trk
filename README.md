# trk

trk is a MIDI-first music tracker that runs in the terminal. The Rust workspace combines a Ratatui/Crossterm interface with pattern and sequence editing, project persistence, MIDI input/output, sample-backed playback, native audio DSP, deterministic transforms, and reviewable AI-assisted edits.

External instruments remain a first-class realtime path through MIDI. Assigned WAV samples can also play through the internal CPAL backend and the same mixer/DSP graph used by offline audio export. Audio-device selection, sample choking, keyzones, velocity layers, and plugin hosting are not implemented yet.

## Current Capabilities

- Responsive terminal workspace with Pattern, Sequence, Clips, Tracks, Patterns, Sampler, DSP Rack, Sample Browser, Project Browser, MIDI Settings, AI Chat, command palette, and help views.
- Mouse navigation for rendered tracker cells, list rows, browser entries, transport actions, overlays, sampler controls, and DSP controls. Wheel input is routed to the region under the pointer, and unavailable controls are visibly disabled.
- Pattern editing with keyboard note entry, note-off/note-cut, velocity, instrument, volume, pan, delay, effect columns, row insert/delete, selection copy/cut/paste/delete, undo/redo, and playhead follow.
- Optional project, pattern-row, lyric, and cue text annotations that persist without affecting playback.
- Track, pattern, and sequence management, including rename, duplicate, delete, move, mute/solo, pattern length, and arrangement playback.
- MIDI output routing with port listing, connection from the TUI, panic/all-notes-off, channel assignment, logging, and MIDI test-note CLI support.
- MIDI input port listing, command-mode connection, note-on recording into the current pattern, and basic MIDI clock start/continue/stop following.
- Project persistence as JSON `.trk` files with validation and atomic writes.
- WAV sample loading, waveform inspection, in-app sample browser, external chooser integration, sample-backed instruments, mouse/command assignment, replacement, unassignment, unload, cleanup, frame windows, loop-point metadata, and ADSR-style amplitude envelopes.
- Realtime and offline sampler playback with one-shot, forward-loop, backward-loop, ping-pong-loop, and reverse modes.
- Mixer foundations with master gain, per-track audio gain/pan/mute/solo, delay/reverb sends, track-editor display, and offline level metering helpers.
- Native per-track and master DSP chains shared by realtime playback and offline export. The current device palette includes gain, pan, balance, stereo width, phase invert, filter, delay, reverb, drive, bitcrusher, chorus, flanger, phaser, compressor, gate, and limiter.
- Row-scoped parameter locks for supported sampler, mixer, and native DSP parameters.
- Optional C/C++ DSP wrapper boundary for reviewed native modules, with a feature-gated C gain proof of concept and deterministic tests. See [docs/c-dsp-boundary.md](docs/c-dsp-boundary.md).
- WebAssembly DSP ABI evaluation for browser/Web Audio export, with host-side validation tests and terminal realtime execution deferred. See [docs/wasm-dsp-evaluation.md](docs/wasm-dsp-evaluation.md).
- Faust DSP evaluation for optional generated native modules, with UI metadata mapped into trk native descriptors. See [docs/faust-dsp-evaluation.md](docs/faust-dsp-evaluation.md).
- RNBO interoperability evaluation for C++ source and web export boundaries, with opaque RNBO runtime state excluded from project files. See [docs/rnbo-evaluation.md](docs/rnbo-evaluation.md).
- Pattern automation lanes with stepped sample-gain automation observed by realtime playback and offline audio export.
- Deterministic sampler event contracts for routing assigned samples into audio commands.
- Offline audio rendering foundations for sampler preview/event buffers and WAV PCM16 encoding.
- Standard MIDI File format 0 and 1 import, format 0 export at the interoperability-library boundary, and lossy MusicXML/XRNS interchange for the documented subsets. Long MIDI imports are split into 64-row patterns and added to the sequence.
- Deterministic generative transform CLI, currently Euclidean rhythm generation.
- AI-assisted composition boundary with reviewable proposals and explicit apply semantics; no provider contacts external services implicitly.

## Requirements

- Rust stable
- macOS or Linux
- A terminal with alternate-screen support
- A MIDI destination when using external instrument playback
- A default audio output device when using internal assigned-sample playback

On macOS, use IAC Driver for a virtual MIDI cable. On Linux, use your preferred ALSA/JACK/PipeWire MIDI routing setup.

## Quick Start

```bash
cargo run
```

Open an existing project or the included foundation fixture:

```bash
cargo run -- song.trk
cargo run -- fixtures/projects/foundations.trk
```

Inside the TUI, press `H` for contextual help or `Ctrl+P` for the command palette. To import a MIDI file directly, choose **Import MIDI...** from the palette or enter:

```text
:midi import /path/to/song.mid
```

The imported song replaces the current in-memory project, is marked dirty, and is not written until you save it. MIDI format 0 and format 1 files are accepted for the supported event subset; longer songs are divided into consecutive 64-row patterns.

Build a release binary:

```bash
cargo build --release
./target/release/trk
```

Install locally from the workspace:

```bash
cargo install --path crates/trk-app
trk
```

## Test And Lint

```bash
cargo fmt --all -- --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
scripts/test-rust-file-sizes.sh
scripts/check-rust-file-sizes.sh --top 12
python3 scripts/test_check_crate_dependencies.py
python3 scripts/check_crate_dependencies.py
```

GitHub Actions runs formatting, Rust file-size budgets, crate dependency boundaries, tests, and Clippy on pushes to `main` and on pull requests.

Timing assumptions and jitter test limits are documented in [docs/timing.md](docs/timing.md).

## CLI

```bash
trk [OPTIONS] [FILE]
trk --list-midi-outputs
trk --list-midi-inputs
trk --midi-test-output NAME_OR_INDEX [OPTIONS]
trk transform euclidean INPUT OUTPUT [OPTIONS]
trk sample inspect FILE [OPTIONS]
trk import xrns INPUT OUTPUT [OPTIONS]
trk import midi INPUT.mid OUTPUT.trk
trk import musicxml INPUT.musicxml OUTPUT.trk
trk export plan INPUT [OUTPUT.json] [OPTIONS]
trk export audio INPUT OUTPUT.wav [OPTIONS]
trk export stems INPUT OUT_DIR [OPTIONS]
trk export strudel INPUT [OUTPUT.js] [OPTIONS]
trk export musicxml INPUT [OUTPUT.musicxml] [OPTIONS]
trk validate roundtrip INPUT [OUTPUT] [--format text|json]
trk report project INPUT [OUTPUT.md]
trk report critique INPUT [OUTPUT.md]
trk analyze INPUT [OUTPUT] [--format text|json]
trk compare LEFT RIGHT [OUTPUT] [--format text|json]
trk graph validate GRAPH.json
trk graph compile GRAPH.json INPUT.trk OUTPUT.trk
trk --help
trk --version
```

Useful options:

```bash
trk --config config/iac-driver.toml
trk --log-level debug
trk --midi-log trk-midi.log
trk --list-midi-outputs
trk --list-midi-inputs
trk --midi-test-output 0 --midi-test-channel 1 --midi-test-note 60
trk --midi-test-output "IAC Driver Bus 1" --midi-test-duration-ms 500
trk sample inspect kick.wav --format text --buckets 64
trk transform euclidean input.trk output.trk --pattern 1 --track 1 --steps 16 --pulses 5 --pitch 36
trk import midi song.mid song.trk
trk import xrns song.xrns song.trk --sample-dir samples/song
trk import musicxml score.musicxml score.trk
trk export plan song.trk plan.json --sequence
trk export audio song.trk song.wav --pattern 1
trk export audio song.trk song.wav --sequence --sample-rate 48000 --channels 2
trk export stems song.trk stems/ --sequence
trk export strudel song.trk song.js --patterns 1,2
trk export musicxml song.trk score.musicxml --pattern 1
trk validate roundtrip song.trk validation.json --format json
trk report project song.trk reports/project.md
trk report critique song.trk reports/critique.md
trk analyze song.trk reports/style.json --format json
trk compare draft.trk final.trk reports/compare.txt
trk graph validate arrangement.graph.json
trk graph compile arrangement.graph.json song.trk arranged.trk
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
trk --list-midi-outputs
```

Expected output:

```text
0: IAC Driver Bus 1
```

Run trk with the included IAC config:

```bash
trk --config config/iac-driver.toml --midi-log trk-midi.log
```

See [Configuration](docs/configuration.md) for lookup precedence, all preference
sections, validation rules, and the `:config` status command.
For local Renoise demo parity checks, see
[docs/renoise-parity.md](docs/renoise-parity.md).
For Polyend Tracker Mini workflow parity, see
[docs/tracker-mini-parity.md](docs/tracker-mini-parity.md).
For source-monorepo parity, see
[docs/source-monorepo-parity.md](docs/source-monorepo-parity.md).

Or copy the relevant settings into `~/.config/trk/config.toml`:

```toml
[midi]
default_output = "IAC Driver Bus 1"
default_input = "IAC Driver Bus 1"
log_file = "trk-midi.log"
```

## Ableton Live Routing

1. Enable IAC Driver in macOS as described above.
2. In **Live > Settings > Link, Tempo & MIDI**, enable **Track** for `IAC Driver (Bus 1)` under **Input Ports**.
3. Create a MIDI track.
4. Set **MIDI From** to `IAC Driver (Bus 1)`.
5. Choose either **All Channels** or the channel used by the trk track.
6. Arm the track or set monitor to **In**.
7. Load an instrument on the track.
8. Press `Space` in trk to play.

If Live shows MIDI activity but you hear nothing, check the track monitor, arming state, channel filter, and whether an instrument is loaded.

## Renoise Routing

1. Open Renoise MIDI preferences.
2. Select `IAC Driver (Bus 1)` as an input device.
3. Use the Renoise MIDI monitor to confirm incoming note events.
4. Assign the input to the current track or instrument as needed.

Renoise is also useful as a MIDI monitor because it shows the raw note-on and note-off events arriving from trk.

## MIDI Debugging

List ports:

```bash
trk --list-midi-outputs
trk --list-midi-inputs
```

Send a single note outside the TUI:

```bash
trk --config config/iac-driver.toml --midi-test-output 0 --midi-test-channel 1 --midi-test-note 60
```

Log all MIDI messages sent by trk:

```bash
trk --config config/iac-driver.toml --midi-log trk-midi.log
tail -f trk-midi.log
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
F7              Song slot / sequence view
F9              Tracks view
F10             Patterns view
Ctrl+J          Sampler view
Ctrl+P          Command palette
:               Command mode
```

### Mouse Controls

Mouse capture is enabled while the TUI is running. The pointer follows the same commands and validation paths as keyboard input:

- Left-click a visible tracker cell, row, tab, browser entry, overlay action, sampler control, or DSP control to select or activate it.
- Right-click a Pattern Manager row to open it in the tracker, a Sequence row to play from that position, or a supported Sample Browser entry to load and assign it to the current track.
- In the DSP Rack, left-click selects targets, devices, parameters, and palette entries. Right-clicking a parameter selects it and increases it by one edit step.
- The wheel scrolls the list or grid under the pointer. Horizontal wheel input is limited to the pattern grid, clip grid, and loaded sampler waveform.
- Modal overlays capture pointer input so clicks and wheel events do not fall through to the workspace below.
- Controls marked with `×` and dim styling are placeholders and are intentionally not interactive. Drag editing and context menus are not currently supported.

Keyboard navigation remains available for every workflow. Terminal mouse reporting must be enabled; most modern terminal emulators support it by default.

Layout commands:

```text
:layout compact|balanced|studio
:layout fields full|note|instrument|fx|note-instrument|note-fx|instrument-fx
:layout show|hide|toggle tracks|sequence|inspector|track-desk
:layout resize tracks|inspector|track-desk +/-N
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
A               Add current pattern to song slots
Y               Duplicate selected song slot
R               Remove selected song slot
T               Set selected song slot to current pattern
< / >           Move selected song slot up/down
```

## Command Mode Examples

```text
:write
:write song.trk
:saveas song.trk
:wq
:q!
:bpm 140
:lpb 4
:fx D 20
:fx R 04
:fx clear
:fx2 R 02
:fx2 clear
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
:midi import /path/to/song.mid
:midi-input ports
:midi-input connect 0
:midi-input record on
:midi-input clock in on
:midi-input transport in on
:midi-input notes in on
:midi-input channel in 1,10
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
:pattern fill
:pattern copy
:pattern paste
:pattern invert
:pattern expand
:pattern shrink
:pattern duplicate-selection
:note project Arrangement sketch
:note pattern 16 Verse lyric/cue
:note lyric pattern 24 Words aligned to row
:note cue sequence 0 Intro
:note report
:sequence add
:sequence set 0 2
:sequence move 1 0
:clips
:clip add
:clip set 0 2 1
:clip launch scene 0
:clip commit
:clip stop
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
:sample render-selection bounces/loop.wav --assign 2
:sample recorder inputs
:sample recorder capture 48000
:sample recorder trim 1200 36000
:sample recorder save-load recordings/take.wav --assign 2
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
:mixer send delay
:mixer send delay gain 2 0.350
:mixer send delay pre
:mixer send reverb
:dsp track 2 gain 0.500
:dsp track 2 pan -0.250
:dsp track 2 filter lowpass 2000 0.250 0.000 0.500
:dsp master reverb 0.500 20 2.500 0.250
:dsp master gain 0.800
:dsp track 2 clear
:plock dsp track filter-cutoff 1200
:plock dsp track filter-cutoff reset
:plock dsp track filter-cutoff clear
:ai guidance apply dub-techno
:ai propose sparse bass sketch
:ai show
:ai accept
:ai reject
:preset inventory
:preset save ./profiles/current.json
:preset instrument save ./profiles/kick.instrument.json
:preset instrument load ./profiles/kick.instrument.json
:performance slot 1 track 2 gain 0.500
:performance punch 1
:performance release 1
:workspace init ~/Music/trk
:workspace index ~/Music/trk
:tasks
:task cancel 1
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

### Tracker FX Columns vs Native DSP Chains

trk has two separate effect concepts:

- Tracker FX columns are per-cell commands stored in the pattern grid. Use `:fx` for the first FX column and `:fx2` for the second FX column, for example `:fx D 20` for row delay or `:fx R 04` for retrigger.
- Native DSP chains are audio processors on a track or on the master bus. Use `:dsp` or the DSP rack view to add devices such as gain, filter, delay, reverb, drive, modulation, dynamics, and limiter devices.
- Parameter locks are row-scoped overrides. Use `:plock` or the DSP rack parameter editor (`P` lock, `R` reset, `C` clear) to change one DSP/mixer/sample parameter on the current tracker row.

Practical sample-backed workflow:

```text
:sample view ~/Music/Samples/kick.wav
:sample assign 1
:dsp track 1 filter lowpass 2000 0.250 0.000 0.500
:dsp master reverb 0.500 20 2.500 0.250
# Open the DSP rack with :focus dsp, select Filter Cutoff, tweak with Left/Right.
:plock dsp track filter-cutoff 1200
```

## Samples And Audio

trk can inspect and load WAV samples, render waveform overviews, and persist sample references in `.trk` projects. Supported sample loading currently covers 16-bit PCM and 32-bit float RIFF/WAVE files.

Inside the tracker:

```text
Ctrl+J                      Open sampler view
:sample view PATH           Load a WAV reference for waveform inspection
:sample browse [DIR]        Open the in-app sample browser
:sample choose [DIR]        Launch a configured external chooser
:sample assign [TRACK]      Assign the loaded sample to a track
:sample replace [TRACK]     Replace a track assignment
:sample unassign [TRACK]    Remove a track assignment
:sample start FRAME|clear   Set or clear the sample start frame
:sample end FRAME|clear     Set or clear the sample end frame
:sample loop [backward|pingpong] START END|off
:sample mode MODE           one-shot, forward-loop, backward-loop, pingpong-loop, reverse
:sample envelope A D S R    Set attack/decay/sustain/release
:sample recorder inputs     List available system audio inputs
:sample recorder capture FRAMES [DEVICE_ID]
:sample recorder trim START END
:sample recorder save PATH
:sample recorder save-load PATH [--assign TRACK]
:sample settings            Show playback settings for the loaded sample
:sample cleanup             Remove unused sample references
```

Assigned samples are routed into the internal realtime audio command boundary during playback and rendered through the default CPAL output device. Samples are sliced by start/end frame, shaped by the configured envelope, and prepared for the output sample rate and channel count before playback/export. Forward, backward, ping-pong, and reverse playback modes are shared by realtime and offline rendering. `:sample render-selection PATH [--assign TRACK]` bounces the active tracker selection to a WAV sample reference and can assign it immediately. `:sample recorder capture FRAMES [DEVICE_ID]` records a bounded WAV from a system audio input when one is available; `trim`, `save`, and `save-load` crop, persist, and load the take into the sampler. See [docs/sampler.md](docs/sampler.md), [docs/audio-engine.md](docs/audio-engine.md), and [docs/audio-export.md](docs/audio-export.md).

Performance punch-ins are temporary runtime overrides. `:performance slot SLOT [track TRACK] gain|pan|sample-gain VALUE` configures a slot, `:performance punch SLOT` applies it to the playback clone, and `:performance release SLOT` restores the saved project state.

## Generative And AI Foundations

Deterministic transforms live in `trk-transform` and can be used from the CLI today:

```bash
trk transform euclidean input.trk output.trk --pattern 1 --track 1 --steps 16 --pulses 5 --rotation 0 --pitch 36 --velocity 100
```

AI-assisted composition lives behind `trk-ai` and is available in the TUI through the local deterministic provider: `:ai propose PROMPT`, `:ai show`, `:ai accept`, and `:ai reject`. Optional local guidance files from `[ai].guidance_dirs` can be listed, inspected, and applied with `:ai guidance list/show/apply/clear` before proposing. Project reports and critique reports can be generated with `:report project`, `:report critique`, or saved under workspace reports with `:report critique workspace ROOT`; `:revise PROMPT` turns the current critique context into a reviewable AI proposal. Composition graph drafts can be reviewed with `:graph draft PROMPT`, `:graph show`, `:graph reject`, and `:graph apply`, while CLI graph files can be validated and compiled deterministically into tracker sequence slots. Proposals are previewed before application, and accepted proposals use the normal undo transaction history. No network provider is invoked implicitly. Preset inventory profiles can also be saved and loaded as AI guidance with `:preset save` and `:preset load`. See [docs/generative-transforms.md](docs/generative-transforms.md), [docs/ai-assisted-edits.md](docs/ai-assisted-edits.md), [docs/report-workflows.md](docs/report-workflows.md), [docs/composition-graph.md](docs/composition-graph.md), [docs/preset-inventory.md](docs/preset-inventory.md), and [docs/undo-history.md](docs/undo-history.md).

Keyboard commands can be overridden per application mode while all unmapped shortcuts retain their defaults. See [docs/keymaps.md](docs/keymaps.md) for the available layers, key syntax, and conflict diagnostics.

Workspace manifests provide a portable root for projects, samples, preset profiles, reports, and guidance files. Use `:workspace init ROOT`, `:workspace index ROOT`, and non-destructive `:workspace trash/restore` operations to manage local artifacts. See [docs/workspace-manifest.md](docs/workspace-manifest.md).

## Interoperability

`trk-interop` imports a narrow Standard MIDI File format 0/1 subset and exports format 0 at the library boundary. It also supports a MusicXML `score-partwise` subset for notation interchange, XRNS inspection, and a minimal lossy XRNS importer. Legacy module formats such as MOD, XM, IT, and S3M remain explicit unsupported song-import formats; the current probe is limited to metadata/effect diagnostics and MOD sample extraction.

```bash
trk import midi input.mid output.trk
trk import xrns input.xrns output.trk
trk import xrns input.xrns output.trk --sample-dir fixtures/local/samples/demo --sample-path-prefix samples/demo
trk import musicxml score.musicxml score.trk
trk export musicxml score.trk score.musicxml --pattern 1
trk validate roundtrip score.trk report.json --format json
```

See [docs/interoperability.md](docs/interoperability.md).

## Audio Export

Assigned-sample playback can be rendered offline to WAV PCM16:

```bash
trk export audio input.trk output.wav --pattern 1
trk export audio input.trk output.wav --sequence --sample-rate 48000 --channels 2
trk export plan input.trk plan.json --pattern 1 --tracks 1,2
trk export stems input.trk stems/ --sequence
trk export strudel input.trk strudel.js --sequence
```

The exporter renders sampler events only. MIDI-only external instruments are not captured in the WAV file. Tracker instrument/volume/pan/delay columns, stepped sample-gain automation, mixer gain/pan, delay/reverb sends, and native DSP chains are applied through the same sampler event path used by realtime playback. Render plans can be inspected as JSON before writing audio, and stem exports write per-track WAV files plus a manifest. See [docs/audio-export.md](docs/audio-export.md), [docs/automation.md](docs/automation.md), and [docs/mixer.md](docs/mixer.md).

`export strudel` writes or prints a deterministic browser live-coding sketch for selected patterns or the song sequence. It preserves tempo, track comments, pattern lengths, notes, velocity, and simple volume/pan columns, with unsupported sampler, mixer, clip, and tracker-effect features listed as diagnostics. Clip launcher scenes can be edited locally with `:clips` and `:clip ...` commands; Ableton push/pull/clear dry-run plans are available with `:ableton ...`. See [docs/strudel-export.md](docs/strudel-export.md), [docs/clip-launcher.md](docs/clip-launcher.md), and [docs/ableton-live-bridge.md](docs/ableton-live-bridge.md).

`analyze` and `compare` produce deterministic style/profile reports for one or two projects in text or JSON. The TUI equivalents are `:analyze` and `:compare PATH`; see [docs/style-analysis.md](docs/style-analysis.md).

## Project Files

trk saves JSON projects with the `.trk` extension. The file contains a `formatVersion` and a serializable song model so projects can be versioned in Git.
Optional text annotations are saved with the song and can be reviewed with `:note list` or `:note report`; see [docs/text-annotations.md](docs/text-annotations.md).

The app tracks dirty state. Quitting with unsaved changes prompts for save, discard, or cancel.

## Roadmap Gaps

trk is not yet a full Renoise-class workstation. The largest missing product areas are sampler keyzones and velocity layers, voice allocation/choking, loop crossfades, streaming for large samples, audio-device selection, plugin hosting, tracker effect columns and semantics beyond the current FX1/FX2 subset, graphical mixer/send/automation and realtime meter views, richer MIDI input mapping and quantization, and implemented external-provider AI adapters.
