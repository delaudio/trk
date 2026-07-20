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

## Parity Matrix

The local parity suite is intentionally split between committed smoke fixtures
and ignored third-party Renoise demo imports. Keep the matrix below in Git; keep
the generated `.salieri`, sample payloads, and JSON trend reports under
`fixtures/local/renoise-demos/`.

| Area | Current Salieri behavior | Expected audible gap | Diagnostic class | Follow-up |
| --- | --- | --- | --- | --- |
| Notes, velocity, instrument id, row placement, BPM/LPB, sequence order | Imported into the core song model. | Low for simple note/sample patterns. | Validation failures are blocking import errors. | Covered by importer tests. |
| Note-column volume, pan, delay | Imported into cell volume/pan/delay fields. | Low when the source uses only note-column timing. | Timing quantization warnings when source timing is finer than rows. | Covered by importer/playback tests. |
| Renoise effect columns | FX1/FX2 are preserved; supported timing effects `0Q`/`0R` translate to Salieri delay/retrigger playback. Deferred high-priority commands such as pitch slides and sample offset remain visible tracker commands. | Medium: supported timing improves, but deferred commands still do not affect playback. | `UnsupportedEffectCommand` means preserved-without-playback when stored in FX1/FX2; `DroppedExtraEffectColumn` means actual dropped playback data beyond FX2. | #145 completed the current timing slice; broader command behavior remains under #85 follow-ups. |
| Sample playback metadata and keyzones | Root note, tuning, gain, pan, loop windows, envelopes, and multisample key/velocity zones are imported where representable. | Medium for sliced or phrase-driven instruments. | Unsupported sample metadata is warned; unsupported sample formats remain explicit. | #76, #77, #143. |
| Renoise phrases | Not translated into instrument sub-pattern playback yet. | High for phrase-backed instruments because triggering a note can play different material in Renoise. | Phrase diagnostics must state translated, approximated, or unsupported/blocking; silent ignore is not acceptable. | #143. |
| Device chains and native DSP | Basic gain/pan-style foundations import; broader filter, delay, modulation, drive, dynamics, LFO/meta devices, and automation are not parity-complete. | High for demo songs using LFOs, filters, delays, modulation, sidechain, or meta devices. | Unsupported device diagnostics are preserved; they must not imply successful playback parity. | #147, #84. |
| Automation and parameter locks | Salieri has automation/parameter-lock primitives, but XRNS automation envelopes are not imported broadly yet. | High for evolving filter/delay/modulation demos. | Unsupported automation is dropped playback behavior unless converted into Salieri automation or locks. | #147. |
| Send/master routing | Send metadata exists but audio routing is not equivalent to Renoise send/master graphs. | High for cross-track routing and sidechain demos. | Unsupported routing should be explicit in import reports. | #84, #147. |

## Danoise LFO Baseline

`demosong-danoise-lfo.salieri` is the first manual parity focus because it uses
the exact areas that distinguish a simple note/sample import from Renoise
playback: LFO/meta-device modulation, native device chains, automation, sampler
metadata, and routing.

Before #147 and #143 land, expected differences for this song are:

- LFO/meta devices and automated filter/delay/modulation behavior are imported
  only as visible unsupported device/feature diagnostics, not as equivalent DSP
  modulation.
- Any phrase-backed instrument behavior remains unsupported unless represented
  by ordinary imported notes and sample zones.
- FX1/FX2 timing commands now preserve and play supported delay/retrigger
  semantics, but pitch slides, sample offsets, pattern control, and other
  deferred command families remain visible without playback semantics.
- Send/master routing and sidechain-like relationships are not expected to
  match Renoise until routing foundations are implemented.

Use this baseline when reviewing local reports: a lower count of unsupported or
dropped effect columns is meaningful after #145, but device-chain, automation,
phrase, and routing gaps should remain expected until their owning issues close.

## Linked Renoise Work

- #143 owns phrase parsing, diagnostics, and deterministic phrase playback or
  explicit blocking diagnostics.
- #147 owns Renoise device-chain, LFO/meta-device, and automation import into
  native DSP/automation structures.
- #84 owns send/master routing foundations needed before Renoise send graphs can
  claim parity.
- #76 and #77 own sampler playback-mode and instrument-slot parity that affects
  sliced or phrase-backed instruments.
