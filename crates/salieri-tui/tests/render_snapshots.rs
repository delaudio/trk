use salieri_core::{Cursor, DelaySpec, EffectDevice, NoteEvent, Song};
use salieri_sampler::{WaveformBucket, WaveformOverview};
use salieri_tui::{
    DspDevicePaletteEntryView, DspDevicePaletteViewState, DspParameterLockStatusView,
    DspRackTargetView, DspRackViewState, HelpTab, MidiPortView, MidiSettingsState,
    PatternFieldLayout, ProjectBrowserEntryKind, ProjectBrowserEntryView, ProjectBrowserViewState,
    SamplerViewState, SelectionRect, TuiState, TuiView,
};

mod support;
use support::{
    assert_snapshot, render_snapshot, render_waveform_snapshot, test_state, waveform_overview,
};

#[test]
fn snapshots_empty_pattern_editor() {
    assert_snapshot(
        "empty-pattern",
        render_snapshot(Song::empty(), test_state(), 100, 28),
    );
}

#[test]
fn snapshots_populated_pattern_editor() {
    let mut song = Song::empty();
    let pattern = song.current_pattern_mut().expect("pattern");
    pattern
        .set_note(0, 0, NoteEvent::Note { pitch: 36 }, 0x7f)
        .expect("note");
    pattern
        .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x64)
        .expect("note");
    pattern
        .set_note(1, 2, NoteEvent::Note { pitch: 64 }, 0x50)
        .expect("note");
    pattern
        .set_note_event(3, 1, NoteEvent::NoteOff, None)
        .expect("off");

    assert_snapshot(
        "populated-pattern",
        render_snapshot(
            song,
            TuiState {
                cursor: Cursor {
                    row: 1,
                    track: 2,
                    field: salieri_core::CellField::Note,
                    digit: 0,
                },
                dirty: true,
                is_playing: true,
                playhead_row: Some(3),
                midi_status: "MIDI Connected 0",
                sequence_position: Some(0),
                ..test_state()
            },
            100,
            28,
        ),
    );
}

#[test]
fn snapshots_cursor_and_selection_state() {
    assert_snapshot(
        "cursor-selection",
        render_snapshot(
            Song::empty(),
            TuiState {
                cursor: Cursor {
                    row: 4,
                    track: 1,
                    field: salieri_core::CellField::Velocity,
                    digit: 1,
                },
                selection: Some(SelectionRect {
                    row_start: 2,
                    row_end: 5,
                    track_start: 0,
                    track_end: 2,
                }),
                ..test_state()
            },
            100,
            28,
        ),
    );
}

#[test]
fn snapshots_viewport_boundary_large_pattern() {
    assert_snapshot(
        "viewport-boundary-large-pattern",
        render_snapshot(
            large_tracker_song(4_096, 12),
            TuiState {
                cursor: Cursor {
                    row: 4095,
                    track: 11,
                    field: salieri_core::CellField::Effect,
                    digit: 1,
                },
                row_offset: 4088,
                track_offset: 8,
                selection: Some(SelectionRect {
                    row_start: 4090,
                    row_end: 4095,
                    track_start: 9,
                    track_end: 11,
                }),
                is_playing: true,
                playhead_row: Some(4094),
                ..test_state()
            },
            72,
            24,
        ),
    );
}

#[test]
fn snapshots_partially_visible_track_fields() {
    assert_snapshot(
        "partial-track-fields",
        render_snapshot(
            large_tracker_song(64, 8),
            TuiState {
                cursor: Cursor {
                    row: 4,
                    track: 3,
                    field: salieri_core::CellField::Pan,
                    digit: 0,
                },
                track_offset: 1,
                selection: Some(SelectionRect {
                    row_start: 3,
                    row_end: 5,
                    track_start: 2,
                    track_end: 3,
                }),
                ..test_state()
            },
            56,
            18,
        ),
    );
}

