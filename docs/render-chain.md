# Render Chain Plan

Render-chain JSON is metadata for future audio-engine or external render-worker
workflows. It is not a plugin host and does not require VST, AU, CLAP, CPAL, or
source-separation tooling.

Generate a plan from a project:

```bash
salieri render-chain song.salieri render-chain.json --sample-rate 48000 --channels 2 --bit-depth 24
```

Schema version `1` includes:

- `source`: optional project path and project title;
- `format`: sample rate, channel count, and target bit depth;
- `tracks`: one entry per Salieri track with source type, MIDI channel,
  optional instrument/stem reference, effect metadata, mix defaults, and output
  stem path;
- `master`: master effect metadata and mix defaults;
- `targets`: planned outputs such as the stereo mix.

Track `sourceType` is `tracker-midi` unless the Salieri track references an
external stem entry, in which case it is `external-stem`. The plan can therefore
represent current MIDI/tracker projects and future stem-backed workflows without
making stems mandatory at project-load time.
