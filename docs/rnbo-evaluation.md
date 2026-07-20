# RNBO Interoperability Evaluation

## Decision

Do not make RNBO a core Salieri module system. RNBO interop is viable only at
controlled boundaries:

- RNBO C++ source export may be reviewed and wrapped through the #115 C/C++
  native boundary.
- RNBO Web Export artifacts may inform a browser/Web Audio export path through
  the #117 WebAssembly boundary.
- Salieri should not store opaque RNBO runtime state in `.salieri` files until a
  follow-up ADR defines an explicit schema.
- Cloud/plugin binary export workflows are not a foundation for Salieri core.

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

`salieri-interop::rnbo` records the current decision matrix in testable plain
data.

| Workflow | Viability | Decision |
| --- | --- | --- |
| RNBO C++ source export | Viable after export | Review generated source and wrap through #115. Store only Salieri descriptors/parameters. |
| RNBO Web Export JSON + `@rnbo/js` | Viable for web export | Use only behind #117 browser export boundary. Do not import RNBO runtime concepts into `salieri-core`. |
| RNBO cloud plugin binary export | Not recommended | Cloud/export-platform and binary distribution constraints do not fit Salieri project data or terminal runtime. |
| Salieri-to-RNBO wrapper generation | Deferred | Needs ADR for schema ownership and reverse mapping. |
| Importing opaque RNBO runtime state | Not recommended | Project files must stay explicit and inspectable. |

## Required coverage

- Cloud compiler/export-platform dependency: rejected for Salieri core because
  it creates an external service dependency for artifacts that should be
  inspectable and reproducible.
- Max/RNBO licensing and authorization: any workflow requires contributor/user
  license review before generated artifacts ship.
- VST3/binary distribution restrictions: binary plugin export is outside the
  native module boundary and should not influence `.salieri` schema.
- Offline C++ export availability: viable after export because generated source
  can be vendored, reviewed, tested, and wrapped like other C/C++ DSP code.

## Follow-up requirements before implementation

Any concrete RNBO integration must add:

- exact RNBO export target and version;
- generated artifact provenance and license note;
- explicit parameter manifest mapping to Salieri descriptors;
- deterministic offline render fixtures;
- proof that no RNBO-specific opaque state is stored in projects;
- failure behavior for missing artifacts, authorization errors, and parameter
  mismatches.
