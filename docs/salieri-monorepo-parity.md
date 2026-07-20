# Salieri monorepo parity matrix

This page tracks feature parity between Salieri Tracker and the source Salieri
monorepo at `~/dev/current/salieri` / `delaudio/salieri`. The goal is selective
absorption: Salieri Tracker should adopt workflows that fit a fast tracker,
sample, MIDI, and terminal-first product, while leaving broad desktop or cloud
workflows out of the default path.

Status meanings:

- **Implemented**: available in Salieri Tracker now.
- **Partial**: present in both directions but not yet feature-equivalent.
- **Tracked**: accepted for Salieri Tracker and linked to an open tracker issue.
- **Deferred**: useful later, but sequenced behind core tracker/audio work.
- **Out of scope**: intentionally excluded from default Salieri Tracker scope.

## Package and surface mapping

| Source Salieri surface | Source role | Tracker status | Tracker mapping | References |
| --- | --- | --- | --- | --- |
| `packages/core` | Symbolic song model, arrangement, style analysis, critique inputs | Partial | Tracker has its own Rust `salieri-core` model for patterns, sequence, mixer, samples, MIDI, and playback. Narrative/song graph concepts stay separate until they can map cleanly to tracker patterns. | #99 |
| `packages/formats` | MIDI, MusicXML, and Renoise-oriented interchange | Partial | Tracker has MIDI and XRNS foundations. MusicXML import/export and round-trip validation are tracked separately. | #88, delaudio/salieri#4, delaudio/salieri#5, delaudio/salieri#6 |
| `packages/cli` | Command surface for generate, analyze, compare, split, compile, import/export, Live bridge, presets, research, reports | Partial | Tracker CLI is Rust-native and focuses on project load/save, MIDI, XRNS, sample inspect/export, audio export, and TUI commands. Non-tracker workflows are mapped individually below. | existing |
| `packages/ai` | Provider-backed generation, critique, revise | Implemented / Partial | Tracker supports explicit, reviewable AI proposals, persisted chat sessions, deterministic report/critique artifacts, and revise prompts that produce pending proposals before mutation. External provider adapters remain tracked separately. | #68, #69, #70, #98 |
| `packages/runtime-node` | Node runtime paths, storage, doctor, generate/critique/revise workflows | Partial | Tracker has TOML config, workspace libraries, diagnostics, and local app state. Portable artifact roots and richer workflow storage are tracked separately. | #94, #98 |
| `packages/ableton` | Live bridge operations for push, pull, clear, info, presets | Tracked | Tracker keeps Ableton integration optional and dry-run friendly. Clip launcher and bridge work are tracked separately from core tracker editing. | #87, #89, delaudio/salieri#7, delaudio/salieri#22 |
| `packages/audio` | Render chain, stems, transcription, source separation experiments | Tracked / Out of scope | Local render and stem export are tracked. Cloud transcription and source separation are out of default scope unless explicitly configured in later workflows. | #95, delaudio/salieri#12, delaudio/salieri#13, delaudio/salieri#15 |
| `apps/desktop` | Tauri desktop app, chat, stream transport, domain experts | Deferred | Tracker remains terminal-first. Useful typed progress, session history, and review/apply patterns have already been adapted into the TUI AI flow. Desktop IPC parity is not a tracker blocker. | #68, #69, #70, delaudio/salieri#41-#58, delaudio/salieri#62, delaudio/salieri#63 |
| `apps/site` | Product/site surface | Out of scope | Marketing or website surfaces do not affect tracker runtime parity. | n/a |

## Workflow parity matrix

