# Sampler parity roadmap

This matrix tracks Salieri's sampler against the practical workflows covered by
Ableton Simpler and Renoise Sampler. It is intentionally capability-based rather
than UI-compatible: Salieri keeps a tracker-native model and does not attempt to
load Ableton/Renoise presets.

Status meanings:

- **Implemented**: persisted in `.salieri`, exposed through current commands or
  view state, and covered by deterministic core/audio behavior where applicable.
- **Partial**: model or UI exists, but important runtime/editor behavior is not
  complete.
- **Planned**: valid sampler direction after the descriptor/parameter model is
  stable.
- **Deferred**: useful later, but sequenced behind more basic sampler/runtime
  work.
- **Out of scope**: intentionally excluded from this sampler track.

| Capability | Status | Salieri mapping |
| --- | --- | --- |
| Sample-backed instruments/presets | Implemented | `.salieri` stores sample references, sample-backed instruments, track assignments, root note, gain, and playback settings. |
| One-shot playback | Implemented | `sample.playback.mode = oneShot`, rendered by realtime sampler and offline export. |
| Sample start/end | Implemented | `sample.playback.startFrame` and `sample.playback.endFrame`, edited by `:sample start` / `:sample end`, used by realtime/offline rendering. |
| Gain | Implemented | `sample.gain`, automatable through existing sample-gain automation and parameter-lock commands. |
| Amplitude ADSR | Implemented | `sample.envelope.attackS`, `decayS`, `sustain`, `releaseS`, edited by `:sample envelope`, persisted, shown in Sampler View, and applied by realtime/offline rendering. |
| Root note | Partial | `sample.rootNote` is persisted with sample references and included in the stable descriptor catalog; dedicated editing UI remains planned. |
| Forward loop metadata | Partial | `sample.playback.loopStartFrame` / `loopEndFrame` and loop mode are persisted and displayed; sustained loop rendering remains planned. |
| Reverse playback | Planned | Requires direction-aware sampler cursor and reverse-safe interpolation. |
| Backward and ping-pong loops | Planned | Requires loop direction state and click-safe boundary handling. |
| Loop crossfade | Planned | Requires loop-rendering support first, then crossfade window descriptors. |
| Interpolation quality | Planned | Current rendering uses deterministic interpolation; selectable quality tiers need descriptors and a runtime switch. |
| Transpose/fine tune/pitch tracking | Planned | Event pitch ratio exists; persistent sample-level pitch controls need model fields, descriptors, and commands. |
| Mono/polyphony/legato/glide/choke/voice limits | Planned | Requires explicit voice allocation policy in realtime and offline sampler paths. |
| Multimode sampler filter | Planned | Native filter DSP exists at the device level; sampler-local filter state should be added only after instrument parameters are stable. |
| Filter envelopes/key tracking/drive | Planned | Depends on sampler-local filter and modulation target model. |
| LFO and modulation targets | Planned | Native modulation effects exist; sampler modulation should wait for a reusable instrument modulation model. |
| Modulation matrix | Deferred | Sequenced after descriptors, parameter locks, sampler-local targets, and modulation source semantics are stable. |
| Slicing / beat slicing | Deferred | Requires slice table persistence, per-slice playback state, and tracker mapping decisions. |
| Per-slice settings | Deferred | Depends on slicing and descriptor addressing for nested slice targets. |
| Multisample keyzones / velocity zones | Deferred | Requires instrument zone persistence and voice selection rules. |
| Granular / wavetable sampling | Out of scope | Explicitly outside this sampler parity track until the conventional sampler and modulation APIs are stable. |
| Ableton/Renoise preset compatibility | Out of scope | The project targets workflow parity, not foreign preset formats or UI compatibility. |
| Plugin-hosted instruments/effects | Out of scope | Deferred by ADR 0001; no VST/AU/CLAP host types belong in the sampler model. |

## Descriptor coverage

Implemented and partial sampler controls now have stable built-in descriptors:

| ID | Type | Range/default | Automatable |
| --- | --- | --- | --- |
| `sample.gain` | `PlainFloat` | `0..=2`, default `1` | yes |
| `sample.rootNote` | `Note` | `0..=127`, default `60` | no |
| `sample.playback.mode` | `Enum` | `oneShot`, `loop`; default `oneShot` | no |
| `sample.playback.startFrame` | `Integer` | `0..=2147483647`, default `0` | no |
| `sample.playback.endFrame` | `Integer` | `0..=2147483647`, default `0` | no |
| `sample.playback.loopStartFrame` | `Integer` | `0..=2147483647`, default `0` | no |
| `sample.playback.loopEndFrame` | `Integer` | `0..=2147483647`, default `0` | no |
| `sample.envelope.attackS` | `Seconds` | `0..=60`, default `0` | no |
| `sample.envelope.decayS` | `Seconds` | `0..=60`, default `0` | no |
| `sample.envelope.sustain` | `Percentage` | `0..=1`, default `1` | no |
| `sample.envelope.releaseS` | `Seconds` | `0..=60`, default `0` | no |

Only `sample.gain` is currently automatable because it is the only sampler
parameter-lock target applied during playback. Other controls are descriptorized
for stable serialization, formatting, validation, and future parameter-lock
addressing once the runtime applies those locks deterministically.
