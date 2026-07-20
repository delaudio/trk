use crate::{
    model::{EditError, EffectDevice, EffectDeviceKind},
    native_module::*,
    parameters::*,
    FilterMode,
};

#[path = "effect_degradation_parameters.rs"]
mod effect_degradation_parameters;
#[path = "effect_modulation_parameters.rs"]
mod effect_modulation_parameters;
use effect_degradation_parameters::{
    degradation_native_module_state, degradation_parameter_value, set_degradation_parameter_value,
};
use effect_modulation_parameters::{
    modulation_native_module_state, modulation_parameter_value, set_modulation_parameter_value,
};

impl EffectDevice {
    #[must_use]
    pub fn parameter_descriptors(&self) -> Vec<ParameterDescriptor> {
        match self.kind {
            EffectDeviceKind::Gain { .. } => vec![native_gain_descriptor()],
            EffectDeviceKind::Pan { .. } => vec![native_pan_descriptor()],
            EffectDeviceKind::Balance { .. } => vec![native_balance_descriptor()],
            EffectDeviceKind::StereoWidth { .. } => vec![native_width_descriptor()],
            EffectDeviceKind::PhaseInvert { .. } => vec![
                native_phase_invert_left_descriptor(),
                native_phase_invert_right_descriptor(),
            ],
            EffectDeviceKind::Filter { .. } => vec![
                native_filter_mode_descriptor(),
                native_filter_cutoff_descriptor(),
                native_filter_resonance_descriptor(),
                native_filter_drive_descriptor(),
                native_filter_key_track_descriptor(),
                native_filter_env_amount_descriptor(),
                native_filter_mix_descriptor(),
            ],
            EffectDeviceKind::Delay { .. } => vec![
                native_delay_sync_descriptor(),
                native_delay_time_left_descriptor(),
                native_delay_time_right_descriptor(),
                native_delay_link_times_descriptor(),
                native_delay_feedback_descriptor(),
                native_delay_ping_pong_descriptor(),
                native_delay_filter_low_cut_descriptor(),
                native_delay_filter_high_cut_descriptor(),
                native_delay_mod_rate_descriptor(),
                native_delay_mod_depth_descriptor(),
                native_delay_mix_descriptor(),
                native_delay_output_descriptor(),
            ],
            EffectDeviceKind::Reverb { .. } => native_reverb_parameter_descriptors(),
            EffectDeviceKind::Drive { .. } => native_drive_parameter_descriptors(),
            EffectDeviceKind::Bitcrusher { .. } => native_bitcrusher_parameter_descriptors(),
            EffectDeviceKind::Chorus { .. } => native_chorus_parameter_descriptors(),
            EffectDeviceKind::Flanger { .. } => native_flanger_parameter_descriptors(),
            EffectDeviceKind::Phaser { .. } => native_phaser_parameter_descriptors(),
        }
    }

