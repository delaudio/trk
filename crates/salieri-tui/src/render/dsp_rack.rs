use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};
use salieri_core::{EffectDevice, EffectDeviceKind};

use crate::{interaction_region, DspRackChain, InteractionMap, InteractionPayload};

use super::{
    centered_scroll_offset, dsp_parameters::render_dsp_parameter_panel, theme, truncate,
    DspDevicePaletteViewState, DspRackTargetView, DspRackViewState,
};

const HEADER_PREFIX: &str = "DSP Rack  Target: ";
const TRACK_CONTROL: &str = "[Track]";
const TARGET_SEPARATOR: &str = " ";
const MASTER_CONTROL: &str = "[Master]";

pub(super) fn render_dsp_rack_view(
    frame: &mut Frame<'_>,
    area: Rect,
    rack: Option<DspRackViewState<'_>>,
    interactions: &mut InteractionMap,
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
    let header_block = Block::default().title(" Native DSP ").borders(Borders::ALL);
    let header_inner = header_block.inner(sections[0]);
    let header = Paragraph::new(Line::from(vec![
        theme::label_span(HEADER_PREFIX),
        dsp_target_control(
            TRACK_CONTROL,
            rack.selected_target == DspRackTargetView::Track,
        ),
        theme::muted_span(TARGET_SEPARATOR),
        dsp_target_control(
            MASTER_CONTROL,
            rack.selected_target == DspRackTargetView::Master,
        ),
        theme::muted_span("  |  "),
        theme::value_span(format!(
            "Track {:02} {}",
            rack.track_number, rack.track_name
        )),
        theme::muted_span("  |  Native audio DSP chain; tracker FX columns stay in pattern cells"),
    ]))
    .block(header_block);
    frame.render_widget(header, sections[0]);
    register_dsp_target_controls(interactions, header_inner);

    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(sections[1]);
    render_dsp_chain(
        frame,
        columns[0],
        DspChainView {
            title: format!(" Track {:02}: {} ", rack.track_number, rack.track_name),
            effects: rack.track_effects,
            selected: rack.selected_target == DspRackTargetView::Track,
            selected_index: rack.selected_index,
            target: DspRackChain::Track,
        },
        interactions,
    );
    render_dsp_chain(
        frame,
        columns[1],
        DspChainView {
            title: " Master ".to_string(),
            effects: rack.master_effects,
            selected: rack.selected_target == DspRackTargetView::Master,
            selected_index: rack.selected_index,
            target: DspRackChain::Master,
        },
        interactions,
    );
    render_dsp_parameter_panel(frame, sections[2], rack, interactions);
    if let Some(palette) = rack.device_palette {
        render_dsp_device_palette(frame, area, palette, rack.selected_target, interactions);
    }
}

struct DspChainView<'a> {
    title: String,
    effects: &'a [EffectDevice],
    selected: bool,
    selected_index: usize,
    target: DspRackChain,
}

fn render_dsp_chain(
    frame: &mut Frame<'_>,
    area: Rect,
    chain: DspChainView<'_>,
    interactions: &mut InteractionMap,
) {
    let block = Block::default().title(chain.title).borders(Borders::ALL);
    let inner = block.inner(area);
    interactions.register(interaction_region::DSP_CHAIN, area);
    let mut lines = Vec::new();
    if chain.effects.is_empty() {
        lines.push(Line::from(Span::styled("  Empty chain", theme::muted())));
        lines.push(Line::from(Span::styled(
            "  A add native DSP; :fx edits tracker cell FX",
            theme::muted(),
        )));
    } else {
        for (index, effect) in chain.effects.iter().enumerate().take(inner.height as usize) {
            let is_selected =
                chain.selected && index == chain.selected_index.min(chain.effects.len() - 1);
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
            interactions.register_with_payload(
                interaction_region::DSP_DEVICE_ROW,
                Rect::new(
                    inner.x,
                    inner.y.saturating_add(index as u16),
                    inner.width,
                    1,
                ),
                InteractionPayload::DspDeviceRow {
                    target: chain.target,
                    index,
                },
            );
        }
    }
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn dsp_target_control(label: &'static str, selected: bool) -> Span<'static> {
    let style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        theme::base()
    };
    Span::styled(label, style)
}

fn register_dsp_target_controls(interactions: &mut InteractionMap, inner: Rect) {
    let track_offset = HEADER_PREFIX.len() as u16;
    register_dsp_target_control(
        interactions,
        inner,
        track_offset,
        TRACK_CONTROL.len() as u16,
        DspRackChain::Track,
    );
    let master_offset = track_offset
        .saturating_add(TRACK_CONTROL.len() as u16)
        .saturating_add(TARGET_SEPARATOR.len() as u16);
    register_dsp_target_control(
        interactions,
        inner,
        master_offset,
        MASTER_CONTROL.len() as u16,
        DspRackChain::Master,
    );
}

fn register_dsp_target_control(
    interactions: &mut InteractionMap,
    inner: Rect,
    offset: u16,
    width: u16,
    target: DspRackChain,
) {
    if inner.height == 0 || offset.saturating_add(width) > inner.width {
        return;
    }
    interactions.register_with_payload(
        interaction_region::DSP_RACK_TARGET,
        Rect::new(inner.x.saturating_add(offset), inner.y, width, 1),
        InteractionPayload::DspRackTarget { target },
    );
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
    interactions: &mut InteractionMap,
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
    let block = Block::default()
        .title(format!(" Add DSP Device -> {target} "))
        .borders(Borders::ALL);
    let inner = block.inner(overlay);
    let visible_entries = inner.height as usize;
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
        interactions.register_with_payload(
            interaction_region::DSP_PALETTE_ENTRY,
            Rect::new(
                inner.x,
                inner.y.saturating_add(index as u16),
                inner.width,
                1,
            ),
            InteractionPayload::DspPaletteEntry {
                index: absolute_index,
            },
        );
    }
    frame.render_widget(Clear, overlay);
    frame.render_widget(Paragraph::new(lines).block(block), overlay);
}
