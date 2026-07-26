# MIDI Settings pointer controls

Owning ADR: `../../adr/0012-control-midi-settings-with-the-pointer.md`

GitHub issue: #258

## Scope

Separate the MIDI Settings header, virtualized fixed-row output-port list, and
fixed action bar. Expose render-owned targets for visible ports and the
Connect, Disconnect, Panic, Refresh, and Close actions, then route primary
clicks through existing application operations.

Routing-field editing, input-port configuration, wheel scrolling, secondary
clicks, and drag gestures are unchanged.

## Exit criteria

1. Visible fixed-height port rows carry absolute indices and keep selection in
   view for long lists (ADR AC1).
2. All five actions remain visible with distinct typed targets regardless of
   port count (ADR AC2).
3. Primary row clicks select, while action clicks use existing connect,
   disconnect, panic, refresh, and focus-restore paths (ADR AC3).
4. Empty state, empty geometry, chrome, outside clicks, secondary clicks,
   drags, and invalid payloads are no-ops (ADR AC4).
5. Empty-list, multiple-port, routing, geometry, and unchanged keyboard tests
   pass the full repository gate (ADR AC5).

## Dependencies

- `../../adr/0003-render-owned-interaction-regions.md`
- `../done/2026-07-26-render-owned-interaction-regions.md`
- GitHub issue #249 (closed by PR #269).

---

Shipped at HEAD `d706255` via
[PR #287](https://github.com/delaudio/salieri-tracker/pull/287), with GitHub
Actions CI run #300 green and issue #258 closed.
