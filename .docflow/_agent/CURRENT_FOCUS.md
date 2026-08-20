# Current Focus

This file is the live snapshot of any in-flight session. It is short on purpose
— the durable record lives in git, `_agent/WORKLOG.md`, and `plan/done/`. The
queued work lives in `plan/todo/`.

If status files and git disagree, git is authoritative; correct this file.

## Active state

- **Branch:** `agent/318-truecolor-heatmaps`.
- **Active item:** `.docflow/plan/todo/0027-terminal-aware-audio-heatmaps.md`.
- **Blockers:** none.
- **Pending integration:** publish the accepted decision and plan in a draft PR,
  then implement, review, verify, and squash-merge issue #318.

## Last shipped

Issue #317 implementation via PR #342 and Docflow closeout via PR #343, both
with CI green; ADR 0028 is Implemented and the GitHub issue is closed.

## Next item

Implement ADR 0029 through the issue #318 PR, close its Docflow state after the
implementation merge, then clear operational context before selecting #319.

## Exit criteria

1. Startup terminal color-depth detection and render-state propagation satisfy
   ADR AC1.
2. Pure finite-safe RGB/HSB gradient and depth fallback mapping satisfy ADR
   AC2 and AC6.
3. Waveform intensity, transient/zero-crossing treatment, and bounded sample
   markers satisfy ADR AC3–AC4.
4. Live calibration meter gradients satisfy ADR AC5.
5. Deterministic unit, style-buffer, snapshot, and complete repository gates
   satisfy ADR AC7–AC8.
6. Norn review with the Codex provider has no unresolved actionable findings
   before publication.
