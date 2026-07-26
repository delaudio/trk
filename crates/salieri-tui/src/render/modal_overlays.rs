use ratatui::{
    layout::Rect,
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use salieri_core::MidiRoutingSettings;

use super::{interaction_region, CommandPaletteViewState, InteractionMap, MidiSettingsState};
use crate::{InteractionPayload, MidiSettingsAction};

pub(super) fn render_midi_settings_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    midi_settings: MidiSettingsState<'_>,
    interactions: &mut InteractionMap,
) -> Rect {
    let overlay = centered_rect(76, 18, area);
    interactions.register(interaction_region::OVERLAY_MIDI_SETTINGS, overlay);
    let inner = Rect::new(
        overlay.x.saturating_add(1),
        overlay.y.saturating_add(1),
        overlay.width.saturating_sub(2),
        overlay.height.saturating_sub(2),
    );
    let header_area = Rect::new(inner.x, inner.y, inner.width, inner.height.min(5));
    let actions_area = Rect::new(
        inner.x,
        inner.y.saturating_add(inner.height.saturating_sub(1)),
        inner.width,
        u16::from(inner.height > 0),
    );
    let ports_area = Rect::new(
        inner.x,
        inner.y.saturating_add(header_area.height),
        inner.width,
        inner
            .height
            .saturating_sub(header_area.height)
            .saturating_sub(actions_area.height),
    );
    let header_lines = vec![
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
    ];
    let mut port_lines = Vec::new();
    if midi_settings.ports.is_empty() {
        port_lines.push(Line::from("  No MIDI output ports found"));
        port_lines.push(Line::from(""));
        port_lines.push(Line::from(
            "  On macOS, enable IAC Driver in Audio MIDI Setup.",
        ));
    } else {
        let visible_rows = ports_area.height as usize;
        let selected = midi_settings
            .selected_port
            .min(midi_settings.ports.len().saturating_sub(1));
        let start = selected.saturating_sub(visible_rows.saturating_sub(1));
        for (visible_row, (index, port)) in midi_settings
            .ports
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_rows)
            .enumerate()
        {
            interactions.register_with_payload(
                interaction_region::MIDI_SETTINGS_PORT,
                Rect::new(
                    ports_area.x,
                    ports_area.y.saturating_add(visible_row as u16),
                    ports_area.width,
                    1,
                ),
                InteractionPayload::MidiPortRow { index },
            );
            let marker = if index == selected { ">" } else { " " };
            let line = format!("{marker} {:02} {}", port.index, port.name);
            if index == selected {
                port_lines.push(Line::styled(
                    line,
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                port_lines.push(Line::from(line));
            }
        }
    }

    frame.render_widget(Clear, overlay);
    frame.render_widget(
        Block::default()
            .title(" MIDI Settings ")
            .borders(Borders::ALL),
        overlay,
    );
    frame.render_widget(
        Paragraph::new(header_lines)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White)),
        header_area,
    );
    frame.render_widget(
        Paragraph::new(port_lines).style(Style::default().fg(Color::White)),
        ports_area,
    );
    render_midi_settings_actions(frame, actions_area, interactions);
    overlay
}

fn render_midi_settings_actions(
    frame: &mut Frame<'_>,
    area: Rect,
    interactions: &mut InteractionMap,
) {
    const ACTIONS: [(&str, MidiSettingsAction); 5] = [
        ("[Connect]", MidiSettingsAction::Connect),
        ("[Disconnect]", MidiSettingsAction::Disconnect),
        ("[Panic]", MidiSettingsAction::Panic),
        ("[Refresh]", MidiSettingsAction::Refresh),
        ("[Close]", MidiSettingsAction::Close),
    ];
    let mut spans = Vec::new();
    let mut cursor_x = area.x;
    let right = area.x.saturating_add(area.width);
    for (index, (label, action)) in ACTIONS.iter().copied().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
            cursor_x = cursor_x.saturating_add(1);
        }
        let width = (label.len() as u16).min(right.saturating_sub(cursor_x));
        interactions.register_with_payload(
            interaction_region::MIDI_SETTINGS_ACTION,
            Rect::new(cursor_x, area.y, width, area.height),
            InteractionPayload::MidiSettingsAction { action },
        );
        spans.push(Span::styled(
            label,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        cursor_x = cursor_x.saturating_add(width);
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
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
    interactions: &mut InteractionMap,
) -> Rect {
    let overlay = centered_rect(86, 20, area);
    interactions.register(interaction_region::OVERLAY_COMMAND_PALETTE, overlay);
    let visible_rows = overlay.height.saturating_sub(6) as usize;
    let selected = palette
        .selected
        .min(palette.entries.len().saturating_sub(1));
    let start = selected.saturating_sub(visible_rows.saturating_sub(1));
    let visible_count = palette
        .entries
        .len()
        .saturating_sub(start)
        .min(visible_rows);
    let result_area = Rect::new(
        overlay.x.saturating_add(1),
        overlay.y.saturating_add(3),
        overlay.width.saturating_sub(2),
        visible_count as u16,
    );
    interactions.register(interaction_region::COMMAND_PALETTE_RESULTS, result_area);
    for visible_row in 0..visible_count {
        let index = start + visible_row;
        interactions.register_with_payload(
            interaction_region::COMMAND_PALETTE_ENTRY,
            Rect::new(
                result_area.x,
                result_area.y.saturating_add(visible_row as u16),
                result_area.width,
                1,
            ),
            InteractionPayload::CommandPaletteEntry { index },
        );
    }
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
        .style(Style::default().fg(Color::White));
    frame.render_widget(Clear, overlay);
    frame.render_widget(paragraph, overlay);
    overlay
}

pub(super) fn render_quit_confirmation(frame: &mut Frame<'_>, area: Rect) -> Rect {
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
    overlay
}

pub(super) fn render_delete_confirmation(frame: &mut Frame<'_>, area: Rect, message: &str) -> Rect {
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
    overlay
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
