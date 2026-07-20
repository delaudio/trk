use std::fmt;

use serde::{Deserialize, Serialize};

use crate::parameters::*;

pub const NATIVE_GAIN_MODULE_ID: &str = "native.effect.gain";
pub const NATIVE_PAN_MODULE_ID: &str = "native.effect.pan";
pub const NATIVE_BALANCE_MODULE_ID: &str = "native.effect.balance";
pub const NATIVE_WIDTH_MODULE_ID: &str = "native.effect.width";
pub const NATIVE_PHASE_MODULE_ID: &str = "native.effect.phase";
pub const NATIVE_FILTER_MODULE_ID: &str = "native.effect.filter";
pub const NATIVE_DELAY_MODULE_ID: &str = "native.effect.delay";
pub const NATIVE_REVERB_MODULE_ID: &str = "native.effect.reverb";
pub const NATIVE_DRIVE_MODULE_ID: &str = "native.effect.drive";
pub const NATIVE_BITCRUSHER_MODULE_ID: &str = "native.effect.bitcrusher";
pub const NATIVE_CHORUS_MODULE_ID: &str = "native.effect.chorus";
pub const NATIVE_FLANGER_MODULE_ID: &str = "native.effect.flanger";
pub const NATIVE_PHASER_MODULE_ID: &str = "native.effect.phaser";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NativeModuleId(pub String);

