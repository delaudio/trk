mod app;
mod app_effect;
mod app_event;
mod app_mode;
mod browser_io;
mod cli;
mod command;
mod command_palette;
mod config;
mod event_handler;
mod focus;
mod helpers;
mod history;
mod intent_handler;
mod keymap;
mod midi_cli;
mod notifications;
mod persistence;
mod playback_runtime;
mod runner;
mod task_integration;
mod task_runtime;
mod terminal;
mod workflows;

#[cfg(test)]
mod tests;

use browser_io::*;
use cli::*;
use helpers::*;
use midi_cli::*;
use workflows::*;

use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use app_event::{
    AppDispatcher, AppEvent, AppIntent, AppTaskResult, NavigationIntent, ParameterIntent,
    PreparedAiProposal, RequestId, RuntimeEvent, TrackerIntent, TransportIntent,
};
use app_mode::AppMode;
use command::{
    BrowserCommand, CommandDomain, FocusTarget, LayoutCommand, LayoutPanelCommand,
    LayoutPresetCommand, LoopCommand, PlayCommand, SalieriCommand, ViewCommand,
};
use command_palette::{
    command_palette_results, CommandPaletteActionKind, CommandPaletteContext,
    CommandPaletteInternalAction, CommandPaletteMatch,
};
use config::{load_config, AppConfig, ConfigOverrides, ProjectBrowserConfig, SampleBrowserConfig};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use focus::{FocusCapture, FocusManager, FocusPanel};
use history::{SongTransaction, TransactionSpec, UndoHistory};
use keymap::Keymap;
use persistence::{load_project, save_project};
use playback_runtime::PlaybackRuntime;
use playback_runtime::{apply_sample_playback_settings, resolve_sample_path};
use salieri_ai::{apply_proposal, AiProposal, CellAddress};
use salieri_audio::{
    encode_audio, prepare_realtime_sample, render_sampler_events_with_dsp, AudioConfig,
    AudioExportFormat, DspDeviceKind as AudioDspDeviceKind, DspDeviceSpec,
    DspDriveMode as AudioDspDriveMode, DspDynamicsDetector as AudioDspDynamicsDetector,
    DspFilterMode as AudioDspFilterMode, DspGraphSpec, OfflineRenderSpec, OfflineSamplerEvent,
    OfflineSamplerSample, TrackDspChainSpec,
};
use salieri_core::{
    mixer_master_gain_descriptor, mixer_send_gain_descriptor, mixer_track_gain_descriptor,
    mixer_track_pan_descriptor, native_balance_descriptor, native_bitcrusher_bit_depth_descriptor,
    native_bitcrusher_dither_descriptor, native_bitcrusher_mix_descriptor,
    native_bitcrusher_output_descriptor, native_bitcrusher_reduction_descriptor,
    native_delay_feedback_descriptor, native_delay_filter_high_cut_descriptor,
    native_delay_filter_low_cut_descriptor, native_delay_mix_descriptor,
    native_delay_output_descriptor, native_delay_ping_pong_descriptor,
    native_delay_sync_descriptor, native_delay_time_left_descriptor,
    native_delay_time_right_descriptor, native_drive_bias_descriptor,
    native_drive_drive_descriptor, native_drive_mix_descriptor, native_drive_mode_descriptor,
    native_drive_output_descriptor, native_drive_tone_descriptor, native_filter_cutoff_descriptor,
    native_filter_drive_descriptor, native_filter_mix_descriptor, native_filter_mode_descriptor,
    native_filter_resonance_descriptor, native_gain_descriptor, native_pan_descriptor,
    native_phase_invert_left_descriptor, native_phase_invert_right_descriptor,
    native_reverb_damping_descriptor, native_reverb_decay_descriptor,
    native_reverb_diffusion_descriptor, native_reverb_early_reflections_descriptor,
    native_reverb_high_cut_descriptor, native_reverb_low_cut_descriptor,
    native_reverb_mix_descriptor, native_reverb_output_descriptor,
    native_reverb_predelay_descriptor, native_reverb_size_descriptor,
    native_reverb_width_descriptor, native_width_descriptor, row_duration_micros,
    sample_envelope_attack_descriptor, sample_envelope_decay_descriptor,
    sample_envelope_release_descriptor, sample_envelope_sustain_descriptor, sample_gain_descriptor,
    sampler_events, AutomationTarget, BitcrusherSpec, CellField, ChorusSpec, CompressorSpec,
    Cursor, DelaySpec, Direction, DriveMode, DriveSpec, DynamicsDetector, EffectDevice,
    EffectDeviceKind, FilterMode, FilterSpec, FlangerSpec, GateSpec, InstrumentId, LimiterSpec,
    NoteEvent, ParameterDescriptor, ParameterId, ParameterLock, ParameterLockAction,
    ParameterLockTarget, PatternCell, PhaserSpec, ReverbSpec, SampleEnvelope, SamplePlaybackMode,
    SamplePlaybackSettings, SelectionBounds, SelectionEndpoint, Song, TrackerCommand,
    TrackerSelection, MIXER_MASTER_GAIN_PARAMETER_ID, MIXER_SEND_GAIN_PARAMETER_ID,
    MIXER_TRACK_GAIN_PARAMETER_ID, MIXER_TRACK_PAN_PARAMETER_ID, NATIVE_BALANCE_PARAMETER_ID,
    NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID, NATIVE_BITCRUSHER_DITHER_PARAMETER_ID,
    NATIVE_BITCRUSHER_MIX_PARAMETER_ID, NATIVE_BITCRUSHER_OUTPUT_PARAMETER_ID,
    NATIVE_BITCRUSHER_REDUCTION_PARAMETER_ID, NATIVE_DELAY_FEEDBACK_PARAMETER_ID,
    NATIVE_DELAY_FILTER_HIGH_CUT_PARAMETER_ID, NATIVE_DELAY_FILTER_LOW_CUT_PARAMETER_ID,
    NATIVE_DELAY_MIX_PARAMETER_ID, NATIVE_DELAY_OUTPUT_PARAMETER_ID,
    NATIVE_DELAY_PING_PONG_PARAMETER_ID, NATIVE_DELAY_SYNC_PARAMETER_ID,
    NATIVE_DELAY_TIME_LEFT_PARAMETER_ID, NATIVE_DELAY_TIME_RIGHT_PARAMETER_ID,
    NATIVE_DRIVE_BIAS_PARAMETER_ID, NATIVE_DRIVE_DRIVE_PARAMETER_ID, NATIVE_DRIVE_MIX_PARAMETER_ID,
    NATIVE_DRIVE_MODE_PARAMETER_ID, NATIVE_DRIVE_OUTPUT_PARAMETER_ID,
    NATIVE_DRIVE_TONE_PARAMETER_ID, NATIVE_FILTER_CUTOFF_PARAMETER_ID,
    NATIVE_FILTER_DRIVE_PARAMETER_ID, NATIVE_FILTER_MIX_PARAMETER_ID,
    NATIVE_FILTER_MODE_PARAMETER_ID, NATIVE_FILTER_RESONANCE_PARAMETER_ID,
    NATIVE_GAIN_PARAMETER_ID, NATIVE_PAN_PARAMETER_ID, NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID,
    NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID, NATIVE_REVERB_DAMPING_PARAMETER_ID,
    NATIVE_REVERB_DECAY_PARAMETER_ID, NATIVE_REVERB_DIFFUSION_PARAMETER_ID,
    NATIVE_REVERB_EARLY_REFLECTIONS_PARAMETER_ID, NATIVE_REVERB_HIGH_CUT_PARAMETER_ID,
    NATIVE_REVERB_LOW_CUT_PARAMETER_ID, NATIVE_REVERB_MIX_PARAMETER_ID,
    NATIVE_REVERB_OUTPUT_PARAMETER_ID, NATIVE_REVERB_PREDELAY_PARAMETER_ID,
    NATIVE_REVERB_SIZE_PARAMETER_ID, NATIVE_REVERB_WIDTH_PARAMETER_ID, NATIVE_WIDTH_PARAMETER_ID,
    SAMPLE_GAIN_PARAMETER_ID,
};
use salieri_interop::{
    extract_xrns_sample_payloads, import_xrns, import_xrns_with_sample_paths,
    XrnsDiagnosticSeverity,
};
use salieri_midi::{
    list_input_ports, list_output_ports, MidiClockMessage, MidiInput, MidiInputEvent,
    MidiInputPacket, MidiInputPort, MidiMessage, MidiOutput, MidiOutputPort, MidirMidiInput,
    MidirMidiOutput,
};
use salieri_sampler::{Sample, WaveformBucket, WaveformOverview};
use salieri_transform::{apply_euclidean, EuclideanRhythm};
use salieri_tui::{
    render, CommandPaletteEntryView, CommandPaletteViewState, HelpTab, ManagedPanelId,
    MidiPortView, MidiSettingsState, NotificationKind, NotificationView, ProjectBrowserEntryKind,
    ProjectBrowserEntryView, ProjectBrowserViewState, SampleBrowserEntryKind,
    SampleBrowserEntryView, SampleBrowserViewState, SamplerEnvelopeField, SamplerViewState,
    SelectionRect, TrackerLayoutPreset, TrackerLayoutState, TuiState, TuiView, ViewportAxis,
};
use serde::{Deserialize, Serialize};
use task_runtime::TaskRuntime;
use terminal::TerminalGuard;

