# ADR 0001: Defer Plugin Hosting

## Status

Accepted

## Context

Salieri is currently MIDI-first, with internal audio foundations now present for assigned WAV samples, offline audio export, mixer gain/pan, and a minimal native DSP graph. These foundations make plugin hosting more plausible than it was at MVP planning time, but they do not yet make it safe to start.

Hosting third-party audio plugins would introduce realtime audio constraints, plugin scanning, binary loading, preset/state serialization, sandboxing concerns, crash isolation, UI/editor lifecycle issues, and platform-specific distribution work. Those concerns are larger than the current native audio surface, and they would force plugin SDK concepts into project and runtime boundaries before Salieri's own instrument, routing, and device abstractions are stable.

The current internal audio gaps that still matter for plugin hosting are:

- no user-facing audio device enumeration or selection;
- no stable plugin-neutral instrument/device model beyond sample-backed instruments and native gain/pan DSP devices;
- no send/routing graph beyond placeholders;
- no realtime meter transport or broader audio graph observability;
- no crash-isolated process boundary for untrusted plugin binaries;
- no persisted plugin state schema, migration strategy, or compatibility policy.

Plugin formats under consideration:

- VST3: broad DAW ecosystem support, cross-platform SDK, but requires careful SDK/license compliance and a host implementation for scanning, component/controller separation, state, buses, parameters, and realtime processing.
- Audio Unit: native macOS ecosystem support, but Apple-platform specific and unsuitable as the only portable plugin direction for Salieri's initial macOS/Linux target.
- CLAP: permissive modern API with strong Linux positioning and simpler extension-oriented design, but a smaller installed plugin ecosystem than VST3.

## Decision

Salieri will continue to defer direct plugin hosting. The current decision is to remain DAW/MIDI-first for third-party instruments and effects while native sampler, mixer, DSP, routing, and export boundaries mature.

Plugin hosting remains a post-MVP research track, not an implementation track. Before implementation issues are opened, Salieri must have:

- a stable internal audio callback architecture;
- deterministic offline rendering semantics;
- sampler/instrument state serialization;
- user-facing audio device selection;
- a plugin-neutral device/instrument model that can represent native devices without plugin SDK types;
- a bounded realtime command/event boundary suitable for plugin parameter updates and metering;
- a crash/error boundary for plugin failures;
- an ADR choosing the first supported format and host crate/SDK strategy.

If plugin hosting becomes necessary, CLAP should be evaluated first for an experimental Linux/macOS prototype because its model and licensing are friendlier to a Rust-native host. VST3 should be evaluated next for ecosystem reach. Audio Unit should be treated as an optional macOS-specific bridge, not the primary abstraction.

Any future plugin-hosting implementation must live behind a dedicated boundary such as `salieri-plugin-host`. `salieri-core` may store stable serializable plugin references only after a follow-up ADR defines the schema; it must not depend on VST3, AU, CLAP, or host crate types.

No implementation issues are opened from this ADR update. The next valid work item is another ADR or design spike after the prerequisites above are substantially implemented.

## Consequences

- The MVP stays focused on MIDI sequencing and terminal-first editing.
- Internal instruments and sampler work can mature without third-party binary loading.
- Project files do not need plugin state chunks yet.
- Users can still drive plugin instruments indirectly through MIDI in a DAW.
- Salieri avoids committing to a plugin state format before native devices, routing, and automation are stable.
- CLAP remains the first research candidate, but VST3 and Audio Unit are not rejected; they are sequenced behind a follow-up host strategy decision.
- A future plugin host must live behind an isolated crate boundary and must not leak SDK types into `salieri-core`.
- Renoise-class parity continues to treat direct plugin hosting as a later milestone, separate from tracker editing, sampler playback, native DSP, and MIDI/DAW integration.
