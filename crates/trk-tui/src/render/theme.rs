use ratatui::{
    prelude::{Color, Line, Modifier, Span, Style},
    widgets::{Block, Borders},
};

use crate::color::{terminal_color, RgbColor, TerminalColorMode};

pub(super) const SURFACE: Color = Color::Rgb(12, 12, 12);
pub(super) const PANEL: Color = Color::Rgb(18, 18, 18);
pub(super) const BORDER: Color = Color::Rgb(88, 88, 88);
pub(super) const BORDER_DIM: Color = Color::Rgb(48, 48, 48);
pub(super) const TEXT: Color = Color::Rgb(220, 220, 220);
pub(super) const MUTED: Color = Color::Rgb(126, 126, 126);
pub(super) const ACCENT: Color = Color::Rgb(255, 128, 0);
pub(super) const PLAYING: Color = Color::Rgb(0, 210, 96);
pub(super) const METER: Color = Color::Rgb(0, 190, 100);
pub(super) const ERROR: Color = Color::LightRed;

#[derive(Clone, Copy)]
pub(super) enum WorkspaceTabState {
    Active,
    Enabled,
    Disabled,
}

pub(super) fn base() -> Style {
    Style::default().fg(TEXT).bg(SURFACE)
}

pub(super) fn panel() -> Style {
    Style::default().fg(TEXT).bg(PANEL)
}

pub(super) fn label() -> Style {
    Style::default().fg(ACCENT).bg(SURFACE)
}

pub(super) fn muted() -> Style {
    Style::default().fg(MUTED).bg(SURFACE)
}

pub(super) fn disabled() -> Style {
    Style::default()
        .fg(BORDER)
        .bg(SURFACE)
        .add_modifier(Modifier::DIM)
}

pub(super) fn active() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn selected() -> Style {
    Style::default()
        .fg(TEXT)
        .bg(BORDER_DIM)
        .add_modifier(Modifier::REVERSED)
}

pub(super) fn playing() -> Style {
    Style::default()
        .fg(PLAYING)
        .bg(SURFACE)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn warning() -> Style {
    Style::default().fg(ACCENT).bg(SURFACE)
}

pub(super) fn error() -> Style {
    Style::default()
        .fg(ERROR)
        .bg(SURFACE)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn block(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .title(Line::from(Span::styled(title.into(), label())))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER).bg(SURFACE))
        .style(panel())
}

pub(super) fn block_for_color_mode(
    title: impl Into<String>,
    color_mode: TerminalColorMode,
) -> Block<'static> {
    if color_mode == TerminalColorMode::TrueColor {
        return block(title);
    }
    let title_style = terminal_style(RgbColor::new(255, 128, 0), None, color_mode);
    let border_style = terminal_style(
        RgbColor::new(88, 88, 88),
        Some(RgbColor::new(12, 12, 12)),
        color_mode,
    );
    let panel_style = terminal_style(
        RgbColor::new(220, 220, 220),
        Some(RgbColor::new(18, 18, 18)),
        color_mode,
    );
    Block::default()
        .title(Line::from(Span::styled(title.into(), title_style)))
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(panel_style)
}

fn terminal_style(
    foreground: RgbColor,
    background: Option<RgbColor>,
    color_mode: TerminalColorMode,
) -> Style {
    if color_mode == TerminalColorMode::Monochrome {
        return Style::reset();
    }
    let mut style = Style::default();
    if let Some(color) = terminal_color(foreground, color_mode) {
        style = style.fg(color);
    }
    if let Some(color) = background.and_then(|rgb| terminal_color(rgb, color_mode)) {
        style = style.bg(color);
    }
    style
}

pub(super) fn label_span(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), label())
}

pub(super) fn value_span(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), base())
}

pub(super) fn muted_span(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), muted())
}

pub(super) fn disabled_span(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), disabled())
}

pub(super) fn workspace_tab(text: &str, state: WorkspaceTabState) -> Span<'static> {
    match state {
        WorkspaceTabState::Active => Span::styled(format!(" {text} "), active()),
        WorkspaceTabState::Enabled => Span::styled(format!(" {text} "), label()),
        WorkspaceTabState::Disabled => Span::styled(format!(" {text}× "), disabled()),
    }
}
