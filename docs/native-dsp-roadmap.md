# Native DSP Roadmap

This document is the maintained parity matrix and implementation roadmap for
Salieri native effects. It starts from the current gain/pan DSP foundation and
keeps direct VST, AU, and CLAP hosting out of scope per
[ADR 0001](adr/0001-plugin-hosting.md).

Native DSP means Salieri-owned devices represented by stable project data,
processed by `salieri-audio`, and described through
`salieri-core::ParameterDescriptor`. The same descriptor/value model must drive
project validation, TUI inspection/editing, per-step parameter locks, realtime
playback, and offline export.

## Scope Boundaries

| Surface | Owner | Native DSP relationship | Status |
| --- | --- | --- | --- |
| Track insert effects | Mixer track `effects` chains | Main implementation path for utility, filter, delay, reverb, drive, modulation, and dynamics devices | gain/pan implemented; broader suite planned |
| Send effects and buses | Mixer `sends` plus future routing graph | Delay/reverb send buses route audio in realtime/offline paths before the master chain | implemented foundation by #84 |
| Master effects | Mixer `masterEffects` chain | Same native devices as track inserts, rendered after track mixing | gain/pan implemented |
| Sampler/instrument-local processing | Sample-backed instruments and future instrument modules | Filters, envelopes, LFOs, keyzones, and sample-local modulation belong to sampler/instrument scope when they change voice behavior before mixer inserts | coordinated with #121 |
| Per-step tracker commands | Pattern cell command/effect columns and automation lanes | Row commands can control, trigger, or lock device parameters, but tracker commands such as retrigger and note delay are not insert devices | coordinated with #85 and #123 |

## Implementation Status Legend

- **Implemented**: persisted, validated, processed by realtime and offline paths,
  and covered by tests.
- **Partial**: some model or UI exists, but routing, automation, processing, or
  tests are incomplete.
- **Planned**: explicit Salieri-native scope exists and is queued.
- **Deferred**: valid future direction, but blocked by prerequisites.
- **Out of scope**: intentionally not a native DSP device.

## Renoise-Parity Matrix

| Renoise DSP family | Salieri category | Current Salieri status | Planned Salieri coverage | Notes |
| --- | --- | --- | --- | --- |
| Gain, gainer, stereo expander, DC/utility, channel tools | Track insert, master, utility | Implemented | #125 native utility audio devices | Native gain, pan, balance, stereo width, and phase invert cover the initial utility-device suite. Mono/channel swap and DC blocking remain future extensions if justified. |
| EQ and filters | Track insert, sampler-local | Implemented | #126 native multimode filter; parametric EQ deferred | Multimode LP/HP/BP/notch uses a stable state-variable filter. Sampler-local modulation/key tracking remains coordinated with #121. |
| Delay, multitap delay, repeater | Track insert, send, master | Implemented | #127 native delay | Stereo delay covers linked/free times, sync quantization, feedback filtering, ping-pong routing, wet/dry mix, and bounded delay memory. |
| Reverb | Track insert, send, master | Implemented | #128 native reverb | Deterministic bounded Schroeder-style reverb; convolution is deferred. |
| Distortion, cabinet, lo-fi, bit reduction | Track insert, sampler-local | Implemented | #129 native drive and degradation effects | Drive covers overdrive, saturation, hard clip, and soft clip; Bitcrusher covers bit-depth and sample-rate hold reduction. Cabinet/convolution deferred. |
| Chorus, flanger, phaser, tremolo, ring modulation, autopan | Track insert, master | Partial | #130 native modulation effects | Chorus, flanger, and phaser are implemented with shared deterministic modulation state; tremolo, ring modulation, and autopan remain follow-up scope. |
| Compressor, gate, limiter, maximizer | Track insert, master | Planned | #131 native dynamics effects | Sidechain/key input is deferred until send/routing foundations are real. |
| Meta devices, LFO device, hydra, key/velocity trackers | Automation/modulation system | Deferred | Follow-up after #123 and #137 | These control parameters rather than process audio directly. Do not model them as ordinary audio insert devices. |
| Send device and routing utilities | Mixer routing | Partial | #84 foundation | Audio sends now have routing and deterministic summing rules; expanded send-specific utilities and UI remain follow-up scope. |
| Native instrument devices | Instrument/module layer | Partial | #116 boundary exists; concrete instruments deferred | Instruments use the same module/state contract but are not part of this effect roadmap except where sampler-local processing overlaps #121. |
| Reviewed C/C++ DSP wrappers | Native module boundary | Implemented foundation | #115 C/C++ DSP boundary | Optional feature-gated wrappers may adapt vendored, license-reviewed algorithms into Salieri-owned modules. They must expose plain descriptors/state and stay separate from arbitrary binary hosting. |
| WebAssembly DSP modules | Export/runtime boundary | Evaluated | #117 WebAssembly DSP evaluation | Recommended first for browser/Web Audio export. Desktop sandboxed realtime execution remains deferred until runtime overhead and callback safety are proven. |
| Faust-generated modules | Native module source format | Evaluated | #118 Faust DSP evaluation | Recommend C++ generation into the #115 wrapper boundary first; WebAssembly is reserved for web export after native behavior is proven. |
| Tracker note/effect commands: delay, retrigger, arpeggio, slides, sample offset | Pattern command library | Partial | #85 expanded per-step FX commands | These are row playback semantics, not DSP devices. They may also write parameter locks once #123 exists. |
| Third-party plugins | Plugin host boundary | Deferred | Future ADR only | Direct VST/AU/CLAP hosting remains out of scope for this roadmap. |

