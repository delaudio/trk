# Audio Export

Audio export renders assigned-sample playback offline through `salieri-audio`.

Supported format:

- WAV PCM16.

CLI usage:

```bash
salieri export audio input.salieri output.wav --pattern 1
salieri export audio input.salieri output.wav --sequence --sample-rate 48000 --channels 2
```

Current behavior:

- render deterministic audio buffers from sampler preview buffers;
- render scheduled sampler events into deterministic offline audio buffers;
- encode rendered buffers to WAV bytes;
- load assigned project samples from their stored paths;
- prepare samples for the requested output sample rate and channel count before rendering;
- write output files atomically through the app layer so failed exports are non-destructive.

Offline sampler rendering uses plain event data: sample id, target frame, gain, pitch ratio, and velocity. The app adapts `salieri-core::sampler_events` into those events while keeping `salieri-audio` independent from project serialization and TUI state.

The current renderer is deliberately small because Salieri is still MIDI-first. It renders sampler-backed tracks only; MIDI output sent to DAWs, external synths, or plugin hosts is not captured. Future internal instruments, mixer routing, DSP, plugin render/freeze, and higher-quality resampling should render into `RenderedAudio` or a streaming equivalent, then reuse the same export format boundary.
