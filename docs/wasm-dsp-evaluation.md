# WebAssembly DSP Evaluation

## Decision

Use WebAssembly first as a browser/Web Audio export boundary for selected
Salieri modules. Do not make WASM the primary terminal realtime DSP runtime yet.
Native Rust modules and reviewed C/C++ wrappers remain the realtime path until a
follow-up spike measures desktop WASM host overhead, memory behavior, and
scheduling safety under realistic callback deadlines.

This decision is based on:

- WebAssembly's official tooling direction, which supports multiple source
  languages and web/non-web embeddings without prescribing one host runtime:
  <https://webassembly.org/docs/tooling/>
- Web Audio's render-quantum processing model. The W3C Web Audio API describes
  block rendering, a default 128-frame render quantum, and AudioWorklet
  processors running on the audio rendering thread:
  <https://www.w3.org/TR/webaudio-1.1/>
- The existing Salieri native module and #115 C/C++ boundary, which already
  provide deterministic offline/realtime processing without adding a sandboxed
  runtime to the terminal app.

## Compared workflows

| Workflow | Fit | Decision |
| --- | --- | --- |
| Browser AudioWorklet + generated JS/WASM bundle | Best fit for web export. Aligns with Web Audio render quanta and browser deployment. | Recommended first target. |
| Desktop host runtime such as wasmtime/wasmer | Useful for sandboxing later, but adds callback scheduling, memory copy, and runtime configuration risk. | Deferred pending benchmarks and failure-mode tests. |
| Generated JS/WASM artifacts only | Good for static export and sharing, but not enough for terminal realtime playback. | Use as export packaging, not internal engine. |

## ABI v1

The host-side ABI contract lives in `salieri-audio::wasm_dsp`.

- Audio buffers are interleaved `f32`.
- `WASM_DSP_ABI_VERSION` is `1`.
- Supported channel counts are currently mono or stereo.
- The host validates sample rate, block frame count, input/output lengths,
  parameter count, parameter ordering, finite parameter values, and finite
  samples before processing.
- Parameter state is an ordered plain-data array of `WasmDspParameterValue`
  entries. Project files must not store opaque WASM runtime state.
- The deterministic `render_wasm_dsp_gain_fixture` is a host-side reference
  fixture for ABI validation, not a desktop WASM runtime.

## Browser export constraints

Browser export should generate an AudioWorklet wrapper around a WASM module and
adapt to the active `BaseAudioContext` sample rate and render quantum. The
default render quantum is 128 frames, but Web Audio 1.1 also exposes a
`renderSizeHint`, so generated wrappers must not hard-code one size forever.

The browser wrapper owns:

- module loading and initialization;
- copying or mapping AudioWorklet channel buffers into the ABI layout;
- converting Web Audio `AudioParam` values into ordered ABI parameters;
- reporting load or parameter errors to the main thread without blocking the
  render thread.

## Terminal realtime constraints

The terminal app should not execute WASM DSP in the realtime callback until a
follow-up proves:

- no allocations or locks in the callback path after prepare;
- bounded per-block execution time under expected buffer sizes;
- deterministic reset/state restore behavior;
- explicit handling for traps, non-finite output, parameter mismatch, and
  unsupported channel layouts;
- parity with offline rendering.

Until then, native Rust and reviewed C/C++ wrappers remain the supported
terminal realtime implementations.