const UI_TICK_RATE: Duration = Duration::from_millis(33);
const NOTIFICATION_TTL: Duration = Duration::from_secs(4);
const SAMPLE_WAVEFORM_BUCKETS: usize = 2048;
const SAMPLE_WAVEFORM_MAX_ZOOM: usize = 64;
const DEFAULT_NOTE_VELOCITY: u8 = 0x7f;
const MIN_BPM: u16 = 1;
const MAX_BPM: u16 = 999;
const MIN_LPB: u8 = 1;
const MAX_LPB: u8 = 32;

fn main() -> Result<()> {
    runner::run_cli()
}

#[derive(Debug)]
struct App {
    dispatcher: AppDispatcher,
    next_request_id: u64,
    pending_project_load: Option<RequestId>,
    task_runtime: TaskRuntime<AppTaskResult>,
    keymap: Keymap,
    song: Song,
    clean_song: Song,
    project_path: Option<PathBuf>,
    focus: FocusManager,
    pattern_index: usize,
    cursor: Cursor,
    row_offset: usize,
    track_offset: usize,
    mode: AppMode,
    octave: u8,
    edit_step: usize,
    vim_navigation: bool,
    pending_goto_start: bool,
    follow_playhead: bool,
    show_line_numbers_hex: bool,
    tracker_layout: TrackerLayoutState,
    help_scroll: usize,
    help_tab: HelpTab,
    command_buffer: String,
    command_palette_query: String,
    command_palette_selected: usize,
    command_palette_recent: Vec<String>,
    clipboard: Option<Clipboard>,
    selection: Option<TrackerSelection>,
    history: UndoHistory,
    playback: PlaybackRuntime,
    is_playing: bool,
    loop_pattern: bool,
    playhead_row: Option<usize>,
    sequence_position: Option<usize>,
    sequence_cursor: usize,
    midi_status: String,
    midi_ports: Vec<MidiOutputPort>,
    midi_port_cursor: usize,
    midi_input_status: String,
    midi_input_ports: Vec<MidiInputPort>,
    midi_input: Option<AppMidiInput>,
    midi_record_armed: bool,
    midi_clock_follow: bool,
    midi_clock_ticks: u32,
    sample_view: Option<AppSampleView>,
    sample_waveform_zoom: usize,
    sample_waveform_offset: usize,
    sampler_envelope_field: SamplerEnvelopeField,
    sample_browser: SampleBrowserConfig,
    pending_sample_browser: Option<SampleBrowserRequest>,
    sample_browser_view: Option<AppSampleBrowserView>,
    project_browser: ProjectBrowserConfig,
    recent_project_file: Option<PathBuf>,
    recent_projects: Vec<PathBuf>,
    recent_project_limit: usize,
    config_metadata: config::ConfigMetadata,
    project_browser_view: Option<AppProjectBrowserView>,
    pending_ai_proposal: Option<PreparedAiProposal>,
    dirty: bool,
    should_quit: bool,
    dialog: Option<Dialog>,
    notification: Option<Notification>,
    last_tick: Instant,
}

