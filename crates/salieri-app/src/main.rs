mod persistence;
mod terminal;

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use persistence::{load_project, save_project};
use salieri_core::{CellField, Cursor, Direction, NoteEvent, Song};
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
    let project_path = std::env::args_os().nth(1).map(PathBuf::from);
    let mut app = match &project_path {
        Some(path) => App::from_file(path)
            .with_context(|| format!("failed to open project {}", path.display()))?,
        None => App::default(),
    };
    let mut terminal = TerminalGuard::enter()?;

    loop {
        terminal.draw(|frame| {
            render(
                frame,
                &app.song,
                TuiState {
                    cursor: app.cursor,
                    row_offset: app.row_offset,
                    mode_label: app.mode.label(),
                    octave: app.octave,
                    dirty: app.dirty,
                    command_line: app.command_line(),
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

#[derive(Debug)]
struct App {
    song: Song,
    clean_song: Song,
    project_path: Option<PathBuf>,
    cursor: Cursor,
    row_offset: usize,
    mode: AppMode,
    octave: u8,
    edit_step: usize,
    command_buffer: String,
    undo_stack: Vec<Song>,
    redo_stack: Vec<Song>,
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
            cursor: Cursor::new(),
            row_offset: 0,
            mode: AppMode::Normal,
            octave: DEFAULT_OCTAVE,
            edit_step: DEFAULT_EDIT_STEP,
            command_buffer: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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
            _ => true,
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        let direction = match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
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
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.current_pattern_mut() else {
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

    fn enter_velocity_digit(&mut self, digit: u8) {
        let current_digit = self.cursor.digit.min(1);
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.current_pattern_mut() else {
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
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.current_pattern_mut() else {
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

    fn execute_command(&mut self) {
        let command = self.command_buffer.trim().to_string();
        self.command_buffer.clear();
        self.mode = AppMode::Normal;

        let mut parts = command.split_whitespace();
        let Some(name) = parts.next() else {
            return;
        };

        match name {
            "q" | "quit" => self.should_quit = true,
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
            _ => {}
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
            .current_pattern()
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
}

impl AppMode {
    const fn label(self) -> &'static str {
        match self {
            AppMode::Normal => "NORMAL",
            AppMode::Edit => "EDIT",
            AppMode::Command => "COMMAND",
        }
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
