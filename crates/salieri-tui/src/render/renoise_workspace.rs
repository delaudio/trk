use std::ops::Range;

use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    prelude::{Frame, Line, Span, Style},
    widgets::Paragraph,
};
use salieri_core::{EffectDeviceKind, NoteEvent, PatternCell, Song};
use salieri_sampler::WaveformBucket;

use super::{
    renoise_layout::PatternWorkspaceLayout,
    sampler_view::render_sampler_controls,
    theme::{self, workspace_tab, WorkspaceTabState},
    InteractionMap, SamplerViewState, TuiState,
};

const RIGHT_WIDTH: u16 = 38;
const TRACK_CELL_WIDTH: usize = 12;
const ROW_WIDTH: usize = 5;

pub(super) fn render_pattern_workspace(
    frame: &mut Frame<'_>,
    layout: PatternWorkspaceLayout,
    song: &Song,
    state: TuiState<'_>,
    interactions: &mut InteractionMap,
) {
    render_analyzer_strip(frame, layout.analyzer);
    render_util_panel(frame, layout.util, song, state);
    render_tracker_grid(frame, layout.pattern, song, state, interactions);
    render_right_sidebar(frame, layout.inspector, song, state);
    render_effects_panel(frame, layout.effects, song, state);
    render_mixer_panel(frame, layout.mixer, song, state);
    render_vu_panel(frame, layout.vu, song, state);
    render_device_chain_panel(frame, layout.device_chain, song, state);
}

pub(super) fn render_sampler_workspace(
    frame: &mut Frame<'_>,
    area: Rect,
    sampler: Option<SamplerViewState<'_>>,
    interactions: &mut InteractionMap,
) {
    let rows = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(12)])
        .split(area);
    render_sampler_tabs(frame, rows[0]);
    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([
            Constraint::Length(28),
            Constraint::Min(62),
            Constraint::Length(RIGHT_WIDTH),
        ])
        .split(rows[1]);
    render_sampler_list(frame, columns[0], sampler);
    render_sampler_waveform(frame, columns[1], sampler, interactions);
    render_sampler_properties(frame, columns[2], sampler);
}

fn render_sampler_tabs(frame: &mut Frame<'_>, area: Rect) {
    let line = Line::from(vec![
        workspace_tab("Edit", WorkspaceTabState::Disabled),
        workspace_tab("Mix", WorkspaceTabState::Disabled),
        workspace_tab("Sampler", WorkspaceTabState::Active),
        workspace_tab("Plugin", WorkspaceTabState::Disabled),
        workspace_tab("MIDI", WorkspaceTabState::Disabled),
    ]);
    frame.render_widget(Paragraph::new(line).block(theme::block("")), area);
}

fn render_sampler_list(frame: &mut Frame<'_>, area: Rect, sampler: Option<SamplerViewState<'_>>) {
    let name = sampler.map_or("No sample", |sample| sample.name);
    let preview = sampler.map_or(
        "░░░░░░░░░░░░░░░░",
        |sample| {
            if sample.overview.buckets.is_empty() {
                "░░░░░░░░░░░░░░░░"
            } else {
                "▁▃▅▇▆▄▂▃▅▇▆▄▂▁"
            }
        },
    );
    let lines = vec![
        Line::from(theme::label_span("SAMPLES")),
        Line::from(vec![theme::label_span("> "), theme::value_span(name)]),
        Line::from(""),
        Line::from(theme::label_span("SAMPLE PREVIEW")),
        Line::from(theme::value_span(preview)),
        Line::from(theme::value_span(preview)),
        Line::from(""),
        Line::from(theme::label_span("PLAYBACK")),
        kv("Mode", sampler.map_or("-", |sample| sample.playback_mode)),
        kv(
            "Gain",
            sampler.map_or("-".to_string(), |sample| format!("{:.2}", sample.gain)),
        ),
    ];
    frame.render_widget(Paragraph::new(lines).block(theme::block(" Samples ")), area);
}

