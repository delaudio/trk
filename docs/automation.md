# Automation

Automation is pattern-local and tracker-native. Stepped sample-gain lanes drive
sampler-backed tracks, while MIDI CC lanes are created and edited in the Web
Companion Piano Roll.

Commands:

```text
:automation sample-gain VALUE
:automation sample-gain ROW VALUE
:automation sample-gain clear
:automation sample-gain clear ROW
```

`VALUE` is a non-negative gain value. Without an explicit row, the command uses
the cursor row. The target sample is resolved from the current track assignment.

Semantics:

- interpolation is stepped;
- each point applies from its row until the next point for the same target;
- rows before the first point use the sample reference gain;
- realtime playback and offline audio export observe the same scheduled sampler
  event gain;
- automation lanes are saved inside each pattern in `.trk` project files.

Current limitations:

- MIDI CC points use normalized values, emit deterministic `0..127` Control
  Change messages, and respect the track's MIDI channel and `cc_out` routing;
- generic row-level parameter locks can set or reset sampler gain, track mixer
  gain/pan, master gain, send gain, and native gain/pan device parameters on the
  current row through `:plock`;
- the Web Companion provides the graphical MIDI CC editor; sample-gain lanes
  remain command-driven;
- linear interpolation is future work.

The native effect catalog in [Native DSP Roadmap](native-dsp-roadmap.md) marks
which planned parameters must become addressable by generic per-step parameter
locks before their devices are considered production-ready.

Parameter-lock commands:

```text
:plock sample-gain VALUE|reset|clear
:plock mixer gain VALUE|reset|clear
:plock mixer pan VALUE|reset|clear
:plock master gain VALUE|reset|clear
:plock send SEND_ID VALUE|reset|clear
:plock dsp track gain VALUE|reset|clear
:plock dsp track pan VALUE|reset|clear
:plock dsp master gain VALUE|reset|clear
:plock dsp master pan VALUE|reset|clear
```

Values are parsed, formatted, and validated by `ParameterDescriptor` metadata.
Realtime playback and offline export consume the same sampler/mixer row locks;
native effect locks are emitted as ordered parameter events for the native module
runtime boundary.
