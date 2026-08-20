---
adr: 0024
title: Select AI proposal engines at runtime
status: Accepted
date: 2026-08-20
owner: default-agent
supersedes:
superseded-by:
depends-on: [0001]
tags: [ai, tui, providers, configuration, subprocess]
---

# ADR 0024 — Select AI proposal engines at runtime

## Context

The AI proposal workflow is currently fixed to the provider selected when the
application starts. The local deterministic and mock providers work, while the
generic command-provider configuration only reports that external adapters are
reserved for future integration. Users therefore cannot discover installed AI
engines, understand why an engine is unavailable, or switch proposal generation
without editing configuration and restarting `trk`.

External engines have different invocation and authentication requirements.
CLI engines must be discovered from `PATH` and invoked without a shell, while an
API-backed OpenAI engine depends on credentials. Selection must never expose
credential values, and provider output must be parsed and validated as the same
reviewable cell-level proposal used by local generation.

## Capability statement

`trk` discovers supported local and credential-backed AI engines, presents their
availability in an in-TUI selector, switches the active proposal engine at
runtime, and converts successful external responses into validated cell-level
proposals without restarting the application.

## User stories / scenarios

- As a musician, I want to see which AI engines are available on my machine, so
  that I can choose a working engine without editing configuration files.
- As a live user, I want to change the active engine from the AI workspace, so
  that subsequent proposals use it immediately without interrupting the session.
- As a security-conscious user, I want missing credentials reported by name but
  never displayed, so that engine discovery is actionable without leaking
  secrets.
- As a contributor, I want external responses normalized into the existing
  proposal model, so that preview, validation, apply, and undo remain provider
  independent.

## Acceptance criteria

1. Engine discovery always includes the built-in deterministic engine, detects
   `claude`, `codex`, and `ollama` executables from `PATH`, detects OpenAI
   availability from `OPENAI_API_KEY`, and reports unavailable engines without
   panicking or exposing credential values.
2. In the AI workspace, `m` opens a double-bordered engine selector; Up/Down
   changes the selected row, Enter activates an available engine, and Esc closes
   the selector without changing the active engine.
3. Activating an engine updates the runtime provider configuration and visible
   active-engine badge immediately; the next proposal uses the selected engine
   without an application restart.
4. CLI engines are launched directly without a shell. Provider-specific fixed
   arguments request non-interactive JSON output; the complete composition
   request is supplied through stdin, and stdout must contain one JSON object
   with a non-empty `summary` plus an `edits` array of tagged `set_note` or
   `clear_cell` operations carrying pattern, row, and track coordinates.
   Unknown or missing fields, non-zero exit, malformed stdout, and bounded
   stderr diagnostics map to provider errors without applying partial output.
5. External execution has a configurable finite timeout, observes task
   cancellation, terminates and reaps the child process when cancelled or timed
   out, and reports the distinction between cancellation, timeout, launch, exit,
   and response-parse failures without blocking the TUI event loop.
6. Automated tests cover discovery with present and absent executables and
   credentials, selection state and keyboard behavior, runtime provider
   switching, selector rendering, external JSON response parsing, and bounded
   process failure/cancellation behavior.

## Out of scope

- Persisting an in-session engine choice back to the user's configuration file.
- Installing AI engines, provisioning API credentials, or managing accounts.
- Streaming partial external responses or applying proposals without the
  existing preview and validation flow.
- Provider-specific conversation history beyond the current AI session model.

## Open questions

- None.

## References

- `0001-record-architecture-decisions.md`
- `../plan/todo/0022-runtime-ai-engine-selection.md`
- GitHub issue #313.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-08-20 | r1 | default-agent | Recorded and accepted runtime AI engine discovery, selection, and external proposal generation. |
| 2026-08-20 | r2 | default-agent | Defined the external JSON protocol and bounded timeout/cancellation contract after Norn review. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-08-20 | Approved autonomous resolution of the prioritized GitHub issue queue in chat. |
