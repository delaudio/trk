---
adr: 0026
title: Calibrate realtime output with live metering
status: Accepted
date: 2026-08-20
owner: default-agent
supersedes:
superseded-by:
depends-on: [0001]
tags: [audio, dsp, metering, performance, tui]
---

# ADR 0026 — Calibrate realtime output with live metering

## Context

`trk` already persists mixer gain, pan, routing, and native DSP chains, and the
same graph processes realtime and offline sampler audio. Live performance still
lacks a shallow calibration surface: changing gain staging requires navigating
the DSP rack, and the application receives no peak or RMS measurements from the
CPAL callback. The existing `measure_levels` helper operates only on completed
offline buffers.

Issue #315 requests a `t` modal with immediate gain, three-band sensitivity,
gate, smoothing, automatic gain, and animated meters. The referenced grain
implementation applies these settings to visualization features rather than to
audio samples. `trk` must instead preserve its existing project DSP as the
canonical persisted mix and apply calibration as an explicitly session-local
performance layer in the realtime audio path.

Normal-mode lowercase `t` is currently free. `Ctrl+t` creates a track and must
remain unchanged; uppercase `T` retains its view-specific Sequence and Clips
actions.

## Capability statement

`trk` provides a session-local realtime calibration processor and lock-free
meter bridge, controlled by a keyboard modal, so gain, spectral balance, gate,
meter decay, and bounded automatic gain can be tuned audibly during playback
without blocking the audio callback or mutating persisted project DSP.

## User stories / scenarios

- As a live performer, I want one-key access to gain staging and spectral
  balance, so that I can adapt playback without navigating device chains.
- As a mixer, I want continuously refreshed peak, RMS, and band meters, so that
  calibration is based on the actual post-DSP output.
- As an editor, I want calibration changes to remain temporary, so that an
  exploratory live adjustment does not silently rewrite the saved mix.
- As a keyboard user, I want existing track and sequence shortcuts preserved,
  so that the modal does not regress established workflows.

## Acceptance criteria

1. `trk-audio` owns a validated realtime calibration settings model with master
   and selected-track gain multipliers in `0.1..=4.0`, low/mid/high multipliers
   in `0.1..=4.0`, a normalized gate threshold in `0.0..=0.5`, a meter decay
   coefficient in `0.0..=0.95`, and an automatic-gain toggle. Balanced reset
   defaults are all gains `1.0`, gate `0.0`, decay `0.30`, and AGC disabled.
2. A cloneable control handle transfers settings to the realtime callback and
   publishes the latest meter snapshot through atomics or an equivalently
   allocation-free, non-blocking mechanism. The callback takes no mutex and
   performs no channel send for each audio frame or buffer.
3. The realtime sampler applies selected-track gain before that track's DSP and
   applies three-band gain, gate, bounded master gain, and AGC after the existing
   master DSP. AGC targets a safe peak without exceeding `0.1..=4.0`, reduces
   gain immediately for transients, and releases smoothly. Invalid/non-finite
   controls are rejected or clamped before the callback uses them.
4. Every rendered callback publishes post-calibration master peak/RMS and
   low/mid/high energy normalized to `0.0..=1.0`. Meter attack is immediate and
   decay follows the configured coefficient; silence and stopped playback
   converge to zero without non-finite values.
5. Lowercase `t` in normal Pattern mode opens a double-bordered DSP Calibration
   modal whether playback is running or stopped. Up/Down (or `j`/`k`) selects
   Master Gain, Track Gain, Low, Mid, High, Noise Gate, Meter Decay, or AGC;
   Left/Right and `-`/`+` adjust, `r` resets balanced defaults, and Esc or `t`
   closes. Modal input and pointer events are captured. `Ctrl+t` and
   view-specific uppercase `T` behavior remain unchanged.
6. Adjustments update the shared audio control immediately without restarting
   transport and without mutating `Song`, undo history, or dirty state. The
   modal identifies the selected track, shows ASCII sliders/toggle state, and
   renders animated LOW/MID/HIGH plus RMS/PEAK meters from the latest snapshot.
7. Automated tests cover validation/default/reset, deterministic band/gate/AGC
   and envelope processing, realtime handle propagation, silence/finite meter
   behavior, modal key capture and shortcut compatibility, non-persistence,
   and representative TUI snapshots.

## Out of scope

- Persisting calibration settings in `.trk` or replacing mixer/native-device
  parameters.
- Applying the performance calibration layer to offline export.
- MIDI-only track metering, external audio input, sidechain analysis, LUFS, or
  oversampled true-peak measurement.
- A full graphical mixer, per-track meter bank, or mouse-editable sliders.
- Spectrally transparent mastering-grade crossover or loudness normalization.

## Open questions

- None.

## References

- `0001-record-architecture-decisions.md`
- `../plan/todo/0024-dsp-calibration-modal.md`
- GitHub issue #315.
- [grain DSP settings and feature processing](https://github.com/delaudio/grain/blob/c609fcd5a05d8862d88819690a051f5df13238be/src/audio/dsp.rs)
- [grain tuning modal](https://github.com/delaudio/grain/blob/c609fcd5a05d8862d88819690a051f5df13238be/src/ui/mod.rs#L632-L766)

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-08-20 | r1 | default-agent | Recorded and accepted session-local audible calibration, lock-free realtime meters, and the keyboard tuning modal. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-08-20 | Approved autonomous resolution of the prioritized GitHub issue queue in chat. |
