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
- basic MIDI clock start/continue/stop transport following.

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
```

Enable external MIDI transport following:

```text
:midi-input clock on
:midi-input clock off
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
cursor cell with the incoming velocity, then the cursor advances by the current
edit step. This is the first quantization boundary: the active row is the
quantized destination.

## Current Limitations

- no MIDI learn mapping table;
- no CC automation recording;
- no MPE or channel-per-note handling;
- no graphical TUI MIDI input settings panel;
- MIDI clock timing pulses are visible as status, but the transport does not yet
  derive BPM or phase-lock row scheduling from incoming clock ticks.