fn render_sampler_waveform(
    frame: &mut Frame<'_>,
    area: Rect,
    sampler: Option<SamplerViewState<'_>>,
    interactions: &mut InteractionMap,
) {
    let sections = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(4)])
        .split(area);
    let Some(sample) = sampler else {
        frame.render_widget(
            Paragraph::new("No sample loaded").block(theme::block(" Waveform ")),
            sections[0],
        );
        render_sampler_controls(frame, sections[1], None, interactions);
        return;
    };
    let width = sections[0].width.saturating_sub(4) as usize;
    let visible = sample
        .waveform_end_bucket
        .min(sample.overview.buckets.len())
        .saturating_sub(sample.waveform_start_bucket);
    let buckets = sample
        .overview
        .buckets
        .iter()
        .skip(sample.waveform_start_bucket)
        .take(visible)
        .copied()
        .collect::<Vec<_>>();
    let view = format!(
        "View {}..{}  Zoom {}x",
        sample.waveform_start_bucket, sample.waveform_end_bucket, sample.waveform_zoom
    );
    let lines = vec![
        Line::from(vec![
            theme::disabled_span("Record×  "),
            theme::value_span(format!(
                "{} [{}Hz {}ch {:.2}s]",
                sample.name,
                sample.overview.sample_rate,
                sample.overview.channels,
                sample.overview.duration_seconds
            )),
        ]),
        Line::from(theme::muted_span(view)),
        Line::from(theme::muted_span(
            "00     10     20     30     40     50     60     70     80",
        )),
        Line::from(vec![
            theme::label_span("L "),
            Span::styled(
                waveform_bar(&buckets, width),
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(vec![
            theme::label_span("  "),
            Span::styled(
                waveform_bar(&buckets, width),
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(theme::muted_span(
            "────────────────────────────────────────────────────────",
        )),
        Line::from(vec![
            theme::label_span("R "),
            Span::styled(
                waveform_bar(&buckets, width),
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(vec![
            theme::label_span("  "),
            Span::styled(
                waveform_bar(&buckets, width),
                Style::default().fg(theme::TEXT),
            ),
        ]),
        Line::from(theme::disabled_span(
            "Undo× | Normalize× | Slice× | FFT× | Loop× | Preview×",
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(theme::block(format!(" Samples  ■  {} ", sample.name))),
        sections[0],
    );
    render_sampler_controls(frame, sections[1], Some(sample), interactions);
}

fn render_sampler_properties(
    frame: &mut Frame<'_>,
    area: Rect,
    sampler: Option<SamplerViewState<'_>>,
) {
    let mut lines = vec![
        Line::from(vec![
            workspace_tab("Songs", WorkspaceTabState::Disabled),
            workspace_tab("Instr.", WorkspaceTabState::Disabled),
            workspace_tab("Samples", WorkspaceTabState::Active),
            workspace_tab("Other", WorkspaceTabState::Disabled),
        ]),
        Line::from(""),
        kv("Name", sampler.map_or("-", |sample| sample.name)),
        kv("Path", sampler.map_or("-", |sample| sample.source_path)),
        kv("Format", "Renoise Song"),
        kv("Time Base", "Beats"),
        kv("Speed", "6x"),
        Line::from(""),
        Line::from(theme::label_span("SAMPLE PROPERTIES")),
        kv(
            "Volume",
            sampler.map_or("-".to_string(), |sample| format!("{:.2} dB", sample.gain)),
        ),
        kv("Panning", "Center"),
        kv("Transpose", "0 st"),
        kv("BeatSync", "16"),
    ];
    if let Some(sample) = sampler {
        lines.extend([
            kv("Instrument", sample.instrument.unwrap_or("-")),
            kv("Track", sample.assigned_track.unwrap_or("-")),
            kv(
                "Loop",
                format_window(sample.loop_start_frame, sample.loop_end_frame),
            ),
            kv(
                "Window",
                format_window(sample.start_frame, sample.end_frame),
            ),
            Line::from(""),
            Line::from(theme::muted_span("BROWSER (read-only)")),
            Line::from(theme::muted_span("~/Music/DemoSong/")),
            Line::from(vec![
                theme::label_span("> "),
                theme::value_span(format!("{}.flac", sample.name)),
            ]),
        ]);
    }
    frame.render_widget(
        Paragraph::new(lines).block(theme::block(" Instrument Properties ")),
        area,
    );
}

fn render_analyzer_strip(frame: &mut Frame<'_>, area: Rect) {
    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([
            Constraint::Percentage(66),
            Constraint::Percentage(17),
            Constraint::Percentage(17),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![theme::muted_span(
                "-18dB       50       100       200       500       1K       2K       5K       10K",
            )]),
            Line::from(theme::muted_span(
                "-36dB   · · · · · · · · · · · · · · · · · · · · · · · · · · · · ·",
            )),
            Line::from(theme::muted_span(
                "-72dB   · · · · · · · · · · · · · · · · · · · · · · · · · · · · ·",
            )),
        ])
        .block(theme::block("")),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(theme::muted_span("+L        ╲      +R")),
            Line::from(theme::muted_span("     ╲    ╱")),
            Line::from(theme::muted_span("+R     ╲╱      -L")),
        ])
        .block(theme::block("")),
        columns[1],
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(theme::disabled_span("MIDI MAP×")),
            Line::from(theme::disabled_span(" 1 2 3 4 5 6 7 8")),
        ])
        .block(theme::block("")),
        columns[2],
    );
}

fn render_util_panel(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let lines = vec![
        Line::from(theme::label_span("UTIL")),
        Line::from(""),
        kv("BPM", song.transport.bpm.to_string()),
        kv("LPB", song.transport.lines_per_beat.to_string()),
        kv("OCT", state.octave.to_string()),
        Line::from(theme::muted_span("--   --")),
        Line::from(""),
        Line::from(theme::value_span("Follow")),
        Line::from(theme::value_span("  Pattern")),
        Line::from(theme::value_span("Scroll")),
        Line::from(theme::value_span("  Page")),
        Line::from(""),
        kv("Edit Step", state.edit_step.to_string()),
    ];
    frame.render_widget(Paragraph::new(lines).block(theme::block(" UTIL ")), area);
}

fn render_tracker_grid(
    frame: &mut Frame<'_>,
    area: Rect,
    song: &Song,
    state: TuiState<'_>,
    interactions: &mut InteractionMap,
) {
    let Some(pattern) = song.pattern(state.pattern_index) else {
        frame.render_widget(
            Paragraph::new("No pattern").block(theme::block(" Pattern ")),
            area,
        );
        return;
    };
    let inner_width = area.width.saturating_sub(2) as usize;
    let row_count = pattern.row_count();
    let track_count = song.tracks.len();
    let track_capacity = inner_width
        .saturating_sub(ROW_WIDTH * 2)
        .checked_div(TRACK_CELL_WIDTH)
        .unwrap_or(1)
        .max(1);
    let row_capacity = area.height.saturating_sub(4) as usize;
    let visible_tracks = visible_range(
        track_count,
        track_capacity,
        state.track_offset,
        state.cursor.track,
    );
    let visible_rows = visible_range(row_count, row_capacity, state.row_offset, state.cursor.row);
    interactions.register_pattern_cells(
        bordered_content_area(area),
        2,
        ROW_WIDTH as u16,
        TRACK_CELL_WIDTH as u16,
        visible_rows.clone(),
        visible_tracks.clone(),
    );

    let mut lines = Vec::with_capacity(row_capacity + 2);
    lines.push(track_header(
        song,
        state.cursor.track,
        visible_tracks.clone(),
        "Track",
    ));
    lines.push(track_header(
        song,
        state.cursor.track,
        visible_tracks.clone(),
        "Note Ins FX",
    ));
    for row in visible_rows {
        let mut spans = vec![row_span(row, state.playhead_row == Some(row), state)];
        for track in visible_tracks.clone() {
            let cell = pattern
                .rows
                .get(row)
                .and_then(|pattern_row| pattern_row.cells.get(track))
                .cloned()
                .unwrap_or_default();
            spans.push(cell_span(&cell, row, track, state));
        }
        spans.push(row_span(row, state.playhead_row == Some(row), state));
        lines.push(Line::from(spans));
    }

    let title = format!(
        " Pattern Editor: {} | rows={} | tracks={} ",
        pattern.name, row_count, track_count
    );
    frame.render_widget(Paragraph::new(lines).block(theme::block(title)), area);
}

fn bordered_content_area(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn render_right_sidebar(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let instrument = song.instrument_for_track(
        song.tracks
            .get(state.cursor.track)
            .map_or(salieri_core::TrackId(0), |track| track.id),
    );
    let sample = song.sample_for_track(
        song.tracks
            .get(state.cursor.track)
            .map_or(salieri_core::TrackId(0), |track| track.id),
    );
    let selected_song = if song.metadata.title.is_empty() {
        "DemoSong"
    } else {
        song.metadata.title.as_str()
    };
    let mut lines = vec![
        Line::from(vec![
            workspace_tab("Songs", WorkspaceTabState::Active),
            workspace_tab("Instr.", WorkspaceTabState::Enabled),
            workspace_tab("Samples", WorkspaceTabState::Enabled),
            workspace_tab("Other", WorkspaceTabState::Disabled),
        ]),
        Line::from(""),
        kv("Name", selected_song),
        kv("Path", "~/Music/DemoSong/"),
        kv("Format", "Renoise Song"),
        kv("Time Base", "Beats"),
        kv("Speed", "6x"),
        kv("Edit Mode", "Pattern"),
    ];
    if let Some(cell) = active_cell(song, state).filter(|cell| cell_has_data(cell)) {
        lines.push(kv("Cell", full_cell_text(cell)));
    }
    lines.extend([
        Line::from(""),
        Line::from(theme::label_span("BROWSER")),
        Line::from(theme::muted_span("~/Music/DemoSong/")),
        Line::from(theme::value_span("▾ Samples")),
    ]);
    for sample in song.samples.iter().take(8) {
        lines.push(Line::from(vec![
            theme::muted_span("  ♫ "),
            theme::value_span(clip(&sample.name, 28)),
        ]));
    }
    lines.push(Line::from(theme::value_span("▾ Songs")));
    for (index, pattern) in song.patterns.iter().take(8).enumerate() {
        let style = if index == state.pattern_index {
            theme::active()
        } else {
            theme::base()
        };
        lines.push(Line::from(Span::styled(
            format!("  ▹ {}", clip(&pattern.name, 29)),
            style,
        )));
    }
    if let Some(instrument) = instrument {
        lines.push(kv("Instrument", clip(&instrument.name, 20)));
    }
    if let Some(sample) = sample {
        lines.push(kv("Sample", clip(&sample.name, 24)));
    }
    frame.render_widget(
        Paragraph::new(lines).block(theme::block(" Instrument Properties ")),
        area,
    );
}

fn render_effects_panel(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let effects = song
        .tracks
        .get(state.cursor.track)
        .map(|track| song.track_mixer_for_track(track.id).effects)
        .unwrap_or_default();
    let mut lines = vec![Line::from(theme::muted_span("#  Effect"))];
    for (index, effect) in effects.iter().take(6).enumerate() {
        lines.push(Line::from(format!(
            "{:02} {}",
            index + 1,
            effect_name(&effect.kind)
        )));
    }
    for index in effects.len().min(6)..6 {
        lines.push(Line::from(format!("{:02} --", index + 1)));
    }
    frame.render_widget(
        Paragraph::new(lines).block(theme::block(" Effects (Track) ")),
        area,
    );
}

fn render_mixer_panel(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let track = song.tracks.get(state.cursor.track);
    let mixer = track.map(|track| song.track_mixer_for_track(track.id));
    let pan = mixer.as_ref().map_or(0.0, |mixer| mixer.pan);
    let gain = mixer.as_ref().map_or(1.0, |mixer| mixer.gain);
    let name = track.map_or("Track", |track| track.name.as_str());
    let lines = vec![
        kv("Name", clip(name, 24)),
        kv("Routing", "Master"),
        slider("Panning", pan, -1.0, 1.0),
        slider("Volume", gain, 0.0, 2.0),
        slider("Width", 1.0, 0.0, 2.0),
        kv("Delay", "0.000 ms"),
        Line::from(theme::muted_span("M  S  [ ]  P  G  >>")),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(theme::block(" Track Mixer ")),
        area,
    );
}

fn render_vu_panel(frame: &mut Frame<'_>, area: Rect, _song: &Song, state: TuiState<'_>) {
    let meter = if state.is_playing {
        "████████░░"
    } else {
        "██░░░░░░░░"
    };
    let lines = vec![
        Line::from(theme::muted_span("        L        R      Peak")),
        Line::from(vec![
            theme::label_span("  0 "),
            theme::value_span("│        │     -1.2 dB"),
        ]),
        Line::from(vec![
            theme::label_span(" -6 "),
            Span::styled(meter, Style::default().fg(theme::METER)),
        ]),
        Line::from(vec![
            theme::label_span("-12 "),
            Span::styled(meter, Style::default().fg(theme::METER)),
        ]),
        Line::from(theme::muted_span("-48      -3.4 dB  -3.1 dB RMS")),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(theme::block(" VU / Levels ")),
        area,
    );
}

fn render_device_chain_panel(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let track_effects = song
        .tracks
        .get(state.cursor.track)
        .map(|track| song.track_mixer_for_track(track.id).effects)
        .unwrap_or_default();
    let mut lines = Vec::new();
    for (index, effect) in song.mixer.master_effects.iter().take(4).enumerate() {
        lines.push(Line::from(format!(
            "{:02} Master: {}",
            index + 1,
            effect_name(&effect.kind)
        )));
    }
    for (index, effect) in track_effects.iter().take(4).enumerate() {
        lines.push(Line::from(format!(
            "{:02} Track: {}",
            lines.len() + index + 1,
            effect_name(&effect.kind)
        )));
    }
    while lines.len() < 6 {
        lines.push(Line::from(format!("{:02} --", lines.len() + 1)));
    }
    frame.render_widget(
        Paragraph::new(lines).block(theme::block(" Device Chain ")),
        area,
    );
}

fn track_header(
    song: &Song,
    active_track: usize,
    tracks: Range<usize>,
    label: &str,
) -> Line<'static> {
    let mut spans = vec![theme::muted_span(format!("{:<ROW_WIDTH$}", ""))];
    for track_index in tracks {
        let track_name = if label == "Track" {
            song.tracks.get(track_index).map_or_else(
                || format!("{:02}", track_index + 1),
                |track| clip(&track.name, 8),
            )
        } else {
            label.to_string()
        };
        let style = if track_index == active_track {
            theme::active()
        } else {
            theme::label()
        };
        spans.push(Span::styled(
            format!("{:^TRACK_CELL_WIDTH$}", track_name),
            style,
        ));
    }
    spans.push(theme::muted_span(format!("{:<ROW_WIDTH$}", "")));
    Line::from(spans)
}

fn cell_span(cell: &PatternCell, row: usize, track: usize, state: TuiState<'_>) -> Span<'static> {
    let text = format!(
        "{:<3} {:02X} {:<3}",
        note_text(cell),
        cell.instrument.map_or(0, |id| id.0.min(0xFF) as u8),
        cell.command.map_or("--".to_string(), |cmd| format!(
            "{}{:02X}",
            cmd.code as char, cmd.value
        ))
    );
    let is_cursor = state.cursor.row == row && state.cursor.track == track;
    let is_selected = state
        .selection
        .is_some_and(|selection| selection.contains(row, track));
    let style = if is_cursor {
        theme::active()
    } else if is_selected {
        theme::selected()
    } else if state.playhead_row == Some(row) {
        theme::playing()
    } else if state.cursor.track == track {
        theme::base().fg(theme::TEXT).bg(theme::BORDER_DIM)
    } else {
        theme::base()
    };
    Span::styled(
        format!(" {:<width$}", text, width = TRACK_CELL_WIDTH - 1),
        style,
    )
}
fn full_cell_text(cell: &PatternCell) -> String {
    format!(
        "{:<3} {:02X} {:02X} {:02X} {:02X} {:02X} {}",
        note_text(cell),
        cell.velocity.unwrap_or(0),
        cell.instrument.map_or(0, |id| id.0.min(0xFF) as u8),
        cell.volume.unwrap_or(0),
        cell.pan.unwrap_or(0),
        cell.delay.unwrap_or(0),
        cell.command.map_or("--".to_string(), |cmd| format!(
            "{}{:02X}",
            cmd.code as char, cmd.value
        ))
    )
}
fn active_cell<'a>(song: &'a Song, state: TuiState<'_>) -> Option<&'a PatternCell> {
    song.pattern(state.pattern_index)?
        .rows
        .get(state.cursor.row)?
        .cells
        .get(state.cursor.track)
}
fn cell_has_data(cell: &PatternCell) -> bool {
    cell.note.is_some()
        || cell.velocity.is_some()
        || cell.instrument.is_some()
        || cell.volume.is_some()
        || cell.pan.is_some()
        || cell.delay.is_some()
        || cell.command.is_some()
        || cell.command2.is_some()
        || !cell.parameter_locks.is_empty()
}
fn row_span(row: usize, is_playhead: bool, state: TuiState<'_>) -> Span<'static> {
    let row_number = if state.show_line_numbers_hex {
        format!("{:02X}", row + state.row_number_offset)
    } else {
        format!("{row:04}")
    };
    let style = if is_playhead {
        theme::playing()
    } else {
        theme::muted()
    };
    let marker = if is_playhead { ">" } else { " " };
    Span::styled(
        format!("{marker}{row_number:>width$}", width = ROW_WIDTH - 1),
        style,
    )
}
fn visible_range(total: usize, capacity: usize, offset: usize, cursor: usize) -> Range<usize> {
    let capacity = capacity.max(1);
    let mut start = offset.min(total.saturating_sub(capacity));
    if cursor < start {
        start = cursor;
    }
    if cursor >= start.saturating_add(capacity) {
        start = cursor.saturating_add(1).saturating_sub(capacity);
    }
    start..start.saturating_add(capacity).min(total)
}
fn note_text(cell: &PatternCell) -> String {
    match cell.note {
        Some(NoteEvent::Note { pitch }) => format_note(pitch),
        Some(NoteEvent::NoteOff) => "OFF".to_string(),
        Some(NoteEvent::NoteCut) => "CUT".to_string(),
        None => "---".to_string(),
    }
}
fn format_note(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
    ];
    format!(
        "{}{}",
        NAMES[(pitch % 12) as usize],
        i16::from(pitch / 12) - 1
    )
}
fn kv(label: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        theme::label_span(format!("{label:<10}")),
        theme::value_span(value.into()),
    ])
}
fn slider(label: &str, value: f32, min: f32, max: f32) -> Line<'static> {
    let normalized = ((value - min) / (max - min)).clamp(0.0, 1.0);
    let filled = (normalized * 12.0).round() as usize;
    let bar = format!("{}{}", "█".repeat(filled), "─".repeat(12 - filled));
    Line::from(vec![
        theme::label_span(format!("{label:<9}")),
        Span::styled(bar, Style::default().fg(theme::METER)),
        theme::value_span(format!(" {value:+.2}")),
    ])
}
fn waveform_bar(buckets: &[WaveformBucket], width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if buckets.is_empty() {
        return "─".repeat(width);
    }
    (0..width)
        .map(|column| {
            let index = column.saturating_mul(buckets.len()) / width;
            let bucket = buckets[index.min(buckets.len() - 1)];
            let amplitude = bucket.max.abs().max(bucket.min.abs());
            match (amplitude * 8.0).round() as u8 {
                0 => '·',
                1 => '▁',
                2 => '▂',
                3 => '▃',
                4 => '▄',
                5 => '▅',
                6 => '▆',
                7 => '▇',
                _ => '█',
            }
        })
        .collect()
}

fn format_window(start: Option<usize>, end: Option<usize>) -> String {
    match (start, end) {
        (Some(start), Some(end)) => format!("{start}..{end}"),
        (Some(start), None) => format!("{start}..-"),
        (None, Some(end)) => format!("-..{end}"),
        (None, None) => "-".to_string(),
    }
}

fn effect_name(kind: &EffectDeviceKind) -> &'static str {
    match kind {
        EffectDeviceKind::Gain { .. } => "Native: Gainer",
        EffectDeviceKind::Pan { .. } => "Native: Pan",
        EffectDeviceKind::Balance { .. } => "Native: Balance",
        EffectDeviceKind::PhaseInvert { .. } => "Native: Phase Invert",
        EffectDeviceKind::StereoWidth { .. } => "Native: Stereo Expander",
        EffectDeviceKind::Filter { .. } => "Native: Filter",
        EffectDeviceKind::Delay { .. } => "Native: Delay",
        EffectDeviceKind::Reverb { .. } => "Native: Reverb",
        EffectDeviceKind::Compressor { .. } => "Native: Compressor",
        EffectDeviceKind::Limiter { .. } => "Native: Limiter",
        EffectDeviceKind::Gate { .. } => "Native: Gate",
        EffectDeviceKind::Bitcrusher { .. } => "Native: Bitcrusher",
        EffectDeviceKind::Chorus { .. } => "Native: Chorus",
        EffectDeviceKind::Flanger { .. } => "Native: Flanger",
        EffectDeviceKind::Phaser { .. } => "Native: Phaser",
        EffectDeviceKind::Drive { .. } => "Native: Drive",
    }
}

fn clip(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_string()
    } else {
        value
            .chars()
            .take(width.saturating_sub(1))
            .collect::<String>()
            + "…"
    }
}
