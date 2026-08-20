---
adr: 0031
title: Live-code patterns with mini-notation
status: Accepted
date: 2026-08-20
owner: default-agent
supersedes:
superseded-by:
depends-on: [0001, 0025]
tags: [strudel, live-coding, patterns, parser, transforms, tui, playback]
---

# ADR 0031 — Live-code patterns with mini-notation

## Context

`trk` can generate a fixed Euclidean rhythm and export tracker data as a
Strudel browser sketch, but it cannot evaluate the compact pattern language
used while composing. Issue #320 asks for nested subdivisions, repetition,
slowing, rests, cycle alternation, Euclidean distribution, scale-aware pitch
input, an atomic command, and an interactive live-coding workflow.

The parser and evaluator are reusable music-domain behavior rather than TUI
state. Evaluation must remain deterministic and bounded by the target pattern,
must mutate the canonical `PatternCell` matrix instead of creating a parallel
event store, and must report syntax or evaluation errors without partially
changing a pattern. During playback, accepted edits need to replace future
pattern rows at a scheduling boundary without rebuilding the audio runtime.

The supported timing semantics are row-quantized and explicit: a top-level
space-separated sequence divides a cycle evenly; `[a b]` divides its parent's
span between `a` and `b`; `[a,b]` evaluates both branches concurrently;
`a*N` repeats `a` N times inside its span; `a/N` stretches `a` across N cycles;
`<a b>` selects `a` on cycle zero, `b` on cycle one, then wraps; and
`a(p,s,r)` gates `a` with the same deterministic `s`-step Euclidean mask as
the existing transform, rotated right by `r` steps. For example, over eight
rows, `c4*2 ~` starts C4 on rows 0 and 2, while `<c4 d4>` starts only C4 in
the first evaluation cycle and only D4 in the next.

## Capability statement

`trk` evaluates a documented, deterministic subset of TidalCycles/Strudel
mini-notation into canonical tracker cells and provides atomic command and
interactive live-coding workflows whose valid edits can update the active
pattern and playback schedule without interrupting the transport.

## User stories / scenarios

- As a composer, I want to express subdivisions, repetition, rests,
  alternation, Euclidean rhythms, and scales compactly, so that I can generate
  useful tracker material without entering every row manually.
- As a live coder, I want valid text edits to update the active pattern while
  it plays, so that I can hear successive ideas without stopping transport.
- As a tracker user, I want invalid expressions to preserve the previous
  pattern, so that exploratory typing cannot destroy musical data.

## Acceptance criteria

1. A reusable parser exposes a typed AST for nested `[]` subdivisions,
   comma-separated concurrent layers, `*N` fast repetition, `/N` slowing,
   `~` rests, `<...>` cycle alternation, and Euclidean `(p,s[,r])` syntax,
   with source-positioned errors and bounded nesting and numeric arguments.
2. Deterministic evaluation over a requested positive row count expands the
   AST into canonical note, velocity, gate, and instrument-aware cell writes;
   concurrent layers map to consecutive tracks and never write outside the
   pattern or available track range.
3. Pitch tokens accept MIDI note names and scale degrees followed by
   `.scale("root:mode")`; supported scales quantize degrees to exact MIDI
   semitones and malformed notes, scales, or out-of-range pitches fail without
   partial output.
4. `:strudel EXPR` evaluates against the active pattern and track as one
   undoable mutation, clears only the addressed output tracks, reports a
   concise result, and preserves the song when parsing or evaluation fails.
5. `:strudel live [EXPR]` opens a dedicated bottom editing bar. Each valid
   edit replaces canonical preview cells derived from the entry snapshot and
   updates the active playback schedule. Invalid intermediate text leaves the
   last valid preview audible and displays an error. Enter commits the latest
   valid preview as one undoable mutation; Escape restores both canonical
   cells and the active playback schedule to the entry snapshot without adding
   undo history.
6. While pattern playback is active, a valid live edit replaces future
   scheduled rows at a bounded row boundary without stopping transport,
   reconstructing the playback runtime, or producing a `Stopped` update.
7. Tests cover AST/error behavior, every supported operator, nested patterns,
   layers, alternation cycles, Euclidean rotation, scale mapping, cell
   generation, command rollback/undo, live accept/cancel/error handling, and
   uninterrupted playback schedule replacement. The repository verification
   gate passes.

## Out of scope

- Full JavaScript Strudel evaluation, arbitrary TidalCycles functions, custom
  user functions, network-loaded packages, or executing untrusted code.
- Sub-row tracker storage, more than one event in a track/row cell, sample
  loading, synthesizer definition, effects, control patterns, or OSC.
- Bidirectional extraction of arbitrary tracker patterns into minimal source;
  the existing deterministic Strudel export remains the supported output path.
- Persisting the live editor buffer or source expression in project files.

## Open questions

- None.

## References

- `0001-record-architecture-decisions.md`
- `0025-browse-and-restore-persistent-pattern-variations.md`
- `../plan/todo/0029-strudel-mini-notation-live-coding.md`
- [Strudel mini-notation](https://strudel.cc/learn/mini-notation/)
- GitHub issue #320.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-08-20 | r1 | default-agent | Recorded and accepted bounded mini-notation parsing, canonical evaluation, atomic commands, and uninterrupted live updates. |
| 2026-08-20 | r2 | default-agent | Defined row-quantized operator behavior and transactional live preview rollback for canonical cells and playback. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-08-20 | Approved autonomous resolution of the prioritized GitHub issue queue in chat. |
