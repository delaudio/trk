use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use salieri_core::{CellField, Cursor, NoteEvent, Pattern, PatternCell, Song};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuiState<'a> {
    pub cursor: Cursor,
    pub row_offset: usize,
    pub pattern_index: usize,
    pub mode_label: &'a str,
    pub octave: u8,
    pub dirty: bool,
    pub command_line: Option<&'a str>,
    pub show_help: bool,
    pub is_playing: bool,
    pub playhead_row: Option<usize>,
    pub midi_status: &'a str,
    pub sequence_position: Option<usize>,
}

pub fn render(frame: &mut Frame<'_>, song: &Song, state: TuiState<'_>) {
    let area = frame.area();
    let vertical = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, vertical[0], song, state);
    render_body(frame, vertical[1], song, state);
    render_status(frame, vertical[2], state);

    if state.show_help {
        render_help_overlay(frame, area, state.mode_label);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let pattern_name = active_pattern(song, state.pattern_index)
        .map_or("No Pattern", |pattern| pattern.name.as_str());
    let dirty = if state.dirty { " *" } else { "" };
    let playback = if state.is_playing { "PLAY" } else { "STOP" };
    let playhead = state
        .playhead_row
        .map_or_else(|| "--".to_string(), |row| format!("{row:02}"));
    let text = format!(
        " BPM {} | LPB {} | {}{} | Oct {} | Row {:02} | Play {playhead} | Track {:02} | Field {} | {} | {playback} | {} ",
        song.transport.bpm,
        song.transport.lines_per_beat,
        pattern_name,
        dirty,
        state.octave,
        state.cursor.row,
        state.cursor.track + 1,
        state.cursor.field,
        state.mode_label,
        state.midi_status,
    );
    let header = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Salieri Tracker ")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White));
    frame.render_widget(header, area);
}

fn render_body(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let chunks = if area.width >= 120 {
        Layout::default()
            .direction(LayoutDirection::Horizontal)
            .constraints([Constraint::Length(25), Constraint::Min(40)])
            .split(area)
    } else {
        Layout::default()
            .direction(LayoutDirection::Horizontal)
            .constraints([Constraint::Min(40)])
            .split(area)
    };

    if area.width >= 120 {
        let side = Layout::default()
            .direction(LayoutDirection::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[0]);
        render_tracks(frame, side[0], song, state.cursor.track);
        render_sequence(frame, side[1], song, state.sequence_position);
        render_pattern(frame, chunks[1], song, state);
    } else {
        render_pattern(frame, chunks[0], song, state);
    }
}

fn render_tracks(frame: &mut Frame<'_>, area: Rect, song: &Song, active_track: usize) {
    let lines = song
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let marker = if index == active_track { ">" } else { " " };
            let mute = if track.muted { "M" } else { "-" };
            let solo = if track.solo { "S" } else { "-" };
            Line::from(format!(
                "{} {:02} {:<10} CH{:02} {mute}{solo}",
                marker,
                index + 1,
                track.name,
                track.midi_channel
            ))
        })
        .collect::<Vec<_>>();

    let tracks =
        Paragraph::new(lines).block(Block::default().title(" Tracks ").borders(Borders::ALL));
    frame.render_widget(tracks, area);
}

fn render_sequence(
    frame: &mut Frame<'_>,
    area: Rect,
    song: &Song,
    active_sequence_position: Option<usize>,
) {
    let lines = song
        .sequence
        .iter()
        .enumerate()
        .map(|(index, pattern_id)| {
            let name = song
                .patterns
                .iter()
                .find(|pattern| pattern.id == *pattern_id)
                .map_or("Missing Pattern", |pattern| pattern.name.as_str());
            let marker = if active_sequence_position.unwrap_or(0) == index {
                ">"
            } else {
                " "
            };
            Line::from(format!("{marker} {index:02} {name}"))
        })
        .collect::<Vec<_>>();

    let sequence =
        Paragraph::new(lines).block(Block::default().title(" Sequence ").borders(Borders::ALL));
    frame.render_widget(sequence, area);
}

