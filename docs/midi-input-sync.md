# MIDI Input And Sync

The MIDI output path remains in `salieri-midi::output`; input is separated in
`salieri-midi::input` so input failures cannot corrupt output playback state.

The input path provides:

- parsed channel voice events for note on/off, CC, and program change;
- MIDI realtime clock events for clock/start/continue/stop;
- `MidiInputPacket` timestamps for deterministic recording tests;
- `FakeMidiInput` for hardware-free tests;
- real `midir` input port listing and connection;
- command-mode note-on recording into the current pattern;
- persisted MIDI routing settings for clock, transport, notes, CC, channel
  filters, middle C calibration, sync delay, and recording options.

## Usage

List input ports from the CLI:

```bash
salieri --list-midi-inputs
```

Connect and record from command mode:

```text
:midi-input ports
:midi-input connect 0
:midi-input record on
:midi-input record off
:midi-input record velocity off
```

Configure MIDI routing from command mode:

```text
:midi-input clock in on
:midi-input transport in on
:midi-input notes in on
:midi-input notes out off
:midi-input cc in on
:midi-input channel in 1,10
:midi-input channel out all
:midi-input middle-c 60
:midi-input sync-delay -12
```

Configure an input to auto-connect by name:

```toml
[midi]
default_input = "IAC Driver Bus 1"
```

## Boundaries

MIDI input does not mutate `salieri-core` directly. Realtime recording translates
input packets into app-level edits, then applies normal undoable song mutations.
Unsupported messages, parse failures, input list/connect failures, and device
disconnects are reported as app status/notifications without affecting MIDI
output state.

Recording is intentionally simple: note-on events are written to the current
cursor cell, then the cursor advances by the current edit step. Incoming
velocity is recorded when `midi.recording.velocity` is enabled; otherwise the
default tracker velocity is used. This is the first quantization boundary: the
active row is the quantized destination.

Clock and transport are separate persisted routes. `clock in` accepts timing
clock ticks for status/sync accounting, while `transport in` follows
start/continue/stop. The legacy `:midi-input clock on|off` command toggles both
inbound clock and inbound transport for compatibility with earlier workflows.

Channel filters are empty for "all channels" or a list of user-facing MIDI
channels `1..16`. Input filters gate recorded note and CC events. Output filters
gate external MIDI note playback; internal sampler/audio playback is not
disabled by MIDI output filters.

## Current Limitations

- no MIDI learn mapping table;
- CC routing is persisted and filtered, but CC recording currently reports that
  no automation/learn target exists;
- no MPE or channel-per-note handling;
- MIDI clock timing pulses are visible as status, but the transport does not yet
  derive BPM or phase-lock row scheduling from incoming clock ticks.
