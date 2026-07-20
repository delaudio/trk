# Composition graph workflow

Composition graphs are reviewable arrangement plans above tracker patterns. The
first supported schema is `salieri.composition-graph.v1`; it names narrative
sections, links each section to an existing 1-based pattern number, records
motifs/evidence, and compiles deterministically into the project sequence.

The graph file validates independently from `.salieri` projects:

```bash
salieri graph validate arrangement.graph.json
```

Compile a graph into a new project file:

```bash
salieri graph compile arrangement.graph.json song.salieri arranged.salieri
```

Compilation is intentionally narrow in this first version: it clears the output
project sequence and appends each referenced pattern for the requested repeat
count. Pattern content, samples, mixer state, notes, and metadata are preserved.
Missing pattern references are hard errors. Clip scenes and Ableton bridge
commands remain future compile targets owned by their separate clip/bridge work.

Example:

```json
{
  "schema": "salieri.composition-graph.v1",
  "title": "Narrative arc",
  "sections": [
    {
      "id": "intro",
      "name": "Intro",
      "pattern": 1,
      "repeats": 2,
      "motifs": ["pulse"],
      "evidence": ["Pattern 01 establishes the pulse"],
      "transition": "build"
    },
    {
      "id": "answer",
      "name": "Answer",
      "pattern": 2,
      "repeats": 1,
      "motifs": ["response"],
      "evidence": ["Pattern 02 answers the intro"]
    }
  ]
}
```

Inside the TUI, graph commands follow the same review/apply boundary as AI
edits:

```text
:graph draft verse then answer
:graph show
:graph reject
:graph apply
```

`:graph draft` creates a deterministic draft from the current pattern list and
the current sequence repeat counts, then stores it as a pending proposal. It
does not mutate the song. `:graph show` previews the pending graph, `:graph
reject` clears it, and `:graph apply` compiles it into the project sequence
inside the normal undo history.
