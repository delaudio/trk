use super::*;
use ratatui::{backend::TestBackend, Terminal};
use trk_core::{InstrumentId, NoteEvent, Song, TrackerCommand};

use super::render_test_support::*;

#[test]
fn renders_default_pattern_without_panic() {
    let song = Song::empty();
    let backend = TestBackend::new(160, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            render(
                frame,
                &song,
                TuiState {
                    cursor: Cursor::new(),
                    row_offset: 0,
                    track_offset: 0,
                    pattern_index: 0,
                    active_view: TuiView::Pattern,
                    selection: None,
                    mode_label: "NORMAL",
                    octave: 4,
                    edit_step: 1,
                    dirty: false,
                    show_line_numbers_hex: false,
                    row_number_offset: 0,
                    pattern_divider_interval: 4,
                    pattern_highlight_interval: 16,
                    show_pattern_top_info: true,
                    command_line: None,
                    notification: None,
                    show_help: false,
                    help_scroll: 0,
                    help_tab: HelpTab::Basics,
                    is_playing: false,
                    loop_pattern: true,
                    playhead_row: None,
                    midi_status: "MIDI Disconnected",
                    sequence_position: None,
                    quit_confirmation: false,
                    delete_confirmation: None,
                    midi_settings: None,
                    command_palette: None,
                    sampler_view: None,
                    dsp_rack: None,
                    sample_browser: None,
                    project_browser: None,
                    ai_chat: None,
                    variation_history: None,
                    tracker_layout: crate::TrackerLayoutState::default(),
                },
            );
        })
        .expect("draw");

    let buffer = terminal.backend().buffer();
    let rendered = buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("trk"));
    assert!(rendered.contains("Pattern Editor"));
    assert!(rendered.contains("Drums"));
    assert!(rendered.contains("Bass"));
}

#[test]
fn pattern_viewport_derives_visible_ranges_from_offsets_and_cursor() {
    let viewport = pattern_viewport(
        Rect::new(0, 0, 50, 8),
        512,
        12,
        TuiState {
            cursor: Cursor {
                row: 120,
                track: 5,
                ..Cursor::new()
            },
            row_offset: 100,
            track_offset: 4,
            pattern_index: 0,
            active_view: TuiView::Pattern,
            selection: None,
            mode_label: "NORMAL",
            octave: 4,
            edit_step: 1,
            dirty: false,
            show_line_numbers_hex: false,
            row_number_offset: 0,
            pattern_divider_interval: 4,
            pattern_highlight_interval: 16,
            show_pattern_top_info: true,
            command_line: None,
            notification: None,
            show_help: false,
            help_scroll: 0,
            help_tab: HelpTab::Basics,
            is_playing: false,
            loop_pattern: true,
            playhead_row: None,
            midi_status: "MIDI Disconnected",
            sequence_position: None,
            quit_confirmation: false,
            delete_confirmation: None,
            midi_settings: None,
            command_palette: None,
            sampler_view: None,
            dsp_rack: None,
            sample_browser: None,
            project_browser: None,
            ai_chat: None,
            variation_history: None,
            tracker_layout: crate::TrackerLayoutState::default(),
        },
    );

    assert_eq!(viewport.visible_rows, 116..121);
    assert_eq!(viewport.visible_tracks, 4..6);
}

#[test]
fn visible_pattern_tracks_includes_partially_visible_cells() {
    let two_full_tracks_plus_one_column =
        2 + ROW_GUTTER_WIDTH as u16 + (PATTERN_CELL_WIDTH as u16 * 2) + 1;

    assert_eq!(
        visible_pattern_tracks(two_full_tracks_plus_one_column, PatternFieldLayout::Full),
        3
    );
    assert!(
        visible_pattern_tracks(80, PatternFieldLayout::Note)
            > visible_pattern_tracks(80, PatternFieldLayout::Full)
    );
}

