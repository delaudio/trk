use super::*;
use ratatui::{backend::TestBackend, style::Color, Terminal};
use trk_core::{Cursor, Song};

#[test]
fn project_display_preferences_affect_pattern_rendering() {
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
                    row_number_offset: 1,
                    pattern_divider_interval: 0,
                    pattern_highlight_interval: 0,
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

    assert!(rendered.contains("Pattern Editor: Pattern 01"));
    assert!(rendered.contains("rows=64"));
    assert!(rendered.contains(" 01"));
}

#[test]
fn pattern_divider_and_highlight_intervals_style_row_gutter() {
    let state = PatternRowRenderState {
        cursor: Cursor::new(),
        playhead_row: None,
        selection: None,
        show_line_numbers_hex: false,
        row_number_offset: 0,
        pattern_divider_interval: 4,
        pattern_highlight_interval: 16,
        visible_tracks: 0..1,
        field_layout: PatternFieldLayout::Full,
    };

    assert_eq!(pattern_row_gutter_style(16, &state).fg, Some(Color::Yellow));
    assert_eq!(pattern_row_gutter_style(4, &state).fg, Some(Color::Gray));
    assert_eq!(
        pattern_row_gutter_style(1, &state).fg,
        Some(Color::DarkGray)
    );
    assert_eq!(format_row_number(0, false, 1), "01");
    assert_eq!(format_row_number(15, true, 1), "10");
}
