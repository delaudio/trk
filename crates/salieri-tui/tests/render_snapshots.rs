use std::{fs, path::PathBuf};

use ratatui::{backend::TestBackend, Terminal};
use salieri_core::{Cursor, NoteEvent, Song};
use salieri_sampler::{WaveformBucket, WaveformOverview};
use salieri_tui::{
    render, render_waveform_overview, MidiPortView, MidiSettingsState, SamplerViewState,
    SelectionRect, TuiState, TuiView,
};

#[test]
fn snapshots_empty_pattern_editor() {
    assert_snapshot(
        "empty-pattern",
        render_snapshot(
            Song::empty(),
            TuiState {
                cursor: Cursor::new(),
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Pattern,
                selection: None,
                mode_label: "NORMAL",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: None,
                sample_browser: None,
            },
            100,
            28,
        ),
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
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Pattern,
                selection: None,
                mode_label: "NORMAL",
                octave: 4,
                dirty: true,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: true,
                loop_pattern: true,
                playhead_row: Some(3),
                midi_status: "MIDI Connected 0",
                sequence_position: Some(0),
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: None,
                sample_browser: None,
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
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Pattern,
                selection: Some(SelectionRect {
                    row_start: 2,
                    row_end: 5,
                    track_start: 0,
                    track_end: 2,
                }),
                mode_label: "NORMAL",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: None,
                sample_browser: None,
            },
            100,
            28,
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
                cursor: Cursor::new(),
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Pattern,
                selection: None,
                mode_label: "HELP",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: true,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: None,
                sample_browser: None,
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

    assert_snapshot(
        "midi-settings",
        render_snapshot(
            Song::empty(),
            TuiState {
                cursor: Cursor::new(),
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Pattern,
                selection: None,
                mode_label: "MIDI",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: Some(MidiSettingsState {
                    ports: &ports,
                    selected_port: 1,
                    status: "MIDI Disconnected",
                }),
                sampler_view: None,
                sample_browser: None,
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
        render_snapshot(
            Song::empty(),
            TuiState {
                cursor: Cursor::new(),
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Pattern,
                selection: None,
                mode_label: "NORMAL",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: None,
                sample_browser: None,
            },
            72,
            24,
        ),
    );
}

#[test]
fn snapshots_medium_layout() {
    assert_snapshot(
        "responsive-medium",
        render_snapshot(
            Song::empty(),
            TuiState {
                cursor: Cursor::new(),
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Pattern,
                selection: None,
                mode_label: "NORMAL",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: None,
                sample_browser: None,
            },
            100,
            28,
        ),
    );
}

#[test]
fn snapshots_large_layout() {
    assert_snapshot(
        "responsive-large",
        render_snapshot(
            Song::empty(),
            TuiState {
                cursor: Cursor::new(),
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Pattern,
                selection: None,
                mode_label: "NORMAL",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: None,
                sample_browser: None,
            },
            140,
            32,
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
                cursor: Cursor::new(),
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Sequence,
                selection: None,
                mode_label: "SEQUENCE",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: Some(0),
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: None,
                sample_browser: None,
            },
            72,
            24,
        ),
    );
}

#[test]
fn snapshots_tracks_view() {
    assert_snapshot(
        "tracks-view",
        render_snapshot(
            Song::empty(),
            TuiState {
                cursor: Cursor {
                    track: 1,
                    ..Cursor::new()
                },
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Tracks,
                selection: None,
                mode_label: "TRACKS",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: None,
                sample_browser: None,
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
                cursor: Cursor::new(),
                row_offset: 0,
                pattern_index: 1,
                active_view: TuiView::Patterns,
                selection: None,
                mode_label: "PATTERNS",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: None,
                sample_browser: None,
            },
            72,
            24,
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
                cursor: Cursor::new(),
                row_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Sampler,
                selection: None,
                mode_label: "SAMPLER",
                octave: 4,
                dirty: false,
                show_line_numbers_hex: false,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                sampler_view: Some(SamplerViewState {
                    name: "break.wav",
                    source_path: "/samples/drums/break.wav",
                    overview: &overview,
                    instrument: Some("Break"),
                    assigned_track: Some("Drums"),
                    assigned_track_count: 1,
                    playback_mode: "loop",
                    start_frame: Some(10),
                    end_frame: Some(1_000),
                    loop_start_frame: Some(100),
                    loop_end_frame: Some(900),
                    envelope: (0.010, 0.050, 0.750, 0.100),
                }),
                sample_browser: None,
            },
            100,
            28,
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

fn render_snapshot(song: Song, state: TuiState<'_>, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| render(frame, &song, state))
        .expect("draw");

    let buffer = terminal.backend().buffer();
    let mut output = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            output.push_str(buffer[(x, y)].symbol());
        }
        output.push('\n');
    }
    output
}

fn render_waveform_snapshot(overview: WaveformOverview) -> String {
    let backend = TestBackend::new(42, 10);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| render_waveform_overview(frame, frame.area(), &overview))
        .expect("draw");

    let buffer = terminal.backend().buffer();
    let mut output = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            output.push_str(buffer[(x, y)].symbol());
        }
        output.push('\n');
    }
    output
}

fn waveform_overview(buckets: Vec<WaveformBucket>) -> WaveformOverview {
    WaveformOverview {
        sample_rate: 44_100,
        channels: 2,
        frames: 88_200,
        duration_seconds: 2.0,
        buckets,
    }
}

fn assert_snapshot(name: &str, actual: String) {
    let path = snapshot_path(name);
    if std::env::var_os("UPDATE_SALIERI_SNAPSHOTS").is_some() {
        fs::write(&path, actual).expect("write snapshot");
        return;
    }

    let expected = fs::read_to_string(&path).expect("read snapshot");
    assert_eq!(actual, expected, "snapshot mismatch for {name}");
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(format!("{name}.snap"))
}
