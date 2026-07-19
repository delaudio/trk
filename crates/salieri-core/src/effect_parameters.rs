use crate::{
    model::{EditError, EffectDevice, EffectDeviceKind},
    native_module::{
        native_gain_module_descriptor, native_pan_module_descriptor, NativeModuleDescriptor,
        NativeModuleId, NativeModuleParameter, NativeModuleState, NATIVE_GAIN_MODULE_ID,
        NATIVE_PAN_MODULE_ID,
    },
    parameters::{
        native_gain_descriptor, native_pan_descriptor, ParameterDescriptor, ParameterId,
        ParameterValue, NATIVE_GAIN_PARAMETER_ID, NATIVE_PAN_PARAMETER_ID,
    },
};

impl EffectDevice {
    #[must_use]
    pub fn parameter_descriptors(&self) -> Vec<ParameterDescriptor> {
        match self.kind {
            EffectDeviceKind::Gain { .. } => vec![native_gain_descriptor()],
            EffectDeviceKind::Pan { .. } => vec![native_pan_descriptor()],
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
            _ => Err(EditError::UnknownParameter),
        }
    }

    #[must_use]
    pub fn native_module_descriptor(&self) -> NativeModuleDescriptor {
        match self.kind {
            EffectDeviceKind::Gain { .. } => native_gain_module_descriptor(),
            EffectDeviceKind::Pan { .. } => native_pan_module_descriptor(),
        }
    }

    #[must_use]
    pub fn native_module_state(&self) -> NativeModuleState {
        let (module, parameter) = match self.kind {
            EffectDeviceKind::Gain { gain } => (
                NativeModuleId::from(NATIVE_GAIN_MODULE_ID),
                NativeModuleParameter {
                    id: ParameterId::from(NATIVE_GAIN_PARAMETER_ID),
                    value: ParameterValue::Float(gain),
                },
            ),
            EffectDeviceKind::Pan { pan } => (
                NativeModuleId::from(NATIVE_PAN_MODULE_ID),
                NativeModuleParameter {
                    id: ParameterId::from(NATIVE_PAN_PARAMETER_ID),
                    value: ParameterValue::Bipolar(pan),
                },
            ),
        };
        NativeModuleState {
            module,
            bypassed: self.bypassed,
            parameters: vec![parameter],
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
}
