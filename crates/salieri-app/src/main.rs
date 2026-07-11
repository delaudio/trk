mod persistence;
mod playback_runtime;
mod terminal;

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use persistence::{load_project, save_project};
use playback_runtime::{PlaybackRuntime, PlaybackUpdate};
use salieri_core::{CellField, Cursor, Direction, NoteEvent, Song};
use salieri_midi::list_output_ports;
use salieri_tui::{render, TuiState};
use terminal::TerminalGuard;

const UI_TICK_RATE: Duration = Duration::from_millis(33);
const DEFAULT_OCTAVE: u8 = 4;
const DEFAULT_EDIT_STEP: usize = 1;
const DEFAULT_NOTE_VELOCITY: u8 = 0x7f;
const UNDO_LIMIT: usize = 100;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "salieri=info".into()),
        )
        .init();

    let result = run();
    if let Err(error) = &result {
        tracing::error!(?error, "application exited with an error");
    }
    result
}

fn run() -> Result<()> {
    let args = CliArgs::parse(std::env::args().skip(1));
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
        CliCommand::Run => {}
    }

    let project_path = args.project_path;
    let mut app = match &project_path {
        Some(path) => App::from_file(path)
            .with_context(|| format!("failed to open project {}", path.display()))?,
        None => App::default(),
    };
    let mut terminal = TerminalGuard::enter()?;

    loop {
        app.drain_playback_updates();
        terminal.draw(|frame| {
            render(
                frame,
                &app.song,
                TuiState {
                    cursor: app.cursor,
                    row_offset: app.row_offset,
                    pattern_index: app.pattern_index,
                    mode_label: app.mode.label(),
                    octave: app.octave,
                    dirty: app.dirty,
                    command_line: app.command_line(),
                    show_help: app.mode == AppMode::Help,
                    is_playing: app.is_playing,
                    playhead_row: app.playhead_row,
                    midi_status: app.midi_status.as_str(),
                    sequence_position: app.sequence_position,
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
                    app.keep_cursor_visible(terminal.visible_pattern_rows());
                }
                Event::Resize(_, _) => app.keep_cursor_visible(terminal.visible_pattern_rows()),
                _ => {}
            }
        }

        if app.last_tick.elapsed() >= UI_TICK_RATE {
            app.last_tick = Instant::now();
            app.keep_cursor_visible(terminal.visible_pattern_rows());
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliArgs {
    command: CliCommand,
    project_path: Option<PathBuf>,
}

impl CliArgs {
    fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut project_path = None;

        for arg in args {
            match arg.as_str() {
                "-h" | "--help" => {
                    return Self {
                        command: CliCommand::Help,
                        project_path: None,
                    }
                }
                "-V" | "--version" => {
                    return Self {
                        command: CliCommand::Version,
                        project_path: None,
                    }
                }
                "--list-midi-outputs" => {
                    return Self {
                        command: CliCommand::ListMidiOutputs,
                        project_path: None,
                    }
                }
                _ if project_path.is_none() => project_path = Some(PathBuf::from(arg)),
                _ => {}
            }
        }

        Self {
            command: CliCommand::Run,
            project_path,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliCommand {
    Run,
    Help,
    Version,
    ListMidiOutputs,
}

fn print_help() {
    println!(
        "Salieri Tracker\n\nUsage:\n  salieri [FILE]\n  salieri --list-midi-outputs\n  salieri --help\n  salieri --version\n\nOptions:\n  --list-midi-outputs  List available MIDI output ports\n  --help               Show this help\n  --version            Show version"
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
    command_buffer: String,
    undo_stack: Vec<Song>,
    redo_stack: Vec<Song>,
    playback: PlaybackRuntime,
    is_playing: bool,
    playhead_row: Option<usize>,
    sequence_position: Option<usize>,
    midi_status: String,
    dirty: bool,
    should_quit: bool,
    last_tick: Instant,
}

impl Default for App {
    fn default() -> Self {
        let song = Song::empty();
        Self {
            clean_song: song.clone(),
            song,
            project_path: None,
            pattern_index: 0,
            cursor: Cursor::new(),
            row_offset: 0,
            mode: AppMode::Normal,
            octave: DEFAULT_OCTAVE,
            edit_step: DEFAULT_EDIT_STEP,
            command_buffer: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            playback: PlaybackRuntime::spawn(),
            is_playing: false,
            playhead_row: None,
            sequence_position: None,
            midi_status: "MIDI Disconnected".to_string(),
            dirty: false,
            should_quit: false,
            last_tick: Instant::now(),
        }
    }
}

impl App {
    fn from_file(path: &Path) -> Result<Self> {
        let song = load_project(path)?;
        Ok(Self {
            clean_song: song.clone(),
            song,
            project_path: Some(path.to_path_buf()),
            ..Self::default()
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
                }
                true
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.create_track();
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
        let direction = match key.code {
            KeyCode::Char('q') => {
                self.stop_playback();
                self.should_quit = true;
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
            KeyCode::Char('i') | KeyCode::Enter => {
                self.mode = AppMode::Edit;
                return;
            }
            KeyCode::Char(':') => {
                self.command_buffer.clear();
                self.mode = AppMode::Command;
                return;
            }
            KeyCode::Char('?') => {
                self.mode = AppMode::Help;
                return;
            }
            KeyCode::Up => Some(Direction::Up),
            KeyCode::Char('k') => Some(Direction::Up),
            KeyCode::Down => Some(Direction::Down),
            KeyCode::Char('j') => Some(Direction::Down),
            KeyCode::Left => Some(Direction::Left),
            KeyCode::Char('h') => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            KeyCode::Char('l') => Some(Direction::Right),
            KeyCode::Home => {
                self.cursor.row = 0;
                return;
            }
            KeyCode::End => {
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
            KeyCode::Delete => {
                self.delete_current_track();
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
            KeyCode::Home => self.cursor.row = 0,
            KeyCode::End => self.cursor.row = self.current_row_count().saturating_sub(1),
            KeyCode::PageUp => self.page_cursor_up(),
            KeyCode::PageDown => self.page_cursor_down(),
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

    fn delete_current_track(&mut self) {
        let track = self.cursor.track;
        let before_count = self.song.tracks.len();
        self.mutate_song(|song, _| {
            let _ = song.delete_track(track);
        });

        if self.song.tracks.len() < before_count {
            self.clamp_cursor();
            self.cursor.digit = 0;
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

    fn toggle_current_mute(&mut self) {
        self.mutate_song(|song, cursor| {
            let _ = song.toggle_mute(cursor.track);
        });
    }

    fn toggle_current_solo(&mut self) {
        self.mutate_song(|song, cursor| {
            let _ = song.toggle_solo(cursor.track);
        });
    }

    fn set_track_midi_channel(&mut self, track_index: usize, midi_channel: u8) {
        self.mutate_song(|song, _| {
            let _ = song.set_track_midi_channel(track_index, midi_channel);
        });
    }

    fn rename_track(&mut self, track_index: usize, name: String) {
        self.mutate_song(|song, _| {
            let _ = song.rename_track(track_index, name);
        });
    }

    fn set_bpm(&mut self, bpm: u16) {
        self.mutate_song(|song, _| {
            song.transport.bpm = bpm;
        });
    }

    fn set_lpb(&mut self, lpb: u8) {
        self.mutate_song(|song, _| {
            song.transport.lines_per_beat = lpb;
        });
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

    fn delete_current_pattern(&mut self) {
        let pattern_index = self.pattern_index;
        let before_count = self.song.patterns.len();
        self.mutate_song(|song, _| {
            let _ = song.delete_pattern(pattern_index);
        });
        if self.song.patterns.len() < before_count {
            self.pattern_index = self
                .pattern_index
                .min(self.song.patterns.len().saturating_sub(1));
            self.clamp_cursor();
            self.row_offset = 0;
        }
    }

    fn resize_current_pattern(&mut self, row_count: usize) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, _| {
            let _ = song.resize_pattern(pattern_index, row_count);
        });
        self.clamp_cursor();
        self.keep_cursor_visible(1);
    }

    fn rename_current_pattern(&mut self, name: String) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, _| {
            let _ = song.rename_pattern(pattern_index, name);
        });
    }

    fn select_pattern(&mut self, pattern_index: usize) {
        if pattern_index < self.song.patterns.len() {
            self.pattern_index = pattern_index;
            self.clamp_cursor();
            self.row_offset = 0;
        }
    }

    fn add_sequence_pattern(&mut self, pattern_index: usize) {
        let Some(pattern_id) = self.song.pattern(pattern_index).map(|pattern| pattern.id) else {
            return;
        };
        self.mutate_song(|song, _| {
            let _ = song.push_sequence_pattern(pattern_id);
        });
    }

    fn remove_sequence_position(&mut self, position: usize) {
        self.mutate_song(|song, _| {
            let _ = song.remove_sequence_position(position);
        });
    }

    fn duplicate_sequence_position(&mut self, position: usize) {
        self.mutate_song(|song, _| {
            let _ = song.duplicate_sequence_position(position);
        });
    }

    fn set_sequence_pattern(&mut self, position: usize, pattern_index: usize) {
        let Some(pattern_id) = self.song.pattern(pattern_index).map(|pattern| pattern.id) else {
            return;
        };
        self.mutate_song(|song, _| {
            let _ = song.set_sequence_pattern(position, pattern_id);
        });
    }

    fn move_sequence_position(&mut self, from: usize, to: usize) {
        self.mutate_song(|song, _| {
            let _ = song.move_sequence_position(from, to);
        });
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
            "q" | "quit" => {
                self.stop_playback();
                self.should_quit = true;
            }
            "w" | "write" | "save" => {
                if let Err(error) = self.save() {
                    tracing::error!(?error, "failed to save project");
                }
            }
            "wq" => {
                if let Err(error) = self.save() {
                    tracing::error!(?error, "failed to save project");
                    return;
                }
                self.stop_playback();
                self.should_quit = true;
            }
            "bpm" => {
                if let Some(value) = parts.next().and_then(|value| value.parse::<u16>().ok()) {
                    self.set_bpm(value);
                }
            }
            "lpb" => {
                if let Some(value) = parts.next().and_then(|value| value.parse::<u8>().ok()) {
                    self.set_lpb(value);
                }
            }
            "midi" => match parts.next() {
                Some("connect") => {
                    if let Some(port_index) =
                        parts.next().and_then(|value| value.parse::<usize>().ok())
                    {
                        self.connect_midi(port_index);
                    }
                }
                Some("disconnect") => self.disconnect_midi(),
                Some("panic") => self.panic_midi(),
                None | Some(_) => {}
            },
            "play" => match parts.next() {
                Some("sequence") | Some("seq") => self.start_sequence_playback(),
                Some("pattern") | Some("pat") | None => self.start_playback(),
                Some(_) => {}
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
                None | Some(_) => {}
            },
            "pattern" => match parts.next() {
                Some("new") => self.create_pattern(),
                Some("duplicate") | Some("dup") => self.duplicate_current_pattern(),
                Some("delete") | Some("del") => self.delete_current_pattern(),
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
                None | Some(_) => {}
            },
            _ => {}
        }
    }

    fn toggle_playback(&mut self) {
        if self.is_playing {
            self.stop_playback();
        } else {
            self.start_playback();
        }
    }

    fn start_playback(&mut self) {
        if self.song.pattern(self.pattern_index).is_none() {
            return;
        }

        self.is_playing = true;
        self.playhead_row = Some(0);
        self.sequence_position = None;
        self.playback
            .start_pattern(self.song.clone(), self.pattern_index);
    }

    fn start_sequence_playback(&mut self) {
        if self.song.sequence.is_empty() {
            return;
        }

        if let Some(first_pattern_id) = self.song.sequence.first() {
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
        self.sequence_position = Some(0);
        self.playback.start_sequence(self.song.clone());
    }

    fn stop_playback(&mut self) {
        self.playback.stop();
        self.is_playing = false;
        self.playhead_row = None;
        self.sequence_position = None;
    }

    fn connect_midi(&mut self, port_index: usize) {
        self.midi_status = format!("MIDI Connecting {port_index}");
        self.playback.connect_midi(port_index);
    }

    fn disconnect_midi(&mut self) {
        self.playback.disconnect_midi();
    }

    fn panic_midi(&mut self) {
        self.playback.panic_all_notes_off();
        self.is_playing = false;
        self.playhead_row = None;
        self.sequence_position = None;
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
                }
                PlaybackUpdate::MidiConnected { port_index } => {
                    self.midi_status = format!("MIDI Connected {port_index}");
                }
                PlaybackUpdate::MidiDisconnected => {
                    self.midi_status = "MIDI Disconnected".to_string();
                }
                PlaybackUpdate::MidiError(error) => {
                    self.midi_status = format!("MIDI Error: {error}");
                    self.is_playing = false;
                    self.playhead_row = None;
                    self.sequence_position = None;
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
        }
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            let current = std::mem::replace(&mut self.song, previous);
            self.redo_stack.push(current);
            self.refresh_dirty();
            self.clamp_cursor();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = std::mem::replace(&mut self.song, next);
            self.undo_stack.push(current);
            self.refresh_dirty();
            self.clamp_cursor();
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
        save_project(&path, &self.song)?;
        self.project_path = Some(path);
        self.clean_song = self.song.clone();
        self.refresh_dirty();
        Ok(())
    }

    fn clamp_cursor(&mut self) {
        self.pattern_index = self
            .pattern_index
            .min(self.song.patterns.len().saturating_sub(1));
        self.cursor
            .clamp(self.current_row_count(), self.song.tracks.len());
    }

    fn keep_cursor_visible(&mut self, visible_rows: usize) {
        let visible_rows = visible_rows.max(1);
        if self.cursor.row < self.row_offset {
            self.row_offset = self.cursor.row;
        } else if self.cursor.row >= self.row_offset.saturating_add(visible_rows) {
            self.row_offset = self.cursor.row.saturating_sub(visible_rows - 1);
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    Normal,
    Edit,
    Command,
    Help,
}

impl AppMode {
    const fn label(self) -> &'static str {
        match self {
            AppMode::Normal => "NORMAL",
            AppMode::Edit => "EDIT",
            AppMode::Command => "COMMAND",
            AppMode::Help => "HELP",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_help_version_and_midi_listing() {
        assert_eq!(
            CliArgs::parse(["--help".to_string()]),
            CliArgs {
                command: CliCommand::Help,
                project_path: None
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
                project_path: Some(PathBuf::from("song.salieri"))
            }
        );
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
    fn command_mode_quit_marks_app_for_exit() {
        let mut app = App::default();

        type_command(&mut app, "quit");

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

        type_command(&mut app, "pattern delete");
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.patterns.len(), 3);
        assert_eq!(app.pattern_index, 1);
    }

    #[test]
    fn command_mode_renames_current_pattern() {
        let mut app = App::default();

        type_command(&mut app, "pattern rename Intro Verse");

        assert_eq!(app.song.patterns[0].name, "Intro Verse");
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.patterns[0].name, "Pattern 01");
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

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 64);
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

        type_command(&mut app, "sequence remove 0");
        assert_eq!(app.song.sequence, vec![salieri_core::PatternId(2)]);
        assert_eq!(app.song.patterns.len(), 2);
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

        type_command(&mut app, "sequence set 0 3");
        assert_eq!(app.song.sequence[0], salieri_core::PatternId(3));

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

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.sequence[1], salieri_core::PatternId(2));
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
    fn delete_in_normal_mode_removes_current_track_and_cells() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

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
    fn cannot_delete_last_track_from_app() {
        let mut app = App::default();

        while app.song.tracks.len() > 1 {
            app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

        assert_eq!(app.song.tracks.len(), 1);
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

        type_command(&mut app, "track channel 3 15");
        assert_eq!(app.song.tracks[2].midi_channel, 15);

        type_command(&mut app, "track channel 3 0");
        assert_eq!(app.song.tracks[2].midi_channel, 15);

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

        type_command(&mut app, "track rename 3 Main Lead");
        assert_eq!(app.song.tracks[2].name, "Main Lead");

        type_command(&mut app, "track rename 3    ");
        assert_eq!(app.song.tracks[2].name, "Main Lead");

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.tracks[2].name, "Lead");
    }

    fn type_command(app: &mut App, command: &str) {
        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Command);
        for value in command.chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);
    }
}
