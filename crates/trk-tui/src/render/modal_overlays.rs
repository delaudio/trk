use ratatui::{
    layout::Rect,
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use trk_core::MidiRoutingSettings;

use super::{
    interaction_region, AiEngineSelectorViewState, CommandPaletteViewState, InteractionMap,
    MidiSettingsState, PatternVariationHistoryViewState,
};
use crate::{ConfirmationAction, InteractionPayload, MidiSettingsAction};

pub(super) fn render_ai_engine_selector_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    selector: AiEngineSelectorViewState<'_>,
) {
    let height = u16::try_from(selector.entries.len())
        .unwrap_or(u16::MAX)
        .saturating_add(5)
        .clamp(8, 14);
    let overlay = centered_rect(90, height, area);
    let mut lines = Vec::new();
    for (index, engine) in selector.entries.iter().enumerate() {
        let selected = index == selector.selected;
        let marker = if selected { ">" } else { " " };
        let active = if engine.active { "*" } else { " " };
        let status = if engine.available {
            "[OK] Available".to_string()
        } else {
            format!("[!] {}", engine.unavailable_reason.unwrap_or("Unavailable"))
        };
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(if engine.available {
                    Color::Cyan
                } else {
                    Color::DarkGray
                })
                .add_modifier(Modifier::BOLD)
        } else if engine.available {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::styled(
            format!(
                "{marker}{active} {:<14} {:<18} {status}",
                engine.label, engine.model
            ),
            style,
        ));
    }
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  ↑/↓ select   Enter activate   Esc close   * active",
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(Clear, overlay);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(" AI Engines ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double),
        ),
        overlay,
    );
}

pub(super) fn render_pattern_variation_history_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    history: PatternVariationHistoryViewState<'_>,
) {
    let height = area.height.saturating_sub(4).clamp(10, 18);
    let overlay = centered_rect(92, height, area);
    let visible_rows = usize::from(height.saturating_sub(5));
    let selected = history
        .selected
        .min(history.entries.len().saturating_sub(1));
    let start = selected.saturating_sub(visible_rows.saturating_sub(1));
    let mut lines = Vec::new();
    if history.entries.is_empty() {
        lines.push(Line::from("  No generated pattern variations yet."));
        lines.push(Line::from(
            "  Apply an AI proposal or Euclidean transform first.",
        ));
    } else {
        for (index, entry) in history
            .entries
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_rows)
        {
            let marker = if index == selected { ">" } else { " " };
            let active = if entry.active { " [ACTIVE]" } else { "" };
            let context = entry.track_index.map_or_else(
                || format!("P{:02}", entry.pattern_index + 1),
                |track| format!("P{:02}/T{:02}", entry.pattern_index + 1, track + 1),
            );
            let description = truncate_history_description(entry.description, 34);
            let style = if index == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if entry.active {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::styled(
                format!(
                    "{marker} v{:03} {context:<8} @{} {:<9} {description}{active}",
                    entry.id, entry.timestamp, entry.source
                ),
                style,
            ));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  ↑/↓ select   Enter restore   Esc/v close",
        Style::default().fg(Color::Gray),
    ));
    frame.render_widget(Clear, overlay);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(" Pattern Variation History ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double),
        ),
        overlay,
    );
}

fn truncate_history_description(description: &str, limit: usize) -> String {
    let mut chars = description.chars();
    let head = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

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

pub(super) fn render_quit_confirmation(
    frame: &mut Frame<'_>,
    area: Rect,
    interactions: &mut InteractionMap,
) -> Rect {
    let overlay = centered_rect(48, 7, area);
    interactions.register(interaction_region::OVERLAY_QUIT_CONFIRMATION, overlay);
    let inner = confirmation_inner(overlay);
    frame.render_widget(Clear, overlay);
    frame.render_widget(
        Block::default().title(" Quit ").borders(Borders::ALL),
        overlay,
    );
    frame.render_widget(
        Paragraph::new("Unsaved changes. Save before quitting?")
            .style(Style::default().fg(Color::White)),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    render_confirmation_actions(
        frame,
        Rect::new(inner.x, inner.y.saturating_add(2), inner.width, 1),
        &[
            ("[Y] Save", ConfirmationAction::Save),
            ("[N] Don't Save", ConfirmationAction::DontSave),
            ("[C/Esc] Cancel", ConfirmationAction::Cancel),
        ],
        interactions,
    );
    overlay
}

pub(super) fn render_delete_confirmation(
    frame: &mut Frame<'_>,
    area: Rect,
    message: &str,
    interactions: &mut InteractionMap,
) -> Rect {
    let overlay = centered_rect(52, 7, area);
    interactions.register(interaction_region::OVERLAY_DELETE_CONFIRMATION, overlay);
    let inner = confirmation_inner(overlay);
    frame.render_widget(Clear, overlay);
    frame.render_widget(
        Block::default().title(" Confirm ").borders(Borders::ALL),
        overlay,
    );
    frame.render_widget(
        Paragraph::new(message.to_string()).style(Style::default().fg(Color::White)),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    render_confirmation_actions(
        frame,
        Rect::new(inner.x, inner.y.saturating_add(2), inner.width, 1),
        &[
            ("[Y] Confirm", ConfirmationAction::Confirm),
            ("[N/C/Esc] Cancel", ConfirmationAction::Cancel),
        ],
        interactions,
    );
    overlay
}

fn confirmation_inner(overlay: Rect) -> Rect {
    Rect::new(
        overlay.x.saturating_add(1),
        overlay.y.saturating_add(1),
        overlay.width.saturating_sub(2),
        overlay.height.saturating_sub(2),
    )
}

fn render_confirmation_actions(
    frame: &mut Frame<'_>,
    area: Rect,
    actions: &[(&str, ConfirmationAction)],
    interactions: &mut InteractionMap,
) {
    let mut spans = Vec::new();
    let mut cursor_x = area.x;
    let right = area.x.saturating_add(area.width);
    for (index, (label, action)) in actions.iter().copied().enumerate() {
        if index > 0 {
            spans.push(Span::raw("   "));
            cursor_x = cursor_x.saturating_add(3);
        }
        let width = (label.len() as u16).min(right.saturating_sub(cursor_x));
        interactions.register_with_payload(
            interaction_region::CONFIRMATION_ACTION,
            Rect::new(cursor_x, area.y, width, area.height),
            InteractionPayload::ConfirmationAction { action },
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
