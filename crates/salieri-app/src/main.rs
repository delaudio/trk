mod config;
mod persistence;
mod playback_runtime;
mod terminal;

use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use config::{load_config, AppConfig};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use persistence::{load_project, save_project};
use playback_runtime::{PlaybackRuntime, PlaybackUpdate};
use salieri_core::{CellField, Cursor, Direction, NoteEvent, PatternCell, Song};
use salieri_midi::{list_output_ports, MidiMessage, MidiOutput, MidiOutputPort, MidirMidiOutput};
use salieri_tui::{
    render, MidiPortView, MidiSettingsState, NotificationKind, NotificationView, SelectionRect,
    TuiState,
};
use terminal::TerminalGuard;

const UI_TICK_RATE: Duration = Duration::from_millis(33);
const NOTIFICATION_TTL: Duration = Duration::from_secs(4);
const DEFAULT_NOTE_VELOCITY: u8 = 0x7f;
const UNDO_LIMIT: usize = 100;
const MIN_BPM: u16 = 1;
const MAX_BPM: u16 = 999;
const MIN_LPB: u8 = 1;
const MAX_LPB: u8 = 32;

fn main() -> Result<()> {
    let args = CliArgs::parse(std::env::args().skip(1));
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            args.log_level
                .as_deref()
                .map(tracing_subscriber::EnvFilter::new)
                .unwrap_or_else(|| {
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "salieri=info".into())
                }),
        )
        .init();

    let result = run(args);
    if let Err(error) = &result {
        tracing::error!(?error, "application exited with an error");
    }
    result
}

