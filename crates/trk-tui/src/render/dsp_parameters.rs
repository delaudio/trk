use ratatui::{
    layout::Rect,
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Paragraph},
};
use trk_core::{
    native_balance_descriptor, native_bitcrusher_bit_depth_descriptor,
    native_bitcrusher_mix_descriptor, native_bitcrusher_reduction_descriptor,
    native_chorus_depth_descriptor, native_chorus_mix_descriptor, native_chorus_rate_descriptor,
    native_compressor_attack_descriptor, native_compressor_mix_descriptor,
    native_compressor_ratio_descriptor, native_compressor_release_descriptor,
    native_compressor_threshold_descriptor, native_delay_feedback_descriptor,
    native_delay_mix_descriptor, native_delay_ping_pong_descriptor, native_delay_sync_descriptor,
    native_delay_time_left_descriptor, native_delay_time_right_descriptor,
    native_drive_drive_descriptor, native_drive_mix_descriptor, native_drive_mode_descriptor,
    native_drive_tone_descriptor, native_filter_cutoff_descriptor, native_filter_drive_descriptor,
    native_filter_mix_descriptor, native_filter_mode_descriptor,
    native_filter_resonance_descriptor, native_flanger_depth_descriptor,
    native_flanger_feedback_descriptor, native_flanger_manual_descriptor,
    native_flanger_mix_descriptor, native_flanger_rate_descriptor, native_gain_descriptor,
    native_gate_attack_descriptor, native_gate_hysteresis_descriptor,
    native_gate_release_descriptor, native_gate_threshold_descriptor,
    native_limiter_ceiling_descriptor, native_limiter_input_gain_descriptor,
    native_limiter_lookahead_descriptor, native_limiter_release_descriptor, native_pan_descriptor,
    native_phase_invert_left_descriptor, native_phase_invert_right_descriptor,
    native_phaser_center_descriptor, native_phaser_depth_descriptor, native_phaser_mix_descriptor,
    native_phaser_rate_descriptor, native_phaser_stages_descriptor, native_reverb_decay_descriptor,
    native_reverb_mix_descriptor, native_reverb_predelay_descriptor, native_reverb_size_descriptor,
    native_width_descriptor, EffectDevice, EffectDeviceKind, ParameterDescriptor, ParameterValue,
};

use super::{
    centered_scroll_offset, parameter_flags_label, parameter_meter, theme, truncate,
    DspParameterLockStatusView, DspRackTargetView, DspRackViewState,
};
use crate::{interaction_region, InteractionMap, InteractionPayload};

pub(super) fn render_dsp_parameter_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    rack: DspRackViewState<'_>,
    interactions: &mut InteractionMap,
) {
    let Some(effect) = selected_dsp_effect(&rack) else {
        let paragraph = Paragraph::new(Line::from(vec![
            Span::styled("Select or add a DSP device", theme::muted()),
            Span::styled("  A Add Device", theme::muted()),
        ]))
        .block(Block::default().title(" Parameters ").borders(Borders::ALL));
        frame.render_widget(paragraph, area);
        return;
    };

    let target = match rack.selected_target {
        DspRackTargetView::Track => format!("Track {:02}", rack.track_number),
        DspRackTargetView::Master => "Master".to_string(),
    };
    let lines = dsp_parameter_lines(
        &effect.kind,
        rack.selected_parameter_index,
        rack.selected_lock_status,
    );
    let parameter_count = lines.len();
    let block = Block::default()
        .title(format!(
            " Parameters: {target} {:02} {} ",
            rack.selected_index + 1,
            effect.name
        ))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    let visible_parameters = usize::from(inner.height.saturating_sub(1));
    let start = centered_scroll_offset(
        parameter_count,
        rack.selected_parameter_index,
        visible_parameters,
    );
    let end = start
        .saturating_add(visible_parameters)
        .min(parameter_count);
    let mut visible_lines = lines[start..end].to_vec();
    visible_lines.push(Line::from(Span::styled(
        "P lock current value   R reset row   C clear row   [/] select   Left/Right adjust",
        theme::muted(),
    )));
    for (visible_index, index) in (start..end).enumerate() {
        interactions.register_with_payload(
            interaction_region::DSP_PARAMETER_ROW,
            Rect::new(
                inner.x,
                inner.y.saturating_add(visible_index as u16),
                inner.width,
                1,
            ),
            InteractionPayload::DspParameterRow { index },
        );
    }
    let paragraph = Paragraph::new(visible_lines).block(block);
    frame.render_widget(paragraph, area);
}

fn selected_dsp_effect<'a>(rack: &DspRackViewState<'a>) -> Option<&'a EffectDevice> {
    let effects = match rack.selected_target {
        DspRackTargetView::Track => rack.track_effects,
        DspRackTargetView::Master => rack.master_effects,
    };
    effects.get(rack.selected_index.min(effects.len().saturating_sub(1)))
}

