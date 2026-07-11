# Plugin Inventory

Plugin inventory JSON records local desktop plugin metadata for planning, presets, and future module choices. It is not a plugin host: Salieri does not load plugin binaries, instantiate SDK objects, run realtime DSP, or make tracker editing depend on scanned plugins.

Generate an inventory from platform-default folders:

```bash
salieri plugins inventory plugins.json
```

Include full paths only when explicitly needed:

```bash
salieri plugins inventory plugins.json --include-paths
```

Scan explicit roots:

```bash
salieri plugins inventory plugins.json --root vst3=/Library/Audio/Plug-Ins/VST3 --root clap=/home/me/.clap
```

Supported root formats are `au`, `audio-unit`, `component`, `vst`, `vst3`, and `clap`.

## Schema

Schema version `1`:

```json
{
  "schemaVersion": 1,
  "promptSafe": true,
  "scannedRoots": [
    {
      "format": "vst3",
      "pathHint": "VST3"
    }
  ],
  "entries": [
    {
      "id": "plugin_000_12345678",
      "name": "Example Juno",
      "format": "vst3",
      "kind": "instrument",
      "roleSuitability": ["harmony"],
      "vendorHint": "ExampleVendor",
      "pathHint": "ExampleVendor/Example Juno.vst3",
      "tags": ["emulation", "analog-poly"],
      "metadata": ["format:vst3", "kind:instrument", "tag:emulation"]
    }
  ],
  "failures": [
    {
      "format": "clap",
      "pathHint": "CLAP",
      "message": "No such file or directory"
    }
  ]
}
```

Fields:

- `schemaVersion`: required integer, currently `1`.
- `promptSafe`: true when full paths are hidden.
- `scannedRoots`: the roots attempted by the scanner.
- `entries`: detected plugin files or bundles.
- `failures`: per-root or per-directory failures; one failure does not abort the full inventory.

Entries include:

- `name`: prompt-safe display name derived from the plugin file or bundle.
- `format`: `audio-unit`, `vst`, `vst3`, or `clap`.
- `kind`: `instrument`, `effect`, `midi-effect`, or `unknown`.
- `roleSuitability`: coarse planning roles such as `drums`, `bass`, `lead`, `harmony`, `fx`, `mix`, or `utility`.
- `vendorHint`: directory-derived hint, when available.
- `pathHint`: relative prompt-safe hint by default; full path only with `--include-paths`.
- `tags`: optional enrichment tags for well-known historical or emulation families.
- `metadata`: compact prompt-safe labels suitable for later generation prompts.

The scanner only inspects directory entries and filenames. Classification and enrichment are heuristic metadata, not proof that a plugin can be loaded.