fn run(args: CliArgs) -> Result<()> {
    match args.command {
        CliCommand::Help => {
            print_help();
            return Ok(());
        }
        CliCommand::Version => {
            println!("salieri {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        CliCommand::ListMidiOutputs => {
            print_midi_outputs()?;
            return Ok(());
        }
        CliCommand::Run | CliCommand::MidiTest => {}
    }

    let mut config = load_config(args.config_path.as_deref())?;
    if let Some(midi_log_path) = args.midi_log_path {
        config.midi.log_file = Some(midi_log_path);
    }
    if args.command == CliCommand::MidiTest {
        run_midi_test(&config, &args.midi_test)?;
        return Ok(());
    }

    let project_path = args.project_path;
    let mut app = match &project_path {
        Some(path) => App::from_file(path, config)
            .with_context(|| format!("failed to open project {}", path.display()))?,
        None => App::new(config),
    };
    let mut terminal = TerminalGuard::enter()?;

    loop {
        app.drain_playback_updates();
        app.expire_notification();
        app.keep_active_row_visible(terminal.visible_pattern_rows());
        terminal.draw(|frame| {
            let midi_ports = app.tui_midi_ports();
            let midi_settings = app.tui_midi_settings(&midi_ports);
            let notification = app.tui_notification();
            render(
                frame,
                &app.song,
                TuiState {
                    cursor: app.cursor,
                    row_offset: app.row_offset,
                    pattern_index: app.pattern_index,
                    selection: app.selection_rect(),
                    mode_label: app.mode.label(),
                    octave: app.octave,
                    dirty: app.dirty,
                    show_line_numbers_hex: app.show_line_numbers_hex,
                    command_line: app.command_line(),
                    notification,
                    show_help: app.mode == AppMode::Help,
                    is_playing: app.is_playing,
                    loop_pattern: app.loop_pattern,
                    playhead_row: app.playhead_row,
                    midi_status: app.midi_status.as_str(),
                    sequence_position: app.tui_sequence_position(),
                    quit_confirmation: app.quit_confirmation(),
                    delete_confirmation: app.delete_confirmation_message(),
                    midi_settings,
                },
            );
        })?;

        if app.should_quit {
            break;
        }

        let timeout = UI_TICK_RATE
            .checked_sub(app.last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    app.handle_key(key);
                    app.keep_active_row_visible(terminal.visible_pattern_rows());
                }
                Event::Resize(_, _) => app.keep_active_row_visible(terminal.visible_pattern_rows()),
                _ => {}
            }
        }

        if app.last_tick.elapsed() >= UI_TICK_RATE {
            app.last_tick = Instant::now();
            app.keep_active_row_visible(terminal.visible_pattern_rows());
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliArgs {
    command: CliCommand,
    project_path: Option<PathBuf>,
    config_path: Option<PathBuf>,
    log_level: Option<String>,
    midi_log_path: Option<PathBuf>,
    midi_test: MidiTestArgs,
}

impl CliArgs {
    fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut project_path = None;
        let mut config_path = None;
        let mut log_level = None;
        let mut midi_log_path = None;
        let mut midi_test = MidiTestArgs::default();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    return Self {
                        command: CliCommand::Help,
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "-V" | "--version" => {
                    return Self {
                        command: CliCommand::Version,
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "--list-midi-outputs" => {
                    return Self {
                        command: CliCommand::ListMidiOutputs,
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "--midi-test-output" => {
                    midi_test.output = args.next();
                }
                "--midi-test-channel" => {
                    if let Some(value) = args.next().and_then(|value| value.parse::<u8>().ok()) {
                        midi_test.channel = value;
                    }
                }
                "--midi-test-note" => {
                    if let Some(value) = args.next().and_then(|value| value.parse::<u8>().ok()) {
                        midi_test.note = value;
                    }
                }
                "--midi-test-duration-ms" => {
                    if let Some(value) = args.next().and_then(|value| value.parse::<u64>().ok()) {
                        midi_test.duration_ms = value;
                    }
                }
                "--config" => {
                    if let Some(path) = args.next() {
                        config_path = Some(PathBuf::from(path));
                    }
                }
                "--log-level" => {
                    log_level = args.next();
                }
                "--midi-log" => {
                    if let Some(path) = args.next() {
                        midi_log_path = Some(PathBuf::from(path));
                    }
                }
                _ if arg.starts_with("--config=") => {
                    config_path = Some(PathBuf::from(arg.trim_start_matches("--config=")));
                }
                _ if arg.starts_with("--log-level=") => {
                    log_level = Some(arg.trim_start_matches("--log-level=").to_string());
                }
                _ if arg.starts_with("--midi-log=") => {
                    midi_log_path = Some(PathBuf::from(arg.trim_start_matches("--midi-log=")));
                }
                _ if arg.starts_with("--midi-test-output=") => {
                    midi_test.output =
                        Some(arg.trim_start_matches("--midi-test-output=").to_string());
                }
                _ if arg.starts_with("--midi-test-channel=") => {
                    if let Ok(value) = arg.trim_start_matches("--midi-test-channel=").parse::<u8>()
                    {
                        midi_test.channel = value;
                    }
                }
                _ if arg.starts_with("--midi-test-note=") => {
                    if let Ok(value) = arg.trim_start_matches("--midi-test-note=").parse::<u8>() {
                        midi_test.note = value;
                    }
                }
                _ if arg.starts_with("--midi-test-duration-ms=") => {
                    if let Ok(value) = arg
                        .trim_start_matches("--midi-test-duration-ms=")
                        .parse::<u64>()
                    {
                        midi_test.duration_ms = value;
                    }
                }
                _ if project_path.is_none() => project_path = Some(PathBuf::from(arg)),
                _ => {}
            }
        }

        let command = if midi_test.output.is_some() {
            CliCommand::MidiTest
        } else {
            CliCommand::Run
        };

        Self {
            command,
            project_path,
            config_path,
            log_level,
            midi_log_path,
            midi_test,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MidiTestArgs {
    output: Option<String>,
    channel: u8,
    note: u8,
    duration_ms: u64,
}

impl Default for MidiTestArgs {
    fn default() -> Self {
        Self {
            output: None,
            channel: 1,
            note: 60,
            duration_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliCommand {
    Run,
    Help,
    Version,
    ListMidiOutputs,
    MidiTest,
}

fn print_help() {
    println!(
        "Salieri Tracker\n\nUsage:\n  salieri [OPTIONS] [FILE]\n  salieri --list-midi-outputs\n  salieri --midi-test-output NAME_OR_INDEX [OPTIONS]\n  salieri --help\n  salieri --version\n\nOptions:\n  --config PATH                 Load config from PATH\n  --log-level LEVEL             Set tracing filter, e.g. debug or salieri=debug\n  --midi-log PATH               Write sent MIDI messages to PATH\n  --list-midi-outputs           List available MIDI output ports\n  --midi-test-output VALUE      Send one test note to a MIDI output name or index\n  --midi-test-channel CHANNEL   Test channel, 1-16 (default 1)\n  --midi-test-note NOTE         Test MIDI note, 0-127 (default 60)\n  --midi-test-duration-ms MS    Test note length (default 1000)\n  --help                        Show this help\n  --version                     Show version"
    );
}

fn print_midi_outputs() -> Result<()> {
    let ports = match list_output_ports() {
        Ok(ports) => ports,
        Err(error) => {
            println!("MIDI output unavailable: {error}");
            return Ok(());
        }
    };
    if ports.is_empty() {
        println!("No MIDI output ports found");
        return Ok(());
    }

    for port in ports {
        println!("{}: {}", port.index, port.name);
    }

    Ok(())
}

fn run_midi_test(config: &AppConfig, args: &MidiTestArgs) -> Result<()> {
    let ports = list_output_ports().context("failed to list MIDI output ports")?;
    let output = args
        .output
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(config.midi.default_output.as_str());
    let Some((_, port)) = resolve_midi_output_port(&ports, output) else {
        anyhow::bail!("MIDI output not found: {output}");
    };

    let channel = args.channel.clamp(1, 16);
    let note = args.note.min(127);
    let duration = Duration::from_millis(args.duration_ms.max(1));
    let mut output = MidirMidiOutput::connect(port.index, "salieri-midi-test")
        .with_context(|| format!("failed to connect MIDI output {}", port.name))?;

    println!(
        "Sending MIDI test note: port {} ({}) channel {} note {} duration {}ms",
        port.index,
        port.name,
        channel,
        note,
        duration.as_millis()
    );

    send_logged_midi_message(
        &mut output,
        MidiMessage::note_on(channel, note, DEFAULT_NOTE_VELOCITY),
        config.midi.log_file.as_deref(),
    )?;
    thread::sleep(duration);
    send_logged_midi_message(
        &mut output,
        MidiMessage::note_off(channel, note, 0),
        config.midi.log_file.as_deref(),
    )?;
    thread::sleep(Duration::from_millis(20));

    println!("MIDI test complete");
    Ok(())
}

fn send_logged_midi_message(
    output: &mut impl MidiOutput,
    message: MidiMessage,
    log_file: Option<&Path>,
) -> Result<()> {
    output.send(message)?;
    if let Some(log_file) = log_file {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .with_context(|| format!("failed to open MIDI log {}", log_file.display()))?;
        let bytes = message.to_bytes();
        writeln!(
            file,
            "TEST {:?} bytes={:02X} {:02X} {:02X}",
            message, bytes[0], bytes[1], bytes[2]
        )
        .with_context(|| format!("failed to write MIDI log {}", log_file.display()))?;
    }
    Ok(())
}

#[derive(Debug)]
struct App {
    song: Song,
    clean_song: Song,
    project_path: Option<PathBuf>,
    pattern_index: usize,
    cursor: Cursor,
    row_offset: usize,
    mode: AppMode,
    octave: u8,
    edit_step: usize,
    vim_navigation: bool,
    pending_goto_start: bool,
    follow_playhead: bool,
    show_line_numbers_hex: bool,
    command_buffer: String,
    clipboard: Option<Clipboard>,
    selection_anchor: Option<SelectionAnchor>,
    undo_stack: Vec<Song>,
    redo_stack: Vec<Song>,
    playback: PlaybackRuntime,
    is_playing: bool,
    loop_pattern: bool,
    playhead_row: Option<usize>,
    sequence_position: Option<usize>,
    sequence_cursor: usize,
    midi_status: String,
    midi_ports: Vec<MidiOutputPort>,
    midi_port_cursor: usize,
    dirty: bool,
    should_quit: bool,
    dialog: Option<Dialog>,
    notification: Option<Notification>,
    last_tick: Instant,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Notification {
    kind: NotificationKind,
    message: String,
    expires_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Clipboard {
    Cell(PatternCell),
    Region(ClipboardRegion),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClipboardRegion {
    cells: Vec<Vec<PatternCell>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectionAnchor {
    row: usize,
    track: usize,
}

impl Default for App {
    fn default() -> Self {
        Self::new(AppConfig::default())
    }
}

impl App {
    fn new(config: AppConfig) -> Self {
        let song = Song::empty();
        let default_midi_output = config.midi.default_output.trim().to_string();
        let midi_status = if default_midi_output.is_empty() {
            "MIDI Disconnected".to_string()
        } else {
            format!("MIDI Disconnected ({default_midi_output})")
        };
        let mut app = Self {
            clean_song: song.clone(),
            song,
            project_path: None,
            pattern_index: 0,
            cursor: Cursor::new(),
            row_offset: 0,
            mode: AppMode::Normal,
            octave: config.keyboard.default_octave,
            edit_step: config.keyboard.edit_step.max(1),
            vim_navigation: config.keyboard.vim_navigation,
            pending_goto_start: false,
            follow_playhead: config.ui.follow_playhead,
            show_line_numbers_hex: config.ui.show_line_numbers_hex,
            command_buffer: String::new(),
            clipboard: None,
            selection_anchor: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            playback: PlaybackRuntime::spawn(config.midi.log_file.clone()),
            is_playing: false,
            loop_pattern: true,
            playhead_row: None,
            sequence_position: None,
            sequence_cursor: 0,
            midi_status,
            midi_ports: Vec::new(),
            midi_port_cursor: 0,
            dirty: false,
            should_quit: false,
            dialog: None,
            notification: None,
            last_tick: Instant::now(),
        };
        app.connect_default_midi_output(&default_midi_output);
        app
    }

    fn from_file(path: &Path, config: AppConfig) -> Result<Self> {
        let song = load_project(path)?;
        Ok(Self {
            clean_song: song.clone(),
            song,
            project_path: Some(path.to_path_buf()),
            ..Self::new(config)
        })
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.handle_control_key(key) {
            return;
        }

        match self.mode {
            AppMode::Normal => self.handle_normal_key(key),
            AppMode::Edit => self.handle_edit_key(key),
            AppMode::Command => self.handle_command_key(key),
            AppMode::Help => self.handle_help_key(key),
            AppMode::Dialog => self.handle_dialog_key(key),
            AppMode::MidiSettings => self.handle_midi_settings_key(key),
        }
    }

    fn handle_control_key(&mut self, key: KeyEvent) -> bool {
        if !key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }

        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if let Err(error) = self.save() {
                    tracing::error!(?error, "failed to save project");
                    self.notify_error(format!("Save failed: {error}"));
                }
                true
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.create_track();
                true
            }
            KeyCode::Up => {
                self.adjust_bpm(1);
                true
            }
            KeyCode::Down => {
                self.adjust_bpm(-1);
                true
            }
            KeyCode::Right => {
                self.adjust_lpb(1);
                true
            }
            KeyCode::Left => {
                self.adjust_lpb(-1);
                true
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.copy_selection_or_current_cell();
                true
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.cut_selection_or_current_cell();
                true
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.paste_clipboard();
                true
            }
            KeyCode::Delete => {
                self.delete_current_row();
                true
            }
            KeyCode::Char('z') | KeyCode::Char('Z') => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.redo();
                } else {
                    self.undo();
                }
                true
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.redo();
                true
            }
            KeyCode::Char('p') | KeyCode::Char('P')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.panic_midi();
                true
            }
            _ => true,
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        if self.pending_goto_start {
            self.pending_goto_start = false;
            if self.vim_navigation && key.code == KeyCode::Char('g') {
                self.cursor.row = 0;
                return;
            }
        }

        let direction = match key.code {
            KeyCode::Esc => {
                self.selection_anchor = None;
                return;
            }
            KeyCode::Char('q') => {
                self.request_quit(false);
                return;
            }
            KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.start_playback();
                return;
            }
            KeyCode::Char(' ') => {
                self.toggle_playback();
                return;
            }
            KeyCode::F(8) => {
                self.stop_playback();
                return;
            }
            KeyCode::F(1) => {
                self.decrement_octave();
                return;
            }
            KeyCode::F(2) => {
                self.increment_octave();
                return;
            }
            KeyCode::F(4) => {
                self.open_midi_settings();
                return;
            }
            KeyCode::Char('r') => {
                self.start_track_rename_command();
                return;
            }
            KeyCode::Char('D') => {
                self.duplicate_track(self.cursor.track);
                return;
            }
            KeyCode::Char('{') => {
                self.move_current_track_left();
                return;
            }
            KeyCode::Char('}') => {
                self.move_current_track_right();
                return;
            }
            KeyCode::Char('N') => {
                self.create_pattern();
                return;
            }
            KeyCode::Char('P') => {
                self.duplicate_current_pattern();
                return;
            }
            KeyCode::Char('X') => {
                self.request_delete_current_pattern();
                return;
            }
            KeyCode::Char('A') => {
                self.add_sequence_pattern(self.pattern_index);
                return;
            }
            KeyCode::Char(',') => {
                self.previous_sequence_position();
                return;
            }
            KeyCode::Char('.') => {
                self.next_sequence_position();
                return;
            }
            KeyCode::Char('Y') => {
                self.duplicate_selected_sequence_position();
                return;
            }
            KeyCode::Char('R') => {
                self.remove_selected_sequence_position();
                return;
            }
            KeyCode::Char('T') => {
                self.set_selected_sequence_to_current_pattern();
                return;
            }
            KeyCode::Char('<') => {
                self.move_selected_sequence_position_up();
                return;
            }
            KeyCode::Char('>') => {
                self.move_selected_sequence_position_down();
                return;
            }
            KeyCode::Char('L') => {
                self.toggle_loop();
                return;
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.start_sequence_playback_from_selected_position();
                return;
            }
            KeyCode::Enter => {
                self.start_playback_from_cursor();
                return;
            }
            KeyCode::Char('i') => {
                self.selection_anchor = None;
                self.mode = AppMode::Edit;
                return;
            }
            KeyCode::Char(':') => {
                self.command_buffer.clear();
                self.mode = AppMode::Command;
                return;
            }
            KeyCode::Char('?') | KeyCode::Char('H') => {
                self.mode = AppMode::Help;
                return;
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.start_selection();
                return;
            }
            KeyCode::Char('[') => {
                self.select_pattern(self.pattern_index.saturating_sub(1));
                return;
            }
            KeyCode::Char(']') => {
                self.select_pattern(self.pattern_index.saturating_add(1));
                return;
            }
            KeyCode::Up => Some(Direction::Up),
            KeyCode::Char('k') if self.vim_navigation => Some(Direction::Up),
            KeyCode::Down => Some(Direction::Down),
            KeyCode::Char('j') if self.vim_navigation => Some(Direction::Down),
            KeyCode::Left => Some(Direction::Left),
            KeyCode::Char('h') if self.vim_navigation => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            KeyCode::Char('l') if self.vim_navigation => Some(Direction::Right),
            KeyCode::Tab => {
                self.next_track();
                return;
            }
            KeyCode::BackTab => {
                self.previous_track();
                return;
            }
            KeyCode::Home => {
                self.cursor.row = 0;
                return;
            }
            KeyCode::End => {
                self.cursor.row = self.current_row_count().saturating_sub(1);
                return;
            }
            KeyCode::Char('g') if self.vim_navigation => {
                self.pending_goto_start = true;
                return;
            }
            KeyCode::Char('G') if self.vim_navigation => {
                self.cursor.row = self.current_row_count().saturating_sub(1);
                return;
            }
            KeyCode::PageUp => {
                self.page_cursor_up();
                return;
            }
            KeyCode::PageDown => {
                self.page_cursor_down();
                return;
            }
            KeyCode::Insert => {
                self.insert_current_row();
                return;
            }
            KeyCode::Delete => {
                if self.selection_anchor.is_some() {
                    self.clear_selection_region();
                } else {
                    self.request_delete_current_track();
                }
                return;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.toggle_current_mute();
                return;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.toggle_current_solo();
                return;
            }
            _ => None,
        };

        if let Some(direction) = direction {
            self.move_cursor(direction);
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = AppMode::Normal,
            KeyCode::Up => self.move_cursor(Direction::Up),
            KeyCode::Down => self.move_cursor(Direction::Down),
            KeyCode::Left => self.move_cursor(Direction::Left),
            KeyCode::Right => self.move_cursor(Direction::Right),
            KeyCode::Tab => self.next_track(),
            KeyCode::BackTab => self.previous_track(),
            KeyCode::Home => self.cursor.row = 0,
            KeyCode::End => self.cursor.row = self.current_row_count().saturating_sub(1),
            KeyCode::PageUp => self.page_cursor_up(),
            KeyCode::PageDown => self.page_cursor_down(),
            KeyCode::Insert => self.insert_current_row(),
            KeyCode::Delete | KeyCode::Backspace => self.clear_current_cell(),
            KeyCode::F(1) | KeyCode::Char('-') => self.decrement_octave(),
            KeyCode::F(2) | KeyCode::Char('+') | KeyCode::Char('=') => self.increment_octave(),
            KeyCode::Char('o') | KeyCode::Char('O') => self.insert_note_event(NoteEvent::NoteOff),
            KeyCode::Char('.') => self.insert_note_event(NoteEvent::NoteCut),
            KeyCode::Char(value) if self.cursor.field == CellField::Velocity => {
                if let Some(hex) = value.to_digit(16) {
                    self.enter_velocity_digit(hex as u8);
                }
            }
            KeyCode::Char(value) => {
                if let Some(note) = keyboard_note(value, self.octave) {
                    self.insert_note(note);
                }
            }
            _ => {}
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => match self.dialog.clone() {
                Some(Dialog::QuitDirty) => {
                    if let Err(error) = self.save() {
                        tracing::error!(?error, "failed to save project");
                        self.notify_error(format!("Save failed: {error}"));
                    } else {
                        self.force_quit();
                    }
                }
                Some(Dialog::DeleteTrack { track_index, .. }) => {
                    self.dialog = None;
                    self.mode = AppMode::Normal;
                    self.delete_track(track_index);
                }
                Some(Dialog::DeletePattern { pattern_index, .. }) => {
                    self.dialog = None;
                    self.mode = AppMode::Normal;
                    self.delete_pattern(pattern_index);
                }
                None => self.mode = AppMode::Normal,
            },
            KeyCode::Char('n') | KeyCode::Char('N') => match self.dialog {
                Some(Dialog::QuitDirty) => self.force_quit(),
                Some(Dialog::DeleteTrack { .. } | Dialog::DeletePattern { .. }) | None => {
                    self.cancel_dialog();
                }
            },
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Esc => {
                self.cancel_dialog();
            }
            _ => {}
        }
    }

    fn handle_midi_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.mode = AppMode::Normal,
            KeyCode::Up => self.previous_midi_port(),
            KeyCode::Char('k') if self.vim_navigation => self.previous_midi_port(),
            KeyCode::Down => self.next_midi_port(),
            KeyCode::Char('j') if self.vim_navigation => self.next_midi_port(),
            KeyCode::Home => self.midi_port_cursor = 0,
            KeyCode::End => {
                self.midi_port_cursor = self.midi_ports.len().saturating_sub(1);
            }
            KeyCode::Enter => self.connect_selected_midi_port(),
            KeyCode::Char('d') | KeyCode::Char('D') => self.disconnect_midi(),
            KeyCode::Char('p') | KeyCode::Char('P') => self.panic_midi(),
            KeyCode::F(5) | KeyCode::Char('r') | KeyCode::Char('R') => self.refresh_midi_ports(),
            _ => {}
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.command_buffer.clear();
                self.mode = AppMode::Normal;
            }
            KeyCode::Enter => self.execute_command(),
            KeyCode::Backspace => {
                self.command_buffer.pop();
            }
            KeyCode::Char(value) => self.command_buffer.push(value),
            _ => {}
        }
    }

    fn move_cursor(&mut self, direction: Direction) {
        let row_count = self.current_row_count();
        let track_count = self.song.tracks.len();
        self.cursor.move_in(direction, row_count, track_count);
    }

    fn next_track(&mut self) {
        if self.song.tracks.is_empty() {
            return;
        }
        self.cursor.track = self
            .cursor
            .track
            .saturating_add(1)
            .min(self.song.tracks.len().saturating_sub(1));
        self.cursor.digit = 0;
    }

    fn previous_track(&mut self) {
        self.cursor.track = self.cursor.track.saturating_sub(1);
        self.cursor.digit = 0;
    }

    fn page_cursor_up(&mut self) {
        self.cursor.row = self.cursor.row.saturating_sub(16);
    }

    fn page_cursor_down(&mut self) {
        self.cursor.row = self
            .cursor
            .row
            .saturating_add(16)
            .min(self.current_row_count().saturating_sub(1));
    }

    fn insert_note(&mut self, pitch: u8) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let _ = pattern.set_note(
                cursor.row,
                cursor.track,
                NoteEvent::Note { pitch },
                DEFAULT_NOTE_VELOCITY,
            );
        });
        self.advance_after_edit();
    }

    fn insert_note_event(&mut self, note: NoteEvent) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let _ = pattern.set_note_event(cursor.row, cursor.track, note, None);
        });
        self.advance_after_edit();
    }

    fn enter_velocity_digit(&mut self, digit: u8) {
        let current_digit = self.cursor.digit.min(1);
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let current_velocity = pattern
                .cell(cursor.row, cursor.track)
                .and_then(|cell| cell.velocity)
                .unwrap_or(0);
            let next_velocity = if current_digit == 0 {
                (digit << 4) | (current_velocity & 0x0f)
            } else {
                (current_velocity & 0xf0) | digit
            };
            let _ = pattern.set_velocity(cursor.row, cursor.track, next_velocity);
        });

        if current_digit == 0 {
            self.cursor.digit = 1;
        } else {
            self.cursor.digit = 0;
            self.advance_after_edit();
        }
    }

    fn clear_current_cell(&mut self) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let _ = pattern.clear_cell(cursor.row, cursor.track);
        });
    }

    fn copy_current_cell(&mut self) {
        self.clipboard = self
            .song
            .pattern(self.pattern_index)
            .and_then(|pattern| pattern.cell(self.cursor.row, self.cursor.track))
            .cloned()
            .map(Clipboard::Cell);
    }

    fn cut_current_cell(&mut self) {
        self.copy_current_cell();
        self.clear_current_cell();
    }

    fn copy_selection_or_current_cell(&mut self) {
        if let Some(selection) = self.selection_rect() {
            self.copy_selection(selection);
        } else {
            self.copy_current_cell();
        }
    }

    fn cut_selection_or_current_cell(&mut self) {
        if self.selection_anchor.is_some() {
            if let Some(selection) = self.selection_rect() {
                self.copy_selection(selection);
                self.clear_region(selection);
                self.selection_anchor = None;
            }
        } else {
            self.cut_current_cell();
        }
    }

    fn paste_clipboard(&mut self) {
        let Some(clipboard) = self.clipboard.clone() else {
            return;
        };
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            match clipboard {
                Clipboard::Cell(cell) => {
                    let _ = pattern.set_cell(cursor.row, cursor.track, cell);
                }
                Clipboard::Region(region) => {
                    for (row_offset, row) in region.cells.iter().enumerate() {
                        for (track_offset, cell) in row.iter().enumerate() {
                            let _ = pattern.set_cell(
                                cursor.row.saturating_add(row_offset),
                                cursor.track.saturating_add(track_offset),
                                cell.clone(),
                            );
                        }
                    }
                }
            }
        });
    }

    fn start_selection(&mut self) {
        self.selection_anchor = Some(SelectionAnchor {
            row: self.cursor.row,
            track: self.cursor.track,
        });
    }

    fn selection_rect(&self) -> Option<SelectionRect> {
        let anchor = self.selection_anchor?;
        let row_count = self.current_row_count();
        let track_count = self.song.tracks.len();
        if row_count == 0 || track_count == 0 {
            return None;
        }

        let anchor_row = anchor.row.min(row_count.saturating_sub(1));
        let cursor_row = self.cursor.row.min(row_count.saturating_sub(1));
        let anchor_track = anchor.track.min(track_count.saturating_sub(1));
        let cursor_track = self.cursor.track.min(track_count.saturating_sub(1));

        Some(SelectionRect {
            row_start: anchor_row.min(cursor_row),
            row_end: anchor_row.max(cursor_row),
            track_start: anchor_track.min(cursor_track),
            track_end: anchor_track.max(cursor_track),
        })
    }

    fn copy_selection(&mut self, selection: SelectionRect) {
        let Some(pattern) = self.song.pattern(self.pattern_index) else {
            return;
        };
        let cells = (selection.row_start..=selection.row_end)
            .map(|row| {
                (selection.track_start..=selection.track_end)
                    .map(|track| pattern.cell(row, track).cloned().unwrap_or_default())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        self.clipboard = Some(Clipboard::Region(ClipboardRegion { cells }));
    }

    fn clear_selection_region(&mut self) {
        if let Some(selection) = self.selection_rect() {
            self.clear_region(selection);
            self.selection_anchor = None;
        }
    }

    fn clear_region(&mut self, selection: SelectionRect) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, _| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            for row in selection.row_start..=selection.row_end {
                for track in selection.track_start..=selection.track_end {
                    let _ = pattern.clear_cell(row, track);
                }
            }
        });
    }

    fn insert_current_row(&mut self) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let _ = song.insert_pattern_row(pattern_index, cursor.row);
        });
    }

    fn delete_current_row(&mut self) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let _ = song.delete_pattern_row(pattern_index, cursor.row);
        });
        self.clamp_cursor();
    }

    fn create_track(&mut self) {
        let before_count = self.song.tracks.len();
        self.mutate_song(|song, _| {
            song.create_track();
        });

        if self.song.tracks.len() > before_count {
            self.cursor.track = self.song.tracks.len().saturating_sub(1);
            self.cursor.field = CellField::Note;
            self.cursor.digit = 0;
        }
    }

    fn request_delete_current_track(&mut self) {
        self.request_delete_track(self.cursor.track);
    }

    fn request_delete_track(&mut self, track_index: usize) {
        if self.song.tracks.len() <= 1 {
            self.notify_warning("Cannot delete the last track");
            return;
        }

        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };
        self.dialog = Some(Dialog::DeleteTrack {
            track_index,
            message: format!("Delete track {:02} {}?", track_index + 1, track.name),
        });
        self.mode = AppMode::Dialog;
        self.notify_warning("Confirm track delete");
    }

    fn delete_track(&mut self, track: usize) {
        let before_count = self.song.tracks.len();
        self.mutate_song(|song, _| {
            let _ = song.delete_track(track);
        });

        if self.song.tracks.len() < before_count {
            self.clamp_cursor();
            self.cursor.digit = 0;
            self.notify_success("Track deleted");
        }
    }

    fn duplicate_track(&mut self, track_index: usize) {
        let before_count = self.song.tracks.len();
        self.mutate_song(|song, _| {
            let _ = song.duplicate_track(track_index);
        });

        if self.song.tracks.len() > before_count {
            self.cursor.track = self.song.tracks.len().saturating_sub(1);
            self.cursor.field = CellField::Note;
            self.cursor.digit = 0;
        }
    }

    fn move_track(&mut self, from: usize, to: usize) {
        let before = self.song.clone();
        self.mutate_song(|song, _| {
            let _ = song.move_track(from, to);
        });

        if self.song != before {
            self.cursor.track = to.min(self.song.tracks.len().saturating_sub(1));
            self.cursor.field = CellField::Note;
            self.cursor.digit = 0;
            self.notify_success("Track moved");
        }
    }

    fn move_current_track_left(&mut self) {
        if self.cursor.track == 0 {
            self.notify_warning("Track already at first position");
            return;
        }

        self.move_track(self.cursor.track, self.cursor.track - 1);
    }

    fn move_current_track_right(&mut self) {
        let next_track = self.cursor.track.saturating_add(1);
        if next_track >= self.song.tracks.len() {
            self.notify_warning("Track already at last position");
            return;
        }

        self.move_track(self.cursor.track, next_track);
    }

    fn toggle_current_mute(&mut self) {
        self.toggle_track_mute(self.cursor.track);
    }

    fn toggle_current_solo(&mut self) {
        self.toggle_track_solo(self.cursor.track);
    }

    fn toggle_track_mute(&mut self, track_index: usize) {
        if track_index >= self.song.tracks.len() {
            self.notify_warning("Track out of range");
            return;
        }

        self.mutate_song(|song, _| {
            let _ = song.toggle_mute(track_index);
        });
    }

    fn toggle_track_solo(&mut self, track_index: usize) {
        if track_index >= self.song.tracks.len() {
            self.notify_warning("Track out of range");
            return;
        }

        self.mutate_song(|song, _| {
            let _ = song.toggle_solo(track_index);
        });
    }

    fn set_track_midi_channel(&mut self, track_index: usize, midi_channel: u8) {
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.set_track_midi_channel(track_index, midi_channel) {
            self.notify_warning(format!("Track channel failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success(format!("Track channel set to {midi_channel}"));
    }

    fn rename_track(&mut self, track_index: usize, name: String) {
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.rename_track(track_index, name) {
            self.notify_warning(format!("Track rename failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success("Track renamed");
    }

    fn start_track_rename_command(&mut self) {
        self.command_buffer = format!("track rename {} ", self.cursor.track + 1);
        self.mode = AppMode::Command;
        self.notify_info("Rename current track");
    }

    fn set_bpm(&mut self, bpm: u16) {
        self.mutate_song(|song, _| {
            song.transport.bpm = bpm;
        });
    }

    fn adjust_bpm(&mut self, delta: i16) {
        let bpm = (i32::from(self.song.transport.bpm) + i32::from(delta))
            .clamp(i32::from(MIN_BPM), i32::from(MAX_BPM)) as u16;
        self.set_bpm(bpm);
        self.notify_info(format!("BPM {bpm}"));
    }

    fn set_lpb(&mut self, lpb: u8) {
        self.mutate_song(|song, _| {
            song.transport.lines_per_beat = lpb;
        });
    }

    fn adjust_lpb(&mut self, delta: i8) {
        let lpb = (i16::from(self.song.transport.lines_per_beat) + i16::from(delta))
            .clamp(i16::from(MIN_LPB), i16::from(MAX_LPB)) as u8;
        self.set_lpb(lpb);
        self.notify_info(format!("LPB {lpb}"));
    }

    fn create_pattern(&mut self) {
        let before_count = self.song.patterns.len();
        self.mutate_song(|song, _| {
            song.create_pattern(64);
        });
        if self.song.patterns.len() > before_count {
            self.pattern_index = self.song.patterns.len().saturating_sub(1);
            self.cursor.row = 0;
            self.row_offset = 0;
        }
    }

    fn duplicate_current_pattern(&mut self) {
        let pattern_index = self.pattern_index;
        let before_count = self.song.patterns.len();
        self.mutate_song(|song, _| {
            let _ = song.duplicate_pattern(pattern_index);
        });
        if self.song.patterns.len() > before_count {
            self.pattern_index = self.song.patterns.len().saturating_sub(1);
            self.cursor.row = 0;
            self.row_offset = 0;
        }
    }

    fn request_delete_current_pattern(&mut self) {
        if self.song.patterns.len() <= 1 {
            self.notify_warning("Cannot delete the last pattern");
            return;
        }

        let pattern_index = self
            .pattern_index
            .min(self.song.patterns.len().saturating_sub(1));
        let Some(pattern) = self.song.patterns.get(pattern_index) else {
            self.notify_warning("Pattern out of range");
            return;
        };
        self.dialog = Some(Dialog::DeletePattern {
            pattern_index,
            message: format!("Delete pattern {:02} {}?", pattern_index + 1, pattern.name),
        });
        self.mode = AppMode::Dialog;
        self.notify_warning("Confirm pattern delete");
    }

    fn delete_pattern(&mut self, pattern_index: usize) {
        let before_count = self.song.patterns.len();
        self.mutate_song(|song, _| {
            let _ = song.delete_pattern(pattern_index);
        });
        if self.song.patterns.len() < before_count {
            self.pattern_index = self
                .pattern_index
                .min(self.song.patterns.len().saturating_sub(1));
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.row_offset = 0;
            self.notify_success("Pattern deleted");
        }
    }

    fn resize_current_pattern(&mut self, row_count: usize) {
        let pattern_index = self.pattern_index;
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.resize_pattern(pattern_index, row_count) {
            self.notify_warning(format!("Pattern length failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.clamp_cursor();
        self.keep_cursor_visible(1);
        self.notify_success(format!("Pattern length set to {row_count}"));
    }

    fn rename_current_pattern(&mut self, name: String) {
        let pattern_index = self.pattern_index;
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.rename_pattern(pattern_index, name) {
            self.notify_warning(format!("Pattern rename failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success("Pattern renamed");
    }

    fn select_pattern(&mut self, pattern_index: usize) {
        if pattern_index < self.song.patterns.len() {
            self.pattern_index = pattern_index;
            self.clamp_cursor();
            self.row_offset = 0;
        }
    }

    fn selected_sequence_position(&mut self) -> Option<usize> {
        if self.song.sequence.is_empty() {
            self.notify_warning("Sequence is empty");
            return None;
        }

        self.clamp_sequence_cursor();
        Some(self.sequence_cursor)
    }

    fn previous_sequence_position(&mut self) {
        self.sequence_cursor = self.sequence_cursor.saturating_sub(1);
        self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
    }

    fn next_sequence_position(&mut self) {
        if self.song.sequence.is_empty() {
            self.notify_warning("Sequence is empty");
            return;
        }

        self.sequence_cursor = self
            .sequence_cursor
            .saturating_add(1)
            .min(self.song.sequence.len().saturating_sub(1));
        self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
    }

    fn add_sequence_pattern(&mut self, pattern_index: usize) {
        let Some(pattern_id) = self.song.pattern(pattern_index).map(|pattern| pattern.id) else {
            self.notify_warning("Pattern out of range");
            return;
        };
        let before_len = self.song.sequence.len();
        self.mutate_song(|song, _| {
            let _ = song.push_sequence_pattern(pattern_id);
        });
        if self.song.sequence.len() > before_len {
            self.sequence_cursor = self.song.sequence.len().saturating_sub(1);
        }
        self.notify_success(format!("Sequence added pattern {:02}", pattern_index + 1));
    }

    fn remove_sequence_position(&mut self, position: usize) {
        let before_len = self.song.sequence.len();
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.remove_sequence_position(position) {
            self.notify_warning(format!("Sequence remove failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        if self.song.sequence.len() < before_len {
            self.sequence_cursor = position.min(self.song.sequence.len().saturating_sub(1));
        }
        self.clamp_sequence_cursor();
        self.notify_success(format!("Sequence removed position {position:02}"));
    }

    fn duplicate_sequence_position(&mut self, position: usize) {
        let before_len = self.song.sequence.len();
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.duplicate_sequence_position(position) {
            self.notify_warning(format!("Sequence duplicate failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        if self.song.sequence.len() > before_len {
            self.sequence_cursor = position.saturating_add(1);
            self.clamp_sequence_cursor();
        }
        self.notify_success(format!("Sequence duplicated position {position:02}"));
    }

    fn duplicate_selected_sequence_position(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.duplicate_sequence_position(position);
        }
    }

    fn remove_selected_sequence_position(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.remove_sequence_position(position);
        }
    }

    fn set_sequence_pattern(&mut self, position: usize, pattern_index: usize) {
        let Some(pattern_id) = self.song.pattern(pattern_index).map(|pattern| pattern.id) else {
            self.notify_warning("Pattern out of range");
            return;
        };
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.set_sequence_pattern(position, pattern_id) {
            self.notify_warning(format!("Sequence set failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success(format!(
            "Sequence position {position:02} set to pattern {:02}",
            pattern_index + 1
        ));
    }

    fn set_selected_sequence_to_current_pattern(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.set_sequence_pattern(position, self.pattern_index);
        }
    }

    fn move_sequence_position(&mut self, from: usize, to: usize) {
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.move_sequence_position(from, to) {
            self.notify_warning(format!("Sequence move failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.sequence_cursor = to;
        self.notify_success(format!("Sequence moved position {from:02} to {to:02}"));
    }

    fn move_selected_sequence_position_up(&mut self) {
        let Some(position) = self.selected_sequence_position() else {
            return;
        };
        if position == 0 {
            self.notify_warning("Sequence already at first position");
            return;
        }
        self.move_sequence_position(position, position - 1);
    }

    fn move_selected_sequence_position_down(&mut self) {
        let Some(position) = self.selected_sequence_position() else {
            return;
        };
        let next_position = position.saturating_add(1);
        if next_position >= self.song.sequence.len() {
            self.notify_warning("Sequence already at last position");
            return;
        }
        self.move_sequence_position(position, next_position);
    }

    fn execute_command(&mut self) {
        let command = self.command_buffer.trim().to_string();
        self.command_buffer.clear();
        self.mode = AppMode::Normal;

        let mut parts = command.split_whitespace();
        let Some(name) = parts.next() else {
            return;
        };

        match name {
            "h" | "help" => {
                self.mode = AppMode::Help;
            }
            "q" | "quit" => {
                self.request_quit(false);
            }
            "q!" | "quit!" => {
                self.force_quit();
            }
            "w" | "write" | "save" => {
                let path = parts.collect::<Vec<_>>().join(" ");
                let result = if path.is_empty() {
                    self.save()
                } else {
                    self.save_as(PathBuf::from(path))
                };
                if let Err(error) = result {
                    tracing::error!(?error, "failed to save project");
                    self.notify_error(format!("Save failed: {error}"));
                }
            }
            "saveas" | "writeas" => {
                let path = parts.collect::<Vec<_>>().join(" ");
                if !path.is_empty() {
                    if let Err(error) = self.save_as(PathBuf::from(path)) {
                        tracing::error!(?error, "failed to save project");
                        self.notify_error(format!("Save failed: {error}"));
                    }
                } else {
                    self.notify_warning("Usage: :saveas PATH");
                }
            }
            "wq" => {
                if let Err(error) = self.save() {
                    tracing::error!(?error, "failed to save project");
                    self.notify_error(format!("Save failed: {error}"));
                    return;
                }
                self.stop_playback();
                self.should_quit = true;
            }
            "bpm" => {
                if let Some(value) = parts.next().and_then(|value| value.parse::<u16>().ok()) {
                    self.set_bpm(value);
                    self.notify_success(format!("BPM set to {value}"));
                } else {
                    self.notify_warning("Usage: :bpm 140");
                }
            }
            "lpb" => {
                if let Some(value) = parts.next().and_then(|value| value.parse::<u8>().ok()) {
                    self.set_lpb(value);
                    self.notify_success(format!("LPB set to {value}"));
                } else {
                    self.notify_warning("Usage: :lpb 4");
                }
            }
            "loop" => match parts.next() {
                Some("on") => {
                    self.loop_pattern = true;
                    self.notify_info("Pattern loop ON");
                }
                Some("off") => {
                    self.loop_pattern = false;
                    self.notify_info("Pattern loop OFF");
                }
                Some("toggle") | None => self.toggle_loop(),
                Some(_) => self.notify_warning("Usage: :loop [on|off|toggle]"),
            },
            "midi" => match parts.next() {
                Some("outputs") | Some("settings") | Some("ports") => self.open_midi_settings(),
                Some("connect") => {
                    if let Some(port_index) =
                        parts.next().and_then(|value| value.parse::<usize>().ok())
                    {
                        self.connect_midi(port_index);
                    } else {
                        self.notify_warning("Usage: :midi connect PORT_INDEX");
                    }
                }
                Some("disconnect") => self.disconnect_midi(),
                Some("panic") => self.panic_midi(),
                None | Some(_) => {
                    self.notify_warning("Usage: :midi outputs|connect|disconnect|panic")
                }
            },
            "play" => match parts.next() {
                Some("sequence") | Some("seq") => {
                    let start_sequence_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(0);
                    self.start_sequence_playback_at(start_sequence_index);
                }
                Some("pattern") | Some("pat") | None => self.start_playback(),
                Some(_) => self.notify_warning("Usage: :play [pattern|sequence [position]]"),
            },
            "stop" => self.stop_playback(),
            "track" => match parts.next() {
                Some("new") => self.create_track(),
                Some("duplicate") | Some("dup") => {
                    let track_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.cursor.track, |value| value.saturating_sub(1));
                    self.duplicate_track(track_index);
                }
                Some("delete") | Some("del") => {
                    let track_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.cursor.track, |value| value.saturating_sub(1));
                    self.request_delete_track(track_index);
                }
                Some("move") | Some("mv") => {
                    let from = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.cursor.track, |value| value.saturating_sub(1));
                    let to = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map(|value| value.saturating_sub(1));
                    if let Some(to) = to {
                        self.move_track(from, to);
                    } else {
                        self.notify_warning("Usage: :track move FROM TO");
                    }
                }
                Some("mute") => {
                    let track_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.cursor.track, |value| value.saturating_sub(1));
                    self.toggle_track_mute(track_index);
                }
                Some("solo") => {
                    let track_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.cursor.track, |value| value.saturating_sub(1));
                    self.toggle_track_solo(track_index);
                }
                Some("rename") => {
                    let values = parts.collect::<Vec<_>>();
                    if let Some((track_index, name)) =
                        parse_optional_numbered_name(&values, self.cursor.track)
                    {
                        self.rename_track(track_index, name);
                    }
                }
                Some("channel") | Some("ch") => {
                    let first = parts.next().and_then(|value| value.parse::<u8>().ok());
                    let second = parts.next().and_then(|value| value.parse::<u8>().ok());
                    match (first, second) {
                        (Some(channel), None) => {
                            self.set_track_midi_channel(self.cursor.track, channel);
                        }
                        (Some(track_number), Some(channel)) => {
                            self.set_track_midi_channel(
                                usize::from(track_number.saturating_sub(1)),
                                channel,
                            );
                        }
                        _ => {}
                    }
                }
                None | Some(_) => self.notify_warning(
                    "Usage: :track new|duplicate|delete|move|mute|solo|rename|channel",
                ),
            },
            "pattern" => match parts.next() {
                Some("new") => self.create_pattern(),
                Some("duplicate") | Some("dup") => self.duplicate_current_pattern(),
                Some("delete") | Some("del") => self.request_delete_current_pattern(),
                Some("length") | Some("len") => {
                    if let Some(row_count) =
                        parts.next().and_then(|value| value.parse::<usize>().ok())
                    {
                        self.resize_current_pattern(row_count);
                    }
                }
                Some("rename") => {
                    let name = parts.collect::<Vec<_>>().join(" ");
                    self.rename_current_pattern(name);
                }
                Some("next") => self.select_pattern(self.pattern_index.saturating_add(1)),
                Some("prev") => self.select_pattern(self.pattern_index.saturating_sub(1)),
                Some(value) => {
                    if let Ok(pattern_number) = value.parse::<usize>() {
                        self.select_pattern(pattern_number.saturating_sub(1));
                    }
                }
                None => {}
            },
            "sequence" | "seq" => match parts.next() {
                Some("add") => {
                    let pattern_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.pattern_index, |value| value.saturating_sub(1));
                    self.add_sequence_pattern(pattern_index);
                }
                Some("remove") | Some("rm") => {
                    if let Some(position) =
                        parts.next().and_then(|value| value.parse::<usize>().ok())
                    {
                        self.remove_sequence_position(position);
                    }
                }
                Some("duplicate") | Some("dup") => {
                    if let Some(position) =
                        parts.next().and_then(|value| value.parse::<usize>().ok())
                    {
                        self.duplicate_sequence_position(position);
                    }
                }
                Some("set") => {
                    let position = parts.next().and_then(|value| value.parse::<usize>().ok());
                    let pattern_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map(|value| value.saturating_sub(1));
                    if let (Some(position), Some(pattern_index)) = (position, pattern_index) {
                        self.set_sequence_pattern(position, pattern_index);
                    }
                }
                Some("move") | Some("mv") => {
                    let from = parts.next().and_then(|value| value.parse::<usize>().ok());
                    let to = parts.next().and_then(|value| value.parse::<usize>().ok());
                    if let (Some(from), Some(to)) = (from, to) {
                        self.move_sequence_position(from, to);
                    }
                }
                None | Some(_) => {
                    self.notify_warning("Usage: :sequence add|remove|duplicate|set|move")
                }
            },
            _ => self.notify_warning(format!("Unknown command: {name}")),
        }
    }

    fn request_quit(&mut self, force: bool) {
        if force || !self.dirty {
            self.force_quit();
        } else {
            self.stop_playback();
            self.mode = AppMode::Dialog;
            self.dialog = Some(Dialog::QuitDirty);
            self.notify_warning("Unsaved changes");
        }
    }

    fn force_quit(&mut self) {
        self.stop_playback();
        self.dialog = None;
        self.should_quit = true;
    }

    fn cancel_dialog(&mut self) {
        self.dialog = None;
        self.mode = AppMode::Normal;
        self.notify_info("Cancelled");
    }

    fn toggle_playback(&mut self) {
        if self.is_playing {
            self.stop_playback();
        } else {
            self.start_playback();
        }
    }

    fn toggle_loop(&mut self) {
        self.loop_pattern = !self.loop_pattern;
        let state = if self.loop_pattern { "ON" } else { "OFF" };
        self.notify_info(format!("Pattern loop {state}"));
    }

    fn start_playback(&mut self) {
        if self.song.pattern(self.pattern_index).is_none() {
            self.notify_warning("No pattern to play");
            return;
        }

        self.is_playing = true;
        self.playhead_row = Some(0);
        self.sequence_position = None;
        self.playback.start_pattern_from(
            self.song.clone(),
            self.pattern_index,
            0,
            self.loop_pattern,
        );
        self.notify_info("Playing pattern from start");
    }

    fn start_playback_from_cursor(&mut self) {
        if self.song.pattern(self.pattern_index).is_none() {
            self.notify_warning("No pattern to play");
            return;
        }

        self.is_playing = true;
        self.playhead_row = Some(self.cursor.row);
        self.sequence_position = None;
        self.playback.start_pattern_from(
            self.song.clone(),
            self.pattern_index,
            self.cursor.row,
            self.loop_pattern,
        );
        self.notify_info(format!("Playing pattern from row {:02}", self.cursor.row));
    }

    fn start_sequence_playback_at(&mut self, start_sequence_index: usize) {
        if self.song.sequence.is_empty() {
            self.notify_warning("Sequence is empty");
            return;
        }

        if start_sequence_index >= self.song.sequence.len() {
            self.notify_warning("Sequence position out of range");
            return;
        }

        if let Some(first_pattern_id) = self.song.sequence.get(start_sequence_index) {
            if let Some(pattern_index) = self
                .song
                .patterns
                .iter()
                .position(|pattern| pattern.id == *first_pattern_id)
            {
                self.pattern_index = pattern_index;
            }
        }

        self.is_playing = true;
        self.playhead_row = Some(0);
        self.sequence_position = Some(start_sequence_index);
        self.playback
            .start_sequence(self.song.clone(), start_sequence_index);
        self.notify_info(format!("Playing sequence from {start_sequence_index}"));
    }

    fn start_sequence_playback_from_selected_position(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.start_sequence_playback_at(position);
        }
    }

    fn stop_playback(&mut self) {
        self.playback.stop();
        self.is_playing = false;
        self.playhead_row = None;
        self.sequence_position = None;
        self.notify_info("Playback stopped");
    }

    fn connect_midi(&mut self, port_index: usize) {
        self.midi_status = format!("MIDI Connecting {port_index}");
        self.playback.connect_midi(port_index);
        self.notify_info(format!("Connecting MIDI output {port_index}"));
    }

    fn open_midi_settings(&mut self) {
        self.refresh_midi_ports();
        self.mode = AppMode::MidiSettings;
    }

    fn refresh_midi_ports(&mut self) {
        match list_output_ports() {
            Ok(ports) => {
                self.midi_ports = ports;
                self.midi_port_cursor = self
                    .midi_port_cursor
                    .min(self.midi_ports.len().saturating_sub(1));
                if self.midi_ports.is_empty() {
                    self.midi_status = "MIDI No Outputs".to_string();
                    self.notify_warning("No MIDI output ports found");
                } else {
                    self.notify_info(format!("Found {} MIDI output(s)", self.midi_ports.len()));
                }
            }
            Err(error) => {
                self.midi_ports.clear();
                self.midi_port_cursor = 0;
                self.midi_status = format!("MIDI Error: {error}");
                self.notify_error(format!("MIDI output list failed: {error}"));
            }
        }
    }

    fn next_midi_port(&mut self) {
        self.midi_port_cursor = self
            .midi_port_cursor
            .saturating_add(1)
            .min(self.midi_ports.len().saturating_sub(1));
    }

    fn previous_midi_port(&mut self) {
        self.midi_port_cursor = self.midi_port_cursor.saturating_sub(1);
    }

    fn connect_selected_midi_port(&mut self) {
        if let Some(port) = self.midi_ports.get(self.midi_port_cursor) {
            self.connect_midi(port.index);
        }
    }

    fn connect_default_midi_output(&mut self, output_name: &str) {
        if output_name.trim().is_empty() {
            return;
        }

        match list_output_ports() {
            Ok(ports) => {
                self.midi_ports = ports;
                if let Some((position, port)) =
                    resolve_midi_output_port(&self.midi_ports, output_name)
                {
                    self.midi_port_cursor = position;
                    self.midi_status = format!("MIDI Connecting {} ({})", port.index, port.name);
                    self.playback.connect_midi(port.index);
                } else {
                    self.midi_status = format!("MIDI Output Not Found ({output_name})");
                }
            }
            Err(error) => {
                self.midi_status = format!("MIDI Error: {error}");
            }
        }
    }

    fn disconnect_midi(&mut self) {
        self.playback.disconnect_midi();
        self.notify_info("Disconnecting MIDI output");
    }

    fn panic_midi(&mut self) {
        self.playback.panic_all_notes_off();
        self.is_playing = false;
        self.playhead_row = None;
        self.sequence_position = None;
        self.notify_warning("MIDI panic sent");
    }

    fn drain_playback_updates(&mut self) {
        while let Some(update) = self.playback.try_recv() {
            match update {
                PlaybackUpdate::Position(position) => {
                    self.is_playing = true;
                    self.pattern_index = position.pattern_index;
                    self.sequence_position = position.sequence_index;
                    self.playhead_row = Some(position.position.row);
                }
                PlaybackUpdate::Stopped => {
                    self.is_playing = false;
                    self.playhead_row = None;
                    self.sequence_position = None;
                    self.notify_info("Playback stopped");
                }
                PlaybackUpdate::MidiConnected { port_index } => {
                    self.midi_status = format!("MIDI Connected {port_index}");
                    self.notify_success(format!("MIDI output connected: {port_index}"));
                }
                PlaybackUpdate::MidiDisconnected => {
                    self.midi_status = "MIDI Disconnected".to_string();
                    self.notify_info("MIDI output disconnected");
                }
                PlaybackUpdate::MidiError(error) => {
                    self.midi_status = format!("MIDI Error: {error}");
                    self.is_playing = false;
                    self.playhead_row = None;
                    self.sequence_position = None;
                    self.notify_error(format!("MIDI error: {error}"));
                }
                PlaybackUpdate::MidiLogError(error) => {
                    self.midi_status = format!("MIDI Log Error: {error}");
                    self.notify_error(format!("MIDI log error: {error}"));
                }
            }
        }
    }

    fn mutate_song(&mut self, mutate: impl FnOnce(&mut Song, Cursor)) {
        let before = self.song.clone();
        mutate(&mut self.song, self.cursor);
        if self.song != before {
            self.undo_stack.push(before);
            if self.undo_stack.len() > UNDO_LIMIT {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
            self.refresh_dirty();
            self.clamp_sequence_cursor();
        }
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            let current = std::mem::replace(&mut self.song, previous);
            self.redo_stack.push(current);
            self.refresh_dirty();
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.notify_info("Undo");
        } else {
            self.notify_warning("Nothing to undo");
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = std::mem::replace(&mut self.song, next);
            self.undo_stack.push(current);
            self.refresh_dirty();
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.notify_info("Redo");
        } else {
            self.notify_warning("Nothing to redo");
        }
    }

    fn advance_after_edit(&mut self) {
        self.cursor.row = self
            .cursor
            .row
            .saturating_add(self.edit_step)
            .min(self.current_row_count().saturating_sub(1));
    }

    fn increment_octave(&mut self) {
        self.octave = self.octave.saturating_add(1).min(9);
    }

    fn decrement_octave(&mut self) {
        self.octave = self.octave.saturating_sub(1);
    }

    fn refresh_dirty(&mut self) {
        self.dirty = self.song != self.clean_song;
    }

    fn save(&mut self) -> Result<()> {
        let path = self
            .project_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("untitled.salieri"));
        self.save_as(path)
    }

    fn save_as(&mut self, path: PathBuf) -> Result<()> {
        save_project(&path, &self.song)?;
        self.project_path = Some(path);
        self.clean_song = self.song.clone();
        self.refresh_dirty();
        self.notify_success("Project saved");
        Ok(())
    }

    fn clamp_cursor(&mut self) {
        self.pattern_index = self
            .pattern_index
            .min(self.song.patterns.len().saturating_sub(1));
        self.cursor
            .clamp(self.current_row_count(), self.song.tracks.len());
    }

    fn clamp_sequence_cursor(&mut self) {
        if self.song.sequence.is_empty() {
            self.sequence_cursor = 0;
        } else {
            self.sequence_cursor = self
                .sequence_cursor
                .min(self.song.sequence.len().saturating_sub(1));
        }
    }

    fn tui_sequence_position(&self) -> Option<usize> {
        self.sequence_position.or_else(|| {
            (!self.song.sequence.is_empty()).then_some(
                self.sequence_cursor
                    .min(self.song.sequence.len().saturating_sub(1)),
            )
        })
    }

    fn keep_cursor_visible(&mut self, visible_rows: usize) {
        self.keep_row_visible(self.cursor.row, visible_rows);
    }

    fn keep_active_row_visible(&mut self, visible_rows: usize) {
        let row = if self.is_playing && self.follow_playhead {
            self.playhead_row.unwrap_or(self.cursor.row)
        } else {
            self.cursor.row
        };
        self.keep_row_visible(row, visible_rows);
    }

    fn keep_row_visible(&mut self, row: usize, visible_rows: usize) {
        let visible_rows = visible_rows.max(1);
        if row < self.row_offset {
            self.row_offset = row;
        } else if row >= self.row_offset.saturating_add(visible_rows) {
            self.row_offset = row.saturating_sub(visible_rows - 1);
        }

        let max_offset = self.current_row_count().saturating_sub(visible_rows);
        self.row_offset = self.row_offset.min(max_offset);
    }

    fn current_row_count(&self) -> usize {
        self.song
            .pattern(self.pattern_index)
            .map_or(0, |pattern| pattern.row_count())
    }

    fn command_line(&self) -> Option<&str> {
        if self.mode == AppMode::Command {
            Some(self.command_buffer.as_str())
        } else {
            None
        }
    }

    fn quit_confirmation(&self) -> bool {
        self.mode == AppMode::Dialog && matches!(self.dialog, Some(Dialog::QuitDirty))
    }

    fn delete_confirmation_message(&self) -> Option<&str> {
        if self.mode != AppMode::Dialog {
            return None;
        }

        match &self.dialog {
            Some(Dialog::DeleteTrack { message, .. }) => Some(message.as_str()),
            Some(Dialog::DeletePattern { message, .. }) => Some(message.as_str()),
            Some(Dialog::QuitDirty) | None => None,
        }
    }

    fn notify(&mut self, kind: NotificationKind, message: impl Into<String>) {
        self.notification = Some(Notification {
            kind,
            message: message.into(),
            expires_at: Instant::now() + NOTIFICATION_TTL,
        });
    }

    fn notify_info(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Info, message);
    }

    fn notify_success(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Success, message);
    }

    fn notify_warning(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Warning, message);
    }

    fn notify_error(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Error, message);
    }

    fn expire_notification(&mut self) {
        if self
            .notification
            .as_ref()
            .is_some_and(|notification| Instant::now() >= notification.expires_at)
        {
            self.notification = None;
        }
    }

    fn tui_notification(&self) -> Option<NotificationView<'_>> {
        self.notification
            .as_ref()
            .map(|notification| NotificationView {
                kind: notification.kind,
                message: notification.message.as_str(),
            })
    }

    fn tui_midi_ports(&self) -> Vec<MidiPortView<'_>> {
        self.midi_ports
            .iter()
            .map(|port| MidiPortView {
                index: port.index,
                name: port.name.as_str(),
            })
            .collect()
    }

    fn tui_midi_settings<'a>(
        &'a self,
        ports: &'a [MidiPortView<'a>],
    ) -> Option<MidiSettingsState<'a>> {
        (self.mode == AppMode::MidiSettings).then_some(MidiSettingsState {
            ports,
            selected_port: self.midi_port_cursor.min(ports.len().saturating_sub(1)),
            status: self.midi_status.as_str(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    Normal,
    Edit,
    Command,
    Help,
    Dialog,
    MidiSettings,
}

impl AppMode {
    const fn label(self) -> &'static str {
        match self {
            AppMode::Normal => "NORMAL",
            AppMode::Edit => "EDIT",
            AppMode::Command => "COMMAND",
            AppMode::Help => "HELP",
            AppMode::Dialog => "DIALOG",
            AppMode::MidiSettings => "MIDI",
        }
    }
}

fn parse_optional_numbered_name(values: &[&str], default_index: usize) -> Option<(usize, String)> {
    let first = values.first()?;
    if let Ok(number) = first.parse::<usize>() {
        let name = values.get(1..)?.join(" ");
        Some((number.saturating_sub(1), name))
    } else {
        Some((default_index, values.join(" ")))
    }
}

fn keyboard_note(key: char, octave: u8) -> Option<u8> {
    let (semitone, octave_offset) = match key.to_ascii_lowercase() {
        'z' => (0, 0),
        's' => (1, 0),
        'x' => (2, 0),
        'd' => (3, 0),
        'c' => (4, 0),
        'v' => (5, 0),
        'g' => (6, 0),
        'b' => (7, 0),
        'h' => (8, 0),
        'n' => (9, 0),
        'j' => (10, 0),
        'm' => (11, 0),
        'q' => (0, 1),
        '2' => (1, 1),
        'w' => (2, 1),
        '3' => (3, 1),
        'e' => (4, 1),
        'r' => (5, 1),
        '5' => (6, 1),
        't' => (7, 1),
        '6' => (8, 1),
        'y' => (9, 1),
        '7' => (10, 1),
        'u' => (11, 1),
        _ => return None,
    };

    let midi_octave = i16::from(octave) + octave_offset + 1;
    let pitch = midi_octave * 12 + semitone;
    u8::try_from(pitch).ok().filter(|pitch| *pitch <= 127)
}

fn find_midi_output_port<'a>(
    ports: &'a [MidiOutputPort],
    output_name: &str,
) -> Option<(usize, &'a MidiOutputPort)> {
    let needle = output_name.trim().to_lowercase();
    let normalized_needle = normalize_midi_port_name(output_name);
    if needle.is_empty() {
        return None;
    }

    ports
        .iter()
        .enumerate()
        .find(|(_, port)| port.name.eq_ignore_ascii_case(output_name.trim()))
        .or_else(|| {
            ports
                .iter()
                .enumerate()
                .find(|(_, port)| port.name.to_lowercase().contains(&needle))
        })
        .or_else(|| {
            ports.iter().enumerate().find(|(_, port)| {
                let normalized_name = normalize_midi_port_name(&port.name);
                normalized_name == normalized_needle
                    || normalized_name.contains(&normalized_needle)
                    || normalized_needle.contains(&normalized_name)
            })
        })
}

fn resolve_midi_output_port<'a>(
    ports: &'a [MidiOutputPort],
    output_name_or_index: &str,
) -> Option<(usize, &'a MidiOutputPort)> {
    let value = output_name_or_index.trim();
    value
        .parse::<usize>()
        .ok()
        .and_then(|index| {
            ports
                .iter()
                .enumerate()
                .find(|(_, port)| port.index == index)
        })
        .or_else(|| find_midi_output_port(ports, value))
}

