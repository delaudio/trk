# Strudel Export

trk can export tracker patterns or the current sequence to a deterministic
Strudel-oriented JavaScript sketch:

```bash
trk export strudel song.trk strudel.js --pattern 1
trk export strudel song.trk strudel.js --patterns 1,2
trk export strudel song.trk --sequence
```

If no output path is provided, the sketch is printed to stdout.

The export includes:

- tempo as `setcps(bpm/60/linesPerBeat)`;
- one `stack(...)` lane per trk track;
- track names as stable comments and deterministic `s("track_nn_name")` labels;
- note tokens converted from MIDI pitch names such as `c4` and `fs3`;
- velocity tokens normalized to `0.00..1.00`;
- volume columns as `.gain(...)` when present;
- pan columns as `.pan(...)` when present;
- pattern length metadata in comments.

Unsupported or lossy features are listed in `// diagnostics:` comments. Current
diagnostics cover clip launcher slots, sampler/sample assignments, mixer state,
native effects, automation lanes, instrument columns, delay/gate fields, tracker
commands, parameter locks, swing, and note-off/cut events.

The workflow is export-only. It does not change the canonical `.trk` project
model and does not embed sample audio or external instruments into Strudel.