#[test]
fn focused_pattern_layout_renders_more_tracks_with_selected_fields() {
    let mut song = long_track_song(12);
    for index in 0..song.tracks.len() {
        song.rename_track(index, format!("T{:02}", index + 1))
            .expect("rename track");
    }
    song.pattern_mut(0)
        .expect("pattern")
        .cell_mut(0, 0)
        .expect("cell")
        .instrument = Some(InstrumentId(0xab));
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            let tracker_layout = crate::TrackerLayoutState {
                pattern_fields: PatternFieldLayout::Note,
                ..Default::default()
            };
            render_pattern(
                frame,
                Rect::new(0, 0, 80, 12),
                &song,
                TuiState {
                    cursor: Cursor::new(),
                    row_offset: 0,
                    track_offset: 0,
                    pattern_index: 0,
                    active_view: TuiView::Pattern,
                    selection: None,
                    mode_label: "NORMAL",
                    octave: 4,
                    edit_step: 1,
                    dirty: false,
                    show_line_numbers_hex: false,
                    row_number_offset: 0,
                    pattern_divider_interval: 4,
                    pattern_highlight_interval: 16,
                    show_pattern_top_info: true,
                    command_line: None,
                    notification: None,
                    show_help: false,
                    help_scroll: 0,
                    help_tab: HelpTab::Basics,
                    is_playing: false,
                    loop_pattern: true,
                    playhead_row: None,
                    midi_status: "MIDI Disconnected",
                    sequence_position: None,
                    quit_confirmation: false,
                    delete_confirmation: None,
                    midi_settings: None,
                    command_palette: None,
                    sampler_view: None,
                    dsp_rack: None,
                    sample_browser: None,
                    project_browser: None,
                    ai_chat: None,
                    variation_history: None,
                    tracker_layout,
                },
            );
        })
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);
    assert!(rendered.contains("fields=note"));
    assert!(rendered.contains("T11"));
    assert!(!rendered.contains(" AB "));
}

#[test]
fn parameter_controls_use_descriptor_metadata_and_validation() {
    let pan = parameter_control_from_f32(trk_core::mixer_track_pan_descriptor(), -0.5);
    let rendered = line_text(&pan);

    assert!(rendered.contains("Pan"));
    assert!(rendered.contains("L50"));
    assert!(rendered.contains("auto"));

    let descriptor = trk_core::mixer_track_gain_descriptor();
    let invalid = parameter_control_line(&descriptor, trk_core::ParameterValue::Float(f32::NAN));

    assert!(line_text(&invalid).contains("invalid"));
}

#[test]
fn track_desk_renders_sampler_mixer_and_native_effect_parameters_from_descriptors() {
    let mut song = Song::empty();
    let sample = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
    song.samples
        .iter_mut()
        .find(|reference| reference.id == sample)
        .expect("sample reference")
        .gain = 0.5;
    song.assign_sample_to_track(song.tracks[0].id, sample)
        .expect("assign sample");
    song.set_track_mixer_gain(0, 0.625).expect("mixer gain");
    song.set_track_mixer_pan(0, -0.5).expect("mixer pan");
    song.mixer.tracks[0]
        .effects
        .push(trk_core::EffectDevice::gain(1, 0.75));

    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            let state = TuiState {
                cursor: Cursor::new(),
                row_offset: 0,
                track_offset: 0,
                pattern_index: 0,
                active_view: TuiView::Pattern,
                selection: None,
                mode_label: "NORMAL",
                octave: 4,
                edit_step: 1,
                dirty: false,
                show_line_numbers_hex: false,
                row_number_offset: 0,
                pattern_divider_interval: 4,
                pattern_highlight_interval: 16,
                show_pattern_top_info: true,
                command_line: None,
                notification: None,
                show_help: false,
                help_scroll: 0,
                help_tab: HelpTab::Basics,
                is_playing: false,
                loop_pattern: true,
                playhead_row: None,
                midi_status: "MIDI Disconnected",
                sequence_position: None,
                quit_confirmation: false,
                delete_confirmation: None,
                midi_settings: None,
                command_palette: None,
                sampler_view: None,
                dsp_rack: None,
                sample_browser: None,
                project_browser: None,
                ai_chat: None,
                variation_history: None,
                tracker_layout: crate::TrackerLayoutState::default(),
            };
            render_track_properties(frame, Rect::new(0, 0, 100, 12), &song, state);
            render_selected_track_inspector(frame, Rect::new(0, 12, 100, 12), &song, state);
        })
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);
    let sample_gain = trk_core::sample_gain_descriptor();
    let mixer_gain = trk_core::mixer_track_gain_descriptor();
    let mixer_pan = trk_core::mixer_track_pan_descriptor();
    let native_gain = trk_core::native_gain_descriptor();

    assert!(rendered.contains(&sample_gain.format_value(&sample_gain.value_from_f32(0.5))));
    assert!(rendered.contains(&mixer_gain.format_value(&mixer_gain.value_from_f32(0.625))));
    assert!(rendered.contains(&mixer_pan.format_value(&mixer_pan.value_from_f32(-0.5))));
    assert!(rendered.contains(&native_gain.format_value(&native_gain.value_from_f32(0.75))));
}

