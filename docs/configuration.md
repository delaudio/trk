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

[ui]
show_line_numbers_hex = false
follow_playhead = true
display_mode = "adaptive"

[theme]
name = "default"

[audio]
sample_rate = 48000
channels = 2

[midi]
default_output = "IAC Driver Bus 1"
default_input = "IAC Driver Bus 1"
log_file = "salieri-midi.log"

[sample_browser]
chooser_command = "yazi --chooser-file $SALIERI_CHOOSER_FILE"
start_dir = "~/Samples"

[project_browser]
start_dir = "~/Music/Salieri"
recent_file = "~/.config/salieri/recent-projects.json"

[workspace]
recent_project_limit = 12
```

Keymap bindings are grouped by application mode and override built-in shortcuts
for the same key. Unmapped keys continue to use the built-in defaults. See
[Configurable Keymaps](keymaps.md) for the complete layer and key syntax.
Theme names and display modes are validated and exposed as metadata for commands
and presentation code. Applying visual themes remains dedicated rendering work.

## Validation

Unknown fields and invalid TOML are rejected with the selected file path. Semantic
validation reports all independent problems found in one pass, including:

- edit step from 1 through 64;
- octave from 0 through 9;
- non-empty keymap profile and theme name;
- valid key names, typed keymap commands, and no normalized conflicts per mode;
- audio sample rate from 8000 through 384000 Hz;
- audio channels from 1 through 8;
- recent project limit from 1 through 100;
- non-empty sample chooser command when configured.

Inside the TUI, `:config` reports the resolved source, keymap profile, theme, and
display mode through the normal status notification.