#[test]
fn snapshots_focused_note_pattern_fields() {
    let tracker_layout = salieri_tui::TrackerLayoutState {
        pattern_fields: PatternFieldLayout::Note,
        ..Default::default()
    };

    assert_snapshot(
        "focused-note-pattern-fields",
        render_snapshot(
            large_tracker_song(64, 12),
            TuiState {
                cursor: Cursor {
                    row: 4,
                    track: 8,
                    field: salieri_core::CellField::Note,
                    digit: 0,
                },
                tracker_layout,
                ..test_state()
            },
            56,
            18,
        ),
    );
}

#[test]
fn snapshots_help_overlay() {
    assert_snapshot(
        "help-overlay",
        render_snapshot(
            Song::empty(),
            TuiState {
                mode_label: "HELP",
                show_help: true,
                ..test_state()
            },
            120,
            36,
        ),
    );
}

#[test]
fn snapshots_help_overlay_sampler_tab() {
    assert_snapshot(
        "help-overlay-sampler",
        render_snapshot(
            Song::empty(),
            TuiState {
                mode_label: "HELP",
                show_help: true,
                help_tab: HelpTab::Sampler,
                ..test_state()
            },
            120,
            36,
        ),
    );
}

#[test]
fn snapshots_midi_settings_overlay() {
    let ports = [
        MidiPortView {
            index: 0,
            name: "IAC Driver Bus 1",
        },
        MidiPortView {
            index: 2,
            name: "External Synth",
        },
    ];

    let routing_song = Song::empty();
    assert_snapshot(
        "midi-settings",
        render_snapshot(
            Song::empty(),
            TuiState {
                mode_label: "MIDI",
                midi_settings: Some(MidiSettingsState {
                    ports: &ports,
                    selected_port: 1,
                    status: "MIDI Disconnected",
                    input_status: "MIDI In Disconnected",
                    routing: &routing_song.midi,
                }),
                ..test_state()
            },
            100,
            28,
        ),
    );
}

#[test]
fn snapshots_small_layout() {
    assert_snapshot(
        "responsive-small",
        render_snapshot(Song::empty(), test_state(), 72, 24),
    );
}

#[test]
fn snapshots_compact_layout() {
    assert_snapshot(
        "responsive-compact",
        render_snapshot(Song::empty(), test_state(), 80, 24),
    );
}

#[test]
fn snapshots_medium_layout() {
    assert_snapshot(
        "responsive-medium",
        render_snapshot(Song::empty(), test_state(), 100, 28),
    );
}

#[test]
fn snapshots_large_layout() {
    assert_snapshot(
        "responsive-large",
        render_snapshot(Song::empty(), test_state(), 140, 36),
    );
}

#[test]
fn snapshots_renoise_pattern_workspace() {
    assert_snapshot(
        "renoise-pattern-workspace",
        render_snapshot(
            large_tracker_song(96, 8),
            TuiState {
                cursor: Cursor {
                    row: 4,
                    track: 3,
                    field: salieri_core::CellField::Instrument,
                    digit: 1,
                },
                selection: Some(SelectionRect {
                    row_start: 3,
                    row_end: 5,
                    track_start: 2,
                    track_end: 4,
                }),
                is_playing: true,
                playhead_row: Some(4),
                midi_status: "MIDI Connected 0",
                ..test_state()
            },
            140,
            36,
        ),
    );
}

#[test]
fn snapshots_sequence_view() {
    assert_snapshot(
        "sequence-view",
        render_snapshot(
            Song::empty(),
            TuiState {
                active_view: TuiView::Sequence,
                mode_label: "SEQUENCE",
                sequence_position: Some(0),
                ..test_state()
            },
            72,
            24,
        ),
    );
}

