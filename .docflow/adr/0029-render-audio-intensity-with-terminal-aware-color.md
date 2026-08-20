---
adr: 0029
title: Render audio intensity with terminal-aware color
status: Accepted
date: 2026-08-20
owner: default-agent
supersedes:
superseded-by:
depends-on: [0001, 0017, 0026]
tags: [tui, color, waveform, metering, sampler, accessibility]
---

# ADR 0029 — Render audio intensity with terminal-aware color

## Context

The sampler waveform currently preserves peaks with Unicode half-block geometry
but renders every sample column with the same inherited style. Live calibration
meters use one basic ANSI color per band. These representations communicate
shape and magnitude, but do not distinguish transient energy, zero crossings,
loop boundaries, or meter risk zones quickly enough for performance and mixing.

The TUI theme already contains fixed RGB colors, yet the application does not
model terminal color depth. Sending new high-density RGB heatmaps without an
explicit capability boundary would make them muddy or unreadable in 256-color
and standard ANSI terminals, and would violate an explicit `NO_COLOR` request.
The renderer must therefore choose its color mode once at startup, retain
identical semantic geometry across depths and in monochrome, and keep detection
and mapping pure enough for deterministic tests.

Issue #318 points to `grain`, which maps numeric RGB/HSB colors into terminal
cells and uses bright, bold accents for energetic glyphs. `trk` needs the same
clarity while preserving its existing Rust-native waveform aggregation,
responsive layouts, snapshot discipline, and callback-safe meter transport.

## Capability statement

`trk` renders sampler waveforms and live audio meters as terminal-aware
intensity heatmaps: smooth 24-bit color where supported, deterministic indexed
or ANSI palettes elsewhere, and distinct transient, zero-crossing, and loop
semantics without changing audio or project state.

## User stories / scenarios

- As a sample editor, I want transient peaks and zero crossings to stand out in
  the waveform, so that I can identify useful edit boundaries at a glance.
- As a performer, I want loop boundaries to remain visible over the heatmap, so
  that the active playback window is unambiguous.
- As a mixer, I want live levels to move continuously from safe through warning
  and clip colors, so that overload risk is visible before reading a number.
- As a user of an older or restricted terminal, I want the same waveform and
  meter semantics with a compatible palette, so that richer rendering never
  degrades usability.

## Acceptance criteria

1. The application resolves terminal color mode once at startup as TrueColor,
   indexed 256-color, standard ANSI, or monochrome from bounded environment
   evidence. The pure detector gives a present `NO_COLOR` value precedence over
   `COLORTERM` and `TERM`, has deterministic tests, and passes the selected mode
   into TUI rendering without terminal queries during a frame.
2. `trk-tui` provides pure finite-safe color utilities for RGB interpolation,
   HSB-to-RGB conversion, multi-stop intensity gradients, and mode-aware
   conversion to optional Ratatui colors. Inputs outside `0..=1` and non-finite
   inputs are clamped deterministically.
3. Loaded sampler waveforms retain responsive peak-preserving half-block
   geometry while each visible column receives a smooth violet-to-cyan-to-gold
   intensity style. Local attack spikes receive a bold hot accent, and cells
   spanning zero receive a distinct baseline treatment that remains visible
   at every supported color depth.
4. Visible sample start/end and loop start/end frame boundaries are projected
   into the current waveform window and rendered with distinct contrast
   markers without hiding peak geometry. Absent, invalid, or off-window markers
   are inert. Future slice metadata may reuse this marker boundary but is not
   invented by this capability.
5. Callback-safe low/mid/high/RMS/peak values in the live DSP calibration view
   use a continuous decibel-aware safe-to-warning-to-hot-to-clip gradient:
   green below -12 dB, yellow by -3 dB, orange/red toward 0 dB, and a bright
   bold clip cell at full scale. Non-finite values render as silence.
6. Indexed, ANSI, and monochrome fallback output preserves all labels, glyphs,
   marker positions, emphasis, and filled-cell counts. Indexed and ANSI modes
   emit no RGB colors; `NO_COLOR` emits no foreground or background color and
   retains information-bearing distinctions through glyphs and modifiers.
7. Unit tests cover detection, RGB/HSB interpolation, depth conversion,
   transient and zero-crossing classification, marker projection, and meter
   thresholds. Buffer-style assertions and updated visual snapshots cover
   TrueColor, indexed, and ANSI rendering without relying on a particular host
   terminal.
8. Rendering remains bounded by visible waveform/meter cells, does not inspect
   raw audio during a frame, does not mutate project or audio state, and passes
   the complete repository verification gate.

## Out of scope

- A full-screen mixer console, channel-strip interaction, or per-track
  post-DSP meter transport; those belong to the separate mixer capability.
- Sample slicing, transient-to-slice mutation, or persistence of slice points.
- Terminal graphics protocols such as Sixel, Kitty graphics, iTerm images, or
  pixel-frame rendering.
- Replacing the entire existing theme or guaranteeing exact RGB appearance
  after a terminal emulator applies its own profile.
- Runtime capability renegotiation after startup or active OSC terminal probes.

## Open questions

- None.

## References

- `0001-record-architecture-decisions.md`
- `0017-control-supported-sampler-actions-with-the-pointer.md`
- `0026-calibrate-realtime-output-with-live-metering.md`
- `../plan/todo/0027-terminal-aware-audio-heatmaps.md`
- GitHub issue #318.
- [grain preview backend](https://github.com/delaudio/grain/blob/c609fcd5a05d8862d88819690a051f5df13238be/src/preview/backend.rs)
- [grain headless color runtime](https://github.com/delaudio/grain/blob/c609fcd5a05d8862d88819690a051f5df13238be/src/runtime/js/runner.js)

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-08-20 | r1 | default-agent | Recorded and accepted terminal-aware audio heatmaps, semantic waveform accents, loop markers, and live meter gradients. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-08-20 | Approved autonomous resolution of the prioritized GitHub issue queue in chat. |
