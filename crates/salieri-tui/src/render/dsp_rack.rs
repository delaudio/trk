use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use salieri_core::{EffectDevice, EffectDeviceKind};

use super::{
    centered_scroll_offset, dsp_parameters::render_dsp_parameter_panel, theme, truncate,
    DspDevicePaletteViewState, DspRackTargetView, DspRackViewState,
};

pub(super) fn render_dsp_rack_view(
    frame: &mut Frame<'_>,
    area: Rect,
    rack: Option<DspRackViewState<'_>>,
) {
    let Some(rack) = rack else {
        let empty = Paragraph::new("DSP rack unavailable")
            .block(Block::default().title(" DSP Rack ").borders(Borders::ALL))
            .style(theme::base());
        frame.render_widget(empty, area);
        return;
    };

    let sections = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(45),
            Constraint::Min(6),
        ])
        .split(area);
    let selected_label = match rack.selected_target {
        DspRackTargetView::Track => "Track",
        DspRackTargetView::Master => "Master",
    };
    let header = Paragraph::new(Line::from(vec![
        theme::label_span("DSP Rack  "),
        theme::value_span(format!(
            "Track {:02} {}",
            rack.track_number, rack.track_name
        )),
        theme::muted_span("  |  "),
        theme::label_span("Target: "),
        theme::value_span(selected_label),
        theme::muted_span("  |  Tab Target  Up/Down Select  :dsp Add/Edit"),
    ]))
    .block(Block::default().title(" Native DSP ").borders(Borders::ALL));
    frame.render_widget(header, sections[0]);

    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(sections[1]);
    render_dsp_chain(
        frame,
        columns[0],
        format!(" Track {:02}: {} ", rack.track_number, rack.track_name),
        rack.track_effects,
        rack.selected_target == DspRackTargetView::Track,
        rack.selected_index,
    );
    render_dsp_chain(
        frame,
        columns[1],
        " Master ".to_string(),
        rack.master_effects,
        rack.selected_target == DspRackTargetView::Master,
        rack.selected_index,
    );
    render_dsp_parameter_panel(frame, sections[2], rack);
    if let Some(palette) = rack.device_palette {
        render_dsp_device_palette(frame, area, palette, rack.selected_target);
    }
}

fn render_dsp_chain(
    frame: &mut Frame<'_>,
    area: Rect,
    title: String,
    effects: &[EffectDevice],
    selected: bool,
    selected_index: usize,
) {
    let mut lines = Vec::new();
    if effects.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Empty chain", theme::muted()),
            Span::styled("  use :dsp ...", theme::muted()),
        ]));
    } else {
        for (index, effect) in effects
            .iter()
            .enumerate()
            .take(area.height.saturating_sub(2) as usize)
        {
            let is_selected = selected && index == selected_index.min(effects.len() - 1);
            let marker = if is_selected { ">" } else { " " };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if effect.bypassed {
                theme::muted()
            } else {
                theme::base()
            };
            let bypass = if effect.bypassed { "byp" } else { "on " };
            lines.push(Line::from(Span::styled(
                format!(
                    "{marker}{:02} {:<12} {:<3} {}",
                    index + 1,
                    truncate(device_kind_label(&effect.kind), 12),
                    bypass,
                    truncate(
                        &device_summary(&effect.kind),
                        area.width.saturating_sub(24) as usize
                    )
                ),
                style,
            )));
        }
    }
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn device_kind_label(kind: &EffectDeviceKind) -> &'static str {
    match kind {
        EffectDeviceKind::Gain { .. } => "Gain",
        EffectDeviceKind::Pan { .. } => "Pan",
        EffectDeviceKind::Balance { .. } => "Balance",
        EffectDeviceKind::StereoWidth { .. } => "Width",
        EffectDeviceKind::PhaseInvert { .. } => "Phase",
        EffectDeviceKind::Filter { .. } => "Filter",
        EffectDeviceKind::Delay { .. } => "Delay",
        EffectDeviceKind::Reverb { .. } => "Reverb",
        EffectDeviceKind::Drive { .. } => "Drive",
        EffectDeviceKind::Bitcrusher { .. } => "Crusher",
        EffectDeviceKind::Chorus { .. } => "Chorus",
        EffectDeviceKind::Flanger { .. } => "Flanger",
        EffectDeviceKind::Phaser { .. } => "Phaser",
        EffectDeviceKind::Compressor { .. } => "Compressor",
        EffectDeviceKind::Gate { .. } => "Gate",
        EffectDeviceKind::Limiter { .. } => "Limiter",
    }
}

