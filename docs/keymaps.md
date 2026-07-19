# Configurable Keymaps

Salieri resolves user key bindings when configuration loads. Each binding belongs
to one application mode and maps one key press to a typed command. Configured
bindings take precedence in that mode; every other key keeps its built-in
behavior.

## Layers

The available TOML sections are:

- `keymap.normal`
- `keymap.edit`
- `keymap.command`
- `keymap.help`
- `keymap.dialog`
- `keymap.midi_settings`
- `keymap.sequence`
- `keymap.tracks`
- `keymap.patterns`
- `keymap.sampler`
- `keymap.sample_browser`
- `keymap.project_browser`
- `keymap.ai`
- `keymap.clip`

The `ai` and `clip` layers reserve configuration names for their future views.
The older inline `keymap.bindings` map remains supported as a normal-mode layer.

```toml
[keymap]
profile = "studio"

[keymap.normal]
q = "bpm 150"
"ctrl+p" = "play pattern"

[keymap.edit]
"shift+q" = "stop"

[keymap.sample_browser]
r = "sample-browser"
```

Commands use the same typed syntax as command mode, with or without the leading
colon. Invalid or empty commands stop startup with a configuration diagnostic.
Opening the in-app help shows a short summary of the active custom binding
metadata in the notification bar.

## Key Syntax

A binding contains one key plus optional `ctrl`, `alt`, `shift`, or `super`
modifiers joined by `+`. Modifier aliases such as `control`, `option`, and `cmd`
are normalized before conflict detection. Uppercase letters imply `shift`.

Named keys include `space`, `plus`, `esc`, `enter`, `backspace`, `delete`,
`insert`, `tab`, `backtab`, arrow keys, `home`, `end`, `pageup`, `pagedown`, and
`f1` through `f24`. Other single characters are accepted directly.

Only single-key bindings are supported. Multi-key sequences are deliberately
deferred until a concrete workflow justifies chord timeout and prefix semantics.
Two spellings that normalize to the same key in one mode are rejected with a
diagnostic that names the conflicting fields.