## Baseline Device Catalog

Every native device must have:

- stable device ID;
- category and allowed placements;
- stable parameter IDs;
- display name, value type, range, default, step, and unit;
- flags for logarithmic, bipolar, stepped, automatable, modulatable, advanced,
  and read-only behavior where relevant;
- bypass behavior;
- wet/dry, latency, tail, and offline-render behavior;
- implementation/parity status and issue owner.

Device IDs use `native.effect.<name>`. Parameter IDs use
`native.<device>.<parameter>`. Values are plain serializable
`ParameterValue` data; no audio backend, UI, plugin SDK, or project-loader types
belong in descriptors.

### Implemented Foundation

| Device | ID | Category | Placements | Status | Latency | Tail | Offline behavior |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Gain | `native.effect.gain` | Utility | track insert, master | Implemented | 0 frames | none | deterministic sample scaling |
| Pan | `native.effect.pan` | Utility | track insert, master | Implemented | 0 frames | none | deterministic stereo balance |
| Balance | `native.effect.balance` | Utility | track insert, master | Implemented | 0 frames | none | deterministic stereo balance |
| Stereo Width | `native.effect.width` | Utility | track insert, master | Implemented | 0 frames | none | deterministic mid/side width |
| Phase Invert | `native.effect.phase` | Utility | track insert, master | Implemented | 0 frames | none | deterministic channel polarity inversion |

| Parameter | Device | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `native.gain.gain` | Gain | `PlainFloat` | `0.0..=2.0` | `1.0` | `0.001` | gain | automatable |
| `native.pan.pan` | Pan | `BipolarFloat` | `-1.0..=1.0` | `0.0` | `0.001` | pan | automatable, bipolar |
| `native.balance.balance` | Balance | `BipolarFloat` | `-1.0..=1.0` | `0.0` | `0.001` | pan | automatable, bipolar |
| `native.width.width` | Stereo Width | `Percentage` | `0.0..=2.0` | `1.0` | `0.001` | percent | automatable |
| `native.phase.invertLeft` | Phase Invert | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |
| `native.phase.invertRight` | Phase Invert | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |

### #125 Native Utility Audio Devices