fn normalize_midi_port_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_help_version_and_midi_listing() {
        assert_eq!(
            CliArgs::parse(["--help".to_string()]),
            CliArgs {
                command: CliCommand::Help,
                project_path: None,
                config_path: None,
                log_level: None,
                midi_log_path: None,
                midi_test: MidiTestArgs::default(),
            }
        );
        assert_eq!(
            CliArgs::parse(["--version".to_string()]).command,
            CliCommand::Version
        );
        assert_eq!(
            CliArgs::parse(["--list-midi-outputs".to_string()]).command,
            CliCommand::ListMidiOutputs
        );
    }

    #[test]
    fn cli_parses_optional_project_path() {
        assert_eq!(
            CliArgs::parse(["song.salieri".to_string()]),
            CliArgs {
                command: CliCommand::Run,
                project_path: Some(PathBuf::from("song.salieri")),
                config_path: None,
                log_level: None,
                midi_log_path: None,
                midi_test: MidiTestArgs::default(),
            }
        );
    }

    #[test]
    fn cli_parses_config_and_log_level_options() {
        assert_eq!(
            CliArgs::parse([
                "--config".to_string(),
                "custom.toml".to_string(),
                "--log-level=debug".to_string(),
                "--midi-log".to_string(),
                "midi.log".to_string(),
                "song.salieri".to_string()
            ]),
            CliArgs {
                command: CliCommand::Run,
                project_path: Some(PathBuf::from("song.salieri")),
                config_path: Some(PathBuf::from("custom.toml")),
                log_level: Some("debug".to_string()),
                midi_log_path: Some(PathBuf::from("midi.log")),
                midi_test: MidiTestArgs::default(),
            }
        );
    }

    #[test]
    fn cli_parses_midi_test_options() {
        assert_eq!(
            CliArgs::parse([
                "--midi-test-output=0".to_string(),
                "--midi-test-channel".to_string(),
                "2".to_string(),
                "--midi-test-note".to_string(),
                "64".to_string(),
                "--midi-test-duration-ms".to_string(),
                "1500".to_string(),
            ]),
            CliArgs {
                command: CliCommand::MidiTest,
                project_path: None,
                config_path: None,
                log_level: None,
                midi_log_path: None,
                midi_test: MidiTestArgs {
                    output: Some("0".to_string()),
                    channel: 2,
                    note: 64,
                    duration_ms: 1500,
                },
            }
        );
    }

    #[test]
    fn app_uses_keyboard_config_defaults() {
        let app = App::new(AppConfig {
            keyboard: config::KeyboardConfig {
                default_octave: 5,
                edit_step: 4,
                vim_navigation: false,
            },
            ui: config::UiConfig {
                show_line_numbers_hex: true,
                ..config::UiConfig::default()
            },
            ..AppConfig::default()
        });

        assert_eq!(app.octave, 5);
        assert_eq!(app.edit_step, 4);
        assert!(!app.vim_navigation);
        assert!(app.show_line_numbers_hex);
    }

    #[test]
    fn vim_navigation_can_be_disabled_by_config() {
        let mut app = App::new(AppConfig {
            keyboard: config::KeyboardConfig {
                vim_navigation: false,
                ..config::KeyboardConfig::default()
            },
            ..AppConfig::default()
        });
        app.cursor.row = 4;

        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

        assert_eq!(app.cursor.row, 3);
    }

    #[test]
    fn vim_navigation_jumps_to_pattern_bounds() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
        assert_eq!(app.cursor.row, 63);

        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert_eq!(app.cursor.row, 63);

        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert_eq!(app.cursor.row, 0);
    }

    #[test]
    fn playhead_follow_can_be_disabled_by_config() {
        let mut app = App::new(AppConfig {
            ui: config::UiConfig {
                follow_playhead: false,
                ..config::UiConfig::default()
            },
            ..AppConfig::default()
        });
        app.cursor.row = 0;
        app.is_playing = true;
        app.playhead_row = Some(20);

        app.keep_active_row_visible(10);

        assert_eq!(app.row_offset, 0);
    }

    #[test]
    fn finds_midi_output_by_exact_or_partial_name() {
        let ports = vec![
            MidiOutputPort {
                index: 0,
                name: "External Synth".to_string(),
            },
            MidiOutputPort {
                index: 1,
                name: "IAC Driver Bus 1".to_string(),
            },
        ];

        assert_eq!(
            find_midi_output_port(&ports, "IAC Driver")
                .map(|(position, port)| (position, port.index)),
            Some((1, 1))
        );
        assert_eq!(
            find_midi_output_port(&ports, "iac driver bus 1")
                .map(|(position, port)| (position, port.index)),
            Some((1, 1))
        );
        assert_eq!(
            find_midi_output_port(&ports, "IAC Driver (Bus 1)")
                .map(|(position, port)| (position, port.index)),
            Some((1, 1))
        );
        assert_eq!(
            resolve_midi_output_port(&ports, "1")
                .map(|(position, port)| (position, port.name.as_str())),
            Some((1, "IAC Driver Bus 1"))
        );
        assert!(find_midi_output_port(&ports, "Missing").is_none());
    }

    #[test]
    fn midi_settings_keys_select_connect_and_close() {
        let mut app = App {
            midi_ports: vec![
                MidiOutputPort {
                    index: 0,
                    name: "First".to_string(),
                },
                MidiOutputPort {
                    index: 2,
                    name: "Second".to_string(),
                },
            ],
            mode: AppMode::MidiSettings,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.midi_port_cursor, 1);

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.midi_status, "MIDI Connecting 2");

        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn f4_opens_midi_settings_without_mutating_song() {
        let mut app = App::default();
        let song = app.song.clone();

        app.handle_key(KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::MidiSettings);
        assert_eq!(app.song, song);
        assert!(!app.dirty);
    }

    #[test]
    fn f5_refreshes_midi_settings_without_mutating_song() {
        let mut app = App {
            mode: AppMode::MidiSettings,
            ..App::default()
        };
        let song = app.song.clone();

        app.handle_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::MidiSettings);
        assert_eq!(app.song, song);
        assert!(!app.dirty);
    }

    #[test]
    fn scrolls_down_to_keep_cursor_visible() {
        let mut app = App {
            cursor: Cursor {
                row: 20,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.keep_cursor_visible(10);

        assert_eq!(app.row_offset, 11);
    }

    #[test]
    fn scrolls_up_to_keep_cursor_visible() {
        let mut app = App {
            cursor: Cursor {
                row: 5,
                ..Cursor::new()
            },
            row_offset: 20,
            ..App::default()
        };

        app.keep_cursor_visible(10);

        assert_eq!(app.row_offset, 5);
    }

    #[test]
    fn scroll_offset_is_clamped_near_pattern_end() {
        let mut app = App {
            cursor: Cursor {
                row: 63,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.keep_cursor_visible(20);

        assert_eq!(app.row_offset, 44);
    }

    #[test]
    fn scrolls_to_keep_playhead_visible_while_playing() {
        let mut app = App {
            cursor: Cursor {
                row: 0,
                ..Cursor::new()
            },
            is_playing: true,
            playhead_row: Some(20),
            ..App::default()
        };

        app.keep_active_row_visible(10);

        assert_eq!(app.row_offset, 11);
    }

    #[test]
    fn tab_and_backtab_move_between_tracks() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.cursor.track, 1);

        app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert_eq!(app.cursor.track, 0);

        app.mode = AppMode::Edit;
        for _ in 0..10 {
            app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        }
        assert_eq!(app.cursor.track, 3);

        app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert_eq!(app.cursor.track, 2);
    }

    #[test]
    fn edit_mode_inserts_note_and_advances_cursor() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));

        let pattern = app.song.current_pattern().expect("pattern");
        let cell = pattern.cell(0, 0).expect("cell");
        assert_eq!(app.mode, AppMode::Edit);
        assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 60 }));
        assert_eq!(cell.velocity, Some(DEFAULT_NOTE_VELOCITY));
        assert_eq!(app.cursor.row, 1);
        assert!(app.dirty);
    }

    #[test]
    fn edit_mode_inserts_note_off_and_note_cut() {
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE));

        let pattern = app.song.current_pattern().expect("pattern");
        let off = pattern.cell(0, 0).expect("note off cell");
        let cut = pattern.cell(1, 0).expect("note cut cell");
        assert_eq!(off.note, Some(NoteEvent::NoteOff));
        assert_eq!(off.velocity, None);
        assert_eq!(cut.note, Some(NoteEvent::NoteCut));
        assert_eq!(cut.velocity, None);
        assert_eq!(app.cursor.row, 2);
    }

    #[test]
    fn velocity_entry_uses_two_hex_digits() {
        let mut app = App {
            mode: AppMode::Edit,
            cursor: Cursor {
                field: CellField::Velocity,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE));
        assert_eq!(app.cursor.row, 0);
        assert_eq!(app.cursor.digit, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));

        let pattern = app.song.current_pattern().expect("pattern");
        let cell = pattern.cell(0, 0).expect("cell");
        assert_eq!(cell.velocity, Some(0x4f));
        assert_eq!(app.cursor.row, 1);
        assert_eq!(app.cursor.digit, 0);
    }

    #[test]
    fn clipboard_copies_cuts_and_pastes_current_cell() {
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        app.cursor.row = 0;

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        app.cursor.row = 4;
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));

        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(4, 0)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 60 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(4, 0)
                .expect("cell"),
            &PatternCell::default()
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(4, 0)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 60 })
        );
    }

    #[test]
    fn selection_region_can_be_copied_cut_pasted_and_deleted() {
        let mut app = App::default();
        {
            let pattern = app.song.current_pattern_mut().expect("pattern");
            pattern
                .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
                .expect("set note");
            pattern
                .set_note(0, 1, NoteEvent::Note { pitch: 62 }, 0x7f)
                .expect("set note");
            pattern
                .set_note(1, 0, NoteEvent::Note { pitch: 64 }, 0x7f)
                .expect("set note");
            pattern
                .set_note(1, 1, NoteEvent::Note { pitch: 65 }, 0x7f)
                .expect("set note");
        }

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(
            app.selection_rect(),
            Some(SelectionRect {
                row_start: 0,
                row_end: 1,
                track_start: 0,
                track_end: 1,
            })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        app.cursor.row = 4;
        app.cursor.track = 2;
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(5, 3)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 65 })
        );

        app.cursor.row = 0;
        app.cursor.track = 0;
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
        assert_eq!(app.selection_rect(), None);
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(1, 1)
                .expect("cell"),
            &PatternCell::default()
        );

        app.cursor.row = 8;
        app.cursor.track = 0;
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(9, 1)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 65 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(8, 0)
                .expect("cell"),
            &PatternCell::default()
        );
    }

    #[test]
    fn insert_and_ctrl_delete_edit_pattern_rows() {
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.cursor.row = 0;

        app.handle_key(KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE));

        let pattern = app.song.current_pattern().expect("pattern");
        assert_eq!(pattern.row_count(), 65);
        assert_eq!(pattern.cell(0, 0), Some(&PatternCell::default()));
        assert_eq!(
            pattern.cell(1, 0).expect("cell").note,
            Some(NoteEvent::Note { pitch: 60 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::CONTROL));
        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 64);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 65);
    }

    #[test]
    fn undo_and_redo_restore_song_snapshots() {
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 0)
                .expect("cell"),
            &salieri_core::PatternCell::default()
        );
        assert!(!app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 0)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 60 })
        );
        assert!(app.dirty);
    }

    #[test]
    fn keyboard_note_maps_tracker_keys_to_midi_pitches() {
        assert_eq!(keyboard_note('z', 4), Some(60));
        assert_eq!(keyboard_note('s', 4), Some(61));
        assert_eq!(keyboard_note('q', 4), Some(72));
        assert_eq!(keyboard_note('u', 4), Some(83));
    }

    #[test]
    fn ctrl_s_saves_project_and_clears_dirty_state() {
        let path =
            std::env::temp_dir().join(format!("salieri-app-save-{}.salieri", std::process::id()));
        let mut app = App {
            mode: AppMode::Edit,
            project_path: Some(path.clone()),
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved, app.song);
        assert!(!app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Project saved")
        );
    }

    #[test]
    fn command_mode_sets_bpm_and_lpb() {
        let mut app = App::default();

        type_command(&mut app, "bpm 140");
        type_command(&mut app, "lpb 8");

        assert_eq!(app.song.transport.bpm, 140);
        assert_eq!(app.song.transport.lines_per_beat, 8);
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.lines_per_beat, 4);
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.bpm, 120);
        assert!(!app.dirty);
    }

    #[test]
    fn control_arrows_adjust_bpm_and_lpb() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.bpm, 121);
        assert!(app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("BPM 121")
        );

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.bpm, 120);
        assert!(!app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.lines_per_beat, 5);
        assert!(app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("LPB 5")
        );

        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.lines_per_beat, 4);
        assert!(!app.dirty);

        app.song.transport.bpm = MIN_BPM;
        app.song.transport.lines_per_beat = MAX_LPB;
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL));
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL));

        assert_eq!(app.song.transport.bpm, MIN_BPM);
        assert_eq!(app.song.transport.lines_per_beat, MAX_LPB);
    }

    #[test]
    fn command_mode_sets_pattern_loop() {
        let mut app = App::default();

        assert!(app.loop_pattern);
        type_command(&mut app, "loop off");
        assert!(!app.loop_pattern);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern loop OFF")
        );
        type_command(&mut app, "loop on");
        assert!(app.loop_pattern);
        type_command(&mut app, "loop");
        assert!(!app.loop_pattern);
    }

    #[test]
    fn command_mode_reports_unknown_commands() {
        let mut app = App::default();

        type_command(&mut app, "doesnotexist");

        let notification = app.notification.as_ref().expect("notification");
        assert_eq!(notification.kind, NotificationKind::Warning);
        assert_eq!(notification.message, "Unknown command: doesnotexist");
    }

    #[test]
    fn command_mode_write_saves_project() {
        let path = std::env::temp_dir().join(format!(
            "salieri-command-write-{}.salieri",
            std::process::id()
        ));
        let mut app = App {
            mode: AppMode::Edit,
            project_path: Some(path.clone()),
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        type_command(&mut app, "write");

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved, app.song);
        assert!(!app.dirty);
        assert!(!app.should_quit);
    }

    #[test]
    fn command_mode_write_accepts_project_path() {
        let path = std::env::temp_dir().join(format!(
            "salieri-command-write-as-{}.salieri",
            std::process::id()
        ));
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        type_command(&mut app, &format!("write {}", path.display()));

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved, app.song);
        assert_eq!(app.project_path, Some(path));
        assert!(!app.dirty);
    }

    #[test]
    fn command_mode_quit_marks_app_for_exit() {
        let mut app = App::default();

        type_command(&mut app, "quit");

        assert!(app.should_quit);
    }

    #[test]
    fn dirty_quit_opens_confirmation_dialog() {
        let mut app = App::default();

        app.set_bpm(140);
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Dialog);
        assert!(!app.should_quit);

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Normal);
        assert!(!app.should_quit);
    }

    #[test]
    fn dirty_quit_can_discard_changes() {
        let mut app = App::default();

        app.set_bpm(140);
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));

        assert!(app.should_quit);
    }

    #[test]
    fn dirty_quit_can_save_before_exit() {
        let path =
            std::env::temp_dir().join(format!("salieri-quit-save-{}.salieri", std::process::id()));
        let mut app = App {
            project_path: Some(path.clone()),
            ..App::default()
        };

        app.set_bpm(140);
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved.transport.bpm, 140);
        assert!(!app.dirty);
        assert!(app.should_quit);
    }

    #[test]
    fn force_quit_command_bypasses_dirty_confirmation() {
        let mut app = App::default();

        app.set_bpm(140);
        type_command(&mut app, "q!");

        assert_ne!(app.mode, AppMode::Dialog);
        assert!(app.should_quit);
    }

    #[test]
    fn space_toggles_playback_and_f8_stops() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

        assert!(app.is_playing);
        assert_eq!(app.playhead_row, Some(0));

        app.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));

        assert!(!app.is_playing);
        assert_eq!(app.playhead_row, None);
    }

    #[test]
    fn shift_space_starts_playback_from_pattern_start() {
        let mut app = App {
            cursor: Cursor {
                row: 12,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT));

        assert!(app.is_playing);
        assert_eq!(app.playhead_row, Some(0));
        assert_eq!(app.sequence_position, None);
    }

    #[test]
    fn uppercase_l_toggles_pattern_loop_without_breaking_vim_right() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT));
        assert!(!app.loop_pattern);

        app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert_eq!(app.cursor.field, CellField::Velocity);
        assert!(!app.loop_pattern);
    }

    #[test]
    fn enter_starts_playback_from_cursor_row() {
        let mut app = App {
            cursor: Cursor {
                row: 12,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(app.is_playing);
        assert_eq!(app.playhead_row, Some(12));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn command_mode_requests_midi_connection_and_panic_stops_playback() {
        let mut app = App::default();

        type_command(&mut app, "midi connect 3");
        assert_eq!(app.midi_status, "MIDI Connecting 3");

        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert!(app.is_playing);

        type_command(&mut app, "midi panic");
        assert!(!app.is_playing);
        assert_eq!(app.playhead_row, None);
    }

    #[test]
    fn command_mode_can_start_sequence_playback() {
        let mut app = App::default();

        type_command(&mut app, "play sequence");

        assert!(app.is_playing);
        assert_eq!(app.playhead_row, Some(0));
        assert_eq!(app.sequence_position, Some(0));

        type_command(&mut app, "stop");

        assert!(!app.is_playing);
        assert_eq!(app.sequence_position, None);
    }

    #[test]
    fn command_mode_can_start_sequence_from_position() {
        let mut app = App::default();
        type_command(&mut app, "pattern new");
        type_command(&mut app, "sequence add 2");

        type_command(&mut app, "play sequence 1");

        assert!(app.is_playing);
        assert_eq!(app.pattern_index, 1);
        assert_eq!(app.playhead_row, Some(0));
        assert_eq!(app.sequence_position, Some(1));
    }

    #[test]
    fn shift_enter_starts_sequence_from_selected_position() {
        let mut app = App::default();
        type_command(&mut app, "pattern new");
        type_command(&mut app, "sequence add 2");
        app.sequence_cursor = 1;

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));

        assert!(app.is_playing);
        assert_eq!(app.pattern_index, 1);
        assert_eq!(app.playhead_row, Some(0));
        assert_eq!(app.sequence_position, Some(1));
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Playing sequence from 1")
        );
    }

    #[test]
    fn command_mode_wq_saves_and_quits() {
        let path =
            std::env::temp_dir().join(format!("salieri-command-wq-{}.salieri", std::process::id()));
        let mut app = App {
            mode: AppMode::Edit,
            project_path: Some(path.clone()),
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        type_command(&mut app, "wq");

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved, app.song);
        assert!(!app.dirty);
        assert!(app.should_quit);
    }

    #[test]
    fn command_mode_creates_duplicates_selects_and_deletes_patterns() {
        let mut app = App::default();

        type_command(&mut app, "pattern new");
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(app.pattern_index, 1);

        type_command(&mut app, "pattern 1");
        assert_eq!(app.pattern_index, 0);

        type_command(&mut app, "pattern duplicate");
        assert_eq!(app.song.patterns.len(), 3);
        assert_eq!(app.pattern_index, 2);

        enter_command(&mut app, "pattern delete");
        assert_eq!(app.mode, AppMode::Dialog);
        assert!(matches!(
            app.dialog,
            Some(Dialog::DeletePattern {
                pattern_index: 2,
                ..
            })
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.patterns.len(), 3);
        assert_eq!(app.pattern_index, 1);
    }

    #[test]
    fn bracket_keys_select_previous_and_next_pattern() {
        let mut app = App::default();
        type_command(&mut app, "pattern new");
        type_command(&mut app, "pattern new");

        assert_eq!(app.pattern_index, 2);

        app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 0);

        app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 0);

        app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 2);

        app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 2);
    }

    #[test]
    fn uppercase_pattern_shortcuts_create_duplicate_and_delete() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT));
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT));
        assert_eq!(app.song.patterns.len(), 3);
        assert_eq!(app.pattern_index, 2);

        app.handle_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT));
        assert_eq!(app.mode, AppMode::Dialog);
        assert!(matches!(
            app.dialog,
            Some(Dialog::DeletePattern {
                pattern_index: 2,
                ..
            })
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(app.pattern_index, 1);
        assert!(app.dirty);
    }

    #[test]
    fn command_mode_renames_current_pattern() {
        let mut app = App::default();

        type_command(&mut app, "pattern rename Intro Verse");

        assert_eq!(app.song.patterns[0].name, "Intro Verse");
        assert!(app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern renamed")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.patterns[0].name, "Pattern 01");
    }

    #[test]
    fn command_mode_reports_invalid_pattern_rename() {
        let mut app = App::default();

        type_command(&mut app, "pattern rename     ");

        assert_eq!(app.song.patterns[0].name, "Pattern 01");
        assert!(!app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern rename failed: name cannot be empty")
        );
    }

    #[test]
    fn command_mode_resizes_current_pattern_and_clamps_cursor() {
        let mut app = App {
            cursor: Cursor {
                row: 63,
                ..Cursor::new()
            },
            row_offset: 44,
            ..App::default()
        };

        type_command(&mut app, "pattern length 16");

        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 16);
        assert_eq!(app.cursor.row, 15);
        assert_eq!(app.row_offset, 15);
        assert!(app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern length set to 16")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 64);
    }

    #[test]
    fn command_mode_reports_invalid_pattern_length() {
        let mut app = App::default();

        type_command(&mut app, "pattern length 0");

        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 64);
        assert!(!app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern length failed: invalid pattern length: 0")
        );
    }

    #[test]
    fn command_mode_adds_and_removes_sequence_positions() {
        let mut app = App::default();

        type_command(&mut app, "pattern new");
        type_command(&mut app, "sequence add");
        assert_eq!(
            app.song.sequence,
            vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
        );
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence added pattern 02")
        );

        type_command(&mut app, "sequence remove 0");
        assert_eq!(app.song.sequence, vec![salieri_core::PatternId(2)]);
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence removed position 00")
        );
    }

    #[test]
    fn command_mode_reports_sequence_add_pattern_out_of_range() {
        let mut app = App::default();

        type_command(&mut app, "sequence add 99");

        assert_eq!(app.song.sequence, vec![salieri_core::PatternId(1)]);
        assert!(!app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern out of range")
        );
    }

    #[test]
    fn command_mode_duplicates_sets_and_moves_sequence_positions() {
        let mut app = App::default();

        type_command(&mut app, "pattern new");
        type_command(&mut app, "pattern new");
        type_command(&mut app, "sequence add 2");
        type_command(&mut app, "sequence add 3");

        type_command(&mut app, "sequence duplicate 1");
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(1),
                salieri_core::PatternId(2),
                salieri_core::PatternId(2),
                salieri_core::PatternId(3)
            ]
        );
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence duplicated position 01")
        );

        type_command(&mut app, "sequence set 0 3");
        assert_eq!(app.song.sequence[0], salieri_core::PatternId(3));
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence position 00 set to pattern 03")
        );

        type_command(&mut app, "sequence move 3 1");
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(3),
                salieri_core::PatternId(3),
                salieri_core::PatternId(2),
                salieri_core::PatternId(2)
            ]
        );
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence moved position 03 to 01")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.sequence[1], salieri_core::PatternId(2));
    }

    #[test]
    fn keyboard_sequence_shortcuts_edit_selected_position() {
        let mut app = App::default();
        type_command(&mut app, "pattern new");
        type_command(&mut app, "pattern new");
        app.pattern_index = 1;

        app.handle_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
        assert_eq!(
            app.song.sequence,
            vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
        );
        assert_eq!(app.sequence_cursor, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char(','), KeyModifiers::NONE));
        assert_eq!(app.sequence_cursor, 0);
        app.handle_key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE));
        assert_eq!(app.sequence_cursor, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::SHIFT));
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(1),
                salieri_core::PatternId(2),
                salieri_core::PatternId(2)
            ]
        );
        assert_eq!(app.sequence_cursor, 2);

        app.pattern_index = 2;
        app.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
        assert_eq!(app.song.sequence[2], salieri_core::PatternId(3));

        app.handle_key(KeyEvent::new(KeyCode::Char('<'), KeyModifiers::SHIFT));
        assert_eq!(app.sequence_cursor, 1);
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(1),
                salieri_core::PatternId(3),
                salieri_core::PatternId(2)
            ]
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('>'), KeyModifiers::SHIFT));
        assert_eq!(app.sequence_cursor, 2);
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(1),
                salieri_core::PatternId(2),
                salieri_core::PatternId(3)
            ]
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));
        assert_eq!(
            app.song.sequence,
            vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
        );
        assert_eq!(app.sequence_cursor, 1);
        assert!(app.dirty);
    }

    #[test]
    fn command_mode_reports_sequence_position_errors() {
        let mut app = App::default();

        type_command(&mut app, "sequence remove 99");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence remove failed: sequence out of bounds: position 99")
        );
        assert!(!app.dirty);

        type_command(&mut app, "sequence duplicate 99");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence duplicate failed: sequence out of bounds: position 99")
        );

        type_command(&mut app, "sequence set 99 1");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence set failed: sequence out of bounds: position 99")
        );

        type_command(&mut app, "sequence set 0 99");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern out of range")
        );

        type_command(&mut app, "sequence move 99 0");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence move failed: sequence out of bounds: position 99")
        );
    }

    #[test]
    fn help_mode_opens_and_closes_without_mutating_state() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Help);
        assert_eq!(app.cursor.row, 0);
        assert!(!app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);

        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);
        assert!(!app.should_quit);

        app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT));
        assert_eq!(app.mode, AppMode::Help);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Help);
    }

    #[test]
    fn ctrl_t_creates_track_and_undo_restores_previous_shape() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));

        assert_eq!(app.song.tracks.len(), 5);
        assert_eq!(app.cursor.track, 4);
        assert!(app.dirty);
        assert!(app
            .song
            .current_pattern()
            .expect("pattern")
            .rows
            .iter()
            .all(|row| row.cells.len() == 5));

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

        assert_eq!(app.song.tracks.len(), 4);
        assert_eq!(app.cursor.track, 3);
        assert!(!app.dirty);
    }

    #[test]
    fn command_mode_duplicates_track_and_undo_restores_previous_shape() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };
        app.song
            .current_pattern_mut()
            .expect("pattern")
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x64)
            .expect("set note");

        type_command(&mut app, "track duplicate");

        assert_eq!(app.song.tracks.len(), 5);
        assert_eq!(app.song.tracks[4].name, "Bass Copy");
        assert_eq!(app.cursor.track, 4);
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 4)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 48 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

        assert_eq!(app.song.tracks.len(), 4);
        assert_eq!(app.cursor.track, 3);
    }

    #[test]
    fn uppercase_d_duplicates_current_track() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));

        assert_eq!(app.song.tracks.len(), 5);
        assert_eq!(app.song.tracks[4].name, "Bass Copy");
        assert_eq!(app.cursor.track, 4);
        assert!(app.dirty);
    }

    #[test]
    fn command_mode_moves_track_and_undo_restores_order() {
        let mut app = App::default();
        app.song
            .current_pattern_mut()
            .expect("pattern")
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x60)
            .expect("set bass note");
        app.song
            .current_pattern_mut()
            .expect("pattern")
            .set_note(0, 2, NoteEvent::Note { pitch: 64 }, 0x70)
            .expect("set lead note");

        type_command(&mut app, "track move 2 3");

        assert_eq!(app.song.tracks[1].name, "Lead");
        assert_eq!(app.song.tracks[2].name, "Bass");
        assert_eq!(app.cursor.track, 2);
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 1)
                .expect("lead cell")
                .note,
            Some(NoteEvent::Note { pitch: 64 })
        );
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 2)
                .expect("bass cell")
                .note,
            Some(NoteEvent::Note { pitch: 48 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

        assert_eq!(app.song.tracks[1].name, "Bass");
        assert_eq!(app.song.tracks[2].name, "Lead");
    }

    #[test]
    fn brace_shortcuts_move_current_track_left_and_right() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };
        app.song
            .current_pattern_mut()
            .expect("pattern")
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x60)
            .expect("set bass note");

        app.handle_key(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::SHIFT));

        assert_eq!(app.song.tracks[0].name, "Bass");
        assert_eq!(app.cursor.track, 0);
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 0)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 48 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('}'), KeyModifiers::SHIFT));

        assert_eq!(app.song.tracks[1].name, "Bass");
        assert_eq!(app.cursor.track, 1);
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 1)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 48 })
        );
        assert!(app.dirty);
    }

    #[test]
    fn command_mode_deletes_numbered_track_after_confirmation() {
        let mut app = App::default();

        enter_command(&mut app, "track delete 2");

        assert_eq!(app.mode, AppMode::Dialog);
        assert!(matches!(
            app.dialog,
            Some(Dialog::DeleteTrack { track_index: 1, .. })
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.song.tracks.len(), 3);
        assert_eq!(app.song.tracks[1].name, "Lead");
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

        assert_eq!(app.song.tracks.len(), 4);
        assert_eq!(app.song.tracks[1].name, "Bass");
    }

    #[test]
    fn delete_in_normal_mode_removes_current_track_and_cells() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Dialog);
        assert!(matches!(
            app.dialog,
            Some(Dialog::DeleteTrack { track_index: 1, .. })
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

        assert_eq!(app.song.tracks.len(), 3);
        assert_eq!(app.song.tracks[1].name, "Lead");
        assert_eq!(app.cursor.track, 1);
        assert!(app
            .song
            .current_pattern()
            .expect("pattern")
            .rows
            .iter()
            .all(|row| row.cells.len() == 3));
    }

    #[test]
    fn delete_track_dialog_can_be_cancelled() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.song.tracks.len(), 4);
        assert_eq!(app.song.tracks[1].name, "Bass");
    }

    #[test]
    fn cannot_delete_last_track_from_app() {
        let mut app = App::default();

        while app.song.tracks.len() > 1 {
            app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
            app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

        assert_eq!(app.song.tracks.len(), 1);
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Cannot delete the last track")
        );
    }

    #[test]
    fn mute_and_solo_commands_toggle_current_track() {
        let mut app = App {
            cursor: Cursor {
                track: 2,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));

        assert!(app.song.tracks[2].muted);
        assert!(app.song.tracks[2].solo);
        assert!(app.dirty);
    }

    #[test]
    fn command_mode_mutes_and_solos_numbered_track() {
        let mut app = App::default();

        type_command(&mut app, "track mute 2");
        type_command(&mut app, "track solo 2");

        assert!(app.song.tracks[1].muted);
        assert!(app.song.tracks[1].solo);
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert!(app.song.tracks[1].muted);
        assert!(!app.song.tracks[1].solo);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert!(!app.song.tracks[1].muted);
    }

    #[test]
    fn command_mode_changes_current_or_named_track_midi_channel() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        type_command(&mut app, "track channel 12");
        assert_eq!(app.song.tracks[1].midi_channel, 12);
        assert!(app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Track channel set to 12")
        );

        type_command(&mut app, "track channel 3 15");
        assert_eq!(app.song.tracks[2].midi_channel, 15);

        type_command(&mut app, "track channel 3 0");
        assert_eq!(app.song.tracks[2].midi_channel, 15);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Track channel failed: invalid MIDI channel: 0")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.tracks[2].midi_channel, 2);
    }

    #[test]
    fn command_mode_renames_current_or_named_track() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        type_command(&mut app, "track rename Acid Bass");
        assert_eq!(app.song.tracks[1].name, "Acid Bass");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Track renamed")
        );

        type_command(&mut app, "track rename 3 Main Lead");
        assert_eq!(app.song.tracks[2].name, "Main Lead");

        type_command(&mut app, "track rename 3    ");
        assert_eq!(app.song.tracks[2].name, "Main Lead");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Track rename failed: name cannot be empty")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.tracks[2].name, "Lead");
    }

    #[test]
    fn r_prefills_current_track_rename_command() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Command);
        assert_eq!(app.command_buffer, "track rename 2 ");

        for value in "Sub Bass".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.song.tracks[1].name, "Sub Bass");
    }

    #[test]
    fn f1_and_f2_change_octave_in_normal_mode() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert_eq!(app.octave, 5);
        assert_eq!(app.mode, AppMode::Normal);

        app.handle_key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
        assert_eq!(app.octave, 4);
    }

    fn type_command(app: &mut App, command: &str) {
        enter_command(app, command);
        assert_eq!(app.mode, AppMode::Normal);
    }

    fn enter_command(app: &mut App, command: &str) {
        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Command);
        for value in command.chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    }
}
