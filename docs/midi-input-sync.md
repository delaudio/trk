# MIDI Input And Sync Foundation

Advanced MIDI is post-MVP. The MVP output path remains in `salieri-midi::output`; input is separated in `salieri-midi::input`.

The input foundation provides:

- parsed channel voice events for note on/off, CC, and program change;
- MIDI realtime clock events for clock/start/continue/stop;
- `MidiInputPacket` timestamps for deterministic recording tests;
- `FakeMidiInput` for hardware-free tests.

## Boundaries

MIDI input should not mutate `salieri-core` directly. Future realtime recording should translate input packets into semantic app commands, then apply normal undoable edits. MIDI sync should report visible app updates on clock loss, unsupported messages, or input device disconnects.

Input and output are intentionally separate traits so a failing input device cannot corrupt output playback state, and vice versa.

## Current Limitations

- no real `midir` input connection yet;
- no realtime recording quantization;
- no MIDI learn mapping table;
- no MIDI clock follower in the transport runtime;
- no TUI MIDI input settings panel.
