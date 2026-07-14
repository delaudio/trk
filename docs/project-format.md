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

Within `song`, sample playback intent is represented by optional `samples` and
`sampleAssignments` arrays. Older version-1 files that omit those arrays still
load as projects with no sampler assignments.

## Loading Rules

On load, Salieri:

1. parses the JSON into the versioned project envelope;
2. routes the project through the migration entry point;
3. rejects unsupported format versions with a clear error;
4. validates the resulting song before returning it to the app.

Version `1` currently needs no migration. The migration entry point exists so future versions can be upgraded in one place before validation.

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
- patterns with zero rows;
- pattern rows whose cell count does not match the track count;
- note pitches, velocities, or gates outside MIDI `0..=127`;
- sequence entries referencing missing pattern IDs.
- sample assignments referencing missing track or sample IDs.

Save operations run the same validation before writing. This prevents Salieri from creating a `.salieri` file it would reject on the next load.

## Fixture Policy

Committed fixtures under `fixtures/` should remain valid for the current format unless their filename explicitly describes an invalid, migration, or error case used by tests.