impl NativeModuleId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for NativeModuleId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for NativeModuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeModuleRole {
    Instrument,
    Effect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeModuleDescriptor {
    pub id: NativeModuleId,
    pub name: String,
    pub role: NativeModuleRole,
    pub parameters: Vec<ParameterDescriptor>,
    pub latency_frames: u32,
    pub realtime_safe: bool,
}

impl NativeModuleDescriptor {
    #[must_use]
    pub fn parameter(&self, id: &ParameterId) -> Option<&ParameterDescriptor> {
        self.parameters
            .iter()
            .find(|descriptor| descriptor.id == *id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeModuleParameter {
    pub id: ParameterId,
    pub value: ParameterValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeModuleState {
    pub module: NativeModuleId,
    pub bypassed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<NativeModuleParameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown_parameters: Vec<NativeModuleParameter>,
}

impl NativeModuleState {
    #[must_use]
    pub fn defaults_for(descriptor: &NativeModuleDescriptor) -> Self {
        Self {
            module: descriptor.id.clone(),
            bypassed: false,
            parameters: descriptor
                .parameters
                .iter()
                .map(|descriptor| NativeModuleParameter {
                    id: descriptor.id.clone(),
                    value: descriptor.default.clone(),
                })
                .collect(),
            unknown_parameters: Vec::new(),
        }
    }

    pub fn validate_against(
        &self,
        descriptor: &NativeModuleDescriptor,
    ) -> Result<(), NativeModuleError> {
        if self.module != descriptor.id {
            return Err(NativeModuleError::ModuleMismatch {
                expected: descriptor.id.clone(),
                actual: self.module.clone(),
            });
        }
        for parameter in &self.parameters {
            let Some(parameter_descriptor) = descriptor.parameter(&parameter.id) else {
                return Err(NativeModuleError::UnknownParameter {
                    id: parameter.id.clone(),
                });
            };
            parameter_descriptor.validate(&parameter.value)?;
        }
        Ok(())
    }

    pub fn set_parameter(
        &mut self,
        descriptor: &NativeModuleDescriptor,
        id: ParameterId,
        value: ParameterValue,
    ) -> Result<(), NativeModuleError> {
        let Some(parameter_descriptor) = descriptor.parameter(&id) else {
            self.unknown_parameters
                .push(NativeModuleParameter { id, value });
            return Ok(());
        };
        parameter_descriptor.validate(&value)?;
        if let Some(parameter) = self
            .parameters
            .iter_mut()
            .find(|parameter| parameter.id == id)
        {
            parameter.value = value;
        } else {
            self.parameters.push(NativeModuleParameter { id, value });
        }
        Ok(())
    }

    pub fn reset_to_defaults(&mut self, descriptor: &NativeModuleDescriptor) {
        self.bypassed = false;
        self.parameters = Self::defaults_for(descriptor).parameters;
    }

    #[must_use]
    pub fn parameter_value(&self, id: &ParameterId) -> Option<&ParameterValue> {
        self.parameters
            .iter()
            .find(|parameter| parameter.id == *id)
            .map(|parameter| &parameter.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NativeModuleError {
    #[error("native module mismatch: expected {expected}, got {actual}")]
    ModuleMismatch {
        expected: NativeModuleId,
        actual: NativeModuleId,
    },
    #[error("unknown native module parameter {id}")]
    UnknownParameter { id: ParameterId },
    #[error(transparent)]
    InvalidParameter(#[from] ParameterValidationError),
}

#[must_use]
pub fn native_gain_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_GAIN_MODULE_ID),
        name: "Gain".to_string(),
        role: NativeModuleRole::Effect,
        parameters: vec![native_gain_descriptor()],
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_pan_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_PAN_MODULE_ID),
        name: "Pan".to_string(),
        role: NativeModuleRole::Effect,
        parameters: vec![native_pan_descriptor()],
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_balance_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_BALANCE_MODULE_ID),
        name: "Balance".to_string(),
        role: NativeModuleRole::Effect,
        parameters: vec![native_balance_descriptor()],
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_width_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_WIDTH_MODULE_ID),
        name: "Stereo Width".to_string(),
        role: NativeModuleRole::Effect,
        parameters: vec![native_width_descriptor()],
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_phase_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_PHASE_MODULE_ID),
        name: "Phase".to_string(),
        role: NativeModuleRole::Effect,
        parameters: vec![
            native_phase_invert_left_descriptor(),
            native_phase_invert_right_descriptor(),
        ],
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_filter_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_FILTER_MODULE_ID),
        name: "Multimode Filter".to_string(),
        role: NativeModuleRole::Effect,
        parameters: vec![
            native_filter_mode_descriptor(),
            native_filter_cutoff_descriptor(),
            native_filter_resonance_descriptor(),
            native_filter_drive_descriptor(),
            native_filter_key_track_descriptor(),
            native_filter_env_amount_descriptor(),
            native_filter_mix_descriptor(),
        ],
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_delay_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_DELAY_MODULE_ID),
        name: "Stereo Delay".to_string(),
        role: NativeModuleRole::Effect,
        parameters: vec![
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
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_reverb_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_REVERB_MODULE_ID),
        name: "Stereo Reverb".to_string(),
        role: NativeModuleRole::Effect,
        parameters: vec![
            native_reverb_size_descriptor(),
            native_reverb_predelay_descriptor(),
            native_reverb_decay_descriptor(),
            native_reverb_damping_descriptor(),
            native_reverb_low_cut_descriptor(),
            native_reverb_high_cut_descriptor(),
            native_reverb_diffusion_descriptor(),
            native_reverb_width_descriptor(),
            native_reverb_early_reflections_descriptor(),
            native_reverb_mix_descriptor(),
            native_reverb_output_descriptor(),
        ],
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_drive_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_DRIVE_MODULE_ID),
        name: "Drive".to_string(),
        role: NativeModuleRole::Effect,
        parameters: vec![
            native_drive_mode_descriptor(),
            native_drive_drive_descriptor(),
            native_drive_tone_descriptor(),
            native_drive_bias_descriptor(),
            native_drive_mix_descriptor(),
            native_drive_output_descriptor(),
        ],
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_bitcrusher_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_BITCRUSHER_MODULE_ID),
        name: "Bitcrusher".to_string(),
        role: NativeModuleRole::Effect,
        parameters: vec![
            native_bitcrusher_bit_depth_descriptor(),
            native_bitcrusher_reduction_descriptor(),
            native_bitcrusher_dither_descriptor(),
            native_bitcrusher_mix_descriptor(),
            native_bitcrusher_output_descriptor(),
        ],
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_chorus_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_CHORUS_MODULE_ID),
        name: "Chorus".to_string(),
        role: NativeModuleRole::Effect,
        parameters: native_chorus_parameter_descriptors(),
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_flanger_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_FLANGER_MODULE_ID),
        name: "Flanger".to_string(),
        role: NativeModuleRole::Effect,
        parameters: native_flanger_parameter_descriptors(),
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn native_phaser_module_descriptor() -> NativeModuleDescriptor {
    NativeModuleDescriptor {
        id: NativeModuleId::from(NATIVE_PHASER_MODULE_ID),
        name: "Phaser".to_string(),
        role: NativeModuleRole::Effect,
        parameters: native_phaser_parameter_descriptors(),
        latency_frames: 0,
        realtime_safe: true,
    }
}

