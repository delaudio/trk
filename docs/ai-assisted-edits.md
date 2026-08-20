# AI-Assisted Edits

AI-assisted composition is optional. The `trk-ai` crate models requests,
reviewable proposals, engine discovery, external adapters, and explicit
application of edits to a `trk-core::Song`.

The built-in default is `local_deterministic` and never contacts a network
service; `mock` is available for tests and dry runs. External CLI or OpenAI
engines receive the current song and selection context only after the user
activates that engine and submits a prompt. They must pass binary and credential
checks before trk queues work.

Current foundation:

- `AiPatternRequest` describes a prompt-scoped generation request;
- `AiProposalProvider` isolates proposal generation from the core editor;
- `AiProposal` stores a summary plus concrete cell edits for review;
- `preview_proposal` validates touched cells without mutating the song;
- `apply_proposal` mutates only after explicit approval and returns the same touched-cell preview.

In-app workflow:

```text
:ai chat
:ai provider
:ai propose PROMPT
:ai guidance list
:ai guidance show FILE
:ai guidance apply FILE
:ai guidance clear
:ai show
:ai accept
:ai reject
:ai save
:ai load
:ai delete
:ai retention N
```

`:ai chat` opens the tracker-native AI Chat view. The view shows local thread
messages, provider/status, selected pattern/track/row context, and a composer.
Typing a prompt and pressing `Enter` submits it through the same reviewable
proposal path as `:ai propose`; `Esc` returns to the tracker and `Ctrl+C`
requests cancellation of the active task.

With an empty composer, press `m` to open the engine selector. It discovers the
built-in engine, `claude`, `codex`, and `ollama` on `PATH`, plus the OpenAI API
when `OPENAI_API_KEY` and `curl` are available. `.env` can provide
`OPENAI_API_KEY`, `TRK_AI_PROVIDER`, and `TRK_AI_MODEL`; process environment
values take precedence. The selector shows missing requirements without showing
credential values. Use Up/Down to move, Enter to activate, and Esc to close.
The provider badge changes immediately, and the next prompt uses the selected
engine without restarting trk.

When a proposal is ready, the chat view treats the pending proposal as the
selected review target. Its preview panel lists the full touched cell set, the
touched pattern/track/row areas, the explicit absence of instrument, automation,
or mixer changes for current cell-only proposals, and the available actions.
With an empty composer, press `a` to apply the selected proposal, `r` to reject
it, or `p` to append the preview summary to the thread. Applied proposals still
go through the normal undo transaction stack, so `Ctrl+Z` restores the
pre-accept song state.

AI jobs are queued on the application task runtime and then reported back into
the chat thread. The TUI surfaces queued, running, cancelling, completed,
failed, and cancelled states without blocking tracker input. Phase progress
events include the current percentage when a total is known, the phase name, the
tool/provider label, and the diagnostic text produced by the job. Final proposal
summaries are appended as assistant messages; failures are appended as error
messages. Cancellation marks the job cancelled and leaves the song and pending
proposal slot unchanged.

`:ai provider` reports the configured provider, model, and availability before a
prompt is submitted. `:ai propose` submits the active provider to the application
[task runtime](tasks.md) with the current song, pattern, and track as context.
CLI engines are launched directly without a shell, receive the structured
request on stdin, and must return one JSON object containing a non-empty
`summary` and `set_note`/`clear_cell` edits. Missing credentials, missing
binaries, invalid JSON, unsuccessful exits, timeouts, and cancellation are
reported without applying partial output. The TUI remains responsive while
generation and preview validation run. A successful task stores a pending
proposal, appends an assistant message, and reports the touched cells without
mutating the song. `:ai show` repeats the summary. `:ai accept` applies the
proposal through the normal undo transaction mechanism, so `Ctrl+Z` can revert
the generated edit. `:ai reject` clears the pending proposal without changing
the song.

Local guidance files can be used to steer proposals without adding any remote
dependency. Configure `[ai].guidance_dirs` with directories containing `.md`,
`.txt`, or `.json` files. `:ai guidance list` appends the discovered local files
to the chat thread, `:ai guidance show FILE` appends the selected file contents,
and `:ai guidance apply FILE` keeps that file active for subsequent prompts.
When guidance is active, trk prepends the local file content and source path
to the prompt sent to the configured provider while preserving the user-visible
chat prompt unchanged. `:ai guidance clear` removes the active guidance. Missing
files, unreadable directories, ambiguous selectors, unsupported extensions, and
malformed JSON are reported as AI guidance diagnostics before a proposal is
queued.

AI chat persistence is local and provider-agnostic. Configure
`[ai].session_file` to a JSON file path to autosave the thread after each
message and load it on startup. The saved session stores thread metadata,
message roles, text blocks, timestamps, status, and linked project path; it does
not store pending proposals as applied song data. `:ai save` and `:ai load`
manually persist or restore the configured file, `:ai delete` removes it and
resets the local thread without mutating the current project, and
`:ai retention N` trims saved history to the most recent N messages while
keeping a system message.

CLI integrations should print or serialize proposals before applying them so
generated changes remain reviewable.