#[test]
fn virtualized_pattern_render_omits_offscreen_rows_and_tracks() {
    let mut song = long_track_song(12);
    song.resize_pattern(0, 4_096).expect("large pattern");
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            render_pattern(
                frame,
                Rect::new(0, 0, 80, 12),
                &song,
                TuiState {
                    cursor: Cursor {
                        row: 1003,
                        track: 6,
                        ..Cursor::new()
                    },
                    row_offset: 1000,
                    track_offset: 5,
                    pattern_index: 0,
                    active_view: TuiView::Pattern,
                    selection: None,
                    mode_label: "NORMAL",
                    octave: 4,
                    edit_step: 1,
                    dirty: false,
                    show_line_numbers_hex: false,
                    row_number_offset: 0,
                    pattern_divider_interval: 4,
                    pattern_highlight_interval: 16,
                    show_pattern_top_info: true,
                    command_line: None,
                    notification: None,
                    show_help: false,
                    help_scroll: 0,
                    help_tab: HelpTab::Basics,
                    is_playing: false,
                    loop_pattern: true,
                    playhead_row: None,
                    midi_status: "MIDI Disconnected",
                    sequence_position: None,
                    quit_confirmation: false,
                    delete_confirmation: None,
                    midi_settings: None,
                    command_palette: None,
                    sampler_view: None,
                    dsp_rack: None,
                    sample_browser: None,
                    project_browser: None,
                    ai_chat: None,
                    variation_history: None,
                    tracker_layout: crate::TrackerLayoutState::default(),
                },
            );
        })
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);

    assert!(rendered.contains("1003"));
    assert!(rendered.contains("Track 06"));
    assert!(rendered.contains("Track 07"));
    assert!(!rendered.contains(" 000 "));
    assert!(!rendered.contains("4095"));
    assert!(!rendered.contains("Track 01"));
    assert!(!rendered.contains("Track 12"));
}

#[test]
fn tracks_panel_scrolls_to_active_track() {
    let song = long_track_song(30);
    let backend = TestBackend::new(32, 8);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            render_tracks(frame, Rect::new(0, 0, 32, 8), &song, 20, &mut interactions);
        })
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);

    assert!(rendered.contains("Tracks 18-23 / 30"));
    assert!(rendered.contains("> 21 Track 21"));
    assert!(!rendered.contains(" 01 Track 01"));
}

#[test]
fn renders_tracker_cell_subcolumns() {
    let mut song = Song::empty();
    let pattern = song.current_pattern_mut().expect("pattern");
    pattern
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x64)
        .expect("note");
    let cell = pattern.cell_mut(0, 0).expect("cell");
    cell.instrument = Some(InstrumentId(1));
    cell.volume = Some(0x40);
    cell.pan = Some(0x7f);
    cell.delay = Some(0x20);
    cell.command = Some(TrackerCommand::retrigger(4));

    let backend = TestBackend::new(180, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            render(
                frame,
                &song,
                TuiState {
                    cursor: Cursor::new(),
                    row_offset: 0,
                    track_offset: 0,
                    pattern_index: 0,
                    active_view: TuiView::Pattern,
                    selection: None,
                    mode_label: "NORMAL",
                    octave: 4,
                    edit_step: 1,
                    dirty: false,
                    show_line_numbers_hex: false,
                    row_number_offset: 0,
                    pattern_divider_interval: 4,
                    pattern_highlight_interval: 16,
                    show_pattern_top_info: true,
                    command_line: None,
                    notification: None,
                    show_help: false,
                    help_scroll: 0,
                    help_tab: HelpTab::Basics,
                    is_playing: false,
                    loop_pattern: true,
                    playhead_row: None,
                    midi_status: "MIDI Disconnected",
                    sequence_position: None,
                    quit_confirmation: false,
                    delete_confirmation: None,
                    midi_settings: None,
                    command_palette: None,
                    sampler_view: None,
                    dsp_rack: None,
                    sample_browser: None,
                    project_browser: None,
                    ai_chat: None,
                    variation_history: None,
                    tracker_layout: crate::TrackerLayoutState::default(),
                },
            );
        })
        .expect("draw");

    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("C-4 64 01 40 7F 20 R04"));
}

