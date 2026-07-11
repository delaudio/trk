use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use salieri_core::{CellField, Cursor, NoteEvent, Pattern, PatternCell, Song};

const TRACK_PANEL_WIDTH: u16 = 27;
const ROW_GUTTER_WIDTH: usize = 5;
const PATTERN_CELL_WIDTH: usize = 10;
const TRACK_LIST_NAME_WIDTH: usize = 11;
const MEDIUM_MIN_WIDTH: u16 = 80;
const LARGE_MIN_WIDTH: u16 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutKind {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuiState<'a> {
    pub cursor: Cursor,
    pub row_offset: usize,
    pub pattern_index: usize,
    pub active_view: TuiView,
    pub selection: Option<SelectionRect>,
    pub mode_label: &'a str,
    pub octave: u8,
    pub dirty: bool,
    pub show_line_numbers_hex: bool,
    pub command_line: Option<&'a str>,
    pub notification: Option<NotificationView<'a>>,
    pub show_help: bool,
    pub is_playing: bool,
    pub loop_pattern: bool,
    pub playhead_row: Option<usize>,
    pub midi_status: &'a str,
    pub sequence_position: Option<usize>,
    pub quit_confirmation: bool,
    pub delete_confirmation: Option<&'a str>,
    pub midi_settings: Option<MidiSettingsState<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiView {
    Pattern,
    Sequence,
    Tracks,
    Patterns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationView<'a> {
    pub kind: NotificationKind,
    pub message: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiSettingsState<'a> {
    pub ports: &'a [MidiPortView<'a>],
    pub selected_port: usize,
    pub status: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiPortView<'a> {
    pub index: usize,
    pub name: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionRect {
    pub row_start: usize,
    pub row_end: usize,
    pub track_start: usize,
    pub track_end: usize,
}

impl SelectionRect {
    #[must_use]
    pub const fn contains(self, row: usize, track: usize) -> bool {
        self.row_start <= row
            && row <= self.row_end
            && self.track_start <= track
            && track <= self.track_end
    }
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
    if let Some(midi_settings) = state.midi_settings {
        render_midi_settings_overlay(frame, area, midi_settings);
    }
    if state.quit_confirmation {
        render_quit_confirmation(frame, area);
    }
    if let Some(message) = state.delete_confirmation {
        render_delete_confirmation(frame, area, message);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let pattern_name = active_pattern(song, state.pattern_index)
        .map_or("No Pattern", |pattern| pattern.name.as_str());
    let dirty = if state.dirty { " *" } else { "" };
    let playback = if state.is_playing { "PLAY" } else { "STOP" };
    let loop_state = if state.loop_pattern {
        "Loop ON"
    } else {
        "Loop OFF"
    };
    let selection = if state.selection.is_some() {
        " | SEL"
    } else {
        ""
    };
    let playhead = state
        .playhead_row
        .map_or_else(|| "--".to_string(), |row| format!("{row:02}"));
    let text = format!(
        " BPM {} | LPB {} | {}{} | Oct {} | Row {:02} | Play {playhead} | {loop_state} | Track {:02} | Field {} | {}{selection} | {playback} | {} ",
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
    if state.active_view == TuiView::Sequence {
        render_sequence_editor(frame, area, song, state.sequence_position);
        return;
    }
    if state.active_view == TuiView::Tracks {
        render_track_editor(frame, area, song, state.cursor.track);
        return;
    }
    if state.active_view == TuiView::Patterns {
        render_pattern_manager(frame, area, song, state.pattern_index);
        return;
    }

    match layout_kind(area.width) {
        LayoutKind::Large => {
            let chunks = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([Constraint::Length(TRACK_PANEL_WIDTH), Constraint::Min(40)])
                .split(area);
            let side = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(chunks[0]);
            render_tracks(frame, side[0], song, state.cursor.track);
            render_sequence(frame, side[1], song, state.sequence_position);
            render_pattern(frame, chunks[1], song, state);
        }
        LayoutKind::Medium => {
            let chunks = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([Constraint::Min(48), Constraint::Length(TRACK_PANEL_WIDTH)])
                .split(area);
            render_pattern(frame, chunks[0], song, state);
            render_medium_side(frame, chunks[1], song, state);
        }
        LayoutKind::Small => {
            render_pattern(frame, area, song, state);
        }
    }
}

fn layout_kind(width: u16) -> LayoutKind {
    if width >= LARGE_MIN_WIDTH {
        LayoutKind::Large
    } else if width >= MEDIUM_MIN_WIDTH {
        LayoutKind::Medium
    } else {
        LayoutKind::Small
    }
}

fn render_medium_side(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let side = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    render_tracks(frame, side[0], song, state.cursor.track);
    render_sequence(frame, side[1], song, state.sequence_position);
}

fn render_tracks(frame: &mut Frame<'_>, area: Rect, song: &Song, active_track: usize) {
    let lines = song
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let is_active = index == active_track;
            let marker = if is_active { ">" } else { " " };
            let mute = if track.muted { "M" } else { "-" };
            let solo = if track.solo { "S" } else { "-" };
            let name = truncate(&track.name, TRACK_LIST_NAME_WIDTH);
            let line = format!(
                "{} {:02} {:<width$} CH{:02} {mute}{solo}",
                marker,
                index + 1,
                name,
                track.midi_channel,
                width = TRACK_LIST_NAME_WIDTH,
            );

            if is_active {
                Line::styled(
                    line,
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Line::from(line)
            }
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
            let marker = if active_sequence_position == Some(index) {
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

fn render_sequence_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    song: &Song,
    active_sequence_position: Option<usize>,
) {
    let mut lines = vec![Line::from(vec![
        Span::styled("POS  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "PATTERN",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    if song.sequence.is_empty() {
        lines.push(Line::from("No sequence positions"));
    } else {
        for (index, pattern_id) in song.sequence.iter().enumerate() {
            let pattern = song
                .patterns
                .iter()
                .find(|pattern| pattern.id == *pattern_id);
            let name = pattern.map_or("Missing Pattern", |pattern| pattern.name.as_str());
            let marker = if active_sequence_position == Some(index) {
                ">"
            } else {
                " "
            };
            let line = format!(
                "{marker}{index:02}  {:<24} id {}",
                truncate(name, 24),
                pattern_id.0
            );
            if active_sequence_position == Some(index) {
                lines.push(Line::styled(
                    line,
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                lines.push(Line::from(line));
            }
        }
    }

    lines.extend([
        Line::from(""),
        Line::from("A add current pattern   R remove   Y duplicate   T set current"),
        Line::from("</> move position   Enter play from position   Esc pattern view"),
    ]);

    let sequence = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Sequence Editor ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(sequence, area);
}

fn render_track_editor(frame: &mut Frame<'_>, area: Rect, song: &Song, active_track: usize) {
    let mut lines = vec![Line::from(vec![
        Span::styled("TRK  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "NAME          CH  M  S  ARM",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    for (index, track) in song.tracks.iter().enumerate() {
        let marker = if index == active_track { ">" } else { " " };
        let muted = if track.muted { "Y" } else { "-" };
        let solo = if track.solo { "Y" } else { "-" };
        let armed = if track.armed { "Y" } else { "-" };
        let line = format!(
            "{marker}{:02}  {:<12} CH{:02} {muted:^3}{solo:^3}{armed:^3}",
            index + 1,
            truncate(&track.name, 12),
            track.midi_channel
        );
        if index == active_track {
            lines.push(Line::styled(
                line,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            lines.push(Line::from(line));
        }
    }

    lines.extend([
        Line::from(""),
        Line::from("N new   D duplicate   r rename   c channel   Delete remove"),
        Line::from("{/} reorder   M mute   S solo   Esc pattern view"),
    ]);

    let tracks = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Track Editor ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(tracks, area);
}

fn render_pattern_manager(frame: &mut Frame<'_>, area: Rect, song: &Song, active_pattern: usize) {
    let mut lines = vec![Line::from(vec![
        Span::styled("PAT  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "NAME                       ROWS",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    for (index, pattern) in song.patterns.iter().enumerate() {
        let marker = if index == active_pattern { ">" } else { " " };
        let line = format!(
            "{marker}{:02}  {:<24} {:>4}",
            index + 1,
            truncate(&pattern.name, 24),
            pattern.row_count()
        );
        if index == active_pattern {
            lines.push(Line::styled(
                line,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            lines.push(Line::from(line));
        }
    }

    lines.extend([
        Line::from(""),
        Line::from("N new   P duplicate   r rename   X/Delete remove   F6 custom length"),
        Line::from("1/2/3/4/5 length 16/32/64/128/256   Esc pattern editor"),
    ]);

    let patterns = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Pattern Manager ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(patterns, area);
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
    lines.push(pattern_header(song, state.cursor.track));

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
            state.selection,
            state.show_line_numbers_hex,
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

fn pattern_header(song: &Song, active_track: usize) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{:<ROW_GUTTER_WIDTH$}", "ROW"),
        Style::default().fg(Color::DarkGray),
    )];

    for (track_index, track) in song.tracks.iter().enumerate() {
        let is_active = track_index == active_track;
        spans.push(Span::styled(
            format!(
                "{:^PATTERN_CELL_WIDTH$}",
                truncate(&track.name, PATTERN_CELL_WIDTH)
            ),
            if is_active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            },
        ));
    }

    Line::from(spans)
}

fn pattern_row(
    song: &Song,
    pattern: &Pattern,
    row_index: usize,
    cursor: Cursor,
    playhead_row: Option<usize>,
    selection: Option<SelectionRect>,
    show_line_numbers_hex: bool,
) -> Line<'static> {
    let is_playhead = playhead_row == Some(row_index);
    let mut spans = vec![Span::styled(
        format!(
            "{:<ROW_GUTTER_WIDTH$}",
            format!(
                "{}{}",
                if is_playhead { ">" } else { " " },
                format_row_number(row_index, show_line_numbers_hex)
            )
        ),
        if is_playhead {
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        },
    )];

    let Some(row) = pattern.rows.get(row_index) else {
        return Line::from(spans);
    };

    for track_index in 0..song.tracks.len() {
        let cell = row.cells.get(track_index).cloned().unwrap_or_default();
        let is_cursor_row = cursor.row == row_index;
        let is_cursor_cell = is_cursor_row && cursor.track == track_index;
        let is_active_track = cursor.track == track_index;
        let is_selected =
            selection.is_some_and(|selection| selection.contains(row_index, track_index));
        spans.extend(cell_spans(
            &cell,
            cursor.field,
            is_cursor_cell,
            is_selected,
            is_playhead,
            is_active_track,
        ));
    }

    Line::from(spans)
}

fn format_row_number(row: usize, hexadecimal: bool) -> String {
    if hexadecimal {
        format!("{:02X}", row.min(0xff))
    } else {
        format!("{row:02}")
    }
}

fn cell_spans(
    cell: &PatternCell,
    focused_field: CellField,
    focused: bool,
    selected: bool,
    playing: bool,
    active_track: bool,
) -> Vec<Span<'static>> {
    let note = match cell.note {
        Some(NoteEvent::Note { pitch }) => format_note(pitch),
        Some(NoteEvent::NoteOff) => "OFF".to_string(),
        Some(NoteEvent::NoteCut) => "CUT".to_string(),
        None => "---".to_string(),
    };
    let velocity = cell
        .velocity
        .map_or_else(|| "--".to_string(), |value| format!("{value:02X}"));
    let command = cell
        .command
        .map(|command| format!("{}{:02X}", command.display_code(), command.value));

    let normal = Style::default().fg(Color::White);
    let focused_style = Style::default()
        .fg(Color::Black)
        .bg(Color::White)
        .add_modifier(Modifier::BOLD);
    let selected_style = Style::default()
        .fg(Color::White)
        .bg(Color::DarkGray)
        .add_modifier(Modifier::REVERSED);
    let playing_style = Style::default()
        .fg(Color::LightGreen)
        .add_modifier(Modifier::BOLD);
    let active_track_style = Style::default().fg(Color::White).bg(Color::DarkGray);
    let note_style = if focused && focused_field == CellField::Note {
        focused_style
    } else if selected {
        selected_style
    } else if playing {
        playing_style
    } else if active_track {
        active_track_style
    } else {
        normal
    };
    let velocity_style = if focused && focused_field == CellField::Velocity {
        focused_style
    } else if selected {
        selected_style
    } else if playing {
        playing_style
    } else if active_track {
        active_track_style
    } else {
        normal
    };
    let spacer_style = if selected {
        selected_style
    } else if playing {
        playing_style
    } else if active_track {
        active_track_style
    } else {
        normal
    };

    let mut spans = vec![
        Span::styled(" ", spacer_style),
        Span::styled(note, note_style),
        Span::styled(" ", spacer_style),
        Span::styled(velocity, velocity_style),
    ];
    if let Some(command) = command {
        spans.push(Span::styled(" ", spacer_style));
        spans.push(Span::styled(command, normal));
    } else {
        spans.push(Span::styled("   ", spacer_style));
    }
    spans
}

fn render_status(frame: &mut Frame<'_>, area: Rect, state: TuiState<'_>) {
    if let Some(command_line) = state.command_line {
        let status = Paragraph::new(format!(" :{command_line}"));
        frame.render_widget(status, area);
        return;
    }

    if let Some(notification) = state.notification {
        let label = match notification.kind {
            NotificationKind::Info => "INFO",
            NotificationKind::Success => "OK",
            NotificationKind::Warning => "WARN",
            NotificationKind::Error => "ERR",
        };
        let style = match notification.kind {
            NotificationKind::Info => Style::default().fg(Color::Cyan),
            NotificationKind::Success => Style::default().fg(Color::LightGreen),
            NotificationKind::Warning => Style::default().fg(Color::Yellow),
            NotificationKind::Error => Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        };
        let status = Paragraph::new(Line::from(vec![
            Span::styled(format!(" {label} "), style.add_modifier(Modifier::BOLD)),
            Span::styled(
                notification.message.to_string(),
                Style::default().fg(Color::White),
            ),
        ]));
        frame.render_widget(status, area);
        return;
    }

    let text = if state.active_view == TuiView::Sequence {
        format!(
            " {} | H Help | Esc Pattern | A Add | R Remove | Y Duplicate | T Set Pattern | </> Move | Enter Play | : Command | Ctrl+S Save | Ctrl+Shift+S Save As | q Quit ",
            state.mode_label
        )
    } else if state.active_view == TuiView::Tracks {
        format!(
            " {} | H Help | Esc Pattern | N New | D Duplicate | r Rename | c Channel | Del Delete | {{/}} Move | M/S Mute/Solo | : Command | Ctrl+S Save | Ctrl+Shift+S Save As | q Quit ",
            state.mode_label
        )
    } else if state.active_view == TuiView::Patterns {
        format!(
            " {} | H Help | Esc Pattern | N New | P Duplicate | r Rename | X/Del Delete | 1-5 Length Presets | F6 Length | : Command | Ctrl+S Save | Ctrl+Shift+S Save As | q Quit ",
            state.mode_label
        )
    } else {
        format!(
            " {}{} | H Help | F4 MIDI | F7 Sequence | F9 Tracks | F10 Patterns | Space Play/Stop | Enter Row | Shift+Enter Seq | L Loop | N/P/X Pattern | A/Y/R Seq | {{/}} Track | : Command | i Edit | V Select | Ctrl+S Save | Ctrl+Shift+S Save As | q Quit ",
            state.mode_label,
            if state.selection.is_some() { " SEL" } else { "" }
        )
    };
    let status = Paragraph::new(text);
    frame.render_widget(status, area);
}

fn render_help_overlay(frame: &mut Frame<'_>, area: Rect, mode_label: &str) {
    let overlay = centered_rect(98, 31, area);
    let lines = vec![
        Line::from(Span::styled(
            "Global",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?/H Help   :h/:help Help   q Quit   Space Play/Stop   Shift+Space Start"),
        Line::from("  Enter Play Row   Shift+Enter Play Sequence From Cursor   L Loop   F8 Stop"),
        Line::from("  F7 Sequence View   F9 Track View   F10 Pattern View   Esc returns from focused views"),
        Line::from("  :play pattern from start   :play sequence arrangement"),
        Line::from("  Ctrl+S Save   Ctrl+Shift+S Save As   Ctrl+Z Undo   Ctrl+Y Redo   Ctrl+Arrows BPM/LPB"),
        Line::from(""),
        Line::from(Span::styled(
            "MIDI",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  F4 or :midi outputs opens MIDI settings and lists output ports"),
        Line::from("  In MIDI settings: arrows select, Enter connects, F5/r refresh, p panic"),
        Line::from("  CLI fallback: salieri --list-midi-outputs, then :midi connect 0"),
        Line::from("  4. Press Space or run :play pattern to send notes to the connected output"),
        Line::from("  :midi disconnect closes the output   :midi panic sends All Notes Off"),
        Line::from("  Use :track channel 2 10 to set track 02 to MIDI channel 10"),
        Line::from("  Config: [midi] default_output = \"IAC Driver\" auto-connects by name"),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Arrows or h/j/k/l move   Tab/Shift+Tab track   PageUp/PageDown jump"),
        Line::from("  Home/End pattern bounds   gg first row   G last row"),
        Line::from(""),
        Line::from(Span::styled(
            "Editing",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  i Edit   Esc Normal   Del/Backspace clear cell   Ctrl+C/X/V cell clipboard"),
        Line::from("  V select region   Esc cancel selection   Delete clears selection"),
        Line::from("  Insert row   Ctrl+Delete delete row   F1/- octave down"),
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
        Line::from("  o = OFF   . = CUT"),
        Line::from(""),
        Line::from(Span::styled(
            "Tracks And Commands",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Ctrl+T create track   D duplicate track   {/} move track left/right"),
        Line::from("  r rename track   c channel"),
        Line::from("  Del delete track   M mute   S solo"),
        Line::from("  :write [path]   :saveas path   :quit   :q!   :wq   :bpm 140   :lpb 4"),
        Line::from("  Dirty quit asks: [Y]es save, [N]o quit, [C]ancel"),
        Line::from("  :track new   :track duplicate 2   :track delete 2   :track move 2 3"),
        Line::from("  :track mute 2   :track solo 2   :track rename Acid Bass"),
        Line::from("  :track channel 12   :fx D 20 delay   :fx R 04 retrigger   :fx clear"),
        Line::from("  :play pattern   :play sequence [position]   :stop"),
        Line::from(""),
        Line::from(Span::styled(
            "Patterns And Sequence",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(
            "  N new pattern   P duplicate pattern   X delete pattern   F3 rename   F6 length",
        ),
        Line::from("  Pattern view: 1/2/3/4/5 set length 16/32/64/128/256"),
        Line::from("  :pattern new   :pattern duplicate   :pattern delete   :pattern length 128"),
        Line::from("  :pattern rename Intro   :pattern 1   [ previous pattern   ] next pattern"),
        Line::from("  A add current pattern to sequence   ,/. move sequence cursor"),
        Line::from("  Y duplicate sequence position   R remove   T set to current pattern"),
        Line::from("  </> move selected sequence position up/down"),
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

fn render_midi_settings_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    midi_settings: MidiSettingsState<'_>,
) {
    let overlay = centered_rect(76, 18, area);
    let mut lines = vec![
        Line::from(Span::styled(
            "Output Ports",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("  Status: {}", midi_settings.status)),
        Line::from(""),
    ];

    if midi_settings.ports.is_empty() {
        lines.push(Line::from("  No MIDI output ports found"));
        lines.push(Line::from(""));
        lines.push(Line::from(
            "  On macOS, enable IAC Driver in Audio MIDI Setup.",
        ));
    } else {
        for (row, port) in midi_settings.ports.iter().enumerate() {
            let marker = if row == midi_settings.selected_port {
                ">"
            } else {
                " "
            };
            let line = format!("{marker} {:02} {}", port.index, port.name);
            if row == midi_settings.selected_port {
                lines.push(Line::styled(
                    line,
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                lines.push(Line::from(line));
            }
        }
    }

    lines.extend([
        Line::from(""),
        Line::from("Enter connect selected   d disconnect   p panic/all notes off"),
        Line::from("F5/r refresh ports   Esc/q close"),
    ]);

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" MIDI Settings ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
    frame.render_widget(Clear, overlay);
    frame.render_widget(paragraph, overlay);
}

fn render_quit_confirmation(frame: &mut Frame<'_>, area: Rect) {
    let overlay = centered_rect(48, 7, area);
    let lines = vec![
        Line::from("Unsaved changes. Save before quitting?"),
        Line::from(""),
        Line::from("[Y]es   [N]o   [C]ancel"),
    ];
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(" Quit ").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(Clear, overlay);
    frame.render_widget(paragraph, overlay);
}

fn render_delete_confirmation(frame: &mut Frame<'_>, area: Rect, message: &str) {
    let overlay = centered_rect(52, 7, area);
    let lines = vec![
        Line::from(message.to_string()),
        Line::from(""),
        Line::from("[Y]es   [N]o   [Esc] Cancel"),
    ];
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Confirm Delete ")
                .borders(Borders::ALL),
        )
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
    fn classifies_responsive_layout_breakpoints() {
        assert_eq!(layout_kind(79), LayoutKind::Small);
        assert_eq!(layout_kind(80), LayoutKind::Medium);
        assert_eq!(layout_kind(119), LayoutKind::Medium);
        assert_eq!(layout_kind(120), LayoutKind::Large);
    }

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
                        is_playing: false,
                        loop_pattern: true,
                        playhead_row: None,
                        midi_status: "MIDI Disconnected",
                        sequence_position: None,
                        quit_confirmation: false,
                        delete_confirmation: None,
                        midi_settings: None,
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
                        is_playing: false,
                        loop_pattern: true,
                        playhead_row: None,
                        midi_status: "MIDI Disconnected",
                        sequence_position: None,
                        quit_confirmation: false,
                        delete_confirmation: None,
                        midi_settings: None,
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
                        is_playing: false,
                        loop_pattern: true,
                        playhead_row: None,
                        midi_status: "MIDI Disconnected",
                        sequence_position: None,
                        quit_confirmation: false,
                        delete_confirmation: None,
                        midi_settings: None,
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
        assert!(rendered.contains("Sequence"));
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
                        active_view: TuiView::Pattern,
                        selection: None,
                        mode_label: "HELP",
                        octave: 4,
                        dirty: false,
                        show_line_numbers_hex: false,
                        command_line: None,
                        notification: None,
                        show_help: true,
                        is_playing: false,
                        loop_pattern: true,
                        playhead_row: None,
                        midi_status: "MIDI Disconnected",
                        sequence_position: None,
                        quit_confirmation: false,
                        delete_confirmation: None,
                        midi_settings: None,
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
        assert!(rendered.contains("MIDI"));
        assert!(rendered.contains(":midi outputs"));
        assert!(rendered.contains("salieri --list-midi-outputs"));
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
                        is_playing: true,
                        loop_pattern: true,
                        playhead_row: Some(0),
                        midi_status: "MIDI Connected 0",
                        sequence_position: Some(0),
                        quit_confirmation: false,
                        delete_confirmation: None,
                        midi_settings: None,
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
                        is_playing: false,
                        loop_pattern: true,
                        playhead_row: None,
                        midi_status: "MIDI Disconnected",
                        sequence_position: None,
                        quit_confirmation: false,
                        delete_confirmation: None,
                        midi_settings: None,
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
                        is_playing: false,
                        loop_pattern: true,
                        playhead_row: None,
                        midi_status: "MIDI Disconnected",
                        sequence_position: None,
                        quit_confirmation: false,
                        delete_confirmation: None,
                        midi_settings: None,
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
                        is_playing: false,
                        loop_pattern: true,
                        playhead_row: None,
                        midi_status: "MIDI Disconnected",
                        sequence_position: None,
                        quit_confirmation: true,
                        delete_confirmation: None,
                        midi_settings: None,
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
                        is_playing: false,
                        loop_pattern: true,
                        playhead_row: None,
                        midi_status: "MIDI Disconnected",
                        sequence_position: None,
                        quit_confirmation: false,
                        delete_confirmation: Some("Delete track 02 Bass?"),
                        midi_settings: None,
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

        assert!(rendered.contains("Confirm Delete"));
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
}
