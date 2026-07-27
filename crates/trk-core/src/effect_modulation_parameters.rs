use crate::{
    model::{EditError, EffectDeviceKind},
    native_module::*,
    parameters::*,
};

macro_rules! value_cases {
    ($id:expr, $kind:expr, [$(($pid:ident, $variant:ident, $field:ident, $value:expr)),+ $(,)?]) => {
        match ($id, $kind) {
            $(( $pid, EffectDeviceKind::$variant { $field, .. }) => Some($value($field)),)+
            _ => None,
        }
    };
}

pub(super) fn modulation_parameter_value(
    id: &str,
    kind: EffectDeviceKind,
) -> Option<ParameterValue> {
    value_cases!(
        id,
        kind,
        [
            (
                NATIVE_CHORUS_RATE_PARAMETER_ID,
                Chorus,
                rate_hz,
                ParameterValue::FrequencyHertz
            ),
            (
                NATIVE_CHORUS_SYNC_PARAMETER_ID,
                Chorus,
                sync,
                ParameterValue::Bool
            ),
            (
                NATIVE_CHORUS_DEPTH_PARAMETER_ID,
                Chorus,
                depth,
                ParameterValue::Percentage
            ),
            (
                NATIVE_CHORUS_DELAY_PARAMETER_ID,
                Chorus,
                delay_ms,
                ParameterValue::Float
            ),
            (NATIVE_CHORUS_VOICES_PARAMETER_ID, Chorus, voices, |value| {
                ParameterValue::Integer(i64::from(value))
            }),
            (
                NATIVE_CHORUS_SPREAD_PARAMETER_ID,
                Chorus,
                spread,
                ParameterValue::Percentage
            ),
            (
                NATIVE_CHORUS_FEEDBACK_PARAMETER_ID,
                Chorus,
                feedback,
                ParameterValue::Percentage
            ),
            (
                NATIVE_CHORUS_MIX_PARAMETER_ID,
                Chorus,
                mix,
                ParameterValue::Percentage
            ),
            (
                NATIVE_CHORUS_OUTPUT_PARAMETER_ID,
                Chorus,
                output_db,
                ParameterValue::Decibels
            ),
            (
                NATIVE_FLANGER_RATE_PARAMETER_ID,
                Flanger,
                rate_hz,
                ParameterValue::FrequencyHertz
            ),
            (
                NATIVE_FLANGER_SYNC_PARAMETER_ID,
                Flanger,
                sync,
                ParameterValue::Bool
            ),
            (
                NATIVE_FLANGER_DEPTH_PARAMETER_ID,
                Flanger,
                depth,
                ParameterValue::Percentage
            ),
            (
                NATIVE_FLANGER_MANUAL_PARAMETER_ID,
                Flanger,
                manual,
                ParameterValue::Percentage
            ),
            (
                NATIVE_FLANGER_DELAY_PARAMETER_ID,
                Flanger,
                delay_ms,
                ParameterValue::Float
            ),
            (
                NATIVE_FLANGER_FEEDBACK_PARAMETER_ID,
                Flanger,
                feedback,
                ParameterValue::Percentage
            ),
            (
                NATIVE_FLANGER_STEREO_PHASE_PARAMETER_ID,
                Flanger,
                stereo_phase,
                ParameterValue::Percentage
            ),
            (
                NATIVE_FLANGER_MIX_PARAMETER_ID,
                Flanger,
                mix,
                ParameterValue::Percentage
            ),
            (
                NATIVE_FLANGER_OUTPUT_PARAMETER_ID,
                Flanger,
                output_db,
                ParameterValue::Decibels
            ),
            (
                NATIVE_PHASER_RATE_PARAMETER_ID,
                Phaser,
                rate_hz,
                ParameterValue::FrequencyHertz
            ),
            (
                NATIVE_PHASER_SYNC_PARAMETER_ID,
                Phaser,
                sync,
                ParameterValue::Bool
            ),
            (
                NATIVE_PHASER_DEPTH_PARAMETER_ID,
                Phaser,
                depth,
                ParameterValue::Percentage
            ),
            (
                NATIVE_PHASER_CENTER_PARAMETER_ID,
                Phaser,
                center_hz,
                ParameterValue::FrequencyHertz
            ),
            (NATIVE_PHASER_STAGES_PARAMETER_ID, Phaser, stages, |value| {
                ParameterValue::Integer(i64::from(value))
            }),
            (
                NATIVE_PHASER_FEEDBACK_PARAMETER_ID,
                Phaser,
                feedback,
                ParameterValue::Percentage
            ),
            (
                NATIVE_PHASER_STEREO_PHASE_PARAMETER_ID,
                Phaser,
                stereo_phase,
                ParameterValue::Percentage
            ),
            (
                NATIVE_PHASER_MIX_PARAMETER_ID,
                Phaser,
                mix,
                ParameterValue::Percentage
            ),
            (
                NATIVE_PHASER_OUTPUT_PARAMETER_ID,
                Phaser,
                output_db,
                ParameterValue::Decibels
            ),
        ]
    )
}

