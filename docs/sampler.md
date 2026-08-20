# Sampler Foundation

The sampler work is post-MVP and intentionally lives outside `trk-core`, `trk-midi`, and `trk-tui`.

`trk-sampler` currently provides:

- WAV loading for 16-bit PCM and 32-bit float RIFF/WAVE files;
- normalized interleaved `f32` sample buffers;
- preview buffer generation with basic pitch and volume handling;
- persistent sample references, sample-backed instruments, playback settings, and assignment metadata for mapping instruments to tracker tracks;
- deterministic waveform overviews for CLI and TUI rendering.

This keeps the MIDI-first runtime intact. Existing pattern playback still emits MIDI, while `trk-core::sampler_events` defines the data contract the audio layer consumes: track id, sample id/path, note pitch, velocity, gain, pan, pitch ratio, and scheduled position. The playback runtime loads assigned WAV files, applies sample tuning, gain/pan, sample start/end, and amplitude-envelope settings, prepares them for the default CPAL output format, and routes assigned sample events to realtime audio commands for audible sampler playback.

Users can inspect supported WAV files without opening the tracker UI:

```sh
trk sample inspect path/to/sample.wav --format text --buckets 64
trk sample inspect path/to/sample.wav --format json --width 32
```

Inside the tracker, `:sample view PATH` loads a WAV reference into the sampler view and renders its metadata and waveform preview. `Ctrl+J` opens the sampler view even when no sample is loaded. `:sample browse [DIR]` opens the in-app sample browser; `:sample choose [DIR]` uses a configured external chooser.

The waveform uses terminal-aware intensity shading: low-energy samples start in
violet, stronger samples pass through cyan and gold, and sharp attacks approach
bold white. Underlined baseline cells identify zero crossings. Sample-window
and loop boundaries use distinct marker glyphs without replacing visible peaks.
The color depth is detected once at startup and falls back automatically from
TrueColor to the 256-color or 16-color palette. Set the standard `NO_COLOR`
environment variable before launching `trk` for a modifier-and-glyph-only
render with no foreground or background colors.

After loading a WAV, assign it to the current track:

```text
:sample assign
:sample assign 2
:sample replace
:sample replace 2
:sample unassign
:sample unassign 2
:sample start 1200
:sample start clear
:sample end 48000
:sample end clear
:sample loop 2400 12000
:sample loop backward 2400 12000
:sample loop pingpong 2400 12000
:sample loop off
:sample mode reverse
:sample envelope 0.005 0.040 0.800 0.080
:sample render-selection bounces/loop.wav
:sample render-selection bounces/loop.wav --assign 2
:sample recorder inputs
:sample recorder capture 48000
:sample recorder trim 1200 36000
:sample recorder save-load recordings/take.wav --assign 2
:sample settings
:sample unload
:sample cleanup
:sample assignments
```

Assignments are saved in `.trk` project files. Loading old projects with direct `sampleAssignments` automatically creates compatible sample-backed instruments, and the sampler view shows the assigned instrument and track for the currently loaded sample.
`replace` swaps the sample on a track and removes the previous sample reference when it is no longer used.
`unload` removes the currently viewed sample reference only when it is unassigned, while `cleanup` prunes all unused sample references.

`render-selection` bounces the active tracker selection through the internal
sampler/native audio path, writes a WAV file atomically, loads the rendered file
into the sampler view, and stores it as a project sample reference. Add
`--assign TRACK` to assign the rendered sample immediately to a 1-based target
track. File/audio failures are reported before the project is mutated. External
MIDI-only destinations are not captured; selections with no internal sampler
events render as silence.

`sample recorder` captures bounded WAV takes from a system audio input when the
platform reports one. `inputs` lists devices, `capture FRAMES [DEVICE_ID]`
records a bounded take, `trim START END` crops the captured frame range, and
`save-load PATH [--assign TRACK]` writes the WAV, opens it in the sampler view,
adds a project sample reference, and optionally assigns it to a track. The
recorder state machine is independent from the platform input source, so
headless and CI environments can test transitions with fake input frames.

Playback settings are also saved in `.trk` project files:

- XRNS import preserves representable Renoise sample root note, transpose, fine tune, gain, pan, loop window, and ADSR-style envelope metadata;
- `start` and `end` set a frame window used by realtime playback and offline audio export;
- `loop` stores validated loop-point metadata for the sample reference;
- `envelope` stores attack seconds, decay seconds, sustain level `0..=1`, and release seconds, and is applied to realtime playback and offline audio export;
- `settings` prints the current settings for the loaded sample.

The implemented sampler controls are defined in the shared parameter descriptor
catalog so validation, formatting, persistence metadata, and future parameter
locks use stable IDs. The current descriptor coverage and Ableton/Renoise parity
status are tracked in [sampler-parity-roadmap.md](sampler-parity-roadmap.md).

Pattern-local `sample-gain` automation can change the effective gain of assigned
samples over time. See [automation.md](automation.md).

Mixer gain, pan, and audio mute/solo are applied to sampler-backed tracks. See
[mixer.md](mixer.md).

External sample browsing is optional. See [sample-browser.md](sample-browser.md) for the Yazi/chooser-file workflow and audition helper.

Current limitations:

- realtime sampler output uses the default CPAL output device only;
- no streaming for large samples;
- forward, backward, ping-pong, and reverse playback modes render in realtime playback and offline export;
- no choking, keyzones, or velocity layers;
- no effects, realtime level meter transport, or device selection yet.

Next sampler playback steps:

- add loop crossfade, choking, keyzones, and velocity layers.
