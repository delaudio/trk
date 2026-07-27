use crate::{
    model::{EditError, EffectDeviceKind},
    native_module::*,
    parameters::*,
    DynamicsDetector,
};

pub(super) fn dynamics_parameter_value(id: &str, kind: EffectDeviceKind) -> Option<ParameterValue> {
    match (id, kind) {
        (
            NATIVE_COMPRESSOR_THRESHOLD_PARAMETER_ID,
            EffectDeviceKind::Compressor { threshold_db, .. },
        )
        | (NATIVE_GATE_THRESHOLD_PARAMETER_ID, EffectDeviceKind::Gate { threshold_db, .. }) => {
            Some(ParameterValue::Decibels(threshold_db))
        }
        (NATIVE_COMPRESSOR_RATIO_PARAMETER_ID, EffectDeviceKind::Compressor { ratio, .. }) => {
            Some(ParameterValue::Ratio(ratio))
        }
        (NATIVE_COMPRESSOR_ATTACK_PARAMETER_ID, EffectDeviceKind::Compressor { attack_ms, .. })
        | (NATIVE_GATE_ATTACK_PARAMETER_ID, EffectDeviceKind::Gate { attack_ms, .. }) => {
            Some(ParameterValue::Float(attack_ms))
        }
        (
            NATIVE_COMPRESSOR_RELEASE_PARAMETER_ID,
            EffectDeviceKind::Compressor { release_ms, .. },
        )
        | (NATIVE_GATE_RELEASE_PARAMETER_ID, EffectDeviceKind::Gate { release_ms, .. })
        | (NATIVE_LIMITER_RELEASE_PARAMETER_ID, EffectDeviceKind::Limiter { release_ms, .. }) => {
            Some(ParameterValue::Float(release_ms))
        }
        (NATIVE_COMPRESSOR_KNEE_PARAMETER_ID, EffectDeviceKind::Compressor { knee_db, .. }) => {
            Some(ParameterValue::Decibels(knee_db))
        }
        (NATIVE_COMPRESSOR_MAKEUP_PARAMETER_ID, EffectDeviceKind::Compressor { makeup_db, .. }) => {
            Some(ParameterValue::Decibels(makeup_db))
        }
        (
            NATIVE_COMPRESSOR_AUTO_MAKEUP_PARAMETER_ID,
            EffectDeviceKind::Compressor { auto_makeup, .. },
        ) => Some(ParameterValue::Bool(auto_makeup)),
        (
            NATIVE_COMPRESSOR_DETECTOR_PARAMETER_ID,
            EffectDeviceKind::Compressor { detector, .. },
        )
        | (NATIVE_GATE_DETECTOR_PARAMETER_ID, EffectDeviceKind::Gate { detector, .. }) => {
            Some(ParameterValue::Enum(detector.parameter_id().to_string()))
        }
        (
            NATIVE_COMPRESSOR_STEREO_LINK_PARAMETER_ID,
            EffectDeviceKind::Compressor { stereo_link, .. },
        )
        | (NATIVE_GATE_STEREO_LINK_PARAMETER_ID, EffectDeviceKind::Gate { stereo_link, .. })
        | (
            NATIVE_LIMITER_STEREO_LINK_PARAMETER_ID,
            EffectDeviceKind::Limiter { stereo_link, .. },
        ) => Some(ParameterValue::Percentage(stereo_link)),
        (NATIVE_COMPRESSOR_MIX_PARAMETER_ID, EffectDeviceKind::Compressor { mix, .. }) => {
            Some(ParameterValue::Percentage(mix))
        }
        (NATIVE_GATE_HYSTERESIS_PARAMETER_ID, EffectDeviceKind::Gate { hysteresis_db, .. }) => {
            Some(ParameterValue::Decibels(hysteresis_db))
        }
        (NATIVE_GATE_HOLD_PARAMETER_ID, EffectDeviceKind::Gate { hold_ms, .. }) => {
            Some(ParameterValue::Float(hold_ms))
        }
        (NATIVE_GATE_RANGE_PARAMETER_ID, EffectDeviceKind::Gate { range_db, .. }) => {
            Some(ParameterValue::Decibels(range_db))
        }
        (NATIVE_LIMITER_CEILING_PARAMETER_ID, EffectDeviceKind::Limiter { ceiling_db, .. }) => {
            Some(ParameterValue::Decibels(ceiling_db))
        }
        (
            NATIVE_LIMITER_INPUT_GAIN_PARAMETER_ID,
            EffectDeviceKind::Limiter { input_gain_db, .. },
        ) => Some(ParameterValue::Decibels(input_gain_db)),
        (NATIVE_LIMITER_LOOKAHEAD_PARAMETER_ID, EffectDeviceKind::Limiter { lookahead_ms, .. }) => {
            Some(ParameterValue::Float(lookahead_ms))
        }
        (NATIVE_LIMITER_TRUE_PEAK_PARAMETER_ID, EffectDeviceKind::Limiter { true_peak, .. }) => {
            Some(ParameterValue::Bool(true_peak))
        }
        (NATIVE_COMPRESSOR_GAIN_REDUCTION_PARAMETER_ID, EffectDeviceKind::Compressor { .. })
        | (NATIVE_LIMITER_GAIN_REDUCTION_PARAMETER_ID, EffectDeviceKind::Limiter { .. }) => {
            Some(ParameterValue::Decibels(0.0))
        }
        (NATIVE_GATE_STATE_PARAMETER_ID, EffectDeviceKind::Gate { .. }) => {
            Some(ParameterValue::Bool(false))
        }
        _ => None,
    }
}