#[must_use]
pub fn builtin_native_module_descriptor(id: &NativeModuleId) -> Option<NativeModuleDescriptor> {
    match id.as_str() {
        NATIVE_GAIN_MODULE_ID => Some(native_gain_module_descriptor()),
        NATIVE_PAN_MODULE_ID => Some(native_pan_module_descriptor()),
        NATIVE_BALANCE_MODULE_ID => Some(native_balance_module_descriptor()),
        NATIVE_WIDTH_MODULE_ID => Some(native_width_module_descriptor()),
        NATIVE_PHASE_MODULE_ID => Some(native_phase_module_descriptor()),
        NATIVE_FILTER_MODULE_ID => Some(native_filter_module_descriptor()),
        NATIVE_DELAY_MODULE_ID => Some(native_delay_module_descriptor()),
        NATIVE_REVERB_MODULE_ID => Some(native_reverb_module_descriptor()),
        NATIVE_DRIVE_MODULE_ID => Some(native_drive_module_descriptor()),
        NATIVE_BITCRUSHER_MODULE_ID => Some(native_bitcrusher_module_descriptor()),
        NATIVE_CHORUS_MODULE_ID => Some(native_chorus_module_descriptor()),
        NATIVE_FLANGER_MODULE_ID => Some(native_flanger_module_descriptor()),
        NATIVE_PHASER_MODULE_ID => Some(native_phaser_module_descriptor()),
        _ => None,
    }
}

