# Mixer

The mixer foundation is intentionally small and audio-only. MIDI output routing
continues to use track mute/solo and MIDI channel settings.

Mixer state is saved in `.salieri` projects:

- `masterGain` scales internal sampler output;
- each track has audio `gain`, `pan`, `muted`, `solo`, and send levels;
- delay and reverb send buses store `preFader` mode plus native return effects;
- per-track, send, and master DSP chains can contain native devices.

Commands:

```text
:mixer master GAIN
:mixer gain [TRACK] GAIN
:mixer pan [TRACK] PAN
:mixer mute [TRACK]
:mixer solo [TRACK]
:mixer send delay
:mixer send reverb
:mixer send list
:mixer send SEND pre|post
:mixer send SEND gain [TRACK] GAIN
:mixer send SEND clear
```

Native DSP commands:

```text
:dsp track [TRACK] gain GAIN
:dsp track [TRACK] pan PAN
:dsp track [TRACK] clear
:dsp master gain GAIN
:dsp master pan PAN
:dsp master clear
```

Generic row-scoped parameter locks can also target mixer and native DSP
parameters without permanently editing the chain:

```text
:plock mixer gain VALUE
:plock mixer pan VALUE
:plock master gain VALUE
:plock dsp track gain VALUE
:plock dsp track pan VALUE
:plock dsp master gain VALUE
:plock dsp master pan VALUE
```

`TRACK` is one-based. Without a track number, commands use the current track.
`GAIN` must be non-negative. `PAN` is `-1.0..=1.0`, where negative is left and
positive is right. `SEND` may be `delay`, `reverb`, or a numeric send id.

The Track Editor shows mixer gain, pan, audio mute (`AM`), and audio solo (`AS`)
beside the existing MIDI-level mute/solo controls.

Realtime playback and offline export apply mixer master gain, track gain, audio
mute/solo, track pan, native DSP chains, and delay/reverb send routing to
sampler-backed tracks. Send buses are mixed into the main output before the
master chain, so master effects process dry and return signal together.
The expanded native effect catalog and implementation order are maintained in
[Native DSP Roadmap](native-dsp-roadmap.md).
MIDI-only tracks are not captured in audio export and are not affected by audio
mixer mute/solo or DSP chains.

`salieri-audio::measure_levels` provides peak/RMS level data for rendered audio.
Realtime meter transport and graphical meter rendering are future work.
