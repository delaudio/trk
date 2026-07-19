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
| Send effects and buses | Mixer `sends` plus future routing graph | Send bus metadata exists, but audio routing and send-return processing are not implemented yet | planned by #84 before send DSP closure |
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
| Gain, gainer, stereo expander, DC/utility, channel tools | Track insert, master, utility | Partial | #125 native utility audio devices | Existing native gain/pan cover the minimum. Width, polarity, mono, and channel swap are planned as utility devices. |
| EQ and filters | Track insert, sampler-local | Planned | #126 native multimode filter; parametric EQ deferred | A multimode filter should land before full EQ. Sampler-local filter semantics stay coordinated with #121. |
| Delay, multitap delay, repeater | Track insert, send, master | Planned | #127 native delay | First implementation should be stereo delay with tempo sync, feedback, wet/dry, and bounded delay memory. |
| Reverb | Track insert, send, master | Planned | #128 native reverb | First implementation should be deterministic and bounded; convolution is deferred. |
| Distortion, cabinet, lo-fi, bit reduction | Track insert, sampler-local | Planned | #129 native drive and degradation effects | Clip/soft-clip and bit/sample-rate reduction are baseline; cabinet/convolution deferred. |
| Chorus, flanger, phaser, tremolo, ring modulation, autopan | Track insert, master | Planned | #130 native modulation effects | All LFO-driven devices must share deterministic phase/reset behavior for offline and realtime. |
| Compressor, gate, limiter, maximizer | Track insert, master | Planned | #131 native dynamics effects | Sidechain/key input is deferred until send/routing foundations are real. |
| Meta devices, LFO device, hydra, key/velocity trackers | Automation/modulation system | Deferred | Follow-up after #123 and #137 | These control parameters rather than process audio directly. Do not model them as ordinary audio insert devices. |
| Send device and routing utilities | Mixer routing | Partial | #84 before expanded send devices | Current send metadata is placeholder-only. Audio sends need routing and deterministic summing rules first. |
| Native instrument devices | Instrument/module layer | Partial | #116 boundary exists; concrete instruments deferred | Instruments use the same module/state contract but are not part of this effect roadmap except where sampler-local processing overlaps #121. |
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

| Parameter | Device | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `native.gain.gain` | Gain | `PlainFloat` | `0.0..=2.0` | `1.0` | `0.001` | gain | automatable |
| `native.pan.pan` | Pan | `BipolarFloat` | `-1.0..=1.0` | `0.0` | `0.001` | pan | automatable, bipolar |

### #125 Native Utility Audio Devices

Purpose: finish small building blocks that are useful on tracks and master before
more expensive effects land.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Stereo Width | `native.effect.stereo_width` | track insert, master | Planned | passthrough | no | 0 | none |
| Polarity | `native.effect.polarity` | track insert, master | Planned | passthrough | no | 0 | none |
| Mono | `native.effect.mono` | track insert, master | Planned | passthrough | no | 0 | none |
| Balance | `native.effect.balance` | track insert, master | Planned | passthrough | no | 0 | none |

| Parameter | Device | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `native.stereo_width.width` | Stereo Width | `PlainFloat` | `0.0..=2.0` | `1.0` | `0.001` | percent/ratio | automatable |
| `native.polarity.left_invert` | Polarity | `Bool` | `false`, `true` | `false` | stepped | none | automatable, stepped |
| `native.polarity.right_invert` | Polarity | `Bool` | `false`, `true` | `false` | stepped | none | automatable, stepped |
| `native.mono.mode` | Mono | `Enum` | `sum`, `left`, `right` | `sum` | stepped | none | automatable, stepped |
| `native.balance.balance` | Balance | `BipolarFloat` | `-1.0..=1.0` | `0.0` | `0.001` | pan | automatable, bipolar |

### #126 Native Multimode Filter

Purpose: provide the first musical tone-shaping effect and a reusable filter
kernel for later sampler-local processing.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Multimode Filter | `native.effect.filter` | track insert, master, sampler-local variant | Planned | passthrough | no | 0 | none |

| Parameter | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| `native.filter.mode` | `Enum` | `low_pass`, `high_pass`, `band_pass`, `notch` | `low_pass` | stepped | none | automatable, stepped |
| `native.filter.cutoff` | `PlainFloat` | `20.0..=20000.0` | `12000.0` | `0.01` | hertz | automatable, modulatable, logarithmic |
| `native.filter.resonance` | `PlainFloat` | `0.0..=1.0` | `0.0` | `0.001` | normalized | automatable, modulatable |
| `native.filter.drive` | `PlainFloat` | `0.0..=2.0` | `0.0` | `0.001` | gain | automatable |

### #127 Native Delay

Purpose: provide tempo-aware echo for insert and later send workflows.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Stereo Delay | `native.effect.delay` | track insert, send, master | Planned | input passthrough; delay line muted | yes | 0 | yes |

