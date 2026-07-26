use std::ops::Range;

mod browser_views;
mod dsp_parameters;
mod dsp_rack;
mod help_overlay;
mod modal_overlays;
mod renoise_workspace;
mod theme;

use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use salieri_core::{
    mixer_master_gain_descriptor, mixer_track_gain_descriptor, mixer_track_pan_descriptor,
    native_balance_descriptor, native_bitcrusher_bit_depth_descriptor,
    native_bitcrusher_mix_descriptor, native_bitcrusher_output_descriptor,
    native_bitcrusher_reduction_descriptor, native_chorus_depth_descriptor,
    native_chorus_mix_descriptor, native_chorus_rate_descriptor, native_chorus_spread_descriptor,
    native_chorus_sync_descriptor, native_compressor_attack_descriptor,
    native_compressor_makeup_descriptor, native_compressor_mix_descriptor,
    native_compressor_ratio_descriptor, native_compressor_release_descriptor,
    native_compressor_threshold_descriptor, native_delay_feedback_descriptor,
    native_delay_mix_descriptor, native_delay_output_descriptor, native_delay_ping_pong_descriptor,
    native_delay_sync_descriptor, native_delay_time_left_descriptor,
    native_delay_time_right_descriptor, native_drive_drive_descriptor, native_drive_mix_descriptor,
    native_drive_mode_descriptor, native_drive_output_descriptor, native_drive_tone_descriptor,
    native_filter_cutoff_descriptor, native_filter_drive_descriptor, native_filter_mix_descriptor,
    native_filter_mode_descriptor, native_filter_resonance_descriptor,
    native_flanger_depth_descriptor, native_flanger_feedback_descriptor,
    native_flanger_manual_descriptor, native_flanger_mix_descriptor,
    native_flanger_rate_descriptor, native_flanger_stereo_phase_descriptor,
    native_flanger_sync_descriptor, native_gain_descriptor, native_gate_attack_descriptor,
    native_gate_hysteresis_descriptor, native_gate_range_descriptor,
    native_gate_release_descriptor, native_gate_threshold_descriptor,
    native_limiter_ceiling_descriptor, native_limiter_input_gain_descriptor,
    native_limiter_lookahead_descriptor, native_limiter_release_descriptor, native_pan_descriptor,
    native_phase_invert_left_descriptor, native_phase_invert_right_descriptor,
    native_phaser_center_descriptor, native_phaser_depth_descriptor,
    native_phaser_feedback_descriptor, native_phaser_mix_descriptor, native_phaser_rate_descriptor,
    native_phaser_stages_descriptor, native_phaser_stereo_phase_descriptor,
    native_phaser_sync_descriptor, native_reverb_damping_descriptor,
    native_reverb_decay_descriptor, native_reverb_mix_descriptor, native_reverb_output_descriptor,
    native_reverb_predelay_descriptor, native_reverb_size_descriptor, native_width_descriptor,
    sample_envelope_attack_descriptor, sample_envelope_decay_descriptor,
    sample_envelope_release_descriptor, sample_envelope_sustain_descriptor, sample_gain_descriptor,
    CellField, Cursor, EffectDevice, EffectDeviceKind, MidiRoutingSettings, NoteEvent,
    ParameterDescriptor, ParameterValue, Pattern, PatternCell, SamplePlaybackMode, Song,
};
use salieri_sampler::{WaveformBucket, WaveformOverview};

use crate::{
    interaction_region, resolve_tracker_layout, InteractionMap, PatternFieldLayout,
    TrackerLayoutPreset, TrackerLayoutState, ViewportAxis,
};
use browser_views::{render_project_browser, render_sample_browser};
use dsp_rack::render_dsp_rack_view;
use help_overlay::render_help_overlay;
use modal_overlays::{
    render_command_palette_overlay, render_delete_confirmation, render_midi_settings_overlay,
    render_quit_confirmation,
};

const ROW_GUTTER_WIDTH: usize = 5;
const PATTERN_CELL_WIDTH: usize = 28;
const TRACK_LIST_NAME_WIDTH: usize = 11;
const SEQUENCE_SLOT_PATTERN_WIDTH: usize = 18;
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
    pub edit_step: usize,
    pub dirty: bool,
    pub show_line_numbers_hex: bool,
    pub row_number_offset: usize,
    pub pattern_divider_interval: usize,
    pub pattern_highlight_interval: usize,
    pub show_pattern_top_info: bool,
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
    pub command_palette: Option<CommandPaletteViewState<'a>>,
    pub sampler_view: Option<SamplerViewState<'a>>,
    pub dsp_rack: Option<DspRackViewState<'a>>,
    pub sample_browser: Option<SampleBrowserViewState<'a>>,
    pub project_browser: Option<ProjectBrowserViewState<'a>>,
    pub ai_chat: Option<AiChatViewState<'a>>,
    pub tracker_layout: TrackerLayoutState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiView {
    Pattern,
    Sequence,
    Clips,
    Tracks,
    Patterns,
    Sampler,
    DspRack,
    SampleBrowser,
    ProjectBrowser,
    AiChat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiChatViewState<'a> {
    pub provider: &'a str,
    pub status: &'a str,
    pub composer: &'a str,
    pub messages: &'a [AiChatMessageView<'a>],
    pub selected_context: &'a str,
    pub proposal_preview: Option<AiChatProposalPreviewView<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DspRackViewState<'a> {
    pub track_name: &'a str,
    pub track_number: usize,
    pub track_effects: &'a [EffectDevice],
    pub master_effects: &'a [EffectDevice],
    pub selected_target: DspRackTargetView,
    pub selected_index: usize,
    pub selected_parameter_index: usize,
    pub selected_lock_status: DspParameterLockStatusView,
    pub device_palette: Option<DspDevicePaletteViewState<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DspRackTargetView {
    Track,
    Master,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DspParameterLockStatusView {
    Unlocked,
    Set,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DspDevicePaletteViewState<'a> {
    pub entries: &'a [DspDevicePaletteEntryView<'a>],
    pub selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DspDevicePaletteEntryView<'a> {
    pub label: &'a str,
    pub summary: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiChatProposalPreviewView<'a> {
    pub lines: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiChatMessageView<'a> {
    pub role: AiChatMessageRole,
    pub text: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiChatMessageRole {
    System,
    User,
    Assistant,
    Error,
    Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPaletteViewState<'a> {
    pub query: &'a str,
    pub entries: &'a [CommandPaletteEntryView<'a>],
    pub selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPaletteEntryView<'a> {
    pub title: &'a str,
    pub category: &'a str,
    pub command: &'a str,
    pub shortcut: Option<&'a str>,
    pub disabled_reason: Option<&'a str>,
    pub recent: bool,
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
    pub input_status: &'a str,
    pub routing: &'a MidiRoutingSettings,
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
    let _ = render_with_interactions(frame, song, state);
}

