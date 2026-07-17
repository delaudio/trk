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
- apply stepped sample-gain automation to scheduled sampler events;
- apply mixer master gain, track gain, track pan, and audio mute/solo to scheduled sampler events;
- apply sample frame windows and amplitude envelopes before rendering;
- prepare samples for the requested output sample rate and channel count before rendering;
- write output files atomically through the app layer so failed exports are non-destructive.

Offline sampler rendering uses plain event data: sample id, target frame, gain, pan, pitch ratio, and velocity. The app adapts `salieri-core::sampler_events` into those events while keeping `salieri-audio` independent from project serialization and TUI state. Pattern automation and mixer state are resolved before events cross into `salieri-audio`.

The current renderer is deliberately small because Salieri is still MIDI-first. It renders sampler-backed tracks only; MIDI output sent to DAWs, external synths, or plugin hosts is not captured. Persisted loop points are not yet rendered as sustained loop playback. Future internal instruments, mixer routing, DSP, plugin render/freeze, and higher-quality resampling should render into `RenderedAudio` or a streaming equivalent, then reuse the same export format boundary.