#[test]
fn renders_sample_browser_view() {
    let song = Song::empty();
    let overview = test_waveform(vec![
        trk_sampler::WaveformBucket {
            min: -0.4,
            max: 0.6,
        },
        trk_sampler::WaveformBucket {
            min: -0.2,
            max: 0.2,
        },
    ]);
    let entries = [
        SampleBrowserEntryView {
            name: "Drums",
            kind: SampleBrowserEntryKind::Directory,
        },
        SampleBrowserEntryView {
            name: "kick.wav",
            kind: SampleBrowserEntryKind::SupportedSample,
        },
    ];
    let backend = TestBackend::new(100, 28);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            render(
                frame,
                &song,
                TuiState {
                    cursor: Cursor::new(),
                    row_offset: 0,
                    track_offset: 0,
                    pattern_index: 0,
                    active_view: TuiView::SampleBrowser,
                    selection: None,
                    mode_label: "SAMPLES",
                    octave: 4,
                    edit_step: 1,
                    dirty: false,
                    show_line_numbers_hex: false,
                    row_number_offset: 0,
                    pattern_divider_interval: 4,
                    pattern_highlight_interval: 16,
                    show_pattern_top_info: true,
                    command_line: None,
                    notification: None,
                    show_help: false,
                    help_scroll: 0,
                    help_tab: HelpTab::Basics,
                    is_playing: false,
                    loop_pattern: true,
                    playhead_row: None,
                    midi_status: "MIDI Disconnected",
                    sequence_position: None,
                    quit_confirmation: false,
                    delete_confirmation: None,
                    midi_settings: None,
                    command_palette: None,
                    sampler_view: None,
                    dsp_rack: None,
                    sample_browser: Some(SampleBrowserViewState {
                        current_dir: "/tmp/samples",
                        entries: &entries,
                        selected: 1,
                        preview: Some(SamplerViewState {
                            color_mode: TerminalColorMode::TrueColor,
                            name: "kick.wav",
                            source_path: "/tmp/samples/kick.wav",
                            overview: &overview,
                            gain: 1.0,
                            waveform_start_bucket: 0,
                            waveform_end_bucket: overview.buckets.len(),
                            waveform_zoom: 1,
                            instrument: None,
                            assigned_track: None,
                            assigned_track_count: 0,
                            playback_mode: "one-shot",
                            start_frame: None,
                            end_frame: None,
                            loop_start_frame: None,
                            loop_end_frame: None,
                            envelope: (0.0, 0.0, 1.0, 0.0),
                            selected_envelope: SamplerEnvelopeField::Attack,
                        }),
                        message: None,
                    }),
                    project_browser: None,
                    ai_chat: None,
                    variation_history: None,
                    tracker_layout: crate::TrackerLayoutState::default(),
                },
            );
        })
        .expect("draw");

    let rendered = terminal.backend().to_string();
    assert!(rendered.contains("kick.wav"));
    assert!(rendered.contains("Sample Metadata"));
}