| Parameter | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| `native.delay.time_ms` | `PlainFloat` | `1.0..=4000.0` | `375.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.delay.division` | `Enum` | `free`, `1/64`, `1/32`, `1/16`, `1/8`, `1/4`, `1/2`, `1/1` | `1/4` | stepped | beat division | automatable, stepped |
| `native.delay.feedback` | `PlainFloat` | `0.0..=0.95` | `0.35` | `0.001` | percent | automatable |
| `native.delay.stereo_offset` | `BipolarFloat` | `-1.0..=1.0` | `0.0` | `0.001` | normalized | automatable, bipolar |
| `native.delay.low_cut_hz` | `PlainFloat` | `20.0..=20000.0` | `20.0` | `0.01` | hertz | automatable, logarithmic, advanced |
| `native.delay.high_cut_hz` | `PlainFloat` | `20.0..=20000.0` | `20000.0` | `0.01` | hertz | automatable, logarithmic, advanced |
| `native.delay.wet` | `PlainFloat` | `0.0..=1.0` | `0.35` | `0.001` | percent | automatable |
| `native.delay.dry` | `PlainFloat` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |

### #128 Native Reverb

Purpose: provide a bounded deterministic room/plate-style space effect.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Reverb | `native.effect.reverb` | track insert, send, master | Planned | input passthrough; reverb tank muted | yes | 0 | yes |

| Parameter | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| `native.reverb.algorithm` | `Enum` | `room`, `hall`, `plate` | `room` | stepped | none | automatable, stepped |
| `native.reverb.size` | `PlainFloat` | `0.0..=1.0` | `0.5` | `0.001` | normalized | automatable |
| `native.reverb.decay_seconds` | `PlainFloat` | `0.1..=20.0` | `2.5` | `0.01` | seconds | automatable, logarithmic |
| `native.reverb.pre_delay_ms` | `PlainFloat` | `0.0..=250.0` | `20.0` | `0.1` | milliseconds | automatable |
| `native.reverb.damping` | `PlainFloat` | `0.0..=1.0` | `0.35` | `0.001` | normalized | automatable |
| `native.reverb.width` | `PlainFloat` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |
| `native.reverb.wet` | `PlainFloat` | `0.0..=1.0` | `0.25` | `0.001` | percent | automatable |
| `native.reverb.dry` | `PlainFloat` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |

### #129 Native Drive And Degradation Effects

Purpose: cover common tracker coloration and lo-fi workflows.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Drive | `native.effect.drive` | track insert, master, sampler-local variant | Planned | passthrough | yes | 0 | none |
| Bitcrusher | `native.effect.bitcrusher` | track insert, master, sampler-local variant | Planned | passthrough | yes | 0 | none |

| Parameter | Device | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `native.drive.mode` | Drive | `Enum` | `soft_clip`, `hard_clip`, `foldback` | `soft_clip` | stepped | none | automatable, stepped |
| `native.drive.drive` | Drive | `PlainFloat` | `0.0..=24.0` | `0.0` | `0.01` | decibels | automatable |
| `native.drive.tone` | Drive | `PlainFloat` | `0.0..=1.0` | `0.5` | `0.001` | normalized | automatable |
| `native.drive.wet` | Drive | `PlainFloat` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |
| `native.bitcrusher.bits` | Bitcrusher | `PlainFloat` | `1.0..=24.0` | `12.0` | `1.0` | bits | automatable, stepped |
| `native.bitcrusher.downsample` | Bitcrusher | `PlainFloat` | `1.0..=64.0` | `1.0` | `1.0` | ratio | automatable, stepped |
| `native.bitcrusher.wet` | Bitcrusher | `PlainFloat` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |

### #130 Native Modulation Effects

Purpose: cover deterministic LFO-based movement.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Chorus | `native.effect.chorus` | track insert, master | Planned | passthrough | yes | 0 | yes |
| Flanger | `native.effect.flanger` | track insert, master | Planned | passthrough | yes | 0 | yes |
| Tremolo | `native.effect.tremolo` | track insert, master | Planned | passthrough | no | 0 | none |
| Auto Pan | `native.effect.auto_pan` | track insert, master | Planned | passthrough | no | 0 | none |