fn bool_label(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

fn dsp_parameter_lines(
    kind: &EffectDeviceKind,
    selected_index: usize,
    lock_status: DspParameterLockStatusView,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match kind {
        EffectDeviceKind::Gain { gain } => {
            push_dsp_parameter_from_f32(&mut lines, native_gain_descriptor(), *gain, selected_index)
        }
        EffectDeviceKind::Pan { pan } => {
            push_dsp_parameter_from_f32(&mut lines, native_pan_descriptor(), *pan, selected_index);
        }
        EffectDeviceKind::Balance { balance } => push_dsp_parameter_from_f32(
            &mut lines,
            native_balance_descriptor(),
            *balance,
            selected_index,
        ),
        EffectDeviceKind::StereoWidth { width } => push_dsp_parameter_from_f32(
            &mut lines,
            native_width_descriptor(),
            *width,
            selected_index,
        ),
        EffectDeviceKind::PhaseInvert {
            invert_left,
            invert_right,
        } => {
            push_dsp_parameter(
                &mut lines,
                native_phase_invert_left_descriptor(),
                ParameterValue::Bool(*invert_left),
                selected_index,
            );
            push_dsp_parameter(
                &mut lines,
                native_phase_invert_right_descriptor(),
                ParameterValue::Bool(*invert_right),
                selected_index,
            );
        }
        EffectDeviceKind::Filter {
            mode,
            cutoff_hz,
            resonance,
            drive_db,
            mix,
            ..
        } => {
            push_dsp_parameter(
                &mut lines,
                native_filter_mode_descriptor(),
                ParameterValue::Enum(mode.parameter_id().to_string()),
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_filter_cutoff_descriptor(),
                *cutoff_hz,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_filter_resonance_descriptor(),
                *resonance,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_filter_drive_descriptor(),
                *drive_db,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_filter_mix_descriptor(),
                *mix,
                selected_index,
            );
        }
        EffectDeviceKind::Delay {
            sync,
            time_left_ms,
            time_right_ms,
            feedback,
            ping_pong,
            mix,
            ..
        } => {
            push_dsp_parameter(
                &mut lines,
                native_delay_sync_descriptor(),
                ParameterValue::Bool(*sync),
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_delay_time_left_descriptor(),
                *time_left_ms,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_delay_time_right_descriptor(),
                *time_right_ms,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_delay_feedback_descriptor(),
                *feedback,
                selected_index,
            );
            push_dsp_parameter(
                &mut lines,
                native_delay_ping_pong_descriptor(),
                ParameterValue::Bool(*ping_pong),
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_delay_mix_descriptor(),
                *mix,
                selected_index,
            );
        }
        EffectDeviceKind::Reverb {
            size,
            predelay_ms,
            decay_s,
            mix,
            ..
        } => {
            push_dsp_parameter_from_f32(
                &mut lines,
                native_reverb_size_descriptor(),
                *size,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_reverb_predelay_descriptor(),
                *predelay_ms,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_reverb_decay_descriptor(),
                *decay_s,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_reverb_mix_descriptor(),
                *mix,
                selected_index,
            );
        }
        EffectDeviceKind::Drive {
            mode,
            drive_db,
            tone,
            mix,
            ..
        } => {
            push_dsp_parameter(
                &mut lines,
                native_drive_mode_descriptor(),
                ParameterValue::Enum(mode.parameter_id().to_string()),
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_drive_drive_descriptor(),
                *drive_db,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_drive_tone_descriptor(),
                *tone,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_drive_mix_descriptor(),
                *mix,
                selected_index,
            );
        }
        EffectDeviceKind::Bitcrusher {
            bit_depth,
            reduction_ratio,
            dither,
            mix,
            ..
        } => {
            push_dsp_parameter(
                &mut lines,
                native_bitcrusher_bit_depth_descriptor(),
                ParameterValue::Integer(i64::from(*bit_depth)),
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_bitcrusher_reduction_descriptor(),
                *reduction_ratio,
                selected_index,
            );
            push_dsp_plain_parameter(&mut lines, "Dither", bool_label(*dither), selected_index);
            push_dsp_parameter_from_f32(
                &mut lines,
                native_bitcrusher_mix_descriptor(),
                *mix,
                selected_index,
            );
        }
        EffectDeviceKind::Chorus {
            rate_hz,
            depth,
            voices,
            mix,
            ..
        } => {
            push_dsp_parameter_from_f32(
                &mut lines,
                native_chorus_rate_descriptor(),
                *rate_hz,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_chorus_depth_descriptor(),
                *depth,
                selected_index,
            );
            push_dsp_plain_parameter(&mut lines, "Voices", &voices.to_string(), selected_index);
            push_dsp_parameter_from_f32(
                &mut lines,
                native_chorus_mix_descriptor(),
                *mix,
                selected_index,
            );
        }
        EffectDeviceKind::Flanger {
            rate_hz,
            depth,
            manual,
            feedback,
            mix,
            ..
        } => {
            push_dsp_parameter_from_f32(
                &mut lines,
                native_flanger_rate_descriptor(),
                *rate_hz,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_flanger_depth_descriptor(),
                *depth,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_flanger_manual_descriptor(),
                *manual,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_flanger_feedback_descriptor(),
                *feedback,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_flanger_mix_descriptor(),
                *mix,
                selected_index,
            );
        }
        EffectDeviceKind::Phaser {
            rate_hz,
            depth,
            center_hz,
            stages,
            mix,
            ..
        } => {
            push_dsp_parameter_from_f32(
                &mut lines,
                native_phaser_rate_descriptor(),
                *rate_hz,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_phaser_depth_descriptor(),
                *depth,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_phaser_center_descriptor(),
                *center_hz,
                selected_index,
            );
            push_dsp_parameter(
                &mut lines,
                native_phaser_stages_descriptor(),
                ParameterValue::Integer(i64::from(*stages)),
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_phaser_mix_descriptor(),
                *mix,
                selected_index,
            );
        }
        EffectDeviceKind::Compressor {
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            mix,
            ..
        } => {
            push_dsp_parameter_from_f32(
                &mut lines,
                native_compressor_threshold_descriptor(),
                *threshold_db,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_compressor_ratio_descriptor(),
                *ratio,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_compressor_attack_descriptor(),
                *attack_ms,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_compressor_release_descriptor(),
                *release_ms,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_compressor_mix_descriptor(),
                *mix,
                selected_index,
            );
        }
        EffectDeviceKind::Gate {
            threshold_db,
            hysteresis_db,
            attack_ms,
            release_ms,
            ..
        } => {
            push_dsp_parameter_from_f32(
                &mut lines,
                native_gate_threshold_descriptor(),
                *threshold_db,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_gate_hysteresis_descriptor(),
                *hysteresis_db,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_gate_attack_descriptor(),
                *attack_ms,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_gate_release_descriptor(),
                *release_ms,
                selected_index,
            );
        }
        EffectDeviceKind::Limiter {
            ceiling_db,
            input_gain_db,
            release_ms,
            lookahead_ms,
            ..
        } => {
            push_dsp_parameter_from_f32(
                &mut lines,
                native_limiter_ceiling_descriptor(),
                *ceiling_db,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_limiter_input_gain_descriptor(),
                *input_gain_db,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_limiter_release_descriptor(),
                *release_ms,
                selected_index,
            );
            push_dsp_parameter_from_f32(
                &mut lines,
                native_limiter_lookahead_descriptor(),
                *lookahead_ms,
                selected_index,
            );
        }
    }
    if let Some(line) = lines.get_mut(selected_index) {
        line.spans.push(Span::styled(
            format!(" {}", dsp_lock_status_label(lock_status)),
            dsp_parameter_style(true),
        ));
    }
    lines
}

fn dsp_lock_status_label(status: DspParameterLockStatusView) -> &'static str {
    match status {
        DspParameterLockStatusView::Unlocked => "row: chain",
        DspParameterLockStatusView::Set => "row: locked",
        DspParameterLockStatusView::Reset => "row: reset",
    }
}

