# Style Analysis and Comparison

Salieri can analyze `.salieri` projects into deterministic style summaries:

```bash
salieri analyze song.salieri
salieri analyze song.salieri style.json --format json
salieri compare draft.salieri final.salieri
salieri compare draft.salieri final.salieri compare.json --format json
```

Analysis covers:

- tempo, track count, pattern count, and sequence length;
- note-cell count and active-track count;
- note density and average velocity;
- pitch range;
- coarse energy classification;
- per-track role inference for empty, percussion, bass, lead, and harmony lanes.

Comparison reports include both project analyses plus deltas for tempo, note
count, active tracks, and density.

Inside the TUI:

```text
:analyze
:compare path/to/other.salieri
```

The TUI commands do not mutate the project. They append the full report to the
AI thread and show a compact status-line summary.
