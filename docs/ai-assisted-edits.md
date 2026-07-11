# AI-Assisted Edits

AI-assisted composition is post-MVP and optional. The initial boundary is the `salieri-ai` crate, which models requests, reviewable proposals, and explicit application of edits to a `salieri-core::Song`.

The crate does not contact network services. External providers can be added later behind an explicit provider implementation, but project data must not leave the machine unless the user invokes that provider intentionally.

Current foundation:

- `AiPatternRequest` describes a prompt-scoped generation request;
- `AiProposalProvider` isolates proposal generation from the core editor;
- `AiProposal` stores a summary plus concrete cell edits for review;
- `preview_proposal` validates touched cells without mutating the song;
- `apply_proposal` mutates only after explicit approval and returns the same touched-cell preview.

In-app integration should wrap `apply_proposal` with the same undo snapshot mechanism used by manual edits. CLI integrations should print or serialize proposals before applying them so generated changes remain reviewable.
