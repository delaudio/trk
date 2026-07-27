---
adr: 0002
title: Defer plugin hosting
status: Implemented
date: 2026-07-24
owner: default-agent
supersedes:
superseded-by:
depends-on: []
tags: [audio, plugins, post-mvp]
---

# ADR 0002 — Defer plugin hosting

## Context

trk is currently MIDI-first, with internal audio foundations now present for
assigned WAV samples, offline audio export, mixer gain/pan, and a minimal native
DSP graph. These foundations make plugin hosting more plausible than it was at
MVP planning time, but they do not yet make it safe to start.

Hosting third-party audio plugins would introduce realtime audio constraints,
plugin scanning, binary loading, preset/state serialization, sandboxing
concerns, crash isolation, UI/editor lifecycle issues, and platform-specific
distribution work. Those concerns are larger than the current native audio
surface, and they would force plugin SDK concepts into project and runtime
boundaries before trk's own instrument, routing, and device abstractions are
stable.

The current internal audio gaps that still matter for plugin hosting are:

- no user-facing audio device enumeration or selection;
- no stable plugin-neutral instrument/device model beyond sample-backed
  instruments and native gain/pan DSP devices;
- no send/routing graph beyond placeholders;
- no realtime meter transport or broader audio graph observability;
- no crash-isolated process boundary for untrusted plugin binaries;
- no persisted plugin state schema, migration strategy, or compatibility policy.

Plugin formats under consideration:

- VST3: broad DAW ecosystem support, cross-platform SDK, but requires careful
  SDK/license compliance and a host implementation for scanning,
  component/controller separation, state, buses, parameters, and realtime
  processing.
- Audio Unit: native macOS ecosystem support, but Apple-platform specific and
  unsuitable as the only portable plugin direction for trk's initial
  macOS/Linux target.
- CLAP: permissive modern API with strong Linux positioning and simpler
  extension-oriented design, but a smaller installed plugin ecosystem than VST3.

## Capability statement

trk defers direct plugin hosting and remains DAW/MIDI-first for third-party
instruments and effects while native sampler, mixer, DSP, routing, and export
boundaries mature.

Plugin hosting remains a post-MVP research track, not an implementation track.
Before implementation issues are opened, trk must have:

- a stable internal audio callback architecture;
- deterministic offline rendering semantics;
- sampler/instrument state serialization;
- user-facing audio device selection;
- a plugin-neutral device/instrument model that can represent native devices
  without plugin SDK types;
- a bounded realtime command/event boundary suitable for plugin parameter
  updates and metering;
- a crash/error boundary for plugin failures;
- an ADR choosing the first supported format and host crate/SDK strategy.

If plugin hosting becomes necessary, CLAP should be evaluated first for an
experimental Linux/macOS prototype because its model and licensing are friendlier
to a Rust-native host. VST3 should be evaluated next for ecosystem reach. Audio
Unit should be treated as an optional macOS-specific bridge, not the primary
abstraction.

Any future plugin-hosting implementation must live behind a dedicated boundary
such as `trk-plugin-host`. `trk-core` may store stable serializable
plugin references only after a follow-up ADR defines the schema; it must not
depend on VST3, AU, CLAP, or host crate types.

## User stories / scenarios

- As a maintainer, I want plugin hosting deferred behind explicit prerequisites,
  so that core audio boundaries can stabilize before third-party binary loading
  enters the runtime.
- As a user, I want trk to keep working as a MIDI-first tracker that can
  drive DAW-hosted instruments externally, so that plugin support is not a
  blocker for tracker editing.
- As a future implementer, I want the first plugin-hosting work to start from a
  format/host strategy ADR, so that SDK types and state schemas do not leak into
  core crates prematurely.

## Acceptance criteria

1. No direct third-party plugin hosting implementation work is opened until the
   prerequisite audio, routing, state, and error boundaries listed above exist.
2. Core crates do not depend on VST3, Audio Unit, CLAP, or host crate types.
3. Any future plugin-hosting implementation is isolated behind a dedicated crate
   boundary and backed by a follow-up ADR choosing the first supported format and
   host strategy.

## Out of scope

- Implementing plugin scanning, plugin loading, plugin UI hosting, plugin state
  serialization, or realtime plugin processing.
- Rejecting VST3, Audio Unit, or CLAP permanently.
- Changing trk's current MIDI/DAW interoperability path.

## Open questions

- None.

## References

- `../../docs/plugin-hosting-evaluation.md`
- `../../docs/native-dsp-roadmap.md`
- `../../docs/audio-engine.md`

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-24 | r1 | default-agent | Migrated the legacy `docs/adr/0001-plugin-hosting.md` decision into the docflow catalogue. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-24 | Requested docflow bootstrap in chat. |
