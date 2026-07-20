# Sampler Foundation

The sampler work is post-MVP and intentionally lives outside `salieri-core`, `salieri-midi`, and `salieri-tui`.

`salieri-sampler` currently provides:

- WAV loading for 16-bit PCM and 32-bit float RIFF/WAVE files;
- normalized interleaved `f32` sample buffers;
- preview buffer generation with basic pitch and volume handling;
- persistent sample references, sample-backed instruments, playback settings, and assignment metadata for mapping instruments to tracker tracks;
- deterministic waveform overviews for CLI and TUI rendering.

This keeps the MIDI-first runtime intact. Existing pattern playback still emits MIDI, while `salieri-core::sampler_events` defines the data contract the audio layer consumes: track id, sample id/path, note pitch, velocity, gain, pitch ratio, and scheduled position. The playback runtime loads assigned WAV files, applies sample start/end and amplitude-envelope settings, prepares them for the default CPAL output format, and routes assigned sample events to realtime audio commands for audible sampler playback.

Users can inspect supported WAV files without opening the tracker UI:

```sh
salieri sample inspect path/to/sample.wav --format text --buckets 64
salieri sample inspect path/to/sample.wav --format json --width 32
```

Inside the tracker, `:sample view PATH` loads a WAV reference into the sampler view and renders its metadata and waveform preview. `Ctrl+J` opens the sampler view even when no sample is loaded. `:sample browse [DIR]` opens the in-app sample browser; `:sample choose [DIR]` uses a configured external chooser.

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
:sample loop off
:sample envelope 0.005 0.040 0.800 0.080
:sample settings
:sample unload
:sample cleanup
:sample assignments
```

Assignments are saved in `.salieri` project files. Loading old projects with direct `sampleAssignments` automatically creates compatible sample-backed instruments, and the sampler view shows the assigned instrument and track for the currently loaded sample.
`replace` swaps the sample on a track and removes the previous sample reference when it is no longer used.
`unload` removes the currently viewed sample reference only when it is unassigned, while `cleanup` prunes all unused sample references.

Playback settings are also saved in `.salieri` project files:

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
- loop points are persisted and displayed but are not yet rendered as sustained loop playback;
- no choking, keyzones, or velocity layers;
- no effects, realtime level meter transport, or device selection yet.

Next sampler playback steps:

- add sustained loop playback, choking, keyzones, and velocity layers.