#[test]
fn renders_project_browser_view() {
    let song = Song::empty();
    let entries = [
        ProjectBrowserEntryView {
            name: "songs",
            path: "/tmp/songs",
            kind: ProjectBrowserEntryKind::Directory,
            detail: "Press Enter to open directory",
        },
        ProjectBrowserEntryView {
            name: "set.trk",
            path: "/tmp/songs/set.trk",
            kind: ProjectBrowserEntryKind::Project,
            detail: "Set | 4 tracks | 2 patterns | 2 sequence slots | modified unknown",
        },
    ];
    let backend = TestBackend::new(100, 28);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            render(
                frame,
                &song,
                TuiState {
                    cursor: Cursor::new(),
                    row_offset: 0,
                    track_offset: 0,
                    pattern_index: 0,
                    active_view: TuiView::ProjectBrowser,
                    selection: None,
                    mode_label: "PROJECTS",
                    octave: 4,
                    edit_step: 1,
                    dirty: false,
                    show_line_numbers_hex: false,
                    row_number_offset: 0,
                    pattern_divider_interval: 4,
                    pattern_highlight_interval: 16,
                    show_pattern_top_info: true,
                    command_line: None,
                    notification: None,
                    show_help: false,
                    help_scroll: 0,
                    help_tab: HelpTab::Basics,
                    is_playing: false,
                    loop_pattern: true,
                    playhead_row: None,
                    midi_status: "MIDI Disconnected",
                    sequence_position: None,
                    quit_confirmation: false,
                    delete_confirmation: None,
                    midi_settings: None,
                    command_palette: None,
                    sampler_view: None,
                    dsp_rack: None,
                    sample_browser: None,
                    project_browser: Some(ProjectBrowserViewState {
                        current_dir: "/tmp/songs",
                        entries: &entries,
                        selected: 1,
                        message: Some("Enter opens a project"),
                    }),
                    ai_chat: None,
                    variation_history: None,
                    tracker_layout: crate::TrackerLayoutState::default(),
                },
            );
        })
        .expect("draw");

    let rendered = terminal.backend().to_string();
    assert!(rendered.contains("Projects"));
    assert!(rendered.contains("set.trk"));
    assert!(rendered.contains("2 patterns"));
}

#[test]
fn renders_small_layout_as_single_pattern_view() {
    let song = Song::empty();
    let backend = TestBackend::new(72, 24);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            render(
                frame,
                &song,
                TuiState {
                    cursor: Cursor::new(),
                    row_offset: 0,
                    track_offset: 0,
                    pattern_index: 0,
                    active_view: TuiView::Pattern,
                    selection: None,
                    mode_label: "NORMAL",
                    octave: 4,
                    edit_step: 1,
                    dirty: false,
                    show_line_numbers_hex: false,
                    row_number_offset: 0,
                    pattern_divider_interval: 4,
                    pattern_highlight_interval: 16,
                    show_pattern_top_info: true,
                    command_line: None,
                    notification: None,
                    show_help: false,
                    help_scroll: 0,
                    help_tab: HelpTab::Basics,
                    is_playing: false,
                    loop_pattern: true,
                    playhead_row: None,
                    midi_status: "MIDI Disconnected",
                    sequence_position: None,
                    quit_confirmation: false,
                    delete_confirmation: None,
                    midi_settings: None,
                    command_palette: None,
                    sampler_view: None,
                    dsp_rack: None,
                    sample_browser: None,
                    project_browser: None,
                    ai_chat: None,
                    variation_history: None,
                    tracker_layout: crate::TrackerLayoutState::default(),
                },
            );
        })
        .expect("draw");

    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Pattern Editor"));
    assert!(!rendered.contains("Track Editor"));
    assert!(!rendered.contains("Sequence Editor"));
}

#[test]
fn renders_medium_layout_with_compact_side_panel() {
    let song = Song::empty();
    let backend = TestBackend::new(100, 28);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            render(
                frame,
                &song,
                TuiState {
                    cursor: Cursor::new(),
                    row_offset: 0,
                    track_offset: 0,
                    pattern_index: 0,
                    active_view: TuiView::Pattern,
                    selection: None,
                    mode_label: "NORMAL",
                    octave: 4,
                    edit_step: 1,
                    dirty: false,
                    show_line_numbers_hex: false,
                    row_number_offset: 0,
                    pattern_divider_interval: 4,
                    pattern_highlight_interval: 16,
                    show_pattern_top_info: true,
                    command_line: None,
                    notification: None,
                    show_help: false,
                    help_scroll: 0,
                    help_tab: HelpTab::Basics,
                    is_playing: false,
                    loop_pattern: true,
                    playhead_row: None,
                    midi_status: "MIDI Disconnected",
                    sequence_position: None,
                    quit_confirmation: false,
                    delete_confirmation: None,
                    midi_settings: None,
                    command_palette: None,
                    sampler_view: None,
                    dsp_rack: None,
                    sample_browser: None,
                    project_browser: None,
                    ai_chat: None,
                    variation_history: None,
                    tracker_layout: crate::TrackerLayoutState::default(),
                },
            );
        })
        .expect("draw");

    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Pattern Editor"));
    assert!(rendered.contains("Tracks"));
    assert!(rendered.contains("Song Slots"));
}
