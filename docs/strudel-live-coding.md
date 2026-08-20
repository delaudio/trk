# Strudel Mini-Notation and Live Coding

trk evaluates a bounded, row-quantized subset of Strudel/TidalCycles
mini-notation directly into the active pattern. The generated notes are normal
tracker cells, so saving, undo, piano-roll editing, MIDI output, and existing
playback all see the same data.

Apply an expression once with command mode:

```text
:strudel note("c3 [eb3 g3] bb3*2").euclid(5,16)
:strudel s("bd [sd cp] bd*2 [~ sd]")
:strudel [c4 e4 g4, c3*2]
```

The active track receives the first layer. Comma-separated layers use the
following tracks, and evaluation fails without changing the pattern when there
are not enough tracks.

## Supported notation

- `a b c` divides one cycle evenly between events.
- `[a b c]` subdivides the surrounding step.
- `[a, b]` creates concurrent layers on consecutive tracks.
- `a*4` repeats an event four times inside its step.
- `a/2` extends an event gate across two source spans; the gate is clipped at
  the pattern boundary.
- `~` is a rest.
- `<a b c>` selects an alternate value by evaluation cycle.
- `a(3,8)` and `.euclid(3,8,1)` apply a Euclidean mask with optional rotation.
  Zero pulses are valid and produce an all-rest mask.
- A token suffix such as `c4@2` writes instrument 2.

Named pitches use MIDI octave notation (`c4` is middle C); flats and sharps are
accepted. Numeric tokens are zero-based scale degrees and require a scale:

Quoted wrappers do not support backslash escape sequences; enter notation
tokens directly between the quotes.

```text
:strudel note("0 2 4 7").scale("d:minor")
```

Supported modes are major/ionian, minor/aeolian, dorian, mixolydian, and
pentatonic. The `s(...)` form maps common drum names (`bd`, `sd`, `cp`, `hh`,
and `oh`) to General MIDI percussion pitches.

## Live editor

Open the bottom live-coding bar with an optional starting expression:

```text
:strudel live note("c3 [eb3 g3] bb3*2")
```

Every valid keystroke recalculates the preview from the entry snapshot. During
pattern playback, the scheduler adopts the latest valid pattern at a row
boundary without stopping transport. Invalid intermediate syntax shows an
inline error and leaves the last valid preview sounding.

- `Enter` accepts the current valid preview as one undoable edit. If the
  visible expression is invalid, the editor stays open until it is fixed.
- `Escape` restores both the entry cells and the playback schedule without
  adding undo history.

Events are quantized to tracker rows. If two generated onsets would occupy the
same track and row, evaluation reports that the expression needs more pattern
rows instead of silently dropping an event. This evaluator does not execute
JavaScript, load Strudel packages, define synthesizers, or store sub-row events.

The existing [Strudel export](strudel-export.md) remains the path in the other
direction; arbitrary tracker patterns are not compacted back into source by the
live editor.