Purpose: finish small building blocks that are useful on tracks and master before
more expensive effects land.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Stereo Width | `native.effect.width` | track insert, master | Implemented | passthrough | no | 0 | none |
| Phase Invert | `native.effect.phase` | track insert, master | Implemented | passthrough | no | 0 | none |
| Mono | `native.effect.mono` | track insert, master | Planned | passthrough | no | 0 | none |
| Balance | `native.effect.balance` | track insert, master | Implemented | passthrough | no | 0 | none |

| Parameter | Device | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `native.width.width` | Stereo Width | `Percentage` | `0.0..=2.0` | `1.0` | `0.001` | percent | automatable |
| `native.phase.invertLeft` | Phase Invert | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |
| `native.phase.invertRight` | Phase Invert | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |
| `native.mono.mode` | Mono | `Enum` | `sum`, `left`, `right` | `sum` | stepped | none | automatable, stepped |
| `native.balance.balance` | Balance | `BipolarFloat` | `-1.0..=1.0` | `0.0` | `0.001` | pan | automatable, bipolar |

### Utility Processing Semantics

- Utility devices perform no implicit output clipping. Samples remain `f32`
  render values after gain, balance, width, and phase processing; later metering,
  export encoding, or future limiter devices own clipping policy.
- Invalid non-finite or out-of-range parameters are rejected before processing.
  Valid silent input remains silent, and utility devices do not allocate, lock,
  log, or touch the filesystem in realtime processing.
- Denormal handling is currently by bounded arithmetic only: the utility suite
  has no feedback state, tails, filters, or accumulators that can generate
  persistent subnormal feedback. Future stateful devices must define explicit
  denormal mitigation in their issue scope.

### #126 Native Multimode Filter

Purpose: provide the first musical tone-shaping effect and a reusable filter
kernel for later sampler-local processing.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Multimode Filter | `native.effect.filter` | track insert, master, sampler-local variant | Implemented | passthrough | yes | 0 | none |

| Parameter | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| `native.filter.mode` | `Enum` | `lowPass`, `highPass`, `bandPass`, `notch` | `lowPass` | stepped | choice | automatable, stepped |
| `native.filter.cutoffHz` | `FrequencyHertz` | `20.0..=24000.0`, clamped to 45% of sample rate at runtime | `12000.0` | `0.1` | hertz | automatable, logarithmic |
| `native.filter.resonance` | `NormalizedFloat` | `0.0..=1.0` | `0.25` | `0.001` | normalized | automatable |
| `native.filter.driveDb` | `Decibels` | `0.0..=24.0` | `0.0` | `0.1` | decibels | automatable |
| `native.filter.keyTrack` | `Percentage` | `-1.0..=1.0` | `0.0` | `0.001` | percent | automatable, bipolar |
| `native.filter.envAmount` | `Percentage` | `-1.0..=1.0` | `0.0` | `0.001` | percent | automatable, bipolar |
| `native.filter.mix` | `Percentage` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |

Implementation notes:

- The filter is a topology-preserving state-variable filter with LP, HP, BP,
  and notch outputs. Resonance maps to a bounded damping range to stay stable at
  high cutoff and high resonance.
- Drive is applied before the filter with soft clipping. `mix` performs dry/wet
  blending after the selected filter output.
- Realtime module parameter changes use one-pole smoothing for numeric
  parameters; mode changes are stepped.
- Track/master processing is implemented in the native DSP graph. `keyTrack`
  and `envAmount` are serialized/described now, but remain neutral until #121
  provides sampler-local note/envelope modulation sources.

### #127 Native Delay

Purpose: provide tempo-aware echo for insert and later send workflows.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Stereo Delay | `native.effect.delay` | track insert, send, master | Implemented | input passthrough; delay line muted | yes | 0 | yes |

