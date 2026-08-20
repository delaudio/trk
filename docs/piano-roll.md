# Piano Roll

Open the horizontal Piano Roll with `:view roll`; `Esc` or `:view tracker`
returns to the tracker. Both views edit the same pattern cells, so switching
views is lossless and every change participates in the normal undo/redo history.
Existing `Tab`, `F1`, and `F2` tracker behavior is unchanged.

The vertical axis is chromatic pitch and the horizontal axis is pattern rows.
The active track uses solid note bars, other tracks can appear as dim ghost
notes, and the playback row is shown as a moving playhead.

| Key | Action |
| --- | --- |
| Arrows | Move one pitch or row |
| Space | Insert/remove the note at the exact cursor cell |
| Shift+Left/Right | Shorten/extend its gate |
| Alt+Arrows | Move the complete source cell when the destination is free |
| 1–8 | Set velocity to 10–80% |
| 9 | Set velocity to 100% |
| `[` / `]` | Select 16, 32, or 64 visible rows |
| `g` | Toggle companion-track ghost notes |

An explicit gate lasts that many rows from the delayed note onset. A project
cell without `gate` keeps the legacy behavior: sustain until the next note,
note-off/cut, or pattern end. The full tracker field layout exposes Gate as a
two-digit hexadecimal column for exact editing.

The local Web Companion presents the same notes on a high-resolution Canvas and
adds pointer editing plus stepped MIDI CC curves. See [Web Companion](web-companion.md).
