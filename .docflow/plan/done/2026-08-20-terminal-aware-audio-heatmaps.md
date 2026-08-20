# Terminal-aware audio heatmaps

Owning ADR: `../../adr/0029-render-audio-intensity-with-terminal-aware-color.md`

GitHub issue: #318

## Scope

Add one startup-resolved terminal color-mode capability, reusable finite-safe
RGB/HSB gradient mapping, semantic sampler waveform heatmaps with transient,
zero-crossing, start/end, and loop markers, and decibel-aware live calibration
meter gradients. Preserve the existing waveform geometry, responsive layout,
audio-thread boundary, explicit `NO_COLOR` behavior, and deterministic
fallbacks. Full mixer-console work and sample slicing remain outside this item.

## Exit criteria

1. Startup color-mode detection and render-state propagation satisfy ADR AC1.
2. Pure RGB/HSB gradient and fallback mapping satisfy ADR AC2 and AC6.
3. Styled waveform geometry, transient/zero-crossing semantics, and bounded
   frame-marker projection satisfy ADR AC3–AC4.
4. Live calibration meter heatmaps satisfy ADR AC5 without changing callback
   or project state.
5. Unit, buffer-style, snapshot, and fallback coverage satisfy ADR AC7.
6. Complexity stays bounded by visible cells and the complete repository gate
   satisfies ADR AC8.
7. Norn review with the Codex provider has no unresolved actionable findings
   before publication.

## Dependencies

- `../../adr/0001-record-architecture-decisions.md`
- `../../adr/0017-control-supported-sampler-actions-with-the-pointer.md`
- `../../adr/0026-calibrate-realtime-output-with-live-metering.md`
- Maintainer approval to execute issue #318 autonomously.

---

Shipped at HEAD `287460b1bbc1987468518ca0817d1b7aa98d6522` via
[PR #344](https://github.com/delaudio/trk/pull/344), with GitHub Actions CI
run `32372743179` green and issue #318 closed.
