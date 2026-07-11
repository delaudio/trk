# Audio Export

Audio export is post-MVP. The first implementation boundary lives in `salieri-audio` and is offline-only.

Supported format:

- WAV PCM16.

Current behavior:

- render deterministic audio buffers from sampler preview buffers;
- encode rendered buffers to WAV bytes without writing files directly;
- reject unsupported sample-rate or channel conversion with explicit errors;
- leave filesystem writes to the app layer so failed exports are non-destructive.

The current renderer is deliberately small because Salieri is still MIDI-first. Future internal instruments, sampler voices, and DSP should render into `RenderedAudio` or a streaming equivalent, then reuse the same export format boundary.