pub(super) fn set_modulation_parameter_value(
    id: &str,
    kind: &mut EffectDeviceKind,
    value: ParameterValue,
) -> Result<bool, EditError> {
    match (id, kind) {
        (NATIVE_CHORUS_SYNC_PARAMETER_ID, EffectDeviceKind::Chorus { sync, .. })
        | (NATIVE_FLANGER_SYNC_PARAMETER_ID, EffectDeviceKind::Flanger { sync, .. })
        | (NATIVE_PHASER_SYNC_PARAMETER_ID, EffectDeviceKind::Phaser { sync, .. }) => {
            set_bool(sync, descriptor(id)?, value)?;
        }
        (NATIVE_CHORUS_VOICES_PARAMETER_ID, EffectDeviceKind::Chorus { voices, .. })
        | (NATIVE_PHASER_STAGES_PARAMETER_ID, EffectDeviceKind::Phaser { stages: voices, .. }) => {
            descriptor(id)?
                .validate(&value)
                .map_err(|_| EditError::InvalidParameterValue)?;
            let ParameterValue::Integer(value) = value else {
                return Err(EditError::InvalidParameterValue);
            };
            *voices = u8::try_from(value).map_err(|_| EditError::InvalidParameterValue)?;
        }
        (NATIVE_CHORUS_RATE_PARAMETER_ID, EffectDeviceKind::Chorus { rate_hz, .. })
        | (NATIVE_FLANGER_RATE_PARAMETER_ID, EffectDeviceKind::Flanger { rate_hz, .. })
        | (NATIVE_PHASER_RATE_PARAMETER_ID, EffectDeviceKind::Phaser { rate_hz, .. }) => {
            set_numeric(id, rate_hz, value)?
        }
        (NATIVE_CHORUS_DEPTH_PARAMETER_ID, EffectDeviceKind::Chorus { depth, .. })
        | (NATIVE_FLANGER_DEPTH_PARAMETER_ID, EffectDeviceKind::Flanger { depth, .. })
        | (NATIVE_PHASER_DEPTH_PARAMETER_ID, EffectDeviceKind::Phaser { depth, .. }) => {
            set_numeric(id, depth, value)?
        }
        (NATIVE_CHORUS_MIX_PARAMETER_ID, EffectDeviceKind::Chorus { mix, .. })
        | (NATIVE_FLANGER_MIX_PARAMETER_ID, EffectDeviceKind::Flanger { mix, .. })
        | (NATIVE_PHASER_MIX_PARAMETER_ID, EffectDeviceKind::Phaser { mix, .. }) => {
            set_numeric(id, mix, value)?
        }
        (NATIVE_CHORUS_OUTPUT_PARAMETER_ID, EffectDeviceKind::Chorus { output_db, .. })
        | (NATIVE_FLANGER_OUTPUT_PARAMETER_ID, EffectDeviceKind::Flanger { output_db, .. })
        | (NATIVE_PHASER_OUTPUT_PARAMETER_ID, EffectDeviceKind::Phaser { output_db, .. }) => {
            set_numeric(id, output_db, value)?
        }
        (NATIVE_CHORUS_DELAY_PARAMETER_ID, EffectDeviceKind::Chorus { delay_ms, .. })
        | (NATIVE_FLANGER_DELAY_PARAMETER_ID, EffectDeviceKind::Flanger { delay_ms, .. }) => {
            set_numeric(id, delay_ms, value)?
        }
        (NATIVE_CHORUS_FEEDBACK_PARAMETER_ID, EffectDeviceKind::Chorus { feedback, .. })
        | (NATIVE_FLANGER_FEEDBACK_PARAMETER_ID, EffectDeviceKind::Flanger { feedback, .. })
        | (NATIVE_PHASER_FEEDBACK_PARAMETER_ID, EffectDeviceKind::Phaser { feedback, .. }) => {
            set_numeric(id, feedback, value)?
        }
        (NATIVE_CHORUS_SPREAD_PARAMETER_ID, EffectDeviceKind::Chorus { spread, .. }) => {
            set_numeric(id, spread, value)?
        }
        (NATIVE_FLANGER_MANUAL_PARAMETER_ID, EffectDeviceKind::Flanger { manual, .. }) => {
            set_numeric(id, manual, value)?
        }
        (
            NATIVE_FLANGER_STEREO_PHASE_PARAMETER_ID,
            EffectDeviceKind::Flanger { stereo_phase, .. },
        )
        | (
            NATIVE_PHASER_STEREO_PHASE_PARAMETER_ID,
            EffectDeviceKind::Phaser { stereo_phase, .. },
        ) => set_numeric(id, stereo_phase, value)?,
        (NATIVE_PHASER_CENTER_PARAMETER_ID, EffectDeviceKind::Phaser { center_hz, .. }) => {
            set_numeric(id, center_hz, value)?
        }
        _ => return Ok(false),
    }
    Ok(true)
}

