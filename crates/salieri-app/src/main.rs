mod app;
mod app_effect;
mod app_event;
mod app_mode;
mod browser_io;
mod cli;
mod command;
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
    BrowserCommand, CommandDomain, FocusTarget, LoopCommand, PlayCommand, SalieriCommand,
    ViewCommand,
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
    DspFilterMode as AudioDspFilterMode, DspGraphSpec, OfflineRenderSpec, OfflineSamplerEvent,
    OfflineSamplerSample, TrackDspChainSpec,
};
use salieri_core::{
    mixer_master_gain_descriptor, mixer_send_gain_descriptor, mixer_track_gain_descriptor,
    mixer_track_pan_descriptor, native_balance_descriptor, native_filter_cutoff_descriptor,
    native_filter_drive_descriptor, native_filter_mix_descriptor, native_filter_mode_descriptor,
    native_filter_resonance_descriptor, native_gain_descriptor, native_pan_descriptor,
    native_phase_invert_left_descriptor, native_phase_invert_right_descriptor,
    native_width_descriptor, row_duration_micros, sample_gain_descriptor, sampler_events,
    AutomationTarget, CellField, Cursor, Direction, EffectDevice, EffectDeviceKind, FilterMode,
    FilterSpec, InstrumentId, NoteEvent, ParameterDescriptor, ParameterId, ParameterLock,
    ParameterLockAction, ParameterLockTarget, PatternCell, SampleEnvelope, SamplePlaybackMode,
    SamplePlaybackSettings, SelectionBounds, SelectionEndpoint, Song, TrackerCommand,
    TrackerSelection, MIXER_MASTER_GAIN_PARAMETER_ID, MIXER_SEND_GAIN_PARAMETER_ID,
    MIXER_TRACK_GAIN_PARAMETER_ID, MIXER_TRACK_PAN_PARAMETER_ID, NATIVE_BALANCE_PARAMETER_ID,
    NATIVE_FILTER_CUTOFF_PARAMETER_ID, NATIVE_FILTER_DRIVE_PARAMETER_ID,
    NATIVE_FILTER_MIX_PARAMETER_ID, NATIVE_FILTER_MODE_PARAMETER_ID,
    NATIVE_FILTER_RESONANCE_PARAMETER_ID, NATIVE_GAIN_PARAMETER_ID, NATIVE_PAN_PARAMETER_ID,
    NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID, NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID,
    NATIVE_WIDTH_PARAMETER_ID, SAMPLE_GAIN_PARAMETER_ID,
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
    render, HelpTab, MidiPortView, MidiSettingsState, NotificationKind, NotificationView,
    ProjectBrowserEntryKind, ProjectBrowserEntryView, ProjectBrowserViewState,
    SampleBrowserEntryKind, SampleBrowserEntryView, SampleBrowserViewState, SamplerEnvelopeField,
    SamplerViewState, SelectionRect, TuiState, TuiView, ViewportAxis,
};
use serde::{Deserialize, Serialize};
use task_runtime::TaskRuntime;
use terminal::TerminalGuard;

const UI_TICK_RATE: Duration = Duration::from_millis(33);
const NOTIFICATION_TTL: Duration = Duration::from_secs(4);
const SAMPLE_WAVEFORM_BUCKETS: usize = 2048;
const SAMPLE_WAVEFORM_MAX_ZOOM: usize = 64;
const DEFAULT_NOTE_VELOCITY: u8 = 0x7f;
const MAX_SAMPLER_ENVELOPE_SECONDS: f32 = 60.0;
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
    help_scroll: usize,
    help_tab: HelpTab,
    command_buffer: String,
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
