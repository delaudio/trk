# Guidance Artifacts

Salieri can load local guidance JSON files for future generation and arrangement workflows. These files are optional inputs: opening and editing a `.salieri` project never depends on them, and the local deterministic AI provider continues to work without them.

Guidance artifacts are explicit request context, not hidden global state. Commands that use them should receive paths from the user or from a project feature that stores visible references.

## Research Dossier

A research dossier captures style notes, observations, and guardrails that can guide analysis or generation.

```json
{
  "schemaVersion": 1,
  "title": "Detroit electro references",
  "sources": ["private-notes.md"],
  "keywords": ["syncopated bass", "machine funk"],
  "observations": ["Short envelopes leave room for dense hats"],
  "guardrails": ["Do not imitate a named artist directly"]
}
```

Fields:

- `schemaVersion`: required integer, currently `1`.
- `title`: required non-empty string.
- `sources`: optional string list for local source labels or paths.
- `keywords`: optional prompt-safe style tags.
- `observations`: optional prompt-safe musical notes.
- `guardrails`: optional prompt-safe constraints.

At least one of `keywords`, `observations`, or `guardrails` must contain a non-empty item.

## Operational Palette

An operational palette describes track roles, sound sources, arrangement functions, and guardrails for a session.

```json
{
  "schemaVersion": 1,
  "title": "Live clip palette",
  "trackRoles": [
    {
      "role": "bass",
      "description": "Short mono phrases that answer drums"
    }
  ],
  "soundSources": ["external MIDI synth"],
  "arrangementFunctions": ["intro scene without kick"],
  "guardrails": ["Keep edits reversible"]
}
```

Fields:

- `schemaVersion`: required integer, currently `1`.
- `title`: required non-empty string.
- `trackRoles`: optional list of role/description pairs.
- `soundSources`: optional prompt-safe source descriptions.
- `arrangementFunctions`: optional prompt-safe scene or clip functions.
- `guardrails`: optional prompt-safe constraints.

At least one role, sound source, arrangement function, or guardrail must contain a non-empty item.

## CLI

Validate and print a prompt-safe summary:

```bash
salieri guidance dossier research.json
salieri guidance palette palette.json
```

The CLI only reads local files. It does not start networking, call external providers, or mutate projects.