    #[must_use]
    pub fn parameter_value(&self, id: &ParameterId) -> Option<ParameterValue> {
        match (id.as_str(), self.kind) {
            (NATIVE_GAIN_PARAMETER_ID, EffectDeviceKind::Gain { gain }) => {
                Some(ParameterValue::Float(gain))
            }
            (NATIVE_PAN_PARAMETER_ID, EffectDeviceKind::Pan { pan }) => {
                Some(ParameterValue::Bipolar(pan))
            }
            (NATIVE_BALANCE_PARAMETER_ID, EffectDeviceKind::Balance { balance }) => {
                Some(ParameterValue::Bipolar(balance))
            }
            (NATIVE_WIDTH_PARAMETER_ID, EffectDeviceKind::StereoWidth { width }) => {
                Some(ParameterValue::Percentage(width))
            }
            (
                NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID,
                EffectDeviceKind::PhaseInvert { invert_left, .. },
            ) => Some(ParameterValue::Bool(invert_left)),
            (
                NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID,
                EffectDeviceKind::PhaseInvert { invert_right, .. },
            ) => Some(ParameterValue::Bool(invert_right)),
            (NATIVE_FILTER_MODE_PARAMETER_ID, EffectDeviceKind::Filter { mode, .. }) => {
                Some(ParameterValue::Enum(mode.parameter_id().to_string()))
            }
            (NATIVE_FILTER_CUTOFF_PARAMETER_ID, EffectDeviceKind::Filter { cutoff_hz, .. }) => {
                Some(ParameterValue::FrequencyHertz(cutoff_hz))
            }
            (NATIVE_FILTER_RESONANCE_PARAMETER_ID, EffectDeviceKind::Filter { resonance, .. }) => {
                Some(ParameterValue::Normalized(resonance))
            }
            (NATIVE_FILTER_DRIVE_PARAMETER_ID, EffectDeviceKind::Filter { drive_db, .. }) => {
                Some(ParameterValue::Decibels(drive_db))
            }
            (NATIVE_FILTER_KEY_TRACK_PARAMETER_ID, EffectDeviceKind::Filter { key_track, .. }) => {
                Some(ParameterValue::Percentage(key_track))
            }
            (
                NATIVE_FILTER_ENV_AMOUNT_PARAMETER_ID,
                EffectDeviceKind::Filter { env_amount, .. },
            ) => Some(ParameterValue::Percentage(env_amount)),
            (NATIVE_FILTER_MIX_PARAMETER_ID, EffectDeviceKind::Filter { mix, .. }) => {
                Some(ParameterValue::Percentage(mix))
            }
            (NATIVE_DELAY_SYNC_PARAMETER_ID, EffectDeviceKind::Delay { sync, .. }) => {
                Some(ParameterValue::Bool(sync))
            }
            (NATIVE_DELAY_TIME_LEFT_PARAMETER_ID, EffectDeviceKind::Delay { time_left_ms, .. }) => {
                Some(ParameterValue::Float(time_left_ms))
            }
            (
                NATIVE_DELAY_TIME_RIGHT_PARAMETER_ID,
                EffectDeviceKind::Delay { time_right_ms, .. },
            ) => Some(ParameterValue::Float(time_right_ms)),
            (NATIVE_DELAY_LINK_TIMES_PARAMETER_ID, EffectDeviceKind::Delay { link_times, .. }) => {
                Some(ParameterValue::Bool(link_times))
            }
            (NATIVE_DELAY_FEEDBACK_PARAMETER_ID, EffectDeviceKind::Delay { feedback, .. }) => {
                Some(ParameterValue::Percentage(feedback))
            }
            (NATIVE_DELAY_PING_PONG_PARAMETER_ID, EffectDeviceKind::Delay { ping_pong, .. }) => {
                Some(ParameterValue::Bool(ping_pong))
            }
            (
                NATIVE_DELAY_FILTER_LOW_CUT_PARAMETER_ID,
                EffectDeviceKind::Delay {
                    filter_low_cut_hz, ..
                },
            ) => Some(ParameterValue::FrequencyHertz(filter_low_cut_hz)),
            (
                NATIVE_DELAY_FILTER_HIGH_CUT_PARAMETER_ID,
                EffectDeviceKind::Delay {
                    filter_high_cut_hz, ..
                },
            ) => Some(ParameterValue::FrequencyHertz(filter_high_cut_hz)),
            (NATIVE_DELAY_MOD_RATE_PARAMETER_ID, EffectDeviceKind::Delay { mod_rate_hz, .. }) => {
                Some(ParameterValue::FrequencyHertz(mod_rate_hz))
            }
            (NATIVE_DELAY_MOD_DEPTH_PARAMETER_ID, EffectDeviceKind::Delay { mod_depth, .. }) => {
                Some(ParameterValue::Percentage(mod_depth))
            }
            (NATIVE_DELAY_MIX_PARAMETER_ID, EffectDeviceKind::Delay { mix, .. }) => {
                Some(ParameterValue::Percentage(mix))
            }
            (NATIVE_DELAY_OUTPUT_PARAMETER_ID, EffectDeviceKind::Delay { output_db, .. }) => {
                Some(ParameterValue::Decibels(output_db))
            }
            (NATIVE_REVERB_SIZE_PARAMETER_ID, EffectDeviceKind::Reverb { size, .. }) => {
                Some(ParameterValue::Percentage(size))
            }
            (NATIVE_REVERB_PREDELAY_PARAMETER_ID, EffectDeviceKind::Reverb { predelay_ms, .. }) => {
                Some(ParameterValue::Float(predelay_ms))
            }
            (NATIVE_REVERB_DECAY_PARAMETER_ID, EffectDeviceKind::Reverb { decay_s, .. }) => {
                Some(ParameterValue::Seconds(decay_s))
            }
            (NATIVE_REVERB_DAMPING_PARAMETER_ID, EffectDeviceKind::Reverb { damping, .. }) => {
                Some(ParameterValue::Percentage(damping))
            }
            (NATIVE_REVERB_LOW_CUT_PARAMETER_ID, EffectDeviceKind::Reverb { low_cut_hz, .. }) => {
                Some(ParameterValue::FrequencyHertz(low_cut_hz))
            }
            (NATIVE_REVERB_HIGH_CUT_PARAMETER_ID, EffectDeviceKind::Reverb { high_cut_hz, .. }) => {
                Some(ParameterValue::FrequencyHertz(high_cut_hz))
            }
            (NATIVE_REVERB_DIFFUSION_PARAMETER_ID, EffectDeviceKind::Reverb { diffusion, .. }) => {
                Some(ParameterValue::Percentage(diffusion))
            }
            (NATIVE_REVERB_WIDTH_PARAMETER_ID, EffectDeviceKind::Reverb { width, .. }) => {
                Some(ParameterValue::Percentage(width))
            }
            (
                NATIVE_REVERB_EARLY_REFLECTIONS_PARAMETER_ID,
                EffectDeviceKind::Reverb {
                    early_reflections, ..
                },
            ) => Some(ParameterValue::Percentage(early_reflections)),
            (NATIVE_REVERB_MIX_PARAMETER_ID, EffectDeviceKind::Reverb { mix, .. }) => {
                Some(ParameterValue::Percentage(mix))
            }
            (NATIVE_REVERB_OUTPUT_PARAMETER_ID, EffectDeviceKind::Reverb { output_db, .. }) => {
                Some(ParameterValue::Decibels(output_db))
            }
            _ => degradation_parameter_value(id.as_str(), self.kind)
                .or_else(|| modulation_parameter_value(id.as_str(), self.kind)),
        }
    }

