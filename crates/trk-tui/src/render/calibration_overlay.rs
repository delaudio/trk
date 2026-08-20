use ratatui::{
    layout::Rect,
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::CalibrationViewState;
use crate::color::{rgb_gradient, terminal_color, RgbColor, TerminalColorMode};

const METER_GRADIENT: [(f32, RgbColor); 5] = [
    (0.0, RgbColor::new(16, 92, 54)),
    (0.80, RgbColor::new(20, 224, 98)),
    (0.95, RgbColor::new(255, 222, 64)),
    (0.99, RgbColor::new(255, 88, 42)),
    (1.0, RgbColor::new(255, 255, 244)),
];
const METER_CLIP_THRESHOLD: f32 = 0.999;

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
    lines.push(meter_line("LOW ", state.meter_low, state.color_mode));
    lines.push(meter_line("MID ", state.meter_mid, state.color_mode));
    lines.push(meter_line("HIGH", state.meter_high, state.color_mode));
    lines.push(meter_line("RMS ", state.meter_rms, state.color_mode));
    lines.push(meter_line("PEAK", state.meter_peak, state.color_mode));
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

fn meter_line(label: &str, value: f32, color_mode: TerminalColorMode) -> Line<'static> {
    let value = finite_meter(value);
    let mut label_style = Style::default();
    if let Some(color) = terminal_color(meter_rgb(value), color_mode) {
        label_style = label_style.fg(color);
    }
    let mut spans = vec![Span::styled(format!("  {label}  "), label_style)];
    spans.extend(meter_spans(value, 32, color_mode));
    let percentage_style = if color_mode == TerminalColorMode::Monochrome {
        Style::default()
    } else {
        Style::default().fg(Color::Gray)
    };
    spans.push(Span::styled(
        format!(" {:>3}%", (value * 100.0).round()),
        percentage_style,
    ));
    Line::from(spans)
}

fn meter_spans(value: f32, width: usize, color_mode: TerminalColorMode) -> Vec<Span<'static>> {
    let value = finite_meter(value);
    let clipped = value >= METER_CLIP_THRESHOLD;
    let filled = if clipped {
        width
    } else {
        ((value * width as f32).floor() as usize)
            .max(usize::from(value > 0.0))
            .min(width)
    };
    (0..width)
        .map(|index| {
            if index >= filled {
                return Span::styled("░", Style::default().add_modifier(Modifier::DIM));
            }
            let cell_amplitude = (index + 1) as f32 / width as f32;
            let mut style = Style::default();
            if let Some(color) = terminal_color(meter_rgb(cell_amplitude), color_mode) {
                style = style.fg(color);
            }
            if index + 1 == width && clipped {
                style = style.add_modifier(Modifier::BOLD);
            }
            Span::styled("█", style)
        })
        .collect()
}

fn meter_rgb(amplitude: f32) -> RgbColor {
    rgb_gradient(&METER_GRADIENT, meter_gradient_position(amplitude))
}

fn meter_gradient_position(amplitude: f32) -> f32 {
    let amplitude = finite_meter(amplitude);
    if amplitude <= 0.0 {
        return 0.0;
    }
    let decibels = (20.0 * amplitude.log10()).clamp(-60.0, 0.0);
    (decibels + 60.0) / 60.0
}

fn finite_meter(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn amplitude(decibels: f32) -> f32 {
        10.0_f32.powf(decibels / 20.0)
    }

    #[test]
    fn meter_gradient_tracks_safe_warning_hot_and_clip_thresholds() {
        let safe = meter_rgb(amplitude(-18.0));
        let warning = meter_rgb(amplitude(-3.0));
        let hot = meter_rgb(amplitude(-0.5));
        let clip = meter_rgb(1.0);

        assert!(safe.green > safe.red);
        assert!(warning.red > 200 && warning.green > 180);
        assert!(hot.red > hot.green);
        assert_eq!(clip, RgbColor::new(255, 255, 244));
        assert_eq!(meter_gradient_position(f32::NAN), 0.0);
    }

    #[test]
    fn meter_fallbacks_preserve_cells_without_unsupported_colors() {
        for mode in [
            TerminalColorMode::TrueColor,
            TerminalColorMode::Indexed256,
            TerminalColorMode::Ansi16,
            TerminalColorMode::Monochrome,
        ] {
            let spans = meter_spans(1.0, 16, mode);
            assert_eq!(spans.len(), 16);
            assert!(spans[15].style.add_modifier.contains(Modifier::BOLD));
            let colors = spans
                .iter()
                .filter_map(|span| span.style.fg)
                .collect::<Vec<_>>();
            match mode {
                TerminalColorMode::TrueColor => {
                    assert!(colors.iter().any(|color| matches!(color, Color::Rgb(..))));
                }
                TerminalColorMode::Indexed256 => {
                    assert!(colors
                        .iter()
                        .all(|color| matches!(color, Color::Indexed(_))));
                }
                TerminalColorMode::Ansi16 => {
                    assert!(!colors.is_empty());
                    assert!(colors
                        .iter()
                        .all(|color| !matches!(color, Color::Rgb(..) | Color::Indexed(_))));
                }
                TerminalColorMode::Monochrome => assert!(colors.is_empty()),
            }
        }
        assert!(meter_spans(f32::NAN, 8, TerminalColorMode::TrueColor)
            .iter()
            .all(|span| span.content == "░"));
        assert_eq!(
            meter_spans(0.99, 32, TerminalColorMode::TrueColor)
                .iter()
                .filter(|span| span.content == "█")
                .count(),
            31
        );
        assert_eq!(
            meter_spans(0.001, 32, TerminalColorMode::TrueColor)
                .iter()
                .filter(|span| span.content == "█")
                .count(),
            1
        );
        let near_clip = meter_spans(METER_CLIP_THRESHOLD, 32, TerminalColorMode::TrueColor);
        assert!(near_clip.last().is_some_and(
            |span| span.content == "█" && span.style.add_modifier.contains(Modifier::BOLD)
        ));
    }
}
