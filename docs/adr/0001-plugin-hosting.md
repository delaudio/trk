# ADR 0001: Defer Plugin Hosting

## Status

Accepted

## Context

Salieri is currently MIDI-first. Internal audio, sampler playback, DSP, device selection, and audio export are still post-MVP foundations. Hosting third-party audio plugins would introduce realtime audio constraints, plugin scanning, binary loading, preset/state serialization, sandboxing concerns, crash isolation, and platform-specific distribution work before the internal audio architecture is stable.

Plugin formats under consideration:

- VST3: broad DAW ecosystem support, cross-platform SDK, but requires careful SDK/license compliance and a host implementation for scanning, component/controller separation, state, buses, parameters, and realtime processing.
- Audio Unit: native macOS ecosystem support, but Apple-platform specific and unsuitable as the only portable plugin direction for Salieri's initial macOS/Linux target.
- CLAP: permissive modern API with strong Linux positioning and simpler extension-oriented design, but a smaller installed plugin ecosystem than VST3.

## Decision

Salieri will not implement plugin hosting in the MIDI-first MVP or in the first internal audio foundation.

Plugin hosting remains a post-MVP research track. Before implementation, Salieri must have:

- a stable internal audio callback architecture;
- deterministic offline rendering semantics;
- sampler/instrument state serialization;
- a crash/error boundary for plugin failures;
- an ADR choosing the first supported format and host crate/SDK strategy.

If plugin hosting becomes necessary, CLAP should be evaluated first for an experimental Linux/macOS prototype because its model and licensing are friendlier to a Rust-native host. VST3 should be evaluated next for ecosystem reach. Audio Unit should be treated as an optional macOS-specific bridge, not the primary abstraction.

## Consequences

- The MVP stays focused on MIDI sequencing and terminal-first editing.
- Internal instruments and sampler work can mature without third-party binary loading.
- Project files do not need plugin state chunks yet.
- Users can still drive plugin instruments indirectly through MIDI in a DAW.
- A future plugin host must live behind an isolated crate boundary and must not leak SDK types into `salieri-core`.
