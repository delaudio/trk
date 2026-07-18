# Architecture Quality Roadmap

This roadmap turns the July 2026 architecture audit into an ordered maintenance plan. It is a planning companion to the normative ownership, dependency, and file-budget rules in [Architecture Notes](architecture.md).

## Audit Baseline

Salieri Tracker currently contains about 26.8k lines of Rust across 21 source and test files. Most behavior is concentrated in a few modules:

| File | July 2026 lines | Hard budget | Split issue |
| --- | ---: | ---: | --- |
| `crates/salieri-app/src/main.rs` | 9,946 | 1,000 | [#103](https://github.com/delaudio/salieri-tracker/issues/103) |
| `crates/salieri-tui/src/render.rs` | 3,620 | 800 | [#105](https://github.com/delaudio/salieri-tracker/issues/105) |
| `crates/salieri-core/src/model.rs` | 3,194 | 800 | [#107](https://github.com/delaudio/salieri-tracker/issues/107) |
| `crates/salieri-interop/src/lib.rs` | 2,646 | 800 | [#111](https://github.com/delaudio/salieri-tracker/issues/111) |
| `crates/salieri-audio/src/lib.rs` | 2,047 | 800 | [#109](https://github.com/delaudio/salieri-tracker/issues/109) |
| `crates/salieri-app/src/playback_runtime.rs` | 1,691 | 1,000 | [#108](https://github.com/delaudio/salieri-tracker/issues/108) |

These files are baselined by the [file-size check](../scripts/check-rust-file-sizes.sh): they can shrink but cannot grow. Files over only a soft limit remain visible in its report and should be split when a cohesive boundary appears.

The comparison project, Yazi, demonstrates that a larger Rust application can keep files focused by separating configuration, parsing, actions, domain state, rendering, scheduling, runners, terminal integration, and shared types. Salieri should adopt the boundaries that address demonstrated pressure rather than reproduce Yazi's crate count or runtime design.

## Target Shape

The target keeps the existing workspace domains and decomposes modules inside them before adding more crates:

```text
terminal / CLI / config
          |
          v
salieri-app actions and typed commands
    |        |         |          |
    v        v         v          v
  TUI     tasks    playback    persistence
                       |
             +---------+---------+
             v                   v
           MIDI                audio
             \                   /
              +---- core events-+
                       |
                 core song model
```

Core remains the dependency leaf for serializable music state and deterministic playback semantics. Edge crates own formats, protocols, devices, and rendering. App owns orchestration and side effects, but delegates typed behavior to focused modules.

## Patterns To Adopt

- Keep binary entrypoints small and move application state, input, commands, and feature actions into focused modules.
- Parse user input into typed commands before validation or mutation.
- Separate pure state transitions from terminal, filesystem, MIDI, audio, and background-task side effects.
- Keep rendering organized by view and reusable widget, with immutable inputs.
- Isolate transport, scheduling, MIDI dispatch, audio dispatch, and fake backends.
- Give configuration, keymaps, and user preferences a validated ownership boundary.
- Protect refactors with small golden fixtures, snapshots, and deterministic fake backends.
- Enforce dependency direction and module budgets mechanically in CI.

## Patterns Not To Adopt Yet

- A plugin runtime, scripting language, or dynamic actor registry before a concrete product requirement and ADR.
- A virtual filesystem or file-watcher architecture beyond project and sample-library needs.
- An application-wide untyped event bus; domain-specific typed actions remain preferred.
- Async infrastructure in pure model, rendering, or transformation code.
- Additional crates created only to reduce line counts when an internal module provides a clear boundary.
- Exact structural parity with Yazi; its file manager domains and plugin lifecycle are not Salieri requirements.

## Delivery Order

### 1. Guardrails

- [#101](https://github.com/delaudio/salieri-tracker/issues/101): file and module size budgets.
- [#113](https://github.com/delaudio/salieri-tracker/issues/113): dependency direction and ownership rules.
- [#112](https://github.com/delaudio/salieri-tracker/issues/112): refactor-safe fixtures and snapshots.

The first two controls prevent new debt. The fixture work supplies behavioral evidence before code movement begins.

### 2. Typed Boundaries

- [#102](https://github.com/delaudio/salieri-tracker/issues/102): typed command parser and executor.
- [#114](https://github.com/delaudio/salieri-tracker/issues/114): configuration and preferences boundary.
- [#104](https://github.com/delaudio/salieri-tracker/issues/104): app event and action dispatcher.
- [#110](https://github.com/delaudio/salieri-tracker/issues/110): cancellable task runtime.
- [#106](https://github.com/delaudio/salieri-tracker/issues/106): configurable keymap layers.

Typed boundaries should land before moving their behavior out of large modules so the extracted APIs are intentional and independently testable.

### 3. Module Decomposition

- [#111](https://github.com/delaudio/salieri-tracker/issues/111): isolate interop formats and diagnostics.
- [#109](https://github.com/delaudio/salieri-tracker/issues/109): isolate audio backend, sampler, DSP, render, and export code.
- [#108](https://github.com/delaudio/salieri-tracker/issues/108): isolate playback transport and scheduling.
- [#105](https://github.com/delaudio/salieri-tracker/issues/105): split TUI views and widgets.
- [#107](https://github.com/delaudio/salieri-tracker/issues/107): split core model domains without changing serialization.
- [#103](https://github.com/delaudio/salieri-tracker/issues/103): finish app decomposition around the typed boundaries above.

Each refactor should be behavior-preserving, reduce or remove its baseline exception, keep public APIs stable where practical, and pass all fixture, snapshot, format, test, and lint gates.

## Feature Planning Gate

Before adding a cross-cutting feature, its issue or design note should answer:

1. Which crate owns the data and behavior?
2. Which existing dependency edges does it use, and does it require a policy change?
3. Which large modules would it otherwise grow, and which focused module should receive it?
4. What deterministic test, fixture, or snapshot proves the behavior?
5. Does it need typed actions, background tasks, persistence migration, or realtime constraints?

Feature work that would grow a baselined file should first complete or advance the associated split issue. Architectural exceptions must be explicit in the versioned policy or baseline and linked to a tracking issue.

## Completion Signals

The roadmap is complete when all six hard-budget exceptions are removed, the app entrypoint is an orchestrator, major domains have focused tests, and new features can be placed without adding forbidden edges or expanding a monolithic module. The CI guards remain permanent even after the current refactor issues close.
