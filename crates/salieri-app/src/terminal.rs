use std::{
    io::{self, Stdout},
    panic,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Once, OnceLock,
    },
};

use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

const TRACK_PANEL_WIDTH: u16 = 27;
const TRACK_CELL_WIDTH: u16 = 21;
const ROW_GUTTER_WIDTH: u16 = 5;
const MEDIUM_MIN_WIDTH: u16 = 80;
const LARGE_MIN_WIDTH: u16 = 120;
const LARGE_INSPECTOR_WIDTH: u16 = 42;

pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    interrupted: Arc<AtomicBool>,
}

impl TerminalGuard {
    pub fn enter() -> Result<Self> {
        install_panic_restore_hook();
        let interrupted = install_sigint_restore_handler()?;

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            interrupted,
        })
    }

    pub fn draw<F>(&mut self, draw: F) -> Result<()>
    where
        F: FnOnce(&mut ratatui::Frame<'_>),
    {
        self.terminal.draw(draw)?;
        Ok(())
    }

    pub fn visible_pattern_rows(&self) -> usize {
        let height = self.terminal.size().map_or(0, |area| area.height);
        height.saturating_sub(7) as usize
    }

    pub fn visible_pattern_tracks(&self) -> usize {
        let width = self.terminal.size().map_or(0, |area| area.width);
        let pattern_width = if width >= LARGE_MIN_WIDTH {
            width
                .saturating_sub(TRACK_PANEL_WIDTH)
                .saturating_sub(LARGE_INSPECTOR_WIDTH)
        } else if width >= MEDIUM_MIN_WIDTH {
            width.saturating_sub(TRACK_PANEL_WIDTH)
        } else {
            width
        };
        pattern_width
            .saturating_sub(2)
            .saturating_sub(ROW_GUTTER_WIDTH)
            .div_ceil(TRACK_CELL_WIDTH)
            .max(1) as usize
    }

    pub fn interrupted(&self) -> bool {
        self.interrupted.load(Ordering::SeqCst)
    }

    pub fn suspend<T>(&mut self, action: impl FnOnce() -> T) -> Result<T> {
        restore_terminal()?;
        let output = action();
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture, Hide)?;
        self.terminal.clear()?;
        Ok(output)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore_terminal();
    }
}

fn install_panic_restore_hook() {
    static PANIC_HOOK: Once = Once::new();
    PANIC_HOOK.call_once(|| {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let _ = restore_terminal();
            original_hook(panic_info);
        }));
    });
}

fn install_sigint_restore_handler() -> Result<Arc<AtomicBool>> {
    static INTERRUPTED: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    let interrupted = INTERRUPTED
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone();

    static SIGINT_HANDLER: Once = Once::new();
    let handler_flag = interrupted.clone();
    let mut install_result = Ok(());
    SIGINT_HANDLER.call_once(|| {
        install_result = ctrlc::set_handler(move || {
            handler_flag.store(true, Ordering::SeqCst);
            let _ = restore_terminal();
        });
    });
    install_result?;
    interrupted.store(false, Ordering::SeqCst);
    Ok(interrupted)
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        Show,
        LeaveAlternateScreen,
        DisableMouseCapture
    )
}