#[test]
fn snapshots_tracks_view() {
    let mut song = Song::empty();
    let sample = song.upsert_sample_reference("samples/bass.wav", "bass.wav");
    song.assign_sample_to_track(song.tracks[1].id, sample)
        .expect("assign sample");
    song.set_track_mixer_gain(1, 0.5).expect("set gain");
    song.set_track_mixer_pan(1, -0.25).expect("set pan");

    assert_snapshot(
        "tracks-view",
        render_snapshot(
            song,
            TuiState {
                cursor: Cursor {
                    track: 1,
                    ..Cursor::new()
                },
                active_view: TuiView::Tracks,
                mode_label: "TRACKS",
                ..test_state()
            },
            72,
            24,
        ),
    );
}

#[test]
fn snapshots_patterns_view() {
    let mut song = Song::empty();
    song.create_pattern(128);

    assert_snapshot(
        "patterns-view",
        render_snapshot(
            song,
            TuiState {
                pattern_index: 1,
                active_view: TuiView::Patterns,
                mode_label: "PATTERNS",
                ..test_state()
            },
            72,
            24,
        ),
    );
}

#[test]
fn snapshots_renoise_demo_project_browser() {
    let entries = [
        ProjectBrowserEntryView {
            name: "Samples",
            path: "fixtures/local/renoise-demos/Samples",
            kind: ProjectBrowserEntryKind::Directory,
            detail: "Renoise demo sample payloads stay local and ignored",
        },
        ProjectBrowserEntryView {
            name: "DemoSong - Daed - Bears.salieri",
            path: "fixtures/local/renoise-demos/Songs/DemoSong - Daed - Bears.salieri",
            kind: ProjectBrowserEntryKind::Project,
            detail: "DemoSong | imported local Renoise demo | samples external",
        },
        ProjectBrowserEntryView {
            name: "Tutorial - Beat Synced Wobbles.salieri",
            path: "fixtures/local/renoise-demos/Tutorial/Tutorial - Beat Synced Wobbles.salieri",
            kind: ProjectBrowserEntryKind::Project,
            detail: "Tutorial | imported local Renoise demo | samples external",
        },
        ProjectBrowserEntryView {
            name: "Instruments",
            path: "fixtures/local/renoise-demos/Instruments",
            kind: ProjectBrowserEntryKind::Directory,
            detail: "Imported Renoise instruments stay local and ignored",
        },
    ];

    assert_snapshot(
        "renoise-demo-browser",
        render_snapshot(
            Song::empty(),
            TuiState {
                active_view: TuiView::ProjectBrowser,
                mode_label: "PROJECTS",
                project_browser: Some(ProjectBrowserViewState {
                    current_dir: "fixtures/local/renoise-demos",
                    entries: &entries,
                    selected: 1,
                    message: Some("Local Renoise demos are optional and ignored by Git"),
                }),
                ..test_state()
            },
            120,
            28,
        ),
    );
}

#[test]
fn snapshots_sampler_view() {
    let overview = waveform_overview(vec![
        WaveformBucket {
            min: -0.2,
            max: 0.8,
        },
        WaveformBucket {
            min: -0.4,
            max: 0.6,
        },
        WaveformBucket {
            min: -0.8,
            max: 0.3,
        },
        WaveformBucket {
            min: -0.5,
            max: 0.9,
        },
    ]);

    assert_snapshot(
        "sampler-view",
        render_snapshot(
            Song::empty(),
            TuiState {
                active_view: TuiView::Sampler,
                mode_label: "SAMPLER",
                sampler_view: Some(SamplerViewState {
                    name: "break.wav",
                    source_path: "/samples/drums/break.wav",
                    overview: &overview,
                    gain: 0.85,
                    waveform_start_bucket: 0,
                    waveform_end_bucket: overview.buckets.len(),
                    waveform_zoom: 1,
                    instrument: Some("Break"),
                    assigned_track: Some("Drums"),
                    assigned_track_count: 1,
                    playback_mode: "loop",
                    start_frame: Some(10),
                    end_frame: Some(1_000),
                    loop_start_frame: Some(100),
                    loop_end_frame: Some(900),
                    envelope: (0.010, 0.050, 0.750, 0.100),
                    selected_envelope: salieri_tui::SamplerEnvelopeField::Attack,
                }),
                ..test_state()
            },
            100,
            28,
        ),
    );
}

