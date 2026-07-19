use super::*;
use ratatui::{backend::TestBackend, Terminal};
use salieri_core::Song;

#[test]
fn renders_help_overlay_when_requested() {
    let song = Song::empty();
    let backend = TestBackend::new(100, 32);
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
                    mode_label: "HELP",
                    octave: 4,
                    dirty: false,
                    show_line_numbers_hex: false,
                    command_line: None,
                    notification: None,
                    show_help: true,
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
                    sampler_view: None,
                    sample_browser: None,
                    project_browser: None,
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

    assert!(rendered.contains("Help"));
    assert!(rendered.contains("Global"));
    assert!(rendered.contains("MIDI"));
    assert!(rendered.contains("Commands"));
    assert!(rendered.contains("Navigation"));
    assert!(rendered.contains("Tab/Right next page"));
}

#[test]
fn renders_playhead_when_playing() {
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
                    selection: Some(SelectionRect {
                        row_start: 0,
                        row_end: 1,
                        track_start: 0,
                        track_end: 1,
                    }),
                    mode_label: "NORMAL",
                    octave: 4,
                    dirty: false,
                    show_line_numbers_hex: false,
                    command_line: None,
                    notification: None,
                    show_help: false,
                    help_scroll: 0,
                    help_tab: HelpTab::Basics,
                    is_playing: true,
                    loop_pattern: true,
                    playhead_row: Some(0),
                    midi_status: "MIDI Connected 0",
                    sequence_position: Some(0),
                    quit_confirmation: false,
                    delete_confirmation: None,
                    midi_settings: None,
                    sampler_view: None,
                    sample_browser: None,
                    project_browser: None,
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

    assert!(rendered.contains("PLAY"));
    assert!(rendered.contains("SEL"));
    assert!(rendered.contains(">00"));
    assert!(rendered.contains("MIDI Connected 0"));
}

#[test]
fn renders_hex_line_numbers_when_enabled() {
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
                    row_offset: 8,
                    track_offset: 0,
                    pattern_index: 0,
                    active_view: TuiView::Pattern,
                    selection: None,
                    mode_label: "NORMAL",
                    octave: 4,
                    dirty: false,
                    show_line_numbers_hex: true,
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
                    sampler_view: None,
                    sample_browser: None,
                    project_browser: None,
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

    assert!(rendered.contains(" 0A"));
}

#[test]
fn renders_status_notification() {
    let song = Song::empty();
    let backend = TestBackend::new(100, 32);
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
                    dirty: false,
                    show_line_numbers_hex: false,
                    command_line: None,
                    notification: Some(NotificationView {
                        kind: NotificationKind::Success,
                        message: "Project saved",
                    }),
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
                    sampler_view: None,
                    sample_browser: None,
                    project_browser: None,
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

    assert!(rendered.contains("OK"));
    assert!(rendered.contains("Project saved"));
}

#[test]
fn renders_quit_confirmation_overlay() {
    let song = Song::empty();
    let backend = TestBackend::new(100, 32);
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
                    mode_label: "DIALOG",
                    octave: 4,
                    dirty: true,
                    show_line_numbers_hex: false,
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
                    quit_confirmation: true,
                    delete_confirmation: None,
                    midi_settings: None,
                    sampler_view: None,
                    sample_browser: None,
                    project_browser: None,
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

    assert!(rendered.contains("Unsaved changes"));
    assert!(rendered.contains("[Y]es"));
}

#[test]
fn renders_delete_confirmation_overlay() {
    let song = Song::empty();
    let backend = TestBackend::new(100, 32);
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
                    mode_label: "DIALOG",
                    octave: 4,
                    dirty: false,
                    show_line_numbers_hex: false,
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
                    delete_confirmation: Some("Delete track 02 Bass?"),
                    midi_settings: None,
                    sampler_view: None,
                    sample_browser: None,
                    project_browser: None,
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

    assert!(rendered.contains("Confirm"));
    assert!(rendered.contains("Delete track 02 Bass?"));
}

#[test]
fn renders_midi_settings_overlay() {
    let song = Song::empty();
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");
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
                    mode_label: "MIDI",
                    octave: 4,
                    dirty: false,
                    show_line_numbers_hex: false,
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
                    midi_settings: Some(MidiSettingsState {
                        ports: &ports,
                        selected_port: 1,
                        status: "MIDI Disconnected",
                    }),
                    sampler_view: None,
                    sample_browser: None,
                    project_browser: None,
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

    assert!(rendered.contains("MIDI Settings"));
    assert!(rendered.contains("IAC Driver Bus 1"));
    assert!(rendered.contains("External Synth"));
    assert!(rendered.contains("Enter connect selected"));
}
