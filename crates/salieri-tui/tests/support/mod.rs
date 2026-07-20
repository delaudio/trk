use std::{fs, path::PathBuf};

use ratatui::{backend::TestBackend, Terminal};
use salieri_core::{Cursor, Song};
use salieri_sampler::{WaveformBucket, WaveformOverview};
use salieri_tui::{render, render_waveform_overview, HelpTab, TuiState, TuiView};

pub fn test_state<'a>() -> TuiState<'a> {
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
        show_pattern_top_info: false,
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
        sample_browser: None,
        project_browser: None,
        ai_chat: None,
        tracker_layout: salieri_tui::TrackerLayoutState::default(),
    }
}

pub fn render_snapshot(song: Song, state: TuiState<'_>, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render(frame, &song, state))
        .expect("draw");
    buffer_text(terminal.backend().buffer())
}

pub fn render_waveform_snapshot(overview: WaveformOverview) -> String {
    let backend = TestBackend::new(42, 10);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render_waveform_overview(frame, frame.area(), &overview))
        .expect("draw");
    buffer_text(terminal.backend().buffer())
}

fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
    let mut output = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            output.push_str(buffer[(x, y)].symbol());
        }
        output.push('\n');
    }
    output
}

pub fn waveform_overview(buckets: Vec<WaveformBucket>) -> WaveformOverview {
    WaveformOverview {
        sample_rate: 44_100,
        channels: 2,
        frames: 88_200,
        duration_seconds: 2.0,
        buckets,
    }
}

pub fn assert_snapshot(name: &str, actual: String) {
    let path = snapshot_path(name);
    if std::env::var_os("UPDATE_SALIERI_SNAPSHOTS").is_some() {
        fs::write(&path, actual).expect("write snapshot");
        return;
    }

    let expected = fs::read_to_string(&path).expect("read snapshot");
    if actual != expected {
        panic!(
            "snapshot mismatch for {name}\n{}",
            line_diff(&expected, &actual)
        );
    }
}

fn line_diff(expected: &str, actual: &str) -> String {
    let expected = expected.lines().collect::<Vec<_>>();
    let actual = actual.lines().collect::<Vec<_>>();
    let first = (0..expected.len().max(actual.len()))
        .find(|&index| expected.get(index) != actual.get(index))
        .unwrap_or(0);
    let start = first.saturating_sub(2);
    let end = (first + 3).min(expected.len().max(actual.len()));
    let mut diff = format!("first differing line: {}\n", first + 1);
    for index in start..end {
        diff.push_str(&format!(
            "{:>5} - {}\n      + {}\n",
            index + 1,
            expected.get(index).copied().unwrap_or("<missing>"),
            actual.get(index).copied().unwrap_or("<missing>")
        ));
    }
    diff
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(format!("{name}.snap"))
}

#[cfg(test)]
mod tests {
    use super::line_diff;

    #[test]
    fn diff_identifies_first_changed_line_with_context() {
        let diff = line_diff("one\ntwo\nthree\n", "one\nchanged\nthree\n");
        assert!(diff.contains("first differing line: 2"));
        assert!(diff.contains("    2 - two"));
        assert!(diff.contains("+ changed"));
    }
}
