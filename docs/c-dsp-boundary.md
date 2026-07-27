# C/C++ DSP Integration Boundary

trk may wrap selected C or C++ DSP algorithms as native modules when the
code is vendored, reviewed, and adapted to trk's plain-data module contract.
This is separate from loading arbitrary third-party binaries at runtime.

## Boundary rules

- C/C++ DSP code is compiled only through explicit Cargo features.
- `unsafe` FFI calls stay inside a dedicated Rust wrapper module.
- Rust wrappers validate sample rate, channel count, max block size, parameter
  ranges, and finite audio before exposing processing to realtime/offline code.
- Wrapped modules expose stable IDs, parameter IDs, parameter ranges, defaults,
  and plain Rust state. Project schemas must not store opaque native pointers.
- Realtime processing must not allocate, log, read files, write files, spawn
  processes, block on locks, or call UI code.
- Every vendored algorithm needs a license note before it can be enabled.

## Proof of concept

`trk-audio` provides the optional `c-dsp-boundary` feature. It compiles the
project-authored C gain fixture in `src/c_dsp/vendor/` and exposes it through
`CNativeGainModule`, which mirrors the native module prepare/process/reset
lifecycle.

The fixture is intentionally small: it proves build integration, FFI isolation,
parameter validation, fixed-buffer deterministic processing, non-finite sample
rejection, buffer-shape checks, and reset behavior without introducing a large
external dependency.

## License review notes

| Algorithm | Source | License | Distribution notes |
| --- | --- | --- | --- |
| `trk_c_gain` | Project-authored fixture in `crates/trk-audio/src/c_dsp/vendor/` | MIT, same as this repository | No third-party source; safe to build in CI under `c-dsp-boundary`. |

Future wrappers must add a row before landing. The row should identify the
upstream project, exact version or commit, license, attribution requirements,
and any static-linking or source-distribution obligations.
