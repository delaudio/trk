# Scale Lock and live chord names

Scale Lock constrains computer-keyboard note entry to a selected root and
mode. Press uppercase `K` in normal or edit mode to toggle it. Configure it in
command mode:

```text
:scale
:scale D minor
:scale F dorian
:scale A hirajoshi
:scale C pentatonic
:scale on
:scale off
:scale toggle
```

`:scale` reports the current state. Selecting a root and mode also enables the
lock. Supported modes are `major`, `minor`, `dorian`, `mixolydian`,
`hirajoshi`, and major `pentatonic`; common short names and flat root spellings
are accepted. The status line shows an enabled lock compactly, for example
`K:D:min`.

The lower physical row `z s x d c v g b h n j m` enters successive degrees
from the selected root. The upper row `q 2 w 3 e r 5 t 6 y 7 u` starts at the
same root one scale octave higher and continues through successive degrees.
Changing the tracker octave still moves the whole mapping. A degree beyond the
MIDI range is inert rather than wrapping.

Scale Lock is session-only input state. It does not dirty a project and is not
saved. Notes entered while it is enabled are ordinary MIDI pitches, so they
remain unchanged in the tracker, piano roll, playback, imports, and exports
after the lock is disabled.

During playback, the Pattern status line names recognized harmony across all
audible tracks at the current row. Explicit gates, legacy sustain, replacement
notes, NoteOff/NoteCut, mute, and solo determine which pitches participate.
The recognizer covers common triads, suspended chords, sixths, sevenths, and
ninths, including names such as `Dm7`, `Fmaj9`, `Gsus4`, and `C#dim7`.
Duplicate octaves are ignored. Two-note shells, altered chords outside the
documented vocabulary, and unmatched pitch-class sets show no placeholder.
