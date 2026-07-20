# Tracker Mini parity matrix

This page tracks Salieri Tracker against the local Polyend Tracker Mini
Essentials reference supplied with issue #74. The goal is workflow parity, not
hardware or UI cloning: Salieri stays a host-based terminal tracker with a
portable `.salieri` project format.

Status meanings:

- **Supported**: usable in the current app and covered by tests or stable model
  behavior.
- **Partial**: the model or workflow exists, but important editor, playback, or
  routing behavior remains incomplete.
- **Planned**: accepted direction with an implementation issue.
- **Deferred**: valid post-MVP capability, sequenced behind lower-level work.
- **Host OS**: hardware behavior maps to the user's computer, audio/MIDI
  devices, filesystem, or terminal.
- **Out of scope**: intentionally excluded from default Salieri Tracker scope.

## Capability matrix

| Area | Capability | Status | Salieri mapping and rationale | Owner |
| --- | --- | --- | --- | --- |
| Hardware | Battery, screen, physical controls, SD card, bundled adapters | Host OS | Salieri runs on the host. Power, display, storage, USB, audio, and MIDI ports are provided by the operating system and connected devices. | n/a |
| Hardware | Built-in games, device-specific firmware update, emergency hardware reset | Out of scope | These are embedded-device maintenance features with no tracker-project equivalent. App release/update work should stay outside Tracker Mini parity. | n/a |
| Project | Portable project file with patterns, tracks, sequence, samples, mixer, and runtime-neutral state | Supported | `.salieri` is the canonical project format with validation and fixture coverage. Hardware card layout is mapped to workspace/project library paths. | existing |
| Project | Workspace artifact roots and portable sample/project locations | Planned | Host filesystem roots replace SD-card folders and should make rendered, recorded, and imported assets portable. | #94 |
| Configuration | General app preferences and keyboard/editing defaults | Supported | TOML config covers UI preferences, keyboard defaults, browser roots, MIDI defaults, AI settings, and validation. | existing |
| Configuration | Tracker Mini-style function-button binding | Partial | Salieri has stable commands, keymaps, focus targets, and command palette recents. Dedicated Fn-button pages map to configurable keybindings and commands rather than fixed hardware buttons. | existing |
| Pattern editor | Full tracker grid with note, velocity, instrument, volume, pan, delay, and two effect slots | Supported | The pattern editor renders and edits all persisted tracker cell fields. | existing |
| Pattern editor | Focused note, instrument, FX, and two-field pattern layouts | Supported | `:layout fields ...` switches the pattern grid to full, note, instrument, FX, note+instrument, note+FX, or instrument+FX layouts while preserving cursor and scroll offsets. | #73 |
| Pattern editor | Manual note entry with edit mode, step jump, note off/cut, velocity, and instrument digits | Supported | Edit-mode tests cover tracker keyboard entry, step jump, two-digit instrument entry, note-off/cut, undo/redo, and cursor clamping. | existing |
| Pattern editor | Selection copy, cut, paste, delete, and undo/redo | Supported | Selection operations are undoable and validated against pattern bounds. | existing |
| Pattern editor | Fill, invert, expand, shrink, duplicate, copy, and paste pattern/selection operations | Supported | `:pattern fill`, `copy`, `paste`, `invert`, `expand`, `shrink`, and `duplicate-selection` operate on the active selection or current pattern where appropriate and use the normal undo/redo stack. | #86 |
| Pattern editor | Render selection into a reusable sample | Partial | `:sample render-selection PATH [--assign TRACK]` renders selected internal sampler/native audio into a WAV sample reference and can assign it immediately. External MIDI-only capture remains deferred. | #79 |
| Tracker FX | Two step FX columns and core timing/position/value commands | Partial | Salieri has FX1/FX2 fields, supported parser commands, and playback behavior for implemented effects. Deferred command families are explicit diagnostics, not silent parity claims. | existing |
| Tracker FX | Full Tracker Mini step FX catalog | Deferred | More command families depend on sampler playback modes, sends, filters, native DSP coverage, and performance routing. | #83, #84 |
| Instruments | Track-level sample assignment and sample-backed instrument data | Partial | Current projects persist sample references, sample-backed instruments, track assignments, root note, gain, playback settings, and envelope data. Instrument slots are not yet independent project resources. | #77 |
| Instruments | Reusable instrument preset files | Partial | Salieri can save local preset metadata profiles for current instruments and devices. Portable instrument import/export remains tied to independent instrument slots. | #77, #93 |
| Sampler | WAV sample browser, preview, assignment, playback range, gain, root note, and envelope basics | Partial | Sampler foundations exist and are tracked in the sampler parity roadmap. Sustained loop behavior and several mode-specific controls remain incomplete. | [sampler parity](sampler-parity-roadmap.md) |
| Sampler | One-shot, forward loop, backward loop, ping-pong, reverse, slice, beat-slice, wavetable, and granular modes | Planned | One-shot exists; loop metadata is partial. Direction-aware, slicing, wavetable, and granular playback are sequenced separately. | #76 |
| Sampling | Host audio input sample recorder with trim/crop/save/load | Planned | Maps line/mic recording to host audio input devices and fake-input tests for CI. | #78 |
| Sampling | Internal render/bounce workflows | Partial | Render plans, deterministic per-track stem export, and selection-to-sample bounce exist for internal sampler/native audio. MIDI-only external instruments remain explicit non-rendered sources. | #79, #95 |
| Song mode | Linear sequence of pattern slots | Supported | The sequence model persists pattern order and supports sequence playback/editing commands. | existing |
| Song mode | Clip-like song slot view with per-track activity scan | Planned | Needs a dedicated TUI view on top of the existing sequence model. | #80 |
| Performance | Momentary performance effects targeting tracks/patterns | Planned | Temporary punch-in state must restore the underlying project after release and stay separate from saved presets. | #83 |
| Mixer and routing | Track mix, pan, mute/solo, sample gain automation, and native device chains | Partial | Mixer and native DSP foundations exist for realtime/offline paths, but Tracker Mini-style send/return and master chains are not complete. | existing |
| Mixer and routing | Delay/reverb sends and master processing chain | Planned | Requires serializable routing, realtime/offline parity, and placeholders or implementations for unsupported native devices. | #84 |
| MIDI | MIDI output, fake MIDI tests, MIDI logging, input parsing, and basic clock/transport foundations | Partial | Salieri is MIDI-first and already has output/input abstractions. Tracker Mini's independent clock, transport, note, CC, channel, middle-C, and latency settings need explicit persisted controls. | #82 |
| Import/export | MIDI import/export and Renoise/XRNS parity diagnostics | Partial | MIDI and XRNS workflows exist with explicit unsupported diagnostics. Tracker Mini parity does not require Renoise UI compatibility. | existing |
| Import/export | MusicXML and notation round-trip validation | Deferred | Useful cross-format workflow but not required for Tracker Mini hardware parity. | #88 |
| DAW integration | Ableton-style clip launcher and Live bridge | Deferred | These extend Salieri beyond Tracker Mini parity and depend on a distinct clip model. | #87, #89 |
| AI and cloud | Cloud transcription, source separation, non-local AI services | Out of scope | Default Tracker Mini parity excludes non-local AI or cloud media processing. Optional AI-assisted edits remain explicit, reviewable, and provider-configured. | #68, #69, #70 |
| Research | Style analysis, reports, dossiers, lyrics, live-coding export, composition graph | Partial | Text notes, lyric lines, cue markers, compact annotation reports, and deterministic Strudel text export are implemented as optional workflows. Style analysis, broader reports/dossiers, and composition graph remain deferred. | #90, #92, #96, #97, #98, #99 |

## Hardware-to-host mapping

| Tracker Mini concept | Salieri equivalent |
| --- | --- |
| Battery and charging | Host battery/power management; not persisted in projects. |
| SD card folders | Workspace and project library roots in config. |
| Line/mic input | Host audio input device once #78 lands. |
| Stereo/headphone output | Host audio backend/output device. |
| MIDI DIN/USB ports | Configured MIDI input/output ports. |
| Physical function buttons | Keymap entries, command palette, and `:focus` / `:layout` commands. |
| Firmware update/reset | Application release/update process; not a tracker feature. |

## Implementation split

The remaining Tracker Mini parity work is intentionally split into focused
issues:

- Pattern workflow: #73, #86
- Instruments and sampler playback: #76, #77
- Sampling and rendering: #78, #79, #95
- Song/performance views: #80, #83
- MIDI and project configuration: #82, #94
- Mixer, sends, and master FX: #84

When adding new Tracker Mini-derived work, prefer updating this matrix and
linking a focused implementation issue instead of expanding a broad catch-all
ticket.
