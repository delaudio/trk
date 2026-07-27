# Responsive transport header

Owning ADR: `../../adr/0019-compose-the-transport-header-by-width.md`

GitHub issue: #265

## Scope

Replace arbitrary clipping of the transport line with explicit width-aware
compositions made from whole segments. Preserve exact Play and Stop pointer
targets and all existing transport behavior.

## Exit criteria

1. The 72- and 80-column compositions retain Play, Stop, BPM, LPB, playback
   state, pattern, and row (ADR AC1).
2. The 100-column composition adds complete synchronization status (ADR AC2).
3. The 140-column composition exposes the complete header (ADR AC3).
4. Every selected composition fits its inner width without partially clipped
   optional labels (ADR AC4).
5. Play and Stop targets match their visible symbols at all supported widths
   (ADR AC5).
6. Focused tests and snapshots cover 72×24, 80×24, 100×28, and 140×36
   (ADR AC6).

## Dependencies

- `../../adr/0014-control-play-and-stop-with-distinct-pointer-targets.md`
- `../done/2026-07-27-distinct-transport-click-targets.md`
- GitHub issue #260 (closed).

---

Shipped at HEAD `632c0c5` via
[PR #301](https://github.com/delaudio/trk/pull/301), with GitHub
Actions CI run #330 green and issue #265 closed.
