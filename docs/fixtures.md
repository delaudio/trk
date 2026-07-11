# Fixtures

Fixtures under `fixtures/` are intentional project files and may be committed.

Allowed fixture types:

- Small `.salieri` files that exercise stable format behavior.
- Corrupt or old-version files used by migration and validation tests.
- Minimal projects for docs and examples.

Do not commit:

- Local scratch projects from manual sessions.
- Large generated songs.
- MIDI logs.
- Temporary `.tmp` files from save operations.

The root `.gitignore` ignores `/untitled.salieri` and `/salieri-midi.log` only. It does not ignore `fixtures/*.salieri`, so test fixtures remain trackable.

## Current Fixtures

- `fixtures/default.salieri`: small valid format-version-1 project with four tracks, one pattern, and a few MIDI notes.
