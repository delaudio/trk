# Sampler Foundation

The sampler work is post-MVP and intentionally lives outside `salieri-core`, `salieri-midi`, and `salieri-tui`.

`salieri-sampler` currently provides:

- WAV loading for 16-bit PCM and 32-bit float RIFF/WAVE files;
- normalized interleaved `f32` sample buffers;
- preview buffer generation with basic pitch and volume handling;
- sample assignment metadata for mapping samples to tracker tracks;
- deterministic waveform overviews for CLI and TUI rendering.

This keeps the MIDI-first core intact. The MVP playback runtime still emits MIDI only. A later audio engine can consume `salieri-sampler` preview buffers and assignments without making the core model depend on an audio backend.

Users can inspect supported WAV files without opening the tracker UI:

```sh
salieri sample inspect path/to/sample.wav --format text --buckets 64
salieri sample inspect path/to/sample.wav --format json --width 32
```

Inside the tracker, `:sample view PATH` loads a WAV reference into the sampler view and renders its metadata and waveform preview. `F11` opens the sampler view even when no sample is loaded.

External sample browsing is optional. See [sample-browser.md](sample-browser.md) for the Yazi/chooser-file workflow and audition helper.

Current limitations:

- no realtime audio output;
- no streaming for large samples;
- no envelopes, looping, choking, or velocity layers;
- no TUI sampler browser or sample assignment workflow yet;
- no persistence schema for sampler instruments yet.

Next sampler playback steps:

- define persistent sample references and assignment slots;
- route pattern events to sampler voices in the future audio engine;
- add explicit sample unload/replace commands;
- decide how external browsers such as Yazi hand selected sample paths back to Salieri.