pub(super) fn modulation_native_module_state(
    kind: EffectDeviceKind,
) -> Option<(NativeModuleId, Vec<NativeModuleParameter>)> {
    let (module, parameters) = match kind {
        EffectDeviceKind::Chorus {
            rate_hz,
            sync,
            depth,
            delay_ms,
            voices,
            spread,
            feedback,
            mix,
            output_db,
        } => (
            NATIVE_CHORUS_MODULE_ID,
            vec![
                p(
                    NATIVE_CHORUS_RATE_PARAMETER_ID,
                    ParameterValue::FrequencyHertz(rate_hz),
                ),
                p(NATIVE_CHORUS_SYNC_PARAMETER_ID, ParameterValue::Bool(sync)),
                p(
                    NATIVE_CHORUS_DEPTH_PARAMETER_ID,
                    ParameterValue::Percentage(depth),
                ),
                p(
                    NATIVE_CHORUS_DELAY_PARAMETER_ID,
                    ParameterValue::Float(delay_ms),
                ),
                p(
                    NATIVE_CHORUS_VOICES_PARAMETER_ID,
                    ParameterValue::Integer(i64::from(voices)),
                ),
                p(
                    NATIVE_CHORUS_SPREAD_PARAMETER_ID,
                    ParameterValue::Percentage(spread),
                ),
                p(
                    NATIVE_CHORUS_FEEDBACK_PARAMETER_ID,
                    ParameterValue::Percentage(feedback),
                ),
                p(
                    NATIVE_CHORUS_MIX_PARAMETER_ID,
                    ParameterValue::Percentage(mix),
                ),
                p(
                    NATIVE_CHORUS_OUTPUT_PARAMETER_ID,
                    ParameterValue::Decibels(output_db),
                ),
            ],
        ),
        EffectDeviceKind::Flanger {
            rate_hz,
            sync,
            depth,
            manual,
            delay_ms,
            feedback,
            stereo_phase,
            mix,
            output_db,
        } => (
            NATIVE_FLANGER_MODULE_ID,
            vec![
                p(
                    NATIVE_FLANGER_RATE_PARAMETER_ID,
                    ParameterValue::FrequencyHertz(rate_hz),
                ),
                p(NATIVE_FLANGER_SYNC_PARAMETER_ID, ParameterValue::Bool(sync)),
                p(
                    NATIVE_FLANGER_DEPTH_PARAMETER_ID,
                    ParameterValue::Percentage(depth),
                ),
                p(
                    NATIVE_FLANGER_MANUAL_PARAMETER_ID,
                    ParameterValue::Percentage(manual),
                ),
                p(
                    NATIVE_FLANGER_DELAY_PARAMETER_ID,
                    ParameterValue::Float(delay_ms),
                ),
                p(
                    NATIVE_FLANGER_FEEDBACK_PARAMETER_ID,
                    ParameterValue::Percentage(feedback),
                ),
                p(
                    NATIVE_FLANGER_STEREO_PHASE_PARAMETER_ID,
                    ParameterValue::Percentage(stereo_phase),
                ),
                p(
                    NATIVE_FLANGER_MIX_PARAMETER_ID,
                    ParameterValue::Percentage(mix),
                ),
                p(
                    NATIVE_FLANGER_OUTPUT_PARAMETER_ID,
                    ParameterValue::Decibels(output_db),
                ),
            ],
        ),
        EffectDeviceKind::Phaser {
            rate_hz,
            sync,
            depth,
            center_hz,
            stages,
            feedback,
            stereo_phase,
            mix,
            output_db,
        } => (
            NATIVE_PHASER_MODULE_ID,
            vec![
                p(
                    NATIVE_PHASER_RATE_PARAMETER_ID,
                    ParameterValue::FrequencyHertz(rate_hz),
                ),
                p(NATIVE_PHASER_SYNC_PARAMETER_ID, ParameterValue::Bool(sync)),
                p(
                    NATIVE_PHASER_DEPTH_PARAMETER_ID,
                    ParameterValue::Percentage(depth),
                ),
                p(
                    NATIVE_PHASER_CENTER_PARAMETER_ID,
                    ParameterValue::FrequencyHertz(center_hz),
                ),
                p(
                    NATIVE_PHASER_STAGES_PARAMETER_ID,
                    ParameterValue::Integer(i64::from(stages)),
                ),
                p(
                    NATIVE_PHASER_FEEDBACK_PARAMETER_ID,
                    ParameterValue::Percentage(feedback),
                ),
                p(
                    NATIVE_PHASER_STEREO_PHASE_PARAMETER_ID,
                    ParameterValue::Percentage(stereo_phase),
                ),
                p(
                    NATIVE_PHASER_MIX_PARAMETER_ID,
                    ParameterValue::Percentage(mix),
                ),
                p(
                    NATIVE_PHASER_OUTPUT_PARAMETER_ID,
                    ParameterValue::Decibels(output_db),
                ),
            ],
        ),
        _ => return None,
    };
    Some((NativeModuleId::from(module), parameters))
}

fn descriptor(id: &str) -> Result<ParameterDescriptor, EditError> {
    builtin_parameter_descriptor(&ParameterId::from(id)).ok_or(EditError::UnknownParameter)
}

fn set_numeric(id: &str, target: &mut f32, value: ParameterValue) -> Result<(), EditError> {
    super::set_reverb_numeric(descriptor(id)?, target, value)
}

fn set_bool(
    target: &mut bool,
    descriptor: ParameterDescriptor,
    value: ParameterValue,
) -> Result<(), EditError> {
    descriptor
        .validate(&value)
        .map_err(|_| EditError::InvalidParameterValue)?;
    let ParameterValue::Bool(value) = value else {
        return Err(EditError::InvalidParameterValue);
    };
    *target = value;
    Ok(())
}

fn p(id: &str, value: ParameterValue) -> NativeModuleParameter {
    NativeModuleParameter {
        id: ParameterId::from(id),
        value,
    }
}
