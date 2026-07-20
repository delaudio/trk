# Configuration

Salieri resolves configuration before starting MIDI, audio, or the TUI. The
configuration boundary owns defaults, TOML loading, CLI overrides, validation,
and preference metadata.

## Resolution Order

Settings are applied in this order:

1. Built-in defaults.
2. A user TOML file.
3. Supported CLI overrides such as `--midi-log`.

`--config PATH` selects an explicit file. Without it, Salieri checks
`$XDG_CONFIG_HOME/salieri/config.toml`, then
`$HOME/.config/salieri/config.toml`. A missing file keeps the built-in defaults.

## Example

Every section and field is optional. This example shows the complete preference
surface:

```toml
[keyboard]
vim_navigation = true
edit_step = 1
default_octave = 4

[keymap]
profile = "tracker"

[keymap.normal]
"ctrl+p" = "play pattern"

[keymap.sampler]
b = "sample-browser"

[ai]
provider = "local_deterministic" # local_deterministic, mock, or command
model = "local-deterministic"
required_env = []                # credentials or flags required before use
# command_path = "/usr/local/bin/codex" # required only for future command providers

[ui]
show_line_numbers_hex = false
row_number_format = "decimal" # decimal or hex; show_line_numbers_hex remains a legacy alias
row_number_base = "zero"      # zero or one
pattern_divider_interval = 4  # 0 disables divider rows
pattern_highlight_interval = 16 # 0 disables major highlight rows
show_pattern_top_info = true
follow_playhead = true
display_mode = "adaptive"

[ui.layout]
default = "balanced"      # compact, balanced, or studio
show_tracks = true
show_sequence = true
show_inspector = false
show_track_desk = true
left_width = 28
inspector_width = 36
track_desk_height = 10

[theme]
name = "default"

[audio]
sample_rate = 48000
channels = 2
playback_headroom_db = 0
limiter_mode = "off"          # off, soft, or brickwall
resampling_quality = "balanced" # draft, balanced, or high
send_mode = "disabled"        # disabled, pre_fader, or post_fader

[midi]
default_output = "IAC Driver Bus 1"
default_input = "IAC Driver Bus 1"
log_file = "salieri-midi.log"

[ai]
provider = "local_deterministic"
model = "local-deterministic"
session_file = "~/.config/salieri/ai-session.json"
retention_messages = 200

[sample_browser]
chooser_command = "yazi --chooser-file $SALIERI_CHOOSER_FILE"
start_dir = "~/Samples" # legacy fallback; prefer workspace.sample_library

[project_browser]
start_dir = "~/Music/Salieri" # legacy fallback; prefer workspace.project_library
recent_file = "~/.config/salieri/recent-projects.json"

[workspace]
project_library = "~/Music/Salieri/Projects"
sample_library = "~/Music/Salieri/Samples"
recent_project_limit = 12

[history]
undo_limit = 100
```

`keyboard.edit_step` is the tracker step-jump value used after manual note and
cell-value entry; `0` keeps the cursor on the edited row.

`[ai]` defaults to the local deterministic provider, which never contacts a
network service. `mock` is available for tests and scripted dry runs. `command`
is reserved for future CLI adapters; when selected, Salieri checks
`command_path` and every `required_env` entry before queuing an AI task and
reports missing binaries or credentials as normal diagnostics.
`session_file` enables local JSON chat history autosave/load, and
`retention_messages` bounds the saved local thread length.

Keymap bindings are grouped by application mode and override built-in shortcuts
for the same key. Unmapped keys continue to use the built-in defaults. See
[Configurable Keymaps](keymaps.md) for the complete layer and key syntax.
Theme names and display modes are validated and exposed as metadata for commands
and presentation code. Applying visual themes remains dedicated rendering work.

## Workspace Libraries

`[workspace] project_library` is the default project folder. When no project is
open, `:write` saves `untitled.salieri` there. `:saveas NAME` resolves bare names
to that folder and adds `.salieri` when no extension is provided. Salieri creates
the project library lazily during those save actions; it does not create folders
just because configuration was loaded.

`[workspace] sample_library` is the default sample folder for `:sample browse`
and `:sample choose` when no directory is passed.

The older `sample_browser.start_dir` and `project_browser.start_dir` fields still
work as browser fallbacks for existing configs. If both a workspace library and a
legacy browser start directory are set, the workspace library wins.

Configured paths expand `~`, `$VAR`, and `${VAR}`. Relative paths in the config
file resolve against the directory containing that config file.

## Validation

Unknown fields and invalid TOML are rejected with the selected file path. Semantic
validation reports all independent problems found in one pass, including:

- edit step from 0 through 64;
- non-empty AI model, command path when configured, and required environment
  variable names;
- octave from 0 through 9;
- non-empty keymap profile and theme name;
- valid key names, typed keymap commands, and no normalized conflicts per mode;
- audio sample rate from 8000 through 384000 Hz;
- audio channels from 1 through 8;
- recent project limit from 1 through 100;
- undo history limit from 1 through 10000 transactions;
- layout `left_width` from 18 through 56 cells;
- layout `inspector_width` from 24 through 64 cells;
- layout `track_desk_height` from 6 through 18 rows;
- non-empty sample chooser command when configured.

Inside the TUI, `:config` reports the resolved source, keymap profile, theme,
display mode, and AI provider through the normal status notification. `:ai
provider` reports the configured AI provider, model, and availability before
submitting a prompt.

## TUI Layout

Tracker layout preferences are user configuration, not portable project data.
Use `[ui.layout]` to choose the startup layout and representative panel sizes.
At runtime, `:layout compact`, `:layout balanced`, and `:layout studio` switch
named presets. `:layout show PANEL`, `:layout hide PANEL`, and
`:layout toggle PANEL` manage representative panels (`tracks`, `sequence`,
`inspector`, `track-desk`). `:layout resize PANEL +/-N` adjusts the stored
runtime size for the left stack, inspector, or track desk.