| Parameter | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| `native.delay.sync` | `Bool` | `false`, `true` | `true` | stepped | choice | automatable, stepped |
| `native.delay.timeLeftMs` | `PlainFloat` | `1.0..=4000.0` | `500.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.delay.timeRightMs` | `PlainFloat` | `1.0..=4000.0` | `500.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.delay.linkTimes` | `Bool` | `false`, `true` | `true` | stepped | choice | automatable, stepped |
| `native.delay.feedback` | `Percentage` | `0.0..=0.95` | `0.35` | `0.001` | percent | automatable |
| `native.delay.pingPong` | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |
| `native.delay.filterLowCutHz` | `FrequencyHertz` | `20.0..=20000.0` | `20.0` | `0.1` | hertz | automatable, logarithmic |
| `native.delay.filterHighCutHz` | `FrequencyHertz` | `20.0..=20000.0` | `20000.0` | `0.1` | hertz | automatable, logarithmic |
| `native.delay.modRateHz` | `FrequencyHertz` | `0.0..=20.0` | `0.0` | `0.01` | hertz | automatable |
| `native.delay.modDepth` | `Percentage` | `0.0..=1.0` | `0.0` | `0.001` | percent | automatable |
| `native.delay.mix` | `Percentage` | `0.0..=1.0` | `0.25` | `0.001` | percent | automatable |
| `native.delay.outputDb` | `Decibels` | `-60.0..=12.0` | `0.0` | `0.1` | decibels | automatable |

Implementation notes:

- The delay uses bounded per-device delay lines sized to four seconds at the
  active sample rate. Processing is deterministic for realtime and offline
  renders, with no locks or filesystem access in the frame loop.
- Free mode uses millisecond delay times directly. Sync mode quantizes the
  stored millisecond values to a fixed set of musical divisions based on the
  current 120 BPM transport baseline until the DSP graph carries tempo events.
- Feedback is capped at 0.95 and runs through first-order low/high cut filters
  before being written back into the delay line. Bypass mutes the delay line and
  passes input through unchanged.
- Ping-pong swaps the feedback source between left and right delay lines.
  `linkTimes` makes the right delay follow the left delay.

### #128 Native Reverb

Purpose: provide a bounded deterministic room/plate-style space effect.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Stereo Reverb | `native.effect.reverb` | track insert, send, master | Implemented | input passthrough; reverb tank muted | yes | 0 | yes |

| Parameter | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| `native.reverb.size` | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.reverb.predelayMs` | `PlainFloat` | `0.0..=250.0` | `20.0` | `0.1` | milliseconds | automatable |
| `native.reverb.decayS` | `Seconds` | `0.1..=30.0` | `2.5` | `0.01` | seconds | automatable, logarithmic |
| `native.reverb.damping` | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.reverb.lowCutHz` | `FrequencyHertz` | `20.0..=2000.0` | `100.0` | `0.1` | hertz | automatable, logarithmic |
| `native.reverb.highCutHz` | `FrequencyHertz` | `1000.0..=20000.0` | `16000.0` | `0.1` | hertz | automatable, logarithmic |
| `native.reverb.diffusion` | `Percentage` | `0.0..=1.0` | `0.75` | `0.001` | percent | automatable |
| `native.reverb.width` | `Percentage` | `0.0..=2.0` | `1.0` | `0.001` | percent | automatable |
| `native.reverb.earlyReflections` | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.reverb.mix` | `Percentage` | `0.0..=1.0` | `0.25` | `0.001` | percent | automatable |
| `native.reverb.outputDb` | `Decibels` | `-60.0..=12.0` | `0.0` | `0.1` | decibels | automatable |

Implementation notes:

- The first reverb is a deterministic bounded Schroeder-style design: one
  bounded predelay line feeds eight bounded feedback lines with damping,
  diffusion, early-reflection blend, wet tonal shaping, stereo width, wet/dry
  mix, and output gain.
- Maximum predelay is 250 ms; each tank line is bounded to 500 ms at the active
  sample rate. The frame loop does not allocate, lock, log, or touch the
  filesystem after `prepare`.
- `decayS` drives the feedback coefficient from an RT60-style estimate and is
  capped so the tank remains stable. Reset clears the tank and tail; bypass
  passes input through and does not emit stored tail.
- Offline and realtime renders share the same DSP implementation and are covered
  by matching fixtures. Convolution, algorithm selection, and true send-return
  wet-only presets remain follow-up scope after the routing graph in #84.

### #129 Native Drive And Degradation Effects

Purpose: cover common tracker coloration and lo-fi workflows.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Drive | `native.effect.drive` | track insert, master, sampler-local variant | Implemented | passthrough | yes | 0 | none |
| Bitcrusher | `native.effect.bitcrusher` | track insert, master, sampler-local variant | Implemented | passthrough | yes | 0 | none |

| Parameter | Device | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `native.drive.mode` | Drive | `Enum` | `overdrive`, `saturation`, `hardClip`, `softClip` | `overdrive` | stepped | choice | automatable, stepped |
| `native.drive.driveDb` | Drive | `Decibels` | `0.0..=48.0` | `12.0` | `0.1` | decibels | automatable |
| `native.drive.tone` | Drive | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.drive.bias` | Drive | `Percentage` | `-1.0..=1.0` | `0.0` | `0.001` | percent | automatable, bipolar |
| `native.drive.mix` | Drive | `Percentage` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |
| `native.drive.outputDb` | Drive | `Decibels` | `-60.0..=12.0` | `0.0` | `0.1` | decibels | automatable |
| `native.bitcrusher.bitDepth` | Bitcrusher | `Integer` | `1..=24` | `12` | `1` | bits | automatable, stepped |
| `native.bitcrusher.reductionRatio` | Bitcrusher | `PlainFloat` | `1.0..=64.0` | `1.0` | `1.0` | ratio | automatable, stepped |
| `native.bitcrusher.dither` | Bitcrusher | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |
| `native.bitcrusher.mix` | Bitcrusher | `Percentage` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |
| `native.bitcrusher.outputDb` | Bitcrusher | `Decibels` | `-60.0..=12.0` | `0.0` | `0.1` | decibels | automatable |

