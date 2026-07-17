# Plugin Hosting Evaluation

Plugin hosting is post-MVP and remains deferred by [ADR 0001](adr/0001-plugin-hosting.md). The July 2026 review keeps Salieri DAW/MIDI-first for third-party instruments and effects while native audio, sampler, mixer, DSP, routing, and export boundaries mature.

## Format Comparison

| Format | Platform Fit | Ecosystem | Licensing / SDK | Host Complexity | Initial Fit |
| --- | --- | --- | --- | --- | --- |
| VST3 | macOS, Linux, Windows | Very broad | Requires Steinberg SDK/license compliance | High: scanning, component/controller model, buses, parameters, state | Strong ecosystem, later |
| Audio Unit | macOS | Strong on Apple platforms | Apple platform APIs | High and platform-specific | Optional macOS bridge |
| CLAP | macOS, Linux, Windows | Growing | Permissive, modern API | Medium-high, extension based | Best research candidate |

## Constraints

- Salieri's initial target is macOS and Linux.
- Plugin hosting requires an internal audio engine first.
- Plugin state must be serializable without making `.salieri` files opaque by default.
- A crashing plugin must not corrupt the project or leave the terminal/audio backend in a broken state.
- Realtime processing must not allocate, log, block on filesystem, or call TUI code.

## Current Decision

Do not implement plugin hosting yet. Continue to use DAWs and software instruments through MIDI output.

The first implementation proposal must be a separate ADR after the audio architecture has:

- user-facing audio device selection;
- a plugin-neutral device/instrument model that can represent native devices before SDK-backed devices;
- routing and send semantics beyond placeholders;
- a bounded realtime command/event queue for parameter updates and metering;
- deterministic offline render/export semantics for native devices;
- documented plugin crash, cleanup, and terminal/audio recovery behavior.

## Likely Architecture

Future plugin hosting should live in a dedicated crate such as `salieri-plugin-host`.

Expected boundaries:

- `salieri-core` stores only stable, serializable plugin references and parameter automation, never SDK types.
- `salieri-audio` owns realtime processing integration.
- `salieri-app` owns scanning commands, user-facing errors, and persistence migration.
- The TUI displays plugin state but never loads plugin binaries directly.
