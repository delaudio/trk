# Interoperability

Salieri native `.salieri` JSON remains the canonical project format. Import/export is handled by the dedicated `salieri-interop` crate so external format details do not leak into the core model.

Current MIDI file support is intentionally narrow:

- export the selected pattern as Standard MIDI File format 0;
- import Standard MIDI File format 0 with PPQN timing, tempo meta events, note on, and note off;
- map imported MIDI channels onto existing tracks by channel, creating tracks only when needed;
- reject unsupported MIDI formats, SMPTE timing, SysEx, and unsupported event types with explicit errors.

Round-trip expectations:

- note pitch, velocity, row placement, channel, and BPM are preserved for the supported subset;
- Salieri-specific concepts such as pattern names, sequence positions, tracker commands, mute/solo state, and future sampler/DSP metadata are not represented in MIDI files;
- `.salieri` should be used for lossless project storage and Git diffs.

Tracker module formats such as MOD, XM, IT, S3M, and Renoise projects are represented as explicit unsupported formats for now. They should be investigated independently because their instrument, sample, effect, and timing semantics do not map cleanly to the MIDI-first MVP.