| Workflow | Source Salieri capability | Tracker status | Rationale and tracker owner |
| --- | --- | --- | --- |
| Generate song | `generate`, `generate-song`, provider selection, style/profile/dossier inputs | Partial | Tracker supports reviewable AI edits against existing projects, not full autonomous symbolic song generation by default. Broader generation/report flows are tracked in #98 and #99. |
| AI chat/session state | Desktop chat, streaming events, checkpointed sessions, tool/result blocks | Implemented / Partial | Tracker now has cancellable AI task progress, review/apply, and persisted sessions. Rich structured result surfaces beyond tracker edits remain deferred. |
| Analyze/compare style | `analyze`, `compare`, style profiles, song comparison | Implemented / Partial | CLI and TUI can generate deterministic style analyses and project comparisons in text or JSON with density, energy, roles, note statistics, and deltas. | #90 |
| Reports, critique, revise | `critique`, `report`, `revise`, durable report artifacts | Implemented / Partial | CLI and TUI can generate project/critique markdown, save workspace report artifacts, and route revision requests through reviewable AI proposals with explicit apply semantics. | #98 |
| Workspace artifacts | `.salieri.json`, arbitrary workspace roots, save destinations, reports/sets/presets directories | Implemented / Partial | Tracker has a portable `.salieri-workspace.json` manifest, local artifact indexing, and non-destructive trash/restore. Rich browser integration remains incremental. | #94, delaudio/salieri#28, delaudio/salieri#29, delaudio/salieri#39, delaudio/salieri#40 |
| Rendering | Render plans, audio export, and stems | Implemented / Partial | Tracker can inspect JSON render plans and export deterministic internal sampler/native audio WAVs and per-track stems. MIDI-only destinations remain explicit non-captured sources. | #95, delaudio/salieri#14, delaudio/salieri#15 |
| MIDI import/export | Source CLI import/export and round-trip checks | Partial | Tracker has MIDI foundations and project CLI workflows; round-trip validation is tracked with MusicXML work. | #88, delaudio/salieri#4, delaudio/salieri#6 |
| MusicXML | Source import/export and notation fixtures | Tracked | Useful for non-tracker notation interchange, but must report unsupported constructs explicitly. | #88, delaudio/salieri#5 |
| Renoise/XRNS | Source import-renoise plus local parity fixtures | Partial | Tracker already owns XRNS import diagnostics and Renoise parity reporting; remaining gaps are tracked in sampler/instrument/audio issues. | [Renoise parity](renoise-parity.md), #76, #77, #84 |
| Ableton push/pull/clear | Source Live bridge CLI and desktop expert | Tracked | Tracker needs a clip model before Live session mapping is first-class. Dry-run behavior should be available without Ableton. | #87, #89, delaudio/salieri#7 |
| Clip launcher | Source Ableton-oriented scene/clip thinking | Tracked | Tracker clip launcher is intentionally distinct from Tracker Mini song-slot view. | #87 |
| Preset and device inventory | Source preset save/load/list/analyze and inventory work | Implemented / Partial | Tracker can save/list/show local preset metadata profiles for sample-backed instruments, native devices, MIDI ports, and AI guidance. Reusable instrument slots remain tracked separately. | #77, #93, delaudio/salieri#8, delaudio/salieri#10, delaudio/salieri#11 |
| Research dossiers and operational palettes | Source `research`, tutorial dossiers, prompt/profile assets | Implemented / Partial | Tracker can list, inspect, and apply local `.md`, `.txt`, and `.json` guidance files to steer reviewable AI proposals. Remote collection and transcription remain out of default scope. | #92, delaudio/salieri#11, delaudio/salieri#16 |
| Lyrics | Source lyrics roadmap and MusicXML fixture examples | Implemented / Partial | Tracker stores project notes, pattern-row lyrics, sequence cue markers, and compact annotation reports locally. MusicXML lyric import/export remains tracked separately. | #96, delaudio/salieri#25 |
| Strudel/live coding | Source Strudel export target proposal | Implemented / Partial | Tracker can export deterministic Strudel sketches for selected patterns or the sequence, with unsupported sampler, mixer, clip, automation, and tracker-effect diagnostics. | #97, delaudio/salieri#20 |
| CompositionGraph | Source narrative graph, round-trip, evidence, and Ableton command compilation issues | Tracked | Valuable for arrangement planning, but it must compile into tracker patterns/sequence without replacing the pattern editor. | #99, delaudio/salieri#59, delaudio/salieri#60, delaudio/salieri#61, delaudio/salieri#64 |
| Local render/audio/stems | Source render plans and stem workflows | Tracked | Tracker has audio export foundations; render-plan, render-audio, and stem export workflows are tracked explicitly. | #95, delaudio/salieri#14, delaudio/salieri#15 |
| Transcription and source separation | Source audio-analysis and separation workflows | Out of scope by default | These require heavyweight providers and/or external models. They should stay opt-in and outside the default tracker scope. | delaudio/salieri#12, delaudio/salieri#13, delaudio/salieri#26 |
| Desktop IPC/Tauri parity | Source desktop IPC and contract-check issues | Deferred | Tracker is terminal-first; only reusable patterns such as typed events and domain boundaries should be adapted. | delaudio/salieri#62, delaudio/salieri#63 |

## Default exclusions

The following source Salieri capabilities are intentionally excluded from the
default tracker scope:

- cloud transcription;
- source separation;
- mandatory external AI providers;
- desktop-only IPC surfaces;
- website/product surfaces;
- broad symbolic CompositionGraph replacement of the tracker pattern model.

Each excluded area can become an opt-in workflow later, but it should not be a
dependency for opening, editing, playing, exporting, or validating `.salieri`
tracker projects.

## Tracker issue split

- AI proposal/session parity: #68, #69, #70, #98
- Pattern and Tracker Mini parity: #73, #74, #76, #77, #80, #82, #83, #84, #86
- Workspace and artifact parity: #92, #93, #94
- Format and DAW parity: #87, #88, #89, #97
- Audio/render parity: #95
- Analysis, lyrics, and narrative composition: #90, #96, #99

Update this matrix when new Salieri source workflows are imported, rejected, or
split into focused Salieri Tracker issues.