fn push_dsp_parameter_from_f32(
    lines: &mut Vec<Line<'static>>,
    descriptor: ParameterDescriptor,
    value: f32,
    selected_index: usize,
) {
    let value = descriptor.value_from_f32(value);
    push_dsp_parameter(lines, descriptor, value, selected_index);
}

fn push_dsp_parameter(
    lines: &mut Vec<Line<'static>>,
    descriptor: ParameterDescriptor,
    value: ParameterValue,
    selected_index: usize,
) {
    let index = lines.len();
    let label = descriptor
        .short_name
        .as_deref()
        .unwrap_or(descriptor.name.as_str());
    let value_label = if descriptor.validate(&value).is_ok() {
        descriptor.format_value(&value)
    } else {
        format!(
            "invalid -> {}",
            descriptor.format_value(&descriptor.clamp(&value))
        )
    };
    let style = dsp_parameter_style(index == selected_index);
    lines.push(Line::from(vec![
        Span::styled(
            format!(
                "{}{:02} ",
                if index == selected_index { ">" } else { " " },
                index + 1
            ),
            style,
        ),
        Span::styled(format!("{:<12}", truncate(label, 12)), style),
        Span::styled(format!("{:>12} ", truncate(&value_label, 12)), style),
        Span::styled(parameter_meter(&descriptor, &value, 18), style),
        Span::styled(format!(" {}", parameter_flags_label(&descriptor)), style),
    ]));
}

fn push_dsp_plain_parameter(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: &str,
    selected_index: usize,
) {
    let index = lines.len();
    let style = dsp_parameter_style(index == selected_index);
    lines.push(Line::from(Span::styled(
        format!(
            "{}{:02} {:<12}{:>12}",
            if index == selected_index { ">" } else { " " },
            index + 1,
            truncate(label, 12),
            truncate(value, 12)
        ),
        style,
    )));
}

fn dsp_parameter_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        theme::base()
    }
}
