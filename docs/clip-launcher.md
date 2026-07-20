# Clip launcher

The clip launcher is a scene x track grid above tracker patterns. It is distinct
from the F7 song-slot view: song slots are a linear arrangement, while clip
scenes are launchable groups of per-track pattern references.

Open it from command mode:

```text
:clips
```

Useful commands:

```text
:clip add                  Add a scene from active tracks in the current pattern
:clip set                  Set the selected scene/track to the current pattern
:clip set SCENE TRACK PAT  Set a specific 0-based scene, 1-based track, 1-based pattern
:clip clear                Clear the selected scene/track
:clip launch scene SCENE   Queue a scene for the next boundary
:clip commit               Mark the queued scene active
:clip stop                 Clear active and queued clip state
```

Inside the clip view, arrow keys select scene/track, `A` adds a scene, `T` sets
the selected clip, `R` clears it, `Enter` queues the selected scene, and `Space`
activates the queued scene. The visible states are:

- `■` stopped clip
- `A` active clip
- `Q` queued clip
- `·` empty slot
- `M` muted track

Clip actions store pattern references and do not duplicate, alter, or destroy
the underlying patterns. Launch state is runtime-only. Ableton Live
push/pull/clear plans are available as an optional dry-run terminal bridge; see
[Ableton Live bridge](ableton-live-bridge.md).
