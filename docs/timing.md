# Sequencer Timing

The playback runtime is intentionally separate from the Ratatui render loop. The app sends semantic playback commands to a dedicated thread, and that thread schedules row deadlines with `std::time::Instant`.

## Row Duration

Rows are derived from transport settings:

```text
row_duration = 60_000_000 micros / BPM / lines_per_beat
```

The core clamps zero BPM or LPB values to one for defensive loading and validation paths. Normal project validation still requires musical values in the accepted project range.

## Runtime Assumptions

- The TUI may render slowly or stop polling playback updates temporarily.
- Playback position updates are delivered through a channel and are not required for MIDI scheduling to advance.
- MIDI events are sent at their row deadline from the playback thread.
- Stop, panic, MIDI disconnect, and shutdown paths attempt All Notes Off before clearing runtime state.

## Jitter Target

The MVP target is stable enough for external MIDI sequencing while editing from the terminal, not hard realtime audio scheduling. Runtime tests use a 50 ms row duration and allow 35 ms drift per measured row interval. That tolerance covers CI and workstation scheduler noise while still catching regressions that accidentally tie playback to UI ticks or block row advancement.

Observed jitter depends on the operating system scheduler, terminal load, MIDI backend, and destination application. Future work can add scheduling lookahead, per-event timestamps where supported, and external MIDI clock sync.