fn render_pattern(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let Some(pattern) = active_pattern(song, state.pattern_index) else {
        let empty = Paragraph::new("No pattern")
            .block(Block::default().title(" Pattern ").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    };

    let inner_height = area.height.saturating_sub(2) as usize;
    let data_height = inner_height.saturating_sub(1);
    let row_offset = state.row_offset.min(pattern.row_count().saturating_sub(1));
    let mut lines = Vec::with_capacity(data_height.saturating_add(1));
    lines.push(pattern_header(song));

    for row_index in row_offset
        ..pattern
            .row_count()
            .min(row_offset.saturating_add(data_height))
    {
        lines.push(pattern_row(
            song,
            pattern,
            row_index,
            state.cursor,
            state.playhead_row,
        ));
    }

    let block = Block::default()
        .title(" Pattern Editor ")
        .borders(Borders::ALL);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn active_pattern(song: &Song, pattern_index: usize) -> Option<&Pattern> {
    song.pattern(pattern_index)
}

fn pattern_header(song: &Song) -> Line<'static> {
    let mut spans = vec![
        Span::styled("ROW", Style::default().fg(Color::DarkGray)),
        Span::raw(" | "),
    ];

    for track in &song.tracks {
        spans.push(Span::styled(
            format!("{:^8}", truncate(&track.name, 8)),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
    }

    Line::from(spans)
}

fn pattern_row(
    song: &Song,
    pattern: &Pattern,
    row_index: usize,
    cursor: Cursor,
    playhead_row: Option<usize>,
) -> Line<'static> {
    let is_playhead = playhead_row == Some(row_index);
    let mut spans = vec![
        Span::styled(
            format!("{}{row_index:02}", if is_playhead { ">" } else { " " }),
            if is_playhead {
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::raw(" "),
    ];

    let Some(row) = pattern.rows.get(row_index) else {
        return Line::from(spans);
    };

    for track_index in 0..song.tracks.len() {
        let cell = row.cells.get(track_index).cloned().unwrap_or_default();
        let is_cursor_row = cursor.row == row_index;
        let is_cursor_cell = is_cursor_row && cursor.track == track_index;
        spans.extend(cell_spans(&cell, cursor.field, is_cursor_cell));
        spans.push(Span::raw(" "));
    }

    Line::from(spans)
}

fn cell_spans(cell: &PatternCell, focused_field: CellField, focused: bool) -> Vec<Span<'static>> {
    let note = match cell.note {
        Some(NoteEvent::Note { pitch }) => format_note(pitch),
        Some(NoteEvent::NoteOff) => "OFF".to_string(),
        Some(NoteEvent::NoteCut) => "CUT".to_string(),
        None => "---".to_string(),
    };
    let velocity = cell
        .velocity
        .map_or_else(|| "--".to_string(), |value| format!("{value:02X}"));

    let normal = Style::default().fg(Color::White);
    let focused_style = Style::default()
        .fg(Color::Black)
        .bg(Color::White)
        .add_modifier(Modifier::BOLD);
    let note_style = if focused && focused_field == CellField::Note {
        focused_style
    } else {
        normal
    };
    let velocity_style = if focused && focused_field == CellField::Velocity {
        focused_style
    } else {
        normal
    };

    vec![
        Span::styled(note, note_style),
        Span::raw(" "),
        Span::styled(velocity, velocity_style),
    ]
}

fn render_status(frame: &mut Frame<'_>, area: Rect, state: TuiState<'_>) {
    if let Some(command_line) = state.command_line {
        let status = Paragraph::new(format!(" :{command_line}"));
        frame.render_widget(status, area);
        return;
    }

    let status = Paragraph::new(format!(
        " {} | Space Play/Stop | F8 Stop | : Command | i Edit | Ctrl+T Track | M Mute | S Solo | Ctrl+S Save | Ctrl+Z Undo | q Quit ",
        state.mode_label
    ));
    frame.render_widget(status, area);
}

fn render_help_overlay(frame: &mut Frame<'_>, area: Rect, mode_label: &str) {
    let overlay = centered_rect(72, 22, area);
    let lines = vec![
        Line::from(Span::styled(
            "Global",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ? Help   q Quit   Space Play/Stop   F8 Stop   Ctrl+S Save   Ctrl+Z Undo"),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Arrows or h/j/k/l move   PageUp/PageDown jump   Home/End pattern bounds"),
        Line::from(""),
        Line::from(Span::styled(
            "Editing",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  i Edit   Esc Normal   Del/Backspace clear cell   F1/- octave down"),
        Line::from("  F2/+/= octave up   Velocity field accepts two hex digits"),
        Line::from(""),
        Line::from(Span::styled(
            "Notes",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  z s x d c v g b h n j m = C C# D D# E F F# G G# A A# B"),
        Line::from("  q 2 w 3 e r 5 t 6 y 7 u = same notes one octave higher"),
        Line::from(""),
        Line::from(Span::styled(
            "Tracks And Commands",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Ctrl+T create track   Del delete track in normal mode   M mute   S solo"),
        Line::from("  :write   :quit   :wq   :bpm 140   :lpb 4"),
        Line::from("  :track new   :track duplicate 2   :track rename Acid Bass"),
        Line::from("  :track channel 12   :track channel 2 12"),
        Line::from("  :midi connect 0   :midi disconnect   :midi panic"),
        Line::from("  :play pattern   :play sequence   :stop"),
        Line::from("  :pattern new   :pattern duplicate   :pattern delete   :pattern length 128"),
        Line::from("  :pattern rename Intro   :pattern 1"),
        Line::from("  :sequence add   :sequence remove 0   :sequence duplicate 0"),
        Line::from("  :sequence set 0 2   :sequence move 1 0"),
        Line::from(""),
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ];

    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(" Help ").borders(Borders::ALL))
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
    frame.render_widget(Clear, overlay);
    frame.render_widget(paragraph, overlay);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        truncated
    } else {
        value.to_string()
    }
}

fn format_note(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
    ];
    let octave = i16::from(pitch / 12) - 1;
    let note = NAMES[(pitch % 12) as usize];
    format!("{note}{octave}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use salieri_core::Song;

    #[test]
    fn renders_default_pattern_without_panic() {
        let song = Song::empty();
        let backend = TestBackend::new(120, 32);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal
            .draw(|frame| {
                render(
                    frame,
                    &song,
                    TuiState {
                        cursor: Cursor::new(),
                        row_offset: 0,
                        pattern_index: 0,
                        mode_label: "NORMAL",
                        octave: 4,
                        dirty: false,
                        command_line: None,
                        show_help: false,
                        is_playing: false,
                        playhead_row: None,
                        midi_status: "MIDI Disconnected",
                        sequence_position: None,
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

        assert!(rendered.contains("Salieri Tracker"));
        assert!(rendered.contains("Pattern Editor"));
        assert!(rendered.contains("Drums"));
        assert!(rendered.contains("Bass"));
    }

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
                        pattern_index: 0,
                        mode_label: "HELP",
                        octave: 4,
                        dirty: false,
                        command_line: None,
                        show_help: true,
                        is_playing: false,
                        playhead_row: None,
                        midi_status: "MIDI Disconnected",
                        sequence_position: None,
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
        assert!(rendered.contains("Notes"));
    }

    #[test]
    fn renders_playhead_when_playing() {
        let song = Song::empty();
        let backend = TestBackend::new(120, 32);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal
            .draw(|frame| {
                render(
                    frame,
                    &song,
                    TuiState {
                        cursor: Cursor::new(),
                        row_offset: 0,
                        pattern_index: 0,
                        mode_label: "NORMAL",
                        octave: 4,
                        dirty: false,
                        command_line: None,
                        show_help: false,
                        is_playing: true,
                        playhead_row: Some(0),
                        midi_status: "MIDI Connected 0",
                        sequence_position: Some(0),
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
        assert!(rendered.contains(">00"));
        assert!(rendered.contains("MIDI Connected 0"));
    }
}
