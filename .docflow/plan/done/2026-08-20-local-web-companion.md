# Local web companion

Owning ADR: `../../adr/0028-expose-a-local-web-companion.md`

GitHub issue: #317

## Scope

Add a lazily started loopback HTTP bridge, a self-contained responsive Canvas
visualizer, bounded projected song/transport/meter state, and queued local
transport/mute/solo/pattern actions. Bind `b` only in normal tracker mode, open
the actual selected URL through a platform process, and preserve a copyable URL
when graphical opening is unavailable.

## Exit criteria

1. Shortcut routing, lazy reuse, selected-URL opening, and compatibility satisfy
   ADR AC1.
2. Loopback binding, bounded HTTP handling, lifecycle, and startup diagnostics
   satisfy ADR AC2.
3. The self-contained responsive Canvas interface satisfies ADR AC3.
4. Projected versioned state, callback-safe meter reads, and privacy boundaries
   satisfy ADR AC4–AC5.
5. Strict same-origin actions and bounded TUI-thread delivery satisfy ADR AC6.
6. Cross-platform opener behavior and headless fallback satisfy ADR AC7.
7. Focused unit/integration coverage, a loopback smoke test, and the complete
   repository gate satisfy ADR AC8.
8. Norn review with the Codex provider has no unresolved actionable findings
   before publication.

## Dependencies

- `../../adr/0001-record-architecture-decisions.md`
- `../../adr/0026-calibrate-realtime-output-with-live-metering.md`
- `../../adr/0027-edit-and-hot-reload-projects-externally.md`
- Maintainer approval to execute issue #317 autonomously.

---

Shipped in implementation squash `9e8ba14` via
[PR #342](https://github.com/delaudio/trk/pull/342), with GitHub Actions CI
run `32368367097` green and issue #317 closed.