    pub fn set_parameter_value(
        &mut self,
        id: &ParameterId,
        value: ParameterValue,
    ) -> Result<(), EditError> {
        match (id.as_str(), &mut self.kind) {
            (NATIVE_GAIN_PARAMETER_ID, EffectDeviceKind::Gain { gain }) => {
                native_gain_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *gain = value.as_f32().expect("validated gain is numeric");
                Ok(())
            }
            (NATIVE_PAN_PARAMETER_ID, EffectDeviceKind::Pan { pan }) => {
                native_pan_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *pan = value.as_f32().expect("validated pan is numeric");
                Ok(())
            }
            (NATIVE_BALANCE_PARAMETER_ID, EffectDeviceKind::Balance { balance }) => {
                native_balance_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *balance = value.as_f32().expect("validated balance is numeric");
                Ok(())
            }
            (NATIVE_WIDTH_PARAMETER_ID, EffectDeviceKind::StereoWidth { width }) => {
                native_width_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *width = value.as_f32().expect("validated width is numeric");
                Ok(())
            }
            (
                NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID,
                EffectDeviceKind::PhaseInvert { invert_left, .. },
            ) => {
                native_phase_invert_left_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                let ParameterValue::Bool(value) = value else {
                    return Err(EditError::InvalidParameterValue);
                };
                *invert_left = value;
                Ok(())
            }
            (
                NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID,
                EffectDeviceKind::PhaseInvert { invert_right, .. },
            ) => {
                native_phase_invert_right_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                let ParameterValue::Bool(value) = value else {
                    return Err(EditError::InvalidParameterValue);
                };
                *invert_right = value;
                Ok(())
            }
            (NATIVE_FILTER_MODE_PARAMETER_ID, EffectDeviceKind::Filter { mode, .. }) => {
                native_filter_mode_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                let ParameterValue::Enum(value) = value else {
                    return Err(EditError::InvalidParameterValue);
                };
                *mode = FilterMode::from_parameter_id(&value)
                    .ok_or(EditError::InvalidParameterValue)?;
                Ok(())
            }
            (NATIVE_FILTER_CUTOFF_PARAMETER_ID, EffectDeviceKind::Filter { cutoff_hz, .. }) => {
                native_filter_cutoff_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *cutoff_hz = value.as_f32().expect("validated cutoff is numeric");
                Ok(())
            }
            (NATIVE_FILTER_RESONANCE_PARAMETER_ID, EffectDeviceKind::Filter { resonance, .. }) => {
                native_filter_resonance_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *resonance = value.as_f32().expect("validated resonance is numeric");
                Ok(())
            }
            (NATIVE_FILTER_DRIVE_PARAMETER_ID, EffectDeviceKind::Filter { drive_db, .. }) => {
                native_filter_drive_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *drive_db = value.as_f32().expect("validated drive is numeric");
                Ok(())
            }
            (NATIVE_FILTER_KEY_TRACK_PARAMETER_ID, EffectDeviceKind::Filter { key_track, .. }) => {
                native_filter_key_track_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *key_track = value.as_f32().expect("validated key track is numeric");
                Ok(())
            }
            (
                NATIVE_FILTER_ENV_AMOUNT_PARAMETER_ID,
                EffectDeviceKind::Filter { env_amount, .. },
            ) => {
                native_filter_env_amount_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *env_amount = value.as_f32().expect("validated env amount is numeric");
                Ok(())
            }
            (NATIVE_FILTER_MIX_PARAMETER_ID, EffectDeviceKind::Filter { mix, .. }) => {
                native_filter_mix_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *mix = value.as_f32().expect("validated mix is numeric");
                Ok(())
            }
            (NATIVE_DELAY_SYNC_PARAMETER_ID, EffectDeviceKind::Delay { sync, .. }) => {
                native_delay_sync_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                let ParameterValue::Bool(value) = value else {
                    return Err(EditError::InvalidParameterValue);
                };
                *sync = value;
                Ok(())
            }
            (NATIVE_DELAY_TIME_LEFT_PARAMETER_ID, EffectDeviceKind::Delay { time_left_ms, .. }) => {
                native_delay_time_left_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *time_left_ms = value.as_f32().expect("validated left time is numeric");
                Ok(())
            }
            (
                NATIVE_DELAY_TIME_RIGHT_PARAMETER_ID,
                EffectDeviceKind::Delay { time_right_ms, .. },
            ) => {
                native_delay_time_right_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *time_right_ms = value.as_f32().expect("validated right time is numeric");
                Ok(())
            }
            (NATIVE_DELAY_LINK_TIMES_PARAMETER_ID, EffectDeviceKind::Delay { link_times, .. }) => {
                native_delay_link_times_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                let ParameterValue::Bool(value) = value else {
                    return Err(EditError::InvalidParameterValue);
                };
                *link_times = value;
                Ok(())
            }
            (NATIVE_DELAY_FEEDBACK_PARAMETER_ID, EffectDeviceKind::Delay { feedback, .. }) => {
                native_delay_feedback_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *feedback = value.as_f32().expect("validated feedback is numeric");
                Ok(())
            }
            (NATIVE_DELAY_PING_PONG_PARAMETER_ID, EffectDeviceKind::Delay { ping_pong, .. }) => {
                native_delay_ping_pong_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                let ParameterValue::Bool(value) = value else {
                    return Err(EditError::InvalidParameterValue);
                };
                *ping_pong = value;
                Ok(())
            }
            (
                NATIVE_DELAY_FILTER_LOW_CUT_PARAMETER_ID,
                EffectDeviceKind::Delay {
                    filter_low_cut_hz, ..
                },
            ) => {
                native_delay_filter_low_cut_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *filter_low_cut_hz = value.as_f32().expect("validated low cut is numeric");
                Ok(())
            }
            (
                NATIVE_DELAY_FILTER_HIGH_CUT_PARAMETER_ID,
                EffectDeviceKind::Delay {
                    filter_high_cut_hz, ..
                },
            ) => {
                native_delay_filter_high_cut_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *filter_high_cut_hz = value.as_f32().expect("validated high cut is numeric");
                Ok(())
            }
            (NATIVE_DELAY_MOD_RATE_PARAMETER_ID, EffectDeviceKind::Delay { mod_rate_hz, .. }) => {
                native_delay_mod_rate_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *mod_rate_hz = value.as_f32().expect("validated mod rate is numeric");
                Ok(())
            }
            (NATIVE_DELAY_MOD_DEPTH_PARAMETER_ID, EffectDeviceKind::Delay { mod_depth, .. }) => {
                native_delay_mod_depth_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *mod_depth = value.as_f32().expect("validated mod depth is numeric");
                Ok(())
            }
            (NATIVE_DELAY_MIX_PARAMETER_ID, EffectDeviceKind::Delay { mix, .. }) => {
                native_delay_mix_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *mix = value.as_f32().expect("validated mix is numeric");
                Ok(())
            }
            (NATIVE_DELAY_OUTPUT_PARAMETER_ID, EffectDeviceKind::Delay { output_db, .. }) => {
                native_delay_output_descriptor()
                    .validate(&value)
                    .map_err(|_| EditError::InvalidParameterValue)?;
                *output_db = value.as_f32().expect("validated output is numeric");
                Ok(())
            }
            (NATIVE_REVERB_SIZE_PARAMETER_ID, EffectDeviceKind::Reverb { size, .. }) => {
                set_reverb_numeric(native_reverb_size_descriptor(), size, value)
            }
            (NATIVE_REVERB_PREDELAY_PARAMETER_ID, EffectDeviceKind::Reverb { predelay_ms, .. }) => {
                set_reverb_numeric(native_reverb_predelay_descriptor(), predelay_ms, value)
            }
            (NATIVE_REVERB_DECAY_PARAMETER_ID, EffectDeviceKind::Reverb { decay_s, .. }) => {
                set_reverb_numeric(native_reverb_decay_descriptor(), decay_s, value)
            }
            (NATIVE_REVERB_DAMPING_PARAMETER_ID, EffectDeviceKind::Reverb { damping, .. }) => {
                set_reverb_numeric(native_reverb_damping_descriptor(), damping, value)
            }
            (NATIVE_REVERB_LOW_CUT_PARAMETER_ID, EffectDeviceKind::Reverb { low_cut_hz, .. }) => {
                set_reverb_numeric(native_reverb_low_cut_descriptor(), low_cut_hz, value)
            }
            (NATIVE_REVERB_HIGH_CUT_PARAMETER_ID, EffectDeviceKind::Reverb { high_cut_hz, .. }) => {
                set_reverb_numeric(native_reverb_high_cut_descriptor(), high_cut_hz, value)
            }
            (NATIVE_REVERB_DIFFUSION_PARAMETER_ID, EffectDeviceKind::Reverb { diffusion, .. }) => {
                set_reverb_numeric(native_reverb_diffusion_descriptor(), diffusion, value)
            }
            (NATIVE_REVERB_WIDTH_PARAMETER_ID, EffectDeviceKind::Reverb { width, .. }) => {
                set_reverb_numeric(native_reverb_width_descriptor(), width, value)
            }
            (
                NATIVE_REVERB_EARLY_REFLECTIONS_PARAMETER_ID,
                EffectDeviceKind::Reverb {
                    early_reflections, ..
                },
            ) => set_reverb_numeric(
                native_reverb_early_reflections_descriptor(),
                early_reflections,
                value,
            ),
            (NATIVE_REVERB_MIX_PARAMETER_ID, EffectDeviceKind::Reverb { mix, .. }) => {
                set_reverb_numeric(native_reverb_mix_descriptor(), mix, value)
            }
            (NATIVE_REVERB_OUTPUT_PARAMETER_ID, EffectDeviceKind::Reverb { output_db, .. }) => {
                set_reverb_numeric(native_reverb_output_descriptor(), output_db, value)
            }
            _ => {
                if set_degradation_parameter_value(id.as_str(), &mut self.kind, value.clone())?
                    || set_modulation_parameter_value(id.as_str(), &mut self.kind, value)?
                {
                    Ok(())
                } else {
                    Err(EditError::UnknownParameter)
                }
            }
        }
    }

