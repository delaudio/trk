---
adr: 0033
title: Lock QWERTY note entry to musical scales
status: Accepted
date: 2026-08-21
owner: default-agent
supersedes:
superseded-by:
depends-on: [0020, 0030, 0031]
tags: [harmony, scales, chords, qwerty, tui, input, playback]
---

# ADR 0033 — Lock QWERTY note entry to musical scales

## Context

The tracker exposes a chromatic two-row computer-keyboard layout for step
entry. That layout is fast once memorized, but it does not help a performer
stay inside a chosen key and the status bar does not describe the harmony
formed by simultaneously sounding monophonic tracks. Issue #322 requests an
OP-Z/M8-inspired Scale Lock toggle, configurable roots and modes, and a live
chord name derived from the notes that are actually active during playback.

Scale definitions and chord recognition are music-domain behavior rather than
TUI-only lookup tables. The existing Strudel evaluator already accepts a
small scale vocabulary, so Scale Lock must share one canonical interval
catalog instead of introducing a conflicting interpretation of major, minor,
or Dorian degrees. The tracker remains one-note-per-track; polyphonic harmony
continues to arise from concurrent tracks and resolved gate durations.

Scale Lock is an input aid, not a property of the generated notes. Its enabled
state, selected root, and selected mode are session workflow state: changing
them does not dirty the project or rewrite existing cells. Inserted notes stay
ordinary canonical MIDI pitches and therefore remain portable through every
existing editor, import/export path, undo operation, and playback engine.

## Capability statement

`trk` can constrain its two-row QWERTY note-entry keyboard to a configurable
musical scale and can identify the chord formed by audible, sustained notes at
the live playback row, while preserving canonical MIDI note storage and the
responsive status-line contract.

## User stories / scenarios

- As a keyboard-first composer, I want every note-entry key to produce a scale
  degree in my selected key, so that fast step entry cannot introduce an
  accidental out-of-scale pitch.
- As a performer, I want the status line to name the harmony sounding across
  tracks, so that I can understand inversions and extensions while playback
  advances.
- As an existing project user, I want Scale Lock to write normal MIDI pitches,
  so that disabling it or reopening material never changes recorded notes.

## Acceptance criteria

1. A reusable harmony model exposes pitch-class parsing/formatting and stable
   interval definitions for major, natural minor (`minor`), Dorian,
   Hirajoshi, and major pentatonic (`pentatonic`) scales. Degree quantization
   handles positive octave crossings within MIDI `0..=127`, rejects pitches
   outside that range, and supplies the same canonical intervals to Strudel's
   overlapping scale names without changing existing mini-notation output.
2. Session state defaults to Scale Lock off with C major selected. Exact `K`
   from Normal or Edit mode toggles it without moving the cursor or dirtying
   the song. `:scale` reports the state; `:scale on`, `:scale off`, and
   `:scale toggle` change only enablement; and `:scale ROOT MODE` validates,
   selects, and enables a root/mode such as `D minor`, `F dorian`,
   `A hirajoshi`, or `C pentatonic`. Invalid input leaves the prior state
   unchanged and reports actionable usage.
3. While Scale Lock is enabled, the existing lower `zsxdcvgbhnjm` and upper
   `q2w3er5t6y7u` piano rows map their stable physical positions to successive
   valid degrees, with the upper row beginning at the next scale octave. Step
   entry remains undoable and preserves octave controls, velocity, edit-step,
   and cursor behavior. No mapped key can emit an off-scale or out-of-range
   pitch; disabling the mode restores the existing chromatic mapping exactly.
4. A deterministic chord identifier deduplicates pitch classes, preserves the
   lowest pitch for inversion disambiguation, and recognizes at least major,
   minor, diminished, augmented, suspended second/fourth, sixth, dominant
   seventh, major seventh, minor seventh, half-diminished seventh, diminished
   seventh, ninth, major ninth, and minor ninth templates. Canonical sharp
   names include results such as `Dm7`, `Fmaj9`, `Gsus4`, and `C#dim7`;
   fewer than three distinct pitch classes or unmatched sets produce no name.
5. Live chord input is derived from every audible track at the current
   playback row using the canonical pattern note, NoteOff/NoteCut, explicit
   gate, legacy sustain, replacement, mute, and solo semantics. It updates on
   each playback position, clears when transport stops or errors, and never
   mutates song or runtime state.
6. The Pattern status line renders a compact enabled-scale indicator and the
   current recognized chord ahead of passive shortcuts while retaining the
   width-aware atomic-segment behavior. Command input and notifications still
   replace the passive line, and absent/unrecognized chords render no
   placeholder or stale name.
7. Help and user documentation describe the toggle, command syntax, supported
   scales, physical-key mapping, session-only behavior, and chord-display
   limits without exposing internal ADR identifiers.
8. Unit and application tests cover scale parsing/quantization and bounds,
   chromatic restoration, command rollback, exact key scope, note insertion
   and undo, chord templates and inversions, gate/mute/solo active-note
   resolution, stopped/error clearing, and responsive status composition. The
   complete repository verification gate and Norn review pass.

## Out of scope

- Changing external MIDI input pitches, adding MIDI learn, pitch correction
  for existing cells, or quantizing imported material.
- More than one simultaneous note in a track cell, a chord-entry macro,
  arpeggiator, voice-leading engine, chord substitution, or key detection.
- Persisting Scale Lock workflow state in project files or configuration, or
  embedding scale metadata in exported MIDI/MusicXML.
- Naming every possible jazz alteration, polychord, slash chord, or incomplete
  two-note shell; unmatched pitch-class sets intentionally remain unnamed.
- Adding computer-keyboard audio audition or a new live-recording transport
  path beyond the existing QWERTY step-entry boundary.

## Open questions

- None.

## References

- `0020-compose-contextual-status-hints-by-width.md`
- `0030-edit-patterns-through-synchronized-piano-rolls.md`
- `0031-live-code-patterns-with-mini-notation.md`
- `../plan/todo/0031-scale-lock-and-chord-identification.md`
- GitHub issue #322.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-08-21 | r1 | default-agent | Recorded and accepted canonical scale-locked QWERTY entry and live audible-row chord identification. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-08-21 | Approved autonomous resolution of the prioritized GitHub issue queue in chat. |
