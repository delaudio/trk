# AGENTS.md

This file provides guidance to coding agents working in this repository.

## What this repository is

trk is a MIDI-first terminal music tracker implemented as a Rust
workspace. The codebase contains the core song model, MIDI import/export,
sampler/audio foundations, tracker TUI, and the CLI application. Project
decisions live in `.docflow/` and are intentionally internal builder artefacts.

## Repository structure

- `.docflow/adr/0000-template.md` — canonical ADR template.
- `.docflow/adr/NNNN-<kebab-slug>.md` — one ADR per decision, contiguous
  numbering, no gaps.
- `.docflow/INDEX.md` — table regenerated from every ADR's metadata block.
- `.docflow/CONVENTIONS.md` — authoring rules; read before editing ADRs or
  queue files.
- `.docflow/plan/todo/NNNN-<slug>.md` — pending work, lower numbers run first.
- `.docflow/plan/done/<YYYY-MM-DD>-<slug>.md` — shipped work, chronological.
- `.docflow/_agent/` — single-agent coordination: `ROLES.md`, `WORKLOG.md`,
  `CURRENT_FOCUS.md`, `HANDOFF.md`, and `prompts/`.
- `crates/` — Rust crates for core data structures, MIDI, audio, sampler,
  transforms, interop, TUI, AI helpers, and the application binary.
- `docs/` — user- and developer-facing documentation that is not part of the
  ADR catalogue.
- `fixtures/` — sample projects, MIDI fixtures, XRNS fixtures, and test data.
- `scripts/` — repository verification helpers.

## Required local checks

Run the relevant subset while developing. Before integration, the full gate is:

```sh
cargo fmt --all -- --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
scripts/check-rust-file-sizes.sh --top 12
```

## Hard rules when editing ADRs

These come from `.docflow/CONVENTIONS.md` and override default behaviour:

- One decision per ADR. If a decision splits, supersede the original ADR and
  create new ADRs rather than expanding scope inside a single document.
- Status lifecycle: `Proposed → Accepted → Implemented`, with terminal
  `Superseded` and `Deprecated`.
- ADR section order: metadata, Context, Capability statement, User stories /
  scenarios, Acceptance criteria, Out of scope, Open questions, References,
  Revision History, Approvals.
- Acceptance criteria are testable and numbered.
- ADRs are internal artefacts. ADR numbers, ADR titles, and the existence of
  the ADR catalogue must not appear in user-visible UI copy, public docs,
  support text, API responses, release notes, or customer-visible logs.

## Implementation work

- Start from the ADRs. Identify which ADRs a code change implements or affects
  before changing behaviour.
- If implementation reveals a capability gap or changed decision, update the
  relevant ADR rather than silently diverging.
- Add or update tests for implemented behaviour. Map tests back to ADR
  acceptance criteria where practical.
- Do not leak ADR identifiers into user-visible surfaces. The ADR link belongs
  in commit messages, PR descriptions, internal docs, and optional inline code
  comments only.

## Audit trail and revision discipline

- Substantive ADR changes append a row to the Revision History table.
- Editorial ADR changes may skip a revision row, but the commit should flag
  them as editorial.
- Approvals populate when an ADR is Accepted and update on each later
  substantive revision.
- Regenerate `.docflow/INDEX.md` after any ADR status change or new ADR.

## Single-agent workflow

A single agent owns this repo. The `.docflow/_agent/` directory tracks live
state and history; LOCKS discipline is not in use.

- Update `.docflow/_agent/CURRENT_FOCUS.md` when active work changes.
- Append `.docflow/_agent/WORKLOG.md` on commits that ship docflow-governed
  work.
- Use `.docflow/_agent/HANDOFF.md` as the entry point for a fresh session.

## Plan folder

- A pending item gets a `.docflow/plan/todo/NNNN-<slug>.md` file before
  substantive work starts, naming the owning ADR(s), scope, and exit criteria.
- Completion event: PR merged to `main` with CI green.
- On completion, move the file to `.docflow/plan/done/<YYYY-MM-DD>-<slug>.md`
  with a shipped footer naming the HEAD SHA and any relevant artefact id.
- The owning ADR(s) advance `Accepted → Implemented` on the same commit.
  Regenerate `.docflow/INDEX.md`.

## Git contract

- Commit messages follow Conventional Commits.
- Mandatory `Rationale:` footer on any commit touching an ADR.
- Signed commits are expected when the local environment supports them.
- Do not add `Co-Authored-By` trailers unless the human explicitly asks.
- Cross-references between ADRs use relative paths such as
  `adr/0001-record-architecture-decisions.md`.
- Integration is PR-based. CI must be green before merge. Merge strategy:
  squash. Completion event: PR merged to `main` with CI green.