pub(super) fn set_dynamics_parameter_value(
    id: &str,
    kind: &mut EffectDeviceKind,
    value: ParameterValue,
) -> Result<bool, EditError> {
    match (id, kind) {
        (
            NATIVE_COMPRESSOR_AUTO_MAKEUP_PARAMETER_ID,
            EffectDeviceKind::Compressor { auto_makeup, .. },
        )
        | (
            NATIVE_LIMITER_TRUE_PEAK_PARAMETER_ID,
            EffectDeviceKind::Limiter {
                true_peak: auto_makeup,
                ..
            },
        ) => {
            set_bool(id, auto_makeup, value)?;
        }
        (
            NATIVE_COMPRESSOR_DETECTOR_PARAMETER_ID,
            EffectDeviceKind::Compressor { detector, .. },
        )
        | (NATIVE_GATE_DETECTOR_PARAMETER_ID, EffectDeviceKind::Gate { detector, .. }) => {
            descriptor(id)?
                .validate(&value)
                .map_err(|_| EditError::InvalidParameterValue)?;
            let ParameterValue::Enum(value) = value else {
                return Err(EditError::InvalidParameterValue);
            };
            *detector = DynamicsDetector::from_parameter_id(&value)
                .ok_or(EditError::InvalidParameterValue)?;
        }
        (
            NATIVE_COMPRESSOR_THRESHOLD_PARAMETER_ID,
            EffectDeviceKind::Compressor { threshold_db, .. },
        )
        | (NATIVE_GATE_THRESHOLD_PARAMETER_ID, EffectDeviceKind::Gate { threshold_db, .. }) => {
            set_numeric(id, threshold_db, value)?
        }
        (NATIVE_COMPRESSOR_RATIO_PARAMETER_ID, EffectDeviceKind::Compressor { ratio, .. }) => {
            set_numeric(id, ratio, value)?
        }
        (NATIVE_COMPRESSOR_ATTACK_PARAMETER_ID, EffectDeviceKind::Compressor { attack_ms, .. })
        | (NATIVE_GATE_ATTACK_PARAMETER_ID, EffectDeviceKind::Gate { attack_ms, .. }) => {
            set_numeric(id, attack_ms, value)?
        }
        (
            NATIVE_COMPRESSOR_RELEASE_PARAMETER_ID,
            EffectDeviceKind::Compressor { release_ms, .. },
        )
        | (NATIVE_GATE_RELEASE_PARAMETER_ID, EffectDeviceKind::Gate { release_ms, .. })
        | (NATIVE_LIMITER_RELEASE_PARAMETER_ID, EffectDeviceKind::Limiter { release_ms, .. }) => {
            set_numeric(id, release_ms, value)?
        }
        (NATIVE_COMPRESSOR_KNEE_PARAMETER_ID, EffectDeviceKind::Compressor { knee_db, .. }) => {
            set_numeric(id, knee_db, value)?
        }
        (NATIVE_COMPRESSOR_MAKEUP_PARAMETER_ID, EffectDeviceKind::Compressor { makeup_db, .. }) => {
            set_numeric(id, makeup_db, value)?
        }
        (
            NATIVE_COMPRESSOR_STEREO_LINK_PARAMETER_ID,
            EffectDeviceKind::Compressor { stereo_link, .. },
        )
        | (NATIVE_GATE_STEREO_LINK_PARAMETER_ID, EffectDeviceKind::Gate { stereo_link, .. })
        | (
            NATIVE_LIMITER_STEREO_LINK_PARAMETER_ID,
            EffectDeviceKind::Limiter { stereo_link, .. },
        ) => set_numeric(id, stereo_link, value)?,
        (NATIVE_COMPRESSOR_MIX_PARAMETER_ID, EffectDeviceKind::Compressor { mix, .. }) => {
            set_numeric(id, mix, value)?
        }
        (NATIVE_GATE_HYSTERESIS_PARAMETER_ID, EffectDeviceKind::Gate { hysteresis_db, .. }) => {
            set_numeric(id, hysteresis_db, value)?
        }
        (NATIVE_GATE_HOLD_PARAMETER_ID, EffectDeviceKind::Gate { hold_ms, .. }) => {
            set_numeric(id, hold_ms, value)?
        }
        (NATIVE_GATE_RANGE_PARAMETER_ID, EffectDeviceKind::Gate { range_db, .. }) => {
            set_numeric(id, range_db, value)?
        }
        (NATIVE_LIMITER_CEILING_PARAMETER_ID, EffectDeviceKind::Limiter { ceiling_db, .. }) => {
            set_numeric(id, ceiling_db, value)?
        }
        (
            NATIVE_LIMITER_INPUT_GAIN_PARAMETER_ID,
            EffectDeviceKind::Limiter { input_gain_db, .. },
        ) => set_numeric(id, input_gain_db, value)?,
        (NATIVE_LIMITER_LOOKAHEAD_PARAMETER_ID, EffectDeviceKind::Limiter { lookahead_ms, .. }) => {
            set_numeric(id, lookahead_ms, value)?
        }
        _ => return Ok(false),
    }
    Ok(true)
}

