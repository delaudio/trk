# IAC Driver Session

This example starts trk with the included macOS IAC Driver config and sends MIDI to a DAW.

## 1. List Ports

```bash
trk --list-midi-outputs
```

Expected macOS IAC output:

```text
0: IAC Driver Bus 1
```

## 2. Test One Note

```bash
trk --config config/iac-driver.toml --midi-test-output 0 --midi-test-channel 1 --midi-test-note 60
```

## 3. Run The TUI

```bash
trk --config config/iac-driver.toml --midi-log trk-midi.log
```

Inside trk:

```text
F4          open MIDI settings
Enter       connect selected output
i           edit mode
z x c       add notes
Esc         normal mode
Space       play/stop
```

## 4. Monitor Messages

```bash
tail -f trk-midi.log
```

If the log shows note-on/note-off messages but the DAW is silent, check the DAW input routing, channel filter, monitoring state, and whether an instrument is loaded on the destination track.
