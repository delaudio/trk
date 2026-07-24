# Plan

This folder holds the project's implementation queue — one file per unit of
work. The queue mirrors the ADR catalogue (`INDEX.md`) but tracks the human
ordering of work, not the ADR catalogue ordering.

## Layout

- `plan/todo/NNNN-<slug>.md` — pending work, ordered by priority. Each file
  names the owning ADR(s), scope, exit criteria, and dependencies.
- `plan/done/<YYYY-MM-DD>-<slug>.md` — shipped work, ordered chronologically.
  The `git mv` from `todo/` to `done/` is the completion event; the file's body
  is amended with a shipped footer naming the HEAD SHA and any artefact id.

## Convention

- A pending item gets a `plan/todo/` file before substantive work starts.
- When work ships, the file is moved to `plan/done/` with a new date prefix and
  a shipped footer.
- A small fix that does not justify a plan file can skip the ceremony. Use
  judgement.
- The status of owning ADRs advances when the work ships: `Accepted` →
  `Implemented`.

## Status semantics on the owning ADRs

| ADR status | Meaning |
|---|---|
| Proposed | Draft; decision authored but not yet approved. |
| Accepted | Decision approved; implementation authorised. Sits in `plan/todo/` when implementation work is needed. |
| Implemented | Shipped per the project's completion event. Sits in `plan/done/` when one existed. |
| Superseded | Replaced by another ADR. |
| Deprecated | Was real; the world moved on; no successor. |

See `CONVENTIONS.md` for the canonical definition.
