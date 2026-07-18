# Fixtures

Fixtures under `fixtures/` are intentional project files and may be committed.

Allowed fixture types:

- Curated `.salieri` demo projects that exercise stable format behavior.
- Corrupt or old-version files used by migration and validation tests.
- Minimal projects for docs and examples.

Do not commit:

- Local scratch projects from manual sessions.
- Local generated songs that are not intended to be the shipped demo.
- Third-party demo songs and extracted samples unless their license explicitly allows redistribution.
- MIDI logs.
- Temporary `.tmp` files from save operations.

The root `.gitignore` ignores `/untitled.salieri`, `/salieri-midi.log`, and `/fixtures/local/`. It does not ignore `fixtures/*.salieri`, so test fixtures remain trackable.

Local Renoise demo imports can live under `fixtures/local/renoise-demos/` with samples under `fixtures/local/renoise-demos/samples/`. These files are useful for manual parity testing, but they are not redistributed by this repository.

## Current Fixtures

- `fixtures/default.salieri`: `256COLOR_rep`, converted from XRNS into a valid format-version-1 demo project with 15 tracks, 40 patterns, sequence data, instruments, and WAV sample assets under `fixtures/samples/256color/`.