#[must_use]
pub fn render_with_interactions(
    frame: &mut Frame<'_>,
    song: &Song,
    state: TuiState<'_>,
) -> InteractionMap {
    let area = frame.area();
    let vertical = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);
    let mut interactions = InteractionMap::new();
    interactions.register(interaction_region::APP_HEADER, vertical[0]);
    interactions.register(interaction_region::APP_BODY, vertical[1]);
    interactions.register(interaction_region::APP_STATUS, vertical[2]);

    render_header(frame, vertical[0], song, state);
    render_body(frame, vertical[1], song, state, &mut interactions);
    render_status(frame, vertical[2], state);

    if state.show_help {
        render_help_overlay(
            frame,
            area,
            state.mode_label,
            state.edit_step,
            state.help_scroll,
            state.help_tab,
        );
    }
    if let Some(midi_settings) = state.midi_settings {
        render_midi_settings_overlay(frame, area, midi_settings);
    }
    if let Some(command_palette) = state.command_palette {
        render_command_palette_overlay(frame, area, command_palette);
    }
    if state.quit_confirmation {
        render_quit_confirmation(frame, area);
    }
    if let Some(message) = state.delete_confirmation {
        render_delete_confirmation(frame, area, message);
    }
    interactions
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
    let block = theme::block(" Waveform ");
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
    let dirty = if state.dirty { "*" } else { "" };
    let playback = if state.is_playing {
        "PLAYING"
    } else {
        "STOPPED"
    };
    let loop_state = if state.loop_pattern { "ON" } else { "OFF" };
    let playhead = state
        .playhead_row
        .map_or_else(|| "0000".to_string(), |row| format!("{row:04}"));
    let sequence_position = state.sequence_position.unwrap_or(0);
    let line = Line::from(vec![
        theme::label_span(" ["),
        theme::value_span(if state.is_playing { "▶" } else { "▷" }),
        theme::label_span("] ["),
        theme::value_span("■"),
        theme::label_span("] ["),
        theme::value_span("●"),
        theme::label_span("]  BPM: "),
        theme::value_span(song.transport.bpm.to_string()),
        theme::label_span("  LPB: "),
        theme::value_span(song.transport.lines_per_beat.to_string()),
        theme::label_span("  Oct: "),
        theme::value_span(state.octave.to_string()),
        theme::label_span("  Vel: "),
        theme::value_span("100"),
        theme::label_span("  Swing: "),
        theme::value_span("0%"),
        theme::label_span("  Sync: "),
        theme::value_span("Internal"),
        theme::muted_span("  |  "),
        theme::label_span("CPU: "),
        theme::value_span("00.0%"),
        theme::muted_span("  |  "),
        Span::styled(
            playback,
            if state.is_playing {
                theme::playing()
            } else {
                theme::muted()
            },
        ),
        theme::muted_span("  |  "),
        theme::label_span("PAT: "),
        theme::value_span(format!("{:02}", state.pattern_index + 1)),
        theme::label_span("  LINE: "),
        theme::value_span(format!("{playhead}/{:04}", state.cursor.row)),
        theme::muted_span("  |  "),
        theme::value_span(state.midi_status.to_string()),
        theme::label_span("  ORDER: "),
        theme::value_span(format!("{sequence_position:02}")),
        theme::label_span("  LOOP: "),
        theme::value_span(loop_state),
        theme::label_span("  TRK: "),
        theme::value_span(format!("{:02}", state.cursor.track + 1)),
        theme::label_span("  FIELD: "),
        theme::value_span(state.cursor.field.to_string()),
        theme::muted_span(if state.selection.is_some() {
            "  SEL"
        } else {
            ""
        }),
        theme::muted_span("  |  MIDI MAP  1 2 3 4 5 6 7 8"),
        theme::muted_span(dirty),
        theme::muted_span(format!("  {}", truncate(pattern_name, 18))),
    ]);
    let header = Paragraph::new(line)
        .block(theme::block(" Salieri Tracker "))
        .style(theme::base());
    frame.render_widget(header, area);
}

