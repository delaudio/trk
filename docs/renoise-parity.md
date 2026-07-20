# Renoise Demo Parity Harness

Renoise demo songs are useful for local playback-parity work, but their source
assets are third-party. Keep imported demo projects and extracted samples under
`fixtures/local/renoise-demos/`; that path is ignored by Git.

Run a CI-safe smoke import with the committed synthetic fixture:

```sh
python3 scripts/renoise-demo-parity.py --synthetic
```

Run the local demo harness against a Renoise demo directory:

```sh
python3 scripts/renoise-demo-parity.py ~/Music/Renoise/DemoSongs
```

The script reuses the existing CLI:

```text
cargo run -q -p salieri-app -- import xrns INPUT OUTPUT --sample-dir DIR --sample-path-prefix PREFIX
```

It writes `.salieri` outputs and extracted samples into
`fixtures/local/renoise-demos/`, then writes
`fixtures/local/renoise-demos/parity-report.json`.

The JSON report records, per song and in totals:

- tracks, patterns, sequence entries, samples, and extracted samples;
- unsupported Renoise devices;
- unsupported phrases;
- unsupported effect commands;
- dropped extra effect columns;
- import errors.

Use the report as a local trend line while implementing XRNS/Renoise parity
issues. CI should use `--synthetic` only; it must not require third-party demo
assets.

XRNS sample import preserves representable sample playback metadata: root note,
transpose, fine tune, volume/gain, pan, loop mode/windows, and ADSR-like envelope
settings. Unsupported sample metadata is reported as an import warning instead
of being dropped silently.
