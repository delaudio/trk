use super::*;

pub(crate) fn command_arguments(arguments: &[String]) -> Vec<&str> {
    arguments.iter().map(String::as_str).collect()
}

pub(crate) fn parse_optional_numbered_name(
    values: &[&str],
    default_index: usize,
) -> Option<(usize, String)> {
    let first = values.first()?;
    if let Ok(number) = first.parse::<usize>() {
        let name = values.get(1..)?.join(" ");
        Some((number.saturating_sub(1), name))
    } else {
        Some((default_index, values.join(" ")))
    }
}

pub(crate) fn parse_track_number(value: &str) -> Option<usize> {
    value
        .parse::<usize>()
        .ok()
        .map(|number| number.saturating_sub(1))
}

pub(crate) fn upsert_effect_device(chain: &mut Vec<EffectDevice>, device: EffectDevice) {
    if let Some(existing) = chain.iter_mut().find(|existing| existing.id == device.id) {
        *existing = device;
    } else {
        chain.push(device);
        chain.sort_by_key(|device| device.id);
    }
}

pub(crate) fn effect_device_is_valid(device: &EffectDevice) -> bool {
    device
        .native_module_state()
        .validate_against(&device.native_module_descriptor())
        .is_ok()
}

pub(crate) fn format_effect_device(device: &EffectDevice) -> String {
    match device.kind {
        EffectDeviceKind::Gain { gain } => format!("gain {gain:.3}"),
        EffectDeviceKind::Pan { pan } => format!("pan {pan:+.3}"),
        EffectDeviceKind::Balance { balance } => format!("balance {balance:+.3}"),
        EffectDeviceKind::StereoWidth { width } => format!("width {width:.3}"),
        EffectDeviceKind::PhaseInvert {
            invert_left,
            invert_right,
        } => format!("phase L:{} R:{}", invert_left as u8, invert_right as u8),
        EffectDeviceKind::Filter {
            mode,
            cutoff_hz,
            resonance,
            drive_db,
            mix,
            ..
        } => format!(
            "filter {} {cutoff_hz:.1}Hz res {resonance:.3} drive {drive_db:.1}dB mix {mix:.3}",
            mode.parameter_id()
        ),
        EffectDeviceKind::Delay {
            sync,
            time_left_ms,
            time_right_ms,
            feedback,
            ping_pong,
            mix,
            output_db,
            ..
        } => format!(
            "delay {} L {time_left_ms:.1}ms R {time_right_ms:.1}ms fb {feedback:.3} ping {} mix {mix:.3} out {output_db:+.1}dB",
            if sync { "sync" } else { "free" },
            ping_pong as u8
        ),
        EffectDeviceKind::Reverb {
            size,
            predelay_ms,
            decay_s,
            damping,
            mix,
            output_db,
            ..
        } => format!(
            "reverb size {size:.3} pre {predelay_ms:.1}ms decay {decay_s:.2}s damp {damping:.3} mix {mix:.3} out {output_db:+.1}dB"
        ),
        EffectDeviceKind::Drive {
            mode,
            drive_db,
            tone,
            bias,
            mix,
            output_db,
        } => format!(
            "drive {} {drive_db:.1}dB tone {tone:.3} bias {bias:+.3} mix {mix:.3} out {output_db:+.1}dB",
            mode.parameter_id()
        ),
        EffectDeviceKind::Bitcrusher {
            bit_depth,
            reduction_ratio,
            dither,
            mix,
            output_db,
        } => format!(
            "bitcrusher {bit_depth}bit rate x{reduction_ratio:.0} dither {} mix {mix:.3} out {output_db:+.1}dB",
            dither as u8
        ),
        EffectDeviceKind::Chorus {
            rate_hz,
            depth,
            delay_ms,
            voices,
            spread,
            mix,
            ..
        } => format!(
            "chorus {rate_hz:.2}Hz depth {depth:.3} delay {delay_ms:.1}ms voices {voices} spread {spread:.3} mix {mix:.3}"
        ),
        EffectDeviceKind::Flanger {
            rate_hz,
            depth,
            manual,
            feedback,
            stereo_phase,
            mix,
            ..
        } => format!(
            "flanger {rate_hz:.2}Hz depth {depth:.3} manual {manual:.3} fb {feedback:+.3} phase {stereo_phase:.3} mix {mix:.3}"
        ),
        EffectDeviceKind::Phaser {
            rate_hz,
            depth,
            center_hz,
            stages,
            feedback,
            stereo_phase,
            mix,
            ..
        } => format!(
            "phaser {rate_hz:.2}Hz depth {depth:.3} center {center_hz:.1}Hz stages {stages} fb {feedback:+.3} phase {stereo_phase:.3} mix {mix:.3}"
        ),
        EffectDeviceKind::Compressor {
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            knee_db,
            makeup_db,
            mix,
            ..
        } => format!(
            "compressor thr {threshold_db:+.1}dB ratio {ratio:.2}:1 atk {attack_ms:.1}ms rel {release_ms:.1}ms knee {knee_db:.1}dB makeup {makeup_db:+.1}dB mix {mix:.3}"
        ),
        EffectDeviceKind::Gate {
            threshold_db,
            hysteresis_db,
            attack_ms,
            hold_ms,
            release_ms,
            range_db,
            ..
        } => format!(
            "gate thr {threshold_db:+.1}dB hyst {hysteresis_db:.1}dB atk {attack_ms:.1}ms hold {hold_ms:.1}ms rel {release_ms:.1}ms range {range_db:.1}dB"
        ),
        EffectDeviceKind::Limiter {
            ceiling_db,
            input_gain_db,
            release_ms,
            lookahead_ms,
            ..
        } => format!(
            "limiter ceiling {ceiling_db:+.1}dB input {input_gain_db:+.1}dB rel {release_ms:.1}ms look {lookahead_ms:.1}ms"
        ),
    }
}