Implementation notes:

- Drive applies deterministic waveshaping with four stepped modes. `driveDb`
  is pre-shaper gain; `bias` offsets the shaper input for asymmetric tones;
  `tone` blends between a darker wet signal and the shaped output; `mix` is
  dry/wet; `outputDb` is post-mix makeup/attenuation. No implicit limiter is
  inserted, matching the utility/filter policy that later devices or export
  encoding own clipping.
- Bitcrusher combines bit-depth quantization and sample-rate reduction. The
  `reductionRatio` parameter holds each input frame for a stepped number of
  frames from 1 to 64, giving deterministic sample-rate reduction without
  resampling latency. Optional dither uses a deterministic per-device PRNG so
  realtime and offline renders remain stable when state is prepared equally.
- Both devices reject non-finite and out-of-range parameters before processing,
  allocate only during graph preparation, and share the same realtime/offline
  frame processor. TUI commands create Drive as device id 9 and Bitcrusher as
  device id 10 for track and master chains; parameter locks target mode,
  drive/tone/bias/mix/output and bit-depth/reduction/dither/mix/output.

### #130 Native Modulation Effects

Purpose: cover deterministic LFO-based movement.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Chorus | `native.effect.chorus` | track insert, master | Implemented | passthrough | yes | 0 | bounded delay memory |
| Flanger | `native.effect.flanger` | track insert, master | Implemented | passthrough | yes | 0 | bounded delay memory |
| Phaser | `native.effect.phaser` | track insert, master | Implemented | passthrough | yes | 0 | none |
| Tremolo | `native.effect.tremolo` | track insert, master | Planned | passthrough | no | 0 | none |
| Auto Pan | `native.effect.auto_pan` | track insert, master | Planned | passthrough | no | 0 | none |

