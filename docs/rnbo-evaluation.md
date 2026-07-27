# RNBO Interoperability Evaluation

## Decision

Do not make RNBO a core trk module system. RNBO interop is viable only at
controlled boundaries:

- RNBO C++ source export may be reviewed and wrapped through the #115 C/C++
  native boundary.
- RNBO Web Export artifacts may inform a browser/Web Audio export path through
  the #117 WebAssembly boundary.
- trk should not store opaque RNBO runtime state in `.trk` files until a
  follow-up architecture review defines an explicit schema.
- Cloud/plugin binary export workflows are not a foundation for trk core.

Official RNBO references used for this evaluation:

- Export targets overview:
  <https://rnbo.cycling74.com/learn/export-targets-overview>
- Web Export target:
  <https://rnbo.cycling74.com/learn/the-web-export-target>
- Export Platform FAQ:
  <https://support.cycling74.com/hc/en-us/articles/10954722178579-RNBO-Export-Platform-FAQ>
- Authorization:
  <https://support.cycling74.com/hc/en-us/articles/10500185155603-RNBO-Authorization>

## Workflow assessment

`trk-interop::rnbo` records the current decision matrix in testable plain
data.

| Workflow | Viability | Decision |
| --- | --- | --- |
| RNBO C++ source export | Viable after export | Review generated source and wrap through #115. Store only trk descriptors/parameters. |
| RNBO Web Export JSON + `@rnbo/js` | Viable for web export | Use only behind #117 browser export boundary. Do not import RNBO runtime concepts into `trk-core`. |
| RNBO cloud plugin binary export | Not recommended | Cloud/export-platform and binary distribution constraints do not fit trk project data or terminal runtime. |
| trk-to-RNBO wrapper generation | Deferred | Needs an architecture review for schema ownership and reverse mapping. |
| Importing opaque RNBO runtime state | Not recommended | Project files must stay explicit and inspectable. |

## Required coverage

- Cloud compiler/export-platform dependency: rejected for trk core because
  it creates an external service dependency for artifacts that should be
  inspectable and reproducible.
- Max/RNBO licensing and authorization: any workflow requires contributor/user
  license review before generated artifacts ship.
- VST3/binary distribution restrictions: binary plugin export is outside the
  native module boundary and should not influence `.trk` schema.
- Offline C++ export availability: viable after export because generated source
  can be vendored, reviewed, tested, and wrapped like other C/C++ DSP code.

## Follow-up requirements before implementation

Any concrete RNBO integration must add:

- exact RNBO export target and version;
- generated artifact provenance and license note;
- explicit parameter manifest mapping to trk descriptors;
- deterministic offline render fixtures;
- proof that no RNBO-specific opaque state is stored in projects;
- failure behavior for missing artifacts, authorization errors, and parameter
  mismatches.
