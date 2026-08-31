# Scale Lock and chord identification

Owning ADR: `../../adr/0033-lock-qwerty-note-entry-to-musical-scales.md`

GitHub issue: #322

## Scope

Add a canonical scale and chord vocabulary, route the existing two-row QWERTY
note-entry surface through session-only Scale Lock state, expose validated
`:scale` control and an exact `K` toggle, derive audible sustained pitches at
the live playback row, and render recognized chords plus the active scale in
the responsive Pattern status line. Reuse the scale catalog from Strudel and
keep stored tracker notes as ordinary MIDI pitches.

## Exit criteria

1. Canonical scales, degree quantization, bounds, and Strudel reuse satisfy ADR
   AC1.
2. Session defaults, exact toggle scope, typed command behavior, and rollback
   satisfy ADR AC2.
3. Both physical keyboard rows, chromatic restoration, and undoable canonical
   step entry satisfy ADR AC3.
4. Chord templates, canonical naming, inversion disambiguation, and unmatched
   behavior satisfy ADR AC4.
5. Gate-aware audible-row resolution and transport lifecycle satisfy ADR AC5.
6. Responsive scale/chord status priority and command/notification precedence
   satisfy ADR AC6.
7. Help and public documentation satisfy ADR AC7.
8. Focused core, application, runtime, and render tests plus the complete
   repository verification gate and Norn review satisfy ADR AC8.

## Dependencies

- `../../adr/0020-compose-contextual-status-hints-by-width.md`
- `../../adr/0030-edit-patterns-through-synchronized-piano-rolls.md`
- `../../adr/0031-live-code-patterns-with-mini-notation.md`
- Maintainer approval to execute issue #322 autonomously.

---

Shipped at HEAD `0aabc9b6f96f9593b98f951f1003d0cc7d60967f` via
[PR #352](https://github.com/delaudio/trk/pull/352), with GitHub Actions CI
run `33398013486` green and issue #322 closed.
