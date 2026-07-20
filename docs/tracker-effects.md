# Tracker effect commands

Salieri tracks per-step FX commands through a catalog instead of treating every
unknown code as an implicit no-op.

## Supported playback commands

| Code | Name | Range | Domain | Behavior |
| --- | --- | --- | --- | --- |
| `D` | Delay | `00`-`FF` | Playback timing | Offsets the note/sample trigger within the row. |
| `R` | Retrigger | `01`-`10` | Playback timing | Emits deterministic repeated note/sample triggers within the row. |

FX slots are evaluated left-to-right: FX1 first, then FX2. Supported commands
with different semantics can therefore coexist, for example `D80` in FX1 and
`R04` in FX2.

## Deferred command families

The command catalog reserves Tracker Mini-style families without assigning
runtime behavior before deterministic tests exist for that behavior.

| Code | Name | Domain | Reserved semantics |
| --- | --- | --- | --- |
| `V` | Volume | Sample | Voice gain and random volume. |
| `P` | Pan | Sample | Voice pan and stereo position. |
| `O` | Sample position | Sample | Sample start offset, reverse, and slice position. |
| `C` | Note cut/gate | Sample | Gate length, note cut, and chord output. |
| `U` | Slide up | Sample | Pitch slide up, tuning, and microtune. |
| `N` | Slide down | Sample | Pitch slide down, tuning, and microtune. |
| `T` | Tempo | Project | Project tempo changes. |
| `W` | Swing | Project | Project swing changes. |
| `M` | Micro move | Playback timing | Micro-timing movement. |
| `G` | Glide | Sample | Sample pitch glide. |
| `Q` | Chance | Playback timing | Conditional note/sample playback. |
| `L` | Roll | Playback timing | Roll and deterministic LFO-rate behavior. |
| `A` | Arp/chord | Sample | Arpeggio, chord output, and chord shape. |
| `X` | Random | Playback timing | Random note, random instrument, random FX, and random volume. |
| `B` | Bit depth | Sample | Sample-local bit-depth and lo-fi behavior. |
| `F` | Filter | Sample | Sample-local filter behavior. |
| `S` | Send/slice | Sample | Track send level and sample slice behavior. |
| `H` | Drive | Sample | Sample-local overdrive and distortion. |
| `I` | MIDI CC | MIDI | MIDI continuous-controller output. |
| `K` | Program change | MIDI | MIDI program-change output. |
| `Y` | Aftertouch | MIDI | MIDI channel and polyphonic aftertouch output. |

Deferred commands are not silently treated as playback behavior. The command
parser rejects them for interactive editing, and imported or manually persisted
deferred/unknown commands are exposed through tracker-command diagnostics.
