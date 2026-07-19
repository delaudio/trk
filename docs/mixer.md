# Mixer

The mixer foundation is intentionally small and audio-only. MIDI output routing
continues to use track mute/solo and MIDI channel settings.

Mixer state is saved in `.salieri` projects:

- `masterGain` scales internal sampler output;
- each track has audio `gain`, `pan`, `muted`, `solo`, and placeholder send
  levels;
- per-track and master DSP chains can host native gain/pan devices;
- send bus metadata exists as a placeholder for future routing and effects.

Commands:

```text
:mixer master GAIN
:mixer gain [TRACK] GAIN
:mixer pan [TRACK] PAN
:mixer mute [TRACK]
:mixer solo [TRACK]
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
positive is right.

The Track Editor shows mixer gain, pan, audio mute (`AM`), and audio solo (`AS`)
beside the existing MIDI-level mute/solo controls.

Realtime playback and offline export apply mixer master gain, track gain, audio
mute/solo, track pan, and native DSP gain/pan chains to sampler-backed tracks.
The expanded native effect catalog and implementation order are maintained in
[Native DSP Roadmap](native-dsp-roadmap.md).
MIDI-only tracks are not captured in audio export and are not affected by audio
mixer mute/solo or DSP chains.

`salieri-audio::measure_levels` provides peak/RMS level data for rendered audio.
Realtime meter transport and graphical meter rendering are future work.