fn render_body(
    frame: &mut Frame<'_>,
    area: Rect,
    song: &Song,
    state: TuiState<'_>,
    interactions: &mut InteractionMap,
) {
    interactions.register(active_view_region(state.active_view), area);
    if state.active_view == TuiView::Sequence {
        render_sequence_editor(frame, area, song, state.sequence_position);
        return;
    }
    if state.active_view == TuiView::Clips {
        render_clip_launcher(frame, area, song, state);
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
        if layout_kind(area.width) == LayoutKind::Large {
            renoise_workspace::render_sampler_workspace(frame, area, state.sampler_view);
            return;
        }
        render_sampler_view(frame, area, state.sampler_view);
        return;
    }
    if state.active_view == TuiView::DspRack {
        render_dsp_rack_view(frame, area, state.dsp_rack);
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
    if state.active_view == TuiView::AiChat {
        render_ai_chat_view(frame, area, state.ai_chat);
        return;
    }
    if layout_kind(area.width) == LayoutKind::Large {
        renoise_workspace::render_pattern_workspace(frame, area, song, state);
        return;
    }

    let mut tracker_layout = state.tracker_layout;
    match layout_kind(area.width) {
        LayoutKind::Large => {}
        LayoutKind::Medium => {
            tracker_layout.inspector_visible = false;
        }
        LayoutKind::Small => {
            tracker_layout = TrackerLayoutState::from_preset(TrackerLayoutPreset::Compact);
        }
    }
    let resolved = resolve_tracker_layout(area, tracker_layout);
    if let Some(area) = resolved.tracks {
        interactions.register(interaction_region::PANEL_TRACKS, area);
        render_tracks(frame, area, song, state.cursor.track);
    }
    if let Some(area) = resolved.sequence {
        interactions.register(interaction_region::PANEL_SEQUENCE, area);
        render_sequence(frame, area, song, state.sequence_position);
    }
    interactions.register(interaction_region::PANEL_PATTERN, resolved.pattern);
    render_pattern(frame, resolved.pattern, song, state);
    if let Some(area) = resolved.track_desk {
        interactions.register(interaction_region::PANEL_TRACK_DESK, area);
        render_track_properties(frame, area, song, state);
    }
    if let Some(area) = resolved.inspector {
        interactions.register(interaction_region::PANEL_INSPECTOR, area);
        render_instrument_sidebar(frame, area, song, state);
    }
}

fn active_view_region(view: TuiView) -> crate::InteractionRegionId {
    match view {
        TuiView::Pattern => interaction_region::VIEW_PATTERN,
        TuiView::Sequence => interaction_region::VIEW_SEQUENCE,
        TuiView::Clips => interaction_region::VIEW_CLIPS,
        TuiView::Tracks => interaction_region::VIEW_TRACKS,
        TuiView::Patterns => interaction_region::VIEW_PATTERNS,
        TuiView::Sampler => interaction_region::VIEW_SAMPLER,
        TuiView::DspRack => interaction_region::VIEW_DSP_RACK,
        TuiView::SampleBrowser => interaction_region::VIEW_SAMPLE_BROWSER,
        TuiView::ProjectBrowser => interaction_region::VIEW_PROJECT_BROWSER,
        TuiView::AiChat => interaction_region::VIEW_AI_CHAT,
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
            let pattern = song
                .patterns
                .iter()
                .enumerate()
                .find(|(_, pattern)| pattern.id == *pattern_id);
            let name = pattern.map_or("Missing Pattern", |(_, pattern)| pattern.name.as_str());
            let pattern_label = pattern.map_or_else(
                || "P??".to_string(),
                |(pattern_index, _)| format!("P{:02}", pattern_index + 1),
            );
            let clips = sequence_slot_clips(song, pattern.map(|(_, pattern)| pattern), 8);
            let marker = if active_sequence_position == Some(index) {
                ">"
            } else {
                " "
            };
            Line::from(format!(
                "{marker} {index:02} {pattern_label} {:<width$} {clips}",
                truncate(name, SEQUENCE_SLOT_PATTERN_WIDTH),
                width = SEQUENCE_SLOT_PATTERN_WIDTH,
            ))
        })
        .collect::<Vec<_>>();

    let sequence = Paragraph::new(lines).block(
        Block::default()
            .title(ranged_title("Song Slots", start, end, song.sequence.len()))
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
            "PATTERN             CLIPS",
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
                .enumerate()
                .find(|(_, pattern)| pattern.id == *pattern_id);
            let name = pattern.map_or("Missing Pattern", |(_, pattern)| pattern.name.as_str());
            let pattern_label = pattern.map_or_else(
                || "P??".to_string(),
                |(pattern_index, _)| format!("P{:02}", pattern_index + 1),
            );
            let clip_capacity = area
                .width
                .saturating_sub(34)
                .saturating_div(2)
                .max(1)
                .into();
            let clips =
                sequence_slot_clips(song, pattern.map(|(_, pattern)| pattern), clip_capacity);
            let marker = if active_sequence_position == Some(index) {
                ">"
            } else {
                " "
            };
            let line = format!(
                "{marker}{index:02}  {pattern_label} {:<width$} {clips}",
                truncate(name, SEQUENCE_SLOT_PATTERN_WIDTH),
                width = SEQUENCE_SLOT_PATTERN_WIDTH,
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
        Line::from("Clips: ■ active  · empty  M muted"),
    ]);

    let active_index = active_sequence_position.unwrap_or(0);
    let visible_items = list_inner_height(area).saturating_sub(5);
    let start = centered_scroll_offset(song.sequence.len(), active_index, visible_items);
    let end = start.saturating_add(visible_items).min(song.sequence.len());
    let sequence = Paragraph::new(lines)
        .block(
            Block::default()
                .title(ranged_title(
                    "Song Slot View",
                    start,
                    end,
                    song.sequence.len(),
                ))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(sequence, area);
}

fn sequence_slot_clips(song: &Song, pattern: Option<&Pattern>, max_tracks: usize) -> String {
    let visible_tracks = song.tracks.len().min(max_tracks.max(1));
    let mut clips = String::from("[");
    for track_index in 0..visible_tracks {
        if track_index > 0 {
            clips.push(' ');
        }
        let symbol = if song.tracks[track_index].muted {
            'M'
        } else if pattern.is_some_and(|pattern| pattern_track_has_activity(pattern, track_index)) {
            '■'
        } else {
            '·'
        };
        clips.push(symbol);
    }
    if song.tracks.len() > visible_tracks {
        clips.push_str(" …");
    }
    clips.push(']');
    clips
}

fn pattern_track_has_activity(pattern: &Pattern, track_index: usize) -> bool {
    pattern
        .rows
        .iter()
        .filter_map(|row| row.cells.get(track_index))
        .any(|cell| {
            cell.note.is_some()
                || cell.instrument.is_some()
                || cell.volume.is_some()
                || cell.pan.is_some()
                || cell.command.is_some()
                || cell.command2.is_some()
        })
}

fn render_track_editor(frame: &mut Frame<'_>, area: Rect, song: &Song, active_track: usize) {
    let sections = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);

    render_track_mixer(frame, sections[0], song, active_track);
    render_instrument_matrix(frame, sections[1], song, active_track);
}

