# Automation

Automation is pattern-local and tracker-native. The first implemented lane type
is stepped sample-gain automation for sampler-backed tracks.

Commands:

```text
:automation sample-gain VALUE
:automation sample-gain ROW VALUE
:automation sample-gain clear
:automation sample-gain clear ROW
```

`VALUE` is a non-negative gain value. Without an explicit row, the command uses
the cursor row. The target sample is resolved from the current track assignment.

Semantics:

- interpolation is stepped;
- each point applies from its row until the next point for the same target;
- rows before the first point use the sample reference gain;
- realtime playback and offline audio export observe the same scheduled sampler
  event gain;
- automation lanes are saved inside each pattern in `.salieri` project files.

Current limitations:

- only sample gain can be automated;
- there is no graphical automation editor yet;
- linear interpolation and plugin parameter automation are future work.
