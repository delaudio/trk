# Project Format Compatibility

Salieri project files use JSON and the `.salieri` extension.

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
On load, Salieri normalizes those assignments into sample-backed instruments and
track instrument assignments. New sample assignment edits keep both the
compatibility assignment and the instrument assignment in sync.

Each sample reference may include optional playback settings. Omitted playback
settings default to one-shot playback with no frame window, no loop points, and a
neutral envelope:

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

## Loading Rules

On load, Salieri:

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
- duplicate instrument IDs;
- empty instrument names;
- instruments referencing missing samples;
- track instrument assignments referencing missing tracks or instruments;
- patterns with zero rows;
- pattern rows whose cell count does not match the track count;
- note pitches, velocities, or gates outside MIDI `0..=127`;
- sequence entries referencing missing pattern IDs;
- sample assignments referencing missing track or sample IDs.

Save operations run the same validation before writing. This prevents Salieri from creating a `.salieri` file it would reject on the next load.

## Fixture Policy

Committed fixtures under `fixtures/` should remain valid for the current format unless their filename explicitly describes an invalid, migration, or error case used by tests.