| Parameter | Device | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `native.chorus.rate_hz` | Chorus | `PlainFloat` | `0.01..=20.0` | `0.5` | `0.001` | hertz | automatable, logarithmic |
| `native.chorus.depth` | Chorus | `PlainFloat` | `0.0..=1.0` | `0.5` | `0.001` | normalized | automatable |
| `native.chorus.phase` | Chorus | `PlainFloat` | `0.0..=360.0` | `180.0` | `0.1` | degrees | automatable |
| `native.chorus.delay_ms` | Chorus | `PlainFloat` | `0.1..=50.0` | `12.0` | `0.01` | milliseconds | automatable |
| `native.chorus.wet` | Chorus | `PlainFloat` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.flanger.rate_hz` | Flanger | `PlainFloat` | `0.01..=20.0` | `0.25` | `0.001` | hertz | automatable, logarithmic |
| `native.flanger.depth` | Flanger | `PlainFloat` | `0.0..=1.0` | `0.5` | `0.001` | normalized | automatable |
| `native.flanger.phase` | Flanger | `PlainFloat` | `0.0..=360.0` | `180.0` | `0.1` | degrees | automatable |
| `native.flanger.feedback` | Flanger | `BipolarFloat` | `-0.95..=0.95` | `0.0` | `0.001` | percent | automatable, bipolar |
| `native.flanger.delay_ms` | Flanger | `PlainFloat` | `0.1..=20.0` | `3.0` | `0.01` | milliseconds | automatable |
| `native.flanger.wet` | Flanger | `PlainFloat` | `0.0..=1.0` | `0.5` | `0.001` | percent | automatable |
| `native.tremolo.rate_hz` | Tremolo | `PlainFloat` | `0.01..=20.0` | `4.0` | `0.001` | hertz | automatable, logarithmic |
| `native.tremolo.depth` | Tremolo | `PlainFloat` | `0.0..=1.0` | `0.5` | `0.001` | normalized | automatable |
| `native.tremolo.shape` | Tremolo | `Enum` | `sine`, `triangle`, `square`, `saw` | `sine` | stepped | none | automatable, stepped |
| `native.auto_pan.rate_hz` | Auto Pan | `PlainFloat` | `0.01..=20.0` | `0.5` | `0.001` | hertz | automatable, logarithmic |
| `native.auto_pan.depth` | Auto Pan | `PlainFloat` | `0.0..=1.0` | `0.5` | `0.001` | normalized | automatable |
| `native.auto_pan.phase` | Auto Pan | `PlainFloat` | `0.0..=360.0` | `180.0` | `0.1` | degrees | automatable |
| `native.auto_pan.shape` | Auto Pan | `Enum` | `sine`, `triangle`, `square`, `saw` | `sine` | stepped | none | automatable, stepped |

### #131 Native Dynamics Effects

Purpose: provide basic mix-control processors before more advanced routing.

| Device | ID | Placements | Status | Bypass | Wet/dry | Latency | Tail |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Compressor | `native.effect.compressor` | track insert, master | Planned | passthrough | yes | 0 | none |
| Gate | `native.effect.gate` | track insert, master | Planned | passthrough | no | 0 | none |
| Limiter | `native.effect.limiter` | master, track insert | Planned | passthrough | no | lookahead optional/deferred | none |

| Parameter | Device | Type | Range / choices | Default | Step | Unit | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `native.compressor.threshold_db` | Compressor | `BipolarFloat` | `-80.0..=0.0` | `-18.0` | `0.1` | decibels | automatable |
| `native.compressor.ratio` | Compressor | `PlainFloat` | `1.0..=20.0` | `4.0` | `0.01` | ratio | automatable, logarithmic |
| `native.compressor.attack_ms` | Compressor | `PlainFloat` | `0.01..=500.0` | `10.0` | `0.01` | milliseconds | automatable, logarithmic |
| `native.compressor.release_ms` | Compressor | `PlainFloat` | `1.0..=5000.0` | `100.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.compressor.knee_db` | Compressor | `PlainFloat` | `0.0..=24.0` | `6.0` | `0.1` | decibels | automatable |
| `native.compressor.makeup_db` | Compressor | `BipolarFloat` | `-24.0..=24.0` | `0.0` | `0.1` | decibels | automatable, bipolar |
| `native.gate.threshold_db` | Gate | `BipolarFloat` | `-80.0..=0.0` | `-48.0` | `0.1` | decibels | automatable |
| `native.gate.attack_ms` | Gate | `PlainFloat` | `0.01..=500.0` | `5.0` | `0.01` | milliseconds | automatable, logarithmic |
| `native.gate.release_ms` | Gate | `PlainFloat` | `1.0..=5000.0` | `100.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.gate.range_db` | Gate | `PlainFloat` | `0.0..=80.0` | `80.0` | `0.1` | decibels | automatable |
| `native.limiter.threshold_db` | Limiter | `BipolarFloat` | `-24.0..=0.0` | `-1.0` | `0.1` | decibels | automatable |
| `native.limiter.attack_ms` | Limiter | `PlainFloat` | `0.01..=100.0` | `1.0` | `0.01` | milliseconds | automatable, logarithmic |
| `native.limiter.release_ms` | Limiter | `PlainFloat` | `1.0..=1000.0` | `50.0` | `0.1` | milliseconds | automatable, logarithmic |
| `native.limiter.ceiling_db` | Limiter | `BipolarFloat` | `-24.0..=0.0` | `-0.1` | `0.1` | decibels | automatable |
| `native.compressor.mix` | Compressor | `PlainFloat` | `0.0..=1.0` | `1.0` | `0.001` | percent | automatable |

## Implementation Order

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

- #84 owns send and master FX routing foundations. This roadmap can list send
  placements, but send devices cannot be complete until audio send routing
  exists.
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
