# Ableton Live bridge

The Ableton bridge is an optional terminal workflow for mapping trk clip
launcher state to Ableton Live Session View operations. The current
implementation is intentionally dry-run only: it produces push, pull, and clear
plans without requiring Live, a control script, or a network/session transport.

Use command mode:

```text
:ableton push --dry-run [scene N] [track N]
:ableton pull --dry-run [scene N] [track N]
:ableton clear --dry-run [scene N] [track N]
```

The aliases `:live ...` and `:bridge ...` use the same command surface. `scene`
is 0-based to match the clip launcher command model. `track` is 1-based to match
the visible tracker track labels.

Non-dry-run commands fail with a clear diagnostic and do not mutate the project.
This keeps Ableton integration unavailable unless a future bridge transport is
explicitly configured.

## Session mapping

| trk source | Ableton Session View target |
| --- | --- |
| Clip scene index | Scene row |
| Track order | Track column |
| Clip slot pattern reference | Session clip payload for that scene/track cell |
| Clip `start_row..end_row` | Tracker row range represented by the session clip |
| Empty clip slot | No-op for push |
| Muted track | Planned with a mute diagnostic so Live can preserve muted intent |

`push` plans outbound session clips from existing trk clip slots. It refuses
to run when no clip scenes exist because there is no unambiguous scene grid to
export.

`pull` plans inbound session clips into tracker patterns or clip launcher slots.
In dry-run mode it never creates patterns and never edits clip scenes; the plan
only describes the target cells.

`clear` plans deletion of selected Ableton session clips. In dry-run mode it
never clears Ableton clips or trk clip slots.

Bridge reports are written to the in-app assistant thread so the full plan can
be reviewed after the short notification disappears.
