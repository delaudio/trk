# Ableton Session Bridge

The Ableton bridge is an optional adapter boundary for exchanging session-view material with Ableton-oriented tools. It does not require Ableton Live for normal Salieri builds or tests, and this first implementation does not connect to Live directly.

Instead, Salieri reads and writes a versioned local JSON document that a future Live sidecar can consume or produce.

## Commands

Export a Salieri project session to a bridge document:

```bash
salieri ableton push song.salieri ableton-session.json
```

Preview without writing:

```bash
salieri ableton push song.salieri ableton-session.json --dry-run
```

Import a bridge document into a `.salieri` project:

```bash
salieri ableton pull ableton-session.json pulled.salieri
```

Preview the import summary without writing:

```bash
salieri ableton pull ableton-session.json pulled.salieri --dry-run
```

## Schema

Schema version `1`:

```json
{
  "schemaVersion": 1,
  "tempoBpm": 128,
  "linesPerBeat": 4,
  "tracks": [
    {
      "index": 0,
      "name": "Bass",
      "midiChannel": 1
    }
  ],
  "scenes": [
    {
      "index": 0,
      "name": "Intro"
    }
  ],
  "clips": [
    {
      "name": "Bass Clip",
      "trackIndex": 0,
      "sceneIndex": 0,
      "lengthBeats": 4.0,
      "sourcePattern": "Pattern 01",
      "notes": [
        {
          "pitch": 48,
          "velocity": 100,
          "startBeat": 0.0,
          "durationBeats": 0.25
        }
      ]
    }
  ]
}
```

## Mapping

Push:

- Salieri tracks become bridge tracks with names and MIDI channels.
- Salieri scenes become bridge scenes.
- Scene slots with clips become bridge clips at `trackIndex` and `sceneIndex`.
- Notes preserve pitch, velocity, start beat, duration beat, clip length, tempo, and track names for the supported row-quantized subset.

Pull:

- Bridge tracks become Salieri tracks.
- Bridge scenes become Salieri scenes.
- Each bridge clip becomes one Salieri pattern and one Salieri clip.
- Scene-indexed bridge clips are assigned to scene slots by track.

## Limitations

- No Live API connection is opened by Salieri.
- No Ableton project files are parsed directly.
- Automation, devices, audio clips, warping, groove, follow actions, and plugin state are out of scope for this first bridge.
- Timing is quantized to Salieri rows using `linesPerBeat`.
