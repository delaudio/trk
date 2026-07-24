# Handoff

Entry point for a fresh agent picking up this repo. Read these files in order
before any tool calls:

1. `AGENTS.md` — hard rules. Read in full.
2. `.docflow/CONVENTIONS.md` — authoring rules, ADR status semantics, and plan
   folder convention.
3. `.docflow/plan/README.md` — the queue convention.
4. `.docflow/_agent/CURRENT_FOCUS.md` — short live snapshot of any in-flight
   session: active branch, active queue item, blockers, uncommitted work.
5. `.docflow/INDEX.md` — the ADR catalogue. Look up an ADR's filename and
   dependency chain. Sorted by ADR number, not by work order.
6. Tail of `.docflow/_agent/WORKLOG.md` — confirms what landed.
7. The next queue item at `.docflow/plan/todo/NNNN-*.md` and the ADR(s) it names
   before implementing.

## Stop conditions

Stop and surface the issue rather than guessing if:

- The verify gate fails.
- The queue is empty.
- A `plan/todo/` item references an ADR whose status is not Accepted.
- An ADR's acceptance criteria are ambiguous or untestable as written.
- Two `plan/todo/` items at the same priority both target the same files.
