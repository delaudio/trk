# Project Format Compatibility

trk project files use JSON and the `.trk` extension.

Every project file must contain:

```json
{
  "formatVersion": 1,
  "song": {}
}
```

`formatVersion` is mandatory. The current supported version is `1`.

Within `song`, sample playback intent is represented by optional `samples`,
`sampleAssignments`, `instruments`, and `trackInstrumentAssignments` arrays.
Older version-1 files that omit those arrays still load as projects with no
sampler assignments.

`sampleAssignments` remains a compatibility field for the early sampler model.
On load, trk normalizes those assignments into sample-backed instruments and
track instrument assignments. New sample assignment edits keep both the
compatibility assignment and the instrument assignment in sync.

Each sample reference may include optional playback settings. Omitted playback
settings default to one-shot playback with no frame window, no loop points, and a
neutral envelope. Supported `mode` values are `oneShot`, legacy `loop`
(treated as forward loop), `forwardLoop`, `backwardLoop`, `pingPongLoop`, and
`reverse`:

```json
{
  "id": 1,
  "name": "break.wav",
  "path": "samples/break.wav",
  "rootPitch": 60,
  "gain": 1.0,
  "playback": {
    "mode": "loop",
    "startFrame": 1200,
    "endFrame": 48000,
    "loopStartFrame": 2400,
    "loopEndFrame": 12000,
    "envelope": {
      "attackSeconds": 0.005,
      "decaySeconds": 0.04,
      "sustain": 0.8,
      "releaseSeconds": 0.08
    }
  }
}
```

Sample paths are stored as portable project data. Relative paths resolve from the
directory containing the `.trk` project first. User configuration can also
define `workspace.sample_library` as the default library used by sample browsing
and future sample-import workflows, but project files do not embed that
machine-specific library root.

Each pattern may also include optional stepped automation lanes. The first
implemented target is `sampleGain`, which automates the effective gain used by
sampler events from the lane point row onward:

```json
{
  "id": 1,
  "name": "Pattern 01",
  "rows": [],
  "automation": [
    {
      "target": {
        "type": "sampleGain",
        "sample": 1
      },
      "interpolation": "step",
      "points": [
        { "row": 0, "value": 1.0 },
        { "row": 4, "value": 0.5 }
      ]
    }
  ]
}
```

Pattern cells are backward-compatible. Older cells may contain only `note`,
`velocity`, `gate`, and `command`; richer tracker cells can also include
instrument, volume, pan, delay metadata, and generic parameter locks:

```json
{
  "note": { "type": "note", "pitch": 60 },
  "velocity": 100,
  "instrument": 1,
  "volume": 64,
  "pan": 127,
  "delay": 32,
  "command": { "code": 82, "value": 4 },
  "parameterLocks": [
    {
      "target": { "type": "sample", "sample": 1 },
      "parameter": "sample.gain",
      "action": {
        "type": "set",
        "value": { "type": "float", "value": 0.5 }
      }
    },
    {
      "target": { "type": "trackEffect", "track": 1, "device": 1 },
      "parameter": "native.gain.gain",
      "action": { "type": "reset" }
    }
  ]
}
```

`instrument` references a sample-backed instrument. `volume` and `pan` use
MIDI-style `0..127` values for sampler gain and stereo position, while `delay`
uses `0..255` row fractions. `command` remains the first tracker effect column
and preserves existing delay/retrigger command compatibility. `parameterLocks`
are row-scoped, copy with cells, target stable sampler/mixer/send/native-device
IDs, and store typed `ParameterValue` payloads. Unknown lock targets or
parameter IDs remain loadable and can be reported as diagnostics; known values
are validated through the descriptor catalog.

Projects also include mixer state. Omitted mixer state is normalized on load
with one default track mixer per song track:

```json
{
  "masterGain": 0.9,
  "masterEffects": [
    {
      "id": 1,
      "name": "Gain",
      "bypassed": false,
      "type": "gain",
      "gain": 0.8
    }
  ],
  "tracks": [
    {
      "track": 1,
      "gain": 0.75,
      "pan": -0.25,
      "muted": false,
      "solo": false,
      "sends": [],
      "effects": [
        {
          "id": 1,
          "name": "Gain",
          "bypassed": false,
          "type": "gain",
          "gain": 0.5
        },
        {
          "id": 2,
          "name": "Pan",
          "bypassed": false,
          "type": "pan",
          "pan": -0.25
        }
      ]
    }
  ],
  "sends": []
}
```

Effect chains are serializable native device references. The initial device set
is `gain` and `pan`; `bypassed` preserves a device in the chain without
processing audio. The maintained native device catalog and planned stable device
IDs live in [Native DSP Roadmap](native-dsp-roadmap.md).

## Loading Rules

On load, trk:

1. parses the JSON into the versioned project envelope;
2. routes the project through the migration entry point;
3. rejects unsupported format versions with a clear error;
4. normalizes sample assignments into sample-backed instruments;
5. validates the resulting song before returning it to the app.

Version `1` keeps the same format version while adding optional instrument
fields. The migration entry point exists so future versions can be upgraded in
one place before validation.

## Validation Rules

Loaded projects are rejected when they contain:

- an empty song title;
- BPM or LPB set to `0`;
- non-finite swing;
- no tracks, no patterns, or an empty sequence;
- duplicate track or pattern IDs;
- duplicate sample IDs;
- empty track or pattern names;
- empty sample names or paths;
- MIDI channels outside `1..=16`;
- invalid sample root pitch or gain;
- invalid sample frame windows, loop windows, or envelope values;
- automation lanes with duplicate targets, missing sample targets, duplicate rows, out-of-bounds rows, or invalid values;
- pattern cells referencing missing instruments or invalid velocity, gate, volume, or pan values;
- known parameter locks with values outside their descriptor type/range;
- invalid mixer master gain, missing/duplicate mixer tracks, mixer tracks referencing missing song tracks, invalid mixer gain/pan, duplicate effect device ids, or invalid effect parameters;
- duplicate instrument IDs;
- empty instrument names;
- instruments referencing missing samples;
- track instrument assignments referencing missing tracks or instruments;
- patterns with zero rows;
- pattern rows whose cell count does not match the track count;
- note pitches, velocities, or gates outside MIDI `0..=127`;
- sequence entries referencing missing pattern IDs;
- sample assignments referencing missing track or sample IDs.

Save operations run the same validation before writing. This prevents trk from creating a `.trk` file it would reject on the next load.

## Fixture Policy

Committed fixtures under `fixtures/` should remain valid for the current format unless their filename explicitly describes an invalid, migration, or error case used by tests.