#[test]
fn snapshots_dsp_rack_empty_view() {
    let song = Song::empty();
    assert_snapshot(
        "dsp-rack-empty",
        render_snapshot(
            song,
            TuiState {
                active_view: TuiView::DspRack,
                mode_label: "DSP",
                dsp_rack: Some(DspRackViewState {
                    track_name: "Drums",
                    track_number: 1,
                    track_effects: &[],
                    master_effects: &[],
                    selected_target: DspRackTargetView::Track,
                    selected_index: 0,
                    selected_parameter_index: 0,
                    selected_lock_status: DspParameterLockStatusView::Unlocked,
                    device_palette: None,
                }),
                ..test_state()
            },
            100,
            28,
        ),
    );
}

#[test]
fn snapshots_dsp_rack_populated_view() {
    let track_effects = vec![
        EffectDevice::gain(1, 0.5),
        EffectDevice::delay(
            7,
            DelaySpec {
                sync: false,
                time_left_ms: 250.0,
                time_right_ms: 500.0,
                feedback: 0.35,
                ping_pong: true,
                mix: 0.25,
                ..DelaySpec::default()
            },
        ),
    ];
    let master_effects = vec![EffectDevice::reverb(8, Default::default())];
    assert_snapshot(
        "dsp-rack-populated",
        render_snapshot(
            Song::empty(),
            TuiState {
                active_view: TuiView::DspRack,
                mode_label: "DSP",
                dsp_rack: Some(DspRackViewState {
                    track_name: "Drums",
                    track_number: 1,
                    track_effects: track_effects.as_slice(),
                    master_effects: master_effects.as_slice(),
                    selected_target: DspRackTargetView::Track,
                    selected_index: 1,
                    selected_parameter_index: 0,
                    selected_lock_status: DspParameterLockStatusView::Unlocked,
                    device_palette: None,
                }),
                ..test_state()
            },
            100,
            28,
        ),
    );
}

#[test]
fn snapshots_dsp_rack_device_palette() {
    let track_effects = vec![EffectDevice::gain(1, 0.5)];
    let entries = [
        DspDevicePaletteEntryView {
            label: "Gain",
            summary: "utility level",
        },
        DspDevicePaletteEntryView {
            label: "Pan",
            summary: "left/right placement",
        },
        DspDevicePaletteEntryView {
            label: "Filter",
            summary: "multimode filter",
        },
        DspDevicePaletteEntryView {
            label: "Reverb",
            summary: "room/space",
        },
    ];
    assert_snapshot(
        "dsp-rack-device-palette",
        render_snapshot(
            Song::empty(),
            TuiState {
                active_view: TuiView::DspRack,
                mode_label: "DSP",
                dsp_rack: Some(DspRackViewState {
                    track_name: "Drums",
                    track_number: 1,
                    track_effects: track_effects.as_slice(),
                    master_effects: &[],
                    selected_target: DspRackTargetView::Master,
                    selected_index: 0,
                    selected_parameter_index: 0,
                    selected_lock_status: DspParameterLockStatusView::Unlocked,
                    device_palette: Some(DspDevicePaletteViewState {
                        entries: &entries,
                        selected: 2,
                    }),
                }),
                ..test_state()
            },
            100,
            28,
        ),
    );
}

