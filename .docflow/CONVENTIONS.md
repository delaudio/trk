# Conventions

## Project

Project name: trk.

Artefact root: `.docflow/` — `adr/`, `plan/`, `INDEX.md`, and this file live
under this root; `AGENTS.md` and `CLAUDE.md` stay at the repository root.

Discovery: the `.docflow/` directory at the repository root is the artefact
root. Tools resolve the root there before probing legacy locations.

Assessment depth: `guided` — the depth chosen at bootstrap, matching the
Lachesi setup style.

Language: English throughout docflow artefacts.

## Existing governance and legacy documents

This repo does not use Archgate. The pre-bootstrap ADR at
`docs/adr/0001-plugin-hosting.md` has been migrated into
`adr/0002-plugin-hosting.md`; do not recreate a parallel `docs/adr/` tree.
Developer documentation outside the ADR catalogue remains under `docs/`.

## ADR Files

ADR filenames use `NNNN-kebab-case-slug.md`, zero-padded to 4 digits, with
contiguous numbering and no reserved gaps.

The number is an integer; the four-digit zero-padding is a display convention
only. Tools sort ADRs numerically, not lexically.

Each ADR describes one decision. If a decision splits, supersede the original
ADR and create new ADRs rather than expanding scope inside a single document.

Status lifecycle: `Proposed → Accepted → Implemented`, with terminal
`Superseded` and `Deprecated`.

| Status | Meaning |
|---|---|
| Proposed | Draft. Decision authored but not yet approved. |
| Accepted | Decision approved; implementation authorised. Work item lives in `plan/todo/` when implementation work is needed. |
| Implemented | Shipped per the completion event. Work item lives in `plan/done/` when one existed. The ADR is the authoritative spec the running system matches. |
| Superseded | Replaced by another ADR. The successor is named in `superseded-by:` metadata. |
| Deprecated | Was real; the world moved on; no successor. Capability is not being rebuilt. |

Terminal states are reachable from any prior state.

The first persisted status is `Proposed`. There is no separate `Draft` state
and no `brainstorming/` or `drafts/` folder.

Cross-references link by relative path to `adr/NNNN-*.md`.

## ADR Shapes

This project uses a single ADR shape. ADRs use `adr/0000-template.md` and
contain these sections in order: Context, Capability statement, User stories /
scenarios, Acceptance criteria, Out of scope, Open questions, References,
Revision History, Approvals.

## ADR Privacy

ADRs are internal artefacts. ADR numbers, ADR titles, and the existence of the
ADR catalogue must never appear in any string the product emits to users: UI
copy, API response bodies, error messages, customer-visible log lines, public
documentation, release notes, marketing copy, or support communications.

Allowed references:

- Inline code comments tying a non-obvious choice to its ADR.
- Commit messages and PR descriptions.
- Internal documents: `AGENTS.md`, `INDEX.md`, the `plan/` queue, `_agent/`
  files, and internal runbooks.

Rule of thumb: if a non-builder could ever read the string, the ADR reference
comes out. Refer to the behaviour by its product-level name instead.

## Multi-Agent Rules

A single agent owns this repo. The `_agent/` directory tracks live state and
history; no LOCKS discipline.

## Plan Folder

Pending and shipped work live in `plan/` under the artefact root:

- `plan/todo/NNNN-<slug>.md` — pending work, lower numbers run first. Each file
  names the owning ADR(s), scope, and exit criteria.
- `plan/done/<YYYY-MM-DD>-<slug>.md` — shipped work, chronological. A `git mv`
  from `todo/` to `done/` is the completion event.

The completion event is: PR merged to `main` with CI green.

When a `plan/todo/` item ships, the file moves to `plan/done/` and the owning
ADR(s)' `status:` advances from `Accepted` to `Implemented`. `INDEX.md` is
regenerated to match.

## Concurrency Guardrails

ADR and `plan/todo` numbers are contiguous and assigned at authoring time, so
concurrent branches can pick the same next number. These guardrails keep
numbering collision-free without changing the identity scheme:

- G1 — decide before do. Prefer to merge an ADR and its plan items to `main`
  before implementation work begins.
- G2 — check before merge. Before integrating, sync onto current `main` and run
  `/audit`. If an ADR number or `plan/todo` slot clashes with what landed,
  renumber locally before integrating.
- G3 — gate backstop. Integration is single-threaded; it rejects duplicate
  numbers as the last line of defence.
- G4 — claim before do. Before implementing a queued item, claim it with a
  draft PR referencing the item.

## Verify gate

The local and CI verification gate is:

```sh
cargo fmt --all -- --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
scripts/check-rust-file-sizes.sh --top 12
```

## Git Contract

- Commit messages follow Conventional Commits.
- Mandatory `Rationale:` footer on any commit touching an ADR.
- Signed commits are expected when the local environment supports them.
- ADR revision tags are not required.
- `Co-Authored-By` trailers are not used unless the human explicitly asks.
- Cross-references between ADRs use relative paths.
- Integration is PR-based. CI must be green before merge. Merge strategy:
  squash.
