# Audio Engine Foundation

Internal audio is post-MVP. The current product remains MIDI-first.

## CPAL Evaluation

CPAL is the preferred first backend for macOS and Linux because it provides a Rust-native cross-platform audio callback model and can later support Windows without changing the tracker core. Salieri should keep CPAL isolated inside `salieri-audio`; no core, TUI, or project model code should depend directly on CPAL types.

The initial `salieri-audio` crate does not open a real device yet. It defines the lifecycle and boundary that a CPAL backend will implement:

- `AudioRuntime` owns an audio thread and command channel;
- `AudioBackend` abstracts start/stop behavior;
- `NullAudioBackend` makes lifecycle tests deterministic without hardware;
- `RealtimeAudioCommand` is plain data intended for future lock-free transport to the callback.

## Realtime Boundary

The audio callback must not depend on Ratatui, filesystem APIs, project serialization, logging, or unbounded allocation. Future communication from app/sequencer code to audio code should use bounded queues or preallocated buffers. Commands crossing the boundary should be immutable data such as sample IDs, target frames, gain, pitch ratio, and all-notes-off markers.

## Lifecycle

The audio runtime supports:

- start;
- stop;
- idempotent stop when already stopped;
- shutdown on explicit command;
- shutdown on `Drop`.

Shutdown must stop the backend before the thread exits. Errors are reported as updates rather than panicking.

## Current Limitations

- no CPAL dependency is linked yet;
- no device enumeration;
- no audio callback;
- no mixer, voices, envelopes, or DSP graph;
- no integration with sampler assignments or pattern playback.
