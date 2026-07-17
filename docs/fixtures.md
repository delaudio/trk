# Fixtures

Fixtures under `fixtures/` are intentional project files and may be committed.

Allowed fixture types:

- Curated `.salieri` demo projects that exercise stable format behavior.
- Corrupt or old-version files used by migration and validation tests.
- Minimal projects for docs and examples.

Do not commit:

- Local scratch projects from manual sessions.
- Local generated songs that are not intended to be the shipped demo.
- MIDI logs.
- Temporary `.tmp` files from save operations.

The root `.gitignore` ignores `/untitled.salieri` and `/salieri-midi.log` only. It does not ignore `fixtures/*.salieri`, so test fixtures remain trackable.

## Current Fixtures

- `fixtures/default.salieri`: `256COLOR_rep`, converted from XRNS into a valid format-version-1 demo project with 15 tracks, 40 patterns, sequence data, instruments, and WAV sample assets under `fixtures/samples/256color/`.