fn render_clip_launcher(frame: &mut Frame<'_>, area: Rect, song: &Song, state: TuiState<'_>) {
    let visible_tracks = song
        .tracks
        .len()
        .min(area.width.saturating_sub(16).max(3) as usize / 4);
    let selected_scene = state
        .pattern_index
        .min(song.clip_scenes.len().saturating_sub(1));
    let selected_track = state.cursor.track.min(song.tracks.len().saturating_sub(1));
    let active_scene = state.sequence_position;
    let queued_scene = state
        .is_playing
        .then_some(selected_scene)
        .filter(|scene| active_scene != Some(*scene));
    let visible_items = list_inner_height(area).saturating_sub(4);
    let active_scroll_index = active_scene.unwrap_or(selected_scene);
    let start = centered_scroll_offset(song.clip_scenes.len(), active_scroll_index, visible_items);
    let end = start
        .saturating_add(visible_items)
        .min(song.clip_scenes.len());

    let mut lines = Vec::new();
    let mut header = String::from("SCENE        ");
    for track_index in 0..visible_tracks {
        header.push_str(&format!("T{:02} ", track_index + 1));
    }
    lines.push(Line::styled(
        header,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));

    if song.clip_scenes.is_empty() {
        lines.push(Line::from("No clip scenes. Use :clip scene add"));
    } else {
        for (scene_index, scene) in song
            .clip_scenes
            .iter()
            .enumerate()
            .skip(start)
            .take(end.saturating_sub(start))
        {
            let marker = if active_scene == Some(scene_index) {
                ">"
            } else if queued_scene == Some(scene_index) {
                "?"
            } else if selected_scene == scene_index {
                "+"
            } else {
                " "
            };
            let mut row = format!(
                "{marker}{scene_index:02} {:<width$}",
                truncate(&scene.name, 10),
                width = 10
            );
            for track_index in 0..visible_tracks {
                let track = &song.tracks[track_index];
                let has_clip = scene.clips.iter().any(|clip| clip.track == track.id);
                let selected_cell = selected_scene == scene_index && selected_track == track_index;
                let symbol = if track.muted {
                    "M"
                } else if active_scene == Some(scene_index) && has_clip {
                    "A"
                } else if queued_scene == Some(scene_index) && has_clip {
                    "Q"
                } else if has_clip {
                    "■"
                } else {
                    "·"
                };
                if selected_cell {
                    row.push_str(&format!("[{symbol}]"));
                } else {
                    row.push_str(&format!(" {symbol} "));
                }
                row.push(' ');
            }
            let style = if active_scene == Some(scene_index) {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if selected_scene == scene_index {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            lines.push(Line::styled(row, style));
        }
    }

    lines.extend([
        Line::from(""),
        Line::from("Arrows select   A add scene   T set clip   R clear clip   Enter queue"),
        Line::from("States: ■ stopped  A active  Q queued  · empty  M muted"),
    ]);

    let title = ranged_title("Clip Launcher", start, end, song.clip_scenes.len());
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
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
        theme::MUTED
    } else if active && filled {
        theme::ACCENT
    } else if active {
        theme::TEXT
    } else if filled {
        theme::METER
    } else {
        theme::MUTED
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
        let empty = Paragraph::new("No sample loaded").block(theme::block(" Sampler "));
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
        .block(theme::block(" Sample Metadata "))
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
            format!(
                "A {}",
                sample_envelope_attack_descriptor()
                    .format_value(&ParameterValue::Seconds(sampler.envelope.0))
            ),
        ),
        Span::raw("  "),
        sampler_envelope_span(
            SamplerEnvelopeField::Decay,
            sampler.selected_envelope,
            format!(
                "D {}",
                sample_envelope_decay_descriptor()
                    .format_value(&ParameterValue::Seconds(sampler.envelope.1))
            ),
        ),
        Span::raw("  "),
        sampler_envelope_span(
            SamplerEnvelopeField::Sustain,
            sampler.selected_envelope,
            format!(
                "S {}",
                sample_envelope_sustain_descriptor()
                    .format_value(&ParameterValue::Percentage(sampler.envelope.2))
            ),
        ),
        Span::raw("  "),
        sampler_envelope_span(
            SamplerEnvelopeField::Release,
            sampler.selected_envelope,
            format!(
                "R {}",
                sample_envelope_release_descriptor()
                    .format_value(&ParameterValue::Seconds(sampler.envelope.3))
            ),
        ),
    ])
}