#[must_use]
pub fn builtin_native_effect_descriptors() -> Vec<NativeModuleDescriptor> {
    vec![
        native_gain_module_descriptor(),
        native_pan_module_descriptor(),
        native_balance_module_descriptor(),
        native_width_module_descriptor(),
        native_phase_module_descriptor(),
        native_filter_module_descriptor(),
        native_delay_module_descriptor(),
        native_reverb_module_descriptor(),
        native_drive_module_descriptor(),
        native_bitcrusher_module_descriptor(),
        native_chorus_module_descriptor(),
        native_flanger_module_descriptor(),
        native_phaser_module_descriptor(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        NATIVE_DELAY_MIX_PARAMETER_ID, NATIVE_DELAY_MODULE_ID, NATIVE_DELAY_TIME_LEFT_PARAMETER_ID,
        NATIVE_FILTER_CUTOFF_PARAMETER_ID, NATIVE_FILTER_MODE_PARAMETER_ID,
        NATIVE_FILTER_MODULE_ID, NATIVE_GAIN_PARAMETER_ID, NATIVE_PAN_PARAMETER_ID,
        NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID, NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID,
        NATIVE_PHASE_MODULE_ID, NATIVE_REVERB_DECAY_PARAMETER_ID, NATIVE_REVERB_MIX_PARAMETER_ID,
        NATIVE_REVERB_MODULE_ID, NATIVE_REVERB_PREDELAY_PARAMETER_ID, NATIVE_WIDTH_MODULE_ID,
        NATIVE_WIDTH_PARAMETER_ID,
    };

    #[test]
    fn native_module_state_validates_resets_and_preserves_unknown_parameters() {
        let descriptor = native_gain_module_descriptor();
        let mut state = NativeModuleState::defaults_for(&descriptor);

        state
            .set_parameter(
                &descriptor,
                ParameterId::from(NATIVE_GAIN_PARAMETER_ID),
                ParameterValue::Float(0.5),
            )
            .expect("set gain");
        state
            .set_parameter(
                &descriptor,
                ParameterId::from("native.gain.future"),
                ParameterValue::Unknown {
                    value_type: "future".to_string(),
                    raw: "opaque".to_string(),
                },
            )
            .expect("preserve unknown");

        state.validate_against(&descriptor).expect("valid state");
        assert_eq!(
            state.parameter_value(&ParameterId::from(NATIVE_GAIN_PARAMETER_ID)),
            Some(&ParameterValue::Float(0.5))
        );
        assert_eq!(state.unknown_parameters.len(), 1);

        state.bypassed = true;
        state.reset_to_defaults(&descriptor);
        assert!(!state.bypassed);
        assert_eq!(
            state.parameter_value(&ParameterId::from(NATIVE_GAIN_PARAMETER_ID)),
            Some(&ParameterValue::Float(1.0))
        );
        assert_eq!(state.unknown_parameters.len(), 1);
    }

    #[test]
    fn native_module_state_rejects_invalid_parameter_values() {
        let descriptor = native_pan_module_descriptor();
        let mut state = NativeModuleState::defaults_for(&descriptor);

        let error = state
            .set_parameter(
                &descriptor,
                ParameterId::from(NATIVE_PAN_PARAMETER_ID),
                ParameterValue::Bipolar(2.0),
            )
            .expect_err("pan is out of range");

        assert!(matches!(error, NativeModuleError::InvalidParameter(_)));
    }

    #[test]
    fn native_module_state_uses_stable_serializable_ids() {
        let descriptor = native_gain_module_descriptor();
        let state = NativeModuleState::defaults_for(&descriptor);

        let serialized = serde_json::to_string(&state).expect("serialize state");

        assert!(serialized.contains(NATIVE_GAIN_MODULE_ID));
        assert!(serialized.contains(NATIVE_GAIN_PARAMETER_ID));
    }

    #[test]
    fn native_utility_module_descriptors_have_defaults_and_serializable_ids() {
        let width_descriptor = native_width_module_descriptor();
        let width_state = NativeModuleState::defaults_for(&width_descriptor);

        assert_eq!(
            width_descriptor.id,
            NativeModuleId::from(NATIVE_WIDTH_MODULE_ID)
        );
        assert_eq!(
            width_state.parameter_value(&ParameterId::from(NATIVE_WIDTH_PARAMETER_ID)),
            Some(&ParameterValue::Percentage(1.0))
        );

        let phase_descriptor = native_phase_module_descriptor();
        let phase_state = NativeModuleState::defaults_for(&phase_descriptor);
        let serialized = serde_json::to_string(&phase_state).expect("serialize phase");

        assert_eq!(
            phase_descriptor.id,
            NativeModuleId::from(NATIVE_PHASE_MODULE_ID)
        );
        assert_eq!(phase_descriptor.parameters.len(), 2);
        assert_eq!(
            phase_state.parameter_value(&ParameterId::from(NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID)),
            Some(&ParameterValue::Bool(false))
        );
        assert!(serialized.contains(NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID));
        assert!(serialized.contains(NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID));
    }

    #[test]
    fn native_filter_module_descriptor_has_stable_defaults() {
        let descriptor = native_filter_module_descriptor();
        let state = NativeModuleState::defaults_for(&descriptor);
        let serialized = serde_json::to_string(&state).expect("serialize filter");

        assert_eq!(descriptor.id, NativeModuleId::from(NATIVE_FILTER_MODULE_ID));
        assert_eq!(descriptor.parameters.len(), 7);
        assert_eq!(
            state.parameter_value(&ParameterId::from(NATIVE_FILTER_MODE_PARAMETER_ID)),
            Some(&ParameterValue::Enum("lowPass".to_string()))
        );
        assert_eq!(
            state.parameter_value(&ParameterId::from(NATIVE_FILTER_CUTOFF_PARAMETER_ID)),
            Some(&ParameterValue::FrequencyHertz(12_000.0))
        );
        assert!(serialized.contains(NATIVE_FILTER_MODULE_ID));
        assert!(serialized.contains(NATIVE_FILTER_MODE_PARAMETER_ID));
    }

    #[test]
    fn native_delay_module_descriptor_has_stable_defaults() {
        let descriptor = native_delay_module_descriptor();
        let state = NativeModuleState::defaults_for(&descriptor);
        let serialized = serde_json::to_string(&state).expect("serialize delay");

        assert_eq!(descriptor.id, NativeModuleId::from(NATIVE_DELAY_MODULE_ID));
        assert_eq!(descriptor.parameters.len(), 12);
        assert_eq!(
            state.parameter_value(&ParameterId::from(NATIVE_DELAY_TIME_LEFT_PARAMETER_ID)),
            Some(&ParameterValue::Float(500.0))
        );
        assert_eq!(
            state.parameter_value(&ParameterId::from(NATIVE_DELAY_MIX_PARAMETER_ID)),
            Some(&ParameterValue::Percentage(0.25))
        );
        assert!(serialized.contains(NATIVE_DELAY_MODULE_ID));
        assert!(serialized.contains(NATIVE_DELAY_TIME_LEFT_PARAMETER_ID));
    }

    #[test]
    fn native_reverb_module_descriptor_has_stable_defaults() {
        let descriptor = native_reverb_module_descriptor();
        let state = NativeModuleState::defaults_for(&descriptor);
        let serialized = serde_json::to_string(&state).expect("serialize reverb");

        assert_eq!(descriptor.id, NativeModuleId::from(NATIVE_REVERB_MODULE_ID));
        assert_eq!(descriptor.parameters.len(), 11);
        assert_eq!(
            state.parameter_value(&ParameterId::from(NATIVE_REVERB_PREDELAY_PARAMETER_ID)),
            Some(&ParameterValue::Float(20.0))
        );
        assert_eq!(
            state.parameter_value(&ParameterId::from(NATIVE_REVERB_DECAY_PARAMETER_ID)),
            Some(&ParameterValue::Seconds(2.5))
        );
        assert_eq!(
            state.parameter_value(&ParameterId::from(NATIVE_REVERB_MIX_PARAMETER_ID)),
            Some(&ParameterValue::Percentage(0.25))
        );
        assert!(serialized.contains(NATIVE_REVERB_MODULE_ID));
    }
}
