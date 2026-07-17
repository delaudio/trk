# Sampler Foundation

The sampler work is post-MVP and intentionally lives outside `salieri-core`, `salieri-midi`, and `salieri-tui`.

`salieri-sampler` currently provides:

- WAV loading for 16-bit PCM and 32-bit float RIFF/WAVE files;
- normalized interleaved `f32` sample buffers;
- preview buffer generation with basic pitch and volume handling;
- persistent sample references and assignment metadata for mapping samples to tracker tracks;
- deterministic waveform overviews for CLI and TUI rendering.

This keeps the MIDI-first runtime intact. Existing pattern playback still emits MIDI, while `salieri-core::sampler_events` defines the data contract the audio layer consumes: track id, sample id/path, note pitch, velocity, gain, pitch ratio, and scheduled position. The playback runtime loads assigned WAV files, prepares them for the default CPAL output format, and routes assigned sample events to realtime audio commands for audible sampler playback.

Users can inspect supported WAV files without opening the tracker UI:

```sh
salieri sample inspect path/to/sample.wav --format text --buckets 64
salieri sample inspect path/to/sample.wav --format json --width 32
```

Inside the tracker, `:sample view PATH` loads a WAV reference into the sampler view and renders its metadata and waveform preview. `F11` opens the sampler view even when no sample is loaded. `:sample browse [DIR]` opens the in-app sample browser; `:sample choose [DIR]` uses a configured external chooser.

After loading a WAV, assign it to the current track:

```text
:sample assign
:sample assign 2
:sample replace
:sample replace 2
:sample unassign
:sample unassign 2
:sample unload
:sample cleanup
:sample assignments
```

Assignments are saved in `.salieri` project files, and the sampler view shows the assigned track for the currently loaded sample.
`replace` swaps the sample on a track and removes the previous sample reference when it is no longer used.
`unload` removes the currently viewed sample reference only when it is unassigned, while `cleanup` prunes all unused sample references.

External sample browsing is optional. See [sample-browser.md](sample-browser.md) for the Yazi/chooser-file workflow and audition helper.

Current limitations:

- realtime sampler output uses the default CPAL output device only;
- no streaming for large samples;
- no envelopes, looping, choking, or velocity layers;
- no mixer, effects, level metering, or device selection yet.

Next sampler playback steps:

- add sampler playback controls such as loop points, envelopes, choking, and velocity layers.
