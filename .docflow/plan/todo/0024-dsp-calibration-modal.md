# DSP calibration modal

Owning ADR: `../../adr/0026-calibrate-realtime-output-with-live-metering.md`

GitHub issue: #315

## Scope

Add a session-local realtime calibration processor and lock-free meter bridge,
wire it through playback without restarting transport, and expose the eight
controls plus live band/RMS/peak meters in a normal-mode `t` modal. Preserve the
persisted mixer and DSP graph, `Ctrl+t`, uppercase `T`, dirty state, and offline
rendering.

## Exit criteria

1. Calibration settings, ranges, balanced defaults, and reset satisfy ADR AC1.
2. Callback-safe control and meter transfer satisfies ADR AC2.
3. Selected-track gain and post-master band/gate/master/AGC processing satisfy
   ADR AC3.
4. Finite, smoothed master and band meters satisfy ADR AC4.
5. The modal and complete keyboard contract satisfy ADR AC5.
6. Live updates are transport-safe and non-persistent, and the modal presents
   the required controls and meters (ADR AC6).
7. Focused audio, app, routing, rendering, and snapshot coverage plus the full
   repository gate pass (ADR AC7).
8. Norn review with the Codex provider has no unresolved in-scope findings
   before publication.

## Dependencies

- `../../adr/0001-record-architecture-decisions.md`
- Maintainer approval to execute issue #315 autonomously.
