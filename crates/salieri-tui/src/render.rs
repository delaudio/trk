use std::ops::Range;

use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
};
use salieri_core::{
    mixer_master_gain_descriptor, mixer_track_gain_descriptor, mixer_track_pan_descriptor,
    native_gain_descriptor, native_pan_descriptor, sample_gain_descriptor, CellField, Cursor,
    EffectDeviceKind, NoteEvent, ParameterDescriptor, ParameterValue, Pattern, PatternCell,
    SamplePlaybackMode, Song,
};
use salieri_sampler::{WaveformBucket, WaveformOverview};

use crate::ViewportAxis;

const TRACK_PANEL_WIDTH: u16 = 27;
const ROW_GUTTER_WIDTH: usize = 5;
const PATTERN_CELL_WIDTH: usize = 21;
const TRACK_LIST_NAME_WIDTH: usize = 11;
const MEDIUM_MIN_WIDTH: u16 = 80;
const LARGE_MIN_WIDTH: u16 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutKind {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TuiState<'a> {
    pub cursor: Cursor,
    pub row_offset: usize,
    pub track_offset: usize,
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
    pub help_scroll: usize,
    pub help_tab: HelpTab,
    pub is_playing: bool,
    pub loop_pattern: bool,
    pub playhead_row: Option<usize>,
    pub midi_status: &'a str,
    pub sequence_position: Option<usize>,
    pub quit_confirmation: bool,
    pub delete_confirmation: Option<&'a str>,
    pub midi_settings: Option<MidiSettingsState<'a>>,
    pub sampler_view: Option<SamplerViewState<'a>>,
    pub sample_browser: Option<SampleBrowserViewState<'a>>,
    pub project_browser: Option<ProjectBrowserViewState<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiView {
    Pattern,
    Sequence,
    Tracks,
    Patterns,
    Sampler,
    SampleBrowser,
    ProjectBrowser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpTab {
    Basics,
    Editing,
    Sampler,
    Midi,
    Commands,
}

impl HelpTab {
    const ALL: [Self; 5] = [
        Self::Basics,
        Self::Editing,
        Self::Sampler,
        Self::Midi,
        Self::Commands,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Basics => "Basics",
            Self::Editing => "Editing",
            Self::Sampler => "Sampler",
            Self::Midi => "MIDI",
            Self::Commands => "Commands",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Basics => Self::Editing,
            Self::Editing => Self::Sampler,
            Self::Sampler => Self::Midi,
            Self::Midi => Self::Commands,
            Self::Commands => Self::Basics,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Basics => Self::Commands,
            Self::Editing => Self::Basics,
            Self::Sampler => Self::Editing,
            Self::Midi => Self::Sampler,
            Self::Commands => Self::Midi,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplerViewState<'a> {
    pub name: &'a str,
    pub source_path: &'a str,
    pub overview: &'a WaveformOverview,
    pub gain: f32,
    pub waveform_start_bucket: usize,
    pub waveform_end_bucket: usize,
    pub waveform_zoom: usize,
    pub instrument: Option<&'a str>,
    pub assigned_track: Option<&'a str>,
    pub assigned_track_count: usize,
    pub playback_mode: &'a str,
    pub start_frame: Option<usize>,
    pub end_frame: Option<usize>,
    pub loop_start_frame: Option<usize>,
    pub loop_end_frame: Option<usize>,
    pub envelope: (f32, f32, f32, f32),
    pub selected_envelope: SamplerEnvelopeField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerEnvelopeField {
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleBrowserViewState<'a> {
    pub current_dir: &'a str,
    pub entries: &'a [SampleBrowserEntryView<'a>],
    pub selected: usize,
    pub preview: Option<SamplerViewState<'a>>,
    pub message: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleBrowserEntryView<'a> {
    pub name: &'a str,
    pub kind: SampleBrowserEntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleBrowserEntryKind {
    Directory,
    SupportedSample,
    UnsupportedFile,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectBrowserViewState<'a> {
    pub current_dir: &'a str,
    pub entries: &'a [ProjectBrowserEntryView<'a>],
    pub selected: usize,
    pub message: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectBrowserEntryView<'a> {
    pub name: &'a str,
    pub path: &'a str,
    pub kind: ProjectBrowserEntryKind,
    pub detail: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectBrowserEntryKind {
    Directory,
    RecentProject,
    Project,
    MissingProject,
    InvalidProject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveformGlyphs {
    Unicode,
    Ascii,
}

impl WaveformGlyphs {
    const fn full(self) -> char {
        match self {
            Self::Unicode => '█',
            Self::Ascii => '#',
        }
    }

    const fn upper(self) -> char {
        match self {
            Self::Unicode => '▀',
            Self::Ascii => '#',
        }
    }

    const fn lower(self) -> char {
        match self {
            Self::Unicode => '▄',
            Self::Ascii => '#',
        }
    }

    const fn baseline(self) -> char {
        match self {
            Self::Unicode => '─',
            Self::Ascii => '-',
        }
    }
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
        render_help_overlay(
            frame,
            area,
            state.mode_label,
            state.help_scroll,
            state.help_tab,
        );
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

pub fn render_waveform_overview(frame: &mut Frame<'_>, area: Rect, overview: &WaveformOverview) {
    render_waveform_overview_with_window(
        frame,
        area,
        overview,
        WaveformWindow::full(overview),
        WaveformGlyphs::Unicode,
    );
}

pub fn render_waveform_overview_with_glyphs(
    frame: &mut Frame<'_>,
    area: Rect,
    overview: &WaveformOverview,
    glyphs: WaveformGlyphs,
) {
    render_waveform_overview_with_window(
        frame,
        area,
        overview,
        WaveformWindow::full(overview),
        glyphs,
    );
}

fn render_waveform_overview_with_window(
    frame: &mut Frame<'_>,
    area: Rect,
    overview: &WaveformOverview,
    window: WaveformWindow,
    glyphs: WaveformGlyphs,
) {
    let block = Block::default().title(" Waveform ").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let lines = waveform_lines(
        overview,
        window,
        inner.width as usize,
        inner.height as usize,
        glyphs,
    );
    frame.render_widget(Paragraph::new(lines), inner);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WaveformWindow {
    start_bucket: usize,
    end_bucket: usize,
    zoom: usize,
}

impl WaveformWindow {
    fn full(overview: &WaveformOverview) -> Self {
        Self {
            start_bucket: 0,
            end_bucket: overview.buckets.len(),
            zoom: 1,
        }
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
    if state.active_view == TuiView::Sampler {
        render_sampler_view(frame, area, state.sampler_view);
        return;
    }
    if state.active_view == TuiView::SampleBrowser {
        render_sample_browser(frame, area, state.sample_browser);
        return;
    }
    if state.active_view == TuiView::ProjectBrowser {
        render_project_browser(frame, area, state.project_browser);
        return;
    }

    match layout_kind(area.width) {
        LayoutKind::Large => {
            let chunks = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([
                    Constraint::Length(TRACK_PANEL_WIDTH),
                    Constraint::Min(72),
                    Constraint::Length(42),
                ])
                .split(area);
            let side = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(chunks[0]);
            render_tracks(frame, side[0], song, state.cursor.track);
            render_sequence(frame, side[1], song, state.sequence_position);
            render_pattern_workspace(frame, chunks[1], chunks[2], song, state);
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

fn list_inner_height(area: Rect) -> usize {
    area.height.saturating_sub(2).into()
}

fn centered_scroll_offset(total_items: usize, active_index: usize, visible_items: usize) -> usize {
    if visible_items == 0 || total_items <= visible_items {
        return 0;
    }

    active_index
        .min(total_items.saturating_sub(1))
        .saturating_sub(visible_items / 2)
        .min(total_items.saturating_sub(visible_items))
}

fn ranged_title(label: &str, start: usize, end: usize, total: usize) -> String {
    if total > 0 && (start > 0 || end < total) {
        format!(" {label} {}-{} / {total} ", start + 1, end)
    } else {
        format!(" {label} ")
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
    let visible_items = list_inner_height(area);
    let start = centered_scroll_offset(song.tracks.len(), active_track, visible_items);
    let end = start.saturating_add(visible_items).min(song.tracks.len());
    let lines = song
        .tracks
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
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

    let tracks = Paragraph::new(lines).block(
        Block::default()
            .title(ranged_title("Tracks", start, end, song.tracks.len()))
            .borders(Borders::ALL),
    );
    frame.render_widget(tracks, area);
}

fn render_sequence(
    frame: &mut Frame<'_>,
    area: Rect,
    song: &Song,
    active_sequence_position: Option<usize>,
) {
    let visible_items = list_inner_height(area);
    let active_index = active_sequence_position.unwrap_or(0);
    let start = centered_scroll_offset(song.sequence.len(), active_index, visible_items);
    let end = start.saturating_add(visible_items).min(song.sequence.len());
    let lines = song
        .sequence
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
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

    let sequence = Paragraph::new(lines).block(
        Block::default()
            .title(ranged_title("Sequence", start, end, song.sequence.len()))
            .borders(Borders::ALL),
    );
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
        let footer_lines = 4;
        let visible_items = list_inner_height(area)
            .saturating_sub(1)
            .saturating_sub(footer_lines);
        let active_index = active_sequence_position.unwrap_or(0);
        let start = centered_scroll_offset(song.sequence.len(), active_index, visible_items);
        let end = start.saturating_add(visible_items).min(song.sequence.len());
        for (index, pattern_id) in song
            .sequence
            .iter()
            .enumerate()
            .skip(start)
            .take(end.saturating_sub(start))
        {
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

    let active_index = active_sequence_position.unwrap_or(0);
    let visible_items = list_inner_height(area).saturating_sub(5);
    let start = centered_scroll_offset(song.sequence.len(), active_index, visible_items);
    let end = start.saturating_add(visible_items).min(song.sequence.len());
    let sequence = Paragraph::new(lines)
        .block(
            Block::default()
                .title(ranged_title(
                    "Sequence Editor",
                    start,
                    end,
                    song.sequence.len(),
                ))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(sequence, area);
}

fn render_track_editor(frame: &mut Frame<'_>, area: Rect, song: &Song, active_track: usize) {
    let sections = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);

    render_track_mixer(frame, sections[0], song, active_track);
    render_instrument_matrix(frame, sections[1], song, active_track);
}

fn render_track_mixer(frame: &mut Frame<'_>, area: Rect, song: &Song, active_track: usize) {
    let inner_width = area.width.saturating_sub(2) as usize;
    let channel_width = if inner_width >= 96 { 12 } else { 10 };
    let visible_channels = (inner_width / channel_width).max(1);
    let start = visible_track_start(song.tracks.len(), active_track, visible_channels);
    let end = (start + visible_channels).min(song.tracks.len());
    let tracks = &song.tracks[start..end];
    let page = start / visible_channels + 1;
    let pages = song.tracks.len().div_ceil(visible_channels).max(1);
    let fader_rows = area.height.saturating_sub(7).max(3) as usize;
    let mut lines = Vec::with_capacity(fader_rows + 5);

    lines.push(Line::from(format!(
        "Track Mixer {page}/{pages} | master {} | tracks {:02}-{:02}/{}",
        format_gain_db(song.mixer.master_gain),
        start + 1,
        end,
        song.tracks.len()
    )));
    lines.push(channel_line(
        tracks
            .iter()
            .map(|track| truncate(&track.name, channel_width - 1)),
        channel_width,
    ));

    for row in 0..fader_rows {
        let mut spans = Vec::new();
        for (offset, track) in tracks.iter().enumerate() {
            let index = start + offset;
            let mixer = song.track_mixer_for_track(track.id);
            let fill_rows = gain_to_meter_rows(mixer.gain, fader_rows);
            let filled = fader_rows.saturating_sub(row) <= fill_rows;
            let marker = if filled {
                "  ███   "
            } else {
                "  │ │   "
            };
            spans.push(Span::styled(
                fixed_width(marker, channel_width),
                mixer_channel_style(index == active_track, mixer.muted || track.muted, filled),
            ));
        }
        lines.push(Line::from(spans));
    }

    lines.push(channel_line(
        tracks
            .iter()
            .map(|track| format_gain_db(song.track_mixer_for_track(track.id).gain)),
        channel_width,
    ));
    lines.push(channel_line(
        tracks.iter().map(|track| {
            let mixer = song.track_mixer_for_track(track.id);
            format!(
                "{}{} {:>+3.0}",
                if track.muted || mixer.muted { "M" } else { "-" },
                if track.solo || mixer.solo { "S" } else { "-" },
                mixer.pan * 100.0
            )
        }),
        channel_width,
    ));

    let mixer = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Track Mixer ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(mixer, area);
}

fn render_instrument_matrix(frame: &mut Frame<'_>, area: Rect, song: &Song, active_track: usize) {
    let mut lines = vec![Line::from(vec![
        Span::styled("TRK  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "NAME          INST          SAMPLE        CH  FLAGS   GAIN  PAN",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    for (index, track) in song.tracks.iter().enumerate() {
        let marker = if index == active_track { ">" } else { " " };
        let mixer = song.track_mixer_for_track(track.id);
        let instrument = song
            .instrument_for_track(track.id)
            .map_or("none", |instrument| instrument.name.as_str());
        let sample = song
            .sample_for_track(track.id)
            .map_or("none", |sample| sample.name.as_str());
        let flags = format!(
            "{}{}{}{}",
            if track.muted { "M" } else { "-" },
            if track.solo { "S" } else { "-" },
            if track.armed { "R" } else { "-" },
            if mixer.muted { "A" } else { "-" }
        );
        let line = format!(
            "{marker}{:02}  {:<12} {:<13} {:<12} CH{:02} {:<6} {:>5} {:+.2}",
            index + 1,
            truncate(&track.name, 12),
            truncate(instrument, 13),
            truncate(sample, 12),
            track.midi_channel,
            flags,
            format_gain_db(mixer.gain),
            mixer.pan
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
        Line::from("{/} reorder   M mute   S solo   :mixer gain|pan|mute|solo   :sample assign"),
    ]);

    let instruments = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Instruments ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(instruments, area);
}

fn channel_line(values: impl Iterator<Item = String>, channel_width: usize) -> Line<'static> {
    Line::from(
        values
            .map(|value| Span::from(fixed_width(&value, channel_width)))
            .collect::<Vec<_>>(),
    )
}

fn visible_track_start(track_count: usize, active_track: usize, visible_channels: usize) -> usize {
    if track_count <= visible_channels {
        return 0;
    }
    active_track
        .saturating_sub(visible_channels / 2)
        .min(track_count.saturating_sub(visible_channels))
}

fn gain_to_meter_rows(gain: f32, fader_rows: usize) -> usize {
    if fader_rows == 0 {
        return 0;
    }
    let normalized = (gain.max(0.0) / 2.0).min(1.0);
    (normalized * fader_rows as f32).round() as usize
}

fn format_gain_db(gain: f32) -> String {
    if gain <= 0.0 || !gain.is_finite() {
        "-inf dB".to_string()
    } else {
        format!("{:+.1}dB", 20.0 * gain.log10())
    }
}

fn mixer_channel_style(active: bool, muted: bool, filled: bool) -> Style {
    let foreground = if muted {
        Color::DarkGray
    } else if active && filled {
        Color::Yellow
    } else if active {
        Color::White
    } else if filled {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let style = Style::default().fg(foreground);
    if active {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
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

    let footer_lines = 4;
    let visible_items = list_inner_height(area)
        .saturating_sub(1)
        .saturating_sub(footer_lines);
    let start = centered_scroll_offset(song.patterns.len(), active_pattern, visible_items);
    let end = start.saturating_add(visible_items).min(song.patterns.len());

    for (index, pattern) in song
        .patterns
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
    {
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
                .title(ranged_title(
                    "Pattern Manager",
                    start,
                    end,
                    song.patterns.len(),
                ))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(patterns, area);
}

fn render_sampler_view(frame: &mut Frame<'_>, area: Rect, sampler: Option<SamplerViewState<'_>>) {
    let Some(sampler) = sampler else {
        let empty = Paragraph::new("No sample loaded")
            .block(Block::default().title(" Sampler ").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    };

    let sections = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Length(14), Constraint::Min(5)])
        .split(area);

    let overview = sampler.overview;
    let assignment = match (sampler.assigned_track, sampler.assigned_track_count) {
        (Some(track), 1) => format!("Assigned: {track}"),
        (Some(track), count) => format!("Assigned: {track} (+{})", count.saturating_sub(1)),
        (None, _) => "Assigned: none".to_string(),
    };
    let mut lines = vec![
        Line::from(format!("Name: {}", truncate(sampler.name, 48))),
        Line::from(format!("Path: {}", truncate(sampler.source_path, 72))),
        Line::from(format!(
            "Instrument: {}",
            sampler.instrument.unwrap_or("none")
        )),
        Line::from(assignment),
        Line::from(format!("Sample rate: {} Hz", overview.sample_rate)),
        Line::from(format!("Channels: {}", overview.channels)),
        Line::from(format!("Frames: {}", overview.frames)),
        Line::from(format!("Duration: {:.3} s", overview.duration_seconds)),
        parameter_control_from_f32(sample_gain_descriptor(), sampler.gain),
        Line::from(format!(
            "Window: {}..{}",
            format_optional_frame(sampler.start_frame),
            format_optional_frame(sampler.end_frame)
        )),
        Line::from(format!(
            "Loop: {} {}",
            sampler.playback_mode,
            format_loop_window(sampler.loop_start_frame, sampler.loop_end_frame)
        )),
    ];
    lines.push(render_sampler_envelope_controls(sampler));
    let metadata = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Sample Metadata ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(metadata, sections[0]);
    render_waveform_overview_with_window(
        frame,
        sections[1],
        overview,
        WaveformWindow {
            start_bucket: sampler.waveform_start_bucket,
            end_bucket: sampler.waveform_end_bucket,
            zoom: sampler.waveform_zoom,
        },
        WaveformGlyphs::Unicode,
    );
}

fn render_sampler_envelope_controls(sampler: SamplerViewState<'_>) -> Line<'static> {
    Line::from(vec![
        Span::raw("Envelope: "),
        sampler_envelope_span(
            SamplerEnvelopeField::Attack,
            sampler.selected_envelope,
            format!("A {:.3}s", sampler.envelope.0),
        ),
        Span::raw("  "),
        sampler_envelope_span(
            SamplerEnvelopeField::Decay,
            sampler.selected_envelope,
            format!("D {:.3}s", sampler.envelope.1),
        ),
        Span::raw("  "),
        sampler_envelope_span(
            SamplerEnvelopeField::Sustain,
            sampler.selected_envelope,
            format!("S {:.3}", sampler.envelope.2),
        ),
        Span::raw("  "),
        sampler_envelope_span(
            SamplerEnvelopeField::Release,
            sampler.selected_envelope,
            format!("R {:.3}s", sampler.envelope.3),
        ),
    ])
}

fn sampler_envelope_span(
    field: SamplerEnvelopeField,
    selected: SamplerEnvelopeField,
    text: String,
) -> Span<'static> {
    if field == selected {
        Span::styled(
            format!("[{text}]"),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(format!(" {text} "), Style::default().fg(Color::White))
    }
}

fn render_sample_browser(
    frame: &mut Frame<'_>,
    area: Rect,
    browser: Option<SampleBrowserViewState<'_>>,
) {
    let Some(browser) = browser else {
        let empty = Paragraph::new("Sample browser unavailable").block(
            Block::default()
                .title(" Sample Browser ")
                .borders(Borders::ALL),
        );
        frame.render_widget(empty, area);
        return;
    };

    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);
    let left = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(columns[0]);

    let path = Paragraph::new(truncate(
        browser.current_dir,
        columns[0].width.saturating_sub(4) as usize,
    ))
    .block(Block::default().title(" Directory ").borders(Borders::ALL));
    frame.render_widget(path, left[0]);

    let visible_rows = left[1].height.saturating_sub(2) as usize;
    let selected = browser
        .selected
        .min(browser.entries.len().saturating_sub(1));
    let mut viewport = ViewportAxis::new(browser.entries.len(), visible_rows);
    viewport.keep_visible(selected);
    let mut lines = Vec::new();

    if browser.entries.is_empty() {
        lines.push(Line::from("No files"));
    } else {
        for (index, entry) in browser
            .entries
            .iter()
            .enumerate()
            .skip(viewport.offset())
            .take(visible_rows)
        {
            let marker = if index == selected { ">" } else { " " };
            let icon = match entry.kind {
                SampleBrowserEntryKind::Directory => "[D]",
                SampleBrowserEntryKind::SupportedSample => "[W]",
                SampleBrowserEntryKind::UnsupportedFile => "[ ]",
            };
            let style = if index == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                match entry.kind {
                    SampleBrowserEntryKind::Directory => Style::default().fg(Color::Cyan),
                    SampleBrowserEntryKind::SupportedSample => Style::default().fg(Color::White),
                    SampleBrowserEntryKind::UnsupportedFile => Style::default().fg(Color::DarkGray),
                }
            };
            lines.push(Line::from(Span::styled(
                format!("{marker} {icon} {}", truncate(entry.name, 38)),
                style,
            )));
        }
    }

    let list = Paragraph::new(lines)
        .block(Block::default().title(" Samples ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(list, left[1]);
    if browser.entries.len() > visible_rows {
        let mut scrollbar_state = viewport.scrollbar_state();
        frame.render_stateful_widget(
            Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight),
            left[1],
            &mut scrollbar_state,
        );
    }

    if let Some(preview) = browser.preview {
        render_sampler_view(frame, columns[1], Some(preview));
    } else {
        let message = browser.message.unwrap_or("Select a WAV file to preview it");
        let preview = Paragraph::new(message)
            .block(Block::default().title(" Preview ").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(preview, columns[1]);
    }
}

fn render_project_browser(
    frame: &mut Frame<'_>,
    area: Rect,
    browser: Option<ProjectBrowserViewState<'_>>,
) {
    let Some(browser) = browser else {
        let empty = Paragraph::new("Project browser unavailable").block(
            Block::default()
                .title(" Project Browser ")
                .borders(Borders::ALL),
        );
        frame.render_widget(empty, area);
        return;
    };

    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(area);
    let left = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(columns[0]);

    let path = Paragraph::new(truncate(
        browser.current_dir,
        columns[0].width.saturating_sub(4) as usize,
    ))
    .block(Block::default().title(" Directory ").borders(Borders::ALL));
    frame.render_widget(path, left[0]);

    let visible_rows = left[1].height.saturating_sub(2) as usize;
    let selected = browser
        .selected
        .min(browser.entries.len().saturating_sub(1));
    let start = selected.saturating_sub(visible_rows.saturating_sub(1));
    let mut lines = Vec::new();

    if browser.entries.is_empty() {
        lines.push(Line::from("No projects"));
    } else {
        for (index, entry) in browser
            .entries
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_rows)
        {
            let marker = if index == selected { ">" } else { " " };
            let icon = match entry.kind {
                ProjectBrowserEntryKind::Directory => "[D]",
                ProjectBrowserEntryKind::RecentProject => "[R]",
                ProjectBrowserEntryKind::Project => "[S]",
                ProjectBrowserEntryKind::MissingProject => "[!]",
                ProjectBrowserEntryKind::InvalidProject => "[X]",
            };
            let style = if index == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                match entry.kind {
                    ProjectBrowserEntryKind::Directory => Style::default().fg(Color::Cyan),
                    ProjectBrowserEntryKind::RecentProject => Style::default().fg(Color::Green),
                    ProjectBrowserEntryKind::Project => Style::default().fg(Color::White),
                    ProjectBrowserEntryKind::MissingProject
                    | ProjectBrowserEntryKind::InvalidProject => Style::default().fg(Color::Red),
                }
            };
            lines.push(Line::from(Span::styled(
                format!("{marker} {icon} {}", truncate(entry.name, 42)),
                style,
            )));
        }
    }

    let list = Paragraph::new(lines)
        .block(Block::default().title(" Projects ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(list, left[1]);

    let mut detail_lines = Vec::new();
    if let Some(entry) = browser.entries.get(selected) {
        detail_lines.push(Line::from(vec![
            Span::styled("Name ", Style::default().fg(Color::Yellow)),
            Span::raw(entry.name.to_string()),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("Type ", Style::default().fg(Color::Yellow)),
            Span::raw(match entry.kind {
                ProjectBrowserEntryKind::Directory => "directory",
                ProjectBrowserEntryKind::RecentProject => "recent project",
                ProjectBrowserEntryKind::Project => "project",
                ProjectBrowserEntryKind::MissingProject => "missing project",
                ProjectBrowserEntryKind::InvalidProject => "invalid project",
            }),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("Path ", Style::default().fg(Color::Yellow)),
            Span::raw(entry.path.to_string()),
        ]));
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(entry.detail.to_string()));
    } else {
        detail_lines.push(Line::from(browser.message.unwrap_or("No project selected")));
    }
    if let Some(message) = browser.message {
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(Span::styled(
            message.to_string(),
            Style::default().fg(Color::Yellow),
        )));
    }

    let details = Paragraph::new(detail_lines)
        .block(Block::default().title(" Details ").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(details, columns[1]);
}

fn render_pattern_workspace(
    frame: &mut Frame<'_>,
    pattern_area: Rect,
    inspector_area: Rect,
    song: &Song,
    state: TuiState<'_>,
) {
    let main = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Min(12), Constraint::Length(10)])
        .split(pattern_area);
    render_pattern(frame, main[0], song, state);
    render_track_properties(frame, main[1], song, state);
    render_instrument_sidebar(frame, inspector_area, song, state);
}

fn render_track_properties(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let Some(track) = song.tracks.get(state.cursor.track) else {
        let empty = Paragraph::new("No track").block(
            Block::default()
                .title(" Track Properties ")
                .borders(Borders::ALL),
        );
        frame.render_widget(empty, area);
        return;
    };
    let mixer = song.track_mixer_for_track(track.id);
    let instrument = song
        .instrument_for_track(track.id)
        .map_or("none", |instrument| instrument.name.as_str());
    let sample = song
        .sample_for_track(track.id)
        .map_or("none", |sample| sample.name.as_str());
    let cell = active_pattern(song, state.pattern_index).and_then(|pattern| {
        pattern
            .rows
            .get(state.cursor.row)
            .and_then(|row| row.cells.get(state.cursor.track))
    });

    let block = Block::default().title(" Track Desk ").borders(Borders::ALL);
    let inner = Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    frame.render_widget(block, area);

    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([
            Constraint::Percentage(32),
            Constraint::Percentage(34),
            Constraint::Min(24),
        ])
        .split(inner);

    let track_flags = format!(
        "{}{}{}",
        if track.muted { "M" } else { "-" },
        if track.solo { "S" } else { "-" },
        if track.armed { "R" } else { "-" },
    );
    let audio_flags = format!(
        "{}{}",
        if mixer.muted { "M" } else { "-" },
        if mixer.solo { "S" } else { "-" },
    );
    let track_lines = vec![
        Line::from(vec![
            Span::styled(
                "TRACK ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:02}", state.cursor.track + 1),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(format!("Name   {}", truncate(&track.name, 22))),
        Line::from(format!("Inst   {}", truncate(instrument, 22))),
        Line::from(format!("Samp {}", truncate(sample, 18))),
        Line::from(format!("CH{:02} Track {}", track.midi_channel, track_flags)),
        Line::from(format!("Audio {}", audio_flags)),
        parameter_control_from_f32(mixer_master_gain_descriptor(), song.mixer.master_gain),
    ];
    frame.render_widget(
        Paragraph::new(track_lines).wrap(Wrap { trim: true }),
        columns[0],
    );

    let mut mixer_lines = vec![
        Line::from(Span::styled(
            "MIXER",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        parameter_control_from_f32(mixer_track_gain_descriptor(), mixer.gain),
        parameter_control_from_f32(mixer_track_pan_descriptor(), mixer.pan),
        parameter_control_from_f32(mixer_master_gain_descriptor(), song.mixer.master_gain),
        Line::from(format!(
            "Sends {:02}   FX {:02}",
            mixer.sends.len(),
            mixer.effects.len()
        )),
    ];
    for effect in &mixer.effects {
        match effect.kind {
            EffectDeviceKind::Gain { gain } => {
                mixer_lines.push(parameter_control_from_f32(native_gain_descriptor(), gain))
            }
            EffectDeviceKind::Pan { pan } => {
                mixer_lines.push(parameter_control_from_f32(native_pan_descriptor(), pan))
            }
        }
    }
    mixer_lines.extend([
        Line::from(":mixer gain pan"),
        Line::from(":dsp track clear"),
    ]);
    frame.render_widget(
        Paragraph::new(mixer_lines).wrap(Wrap { trim: true }),
        columns[1],
    );

    let mut cell_lines = vec![Line::from(vec![
        Span::styled(
            "CELL ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::from(format!("r{:02} {}", state.cursor.row, state.cursor.field)),
    ])];
    if let Some(cell) = cell {
        let [first, second] = format_cell_summary_lines(cell);
        cell_lines.push(Line::from(first));
        cell_lines.push(Line::from(second));
    } else {
        cell_lines.push(Line::from("empty"));
    }
    cell_lines.extend([
        Line::from(":sample assign"),
        Line::from(":cell instrument 01"),
        Line::from("C-j Sampler  F9 Tracks"),
    ]);
    frame.render_widget(
        Paragraph::new(cell_lines).wrap(Wrap { trim: true }),
        columns[2],
    );
}

fn render_instrument_sidebar(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let sections = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Min(5),
        ])
        .split(area);
    render_instrument_list(frame, sections[0], song, state.cursor.track);
    render_sample_list(frame, sections[1], song, state.cursor.track);
    render_selected_track_inspector(frame, sections[2], song, state);
}

fn render_instrument_list(frame: &mut Frame<'_>, area: Rect, song: &Song, active_track: usize) {
    let active_instrument = song
        .tracks
        .get(active_track)
        .and_then(|track| song.instrument_for_track(track.id))
        .map(|instrument| instrument.id);
    let mut lines = vec![Line::from(vec![
        Span::styled("#  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "INSTRUMENT",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])];
    if song.instruments.is_empty() {
        lines.push(Line::from("No instruments"));
    } else {
        for instrument in song
            .instruments
            .iter()
            .take(area.height.saturating_sub(4) as usize)
        {
            let sample_name = instrument
                .sample
                .and_then(|sample| song.sample_for_id(sample))
                .map_or("-", |sample| sample.name.as_str());
            let marker = if Some(instrument.id) == active_instrument {
                ">"
            } else {
                " "
            };
            lines.push(Line::from(format!(
                "{marker}{:02} {:<16} {}",
                instrument.id.0,
                truncate(&instrument.name, 16),
                truncate(sample_name, 12)
            )));
        }
    }
    let list = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Instruments ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(list, area);
}

fn render_sample_list(frame: &mut Frame<'_>, area: Rect, song: &Song, active_track: usize) {
    let active_sample = song
        .tracks
        .get(active_track)
        .and_then(|track| song.sample_for_track(track.id))
        .map(|sample| sample.id);
    let mut lines = vec![Line::from(vec![
        Span::styled("#  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "SAMPLE",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])];
    if song.samples.is_empty() {
        lines.push(Line::from("No samples loaded"));
    } else {
        for sample in song
            .samples
            .iter()
            .take(area.height.saturating_sub(4) as usize)
        {
            let marker = if Some(sample.id) == active_sample {
                ">"
            } else {
                " "
            };
            lines.push(Line::from(format!(
                "{marker}{:02} {:<18} root {}",
                sample.id.0,
                truncate(&sample.name, 18),
                sample.root_pitch
            )));
        }
    }
    let list = Paragraph::new(lines)
        .block(Block::default().title(" Samples ").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(list, area);
}

fn render_selected_track_inspector(
    frame: &mut Frame<'_>,
    area: Rect,
    song: &Song,
    state: TuiState<'_>,
) {
    let Some(track) = song.tracks.get(state.cursor.track) else {
        return;
    };
    let mixer = song.track_mixer_for_track(track.id);
    let sample = song.sample_for_track(track.id);
    let mut lines = vec![
        Line::from(format!(
            "Track {:02}: {}",
            state.cursor.track + 1,
            track.name
        )),
        parameter_control_from_f32(mixer_track_gain_descriptor(), mixer.gain),
        parameter_control_from_f32(mixer_track_pan_descriptor(), mixer.pan),
        Line::from(format!("MIDI CH{:02}", track.midi_channel)),
    ];
    if let Some(sample) = sample {
        lines.extend([
            Line::from(format!("Sample: {}", truncate(&sample.name, 24))),
            Line::from(format!("Root: {}", sample.root_pitch)),
            parameter_control_from_f32(sample_gain_descriptor(), sample.gain),
            Line::from(format!(
                "Window: {}..{}",
                format_optional_frame(sample.playback.start_frame),
                format_optional_frame(sample.playback.end_frame)
            )),
            Line::from(format!(
                "Loop: {} {}",
                match sample.playback.mode {
                    SamplePlaybackMode::OneShot => "one-shot",
                    SamplePlaybackMode::Loop => "loop",
                },
                format_loop_window(
                    sample.playback.loop_start_frame,
                    sample.playback.loop_end_frame
                )
            )),
        ]);
    } else {
        lines.push(Line::from("No sample assigned"));
    }
    let inspector = Paragraph::new(lines)
        .block(Block::default().title(" Inspector ").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(inspector, area);
}

fn parameter_control_from_f32(descriptor: ParameterDescriptor, value: f32) -> Line<'static> {
    let value = descriptor.value_from_f32(value);
    parameter_control_line(&descriptor, value)
}

fn parameter_control_line(
    descriptor: &ParameterDescriptor,
    value: ParameterValue,
) -> Line<'static> {
    let label = descriptor
        .short_name
        .as_deref()
        .unwrap_or(descriptor.name.as_str());
    let value_label = if descriptor.validate(&value).is_ok() {
        descriptor.format_value(&value)
    } else {
        format!(
            "invalid -> {}",
            descriptor.format_value(&descriptor.clamp(&value))
        )
    };
    Line::from(vec![
        Span::styled(
            format!("{:<6}", truncate(label, 6)),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(
            format!("{:>9} ", truncate(&value_label, 9)),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            parameter_meter(descriptor, &value, 10),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!(" {}", parameter_flags_label(descriptor)),
            Style::default().fg(Color::DarkGray),
        ),
    ])
}

fn parameter_meter(
    descriptor: &ParameterDescriptor,
    value: &ParameterValue,
    width: usize,
) -> String {
    if descriptor.flags.bipolar {
        return pan_meter(value.as_f32().unwrap_or_default(), width);
    }
    match descriptor.plain_to_normalized(value) {
        Ok(normalized) => normalized_meter(normalized, width),
        Err(_) => "·".repeat(width),
    }
}

fn normalized_meter(normalized: f32, width: usize) -> String {
    let fill = (normalized.clamp(0.0, 1.0) * width as f32).round() as usize;
    format!(
        "{}{}",
        "█".repeat(fill),
        "─".repeat(width.saturating_sub(fill))
    )
}

fn parameter_flags_label(descriptor: &ParameterDescriptor) -> &'static str {
    if descriptor.flags.read_only {
        "read"
    } else if descriptor.flags.automatable {
        "auto"
    } else if descriptor.flags.modulatable {
        "mod"
    } else {
        "manual"
    }
}

fn pan_meter(pan: f32, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let center = width / 2;
    let mut chars = vec!['─'; width];
    chars[center] = '│';
    let position =
        (((pan.clamp(-1.0, 1.0) + 1.0) / 2.0) * (width.saturating_sub(1)) as f32).round() as usize;
    chars[position.min(width - 1)] = '●';
    chars.into_iter().collect()
}

fn format_cell_summary_lines(cell: &PatternCell) -> [String; 2] {
    let note = match cell.note {
        Some(NoteEvent::Note { pitch }) => format_note(pitch),
        Some(NoteEvent::NoteOff) => "OFF".to_string(),
        Some(NoteEvent::NoteCut) => "CUT".to_string(),
        None => "---".to_string(),
    };
    let velocity = cell
        .velocity
        .map_or_else(|| "--".to_string(), |value| format!("{value:02X}"));
    let instrument = cell.instrument.map_or_else(
        || "--".to_string(),
        |instrument| format!("{:02X}", instrument.0.min(0xff)),
    );
    let volume = cell
        .volume
        .map_or_else(|| "--".to_string(), |value| format!("{value:02X}"));
    let pan = cell
        .pan
        .map_or_else(|| "--".to_string(), |value| format!("{value:02X}"));
    let delay = cell
        .delay
        .map_or_else(|| "--".to_string(), |value| format!("{value:02X}"));
    let command = cell.command.map_or_else(
        || "---".to_string(),
        |command| format!("{}{:02X}", command.display_code(), command.value),
    );
    [
        format!("Note {note} Vel {velocity} Inst {instrument}"),
        format!("Vol {volume} Pan {pan} Dly {delay} FX {command}"),
    ]
}

fn render_pattern(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let Some(pattern) = active_pattern(song, state.pattern_index) else {
        let empty = Paragraph::new("No pattern")
            .block(Block::default().title(" Pattern ").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    };

    let viewport = pattern_viewport(area, pattern.row_count(), song.tracks.len(), state);
    let mut lines = Vec::with_capacity(viewport.row_capacity.saturating_add(1));
    lines.push(pattern_header(
        song,
        state.cursor.track,
        viewport.visible_tracks.clone(),
    ));

    let row_state = PatternRowRenderState {
        cursor: state.cursor,
        playhead_row: state.playhead_row,
        selection: state.selection,
        show_line_numbers_hex: state.show_line_numbers_hex,
        visible_tracks: viewport.visible_tracks,
    };

    for row_index in viewport.visible_rows {
        lines.push(pattern_row(song, pattern, row_index, &row_state));
    }

    let block = Block::default()
        .title(" Pattern Editor ")
        .borders(Borders::ALL);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PatternViewport {
    visible_rows: Range<usize>,
    visible_tracks: Range<usize>,
    row_capacity: usize,
}

fn pattern_viewport(
    area: Rect,
    row_count: usize,
    track_count: usize,
    state: TuiState<'_>,
) -> PatternViewport {
    let inner_height = area.height.saturating_sub(2) as usize;
    let row_capacity = inner_height.saturating_sub(1);
    let visible_track_capacity = visible_pattern_tracks(area.width);

    let mut rows = ViewportAxis::with_offset(row_count, row_capacity, state.row_offset);
    rows.keep_visible(state.cursor.row);
    let mut tracks =
        ViewportAxis::with_offset(track_count, visible_track_capacity, state.track_offset);
    tracks.keep_visible(state.cursor.track);

    PatternViewport {
        visible_rows: rows.visible_range(),
        visible_tracks: tracks.visible_range(),
        row_capacity,
    }
}

fn active_pattern(song: &Song, pattern_index: usize) -> Option<&Pattern> {
    song.pattern(pattern_index)
}

fn visible_pattern_tracks(area_width: u16) -> usize {
    let content_width = area_width
        .saturating_sub(2)
        .saturating_sub(ROW_GUTTER_WIDTH as u16);
    (content_width as usize).div_ceil(PATTERN_CELL_WIDTH).max(1)
}

fn pattern_header(song: &Song, active_track: usize, visible_tracks: Range<usize>) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{:<ROW_GUTTER_WIDTH$}", "ROW"),
        Style::default().fg(Color::DarkGray),
    )];

    for (track_index, track) in song
        .tracks
        .iter()
        .enumerate()
        .skip(visible_tracks.start)
        .take(visible_tracks.end.saturating_sub(visible_tracks.start))
    {
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

struct PatternRowRenderState {
    cursor: Cursor,
    playhead_row: Option<usize>,
    selection: Option<SelectionRect>,
    show_line_numbers_hex: bool,
    visible_tracks: Range<usize>,
}

fn pattern_row(
    song: &Song,
    pattern: &Pattern,
    row_index: usize,
    state: &PatternRowRenderState,
) -> Line<'static> {
    let is_playhead = state.playhead_row == Some(row_index);
    let mut spans = vec![Span::styled(
        format!(
            "{:<ROW_GUTTER_WIDTH$}",
            format!(
                "{}{}",
                if is_playhead { ">" } else { " " },
                format_row_number(row_index, state.show_line_numbers_hex)
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

    for track_index in state.visible_tracks.clone() {
        if track_index >= song.tracks.len() {
            break;
        }
        let cell = row.cells.get(track_index).cloned().unwrap_or_default();
        let is_cursor_row = state.cursor.row == row_index;
        let is_cursor_cell = is_cursor_row && state.cursor.track == track_index;
        let is_active_track = state.cursor.track == track_index;
        let is_selected = state
            .selection
            .is_some_and(|selection| selection.contains(row_index, track_index));
        spans.extend(cell_spans(
            &cell,
            state.cursor.field,
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
    let instrument = cell.instrument.map_or_else(
        || "--".to_string(),
        |instrument| format!("{:02X}", instrument.0.min(0xff)),
    );
    let volume = cell
        .volume
        .map_or_else(|| "--".to_string(), |value| format!("{value:02X}"));
    let pan = cell
        .pan
        .map_or_else(|| "--".to_string(), |value| format!("{value:02X}"));
    let delay = cell
        .delay
        .map_or_else(|| "--".to_string(), |value| format!("{value:02X}"));
    let command = cell.command.map_or_else(
        || "---".to_string(),
        |command| format!("{}{:02X}", command.display_code(), command.value),
    );

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
    let style_for_field = |field| {
        if focused && focused_field == field {
            focused_style
        } else if selected {
            selected_style
        } else if playing {
            playing_style
        } else if active_track {
            active_track_style
        } else {
            normal
        }
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

    vec![
        Span::styled(" ", spacer_style),
        Span::styled(note, style_for_field(CellField::Note)),
        Span::styled(" ", spacer_style),
        Span::styled(velocity, style_for_field(CellField::Velocity)),
        Span::styled(" ", spacer_style),
        Span::styled(instrument, style_for_field(CellField::Instrument)),
        Span::styled(" ", spacer_style),
        Span::styled(volume, style_for_field(CellField::Volume)),
        Span::styled(" ", spacer_style),
        Span::styled(pan, style_for_field(CellField::Pan)),
        Span::styled(" ", spacer_style),
        Span::styled(delay, style_for_field(CellField::Delay)),
        Span::styled(" ", spacer_style),
        Span::styled(command, style_for_field(CellField::Effect)),
    ]
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
    } else if state.active_view == TuiView::Sampler {
        format!(
            " {} | H Help | Esc Pattern | Tab ADSR | [/]/{{/}} Adjust | +/- Zoom | Left/Right Pan | b Browse | F7 Sequence | F9 Tracks | F10 Patterns | : Command | Ctrl+S Save | q Quit ",
            state.mode_label
        )
    } else if state.active_view == TuiView::SampleBrowser {
        format!(
            " {} | H Help | Esc Sampler | Up/Down Select | Enter Load/Open | Backspace Parent | : Command | q Quit ",
            state.mode_label
        )
    } else if state.active_view == TuiView::ProjectBrowser {
        format!(
            " {} | H Help | Esc Tracker | Up/Down Select | Enter Open | Backspace Parent | r Refresh | : Command | q Quit ",
            state.mode_label
        )
    } else {
        format!(
            " {}{} | H Help | Focus :t/:p/:se/:tr/:sa/:sb/:o | F4 MIDI | Space Play/Stop | Enter Row | Shift+Enter Seq | L Loop | N/P/X Pattern | A/Y/R Seq | {{/}} Track | : Command | i Edit | V Select | Ctrl+S Save | q Quit ",
            state.mode_label,
            if state.selection.is_some() { " SEL" } else { "" }
        )
    };
    let status = Paragraph::new(text);
    frame.render_widget(status, area);
}

fn render_help_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    mode_label: &str,
    scroll: usize,
    tab: HelpTab,
) {
    let overlay = large_overlay_rect(area);
    let visible_rows = overlay.height.saturating_sub(2) as usize;
    let lines = help_lines(mode_label, tab);
    let line_count = lines.len();
    let mut viewport = ViewportAxis::with_offset(lines.len(), visible_rows, scroll);
    viewport.clamp();
    let max_scroll = viewport.max_offset();
    let scroll = viewport.offset();
    let title = if max_scroll == 0 {
        format!(" Help: {} | Tab/Shift+Tab pages ", tab.label())
    } else {
        format!(
            " Help: {} {}/{} | Tab/Shift+Tab pages ",
            tab.label(),
            scroll + 1,
            max_scroll + 1
        )
    };

    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    frame.render_widget(Clear, overlay);
    frame.render_widget(paragraph, overlay);
    if line_count > visible_rows {
        let mut scrollbar_state = viewport.scrollbar_state();
        frame.render_stateful_widget(
            Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight),
            overlay,
            &mut scrollbar_state,
        );
    }
}

fn help_lines(mode_label: &str, tab: HelpTab) -> Vec<Line<'static>> {
    let mut lines = vec![
        help_tab_line(tab),
        Line::from("  Tab/Right next page   Shift+Tab/Left previous page   Up/Down scroll"),
        Line::from(""),
    ];

    match tab {
        HelpTab::Basics => lines.extend(help_basics_lines(mode_label)),
        HelpTab::Editing => lines.extend(help_editing_lines(mode_label)),
        HelpTab::Sampler => lines.extend(help_sampler_lines(mode_label)),
        HelpTab::Midi => lines.extend(help_midi_lines(mode_label)),
        HelpTab::Commands => lines.extend(help_command_lines(mode_label)),
    }

    lines
}

fn help_tab_line(active: HelpTab) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, tab) in HelpTab::ALL.iter().copied().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" | "));
        }
        let style = if tab == active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        spans.push(Span::styled(format!(" {} ", tab.label()), style));
    }
    Line::from(spans)
}

fn help_basics_lines(mode_label: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "Global",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?/H Help   :h/:help Help   q Quit   Space Play/Stop   Shift+Space Start"),
        Line::from("  Enter Play Row   Shift+Enter Play Sequence From Cursor   L Loop   F8 Stop"),
        Line::from("  F7 Sequence View   F9 Track View   F10 Pattern View   Ctrl+J Sampler View"),
        Line::from("  :t Tracker   :p Patterns   :se Sequence   :tr Tracks   :sa Sampler   :sb Browser"),
        Line::from("  Esc returns from focused views"),
        Line::from("  :play pattern from start   :play sequence arrangement"),
        Line::from("  Ctrl+S Save   Ctrl+Shift+S Save As   Ctrl+Z Undo   Ctrl+Y Redo   Ctrl+Arrows BPM/LPB"),
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
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ]
}

fn help_editing_lines(mode_label: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "Editing",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  i Edit   Esc Normal   Del/Backspace clear cell   Ctrl+C/X/V cell clipboard"),
        Line::from("  V select region   Esc cancel selection   Delete clears selection"),
        Line::from("  Insert row   Ctrl+Delete delete row   F1/- octave down"),
        Line::from("  F2/+/= octave up   VEL/INST/VOL/PAN/DLY/FX accept two hex digits"),
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
            "Patterns And Sequence",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(
            "  N new pattern   P duplicate pattern   X delete pattern   F3 rename   F6 length",
        ),
        Line::from("  Pattern view: 1/2/3/4/5 set length 16/32/64/128/256"),
        Line::from("  A add current pattern to sequence   ,/. move sequence cursor"),
        Line::from("  Y duplicate sequence position   R remove   T set to current pattern"),
        Line::from("  </> move selected sequence position up/down"),
        Line::from(""),
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ]
}

fn help_sampler_lines(mode_label: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "Sampler And Instruments",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Ctrl+J opens Sampler view   Esc returns to Pattern view"),
        Line::from("  In Sampler view: +/- zoom waveform   Left/Right pan   Home/End bounds"),
        Line::from("  Tab/Shift+Tab selects A/D/S/R   [/]/{/} adjusts selected envelope field"),
        Line::from("  :sample view PATH loads a WAV and shows metadata plus waveform"),
        Line::from("  :sample browse [DIR] opens the in-app sample browser"),
        Line::from("  :sample choose [DIR] opens the configured external chooser"),
        Line::from(""),
        Line::from(Span::styled(
            "Track Assignment",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  :sample assign [TRACK] assigns the loaded sample to a track"),
        Line::from("  :sample replace [TRACK] swaps the track sample and prunes the old reference"),
        Line::from("  :sample unassign [TRACK] clears the track sample and instrument assignment"),
        Line::from("  TRACK is 1-based; omitted TRACK means the current track"),
        Line::from("  :sample assignments lists track=sample mappings"),
        Line::from(""),
        Line::from(Span::styled(
            "Instrument Column",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Assigning a sample creates a sample-backed instrument for that track"),
        Line::from("  Cells can override the track default with INST, e.g. :cell instrument 01"),
        Line::from("  An empty INST field uses the track instrument or sample assignment"),
        Line::from(""),
        Line::from(Span::styled(
            "Playback Window",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  :sample start FRAME|clear   :sample end FRAME|clear"),
        Line::from("  :sample loop START END   :sample loop off"),
        Line::from("  :sample envelope ATTACK DECAY SUSTAIN RELEASE"),
        Line::from("  :sample settings shows mode, frame window, loop and envelope"),
        Line::from(""),
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ]
}

fn help_midi_lines(mode_label: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "MIDI",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  F4 or :midi outputs opens MIDI settings and lists output ports"),
        Line::from("  In MIDI settings: arrows select, Enter connects, F5/r refresh, p panic"),
        Line::from("  CLI fallback: salieri --list-midi-outputs, then :midi connect 0"),
        Line::from("  Input: salieri --list-midi-inputs, then :midi-input connect 0"),
        Line::from("  :midi-input record on captures note-on events into the current pattern"),
        Line::from("  :midi-input clock on follows MIDI start/continue/stop transport"),
        Line::from("  Press Space or run :play pattern to send notes to the connected output"),
        Line::from("  :midi disconnect closes the output   :midi panic sends All Notes Off"),
        Line::from("  Use :track channel 2 10 to set track 02 to MIDI channel 10"),
        Line::from("  Config: [midi] default_output/default_input auto-connect by name"),
        Line::from(""),
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ]
}

fn help_command_lines(mode_label: &str) -> Vec<Line<'static>> {
    vec![
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
        Line::from(
            "  Panel focus: :t tracker   :p patterns   :se sequence   :tr tracks   :sa sampler",
        ),
        Line::from("  :sb [DIR] sample browser   :focus [t|p|se|tr|sa|sb]   :layout multi-panel"),
        Line::from("  Dirty quit asks: [Y]es save, [N]o quit, [C]ancel"),
        Line::from("  :track new   :track duplicate 2   :track delete 2   :track move 2 3"),
        Line::from("  :track mute 2   :track solo 2   :track rename Acid Bass"),
        Line::from("  :track channel 12   :fx D 20 delay   :fx R 04 retrigger   :fx clear"),
        Line::from("  :cell instrument 01   :cell volume 40   :cell pan 7F   :cell delay 20"),
        Line::from("  :cell effect R 04   :cell FIELD clear"),
        Line::from("  :dsp track 2 gain 0.5   :dsp master gain 0.8   :dsp track 2 clear"),
        Line::from("  :ai propose PROMPT   :ai show   :ai accept   :ai reject"),
        Line::from("  :play pattern   :play sequence [position]   :stop"),
        Line::from("  :tasks   :task cancel ID"),
        Line::from("  :pattern new   :pattern duplicate   :pattern delete   :pattern length 128"),
        Line::from("  :pattern rename Intro   :pattern 1   [ previous pattern   ] next pattern"),
        Line::from("  :sequence add   :sequence remove 0   :sequence duplicate 0"),
        Line::from("  :sequence set 0 2   :sequence move 1 0"),
        Line::from(""),
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ]
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
        .block(Block::default().title(" Confirm ").borders(Borders::ALL))
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

fn large_overlay_rect(area: Rect) -> Rect {
    let horizontal_margin = if area.width >= 120 { 6 } else { 2 };
    let vertical_margin = if area.height >= 32 { 3 } else { 1 };
    let width = area.width.saturating_sub(horizontal_margin * 2).max(20);
    let height = area.height.saturating_sub(vertical_margin * 2).max(8);
    Rect {
        x: area.x + horizontal_margin.min(area.width.saturating_sub(1)),
        y: area.y + vertical_margin.min(area.height.saturating_sub(1)),
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn waveform_lines(
    overview: &WaveformOverview,
    window: WaveformWindow,
    width: usize,
    height: usize,
    glyphs: WaveformGlyphs,
) -> Vec<Line<'static>> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let show_metadata = height >= 3;
    let show_ruler = height >= 5;
    let waveform_height = height
        .saturating_sub(usize::from(show_metadata))
        .saturating_sub(usize::from(show_ruler));
    let window = clamp_waveform_window(overview, window);
    let mut lines = Vec::with_capacity(height);
    if show_metadata {
        lines.push(Line::from(fixed_width(
            &waveform_metadata_label(overview, window, width),
            width,
        )));
    }
    if show_ruler {
        lines.push(Line::from(waveform_ruler_label(overview, window, width)));
    }

    if waveform_height == 0 {
        return lines;
    }

    let visible_buckets = &overview.buckets[window.start_bucket..window.end_bucket];
    if visible_buckets.is_empty() {
        lines.extend(empty_waveform_lines(width, waveform_height));
        return lines;
    }

    let subrow_height = waveform_height.saturating_mul(2);
    let mut grid = vec![vec![WaveformCell::default(); width]; waveform_height];
    for (x, bucket) in (0..width).map(|x| (x, waveform_column_bucket(visible_buckets, x, width))) {
        let min = sanitize_waveform_value(bucket.min);
        let max = sanitize_waveform_value(bucket.max);
        if min == 0.0 && max == 0.0 {
            grid[waveform_row(0.0, waveform_height)][x].baseline = true;
            continue;
        }

        let top = waveform_subrow(max, subrow_height);
        let bottom = waveform_subrow(min, subrow_height);
        let (top, bottom) = if top <= bottom {
            (top, bottom)
        } else {
            (bottom, top)
        };
        for subrow in top..=bottom {
            let row = (subrow / 2).min(waveform_height - 1);
            if subrow % 2 == 0 {
                grid[row][x].upper = true;
            } else {
                grid[row][x].lower = true;
            }
        }
    }

    lines.extend(grid.into_iter().map(|row| {
        Line::from(
            row.into_iter()
                .map(|cell| cell.to_char(glyphs))
                .collect::<String>(),
        )
    }));
    lines
}

#[derive(Debug, Clone, Copy, Default)]
struct WaveformCell {
    upper: bool,
    lower: bool,
    baseline: bool,
}

impl WaveformCell {
    const fn to_char(self, glyphs: WaveformGlyphs) -> char {
        match (self.upper, self.lower, self.baseline) {
            (true, true, _) => glyphs.full(),
            (true, false, _) => glyphs.upper(),
            (false, true, _) => glyphs.lower(),
            (false, false, true) => glyphs.baseline(),
            (false, false, false) => ' ',
        }
    }
}

fn clamp_waveform_window(overview: &WaveformOverview, window: WaveformWindow) -> WaveformWindow {
    let bucket_count = overview.buckets.len();
    if bucket_count == 0 {
        return WaveformWindow {
            start_bucket: 0,
            end_bucket: 0,
            zoom: window.zoom.max(1),
        };
    }

    let start = window.start_bucket.min(bucket_count - 1);
    let end = window.end_bucket.min(bucket_count).max(start + 1);
    WaveformWindow {
        start_bucket: start,
        end_bucket: end,
        zoom: window.zoom.max(1),
    }
}

fn waveform_metadata_label(
    overview: &WaveformOverview,
    window: WaveformWindow,
    width: usize,
) -> String {
    let total = format_time(overview.duration_seconds);
    let start = format_time(bucket_position_time_seconds(
        overview,
        window.start_bucket as f32,
    ));
    let end = format_time(bucket_position_time_seconds(
        overview,
        window.end_bucket as f32,
    ));
    if width < 72 {
        return format!(
            "{}fr {} | {}-{} z{}x",
            overview.frames, total, start, end, window.zoom
        );
    }

    format!(
        "{}fr {} {}Hz {}ch | view {}-{} zoom {}x",
        overview.frames, total, overview.sample_rate, overview.channels, start, end, window.zoom
    )
}

fn waveform_ruler_label(
    overview: &WaveformOverview,
    window: WaveformWindow,
    width: usize,
) -> String {
    if width == 0 {
        return String::new();
    }

    let mut chars = vec![' '; width];
    overlay_ruler_label(
        &mut chars,
        0,
        &format!(
            "|{}",
            format_time(bucket_position_time_seconds(
                overview,
                window.start_bucket as f32
            ))
        ),
    );
    overlay_ruler_label(
        &mut chars,
        width / 2,
        &format!(
            "|{}",
            format_time(bucket_position_time_seconds(
                overview,
                (window.start_bucket + window.end_bucket) as f32 / 2.0
            ))
        ),
    );
    overlay_ruler_label(
        &mut chars,
        width.saturating_sub(1),
        &format!(
            "{}|",
            format_time(bucket_position_time_seconds(
                overview,
                window.end_bucket as f32
            ))
        ),
    );
    chars.into_iter().collect()
}

fn overlay_ruler_label(chars: &mut [char], anchor: usize, label: &str) {
    if chars.is_empty() {
        return;
    }
    let label_width = label.chars().count();
    let start = if anchor + label_width >= chars.len() {
        chars.len().saturating_sub(label_width)
    } else if anchor > 0 {
        anchor.saturating_sub(label_width / 2)
    } else {
        0
    };
    for (index, character) in label.chars().enumerate() {
        if let Some(cell) = chars.get_mut(start + index) {
            *cell = character;
        }
    }
}

fn bucket_position_time_seconds(overview: &WaveformOverview, bucket: f32) -> f32 {
    let bucket_count = overview.buckets.len();
    if bucket_count == 0 {
        return 0.0;
    }
    overview.duration_seconds * (bucket.clamp(0.0, bucket_count as f32) / bucket_count as f32)
}

fn format_time(seconds: f32) -> String {
    if seconds < 1.0 {
        format!("{:.1}ms", seconds.max(0.0) * 1000.0)
    } else if seconds < 10.0 {
        format!("{seconds:.3}s")
    } else {
        format!("{seconds:.2}s")
    }
}

fn waveform_column_bucket(buckets: &[WaveformBucket], x: usize, width: usize) -> WaveformBucket {
    let start = x.saturating_mul(buckets.len()) / width;
    let mut end = (x + 1).saturating_mul(buckets.len()) / width;
    if end <= start {
        end = start + 1;
    }
    end = end.min(buckets.len());

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for bucket in &buckets[start..end] {
        min = min.min(bucket.min);
        max = max.max(bucket.max);
    }

    WaveformBucket { min, max }
}

fn empty_waveform_lines(width: usize, height: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(height);
    let label = fixed_width("No waveform", width);
    lines.push(Line::from(label));
    lines.extend((1..height).map(|_| Line::from(" ".repeat(width))));
    lines
}

fn waveform_row(value: f32, height: usize) -> usize {
    if height <= 1 {
        return 0;
    }
    let normalized = (sanitize_waveform_value(value) + 1.0) / 2.0;
    ((1.0 - normalized) * (height - 1) as f32).round() as usize
}

fn waveform_subrow(value: f32, height: usize) -> usize {
    waveform_row(value, height)
}

fn sanitize_waveform_value(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn fixed_width(value: &str, width: usize) -> String {
    let mut text = truncate(value, width);
    let len = text.chars().count();
    if len < width {
        text.push_str(&" ".repeat(width - len));
    }
    text
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

fn format_optional_frame(frame: Option<usize>) -> String {
    frame.map_or_else(|| "-".to_string(), |frame| frame.to_string())
}

fn format_loop_window(start: Option<usize>, end: Option<usize>) -> String {
    match (start, end) {
        (Some(start), Some(end)) => format!("{start}..{end}"),
        _ => "-".to_string(),
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
#[path = "render_tests/overlays.rs"]
mod render_overlay_tests;
#[cfg(test)]
#[path = "render_tests/pattern.rs"]
mod render_pattern_tests;
#[cfg(test)]
#[path = "render_tests/support.rs"]
mod render_test_support;