#[test]
fn snapshots_large_sampler_workspace() {
    let overview = waveform_overview(vec![
        WaveformBucket {
            min: -0.2,
            max: 0.8,
        },
        WaveformBucket {
            min: -0.4,
            max: 0.6,
        },
        WaveformBucket {
            min: -0.8,
            max: 0.3,
        },
        WaveformBucket {
            min: -0.5,
            max: 0.9,
        },
    ]);

    assert_snapshot(
        "sampler-large",
        render_snapshot(
            Song::empty(),
            TuiState {
                active_view: TuiView::Sampler,
                mode_label: "SAMPLER",
                sampler_view: Some(SamplerViewState {
                    name: "choired_B",
                    source_path: "~/Music/DemoSong/Samples/choired_B.flac",
                    overview: &overview,
                    gain: 1.0,
                    waveform_start_bucket: 0,
                    waveform_end_bucket: overview.buckets.len(),
                    waveform_zoom: 1,
                    instrument: Some("DemoSong"),
                    assigned_track: Some("Track 01"),
                    assigned_track_count: 1,
                    playback_mode: "one-shot",
                    start_frame: None,
                    end_frame: None,
                    loop_start_frame: None,
                    loop_end_frame: None,
                    envelope: (0.010, 0.050, 0.750, 0.100),
                    selected_envelope: salieri_tui::SamplerEnvelopeField::Attack,
                }),
                ..test_state()
            },
            140,
            36,
        ),
    );
}

#[test]
fn snapshots_empty_waveform() {
    assert_snapshot(
        "waveform-empty",
        render_waveform_snapshot(WaveformOverview {
            sample_rate: 44_100,
            channels: 1,
            frames: 0,
            duration_seconds: 0.0,
            buckets: Vec::new(),
        }),
    );
}

#[test]
fn snapshots_quiet_waveform() {
    assert_snapshot(
        "waveform-quiet",
        render_waveform_snapshot(waveform_overview(vec![WaveformBucket {
            min: 0.0,
            max: 0.0,
        }])),
    );
}

#[test]
fn snapshots_loud_waveform() {
    assert_snapshot(
        "waveform-loud",
        render_waveform_snapshot(waveform_overview(vec![
            WaveformBucket {
                min: -1.0,
                max: 1.0,
            },
            WaveformBucket {
                min: -0.8,
                max: 0.9,
            },
            WaveformBucket {
                min: -1.0,
                max: 0.7,
            },
            WaveformBucket {
                min: -0.9,
                max: 1.0,
            },
        ])),
    );
}

#[test]
fn snapshots_asymmetric_waveform() {
    assert_snapshot(
        "waveform-asymmetric",
        render_waveform_snapshot(waveform_overview(vec![
            WaveformBucket {
                min: -0.15,
                max: 0.75,
            },
            WaveformBucket {
                min: -0.10,
                max: 0.60,
            },
            WaveformBucket {
                min: -0.25,
                max: 0.35,
            },
            WaveformBucket {
                min: -0.40,
                max: 0.20,
            },
        ])),
    );
}

#[test]
fn snapshots_clipped_looking_waveform() {
    assert_snapshot(
        "waveform-clipped",
        render_waveform_snapshot(waveform_overview(vec![
            WaveformBucket {
                min: -1.0,
                max: 1.0,
            },
            WaveformBucket {
                min: -1.0,
                max: 1.0,
            },
            WaveformBucket {
                min: -1.0,
                max: 1.0,
            },
            WaveformBucket {
                min: -1.0,
                max: 1.0,
            },
        ])),
    );
}

fn large_tracker_song(row_count: usize, track_count: usize) -> Song {
    let mut song = Song::empty();
    while song.tracks.len() < track_count {
        song.create_track();
    }
    for index in 0..song.tracks.len() {
        song.rename_track(index, format!("Track {:02}", index + 1))
            .expect("rename track");
    }
    song.resize_pattern(0, row_count).expect("resize pattern");
    let pattern = song.current_pattern_mut().expect("pattern");
    pattern
        .set_note(4, 3, NoteEvent::Note { pitch: 67 }, 0x64)
        .expect("partial fixture note");
    pattern
        .set_note(
            4094.min(row_count.saturating_sub(1)),
            10.min(track_count.saturating_sub(1)),
            NoteEvent::Note { pitch: 72 },
            0x7f,
        )
        .expect("boundary fixture note");
    song
}