| Parameter | Device | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `native.chorus.rateHz` | Chorus | `FrequencyHertz` | `0.01..=20.0` | `0.5` | `0.01` | hertz | automatable |
| `native.chorus.sync` | Chorus | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |
| `native.chorus.depth` | Chorus | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.chorus.delayMs` | Chorus | `PlainFloat` | `1.0..=40.0` | `12.0` | `0.1` | milliseconds | automatable |
| `native.chorus.voices` | Chorus | `Integer` | `1..=4` | `2` | `1` | none | automatable, stepped |
| `native.chorus.spread` | Chorus | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.chorus.feedback` | Chorus | `Percentage` | `0.0..=0.95` | `0.1` | `0.001` | percent | automatable |
| `native.chorus.mix` | Chorus | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.chorus.outputDb` | Chorus | `Decibels` | `-60.0..=12.0` | `0.0` | `0.1` | decibels | automatable |
| `native.flanger.rateHz` | Flanger | `FrequencyHertz` | `0.01..=20.0` | `0.5` | `0.01` | hertz | automatable |
| `native.flanger.sync` | Flanger | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |
| `native.flanger.depth` | Flanger | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.flanger.manual` | Flanger | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.flanger.delayMs` | Flanger | `PlainFloat` | `0.1..=20.0` | `3.0` | `0.1` | milliseconds | automatable |
| `native.flanger.feedback` | Flanger | `Percentage` | `-0.95..=0.95` | `0.0` | `0.001` | percent | automatable, bipolar |
| `native.flanger.stereoPhase` | Flanger | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.flanger.mix` | Flanger | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.flanger.outputDb` | Flanger | `Decibels` | `-60.0..=12.0` | `0.0` | `0.1` | decibels | automatable |
| `native.phaser.rateHz` | Phaser | `FrequencyHertz` | `0.01..=20.0` | `0.5` | `0.01` | hertz | automatable |
| `native.phaser.sync` | Phaser | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |
| `native.phaser.depth` | Phaser | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.phaser.centerHz` | Phaser | `FrequencyHertz` | `200.0..=8000.0` | `1000.0` | `0.1` | hertz | automatable |
| `native.phaser.stages` | Phaser | `Integer` | `2..=12` | `4` | `1` | none | automatable, stepped |
| `native.phaser.feedback` | Phaser | `Percentage` | `-0.95..=0.95` | `0.0` | `0.001` | percent | automatable, bipolar |
| `native.phaser.stereoPhase` | Phaser | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.phaser.mix` | Phaser | `Percentage` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.phaser.outputDb` | Phaser | `Decibels` | `-60.0..=12.0` | `0.0` | `0.1` | decibels | automatable |
| `native.tremolo.rate_hz` | Tremolo | `PlainFloat` | `0.01..=20.0` | `4.0` | `0.001` | hertz | automatable, logarithmic |
| `native.tremolo.depth` | Tremolo | `PlainFloat` | `0.0..=1.0` | `0.5` | `0.001` | normalized | automatable |
| `native.tremolo.shape` | Tremolo | `Enum` | `sine`, `triangle`, `square`, `saw` | `sine` | stepped | none | automatable, stepped |
| `native.auto_pan.rate_hz` | Auto Pan | `PlainFloat` | `0.01..=20.0` | `0.5` | `0.001` | hertz | automatable, logarithmic |
| `native.auto_pan.depth` | Auto Pan | `PlainFloat` | `0.0..=1.0` | `0.5` | `0.001` | normalized | automatable |
| `native.auto_pan.phase` | Auto Pan | `PlainFloat` | `0.0..=360.0` | `180.0` | `0.1` | degrees | automatable |
| `native.auto_pan.shape` | Auto Pan | `Enum` | `sine`, `triangle`, `square`, `saw` | `sine` | stepped | none | automatable, stepped |

Implementation notes:

- Chorus and flanger share a deterministic modulated-delay kernel. Delay memory
  is allocated during graph preparation, is bounded by each device's maximum
  delay, and is reused by realtime and offline rendering.
- Phaser uses the same LFO/reset path with deterministic all-pass stages. The
  `stages` parameter is stepped, while feedback and stereo phase remain
  automatable parameter-lock targets.
- `sync` currently quantizes the LFO rate to a fixed deterministic set of
  musical-rate values until the DSP graph carries transport tempo into each
  insert processor.