pub(crate) fn format_parameter_lock_target(target: &ParameterLockTarget) -> String {
    match target {
        ParameterLockTarget::Sample { sample } => format!("sample {:?}", sample),
        ParameterLockTarget::Instrument { instrument } => format!("instrument {:?}", instrument),
        ParameterLockTarget::TrackMixer { track } => format!("track mixer {:?}", track),
        ParameterLockTarget::MasterMixer => "master mixer".to_string(),
        ParameterLockTarget::TrackSend { track, send } => {
            format!("track {:?} send {send}", track)
        }
        ParameterLockTarget::SendBus { send } => format!("send bus {send}"),
        ParameterLockTarget::TrackEffect { track, device } => {
            format!("track {:?} device {device}", track)
        }
        ParameterLockTarget::MasterEffect { device } => format!("master device {device}"),
    }
}

pub(crate) fn format_ai_proposal_summary(
    proposal: &AiProposal,
    touched_cells: &[CellAddress],
) -> String {
    format!(
        "{}; touches {} cell(s): {}",
        proposal.summary,
        touched_cells.len(),
        format_touched_cells(touched_cells)
    )
}

pub(crate) fn format_touched_cells(touched_cells: &[CellAddress]) -> String {
    let mut cells = touched_cells
        .iter()
        .take(8)
        .map(|cell| {
            format!(
                "p{:02}/r{:02}/t{:02}",
                cell.pattern + 1,
                cell.row,
                cell.track + 1
            )
        })
        .collect::<Vec<_>>();
    if touched_cells.len() > cells.len() {
        cells.push(format!("+{}", touched_cells.len() - cells.len()));
    }
    if cells.is_empty() {
        "none".to_string()
    } else {
        cells.join(", ")
    }
}

pub(crate) fn parse_optional_frame_value(value: &str) -> Option<Option<usize>> {
    match value.to_ascii_lowercase().as_str() {
        "clear" | "none" | "off" => Some(None),
        _ => value.parse::<usize>().ok().map(Some),
    }
}

pub(crate) fn validate_sample_playback_settings(
    settings: SamplePlaybackSettings,
) -> Result<(), &'static str> {
    if let (Some(start), Some(end)) = (settings.start_frame, settings.end_frame) {
        if start >= end {
            return Err("Sample start must be before end");
        }
    }
    if settings.mode == SamplePlaybackMode::Loop {
        match (settings.loop_start_frame, settings.loop_end_frame) {
            (Some(start), Some(end)) if start < end => {}
            (Some(_), Some(_)) => return Err("Sample loop start must be before loop end"),
            _ => return Err("Sample loop requires start and end frames"),
        }
    }
    let envelope = settings.envelope;
    if !sample_envelope_attack_descriptor().validate_f32(envelope.attack_seconds)
        || !sample_envelope_decay_descriptor().validate_f32(envelope.decay_seconds)
        || !sample_envelope_release_descriptor().validate_f32(envelope.release_seconds)
        || !sample_envelope_sustain_descriptor().validate_f32(envelope.sustain)
    {
        return Err("Sample envelope requires A/D/R between 0 and 60s and sustain between 0 and 1");
    }
    Ok(())
}