struct AppMidiInput {
    inner: Box<dyn MidiInput>,
}

impl AppMidiInput {
    fn new(input: impl MidiInput + 'static) -> Self {
        Self {
            inner: Box::new(input),
        }
    }
}

impl std::fmt::Debug for AppMidiInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppMidiInput")
            .finish_non_exhaustive()
    }
}

impl MidiInput for AppMidiInput {
    fn poll(&mut self) -> Result<Option<MidiInputPacket>, salieri_midi::MidiInputError> {
        self.inner.poll()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Dialog {
    QuitDirty,
    DeleteTrack {
        track_index: usize,
        message: String,
    },
    DeletePattern {
        pattern_index: usize,
        message: String,
    },
    OpenProjectDirty {
        path: PathBuf,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RecentProjectsFile {
    projects: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Notification {
    kind: NotificationKind,
    message: String,
    expires_at: Instant,
}

#[derive(Debug, Clone, PartialEq)]
struct AppSampleView {
    source_path: PathBuf,
    sample: Sample,
    overview: WaveformOverview,
}

#[derive(Debug, Clone, PartialEq)]
struct AppSampleBrowserView {
    current_dir: PathBuf,
    entries: Vec<AppSampleBrowserEntry>,
    cursor: usize,
    preview: Option<AppSampleView>,
    message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppSampleBrowserEntry {
    path: PathBuf,
    name: String,
    kind: SampleBrowserEntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppProjectBrowserView {
    current_dir: PathBuf,
    entries: Vec<AppProjectBrowserEntry>,
    cursor: usize,
    message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppProjectBrowserEntry {
    path: PathBuf,
    name: String,
    kind: ProjectBrowserEntryKind,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SampleBrowserRequest {
    start_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
enum Clipboard {
    Cell(PatternCell),
    Region(ClipboardRegion),
}

#[derive(Debug, Clone, PartialEq)]
struct ClipboardRegion {
    cells: Vec<Vec<PatternCell>>,
}

impl command::CommandExecutor for App {
    type Error = std::convert::Infallible;

    fn execute(&mut self, command: SalieriCommand) -> Result<(), Self::Error> {
        self.dispatch_intent(AppIntent::Command(command));
        Ok(())
    }
}
