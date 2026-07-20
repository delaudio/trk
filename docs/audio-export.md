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
- apply native per-track and master DSP gain/pan chains to sampler output;
- apply sample frame windows and amplitude envelopes before rendering;
- prepare samples for the requested output sample rate and channel count before rendering;
- write output files atomically through the app layer so failed exports are non-destructive.

Offline sampler rendering uses plain event data: track id, sample id, target frame, gain, pan, pitch ratio, and velocity. The app adapts `salieri-core::sampler_events` and persisted mixer DSP chains into those specs while keeping `salieri-audio` independent from project serialization and TUI state. Pattern automation and mixer state are resolved before events cross into `salieri-audio`.

Render plans and stems:

```bash
salieri export plan song.salieri plan.json --pattern 1 --tracks 1,2
salieri export plan song.salieri --sequence
salieri export stems song.salieri stems/ --pattern 1 --tracks 1,2
salieri export stems song.salieri stems/ --sequence
```

`export plan` emits JSON describing the render target, selected tracks, sampler
event counts, sample rate, channel count, and explicit limitations. It can print
to stdout or write a JSON file, so renders can be inspected before any WAV files
are created. `export stems` writes deterministic per-track WAV files plus a
`stems.json` manifest. Tracks without internal sampler events render as silence
for the target duration and are marked with `samplerEvents: 0` in the manifest.

Selection-to-sample rendering is available inside the tracker:

```text
:sample render-selection bounces/loop.wav
:sample render-selection bounces/loop.wav --assign 2
```

The command renders the current row/track selection to a WAV file, registers the
new sample reference only after the file has been written and reloaded
successfully, and can assign it immediately to a target track. It uses the same
internal sampler/native audio path and the same external MIDI-only limitation as
CLI audio export.

The current renderer is deliberately small because Salieri is still MIDI-first. It renders sampler-backed tracks only; MIDI output sent to DAWs, external synths, or plugin hosts is not captured. Persisted loop points are not yet rendered as sustained loop playback. Future internal instruments, mixer routing, broader DSP devices, plugin render/freeze, and higher-quality resampling should render into `RenderedAudio` or a streaming equivalent, then reuse the same export format boundary.