pub(crate) fn format_optional_frame(frame: Option<usize>) -> String {
    frame.map_or_else(|| "clear".to_string(), |frame| frame.to_string())
}

pub(crate) fn format_sample_loop(settings: SamplePlaybackSettings) -> String {
    match (
        settings.mode,
        settings.loop_start_frame,
        settings.loop_end_frame,
    ) {
        (SamplePlaybackMode::Loop, Some(start), Some(end)) => format!("{start}-{end}"),
        _ => "off".to_string(),
    }
}

pub(crate) fn format_sample_envelope(envelope: SampleEnvelope) -> String {
    format!(
        "{:.3}/{:.3}/{:.3}/{:.3}",
        envelope.attack_seconds, envelope.decay_seconds, envelope.sustain, envelope.release_seconds
    )
}

pub(crate) fn format_sample_playback_settings(settings: SamplePlaybackSettings) -> String {
    format!(
        "Sample settings: mode={} start={} end={} loop={} env={}",
        match settings.mode {
            SamplePlaybackMode::OneShot => "one-shot",
            SamplePlaybackMode::Loop => "loop",
        },
        format_optional_frame(settings.start_frame),
        format_optional_frame(settings.end_frame),
        format_sample_loop(settings),
        format_sample_envelope(settings.envelope)
    )
}

pub(crate) fn sampler_envelope_field_label(field: SamplerEnvelopeField) -> &'static str {
    match field {
        SamplerEnvelopeField::Attack => "Attack",
        SamplerEnvelopeField::Decay => "Decay",
        SamplerEnvelopeField::Sustain => "Sustain",
        SamplerEnvelopeField::Release => "Release",
    }
}

pub(crate) fn adjust_sampler_envelope_seconds(value: f32, direction: f32, coarse: bool) -> f32 {
    let step = if coarse { 0.050 } else { 0.005 };
    let descriptor = sample_envelope_attack_descriptor();
    let value = descriptor.clamp(&descriptor.value_from_f32(value + direction * step));
    round_sampler_control(value.as_f32().unwrap_or(0.0))
}

pub(crate) fn adjust_sampler_sustain(value: f32, direction: f32, coarse: bool) -> f32 {
    let step = if coarse { 0.10 } else { 0.05 };
    round_sampler_control((value + direction * step).clamp(0.0, 1.0))
}

