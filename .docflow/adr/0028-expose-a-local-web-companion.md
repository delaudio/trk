---
adr: 0028
title: Expose a local web companion
status: Accepted
date: 2026-08-20
owner: default-agent
supersedes:
superseded-by:
depends-on: [0001, 0026, 0027]
tags: [web, visualization, transport, audio, tui]
---

# ADR 0028 — Expose a local web companion

## Context

The terminal tracker intentionally prioritizes dense keyboard editing, but a
terminal cell grid cannot provide a high-resolution oscilloscope, piano-roll
overview, arrangement map, or touch-friendly transport surface on a second
display. The application already owns live transport position, the complete
song model, mutable mute/solo state, and lock-free master audio meters, but it
does not expose a safe companion boundary for another local process or browser.

Issue #317 references `grain`, whose `b` shortcut starts a small `std::net`
HTTP server on loopback and opens a browser. That prototype proves the useful
shape, but its page depends on a public CDN, its client assumes port 3333 even
when fallback binding selects another port, and it has no authenticated action
or lifecycle contract. `trk` also needs browser actions to return to the TUI
thread instead of mutating application state from a connection thread.

## Capability statement

`trk` can start and open a dependency-light loopback web companion that renders
live transport, arrangement, active notes, and audio metering on a responsive
Canvas surface and can safely request bounded transport, pattern, mute, and
solo actions through the application's existing state-transition boundary.

## User stories / scenarios

- As a performer, I want `b` in normal tracker mode to open a large, smooth
  visualizer on another display without stopping playback.
- As an arranger, I want the browser to show the active pattern, current row,
  note activity, tracks, and song sequence with low visible latency.
- As a local operator, I want touch-friendly play/stop, pattern, mute, and solo
  controls whose effects remain serialized with keyboard and mouse input.
- As a headless user, I want the server URL reported even when no graphical
  browser opener is available.

## Acceptance criteria

1. Lowercase `b` in normal tracker mode requests the web companion. The
   sampler-mode `b`/`B` sample browser, uppercase normal-mode `B`, Edit-mode
   note entry, and configurable keymap precedence remain unchanged. Repeated
   requests reuse the live server and open its actual bound URL rather than
   starting duplicate listeners.
2. A background server implemented with `std::net` and no HTTP framework binds
   only to `127.0.0.1`, tries port 3333 and a bounded sequence of following
   ports, and publishes the selected authority. Startup failure is reported
   without terminating the TUI. Server shutdown is owned and joined; request
   headers and bodies are size-bounded, socket operations have deadlines, and
   malformed or unsupported requests receive finite HTTP error responses.
3. `GET /` returns one self-contained UTF-8 HTML document with inline CSS and
   JavaScript and no network-loaded fonts, scripts, images, or other assets.
   Its responsive Canvas 2D view renders transport, a full-width active-pattern
   piano roll, the song arrangement, per-track note activity and mute/solo
   state, and master low/mid/high/RMS/peak meters. Canvas drawing follows the
   display pixel ratio and animation frames stay decoupled from state polling.
4. `GET /api/state` returns a versioned JSON snapshot containing song title,
   transport state, current sequence/pattern/row/tick, bounded pattern and
   arrangement data, track identity/name/channel/mute/solo/armed state, active
   notes and velocities, and finite master meters. The TUI thread replaces the
   snapshot at its normal tick cadence; HTTP readers never borrow or mutate the
   live `App` or block the audio callback.
5. The page polls state at most every 50 ms, permits only one in-flight state
   request, aborts stale requests, and visibly reports disconnection while
   retaining its last valid frame. JSON generation and response size are
   bounded by the current song model and never include project paths, sample
   paths, AI configuration, environment values, or project file contents
   outside the explicitly projected musical state.
6. `POST /api/action` accepts strict JSON actions for toggle playback, stop,
   select pattern, toggle track mute, and toggle track solo. It requires
   `application/json` plus a non-simple custom request header, validates
   same-origin browser requests and the bound Host, rejects unknown fields,
   methods, media types, oversized bodies, and out-of-range indices, and queues
   accepted actions on a bounded channel. The TUI thread drains that channel
   and invokes existing application actions; a full queue returns a finite
   retryable response instead of blocking either thread.
7. Browser opening uses direct process APIs with the selected URL as one argv
   value: `open` on macOS, `xdg-open` on Linux/other Unix, and the Windows
   command interpreter's `start` builtin on Windows. Missing GUI environment,
   spawn failure, or non-successful opener exit leaves the server running and
   reports a copyable URL as the headless fallback; no shell interpolates
   project or song data.
8. Focused tests cover fallback binding and reuse, HTTP routing and bounds,
   same-origin action validation, bounded action delivery, snapshot projection,
   shortcut compatibility, opener selection/fallback, and clean shutdown. A
   live loopback smoke test exercises the served document and state/action
   round trip, and the complete repository gate passes.

## Out of scope

- Binding to LAN interfaces, remote/tablet discovery, TLS, authentication for
  non-loopback clients, or access outside the local machine.
- Streaming project files, samples, raw audio buffers, or persistent settings.
- Replacing the TUI, editing pattern cells from the browser, or promising a
  stable public HTTP API in this first version.
- Adding WebSocket, Server-Sent Events, FFT libraries, HTTP frameworks, a web
  build toolchain, or browser-side audio capture.
- Exact per-track post-DSP audio metering; this version shows per-track note
  activity alongside the existing callback-safe master frequency and level
  meters.

## Open questions

- None.

## References

- `0001-record-architecture-decisions.md`
- `0026-calibrate-realtime-output-with-live-metering.md`
- `0027-edit-and-hot-reload-projects-externally.md`
- `../plan/todo/0026-local-web-companion.md`
- GitHub issue #317.
- [grain web server](https://github.com/delaudio/grain/blob/c609fcd5a05d8862d88819690a051f5df13238be/src/web/server.rs)
- [grain browser shortcut](https://github.com/delaudio/grain/blob/c609fcd5a05d8862d88819690a051f5df13238be/src/app.rs)

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-08-20 | r1 | default-agent | Recorded and accepted the bounded loopback companion, projected state, Canvas view, safe action bridge, and browser fallback. |
| 2026-08-20 | r2 | default-agent | Linked the established shell-free external-process boundary to browser opening after Norn review. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-08-20 | Approved autonomous resolution of the prioritized GitHub issue queue in chat. |
