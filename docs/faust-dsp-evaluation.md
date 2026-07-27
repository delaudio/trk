# Faust DSP Evaluation

## Decision

Support Faust first as an optional source format for generating reviewed native
trk modules, with the C++ backend as the preferred first target. Do not add
a runtime Faust compiler to the tracker and do not store Faust-internal state in
`.trk` files.

The recommended path is:

1. Compile selected `.dsp` sources outside the realtime callback into C++.
2. Wrap the generated DSP through the #115 C/C++ boundary.
3. Map Faust UI metadata into trk `NativeModuleDescriptor` and
   `NativeModuleState` plain data.
4. Use the #117 WebAssembly ABI only for browser/Web Audio export after native
   behavior is proven.

Official Faust docs describe multiple compiler targets, including C, C++,
Rust, LLVM IR, and WebAssembly: <https://faustdoc.grame.fr/manual/compiler/>.
The Faust deployment docs also cover Web/WASM workflows:
<https://faustdoc.grame.fr/manual/deploying/>.

## Target comparison

| Target | Fit | Decision |
| --- | --- | --- |
| C++ | Best fit for #115 wrappers, mature Faust target, debuggable generated code. | Recommended first. |
| C | Viable for small DSP kernels, but Faust's C++ architecture files are the normal integration route. | Secondary. |
| Rust | Attractive long term, but generated API/stability must be reviewed before project schema commitments. | Defer. |
| LLVM IR | Powerful but increases toolchain and CI complexity. | Defer. |
| WebAssembly | Good for browser export, not terminal realtime by default. | Use after #117 export ABI matures. |

## Metadata mapping

`trk-interop::faust` contains a host-side mapper from normalized Faust UI
metadata into trk native module descriptors. It validates:

- non-empty module IDs and names;
- non-empty parameter lists;
- duplicate UI addresses;
- finite min/max/default/step values;
- default values inside range;
- unit mapping for Hz, dB, percent, normalized, bipolar, and plain float
  controls.

The mapper returns a default `NativeModuleState` that validates against the
generated descriptor, so Faust-generated parameters are visible through the same
plain native module API as hand-written effects.

## Proof of concept status

The current proof of concept is intentionally compiler-free for CI:

- metadata for a representative Faust-style filter maps to trk descriptors
  and default state;
- the #115 C gain fixture proves deterministic native wrapper rendering and
  realtime-safe prepare/process/reset shape;
- the recommended next implementation can replace that fixture with
  Faust-generated C++ once a concrete `.dsp` source and license review are
  selected.

This closes the architecture spike without making Faust a hard build
dependency.

## Licensing and source distribution

Every integrated Faust module must document:

- original `.dsp` source path and upstream project/version;
- Faust compiler version and architecture file used;
- upstream license and attribution requirements;
- whether generated source must be distributed;
- whether static linking changes obligations.

Until those notes exist, the module may not be enabled outside an experimental
feature.