pub(crate) fn round_sampler_control(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

pub(crate) fn sample_waveform_visible_buckets(bucket_count: usize, zoom: usize) -> usize {
    if bucket_count == 0 {
        return 0;
    }
    bucket_count.div_ceil(zoom.max(1)).max(1)
}

pub(crate) fn parse_hex_byte(value: &str) -> Option<u8> {
    u8::from_str_radix(value.trim_start_matches("0x"), 16).ok()
}

pub(crate) fn parse_cell_byte(value: &str) -> Option<u8> {
    parse_hex_byte(value).or_else(|| value.parse::<u8>().ok())
}

pub(crate) fn current_cell_hex_value(cell: &PatternCell, field: CellField) -> Option<u8> {
    match field {
        CellField::Note => None,
        CellField::Velocity => cell.velocity,
        CellField::Instrument => cell.instrument.and_then(|instrument| {
            if instrument.0 <= u32::from(u8::MAX) {
                Some(instrument.0 as u8)
            } else {
                None
            }
        }),
        CellField::Volume => cell.volume,
        CellField::Pan => cell.pan,
        CellField::Delay => cell.delay,
        CellField::Effect => cell.command.map(|command| command.value),
        CellField::Effect2 => cell.command2.map(|command| command.value),
    }
}

pub(crate) fn set_current_cell_hex_value(cell: &mut PatternCell, field: CellField, value: u8) {
    match field {
        CellField::Note => {}
        CellField::Velocity => cell.velocity = Some(value.min(0x7f)),
        CellField::Instrument => {
            cell.instrument = if value == 0 {
                None
            } else {
                Some(InstrumentId(u32::from(value)))
            };
        }
        CellField::Volume => cell.volume = Some(value.min(0x7f)),
        CellField::Pan => cell.pan = Some(value.min(0x7f)),
        CellField::Delay => cell.delay = Some(value),
        CellField::Effect => {
            let code = cell
                .command
                .map_or(TrackerCommand::DELAY_CODE, |command| command.code);
            cell.command = Some(TrackerCommand { code, value });
        }
        CellField::Effect2 => {
            let code = cell
                .command2
                .map_or(TrackerCommand::DELAY_CODE, |command| command.code);
            cell.command2 = Some(TrackerCommand { code, value });
        }
    }
}

pub(crate) fn keyboard_note(key: char, octave: u8) -> Option<u8> {
    let (semitone, octave_offset) = match key.to_ascii_lowercase() {
        'z' => (0, 0),
        's' => (1, 0),
        'x' => (2, 0),
        'd' => (3, 0),
        'c' => (4, 0),
        'v' => (5, 0),
        'g' => (6, 0),
        'b' => (7, 0),
        'h' => (8, 0),
        'n' => (9, 0),
        'j' => (10, 0),
        'm' => (11, 0),
        'q' => (0, 1),
        '2' => (1, 1),
        'w' => (2, 1),
        '3' => (3, 1),
        'e' => (4, 1),
        'r' => (5, 1),
        '5' => (6, 1),
        't' => (7, 1),
        '6' => (8, 1),
        'y' => (9, 1),
        '7' => (10, 1),
        'u' => (11, 1),
        _ => return None,
    };

    let midi_octave = i16::from(octave) + octave_offset + 1;
    let pitch = midi_octave * 12 + semitone;
    u8::try_from(pitch).ok().filter(|pitch| *pitch <= 127)
}

pub(crate) fn find_midi_output_port<'a>(
    ports: &'a [MidiOutputPort],
    output_name: &str,
) -> Option<(usize, &'a MidiOutputPort)> {
    let needle = output_name.trim().to_lowercase();
    let normalized_needle = normalize_midi_port_name(output_name);
    if needle.is_empty() {
        return None;
    }

    ports
        .iter()
        .enumerate()
        .find(|(_, port)| port.name.eq_ignore_ascii_case(output_name.trim()))
        .or_else(|| {
            ports
                .iter()
                .enumerate()
                .find(|(_, port)| port.name.to_lowercase().contains(&needle))
        })
        .or_else(|| {
            ports.iter().enumerate().find(|(_, port)| {
                let normalized_name = normalize_midi_port_name(&port.name);
                normalized_name == normalized_needle
                    || normalized_name.contains(&normalized_needle)
                    || normalized_needle.contains(&normalized_name)
            })
        })
}

pub(crate) fn resolve_midi_output_port<'a>(
    ports: &'a [MidiOutputPort],
    output_name_or_index: &str,
) -> Option<(usize, &'a MidiOutputPort)> {
    let value = output_name_or_index.trim();
    value
        .parse::<usize>()
        .ok()
        .and_then(|index| {
            ports
                .iter()
                .enumerate()
                .find(|(_, port)| port.index == index)
        })
        .or_else(|| find_midi_output_port(ports, value))
}

pub(crate) fn find_midi_input_port<'a>(
    ports: &'a [MidiInputPort],
    input_name: &str,
) -> Option<(usize, &'a MidiInputPort)> {
    let needle = input_name.trim().to_lowercase();
    let normalized_needle = normalize_midi_port_name(input_name);
    if needle.is_empty() {
        return None;
    }

    ports
        .iter()
        .enumerate()
        .find(|(_, port)| port.name.eq_ignore_ascii_case(input_name.trim()))
        .or_else(|| {
            ports
                .iter()
                .enumerate()
                .find(|(_, port)| port.name.to_lowercase().contains(&needle))
        })
        .or_else(|| {
            ports.iter().enumerate().find(|(_, port)| {
                let normalized_name = normalize_midi_port_name(&port.name);
                normalized_name == normalized_needle
                    || normalized_name.contains(&normalized_needle)
                    || normalized_needle.contains(&normalized_name)
            })
        })
}

pub(crate) fn resolve_midi_input_port<'a>(
    ports: &'a [MidiInputPort],
    input_name_or_index: &str,
) -> Option<(usize, &'a MidiInputPort)> {
    let value = input_name_or_index.trim();
    value
        .parse::<usize>()
        .ok()
        .and_then(|index| {
            ports
                .iter()
                .enumerate()
                .find(|(_, port)| port.index == index)
        })
        .or_else(|| find_midi_input_port(ports, value))
}

pub(crate) fn normalize_midi_port_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
