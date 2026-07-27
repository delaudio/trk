---
adr: 0018
title: Distinguish unavailable workspace affordances
status: Accepted
date: 2026-07-27
owner: default-agent
supersedes:
superseded-by:
depends-on: [0003]
tags: [tui, ux, affordance, sampler, workspace]
---

# ADR 0018 — Distinguish unavailable workspace affordances

## Context

The large Renoise-inspired pattern and sampler workspaces reproduce tabs and
toolbar labels from the reference application. Some are implemented and
clickable, while others are decorative placeholders rendered with nearly the
same treatment. Users cannot tell capability from imitation before clicking.

## Capability statement

Workspace chrome uses explicit active, enabled, and disabled visual states.
Unavailable controls carry a compact `×` marker plus a dim disabled style and
never register semantic interaction targets. Implemented controls retain
active or accent treatment and their existing pointer behavior.

## User stories / scenarios

- As a user, I want to distinguish the current tab, another available tab, and
  an unavailable tab at a glance.
- As a sampler user, I do not want Record or editing toolbar placeholders to
  look actionable.
- As a pattern user, I want implemented inspector destinations to remain
  visibly available while the unsupported Other tab is disabled.
- As a pointer user, I do not want clicking disabled chrome to mutate project
  or navigation state.

## Acceptance criteria

1. Workspace tabs use three explicit states: active, enabled, and disabled;
   disabled tabs include `×`, use a shared dim style, and enabled pattern
   inspector tabs keep their existing navigation.
2. Unsupported sampler Record and legacy editing-toolbar labels use the same
   disabled marker/style and expose no semantic interaction payload.
3. Unsupported pattern MIDI Map chrome uses the shared disabled treatment.
4. Implemented sampler ADSR, adjustment, waveform, and Browse controls retain
   their enabled styles and exact interaction payloads.
5. Primary, secondary, and drag clicks on disabled sampler chrome do not mutate
   sampler state or navigate away; pattern Other remains non-mutating and may
   keep its existing concise unavailable notification.
6. Snapshot and style/interaction tests cover enabled and disabled affordances
   in both large workspaces.

## Out of scope

- Implementing recording, Draw, Normalize, Slice, FFT, plugin hosting, MIDI
  mapping, the Other inspector, or any other placeholder feature.
- Adding hover states or new workspace navigation destinations.

## Open questions

- None.

## References

- `0003-render-owned-interaction-regions.md`
- `../plan/todo/0016-disabled-workspace-affordances.md`
- GitHub issue #264.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-27 | r1 | default-agent | Accepted explicit disabled treatment for unavailable workspace chrome. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-27 | Authorised autonomous implementation, merge, and closeout of the queued UX issues in chat. |