- Commands create Chorus as device id 11, Flanger as id 12, and Phaser as id 13
  for track and master chains. Parameter locks cover rate, sync, depth,
  feedback, mix, output, and device-specific controls (`voices`, `spread`,
  `manual`, `stereoPhase`, `centerHz`, `stages`).

### #131 Native Dynamics Effects

Purpose: provide basic mix-control processors before more advanced routing.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Compressor | `native.effect.compressor` | track insert, master | Implemented | passthrough | yes | 0 | none |
| Gate | `native.effect.gate` | track insert, master | Implemented | passthrough | no | 0 | hold/release envelope only |
| Limiter | `native.effect.limiter` | master, track insert | Implemented | passthrough | no | bounded lookahead; descriptor default 48 frames | none |

| Parameter | Device | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `native.compressor.thresholdDb` | Compressor | `Decibels` | `-80.0..=0.0` | `-18.0` | `0.1` | decibels | automatable |
| `native.compressor.ratio` | Compressor | `Ratio` | `1.0..=20.0` | `4.0` | `0.01` | ratio | automatable, logarithmic |
| `native.compressor.attackMs` | Compressor | `PlainFloat` | `0.01..=500.0` | `10.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.compressor.releaseMs` | Compressor | `PlainFloat` | `1.0..=5000.0` | `100.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.compressor.kneeDb` | Compressor | `Decibels` | `0.0..=24.0` | `6.0` | `0.1` | decibels | automatable |
| `native.compressor.makeupDb` | Compressor | `Decibels` | `-24.0..=24.0` | `0.0` | `0.1` | decibels | automatable |
| `native.compressor.autoMakeup` | Compressor | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |
| `native.compressor.detector` | Compressor | `Enum` | `peak`, `rms` | `peak` | stepped | choice | automatable, stepped |
| `native.compressor.stereoLink` | Compressor | `Percentage` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |
| `native.compressor.mix` | Compressor | `Percentage` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |
| `native.compressor.gainReductionDb` | Compressor | `Decibels` | `-80.0..=0.0` | `0.0` | `0.1` | decibels | read-only |
| `native.gate.thresholdDb` | Gate | `Decibels` | `-80.0..=0.0` | `-48.0` | `0.1` | decibels | automatable |
| `native.gate.hysteresisDb` | Gate | `Decibels` | `0.0..=24.0` | `3.0` | `0.1` | decibels | automatable |
| `native.gate.attackMs` | Gate | `PlainFloat` | `0.01..=500.0` | `5.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.gate.holdMs` | Gate | `PlainFloat` | `0.0..=1000.0` | `25.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.gate.releaseMs` | Gate | `PlainFloat` | `1.0..=5000.0` | `100.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.gate.rangeDb` | Gate | `Decibels` | `0.0..=80.0` | `80.0` | `0.1` | decibels | automatable |
| `native.gate.detector` | Gate | `Enum` | `peak`, `rms` | `peak` | stepped | choice | automatable, stepped |
| `native.gate.stereoLink` | Gate | `Percentage` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |
| `native.gate.open` | Gate | `Bool` | `false`, `true` | `false` | stepped | choice | read-only |
| `native.limiter.ceilingDb` | Limiter | `Decibels` | `-24.0..=0.0` | `-0.1` | `0.1` | decibels | automatable |
| `native.limiter.inputGainDb` | Limiter | `Decibels` | `-24.0..=24.0` | `0.0` | `0.1` | decibels | automatable |
| `native.limiter.releaseMs` | Limiter | `PlainFloat` | `1.0..=1000.0` | `50.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.limiter.lookaheadMs` | Limiter | `PlainFloat` | `0.0..=20.0` | `1.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.limiter.stereoLink` | Limiter | `Percentage` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |
| `native.limiter.truePeak` | Limiter | `Bool` | `false`, `true` | `false` | stepped | choice | automatable, stepped |
| `native.limiter.gainReductionDb` | Limiter | `Decibels` | `-80.0..=0.0` | `0.0` | `0.1` | decibels | read-only |

Implementation notes:

