use std::collections::HashSet;

use crate::{
    model::{
        AutomationTarget, EffectDevice, EffectDeviceKind, MixerState, Pattern, SampleId,
        SamplePlaybackMode, SamplePlaybackSettings, TrackId, ValidationError,
    },
    parameters::*,
};

pub(crate) fn validate_sample_playback_settings(
    sample_index: usize,
    settings: SamplePlaybackSettings,
) -> Result<(), ValidationError> {
    if let (Some(start_frame), Some(end_frame)) = (settings.start_frame, settings.end_frame) {
        if start_frame >= end_frame {
            return Err(ValidationError::InvalidSampleFrameWindow { sample_index });
        }
    }
    match (
        settings.mode,
        settings.loop_start_frame,
        settings.loop_end_frame,
    ) {
        (SamplePlaybackMode::Loop, Some(loop_start), Some(loop_end)) if loop_start < loop_end => {}
        (SamplePlaybackMode::Loop, _, _) => {
            return Err(ValidationError::InvalidSampleLoopWindow { sample_index });
        }
        (_, Some(loop_start), Some(loop_end)) if loop_start < loop_end => {}
        (_, Some(_), Some(_)) | (_, Some(_), None) | (_, None, Some(_)) => {
            return Err(ValidationError::InvalidSampleLoopWindow { sample_index });
        }
        (_, None, None) => {}
    }
    let envelope = settings.envelope;
    if !sample_envelope_attack_descriptor().validate_f32(envelope.attack_seconds)
        || !sample_envelope_decay_descriptor().validate_f32(envelope.decay_seconds)
        || !sample_envelope_release_descriptor().validate_f32(envelope.release_seconds)
        || !sample_envelope_sustain_descriptor().validate_f32(envelope.sustain)
    {
        return Err(ValidationError::InvalidSampleEnvelope { sample_index });
    }
    Ok(())
}

