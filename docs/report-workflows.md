# Report, Critique, and Revision Workflows

Salieri can generate deterministic local markdown reports from `.salieri`
projects:

```bash
salieri report project song.salieri reports/project.md
salieri report critique song.salieri reports/critique.md
salieri report project song.salieri
```

The project report summarizes tempo, tracks, patterns, sequence length, note
density, samples, instruments, annotations, and per-track note counts.

The critique report assigns a deterministic score and lists strengths, issues,
suggested revisions, and follow-up commands. It is intentionally local and does
not call an AI provider.

Inside the TUI:

```text
:report project
:report critique
:report project save reports/project.md
:report critique workspace /path/to/workspace
:revise add a sparse counter melody
```

`:report project` and `:report critique` summarize the report in the status line
and append the full markdown to the AI thread. `save PATH` writes a report file
directly. `workspace ROOT` writes into the workspace manifest's `reports`
directory, creating it when needed.

`:revise PROMPT` builds a revision prompt from the current critique report and
submits it through the configured AI proposal provider. The song is not mutated
when the revision is generated. Review it with `:ai show`, apply it with
`:ai accept`, or discard it with `:ai reject`; accepted revisions use the normal
undo history.
