# Fixtures

Fixtures under `fixtures/` are intentional project files and may be committed.

Allowed fixture types:

- Curated `.salieri` demo projects that exercise stable format behavior.
- Corrupt or old-version files used by migration and validation tests.
- Minimal projects for docs and examples.
- Synthetic external-format inputs and deterministic AI outputs used by regression tests.

Do not commit:

- Local scratch projects from manual sessions.
- Local generated songs that are not intended to be the shipped demo.
- Third-party demo songs and extracted samples unless their license explicitly allows redistribution.
- MIDI logs.
- Temporary `.tmp` files from save operations.

The root `.gitignore` ignores `/untitled.salieri`, `/salieri-midi.log`, and `/fixtures/local/`. It does not ignore `fixtures/*.salieri`, so test fixtures remain trackable.

Local Renoise demo imports can live under `fixtures/local/renoise-demos/` with samples under `fixtures/local/renoise-demos/samples/`. These files are useful for manual parity testing, but they are not redistributed by this repository.
Point `[project_browser].start_dir` (or `workspace.project_library`) at `fixtures/local/renoise-demos/` to review those imported demos in the TUI project browser. The browser also has deterministic snapshots for the Renoise-style demo sections, so tests do not require local third-party assets.

## Current Fixtures

- `fixtures/default.salieri`: `256COLOR_rep`, converted from XRNS into a valid format-version-1 demo project with 15 tracks, 40 patterns, sequence data, instruments, and WAV sample assets under `fixtures/samples/256color/`.
- `fixtures/projects/foundations.salieri`: small format-version-1 golden project covering notes, sample playback settings, instruments, mixer state, DSP devices, and automation.
- `fixtures/midi/simple-format0.hex`: minimal Standard MIDI File format-0 input stored as reviewable hexadecimal text.
- `fixtures/xrns/minimal-song.xml`: synthetic Renoise `Song.xml` input packaged into an in-memory XRNS archive by interop tests.
- `fixtures/ai/local-proposal.txt`: deterministic local AI proposal output.

## Updating Goldens

Snapshot and fixture changes must be intentional and reviewed like code. To regenerate TUI snapshots:

```bash
UPDATE_SALIERI_SNAPSHOTS=1 cargo test -p salieri-tui --test render_snapshots
```

Renoise-style UI snapshots include `renoise-pattern-workspace`, `sampler-large`, and `renoise-demo-browser`; review those files when changing the tracker/sampler/browser visual migration.

To regenerate project and AI golden fixtures:

```bash
UPDATE_SALIERI_FIXTURES=1 cargo test -p salieri-app persistence::tests::foundations_fixture_preserves_project_contracts -- --exact
UPDATE_SALIERI_FIXTURES=1 cargo test -p salieri-ai tests::local_proposal_matches_golden_fixture -- --exact
```

Run the same tests again without the update variables before committing. Project mismatches report JSON paths and TUI mismatches report the first changed line with surrounding context. Never use update mode merely to make an unexplained failure pass.
