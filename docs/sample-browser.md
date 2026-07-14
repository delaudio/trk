# Sample Browsing

Salieri includes an in-app sample browser for navigating directories, previewing supported WAV files, and loading the selected file into the sampler view:

```text
:sample browse
:sample browse ~/Samples/Drums
```

The in-app browser shows directories, supported WAV files, unsupported files, metadata, and waveform previews. Press Enter to open a directory or load the highlighted WAV.

Salieri can also open an external sample chooser and load the selected WAV into the sampler view. This is optional: if no chooser is configured, the in-app browser and direct `:sample view PATH` continue to work without Yazi or any other browser installed.

Configure a chooser command in `~/.config/salieri/config.toml`:

```toml
[sample_browser]
start_dir = "~/Samples"
chooser_command = 'YAZI_CONFIG_HOME="$HOME/.config/yazi-readonly" yazi --chooser-file "$SALIERI_CHOOSER_FILE" "$SALIERI_SAMPLE_START_DIR"'
```

The chooser contract is:

- Salieri creates a temporary chooser file and exposes it as `SALIERI_CHOOSER_FILE`.
- Salieri exposes the configured or command-provided start directory as `SALIERI_SAMPLE_START_DIR`.
- The external chooser writes the selected file path into `SALIERI_CHOOSER_FILE`.
- Salieri reads that path and loads it through the existing sampler WAV path.

Inside Salieri:

```text
:sample choose
:sample choose ~/Samples/Drums
```

Yazi remains only one possible chooser. Any command that writes a selected path to `SALIERI_CHOOSER_FILE` can be used.

## Yazi Audition

The optional helper at `scripts/yazi-audition` previews one audio file at a time and supports an explicit stop command:

```sh
scripts/yazi-audition path/to/sample.wav
scripts/yazi-audition --stop
```

Example readonly Yazi opener:

```toml
[opener]
play = [
  { run = '~/dev/current/salieri-tracker/scripts/yazi-audition %s1', desc = "Audition sample", orphan = true, for = "macos" },
]

[open]
prepend_rules = [
  { mime = "audio/*", use = "play" },
  { url = "*.wav", use = "play" },
  { url = "*.aif", use = "play" },
  { url = "*.aiff", use = "play" },
  { url = "*.flac", use = "play" },
  { url = "*.mp3", use = "play" },
  { url = "*.ogg", use = "play" },
  { url = "*.opus", use = "play" },
]
```

Example readonly Yazi keymap for stop:

```toml
[mgr]
prepend_keymap = [
  { on = "<C-x>", run = "shell -- ~/dev/current/salieri-tracker/scripts/yazi-audition --stop", desc = "Stop sample audition" },
]
```

`Ctrl+x` is used because common Yazi defaults already use comma for sort and `Shift+s` for search in many setups.