pub(crate) fn validate_pattern_automation(
    pattern_index: usize,
    pattern: &Pattern,
    sample_ids: &HashSet<SampleId>,
) -> Result<(), ValidationError> {
    let mut targets = HashSet::new();
    for lane in &pattern.automation {
        if !targets.insert(lane.target) {
            return Err(ValidationError::DuplicateAutomationLane {
                pattern_index,
                target: lane.target,
            });
        }
        match lane.target {
            AutomationTarget::SampleGain { sample } if !sample_ids.contains(&sample) => {
                return Err(ValidationError::AutomationSampleNotFound {
                    pattern_index,
                    sample_id: sample,
                });
            }
            AutomationTarget::SampleGain { .. } => {}
        }

        let mut rows = HashSet::new();
        for point in &lane.points {
            if point.row >= pattern.rows.len() {
                return Err(ValidationError::AutomationRowOutOfBounds {
                    pattern_index,
                    row: point.row,
                });
            }
            if !rows.insert(point.row) {
                return Err(ValidationError::DuplicateAutomationPoint {
                    pattern_index,
                    row: point.row,
                });
            }
            if !sample_gain_descriptor().validate_f32(point.value) {
                return Err(ValidationError::InvalidAutomationValue {
                    pattern_index,
                    row: point.row,
                });
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_mixer(
    mixer: &MixerState,
    track_ids: &HashSet<TrackId>,
) -> Result<(), ValidationError> {
    if !mixer_master_gain_descriptor().validate_f32(mixer.master_gain) {
        return Err(ValidationError::InvalidMixerMasterGain);
    }

    let mut mixer_sends = HashSet::new();
    for send in &mixer.sends {
        if send.name.trim().is_empty() {
            return Err(ValidationError::EmptyMixerSend { send_id: send.id });
        }
        if !mixer_sends.insert(send.id) {
            return Err(ValidationError::DuplicateMixerSend { send_id: send.id });
        }
        validate_effect_chain(&send.effects)?;
    }

    let mut mixer_tracks = HashSet::new();
    for track in &mixer.tracks {
        if !track_ids.contains(&track.track) {
            return Err(ValidationError::MixerTrackNotFound {
                track_id: track.track,
            });
        }
        if !mixer_tracks.insert(track.track) {
            return Err(ValidationError::DuplicateMixerTrack {
                track_id: track.track,
            });
        }
        if !mixer_track_gain_descriptor().validate_f32(track.gain) {
            return Err(ValidationError::InvalidMixerTrackGain {
                track_id: track.track,
            });
        }
        if !mixer_track_pan_descriptor().validate_f32(track.pan) {
            return Err(ValidationError::InvalidMixerTrackPan {
                track_id: track.track,
            });
        }
        let mut track_sends = HashSet::new();
        for send in &track.sends {
            if !mixer_sends.contains(&send.send) {
                return Err(ValidationError::MixerSendNotFound {
                    track_id: track.track,
                    send_id: send.send,
                });
            }
            if !track_sends.insert(send.send) {
                return Err(ValidationError::DuplicateTrackSend {
                    track_id: track.track,
                    send_id: send.send,
                });
            }
            if !mixer_send_gain_descriptor().validate_f32(send.gain) {
                return Err(ValidationError::InvalidMixerSendGain {
                    track_id: track.track,
                    send_id: send.send,
                });
            }
        }
        validate_effect_chain(&track.effects)?;
    }
    validate_effect_chain(&mixer.master_effects)?;
    for track_id in track_ids {
        if !mixer_tracks.contains(track_id) {
            return Err(ValidationError::MixerTrackMissing {
                track_id: *track_id,
            });
        }
    }
    Ok(())
}

fn validate_effect_chain(effects: &[EffectDevice]) -> Result<(), ValidationError> {
    let mut ids = HashSet::new();
    for effect in effects {
        if !ids.insert(effect.id) {
            return Err(ValidationError::DuplicateEffectDevice {
                device_id: effect.id,
            });
        }
        match effect.kind {
            EffectDeviceKind::Gain { gain } if !native_gain_descriptor().validate_f32(gain) => {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Pan { pan } if !native_pan_descriptor().validate_f32(pan) => {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Balance { balance }
                if !native_balance_descriptor().validate_f32(balance) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::StereoWidth { width }
                if !native_width_descriptor().validate_f32(width) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Filter {
                cutoff_hz,
                resonance,
                drive_db,
                key_track,
                env_amount,
                mix,
                ..
            } if !native_filter_cutoff_descriptor().validate_f32(cutoff_hz)
                || !native_filter_resonance_descriptor().validate_f32(resonance)
                || !native_filter_drive_descriptor().validate_f32(drive_db)
                || !native_filter_key_track_descriptor().validate_f32(key_track)
                || !native_filter_env_amount_descriptor().validate_f32(env_amount)
                || !native_filter_mix_descriptor().validate_f32(mix) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Delay {
                time_left_ms,
                time_right_ms,
                feedback,
                filter_low_cut_hz,
                filter_high_cut_hz,
                mod_rate_hz,
                mod_depth,
                mix,
                output_db,
                ..
            } if !native_delay_time_left_descriptor().validate_f32(time_left_ms)
                || !native_delay_time_right_descriptor().validate_f32(time_right_ms)
                || !native_delay_feedback_descriptor().validate_f32(feedback)
                || !native_delay_filter_low_cut_descriptor().validate_f32(filter_low_cut_hz)
                || !native_delay_filter_high_cut_descriptor().validate_f32(filter_high_cut_hz)
                || filter_low_cut_hz > filter_high_cut_hz
                || !native_delay_mod_rate_descriptor().validate_f32(mod_rate_hz)
                || !native_delay_mod_depth_descriptor().validate_f32(mod_depth)
                || !native_delay_mix_descriptor().validate_f32(mix)
                || !native_delay_output_descriptor().validate_f32(output_db) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Reverb {
                size,
                predelay_ms,
                decay_s,
                damping,
                low_cut_hz,
                high_cut_hz,
                diffusion,
                width,
                early_reflections,
                mix,
                output_db,
            } if !native_reverb_size_descriptor().validate_f32(size)
                || !native_reverb_predelay_descriptor().validate_f32(predelay_ms)
                || !native_reverb_decay_descriptor().validate_f32(decay_s)
                || !native_reverb_damping_descriptor().validate_f32(damping)
                || !native_reverb_low_cut_descriptor().validate_f32(low_cut_hz)
                || !native_reverb_high_cut_descriptor().validate_f32(high_cut_hz)
                || low_cut_hz > high_cut_hz
                || !native_reverb_diffusion_descriptor().validate_f32(diffusion)
                || !native_reverb_width_descriptor().validate_f32(width)
                || !native_reverb_early_reflections_descriptor()
                    .validate_f32(early_reflections)
                || !native_reverb_mix_descriptor().validate_f32(mix)
                || !native_reverb_output_descriptor().validate_f32(output_db) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Drive {
                drive_db,
                tone,
                bias,
                mix,
                output_db,
                ..
            } if !native_drive_drive_descriptor().validate_f32(drive_db)
                || !native_drive_tone_descriptor().validate_f32(tone)
                || !native_drive_bias_descriptor().validate_f32(bias)
                || !native_drive_mix_descriptor().validate_f32(mix)
                || !native_drive_output_descriptor().validate_f32(output_db) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Bitcrusher {
                bit_depth,
                reduction_ratio,
                mix,
                output_db,
                ..
            } if native_bitcrusher_bit_depth_descriptor()
                .validate(&crate::ParameterValue::Integer(i64::from(bit_depth)))
                .is_err()
                || !native_bitcrusher_reduction_descriptor().validate_f32(reduction_ratio)
                || !native_bitcrusher_mix_descriptor().validate_f32(mix)
                || !native_bitcrusher_output_descriptor().validate_f32(output_db) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Chorus {
                rate_hz,
                depth,
                delay_ms,
                voices,
                spread,
                feedback,
                mix,
                output_db,
                ..
            } if !native_chorus_rate_descriptor().validate_f32(rate_hz)
                || !native_chorus_depth_descriptor().validate_f32(depth)
                || !native_chorus_delay_descriptor().validate_f32(delay_ms)
                || native_chorus_voices_descriptor()
                    .validate(&crate::ParameterValue::Integer(i64::from(voices)))
                    .is_err()
                || !native_chorus_spread_descriptor().validate_f32(spread)
                || !native_chorus_feedback_descriptor().validate_f32(feedback)
                || !native_chorus_mix_descriptor().validate_f32(mix)
                || !native_chorus_output_descriptor().validate_f32(output_db) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Flanger {
                rate_hz,
                depth,
                manual,
                delay_ms,
                feedback,
                stereo_phase,
                mix,
                output_db,
                ..
            } if !native_flanger_rate_descriptor().validate_f32(rate_hz)
                || !native_flanger_depth_descriptor().validate_f32(depth)
                || !native_flanger_manual_descriptor().validate_f32(manual)
                || !native_flanger_delay_descriptor().validate_f32(delay_ms)
                || !native_flanger_feedback_descriptor().validate_f32(feedback)
                || !native_flanger_stereo_phase_descriptor().validate_f32(stereo_phase)
                || !native_flanger_mix_descriptor().validate_f32(mix)
                || !native_flanger_output_descriptor().validate_f32(output_db) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Phaser {
                rate_hz,
                depth,
                center_hz,
                stages,
                feedback,
                stereo_phase,
                mix,
                output_db,
                ..
            } if !native_phaser_rate_descriptor().validate_f32(rate_hz)
                || !native_phaser_depth_descriptor().validate_f32(depth)
                || !native_phaser_center_descriptor().validate_f32(center_hz)
                || native_phaser_stages_descriptor()
                    .validate(&crate::ParameterValue::Integer(i64::from(stages)))
                    .is_err()
                || !native_phaser_feedback_descriptor().validate_f32(feedback)
                || !native_phaser_stereo_phase_descriptor().validate_f32(stereo_phase)
                || !native_phaser_mix_descriptor().validate_f32(mix)
                || !native_phaser_output_descriptor().validate_f32(output_db) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Compressor {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                knee_db,
                makeup_db,
                stereo_link,
                mix,
                ..
            } if !native_compressor_threshold_descriptor().validate_f32(threshold_db)
                || !native_compressor_ratio_descriptor().validate_f32(ratio)
                || !native_compressor_attack_descriptor().validate_f32(attack_ms)
                || !native_compressor_release_descriptor().validate_f32(release_ms)
                || !native_compressor_knee_descriptor().validate_f32(knee_db)
                || !native_compressor_makeup_descriptor().validate_f32(makeup_db)
                || !native_compressor_stereo_link_descriptor().validate_f32(stereo_link)
                || !native_compressor_mix_descriptor().validate_f32(mix) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Gate {
                threshold_db,
                hysteresis_db,
                attack_ms,
                hold_ms,
                release_ms,
                range_db,
                stereo_link,
                ..
            } if !native_gate_threshold_descriptor().validate_f32(threshold_db)
                || !native_gate_hysteresis_descriptor().validate_f32(hysteresis_db)
                || !native_gate_attack_descriptor().validate_f32(attack_ms)
                || !native_gate_hold_descriptor().validate_f32(hold_ms)
                || !native_gate_release_descriptor().validate_f32(release_ms)
                || !native_gate_range_descriptor().validate_f32(range_db)
                || !native_gate_stereo_link_descriptor().validate_f32(stereo_link) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Limiter {
                ceiling_db,
                input_gain_db,
                release_ms,
                lookahead_ms,
                stereo_link,
                ..
            } if !native_limiter_ceiling_descriptor().validate_f32(ceiling_db)
                || !native_limiter_input_gain_descriptor().validate_f32(input_gain_db)
                || !native_limiter_release_descriptor().validate_f32(release_ms)
                || !native_limiter_lookahead_descriptor().validate_f32(lookahead_ms)
                || !native_limiter_stereo_link_descriptor().validate_f32(stereo_link) =>
            {
                return Err(ValidationError::InvalidEffectParameter);
            }
            EffectDeviceKind::Gain { .. }
            | EffectDeviceKind::Pan { .. }
            | EffectDeviceKind::Balance { .. }
            | EffectDeviceKind::StereoWidth { .. }
            | EffectDeviceKind::PhaseInvert { .. }
            | EffectDeviceKind::Filter { .. }
            | EffectDeviceKind::Delay { .. }
            | EffectDeviceKind::Reverb { .. }
            | EffectDeviceKind::Drive { .. }
            | EffectDeviceKind::Bitcrusher { .. }
            | EffectDeviceKind::Chorus { .. }
            | EffectDeviceKind::Flanger { .. }
            | EffectDeviceKind::Phaser { .. }
            | EffectDeviceKind::Compressor { .. }
            | EffectDeviceKind::Gate { .. }
            | EffectDeviceKind::Limiter { .. } => {}
        }
    }
    Ok(())
}
