use crate::{
    model::{EditError, EffectDeviceKind},
    native_module::*,
    parameters::*,
    DriveMode,
};

pub(super) fn degradation_parameter_value(
    id: &str,
    kind: EffectDeviceKind,
) -> Option<ParameterValue> {
    match (id, kind) {
        (NATIVE_DRIVE_MODE_PARAMETER_ID, EffectDeviceKind::Drive { mode, .. }) => {
            Some(ParameterValue::Enum(mode.parameter_id().to_string()))
        }
        (NATIVE_DRIVE_DRIVE_PARAMETER_ID, EffectDeviceKind::Drive { drive_db, .. }) => {
            Some(ParameterValue::Decibels(drive_db))
        }
        (NATIVE_DRIVE_TONE_PARAMETER_ID, EffectDeviceKind::Drive { tone, .. }) => {
            Some(ParameterValue::Percentage(tone))
        }
        (NATIVE_DRIVE_BIAS_PARAMETER_ID, EffectDeviceKind::Drive { bias, .. }) => {
            Some(ParameterValue::Percentage(bias))
        }
        (NATIVE_DRIVE_MIX_PARAMETER_ID, EffectDeviceKind::Drive { mix, .. }) => {
            Some(ParameterValue::Percentage(mix))
        }
        (NATIVE_DRIVE_OUTPUT_PARAMETER_ID, EffectDeviceKind::Drive { output_db, .. }) => {
            Some(ParameterValue::Decibels(output_db))
        }
        (
            NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID,
            EffectDeviceKind::Bitcrusher { bit_depth, .. },
        ) => Some(ParameterValue::Integer(i64::from(bit_depth))),
        (
            NATIVE_BITCRUSHER_REDUCTION_PARAMETER_ID,
            EffectDeviceKind::Bitcrusher {
                reduction_ratio, ..
            },
        ) => Some(ParameterValue::Ratio(reduction_ratio)),
        (NATIVE_BITCRUSHER_DITHER_PARAMETER_ID, EffectDeviceKind::Bitcrusher { dither, .. }) => {
            Some(ParameterValue::Bool(dither))
        }
        (NATIVE_BITCRUSHER_MIX_PARAMETER_ID, EffectDeviceKind::Bitcrusher { mix, .. }) => {
            Some(ParameterValue::Percentage(mix))
        }
        (NATIVE_BITCRUSHER_OUTPUT_PARAMETER_ID, EffectDeviceKind::Bitcrusher { output_db, .. }) => {
            Some(ParameterValue::Decibels(output_db))
        }
        _ => None,
    }
}

