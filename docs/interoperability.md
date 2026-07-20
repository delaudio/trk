# Interoperability

Salieri native `.salieri` JSON remains the canonical project format. Import/export is handled by the dedicated `salieri-interop` crate so external format details do not leak into the core model.

Current MIDI file support is intentionally narrow:

- export the selected pattern as Standard MIDI File format 0;
- import Standard MIDI File format 0 with PPQN timing, tempo meta events, note on, and note off;
- map imported MIDI channels onto existing tracks by channel, creating tracks only when needed;
- reject unsupported MIDI formats, SMPTE timing, SysEx, and unsupported event types with explicit errors.

Current XRNS support is library-level and intentionally lossy:

- inspect XRNS ZIP archives, locate root stored-or-deflated `Song.xml`, enumerate sample payloads, and report track, pattern, instrument, sample, and device metadata;
- import a constrained XML subset into a validated `.salieri` `Song`;
- map track names, pattern row counts, sequence order, note/velocity/instrument/volume/pan/delay cells, the first two effect commands, instrument IDs, supported WAV sample payloads, mixer gain/pan, and recognized native gain/pan devices;
- report unsupported samples, devices, extra effect columns, unknown effect commands, quantized timing, malformed archives/XML, nested/encrypted archives, and validation failures as structured diagnostics.

The CLI can write the supported subset directly to a Salieri project:

```bash
salieri import xrns input.xrns output.salieri
salieri import xrns input.xrns output.salieri --sample-dir fixtures/local/samples/demo --sample-path-prefix samples/demo
```

`--sample-dir` extracts supported WAV payloads from the XRNS archive and rewrites imported sample references to the stored path prefix. This is intended for local demo libraries and manual parity checks; third-party Renoise demo songs and samples should stay under ignored local folders unless their license explicitly allows redistribution.

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
| Velocity/volume/pan/delay/effect columns | XRNS note/effect columns plus supported FX1/FX2 timing commands | Deferred Renoise effect commands as preserved tracker commands with warnings | Effect columns beyond FX2 and DSP/device parameter commands without a Salieri equivalent |
| WAV/AIFF/FLAC sample references embedded in XRNS | WAV samples after extraction/normalization | Non-WAV sample formats after decode support exists | Plugin instruments and generator devices |
| Instruments and sample mappings | Single-sample instruments and simple key mapping | Multi-sample instruments as multiple Salieri instruments | Keyzones, velocity layers, slicing, modulation sets |
| Mixer gain/pan and native DSP gain/pan | Directly mappable when present | Renoise device chains reduced to supported gain/pan devices | Third-party plugins, complex DSP devices, meta/modulation devices |
| Automation | None lossless yet | Sample gain or mixer/DSP automation after Salieri automation targets expand | Arbitrary device automation |
| Arrangement/sequence | Pattern order can map to Salieri sequence | Pattern aliases/clips flattened with warnings | Renoise features without Salieri sequence equivalents |

## Decision

First implementation target: **XRNS read/import subset**, not legacy tracker modules.

The first target is split into two stages:

1. **XRNS inspector and diagnostics**: read the ZIP, locate `Song.xml` and sample payloads, parse enough metadata to report tracks, patterns, instruments, samples, device-chain kinds, and unsupported features without mutating a Salieri project. Implemented by #62.
2. **XRNS minimal importer**: convert a constrained subset into `.salieri`: pattern length/order, note pitch, velocity, instrument number, volume, pan, delay, first effect command, simple sample-backed instruments, track names, mixer gain/pan, and native gain/pan DSP devices. Implemented by #61.

The importer must be explicit about loss. It should return an import report with warnings, not silently discard project data.

## Minimal XRNS Import Subset

Accept initially:

- ZIP container with stored or deflated `Song.xml` at archive root;
- pattern lines with note columns that can map to Salieri note, velocity, instrument, volume, pan, and delay fields;
- at most two effect commands mapped to Salieri's FX1/FX2 tracker commands per cell;
- pattern sequence/order that can map to Salieri sequence entries;
- sample-backed instruments whose sample payload can be loaded by `salieri-sampler` after extraction/preparation;
- mixer track gain/pan and native gain/pan DSP devices when they can be recognized safely.

Warn and preserve where possible:

- additional effect columns beyond FX2;
- unknown or deferred effect commands that can be preserved without playback semantics;
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

The current `salieri-interop` probe can inspect:

- MOD title, channel count for common signatures, pattern count, 31 sample headers, raw effect command nibbles from pattern data, and contiguous sample payloads when the module is not truncated;
- XM title, channel count, pattern count, and instrument count from the module header;
- S3M title, active channel count, pattern count, and instrument count from the module header;
- IT title, enabled channel count, pattern count, and instrument count from the module header.

The probe deliberately does not decode player-compatible note data or effect semantics. It always reports timing/effect-memory diagnostics because those semantics are not represented by Salieri's current row-event model.

Recommendation: keep the first legacy-module feature as **sample extraction only**, with metadata/effect diagnostics shown before extraction. The initial extraction implementation is safe for MOD sample payloads; XM/S3M/IT sample extraction still needs their instrument/sample offset tables decoded before payload bytes can be returned. Coarse note import should wait for a second spike that either embeds a player-compatibility layer or defines an explicitly lossy effect/timing translation table. This must not claim MOD/XM/S3M/IT song import.

## Error And Warning Shape

Interop should expose structured diagnostics rather than strings only:

- `UnsupportedContainer`: file is not a supported archive/module/container;
- `MalformedArchive`: XRNS data is not a readable ZIP archive;
- `MissingSongXml`: XRNS archive has no root `Song.xml`;
- `MalformedSongXml`: XML cannot be parsed;
- `EncryptedArchive`: encrypted ZIP entries cannot be inspected/imported;
- `NestedArchive`: nested ZIP/XRNS payloads are detected but not imported recursively;
- `UnsupportedCompression`: a required XRNS entry uses unsupported ZIP compression;
- `UnsupportedRenoiseFeature`: feature name, location, and severity;
- `UnsupportedSampleFormat`: sample path and codec/extension;
- `UnsupportedEffectCommand`: pattern, track, row, command, and value;
- `DroppedExtraEffectColumn`: pattern, track, row, and column index;
- `TimingQuantized`: source position and resulting Salieri row;
- `ValidationFailed`: produced project did not pass Salieri validation.
- `MalformedModule`: legacy module header/data is too short or has the wrong signature;
- `UnsupportedTimingSemantics`: module tick/control-flow timing cannot be represented losslessly;
- `UnsupportedEffectMemory`: module effect memory/channel state cannot be represented losslessly;
- `EffectDecodeIncomplete`: effect command numbers were observed but player-compatible semantics were not decoded.

Diagnostics should be collected into an import report and surfaced by CLI/app code before save.