fn sampler_envelope_span(
    field: SamplerEnvelopeField,
    selected: SamplerEnvelopeField,
    text: String,
) -> Span<'static> {
    if field == selected {
        Span::styled(format!("[{text}]"), theme::active())
    } else {
        Span::styled(format!(" {text} "), theme::base())
    }
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

    let block = theme::block(" Track Desk ");
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
            Span::styled("TRACK ", theme::label().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:02}", state.cursor.track + 1),
                theme::base().add_modifier(Modifier::BOLD),
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
            theme::label().add_modifier(Modifier::BOLD),
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
            EffectDeviceKind::Balance { balance } => mixer_lines.push(parameter_control_from_f32(
                native_balance_descriptor(),
                balance,
            )),
            EffectDeviceKind::StereoWidth { width } => {
                mixer_lines.push(parameter_control_from_f32(native_width_descriptor(), width))
            }
            EffectDeviceKind::PhaseInvert {
                invert_left,
                invert_right,
            } => {
                mixer_lines.push(parameter_control_line(
                    &native_phase_invert_left_descriptor(),
                    ParameterValue::Bool(invert_left),
                ));
                mixer_lines.push(parameter_control_line(
                    &native_phase_invert_right_descriptor(),
                    ParameterValue::Bool(invert_right),
                ));
            }
            EffectDeviceKind::Filter {
                mode,
                cutoff_hz,
                resonance,
                drive_db,
                mix,
                ..
            } => {
                mixer_lines.push(parameter_control_line(
                    &native_filter_mode_descriptor(),
                    ParameterValue::Enum(mode.parameter_id().to_string()),
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_filter_cutoff_descriptor(),
                    cutoff_hz,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_filter_resonance_descriptor(),
                    resonance,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_filter_drive_descriptor(),
                    drive_db,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_filter_mix_descriptor(),
                    mix,
                ));
            }
            EffectDeviceKind::Delay {
                sync,
                time_left_ms,
                time_right_ms,
                feedback,
                ping_pong,
                mix,
                output_db,
                ..
            } => {
                mixer_lines.push(parameter_control_line(
                    &native_delay_sync_descriptor(),
                    ParameterValue::Bool(sync),
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_delay_time_left_descriptor(),
                    time_left_ms,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_delay_time_right_descriptor(),
                    time_right_ms,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_delay_feedback_descriptor(),
                    feedback,
                ));
                mixer_lines.push(parameter_control_line(
                    &native_delay_ping_pong_descriptor(),
                    ParameterValue::Bool(ping_pong),
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_delay_mix_descriptor(),
                    mix,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_delay_output_descriptor(),
                    output_db,
                ));
            }
            EffectDeviceKind::Reverb {
                size,
                predelay_ms,
                decay_s,
                damping,
                mix,
                output_db,
                ..
            } => {
                mixer_lines.push(parameter_control_from_f32(
                    native_reverb_size_descriptor(),
                    size,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_reverb_predelay_descriptor(),
                    predelay_ms,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_reverb_decay_descriptor(),
                    decay_s,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_reverb_damping_descriptor(),
                    damping,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_reverb_mix_descriptor(),
                    mix,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_reverb_output_descriptor(),
                    output_db,
                ));
            }
            EffectDeviceKind::Drive {
                mode,
                drive_db,
                tone,
                mix,
                output_db,
                ..
            } => {
                mixer_lines.push(parameter_control_line(
                    &native_drive_mode_descriptor(),
                    ParameterValue::Enum(mode.parameter_id().to_string()),
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_drive_drive_descriptor(),
                    drive_db,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_drive_tone_descriptor(),
                    tone,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_drive_mix_descriptor(),
                    mix,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_drive_output_descriptor(),
                    output_db,
                ));
            }
            EffectDeviceKind::Bitcrusher {
                bit_depth,
                reduction_ratio,
                mix,
                output_db,
                ..
            } => {
                mixer_lines.push(parameter_control_line(
                    &native_bitcrusher_bit_depth_descriptor(),
                    ParameterValue::Integer(i64::from(bit_depth)),
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_bitcrusher_reduction_descriptor(),
                    reduction_ratio,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_bitcrusher_mix_descriptor(),
                    mix,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_bitcrusher_output_descriptor(),
                    output_db,
                ));
            }
            EffectDeviceKind::Chorus {
                rate_hz,
                sync,
                depth,
                spread,
                mix,
                ..
            } => {
                mixer_lines.push(parameter_control_from_f32(
                    native_chorus_rate_descriptor(),
                    rate_hz,
                ));
                mixer_lines.push(parameter_control_line(
                    &native_chorus_sync_descriptor(),
                    ParameterValue::Bool(sync),
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_chorus_depth_descriptor(),
                    depth,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_chorus_spread_descriptor(),
                    spread,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_chorus_mix_descriptor(),
                    mix,
                ));
            }
            EffectDeviceKind::Flanger {
                rate_hz,
                sync,
                depth,
                manual,
                feedback,
                stereo_phase,
                mix,
                ..
            } => {
                mixer_lines.push(parameter_control_from_f32(
                    native_flanger_rate_descriptor(),
                    rate_hz,
                ));
                mixer_lines.push(parameter_control_line(
                    &native_flanger_sync_descriptor(),
                    ParameterValue::Bool(sync),
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_flanger_depth_descriptor(),
                    depth,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_flanger_manual_descriptor(),
                    manual,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_flanger_feedback_descriptor(),
                    feedback,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_flanger_stereo_phase_descriptor(),
                    stereo_phase,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_flanger_mix_descriptor(),
                    mix,
                ));
            }
            EffectDeviceKind::Phaser {
                rate_hz,
                sync,
                depth,
                center_hz,
                stages,
                feedback,
                stereo_phase,
                mix,
                ..
            } => {
                mixer_lines.push(parameter_control_from_f32(
                    native_phaser_rate_descriptor(),
                    rate_hz,
                ));
                mixer_lines.push(parameter_control_line(
                    &native_phaser_sync_descriptor(),
                    ParameterValue::Bool(sync),
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_phaser_depth_descriptor(),
                    depth,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_phaser_center_descriptor(),
                    center_hz,
                ));
                mixer_lines.push(parameter_control_line(
                    &native_phaser_stages_descriptor(),
                    ParameterValue::Integer(i64::from(stages)),
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_phaser_feedback_descriptor(),
                    feedback,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_phaser_stereo_phase_descriptor(),
                    stereo_phase,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_phaser_mix_descriptor(),
                    mix,
                ));
            }
            EffectDeviceKind::Compressor {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                makeup_db,
                mix,
                ..
            } => {
                mixer_lines.push(parameter_control_from_f32(
                    native_compressor_threshold_descriptor(),
                    threshold_db,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_compressor_ratio_descriptor(),
                    ratio,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_compressor_attack_descriptor(),
                    attack_ms,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_compressor_release_descriptor(),
                    release_ms,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_compressor_makeup_descriptor(),
                    makeup_db,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_compressor_mix_descriptor(),
                    mix,
                ));
            }
            EffectDeviceKind::Gate {
                threshold_db,
                hysteresis_db,
                attack_ms,
                release_ms,
                range_db,
                ..
            } => {
                mixer_lines.push(parameter_control_from_f32(
                    native_gate_threshold_descriptor(),
                    threshold_db,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_gate_hysteresis_descriptor(),
                    hysteresis_db,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_gate_attack_descriptor(),
                    attack_ms,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_gate_release_descriptor(),
                    release_ms,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_gate_range_descriptor(),
                    range_db,
                ));
            }
            EffectDeviceKind::Limiter {
                ceiling_db,
                input_gain_db,
                release_ms,
                lookahead_ms,
                ..
            } => {
                mixer_lines.push(parameter_control_from_f32(
                    native_limiter_ceiling_descriptor(),
                    ceiling_db,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_limiter_input_gain_descriptor(),
                    input_gain_db,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_limiter_release_descriptor(),
                    release_ms,
                ));
                mixer_lines.push(parameter_control_from_f32(
                    native_limiter_lookahead_descriptor(),
                    lookahead_ms,
                ));
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
                    SamplePlaybackMode::ForwardLoop => "forward-loop",
                    SamplePlaybackMode::BackwardLoop => "backward-loop",
                    SamplePlaybackMode::PingPongLoop => "ping-pong-loop",
                    SamplePlaybackMode::Reverse => "reverse",
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
    let command2 = cell.command2.map_or_else(
        || "---".to_string(),
        |command| format!("{}{:02X}", command.display_code(), command.value),
    );
    [
        format!("Note {note} Vel {velocity} Inst {instrument}"),
        format!("Vol {volume} Pan {pan} Dly {delay} FX1 {command} FX2 {command2}"),
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
        state.tracker_layout.pattern_fields,
    ));

    let row_state = PatternRowRenderState {
        cursor: state.cursor,
        playhead_row: state.playhead_row,
        selection: state.selection,
        show_line_numbers_hex: state.show_line_numbers_hex,
        row_number_offset: state.row_number_offset,
        pattern_divider_interval: state.pattern_divider_interval,
        pattern_highlight_interval: state.pattern_highlight_interval,
        visible_tracks: viewport.visible_tracks,
        field_layout: state.tracker_layout.pattern_fields,
    };

    for row_index in viewport.visible_rows {
        lines.push(pattern_row(song, pattern, row_index, &row_state));
    }

    let title = if state.show_pattern_top_info {
        format!(
            " Pattern Editor: {} | rows={} | tracks={} | fields={} ",
            pattern.name,
            pattern.row_count(),
            song.tracks.len(),
            state.tracker_layout.pattern_fields.label()
        )
    } else {
        " Pattern Editor ".to_string()
    };
    let block = Block::default().title(title).borders(Borders::ALL);
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
    let visible_track_capacity =
        visible_pattern_tracks(area.width, state.tracker_layout.pattern_fields);

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

