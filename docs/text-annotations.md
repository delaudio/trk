# Text Annotations

Text annotations are optional project notes, lyrics, and cue markers stored in
`.trk` project files. They do not affect playback.

Command-mode usage:

```text
:note project Sketch goals and references
:note pattern Verse starts here
:note pattern 12 Fill enters on this row
:note lyric pattern 16 First lyric line
:note cue sequence 0 Intro cue
:note list
:note report
:note clear 1
```

Scopes:

- `project` stores a project-level note.
- `pattern` stores a note on the active pattern. If no row is passed, the
  current cursor row is used.
- `sequence POSITION` stores a cue marker on a sequence slot.

Kinds:

- `note` / `notes` create ordinary notes.
- `lyric` / `lyrics` create lyric lines.
- `cue` / `cues` create cue markers.

`:note list` shows the current annotation inventory. `:note report` formats the
same annotations as a compact text report so local review, critique, and
revision workflows can include them when present.