fn device_summary(kind: &EffectDeviceKind) -> String {
    match kind {
        EffectDeviceKind::Gain { gain } => format!("gain={gain:.2}"),
        EffectDeviceKind::Pan { pan } => format!("pan={pan:.2}"),
        EffectDeviceKind::Balance { balance } => format!("balance={balance:.2}"),
        EffectDeviceKind::StereoWidth { width } => format!("width={width:.2}"),
        EffectDeviceKind::PhaseInvert {
            invert_left,
            invert_right,
        } => format!(
            "L={} R={}",
            bool_label(*invert_left),
            bool_label(*invert_right)
        ),
        EffectDeviceKind::Filter {
            mode,
            cutoff_hz,
            resonance,
            mix,
            ..
        } => format!("{mode:?} cut={cutoff_hz:.0}Hz res={resonance:.2} mix={mix:.2}"),
        EffectDeviceKind::Delay {
            sync,
            time_left_ms,
            time_right_ms,
            feedback,
            ping_pong,
            mix,
            ..
        } => format!(
            "{} L={time_left_ms:.0} R={time_right_ms:.0} fb={feedback:.2} mix={mix:.2} {}",
            if *sync { "sync" } else { "free" },
            if *ping_pong { "ping" } else { "" }
        ),
        EffectDeviceKind::Reverb {
            size,
            predelay_ms,
            decay_s,
            mix,
            ..
        } => {
            format!("size={size:.2} pre={predelay_ms:.0}ms decay={decay_s:.2}s mix={mix:.2}")
        }
        EffectDeviceKind::Drive {
            mode,
            drive_db,
            tone,
            mix,
            ..
        } => format!("{mode:?} drive={drive_db:.1}dB tone={tone:.2} mix={mix:.2}"),
        EffectDeviceKind::Bitcrusher {
            bit_depth,
            reduction_ratio,
            dither,
            mix,
            ..
        } => format!(
            "bits={bit_depth} reduce={reduction_ratio:.1} dither={} mix={mix:.2}",
            bool_label(*dither)
        ),
        EffectDeviceKind::Chorus {
            rate_hz,
            depth,
            voices,
            mix,
            ..
        } => format!("rate={rate_hz:.2}Hz depth={depth:.2} voices={voices} mix={mix:.2}"),
        EffectDeviceKind::Flanger {
            rate_hz,
            depth,
            manual,
            feedback,
            mix,
            ..
        } => format!(
            "rate={rate_hz:.2}Hz depth={depth:.2} manual={manual:.2} fb={feedback:.2} mix={mix:.2}"
        ),
        EffectDeviceKind::Phaser {
            rate_hz,
            depth,
            center_hz,
            stages,
            mix,
            ..
        } => format!(
            "rate={rate_hz:.2}Hz depth={depth:.2} center={center_hz:.0}Hz stages={stages} mix={mix:.2}"
        ),
        EffectDeviceKind::Compressor {
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            mix,
            ..
        } => format!(
            "thr={threshold_db:.1}dB ratio={ratio:.1} atk={attack_ms:.1} rel={release_ms:.1} mix={mix:.2}"
        ),
        EffectDeviceKind::Gate {
            threshold_db,
            hysteresis_db,
            attack_ms,
            release_ms,
            ..
        } => format!(
            "thr={threshold_db:.1}dB hyst={hysteresis_db:.1} atk={attack_ms:.1} rel={release_ms:.1}"
        ),
        EffectDeviceKind::Limiter {
            ceiling_db,
            input_gain_db,
            release_ms,
            lookahead_ms,
            ..
        } => format!(
            "ceil={ceiling_db:.1}dB input={input_gain_db:.1}dB rel={release_ms:.1} look={lookahead_ms:.1}"
        ),
    }
}

fn bool_label(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

fn render_dsp_device_palette(
    frame: &mut Frame<'_>,
    area: Rect,
    palette: DspDevicePaletteViewState<'_>,
    target: DspRackTargetView,
) {
    let width = area.width.saturating_sub(4).min(64);
    let height = (palette.entries.len() as u16 + 2)
        .min(area.height.saturating_sub(5))
        .max(4);
    let overlay = Rect {
        x: area.x + 2,
        y: area.y + 4,
        width,
        height,
    };
    let target = match target {
        DspRackTargetView::Track => "Track",
        DspRackTargetView::Master => "Master",
    };
    let visible_entries = overlay.height.saturating_sub(2) as usize;
    let start = centered_scroll_offset(palette.entries.len(), palette.selected, visible_entries);
    let end = (start + visible_entries).min(palette.entries.len());
    let mut lines = Vec::new();
    for (index, entry) in palette.entries[start..end].iter().enumerate() {
        let absolute_index = start + index;
        let selected = absolute_index == palette.selected.min(palette.entries.len() - 1);
        let marker = if selected { ">" } else { " " };
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            theme::base()
        };
        lines.push(Line::from(Span::styled(
            format!(
                "{marker}{:02} {:<12} {}",
                absolute_index + 1,
                truncate(entry.label, 12),
                truncate(entry.summary, overlay.width.saturating_sub(20) as usize)
            ),
            style,
        )));
    }
    frame.render_widget(Clear, overlay);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(format!(" Add DSP Device -> {target} "))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        overlay,
    );
}
