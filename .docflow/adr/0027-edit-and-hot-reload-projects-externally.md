---
adr: 0027
title: Edit and hot-reload projects externally
status: Accepted
date: 2026-08-20
owner: default-agent
supersedes:
superseded-by:
depends-on: [0001]
tags: [workflow, editor, persistence, hot-reload, tui]
---

# ADR 0027 — Edit and hot-reload projects externally

## Context

`trk` projects are human-readable JSON, but the TUI currently loads and saves
them only through explicit application commands. Users cannot hand the active
project to Neovim, VS Code, Helix, or another preferred editor without leaving
the application, and edits made by another process are invisible until the
project is reopened manually.

The terminal boundary already supports suspending raw mode and the alternate
screen around an external sample-browser process. Persistence already validates
and atomically replaces complete project envelopes. External editing should
reuse both boundaries while protecting unsaved in-memory work, avoiding reload
loops after malformed writes, and preserving active MIDI/audio transport.

Issue #316 references `grain`, which launches `$EDITOR` after restoring the
terminal and compares modification times on each tick. `trk` additionally has
undo history, project-level variation metadata, asynchronous event reduction,
and a playback thread that owns a start-time song snapshot, so reload semantics
must be explicit rather than copying that implementation literally.

## Capability statement

`trk` can suspend its terminal, edit the current project in a platform-appropriate
external editor, and safely adopt valid external project-file changes in the
background without crashing, discarding dirty local edits, or interrupting the
currently running transport.

## User stories / scenarios

- As a tracker user, I want `e` to open the complete current project in my
  configured editor, so that I can make precise JSON edits without ending the
  TUI session.
- As a live coder, I want valid writes from another process to appear
  automatically while playback continues, so that scripts and the TUI can work
  together.
- As an editor, I want malformed or conflicting external writes reported once
  without replacing my current state, so that an incomplete save cannot destroy
  work or flood the status bar.

## Acceptance criteria

1. Lowercase `e` in normal Pattern mode requests external editing and does not
   alter Edit-mode note entry or other view-specific keys. The runner suspends
   raw mode, mouse capture, cursor hiding, and the alternate screen while the
   editor owns the terminal, then restores and redraws the TUI even when launch
   or exit fails.
2. Editor resolution uses a non-empty `EDITOR`, then `VISUAL`, then `nano` on
   macOS/Linux or `notepad` on Windows. A shell-free parser converts the chosen
   value to executable plus argv using one portable grammar: unquoted
   whitespace separates words, single and double quotes group words, and a
   backslash escapes only whitespace, a quote, or another backslash. Empty
   commands and unmatched quotes are rejected. Arguments such as `code --wait`
   are supported; an executable path containing spaces must be enclosed in
   single or double quotes, with no filesystem-prefix guessing. The project
   path is appended with the process API as a distinct argv value and is never
   interpolated into a shell command; launch/non-zero-exit diagnostics contain
   no project content.
3. For a named project, invoking `e` atomically saves the complete live project
   envelope first when it is dirty, opens that path, and immediately validates
   and adopts valid edits after a successful editor exit. For an unnamed
   project, the same envelope is exported to a unique temporary `.trk` scratch
   file; valid edits return to memory without assigning a permanent project
   path. Successful adoption removes the scratch. A launch failure removes the
   still-unedited scratch, while a non-zero editor exit or invalid edited
   project preserves it and reports its path so the user can recover or remove
   it explicitly.
4. The application watches only its current named `.trk` path. It initializes
   or refreshes a portable modification signature after load, save, editor
   preparation, and successful or failed external-change handling, and polls at
   a bounded interval rather than performing filesystem work every UI frame.
   Internal atomic saves do not trigger a redundant reload.
5. A valid external project replaces the in-memory song and variation history,
   clears stale undo/selection/modal state, preserves the nearest valid cursor
   and view indices, and becomes the clean disk baseline. Hot reload neither
   sends Stop nor changes the current playing flag, playhead, or sequence
   position; the existing transport keeps its start-time snapshot and the next
   transport start uses the reloaded project.
6. A watcher never overwrites dirty local state. It reports one conflict for a
   given external signature and retries adoption once local state is clean or
   the file changes again. Malformed JSON, unsupported formats, invalid project
   structure, missing files, editor failures, and metadata/read failures leave
   live state and undo history unchanged and produce a concise status message
   without panic or per-tick repetition.
7. Successful editor and watcher adoption display concise status messages.
   Automated tests cover editor selection/arguments, named and scratch flows,
   valid and invalid reloads, dirty conflict suppression, self-save suppression,
   transport preservation, shortcut compatibility, and terminal suspension;
   the complete repository gate passes on macOS, Linux, and Windows code paths.

## Out of scope

- Merging simultaneous local and external edits or presenting a conflict diff.
- Applying reloaded notes to a transport run that was already scheduled.
- Watching directories, samples, configuration, AI sessions, or multiple
  projects.
- Embedding an editor, implementing language-server features, or guaranteeing
  availability of any optional third-party editor.
- Persisting scratch files after a successful unnamed-project edit.

## Open questions

- None.

## References

- `0001-record-architecture-decisions.md`
- `../plan/todo/0025-external-editor-hot-reload.md`
- GitHub issue #316.
- [grain editor handoff](https://github.com/delaudio/grain/blob/c609fcd5a05d8862d88819690a051f5df13238be/src/main.rs#L124-L177)
- [grain file watcher](https://github.com/delaudio/grain/blob/c609fcd5a05d8862d88819690a051f5df13238be/src/app.rs#L446-L466)

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-08-20 | r1 | default-agent | Recorded and accepted guarded external editing, bounded hot reload, and transport-preserving adoption. |
| 2026-08-20 | r2 | default-agent | Defined shell-free portable editor argv parsing and distinct project-path argument passing after Norn review. |
| 2026-08-20 | r3 | default-agent | Required quotes for spaced executable paths and defined scratch recovery/cleanup after the bounded review rerun. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-08-20 | Approved autonomous resolution of the prioritized GitHub issue queue in chat. |
