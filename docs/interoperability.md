# Interoperability

Salieri native `.salieri` JSON remains the canonical project format. Import/export is handled by the dedicated `salieri-interop` crate so external format details do not leak into the core model.

Current MIDI file support is intentionally narrow:

- export the selected pattern as Standard MIDI File format 0;
- import Standard MIDI File format 0 with PPQN timing, tempo meta events, note on, and note off;
- map imported MIDI channels onto existing tracks by channel, creating tracks only when needed;
- reject unsupported MIDI formats, SMPTE timing, SysEx, and unsupported event types with explicit errors.

Round-trip expectations:

- note pitch, velocity, row placement, channel, and BPM are preserved for the supported subset;
- Salieri-specific concepts such as pattern names, sequence positions, tracker commands, mute/solo state, sampler metadata, mixer state, and native DSP chains are not represented in MIDI files;
- `.salieri` should be used for lossless project storage and Git diffs.

## Tracker And Renoise Research

Tracker module formats such as MOD, XM, IT, and S3M are not just pattern containers. They combine patterns, embedded samples, instrument behavior, effect memory, tempo/tick timing, channel state, and historical player quirks. A faithful import path needs either a module player compatibility layer or a deliberately lossy semantic importer.

Renoise XRNS is a better first target for Salieri than MOD/XM/IT/S3M because XRNS is a ZIP container with XML song data and embedded sample data. Renoise also exposes note columns, effect columns, pattern lines, samples, instruments, sample mappings, and device chains through its public song model. This does not make XRNS trivial, but it means an importer can start from structured project data instead of reverse-engineering player behavior.

References used for this decision:

- Renoise forum notes from Renoise developers describe XRNS/XRNI/XRNT as standard ZIP archives containing XML plus sample data: <https://forum.renoise.com/t/accessing-the-xml-and-sample-files-in-xrns-xrni-and-xrnt-files/18683> and <https://forum.renoise.com/t/is-there-a-way-i-could-get-a-copy-of-the-file-formats-xrns-xrni/48894/2>.
- Renoise Lua Song API documents the song concepts Salieri must map: instruments, samples, sample mappings, patterns, note columns, effect columns, automation, and device chains: <https://files.renoise.com/xrnx/documentation/Renoise.Song.API.lua.html>.
- Tracker module format references describe MOD/S3M/XM/IT effect timing, tick behavior, volume columns, and effect parameter memory, which are not equivalent to Salieri's current row-event model: <https://pollak.thebe.de/b/module-formats---introduction/>.

## Compatibility Matrix

| Source data | Lossless now | Approximate | Unsupported for first pass |
| --- | --- | --- | --- |
| Pattern row count, track count, note pitch, note-off/cut intent | XRNS subset when directly represented | SMF row quantization | MOD/XM/IT/S3M quirks until a module parser/player exists |
| Velocity/volume/pan/delay/effect columns | XRNS note/effect columns that map to Salieri columns | Unknown Renoise effect commands as preserved tracker commands with warnings | DSP/device parameter commands without a Salieri equivalent |
| WAV/AIFF/FLAC sample references embedded in XRNS | WAV samples after extraction/normalization | Non-WAV sample formats after decode support exists | Plugin instruments and generator devices |
| Instruments and sample mappings | Single-sample instruments and simple key mapping | Multi-sample instruments as multiple Salieri instruments | Keyzones, velocity layers, slicing, modulation sets |
| Mixer gain/pan and native DSP gain/pan | Directly mappable when present | Renoise device chains reduced to supported gain/pan devices | Third-party plugins, complex DSP devices, meta/modulation devices |
| Automation | None lossless yet | Sample gain or mixer/DSP automation after Salieri automation targets expand | Arbitrary device automation |
| Arrangement/sequence | Pattern order can map to Salieri sequence | Pattern aliases/clips flattened with warnings | Renoise features without Salieri sequence equivalents |

## Decision

First implementation target: **XRNS read/import subset**, not legacy tracker modules.

The first target should be split into two stages:

1. **XRNS inspector and diagnostics**: read the ZIP, locate `Song.xml` and sample payloads, parse enough metadata to report tracks, patterns, instruments, samples, device-chain kinds, and unsupported features without mutating a Salieri project. Follow-up: #62.
2. **XRNS minimal importer**: convert a constrained subset into `.salieri`: pattern length/order, note pitch, velocity, instrument number, volume, pan, delay, first effect command, simple sample-backed instruments, track names, mixer gain/pan, and native gain/pan DSP devices. Follow-up: #61.

The importer must be explicit about loss. It should return an import report with warnings, not silently discard project data.

## Minimal XRNS Import Subset

Accept initially:

- ZIP container with `Song.xml` at archive root;
- pattern lines with note columns that can map to Salieri note, velocity, instrument, volume, pan, and delay fields;
- at most one effect command mapped to Salieri's first `TrackerCommand` per cell;
- pattern sequence/order that can map to Salieri sequence entries;
- sample-backed instruments whose sample payload can be loaded by `salieri-sampler` after extraction/preparation;
- mixer track gain/pan and native gain/pan DSP devices when they can be recognized safely.

Warn and preserve where possible:

- additional effect columns beyond the first;
- unknown effect commands;
- pattern timing that does not divide cleanly into Salieri row timing;
- unsupported sample formats that may become available after decoder support;
- device chains with unsupported native devices.

Reject with explicit errors:

- archives missing `Song.xml`;
- malformed XML;
- encrypted or nested archives;
- projects requiring plugin instruments for note playback;
- unsupported structural versions when the importer cannot identify safe fields;
- imports that would create invalid Salieri projects after validation.

## Legacy Module Formats

MOD, XM, S3M, and IT should remain explicit unsupported formats until Salieri has either:

- a module parser/player compatibility layer that can evaluate tick timing, effect memory, pattern jumps, arpeggios, slides, retrigger, sample offset, tempo changes, global volume, and volume-column effects; or
- a declared lossy importer mode that only extracts samples and coarse note data.

The recommended first legacy-module follow-up is not full import. It is a **module diagnostics and sample extraction spike** that reports module metadata and identifies which effects would be lost. Follow-up: #63.

## Error And Warning Shape

Interop should expose structured diagnostics rather than strings only:

- `UnsupportedContainer`: file is not a supported archive/module/container;
- `MissingSongXml`: XRNS archive has no root `Song.xml`;
- `MalformedSongXml`: XML cannot be parsed;
- `UnsupportedRenoiseFeature`: feature name, location, and severity;
- `UnsupportedSampleFormat`: sample path and codec/extension;
- `UnsupportedEffectCommand`: pattern, track, row, command, and value;
- `DroppedExtraEffectColumn`: pattern, track, row, and column index;
- `TimingQuantized`: source position and resulting Salieri row;
- `ValidationFailed`: produced project did not pass Salieri validation.

Diagnostics should be collected into an import report and surfaced by CLI/app code before save.
