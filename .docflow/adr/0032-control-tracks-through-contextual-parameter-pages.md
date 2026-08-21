---
adr: 0032
title: Control tracks through contextual parameter pages
status: Implemented
date: 2026-08-20
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003, 0015, 0016, 0017, 0025, 0031]
tags: [performance, tui, parameters, p-locks, sampler, dsp, live]
---

# ADR 0032 — Control tracks through contextual parameter pages

## Context

`trk` exposes sampler, mixer, DSP, transform, and live-coding capabilities,
but many of them require a modal editor or a typed command. Issue #321 asks
for an instrument-like performance surface with six predictable pages, eight
fixed physical controls, row-local parameter locks, fast track muting, and a
temporary save/reload gesture that does not interrupt playback.

The workflow must not create a second sound model. Every adjustable slot needs
to resolve to an existing canonical parameter descriptor and
`ParameterLockTarget`, with the current row's lock as its displayed value.
Parameters or engines that are not available for the active track remain
visible and explicitly disabled instead of accepting edits that cannot affect
playback. Command mode remains the complete administrative and expert surface.

Page shortcuts need a context of their own because their encoder keys overlap
normal tracker entry. The page view therefore preserves the tracker cursor but
owns its keyboard and render-generated pointer targets while open. A repeated
press of the active page key routes to the closest existing deep editor rather
than duplicating those editors inside the matrix.

Temporary performance snapshots are session-local and cover the complete
canonical song so pattern cells, locks, sampler state, mixer state, and DSP
routing cannot be restored inconsistently. Reload applies immediately while
stopped and at the first row on or after the next beat boundary while playing;
the scheduler replaces its song and audio graph without emitting a stopped
transition.

## Capability statement

`trk` provides a six-page, eight-control performance surface that edits
canonical row parameter locks, communicates availability and lock state
visually, routes into existing deep editors, and offers transport-safe
temporary song reload and instant track mutes.

## User stories / scenarios

- As a performer, I want the same page and encoder keys to expose relevant
  parameters on every track, so that I can shape sound without typing commands.
- As a tracker user, I want an encoder adjustment to lock only that parameter
  on the cursor row, so that notes and other step data remain intact.
- As a live performer, I want to save an exploratory state and restore it on a
  beat boundary, so that I can create a reliable drop without stopping audio.

## Acceptance criteria

1. A typed `ParameterPage` model exposes exactly `SRC`, `FLTR`, `AMP`, `FX`,
   `LFO`, and `ALG` in stable order with eight stable encoder slots. `F1`–`F6`
   enter or switch the page surface, and repeating the active page shortcut
   opens that page's existing deep editor.
2. Encoder slots resolve dynamically from the active track and song to
   canonical parameter descriptors and `ParameterLockTarget` values. Missing,
   non-automatable, or inapplicable bindings are visibly disabled and all
   keyboard and pointer operations on them are inert.
3. The responsive page renderer shows the six tabs, a two-by-four grid with
   `QWER`/`ASDF` keys, labels, formatted values and meters, selection, disabled
   reasons, and an LED-style indication plus summary tag for every lock on the
   cursor row. Its click and wheel regions are registered in the render-owned
   interaction map.
4. Within the page surface, `QWER`/`ASDF` select slots; arrows and `+`/`-`
   adjust by a descriptor-aware fine step and Shift applies a coarse step.
   Adjusting an enabled slot upserts exactly one canonical parameter lock on
   the active track and row as an undoable merged edit. The Backspace removal
   gesture clears only the selected parameter lock and preserves the note,
   cell metadata, and all other locks.
5. Pointer click selects the exact rendered slot and wheel input adjusts only
   the exact enabled slot under the pointer. Stale, clipped, disabled, or
   absent targets do not mutate the song.
6. Repeating a page shortcut routes `SRC` to the sampler workspace,
   `FLTR`/`AMP`/`FX`/`LFO` to the DSP rack, and `ALG` to the live mini-notation
   editor. Escape closes the matrix without moving the tracker cursor.
7. `Shift+S` stores one session-only snapshot of the complete canonical song.
   `Shift+R` restores it atomically and undoably immediately while stopped or
   at the next beat boundary while playing. A playing reload updates the
   scheduler's pattern events and audio graph without reconstructing the
   runtime or emitting `Stopped`; absent snapshots are inert with feedback.
8. `Shift+1` through `Shift+8` toggle the corresponding existing track's mute
   state immediately from the normal or page workflow without changing page,
   focus, or cursor. Shortcuts for absent tracks are inert.
9. Tests cover page order and binding resolution, keyboard and pointer input,
   disabled slots, lock upsert/removal and undo, responsive rendering, deep
   editor routing, snapshot restore at a beat boundary, uninterrupted
   playback, and quick mute. The complete repository verification gate passes.

## Out of scope

- MIDI controller learning or mapping; physical MIDI integration is handled by
  a separate capability.
- Adding synthesizers, DSP algorithms, modulation engines, polyphony, or
  multi-velocity sample layers solely to fill an otherwise unsupported slot.
- Persisting the active page, selected encoder, or temporary snapshot in a
  project file, or keeping more than one temporary snapshot.
- Instant mute shortcuts beyond the first eight tracks or replacing command
  mode for administrative, import, export, and project operations.

## Open questions

- None.

## References

- `0003-expose-render-owned-interaction-regions.md`
- `0015-select-dsp-targets-and-device-rows-with-the-pointer.md`
- `0016-select-dsp-parameters-and-palette-entries-from-rendered-rows.md`
- `0017-control-supported-sampler-actions-with-the-pointer.md`
- `0025-browse-and-restore-persistent-pattern-variations.md`
- `0031-live-code-patterns-with-mini-notation.md`
- `../plan/done/2026-08-21-contextual-parameter-pages.md`
- GitHub issue #321.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-08-20 | r1 | default-agent | Recorded and accepted canonical contextual pages, row locks, deep routing, and transport-safe performance controls. |
| 2026-08-21 | r2 | default-agent | Marked the capability Implemented after PR #350 merged with the complete gate green. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-08-21 | Approved autonomous resolution and verified delivery through the merged implementation pull request. |
