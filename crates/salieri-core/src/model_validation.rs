use std::collections::HashSet;

use crate::{
    model::{
        AutomationTarget, EffectDevice, EffectDeviceKind, MixerState, Pattern, SampleId,
        SamplePlaybackMode, SamplePlaybackSettings, TrackId, ValidationError,
    },
    parameters::{
        mixer_master_gain_descriptor, mixer_send_gain_descriptor, mixer_track_gain_descriptor,
        mixer_track_pan_descriptor, native_balance_descriptor, native_gain_descriptor,
        native_pan_descriptor, native_width_descriptor, sample_gain_descriptor,
    },
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
    if !envelope.attack_seconds.is_finite()
        || envelope.attack_seconds < 0.0
        || !envelope.decay_seconds.is_finite()
        || envelope.decay_seconds < 0.0
        || !envelope.release_seconds.is_finite()
        || envelope.release_seconds < 0.0
        || !envelope.sustain.is_finite()
        || !(0.0..=1.0).contains(&envelope.sustain)
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
        for send in &track.sends {
            if !mixer_send_gain_descriptor().validate_f32(send.gain) {
                return Err(ValidationError::InvalidMixerTrackGain {
                    track_id: track.track,
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
            EffectDeviceKind::Gain { .. }
            | EffectDeviceKind::Pan { .. }
            | EffectDeviceKind::Balance { .. }
            | EffectDeviceKind::StereoWidth { .. }
            | EffectDeviceKind::PhaseInvert { .. } => {}
        }
    }
    Ok(())
}
