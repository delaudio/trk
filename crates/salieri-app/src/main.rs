mod terminal;

use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use salieri_core::{Cursor, Direction, Song};
use salieri_tui::{render, TuiState};
use terminal::TerminalGuard;

const UI_TICK_RATE: Duration = Duration::from_millis(33);

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
    let mut terminal = TerminalGuard::enter()?;
    let mut app = App::default();

    loop {
        terminal.draw(|frame| {
            render(
                frame,
                &app.song,
                TuiState {
                    cursor: app.cursor,
                    row_offset: app.row_offset,
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
    cursor: Cursor,
    row_offset: usize,
    should_quit: bool,
    last_tick: Instant,
}

impl Default for App {
    fn default() -> Self {
        Self {
            song: Song::empty(),
            cursor: Cursor::new(),
            row_offset: 0,
            should_quit: false,
            last_tick: Instant::now(),
        }
    }
}

impl App {
    fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return;
        }

        let direction = match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Up => Some(Direction::Up),
            KeyCode::Down => Some(Direction::Down),
            KeyCode::Left => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            _ => None,
        };

        if let Some(direction) = direction {
            let row_count = self.current_row_count();
            let track_count = self.song.tracks.len();
            self.cursor.move_in(direction, row_count, track_count);
        }
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
}
