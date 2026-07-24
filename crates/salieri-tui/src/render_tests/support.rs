use super::*;
use ratatui::{backend::TestBackend, Terminal};
use salieri_core::Song;

pub(super) fn test_waveform(buckets: Vec<salieri_sampler::WaveformBucket>) -> WaveformOverview {
    WaveformOverview {
        sample_rate: 44_100,
        channels: 1,
        frames: 44_100,
        duration_seconds: 1.0,
        buckets,
    }
}

pub(super) fn long_sequence_song(count: usize) -> Song {
    let mut song = Song::empty();
    song.sequence.clear();
    for index in 0..count {
        let pattern_id = if index == 0 {
            song.patterns[0].id
        } else {
            song.create_pattern(64)
        };
        song.rename_pattern(index, format!("Pattern {:02}", index + 1))
            .expect("rename pattern");
        song.sequence.push(pattern_id);
    }
    song
}

pub(super) fn long_track_song(count: usize) -> Song {
    let mut song = Song::empty();
    while song.tracks.len() < count {
        song.create_track();
    }
    for index in 0..song.tracks.len() {
        song.rename_track(index, format!("Track {:02}", index + 1))
            .expect("rename track");
    }
    song
}

pub(super) fn terminal_buffer_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}

pub(super) fn render_test_state<'a>() -> TuiState<'a> {
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
        tracker_layout: crate::TrackerLayoutState::default(),
    }
}

pub(super) fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>()
}
