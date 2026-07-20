# AI-Assisted Edits

AI-assisted composition is post-MVP and optional. The initial boundary is the `salieri-ai` crate, which models requests, reviewable proposals, and explicit application of edits to a `salieri-core::Song`.

The crate does not contact network services. External providers can be added later behind an explicit provider implementation, but project data must not leave the machine unless the user invokes that provider intentionally. The built-in default is `local_deterministic`; `mock` is available for tests and dry runs. Future command-backed providers must pass configured binary and environment checks before Salieri queues work.

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
:ai show
:ai accept
:ai reject
```

`:ai chat` opens the tracker-native AI Chat view. The view shows local thread
messages, provider/status, selected pattern/track/row context, and a composer.
Typing a prompt and pressing `Enter` submits it through the same reviewable
proposal path as `:ai propose`; `Esc` returns to the tracker and `Ctrl+C`
requests cancellation of the active task.

`:ai provider` reports the configured provider, model, and availability before a
prompt is submitted. `:ai propose` submits the configured local or mock provider
to the application [task runtime](tasks.md) with the current pattern and track as
context. Missing credentials or missing CLI binaries are reported before any task
is queued. The TUI remains responsive while generation and preview validation
run. A successful task stores a pending proposal, appends an assistant message,
and reports the touched cells without mutating the song. `:ai show` repeats the
summary. `:ai accept` applies the proposal through the normal undo transaction
mechanism, so `Ctrl+Z` can revert the generated edit. `:ai reject` clears the
pending proposal without changing the song.

CLI integrations should print or serialize proposals before applying them so
generated changes remain reviewable.