pub(super) fn set_degradation_parameter_value(
    id: &str,
    kind: &mut EffectDeviceKind,
    value: ParameterValue,
) -> Result<bool, EditError> {
    match (id, kind) {
        (NATIVE_DRIVE_MODE_PARAMETER_ID, EffectDeviceKind::Drive { mode, .. }) => {
            native_drive_mode_descriptor()
                .validate(&value)
                .map_err(|_| EditError::InvalidParameterValue)?;
            let ParameterValue::Enum(value) = value else {
                return Err(EditError::InvalidParameterValue);
            };
            *mode = DriveMode::from_parameter_id(&value).ok_or(EditError::InvalidParameterValue)?;
        }
        (NATIVE_DRIVE_DRIVE_PARAMETER_ID, EffectDeviceKind::Drive { drive_db, .. }) => {
            super::set_reverb_numeric(native_drive_drive_descriptor(), drive_db, value)?;
        }
        (NATIVE_DRIVE_TONE_PARAMETER_ID, EffectDeviceKind::Drive { tone, .. }) => {
            super::set_reverb_numeric(native_drive_tone_descriptor(), tone, value)?;
        }
        (NATIVE_DRIVE_BIAS_PARAMETER_ID, EffectDeviceKind::Drive { bias, .. }) => {
            super::set_reverb_numeric(native_drive_bias_descriptor(), bias, value)?;
        }
        (NATIVE_DRIVE_MIX_PARAMETER_ID, EffectDeviceKind::Drive { mix, .. }) => {
            super::set_reverb_numeric(native_drive_mix_descriptor(), mix, value)?;
        }
        (NATIVE_DRIVE_OUTPUT_PARAMETER_ID, EffectDeviceKind::Drive { output_db, .. }) => {
            super::set_reverb_numeric(native_drive_output_descriptor(), output_db, value)?;
        }
        (
            NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID,
            EffectDeviceKind::Bitcrusher { bit_depth, .. },
        ) => {
            native_bitcrusher_bit_depth_descriptor()
                .validate(&value)
                .map_err(|_| EditError::InvalidParameterValue)?;
            let ParameterValue::Integer(value) = value else {
                return Err(EditError::InvalidParameterValue);
            };
            *bit_depth = u8::try_from(value).map_err(|_| EditError::InvalidParameterValue)?;
        }
        (
            NATIVE_BITCRUSHER_REDUCTION_PARAMETER_ID,
            EffectDeviceKind::Bitcrusher {
                reduction_ratio, ..
            },
        ) => {
            super::set_reverb_numeric(
                native_bitcrusher_reduction_descriptor(),
                reduction_ratio,
                value,
            )?;
        }
        (NATIVE_BITCRUSHER_DITHER_PARAMETER_ID, EffectDeviceKind::Bitcrusher { dither, .. }) => {
            native_bitcrusher_dither_descriptor()
                .validate(&value)
                .map_err(|_| EditError::InvalidParameterValue)?;
            let ParameterValue::Bool(value) = value else {
                return Err(EditError::InvalidParameterValue);
            };
            *dither = value;
        }
        (NATIVE_BITCRUSHER_MIX_PARAMETER_ID, EffectDeviceKind::Bitcrusher { mix, .. }) => {
            super::set_reverb_numeric(native_bitcrusher_mix_descriptor(), mix, value)?;
        }
        (NATIVE_BITCRUSHER_OUTPUT_PARAMETER_ID, EffectDeviceKind::Bitcrusher { output_db, .. }) => {
            super::set_reverb_numeric(native_bitcrusher_output_descriptor(), output_db, value)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

pub(super) fn degradation_native_module_state(
    kind: EffectDeviceKind,
) -> Option<(NativeModuleId, Vec<NativeModuleParameter>)> {
    match kind {
        EffectDeviceKind::Drive {
            mode,
            drive_db,
            tone,
            bias,
            mix,
            output_db,
        } => Some((
            NativeModuleId::from(NATIVE_DRIVE_MODULE_ID),
            vec![
                module_parameter(
                    NATIVE_DRIVE_MODE_PARAMETER_ID,
                    ParameterValue::Enum(mode.parameter_id().to_string()),
                ),
                module_parameter(
                    NATIVE_DRIVE_DRIVE_PARAMETER_ID,
                    ParameterValue::Decibels(drive_db),
                ),
                module_parameter(
                    NATIVE_DRIVE_TONE_PARAMETER_ID,
                    ParameterValue::Percentage(tone),
                ),
                module_parameter(
                    NATIVE_DRIVE_BIAS_PARAMETER_ID,
                    ParameterValue::Percentage(bias),
                ),
                module_parameter(
                    NATIVE_DRIVE_MIX_PARAMETER_ID,
                    ParameterValue::Percentage(mix),
                ),
                module_parameter(
                    NATIVE_DRIVE_OUTPUT_PARAMETER_ID,
                    ParameterValue::Decibels(output_db),
                ),
            ],
        )),
        EffectDeviceKind::Bitcrusher {
            bit_depth,
            reduction_ratio,
            dither,
            mix,
            output_db,
        } => Some((
            NativeModuleId::from(NATIVE_BITCRUSHER_MODULE_ID),
            vec![
                module_parameter(
                    NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID,
                    ParameterValue::Integer(i64::from(bit_depth)),
                ),
                module_parameter(
                    NATIVE_BITCRUSHER_REDUCTION_PARAMETER_ID,
                    ParameterValue::Ratio(reduction_ratio),
                ),
                module_parameter(
                    NATIVE_BITCRUSHER_DITHER_PARAMETER_ID,
                    ParameterValue::Bool(dither),
                ),
                module_parameter(
                    NATIVE_BITCRUSHER_MIX_PARAMETER_ID,
                    ParameterValue::Percentage(mix),
                ),
                module_parameter(
                    NATIVE_BITCRUSHER_OUTPUT_PARAMETER_ID,
                    ParameterValue::Decibels(output_db),
                ),
            ],
        )),
        _ => None,
    }
}

fn module_parameter(id: &str, value: ParameterValue) -> NativeModuleParameter {
    NativeModuleParameter {
        id: ParameterId::from(id),
        value,
    }
}
