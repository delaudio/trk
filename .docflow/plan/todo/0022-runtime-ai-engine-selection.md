# Runtime AI engine selection

Owning ADR: `../../adr/0024-select-ai-proposal-engines-at-runtime.md`

GitHub issue: #313

## Scope

Add supported-engine discovery, runtime selection state, direct CLI adapter
execution, structured JSON proposal parsing, and an AI-workspace engine selector
for the built-in, Claude, Codex, Ollama, and OpenAI engines. Keep proposal
preview, validation, apply, undo, and task execution provider-independent. Do
not install engines, write secrets, persist the runtime choice, or bypass the
existing proposal review flow.

## Exit criteria

1. Discovery covers built-in, CLI, and credential-backed engines and represents
   missing requirements safely (ADR AC1).
2. The AI-workspace `m` selector renders and supports navigation, activation,
   and cancellation (ADR AC2).
3. Selection updates the active provider and the next proposal path without a
   restart (ADR AC3).
4. CLI invocation and JSON cell-diff parsing return validated external
   proposals or actionable errors (ADR AC4).
5. Focused discovery, selection, app routing, rendering, and parser tests plus
   the complete repository verification gate pass (ADR AC5).
6. Norn review with the Codex provider reports no unresolved in-scope findings
   before publication.

## Dependencies

- `../../adr/0001-record-architecture-decisions.md`
- Maintainer approval to execute issue #313 autonomously.
