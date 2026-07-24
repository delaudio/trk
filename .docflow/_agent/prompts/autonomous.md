# Autonomous-completion prompt

You are this project's autonomous agent. Your task: drive the implementation
queue in `.docflow/plan/todo/` to completion, unsupervised, committing per item
with the verify gate green, until the queue is empty or a documented stop
condition fires.

## Step 1 — Orient

Read these files in order, in full, before any tool calls:

1. `AGENTS.md`
2. `.docflow/CONVENTIONS.md`
3. `.docflow/plan/README.md`
4. `.docflow/_agent/CURRENT_FOCUS.md`
5. `.docflow/INDEX.md`
6. Tail of `.docflow/_agent/WORKLOG.md`
7. The queue item file at `.docflow/plan/todo/NNNN-*.md` you are about to work,
   and the ADR(s) it names.

## Step 2 — Pick the next item

List `.docflow/plan/todo/` and pick the lowest-numbered file.

## Step 3 — Implement

Implement against the ADR's numbered acceptance criteria. Add or update tests
that map back to those criteria.

## Step 4 — Verify

Run the project's verify gate:

```sh
cargo fmt --all -- --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
scripts/check-rust-file-sizes.sh --top 12
```

Do not proceed if the gate fails.

## Step 5 — Commit

Use Conventional Commits per `AGENTS.md`. Add a `Rationale:` footer on any
commit touching an ADR.

## Step 6 — Integrate

- Check before merge: sync onto current `main` and run `/audit`. If a new ADR or
  `plan/todo` number clashes with what landed, renumber locally before
  integrating.
- Push the work branch.
- Open a draft PR.
- Wait for CI; do not proceed until it is green.
- Mark ready and merge with squash.
- Confirm the merge landed on `main` before treating the item as shipped.

## Step 7 — Ship the queue item

Once the change is on `main`:

- Move `.docflow/plan/todo/NNNN-<slug>.md` to
  `.docflow/plan/done/<YYYY-MM-DD>-<slug>.md`.
- Amend the moved file with a "Shipped at HEAD `<sha>`" footer and any artefact
  id.
- Advance owning ADRs from `Accepted` to `Implemented`; regenerate
  `.docflow/INDEX.md`.

## Step 8 — Record

- Append a one-line `.docflow/_agent/WORKLOG.md` entry naming the branch, HEAD,
  verify result, and any deferral.
- Update `.docflow/_agent/CURRENT_FOCUS.md`.

## Stop conditions

- Verify gate fails and the cause is not understood.
- Queue empty.
- A queue item references an ADR whose status is not Accepted.
- Acceptance criteria are ambiguous or untestable as written.