fn visible_pattern_tracks(area_width: u16, field_layout: PatternFieldLayout) -> usize {
    let content_width = area_width
        .saturating_sub(2)
        .saturating_sub(ROW_GUTTER_WIDTH as u16);
    (content_width as usize)
        .div_ceil(pattern_cell_width(field_layout))
        .max(1)
}

fn pattern_header(
    song: &Song,
    active_track: usize,
    visible_tracks: Range<usize>,
    field_layout: PatternFieldLayout,
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{:<ROW_GUTTER_WIDTH$}", "ROW"),
        Style::default().fg(Color::DarkGray),
    )];
    let cell_width = pattern_cell_width(field_layout);

    for (track_index, track) in song
        .tracks
        .iter()
        .enumerate()
        .skip(visible_tracks.start)
        .take(visible_tracks.end.saturating_sub(visible_tracks.start))
    {
        let is_active = track_index == active_track;
        spans.push(Span::styled(
            format!("{:^cell_width$}", truncate(&track.name, cell_width)),
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
    row_number_offset: usize,
    pattern_divider_interval: usize,
    pattern_highlight_interval: usize,
    visible_tracks: Range<usize>,
    field_layout: PatternFieldLayout,
}

fn pattern_row(
    song: &Song,
    pattern: &Pattern,
    row_index: usize,
    state: &PatternRowRenderState,
) -> Line<'static> {
    let is_playhead = state.playhead_row == Some(row_index);
    let row_style = pattern_row_gutter_style(row_index, state);
    let mut spans = vec![Span::styled(
        format!(
            "{:<ROW_GUTTER_WIDTH$}",
            format!(
                "{}{}",
                if is_playhead { ">" } else { " " },
                format_row_number(
                    row_index,
                    state.show_line_numbers_hex,
                    state.row_number_offset
                )
            )
        ),
        if is_playhead {
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        } else {
            row_style
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
            state.field_layout,
        ));
    }

    Line::from(spans)
}

fn pattern_row_gutter_style(row_index: usize, state: &PatternRowRenderState) -> Style {
    if state.pattern_highlight_interval > 0
        && row_index.is_multiple_of(state.pattern_highlight_interval)
    {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if state.pattern_divider_interval > 0
        && row_index.is_multiple_of(state.pattern_divider_interval)
    {
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn format_row_number(row: usize, hexadecimal: bool, offset: usize) -> String {
    let display_row = row.saturating_add(offset);
    if hexadecimal {
        format!("{:02X}", display_row.min(0xff))
    } else {
        format!("{display_row:02}")
    }
}

fn cell_spans(
    cell: &PatternCell,
    focused_field: CellField,
    focused: bool,
    selected: bool,
    playing: bool,
    active_track: bool,
    field_layout: PatternFieldLayout,
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
    let command2 = cell.command2.map_or_else(
        || "---".to_string(),
        |command| format!("{}{:02X}", command.display_code(), command.value),
    );

    let normal = theme::base();
    let focused_style = theme::active();
    let selected_style = theme::selected();
    let playing_style = theme::playing();
    let active_track_style = Style::default().fg(theme::TEXT).bg(theme::BORDER_DIM);
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

    let mut spans = Vec::new();
    let mut used_width = 0;
    let mut push_field = |value: String, field: CellField| {
        spans.push(Span::styled(" ", spacer_style));
        spans.push(Span::styled(value.clone(), style_for_field(field)));
        used_width += 1 + value.len();
    };

    match field_layout {
        PatternFieldLayout::Full => {
            push_field(note, CellField::Note);
            push_field(velocity, CellField::Velocity);
            push_field(instrument, CellField::Instrument);
            push_field(volume, CellField::Volume);
            push_field(pan, CellField::Pan);
            push_field(delay, CellField::Delay);
            push_field(command, CellField::Effect);
            push_field(command2, CellField::Effect2);
        }
        PatternFieldLayout::Note => push_field(note, CellField::Note),
        PatternFieldLayout::Instrument => push_field(instrument, CellField::Instrument),
        PatternFieldLayout::Fx => {
            push_field(command, CellField::Effect);
            push_field(command2, CellField::Effect2);
        }
        PatternFieldLayout::NoteInstrument => {
            push_field(note, CellField::Note);
            push_field(instrument, CellField::Instrument);
        }
        PatternFieldLayout::NoteFx => {
            push_field(note, CellField::Note);
            push_field(command, CellField::Effect);
            push_field(command2, CellField::Effect2);
        }
        PatternFieldLayout::InstrumentFx => {
            push_field(instrument, CellField::Instrument);
            push_field(command, CellField::Effect);
            push_field(command2, CellField::Effect2);
        }
    }
    let cell_width = pattern_cell_width(field_layout);
    if field_layout != PatternFieldLayout::Full && used_width < cell_width {
        spans.push(Span::styled(
            " ".repeat(cell_width - used_width),
            spacer_style,
        ));
    }
    spans
}

fn pattern_cell_width(field_layout: PatternFieldLayout) -> usize {
    match field_layout {
        PatternFieldLayout::Full => PATTERN_CELL_WIDTH,
        PatternFieldLayout::Note => 5,
        PatternFieldLayout::Instrument => 4,
        PatternFieldLayout::Fx => 9,
        PatternFieldLayout::NoteInstrument => 8,
        PatternFieldLayout::NoteFx => 13,
        PatternFieldLayout::InstrumentFx => 12,
    }
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
            NotificationKind::Info => theme::label(),
            NotificationKind::Success => theme::playing(),
            NotificationKind::Warning => theme::warning(),
            NotificationKind::Error => theme::error(),
        };
        let status = Paragraph::new(Line::from(vec![
            Span::styled(format!(" {label} "), style.add_modifier(Modifier::BOLD)),
            Span::styled(notification.message.to_string(), theme::base()),
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
    } else if state.active_view == TuiView::DspRack {
        format!(
            " {} | H Help | Esc Pattern | Tab Track/Master | Up/Down Device | [/]/Left/Right Param | A Add | P/R/C Lock | Ctrl+S Save | q Quit ",
            state.mode_label
        )
    } else if state.active_view == TuiView::SampleBrowser {
        format!(
            " {} | H Help | Esc Sampler | Up/Down Select | A Assign | Right-click Assign | Enter Load/Open | Backspace Parent | : Command | q Quit ",
            state.mode_label
        )
    } else if state.active_view == TuiView::ProjectBrowser {
        format!(
            " {} | H Help | Esc Tracker | Up/Down Select | Enter Open | Backspace Parent | r Refresh | : Command | q Quit ",
            state.mode_label
        )
    } else if state.active_view == TuiView::AiChat {
        format!(
            " {} | Enter Submit | a Apply | r Reject | p Preview | Ctrl+C Cancel Task | Esc Tracker | : Command | q Quit ",
            state.mode_label
        )
    } else {
        let field_segment = if state.tracker_layout.pattern_fields == PatternFieldLayout::Full {
            String::new()
        } else {
            format!(" | Fields {}", state.tracker_layout.pattern_fields.label())
        };
        format!(
            " {}{} | Step {}{} | Ctrl+P Palette | H Help | Focus :t/:p/:se/:tr/:sa/:sb/:o | F4-MIDI | Space Play/Stop | Enter Row | Shift+Enter Seq | L Loop | N/P/X Pattern | A/Y/R Seq | : Command | i Edit | V Select | Ctrl+S Save | q Quit ",
            state.mode_label,
            if state.selection.is_some() { " SEL" } else { "" },
            state.edit_step,
            field_segment
        )
    };
    let status = Paragraph::new(text).style(theme::base());
    frame.render_widget(status, area);
}

fn render_ai_chat_view(frame: &mut Frame<'_>, area: Rect, chat: Option<AiChatViewState<'_>>) {
    let Some(chat) = chat else {
        let empty = Paragraph::new("AI chat unavailable")
            .block(Block::default().title(" AI Chat ").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    };
    let chunks = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(
                chat.proposal_preview
                    .map(|preview| (preview.lines.len() as u16 + 2).clamp(3, 7))
                    .unwrap_or(0),
            ),
            Constraint::Length(3),
        ])
        .split(area);
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Provider ", Style::default().fg(Color::Cyan)),
            Span::raw(chat.provider.to_string()),
            Span::raw(" | "),
            Span::styled("Status ", Style::default().fg(Color::Cyan)),
            Span::raw(chat.status.to_string()),
        ]),
        Line::from(chat.selected_context.to_string()),
    ])
    .block(Block::default().title(" AI Chat ").borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    let available_rows = chunks[1].height.saturating_sub(2) as usize;
    let skip = chat.messages.len().saturating_sub(available_rows);
    let mut lines = Vec::new();
    for message in chat.messages.iter().skip(skip) {
        lines.push(Line::from(vec![
            Span::styled(
                ai_chat_role_label(message.role),
                Style::default()
                    .fg(ai_chat_role_color(message.role))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(message.text.to_string()),
        ]));
    }
    if lines.is_empty() {
        lines.push(Line::from(
            "No messages yet. Type a prompt below and press Enter.",
        ));
    }
    let transcript = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(Block::default().title(" Thread ").borders(Borders::ALL));
    frame.render_widget(transcript, chunks[1]);

    if let Some(preview) = chat.proposal_preview {
        let lines = preview
            .lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect::<Vec<_>>();
        let proposal = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(" Selected Proposal ")
                .borders(Borders::ALL),
        );
        frame.render_widget(proposal, chunks[2]);
    }

    let composer = Paragraph::new(chat.composer.to_string())
        .wrap(Wrap { trim: false })
        .block(Block::default().title(" Composer ").borders(Borders::ALL));
    frame.render_widget(composer, chunks[3]);
}

fn ai_chat_role_label(role: AiChatMessageRole) -> &'static str {
    match role {
        AiChatMessageRole::System => "system:",
        AiChatMessageRole::User => "user:",
        AiChatMessageRole::Assistant => "assistant:",
        AiChatMessageRole::Error => "error:",
        AiChatMessageRole::Progress => "progress:",
    }
}

fn ai_chat_role_color(role: AiChatMessageRole) -> Color {
    match role {
        AiChatMessageRole::System => Color::Yellow,
        AiChatMessageRole::User => Color::Green,
        AiChatMessageRole::Assistant => Color::Cyan,
        AiChatMessageRole::Error => Color::Red,
        AiChatMessageRole::Progress => Color::Magenta,
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
#[path = "render_tests/ai_chat.rs"]
mod render_ai_chat_tests;
#[cfg(test)]
#[path = "render_tests/clips.rs"]
mod render_clip_tests;
#[cfg(test)]
#[path = "render_tests/display.rs"]
mod render_display_tests;
#[cfg(test)]
#[path = "render_tests/layout.rs"]
mod render_layout_tests;
#[cfg(test)]
#[path = "render_tests/overlays.rs"]
mod render_overlay_tests;
#[cfg(test)]
#[path = "render_tests/pattern.rs"]
mod render_pattern_tests;
#[cfg(test)]
#[path = "render_tests/sequence.rs"]
mod render_sequence_tests;
#[cfg(test)]
#[path = "render_tests/support.rs"]
mod render_test_support;
#[cfg(test)]
#[path = "render_tests/waveform.rs"]
mod render_waveform_tests;