- Compressor uses deterministic peak/RMS detection, optional stereo linking,
  attack/release envelope following, soft knee gain computation, dry/wet mix,
  and optional auto makeup.
- Gate uses the same detector/link model with hysteresis, hold, release, and
  range attenuation. Gate state is exposed as a read-only descriptor default;
  realtime callbacks do not allocate or lock to publish meters.
- Limiter applies input gain, release smoothing, bounded sample lookahead, and
  ceiling clamp. `truePeak` is serialized and automatable, but currently maps to
  sample-peak processing; oversampled true-peak limiting remains a follow-up.

## Implementation Order

0. **#115 C/C++ DSP boundary**: optional FFI wrapper pattern for reviewed
   native algorithms. The foundation compiles a project-authored C gain fixture
   under `c-dsp-boundary`, keeps unsafe calls isolated in `salieri-audio`, and
   validates deterministic fixed-buffer processing before broader wrappers.
1. **#125 Utility devices**: lowest DSP risk; hardens descriptor catalog,
   bypass, TUI editing, realtime/offline tests, and state migration patterns.
2. **#126 Multimode filter**: introduces coefficient/state lifecycle without
   tail memory; coordinate sampler-local reuse with #121.
3. **#127 Delay**: introduces bounded time buffers, tempo sync, and tail
   rendering; should wait until filter descriptor patterns are stable.
4. **#128 Reverb**: builds on tail handling and bounded state from delay.
5. **#129 Drive/degradation**: independent waveshaping and stepped degradation;
   can run after utility/filter descriptor conventions are stable.
6. **#130 Modulation effects**: requires deterministic phase/reset semantics and
   a clear offline/realtime clock contract.
7. **#131 Dynamics**: requires envelope follower tests and later sidechain
   extensions must wait for #84 routing.

Implementation issues may proceed independently only when they satisfy the
shared gates below.

## Shared Gates For Every Native Effect Issue

- Define descriptors in `salieri-core` using stable IDs and serializable
  `ParameterValue` defaults.
- Persist device state without backend, TUI, plugin SDK, or project-loader types.
- Validate all values through descriptors; do not duplicate range constants in
  command handlers.
- Expose implemented parameters in the relevant TUI view. Until generic
  descriptor-driven editors from #137 exist, command/status text must show every
  implemented parameter.
- Make every automatable parameter targetable by the generic per-step parameter
  lock model from #123 before closing the device as production-ready.
- Process the same plain runtime spec in realtime and offline render paths.
- Avoid allocation, logging, filesystem access, locks, and unbounded work in the
  audio callback.
- Cover lifecycle, validation, bypass, reset, latency/tail, and deterministic
  realtime/offline rendering in tests.
- Keep `salieri-audio` independent from project serialization and TUI internals.

## Relationship To Other Work

- #84 owns send and master FX routing foundations. Audio send routing now
  exists; follow-up send devices should build on the same deterministic
  realtime/offline bus path.
- #85 owns tracker-native per-step commands. Commands such as retrigger, delay,
  arpeggio, and slides remain row-event semantics rather than native insert
  devices.
- #116 owns the plugin-neutral native module lifecycle boundary used by effects
  and future instruments.
- #121 owns sampler-local filtering, envelopes, keyzones, and modulation
  boundaries.
- #123 owns generic parameter locks. The initial model is implemented for
  sampler, mixer/send, and native gain/pan targets; new automatable native
  effect parameters in this catalog must extend that model.
- #124 owns canonical parameter descriptor/value metadata. New devices extend
  the catalog; they must not create a parallel parameter system.
- #137 owns generated descriptor-driven inspectors/editors. It should consume
  this catalog rather than hardcoding device-specific widgets first.

## Explicit Deferrals

- Direct VST, AU, and CLAP hosting.
- Renoise binary preset compatibility.
- Convolution reverb/cabinet processing.
- Sidechain dynamics before send/routing graph semantics.
- Meta/modulation devices that only control other parameters before #123 and
  #137 establish generic parameter targets and descriptor-driven editing.
