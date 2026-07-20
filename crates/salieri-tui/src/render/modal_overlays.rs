use ratatui::{
    layout::Rect,
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use salieri_core::MidiRoutingSettings;

use super::{CommandPaletteViewState, MidiSettingsState};

pub(super) fn render_midi_settings_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    midi_settings: MidiSettingsState<'_>,
) {
    let overlay = centered_rect(76, 18, area);
    let mut lines = vec![
        Line::from(Span::styled(
            "Output Ports",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("  Status: {}", midi_settings.status)),
        Line::from(format!("  Input: {}", midi_settings.input_status)),
        Line::from(format!(
            "  Routing: {}",
            format_midi_routing(midi_settings.routing)
        )),
        Line::from(""),
    ];

    if midi_settings.ports.is_empty() {
        lines.push(Line::from("  No MIDI output ports found"));
        lines.push(Line::from(""));
        lines.push(Line::from(
            "  On macOS, enable IAC Driver in Audio MIDI Setup.",
        ));
    } else {
        for (row, port) in midi_settings.ports.iter().enumerate() {
            let marker = if row == midi_settings.selected_port {
                ">"
            } else {
                " "
            };
            let line = format!("{marker} {:02} {}", port.index, port.name);
            if row == midi_settings.selected_port {
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
        Line::from("Enter connect selected   d disconnect   p panic/all notes off"),
        Line::from("F5/r refresh ports   Esc/q close"),
    ]);

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" MIDI Settings ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
    frame.render_widget(Clear, overlay);
    frame.render_widget(paragraph, overlay);
}

fn format_midi_routing(settings: &MidiRoutingSettings) -> String {
    format!(
        "clock {}/{}, transport {}/{}, notes {}/{}, cc {}/{}, ch {}/{}, C={}, delay={}ms",
        on_off(settings.clock_in),
        on_off(settings.clock_out),
        on_off(settings.transport_in),
        on_off(settings.transport_out),
        on_off(settings.notes_in),
        on_off(settings.notes_out),
        on_off(settings.cc_in),
        on_off(settings.cc_out),
        format_channel_filter(&settings.input_channels),
        format_channel_filter(&settings.output_channels),
        settings.middle_c,
        settings.clock_sync_delay_ms,
    )
}

fn on_off(value: bool) -> &'static str {
    if value {
        "ON"
    } else {
        "OFF"
    }
}

fn format_channel_filter(channels: &[u8]) -> String {
    if channels.is_empty() {
        "all".to_string()
    } else {
        channels
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}

pub(super) fn render_command_palette_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    palette: CommandPaletteViewState<'_>,
) {
    let overlay = centered_rect(86, 20, area);
    let visible_rows = overlay.height.saturating_sub(6) as usize;
    let selected = palette
        .selected
        .min(palette.entries.len().saturating_sub(1));
    let start = selected.saturating_sub(visible_rows.saturating_sub(1));
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Search: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                if palette.query.is_empty() {
                    "<type to filter>".to_string()
                } else {
                    palette.query.to_string()
                },
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
    ];

    if palette.entries.is_empty() {
        lines.push(Line::from("  No matching actions"));
    } else {
        for (row, entry) in palette
            .entries
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_rows)
        {
            let selected_row = row == selected;
            let disabled = entry.disabled_reason.is_some();
            let base_style = if disabled {
                Style::default().fg(Color::DarkGray)
            } else if selected_row {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let marker = if selected_row { ">" } else { " " };
            let recent = if entry.recent { " recent" } else { "" };
            let shortcut = entry.shortcut.unwrap_or("");
            let detail = entry
                .disabled_reason
                .map_or_else(|| entry.command.to_string(), |reason| reason.to_string());
            lines.push(Line::from(vec![
                Span::styled(format!("{marker} "), base_style),
                Span::styled(
                    format!("{:<11}", entry.category),
                    base_style.fg(Color::Cyan),
                ),
                Span::styled(format!("{:<28}", entry.title), base_style),
                Span::styled(format!("{:<14}", shortcut), base_style.fg(Color::Green)),
                Span::styled(detail, base_style.fg(Color::Gray)),
                Span::styled(recent, base_style.fg(Color::Magenta)),
            ]));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from("  Enter execute   Esc cancel   ↑/↓ navigate"));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Command Palette ")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    frame.render_widget(Clear, overlay);
    frame.render_widget(paragraph, overlay);
}

pub(super) fn render_quit_confirmation(frame: &mut Frame<'_>, area: Rect) {
    let overlay = centered_rect(48, 7, area);
    let lines = vec![
        Line::from("Unsaved changes. Save before quitting?"),
        Line::from(""),
        Line::from("[Y]es   [N]o   [C]ancel"),
    ];
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(" Quit ").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(Clear, overlay);
    frame.render_widget(paragraph, overlay);
}

pub(super) fn render_delete_confirmation(frame: &mut Frame<'_>, area: Rect, message: &str) {
    let overlay = centered_rect(52, 7, area);
    let lines = vec![
        Line::from(message.to_string()),
        Line::from(""),
        Line::from("[Y]es   [N]o   [Esc] Cancel"),
    ];
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(" Confirm ").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(Clear, overlay);
    frame.render_widget(paragraph, overlay);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}
