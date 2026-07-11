# Sampler Foundation

The sampler work is post-MVP and intentionally lives outside `salieri-core`, `salieri-midi`, and `salieri-tui`.

`salieri-sampler` currently provides:

- WAV loading for 16-bit PCM and 32-bit float RIFF/WAVE files;
- normalized interleaved `f32` sample buffers;
- preview buffer generation with basic pitch and volume handling;
- sample assignment metadata for mapping samples to tracker tracks.

This keeps the MIDI-first core intact. The MVP playback runtime still emits MIDI only. A later audio engine can consume `salieri-sampler` preview buffers and assignments without making the core model depend on an audio backend.

Current limitations:

- no realtime audio output;
- no streaming for large samples;
- no envelopes, looping, choking, or velocity layers;
- no TUI sampler browser yet;
- no persistence schema for sampler instruments yet.
