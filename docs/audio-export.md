# Audio Export

Audio export is post-MVP. The first implementation boundary lives in `salieri-audio` and is offline-only.

Supported format:

- WAV PCM16.

Current behavior:

- render deterministic audio buffers from sampler preview buffers;
- render scheduled sampler events into deterministic offline audio buffers;
- encode rendered buffers to WAV bytes without writing files directly;
- reject unsupported sample-rate or channel conversion with explicit errors;
- leave filesystem writes to the app layer so failed exports are non-destructive.

Offline sampler rendering uses plain event data: sample id, target frame, gain, pitch ratio, and velocity. App or interop code can adapt `salieri-core::sampler_events` into those events while keeping `salieri-audio` independent from project serialization and TUI state.

The current renderer is deliberately small because Salieri is still MIDI-first. It supports in-memory sample buffers that already match the requested sample rate and channel count. Future internal instruments, realtime sampler voices, resampling, channel conversion, and DSP should render into `RenderedAudio` or a streaming equivalent, then reuse the same export format boundary.