pub(super) fn dynamics_native_module_state(
    kind: EffectDeviceKind,
) -> Option<(NativeModuleId, Vec<NativeModuleParameter>)> {
    let (module, descriptors) = match kind {
        EffectDeviceKind::Compressor { .. } => (
            NativeModuleId::from(NATIVE_COMPRESSOR_MODULE_ID),
            native_compressor_parameter_descriptors(),
        ),
        EffectDeviceKind::Gate { .. } => (
            NativeModuleId::from(NATIVE_GATE_MODULE_ID),
            native_gate_parameter_descriptors(),
        ),
        EffectDeviceKind::Limiter { .. } => (
            NativeModuleId::from(NATIVE_LIMITER_MODULE_ID),
            native_limiter_parameter_descriptors(),
        ),
        _ => return None,
    };
    Some((
        module,
        descriptors
            .into_iter()
            .filter_map(|descriptor| {
                dynamics_parameter_value(descriptor.id.as_str(), kind).map(|value| {
                    NativeModuleParameter {
                        id: descriptor.id,
                        value,
                    }
                })
            })
            .collect(),
    ))
}

fn descriptor(id: &str) -> Result<ParameterDescriptor, EditError> {
    builtin_parameter_descriptor(&ParameterId::from(id)).ok_or(EditError::UnknownParameter)
}

fn set_numeric(id: &str, target: &mut f32, value: ParameterValue) -> Result<(), EditError> {
    descriptor(id)?
        .validate(&value)
        .map_err(|_| EditError::InvalidParameterValue)?;
    *target = value.as_f32().ok_or(EditError::InvalidParameterValue)?;
    Ok(())
}

fn set_bool(id: &str, target: &mut bool, value: ParameterValue) -> Result<(), EditError> {
    descriptor(id)?
        .validate(&value)
        .map_err(|_| EditError::InvalidParameterValue)?;
    let ParameterValue::Bool(value) = value else {
        return Err(EditError::InvalidParameterValue);
    };
    *target = value;
    Ok(())
}
