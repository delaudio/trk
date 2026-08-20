# Undo And Redo Transactions

trk owns undo and redo in `trk-app`. Persistent musical state is edited
through a staged `SongTransaction`; the live song changes only after the whole
transaction succeeds. A failed outer transaction is discarded, and a failed
nested transaction restores its local checkpoint before control returns to the
outer transaction.

Each committed transaction records a reversible `SongPatch` containing the
before and after song states. Multi-cell paste, imports, AI proposals, sampler
changes, mixer and device parameters, and project-structure commands therefore
remain atomic even when they touch several model objects. Runtime events such as
playback position, MIDI connection state, dialogs, and selections are not part
of project history or serialization.

Repeated tracker typing and continuous parameter adjustments use stable merge
keys. Consecutive edits with the same key retain the first before-state and the
latest after-state, so one undo restores the value from before the adjustment.
Any new committed edit clears redo history.

The `[history] undo_limit` configuration setting bounds retained transactions.
It defaults to 100 and accepts values from 1 through 10000. Loading another
project clears both history directions. Dirty state is derived by comparing the
current song with the last successfully saved or loaded song, so undoing back to
that content clears the dirty indicator and redo restores it.

Generated pattern variations have a separate, persistent snapshot history. A
successful AI proposal application or Euclidean transform records the resulting
pattern, its source, description, target pattern and optional track. No-op and
failed operations do not create entries. The history retains the newest 64
entries in each project.

In normal mode, `v` opens the variation browser, Up/Down (or `k`/`j`) moves the
selection, and Enter restores the selected pattern through the ordinary undo
transaction stack. `V` starts visual selection and `Ctrl+v` continues to paste.
Undo, redo, or ordinary edits reconcile the active badge against the current
pattern contents.
