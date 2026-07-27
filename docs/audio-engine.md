# Audio Engine Foundation

Internal audio is post-MVP. The current product remains MIDI-first.

## CPAL Evaluation

CPAL is the preferred first backend for macOS and Linux because it provides a Rust-native cross-platform audio callback model and can later support Windows without changing the tracker core. trk should keep CPAL isolated inside `trk-audio`; no core, TUI, or project model code should depend directly on CPAL types.

`trk-audio` defines the lifecycle boundary and includes a first CPAL backend:

- `AudioRuntime` owns an audio thread and command channel;
- `AudioBackend` abstracts start/stop behavior;
- `NullAudioBackend` makes lifecycle tests deterministic without hardware;
- `CpalAudioBackend` opens the default output device and renders registered realtime sampler voices;
- `RealtimeAudioCommand` is plain data intended for future lock-free transport to the callback.
- offline export supports deterministic sampler-preview rendering and WAV PCM16 encoding without writing files directly.
- `RealtimeSampler` provides a hardware-free voice pool that consumes realtime sample trigger, stop-voice, and all-notes-off commands.

The app playback runtime loads WAV files for assigned samples before playback, applies the persisted sample frame window and amplitude envelope, prepares them for the output sample rate and channel count, registers them with the CPAL backend, installs the native DSP graph, and then routes scheduled sampler events to realtime voices. Stepped sample-gain automation and mixer master/track gain, pan, and audio mute/solo are resolved into sampler event gain/pan before the realtime boundary. Per-track and master DSP gain/pan devices run in the audio layer for realtime playback and offline export. The broader native DSP parity matrix and effect implementation order are maintained in [Native DSP Roadmap](native-dsp-roadmap.md). MIDI output remains unchanged for non-sample tracks and external instruments. Forward, backward, ping-pong, and reverse sample playback modes use the same source-frame resolver in realtime and offline rendering.

## Realtime Boundary

The audio callback must not depend on Ratatui, filesystem APIs, project serialization, logging, or unbounded allocation. Future communication from app/sequencer code to audio code should use bounded queues or preallocated buffers. Commands crossing the boundary should be immutable data such as sample IDs, track IDs, target frames, gain, pitch ratio, DSP graph specs, and all-notes-off markers.

## Lifecycle

The audio runtime supports:

- start;
- stop;
- idempotent stop when already stopped;
- shutdown on explicit command;
- shutdown on `Drop`.

Shutdown must stop the backend before the thread exits. Errors are reported as updates rather than panicking.

## Current Limitations

- no user-facing device enumeration or selection;
- no sample choking or loop crossfade;
- no realtime meter transport or graphical meter rendering yet.
