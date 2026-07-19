use crate::{
    model::{EditError, EffectDevice, EffectDeviceKind},
    native_module::{
        native_balance_module_descriptor, native_gain_module_descriptor,
        native_pan_module_descriptor, native_phase_module_descriptor,
        native_width_module_descriptor, NativeModuleDescriptor, NativeModuleId,
        NativeModuleParameter, NativeModuleState, NATIVE_BALANCE_MODULE_ID, NATIVE_GAIN_MODULE_ID,
        NATIVE_PAN_MODULE_ID, NATIVE_PHASE_MODULE_ID, NATIVE_WIDTH_MODULE_ID,
    },
    parameters::{
        native_balance_descriptor, native_gain_descriptor, native_pan_descriptor,
        native_phase_invert_left_descriptor, native_phase_invert_right_descriptor,
        native_width_descriptor, ParameterDescriptor, ParameterId, ParameterValue,
        NATIVE_BALANCE_PARAMETER_ID, NATIVE_GAIN_PARAMETER_ID, NATIVE_PAN_PARAMETER_ID,
        NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID, NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID,
        NATIVE_WIDTH_PARAMETER_ID,
    },
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
            _ => None,
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
            _ => Err(EditError::UnknownParameter),
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
mod tests {
    use super::*;

    #[test]
    fn effect_devices_expose_and_validate_parameter_values() {
        let mut gain = EffectDevice::gain(1, 1.0);
        let descriptor = gain
            .parameter_descriptors()
            .into_iter()
            .next()
            .expect("gain descriptor");

        assert_eq!(descriptor.id, ParameterId::from(NATIVE_GAIN_PARAMETER_ID));
        assert_eq!(
            gain.parameter_value(&descriptor.id),
            Some(ParameterValue::Float(1.0))
        );

        gain.set_parameter_value(&descriptor.id, ParameterValue::Float(0.5))
            .expect("set gain parameter");
        assert_eq!(gain.kind, EffectDeviceKind::Gain { gain: 0.5 });
        assert_eq!(
            gain.set_parameter_value(&descriptor.id, ParameterValue::Float(3.0))
                .expect_err("gain outside descriptor range"),
            EditError::InvalidParameterValue
        );

        let mut width = EffectDevice::stereo_width(4, 1.0);
        assert_eq!(
            width.parameter_value(&ParameterId::from(NATIVE_WIDTH_PARAMETER_ID)),
            Some(ParameterValue::Percentage(1.0))
        );
        width
            .set_parameter_value(
                &ParameterId::from(NATIVE_WIDTH_PARAMETER_ID),
                ParameterValue::Percentage(2.0),
            )
            .expect("set width");
        assert_eq!(width.kind, EffectDeviceKind::StereoWidth { width: 2.0 });

        let mut phase = EffectDevice::phase_invert(5, false, false);
        phase
            .set_parameter_value(
                &ParameterId::from(NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID),
                ParameterValue::Bool(true),
            )
            .expect("set phase invert left");
        assert_eq!(
            phase.kind,
            EffectDeviceKind::PhaseInvert {
                invert_left: true,
                invert_right: false
            }
        );
    }

    #[test]
    fn effect_devices_round_trip_native_module_state() {
        let mut gain = EffectDevice::gain(1, 1.0);
        let mut state = gain.native_module_state();

        state.bypassed = true;
        state
            .set_parameter(
                &gain.native_module_descriptor(),
                ParameterId::from(NATIVE_GAIN_PARAMETER_ID),
                ParameterValue::Float(0.25),
            )
            .expect("set native module parameter");

        gain.apply_native_module_state(&state)
            .expect("apply module state");

        assert!(gain.bypassed);
        assert_eq!(gain.kind, EffectDeviceKind::Gain { gain: 0.25 });
    }

    #[test]
    fn effect_devices_round_trip_multi_parameter_native_module_state() {
        let mut phase = EffectDevice::phase_invert(5, false, false);
        let mut state = phase.native_module_state();

        assert_eq!(state.parameters.len(), 2);
        state
            .set_parameter(
                &phase.native_module_descriptor(),
                ParameterId::from(NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID),
                ParameterValue::Bool(true),
            )
            .expect("set phase left");
        state
            .set_parameter(
                &phase.native_module_descriptor(),
                ParameterId::from(NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID),
                ParameterValue::Bool(true),
            )
            .expect("set phase right");

        phase
            .apply_native_module_state(&state)
            .expect("apply phase state");

        assert_eq!(
            phase.kind,
            EffectDeviceKind::PhaseInvert {
                invert_left: true,
                invert_right: true
            }
        );
    }
}
