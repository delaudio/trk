use ratatui::{
    layout::Rect,
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::CalibrationViewState;

#[derive(Debug, Clone)]
struct ControlRow<'a> {
    label: &'a str,
    value: f32,
    min: f32,
    max: f32,
    display: String,
}

pub fn render_calibration_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    state: CalibrationViewState<'_>,
) {
    let overlay = centered_rect(74, 22, area);
    let controls = [
        control("Master", state.master_gain, 0.1, 4.0, "x"),
        control("Track", state.track_gain, 0.1, 4.0, "x"),
        control("Low", state.low_gain, 0.1, 4.0, "x"),
        control("Mid", state.mid_gain, 0.1, 4.0, "x"),
        control("High", state.high_gain, 0.1, 4.0, "x"),
        control("Gate", state.gate_threshold, 0.0, 0.5, ""),
        control("Decay", state.meter_decay, 0.0, 0.95, ""),
    ];
    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Selected track: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                state.track_name.unwrap_or("unavailable"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];
    for (index, row) in controls.iter().enumerate() {
        lines.push(control_line(index, state.selected, row));
    }
    let auto_gain = if state.auto_gain { "ON" } else { "OFF" };
    lines.push(selectable_line(
        7,
        state.selected,
        format!("  Auto gain  [{auto_gain:^20}]"),
    ));
    lines.push(Line::from(""));
    lines.push(meter_line("LOW ", state.meter_low, Color::Blue));
    lines.push(meter_line("MID ", state.meter_mid, Color::Green));
    lines.push(meter_line("HIGH", state.meter_high, Color::Yellow));
    lines.push(meter_line("RMS ", state.meter_rms, Color::Cyan));
    lines.push(meter_line("PEAK", state.meter_peak, Color::Red));
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  ↑/↓ select  ←/→ adjust  r reset  t/Esc close",
        Style::default().fg(Color::DarkGray),
    ));

    frame.render_widget(Clear, overlay);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(" DSP Calibration · Live ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double),
        ),
        overlay,
    );
}

fn control<'a>(label: &'a str, value: f32, min: f32, max: f32, suffix: &str) -> ControlRow<'a> {
    ControlRow {
        label,
        value,
        min,
        max,
        display: format!("{value:.2}{suffix}"),
    }
}

fn control_line(index: usize, selected: usize, row: &ControlRow<'_>) -> Line<'static> {
    let ratio = normalized(row.value, row.min, row.max);
    selectable_line(
        index,
        selected,
        format!("  {:<9} [{}] {:>6}", row.label, bar(ratio, 20), row.display),
    )
}

fn selectable_line(index: usize, selected: usize, text: String) -> Line<'static> {
    let marker = if index == selected { ">" } else { " " };
    let style = if index == selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    Line::styled(format!("{marker}{text}"), style)
}

fn meter_line(label: &str, value: f32, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {label}  "), Style::default().fg(color)),
        Span::styled(bar(value, 32), Style::default().fg(color)),
        Span::styled(
            format!(" {:>3}%", (value.clamp(0.0, 1.0) * 100.0).round()),
            Style::default().fg(Color::Gray),
        ),
    ])
}

fn normalized(value: f32, min: f32, max: f32) -> f32 {
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

fn bar(value: f32, width: usize) -> String {
    let filled = (value.clamp(0.0, 1.0) * width as f32).round() as usize;
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}