    #[must_use]
    pub fn native_module_descriptor(&self) -> NativeModuleDescriptor {
        match self.kind {
            EffectDeviceKind::Gain { .. } => native_gain_module_descriptor(),
            EffectDeviceKind::Pan { .. } => native_pan_module_descriptor(),
            EffectDeviceKind::Balance { .. } => native_balance_module_descriptor(),
            EffectDeviceKind::StereoWidth { .. } => native_width_module_descriptor(),
            EffectDeviceKind::PhaseInvert { .. } => native_phase_module_descriptor(),
            EffectDeviceKind::Filter { .. } => native_filter_module_descriptor(),
            EffectDeviceKind::Delay { .. } => native_delay_module_descriptor(),
            EffectDeviceKind::Reverb { .. } => native_reverb_module_descriptor(),
            EffectDeviceKind::Drive { .. } => native_drive_module_descriptor(),
            EffectDeviceKind::Bitcrusher { .. } => native_bitcrusher_module_descriptor(),
            EffectDeviceKind::Chorus { .. } => native_chorus_module_descriptor(),
            EffectDeviceKind::Flanger { .. } => native_flanger_module_descriptor(),
            EffectDeviceKind::Phaser { .. } => native_phaser_module_descriptor(),
        }
    }

    #[must_use]
    pub fn native_module_state(&self) -> NativeModuleState {
        let (module, parameters) = match self.kind {
            EffectDeviceKind::Gain { gain } => (
                NativeModuleId::from(NATIVE_GAIN_MODULE_ID),
                vec![NativeModuleParameter {
                    id: ParameterId::from(NATIVE_GAIN_PARAMETER_ID),
                    value: ParameterValue::Float(gain),
                }],
            ),
            EffectDeviceKind::Pan { pan } => (
                NativeModuleId::from(NATIVE_PAN_MODULE_ID),
                vec![NativeModuleParameter {
                    id: ParameterId::from(NATIVE_PAN_PARAMETER_ID),
                    value: ParameterValue::Bipolar(pan),
                }],
            ),
            EffectDeviceKind::Balance { balance } => (
                NativeModuleId::from(NATIVE_BALANCE_MODULE_ID),
                vec![NativeModuleParameter {
                    id: ParameterId::from(NATIVE_BALANCE_PARAMETER_ID),
                    value: ParameterValue::Bipolar(balance),
                }],
            ),
            EffectDeviceKind::StereoWidth { width } => (
                NativeModuleId::from(NATIVE_WIDTH_MODULE_ID),
                vec![NativeModuleParameter {
                    id: ParameterId::from(NATIVE_WIDTH_PARAMETER_ID),
                    value: ParameterValue::Percentage(width),
                }],
            ),
            EffectDeviceKind::PhaseInvert {
                invert_left,
                invert_right,
            } => (
                NativeModuleId::from(NATIVE_PHASE_MODULE_ID),
                vec![
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID),
                        value: ParameterValue::Bool(invert_left),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID),
                        value: ParameterValue::Bool(invert_right),
                    },
                ],
            ),
            EffectDeviceKind::Filter {
                mode,
                cutoff_hz,
                resonance,
                drive_db,
                key_track,
                env_amount,
                mix,
            } => (
                NativeModuleId::from(NATIVE_FILTER_MODULE_ID),
                vec![
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_FILTER_MODE_PARAMETER_ID),
                        value: ParameterValue::Enum(mode.parameter_id().to_string()),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_FILTER_CUTOFF_PARAMETER_ID),
                        value: ParameterValue::FrequencyHertz(cutoff_hz),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_FILTER_RESONANCE_PARAMETER_ID),
                        value: ParameterValue::Normalized(resonance),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_FILTER_DRIVE_PARAMETER_ID),
                        value: ParameterValue::Decibels(drive_db),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_FILTER_KEY_TRACK_PARAMETER_ID),
                        value: ParameterValue::Percentage(key_track),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_FILTER_ENV_AMOUNT_PARAMETER_ID),
                        value: ParameterValue::Percentage(env_amount),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_FILTER_MIX_PARAMETER_ID),
                        value: ParameterValue::Percentage(mix),
                    },
                ],
            ),
            EffectDeviceKind::Delay {
                sync,
                time_left_ms,
                time_right_ms,
                link_times,
                feedback,
                ping_pong,
                filter_low_cut_hz,
                filter_high_cut_hz,
                mod_rate_hz,
                mod_depth,
                mix,
                output_db,
            } => (
                NativeModuleId::from(NATIVE_DELAY_MODULE_ID),
                vec![
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_SYNC_PARAMETER_ID),
                        value: ParameterValue::Bool(sync),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_TIME_LEFT_PARAMETER_ID),
                        value: ParameterValue::Float(time_left_ms),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_TIME_RIGHT_PARAMETER_ID),
                        value: ParameterValue::Float(time_right_ms),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_LINK_TIMES_PARAMETER_ID),
                        value: ParameterValue::Bool(link_times),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_FEEDBACK_PARAMETER_ID),
                        value: ParameterValue::Percentage(feedback),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_PING_PONG_PARAMETER_ID),
                        value: ParameterValue::Bool(ping_pong),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_FILTER_LOW_CUT_PARAMETER_ID),
                        value: ParameterValue::FrequencyHertz(filter_low_cut_hz),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_FILTER_HIGH_CUT_PARAMETER_ID),
                        value: ParameterValue::FrequencyHertz(filter_high_cut_hz),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_MOD_RATE_PARAMETER_ID),
                        value: ParameterValue::FrequencyHertz(mod_rate_hz),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_MOD_DEPTH_PARAMETER_ID),
                        value: ParameterValue::Percentage(mod_depth),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_MIX_PARAMETER_ID),
                        value: ParameterValue::Percentage(mix),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_DELAY_OUTPUT_PARAMETER_ID),
                        value: ParameterValue::Decibels(output_db),
                    },
                ],
            ),
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
            } => (
                NativeModuleId::from(NATIVE_REVERB_MODULE_ID),
                vec![
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_SIZE_PARAMETER_ID),
                        value: ParameterValue::Percentage(size),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_PREDELAY_PARAMETER_ID),
                        value: ParameterValue::Float(predelay_ms),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_DECAY_PARAMETER_ID),
                        value: ParameterValue::Seconds(decay_s),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_DAMPING_PARAMETER_ID),
                        value: ParameterValue::Percentage(damping),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_LOW_CUT_PARAMETER_ID),
                        value: ParameterValue::FrequencyHertz(low_cut_hz),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_HIGH_CUT_PARAMETER_ID),
                        value: ParameterValue::FrequencyHertz(high_cut_hz),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_DIFFUSION_PARAMETER_ID),
                        value: ParameterValue::Percentage(diffusion),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_WIDTH_PARAMETER_ID),
                        value: ParameterValue::Percentage(width),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_EARLY_REFLECTIONS_PARAMETER_ID),
                        value: ParameterValue::Percentage(early_reflections),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_MIX_PARAMETER_ID),
                        value: ParameterValue::Percentage(mix),
                    },
                    NativeModuleParameter {
                        id: ParameterId::from(NATIVE_REVERB_OUTPUT_PARAMETER_ID),
                        value: ParameterValue::Decibels(output_db),
                    },
                ],
            ),
            EffectDeviceKind::Drive { .. } | EffectDeviceKind::Bitcrusher { .. } => {
                degradation_native_module_state(self.kind).expect("degradation device state")
            }
            EffectDeviceKind::Chorus { .. }
            | EffectDeviceKind::Flanger { .. }
            | EffectDeviceKind::Phaser { .. } => {
                modulation_native_module_state(self.kind).expect("modulation device state")
            }
        };
        NativeModuleState {
            module,
            bypassed: self.bypassed,
            parameters,
            unknown_parameters: Vec::new(),
        }
    }

    pub fn apply_native_module_state(
        &mut self,
        state: &NativeModuleState,
    ) -> Result<(), EditError> {
        let descriptor = self.native_module_descriptor();
        state
            .validate_against(&descriptor)
            .map_err(|_| EditError::InvalidParameterValue)?;
        self.bypassed = state.bypassed;
        for parameter in &state.parameters {
            self.set_parameter_value(&parameter.id, parameter.value.clone())?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "effect_parameters_tests.rs"]
mod tests;

fn set_reverb_numeric(
    descriptor: ParameterDescriptor,
    target: &mut f32,
    value: ParameterValue,
) -> Result<(), EditError> {
    descriptor
        .validate(&value)
        .map_err(|_| EditError::InvalidParameterValue)?;
    *target = value.as_f32().expect("validated reverb value is numeric");
    Ok(())
}
