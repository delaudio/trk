# Disabled workspace affordances

Owning ADR: `../../adr/0018-distinguish-unavailable-workspace-affordances.md`

GitHub issue: #264

## Scope

Introduce explicit active, enabled, and disabled workspace chrome states.
Apply the disabled state to unsupported sampler tabs, Record/editing
placeholders, pattern Other and MIDI Map placeholders while preserving
implemented pattern inspector navigation and sampler direct controls.

No placeholder feature is implemented by this item.

## Exit criteria

1. Active, enabled, and disabled tab states are explicit; disabled tabs use a
   common marker/style and implemented pattern tabs still navigate (ADR AC1).
2. Unsupported sampler Record/editing labels are disabled and have no semantic
   payloads (ADR AC2).
3. Pattern MIDI Map chrome uses the same disabled treatment (ADR AC3).
4. Existing sampler direct controls retain their enabled payloads and visual
   treatment (ADR AC4).
5. Disabled sampler clicks and pattern Other clicks are non-mutating across
   primary, secondary, and drag input (ADR AC5).
6. Focused style/interaction tests plus intentional pattern/sampler snapshots
   cover both enabled and disabled affordances (ADR AC6).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- `../done/2026-07-27-sampler-direct-pointer-controls.md`
- GitHub issues #249 and #263 (closed).

---

Shipped at HEAD `a7c93e3` via
[PR #299](https://github.com/delaudio/salieri-tracker/pull/299), with GitHub
Actions CI run #326 green and issue #264 closed.
