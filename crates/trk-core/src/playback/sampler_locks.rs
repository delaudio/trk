use crate::{
    builtin_parameter_descriptor, ParameterId, ParameterLockAction, ParameterLockTarget,
    ParameterValue, PatternCell, SamplePlaybackMode, SamplePlaybackSettings,
    SAMPLE_END_FRAME_PARAMETER_ID, SAMPLE_ENVELOPE_ATTACK_PARAMETER_ID,
    SAMPLE_ENVELOPE_DECAY_PARAMETER_ID, SAMPLE_ENVELOPE_RELEASE_PARAMETER_ID,
    SAMPLE_ENVELOPE_SUSTAIN_PARAMETER_ID, SAMPLE_LOOP_END_FRAME_PARAMETER_ID,
    SAMPLE_LOOP_START_FRAME_PARAMETER_ID, SAMPLE_PLAYBACK_MODE_PARAMETER_ID,
    SAMPLE_START_FRAME_PARAMETER_ID,
};

pub(super) fn locked_sample_note(
    cell: &PatternCell,
    target: &ParameterLockTarget,
    parameter: &str,
    default: u8,
) -> u8 {
    let Some(value) = locked_parameter_value(cell, target, parameter) else {
        return default;
    };
    let parameter = ParameterId::from(parameter);
    match builtin_parameter_descriptor(&parameter)
        .map_or_else(|| value.clone(), |descriptor| descriptor.clamp(value))
    {
        ParameterValue::Note(value) => value,
        _ => default,
    }
}

pub(super) fn locked_sample_playback(
    cell: &PatternCell,
    target: &ParameterLockTarget,
    mut playback: SamplePlaybackSettings,
) -> SamplePlaybackSettings {
    playback.mode = match locked_parameter_value(cell, target, SAMPLE_PLAYBACK_MODE_PARAMETER_ID) {
        Some(ParameterValue::Enum(value)) => sample_playback_mode(value).unwrap_or(playback.mode),
        _ => playback.mode,
    };
    playback.start_frame = locked_optional_frame(
        cell,
        target,
        SAMPLE_START_FRAME_PARAMETER_ID,
        playback.start_frame,
    );
    playback.end_frame = locked_optional_frame(
        cell,
        target,
        SAMPLE_END_FRAME_PARAMETER_ID,
        playback.end_frame,
    );
    playback.loop_start_frame = locked_optional_frame(
        cell,
        target,
        SAMPLE_LOOP_START_FRAME_PARAMETER_ID,
        playback.loop_start_frame,
    );
    playback.loop_end_frame = locked_optional_frame(
        cell,
        target,
        SAMPLE_LOOP_END_FRAME_PARAMETER_ID,
        playback.loop_end_frame,
    );
    playback.envelope.attack_seconds = locked_sample_float(
        cell,
        target,
        SAMPLE_ENVELOPE_ATTACK_PARAMETER_ID,
        playback.envelope.attack_seconds,
    );
    playback.envelope.decay_seconds = locked_sample_float(
        cell,
        target,
        SAMPLE_ENVELOPE_DECAY_PARAMETER_ID,
        playback.envelope.decay_seconds,
    );
    playback.envelope.sustain = locked_sample_float(
        cell,
        target,
        SAMPLE_ENVELOPE_SUSTAIN_PARAMETER_ID,
        playback.envelope.sustain,
    );
    playback.envelope.release_seconds = locked_sample_float(
        cell,
        target,
        SAMPLE_ENVELOPE_RELEASE_PARAMETER_ID,
        playback.envelope.release_seconds,
    );
    playback
}

fn locked_parameter_value<'a>(
    cell: &'a PatternCell,
    target: &ParameterLockTarget,
    parameter: &str,
) -> Option<&'a ParameterValue> {
    let parameter = ParameterId::from(parameter);
    match cell
        .parameter_locks
        .iter()
        .rfind(|lock| &lock.target == target && lock.parameter == parameter)
        .map(|lock| &lock.action)
    {
        Some(ParameterLockAction::Set { value }) => Some(value),
        Some(ParameterLockAction::Reset) | None => None,
    }
}

fn locked_optional_frame(
    cell: &PatternCell,
    target: &ParameterLockTarget,
    parameter: &str,
    default: Option<usize>,
) -> Option<usize> {
    let Some(value) = locked_parameter_value(cell, target, parameter) else {
        return default;
    };
    let parameter = ParameterId::from(parameter);
    let value = builtin_parameter_descriptor(&parameter)
        .map_or_else(|| value.clone(), |descriptor| descriptor.clamp(value));
    match value {
        ParameterValue::Integer(value) => usize::try_from(value).ok(),
        _ => default,
    }
}

pub(super) fn locked_sample_float(
    cell: &PatternCell,
    target: &ParameterLockTarget,
    parameter: &str,
    default: f32,
) -> f32 {
    let Some(value) = locked_parameter_value(cell, target, parameter) else {
        return default;
    };
    let parameter = ParameterId::from(parameter);
    builtin_parameter_descriptor(&parameter)
        .map_or_else(|| value.clone(), |descriptor| descriptor.clamp(value))
        .as_f32()
        .unwrap_or(default)
}

fn sample_playback_mode(value: &str) -> Option<SamplePlaybackMode> {
    match value {
        "oneShot" => Some(SamplePlaybackMode::OneShot),
        "loop" => Some(SamplePlaybackMode::Loop),
        "forwardLoop" => Some(SamplePlaybackMode::ForwardLoop),
        "backwardLoop" => Some(SamplePlaybackMode::BackwardLoop),
        "pingPongLoop" => Some(SamplePlaybackMode::PingPongLoop),
        "reverse" => Some(SamplePlaybackMode::Reverse),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_sampler_mode_is_inert() {
        assert_eq!(sample_playback_mode("futureMode"), None);
        assert_eq!(
            sample_playback_mode("reverse"),
            Some(SamplePlaybackMode::Reverse)
        );
    }

    #[test]
    fn sample_locks_are_cell_local_and_root_notes_are_clamped() {
        let target = ParameterLockTarget::Sample {
            sample: crate::SampleId(1),
        };
        let locked = PatternCell {
            parameter_locks: vec![
                crate::ParameterLock {
                    target: target.clone(),
                    parameter: ParameterId::from(crate::SAMPLE_GAIN_PARAMETER_ID),
                    action: ParameterLockAction::Set {
                        value: ParameterValue::Float(0.25),
                    },
                },
                crate::ParameterLock {
                    target: target.clone(),
                    parameter: ParameterId::from(crate::SAMPLE_ROOT_NOTE_PARAMETER_ID),
                    action: ParameterLockAction::Set {
                        value: ParameterValue::Note(u8::MAX),
                    },
                },
            ],
            ..PatternCell::default()
        };
        let unlocked = PatternCell::default();

        assert_eq!(
            locked_sample_float(&locked, &target, crate::SAMPLE_GAIN_PARAMETER_ID, 1.0),
            0.25
        );
        assert_eq!(
            locked_sample_float(&unlocked, &target, crate::SAMPLE_GAIN_PARAMETER_ID, 1.0),
            1.0
        );
        assert_eq!(
            locked_sample_note(&locked, &target, crate::SAMPLE_ROOT_NOTE_PARAMETER_ID, 60),
            127
        );
    }
}
