use crate::{dsp::DspDeviceKind, errors::AudioExportError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeModulePrepareSpec {
    pub sample_rate: u32,
    pub channels: u16,
    pub max_block_frames: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeEffectModuleSpec {
    pub bypassed: bool,
    pub kind: DspDeviceKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeEffectParameterValue {
    Gain(f32),
    Pan(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeEffectModule {
    spec: NativeEffectModuleSpec,
    default: NativeEffectModuleSpec,
    prepared: Option<NativeModulePrepareSpec>,
}

impl NativeEffectModule {
    #[must_use]
    pub fn new(spec: NativeEffectModuleSpec) -> Self {
        Self {
            spec,
            default: spec,
            prepared: None,
        }
    }

    pub fn prepare(&mut self, spec: NativeModulePrepareSpec) -> Result<(), AudioExportError> {
        if spec.sample_rate == 0 || spec.channels == 0 || spec.max_block_frames == 0 {
            return Err(AudioExportError::InvalidDspParameter);
        }
        self.prepared = Some(spec);
        Ok(())
    }

    pub fn reset(&mut self) {
        self.spec = self.default;
    }

    #[must_use]
    pub fn latency_frames(&self) -> u32 {
        0
    }

    #[must_use]
    pub fn is_prepared(&self) -> bool {
        self.prepared.is_some()
    }

    #[must_use]
    pub fn spec(&self) -> NativeEffectModuleSpec {
        self.spec
    }

    pub fn set_bypassed(&mut self, bypassed: bool) {
        self.spec.bypassed = bypassed;
    }

    pub fn set_parameter(
        &mut self,
        value: NativeEffectParameterValue,
    ) -> Result<(), AudioExportError> {
        match (&mut self.spec.kind, value) {
            (DspDeviceKind::Gain { gain }, NativeEffectParameterValue::Gain(value)) => {
                if !value.is_finite() || value < 0.0 {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *gain = value;
                Ok(())
            }
            (DspDeviceKind::Pan { pan }, NativeEffectParameterValue::Pan(value)) => {
                if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *pan = value;
                Ok(())
            }
            _ => Err(AudioExportError::InvalidDspParameter),
        }
    }

    pub fn process_in_place(&self, data: &mut [f32]) -> Result<(), AudioExportError> {
        let Some(prepared) = self.prepared else {
            return Err(AudioExportError::InvalidDspParameter);
        };
        let channels = usize::from(prepared.channels);
        if channels == 0 || !data.len().is_multiple_of(channels) {
            return Err(AudioExportError::InvalidBufferLength {
                expected: data.len().next_multiple_of(channels.max(1)),
                actual: data.len(),
            });
        }
        let frames = data.len() / channels;
        if frames > prepared.max_block_frames {
            return Err(AudioExportError::InvalidBufferLength {
                expected: prepared.max_block_frames.saturating_mul(channels),
                actual: data.len(),
            });
        }
        if self.spec.bypassed {
            return Ok(());
        }
        match self.spec.kind {
            DspDeviceKind::Gain { gain } if gain.is_finite() && gain >= 0.0 => {
                for sample in data {
                    *sample *= gain;
                }
                Ok(())
            }
            DspDeviceKind::Pan { pan } if pan.is_finite() && (-1.0..=1.0).contains(&pan) => {
                process_pan(data, channels, pan);
                Ok(())
            }
            _ => Err(AudioExportError::InvalidDspParameter),
        }
    }
}

fn process_pan(data: &mut [f32], channels: usize, pan: f32) {
    if channels < 2 {
        return;
    }
    for frame in data.chunks_exact_mut(channels) {
        if pan > 0.0 {
            frame[0] *= 1.0 - pan;
        } else if pan < 0.0 {
            frame[1] *= 1.0 + pan;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepare_stereo(module: &mut NativeEffectModule) {
        module
            .prepare(NativeModulePrepareSpec {
                sample_rate: 48_000,
                channels: 2,
                max_block_frames: 16,
            })
            .expect("prepare module");
    }

    #[test]
    fn native_effect_module_lifecycle_processes_gain_and_reset() {
        let mut module = NativeEffectModule::new(NativeEffectModuleSpec {
            bypassed: false,
            kind: DspDeviceKind::Gain { gain: 0.5 },
        });
        prepare_stereo(&mut module);

        let mut data = vec![1.0, -1.0, 0.5, -0.5];
        module.process_in_place(&mut data).expect("process gain");
        assert_eq!(data, vec![0.5, -0.5, 0.25, -0.25]);

        module
            .set_parameter(NativeEffectParameterValue::Gain(0.25))
            .expect("set gain");
        module.reset();
        assert_eq!(module.spec().kind, DspDeviceKind::Gain { gain: 0.5 });
        assert_eq!(module.latency_frames(), 0);
    }

    #[test]
    fn native_effect_module_bypass_leaves_buffer_unchanged() {
        let mut module = NativeEffectModule::new(NativeEffectModuleSpec {
            bypassed: true,
            kind: DspDeviceKind::Gain { gain: 0.0 },
        });
        prepare_stereo(&mut module);

        let mut data = vec![1.0, -1.0];
        module.process_in_place(&mut data).expect("bypassed");
        assert_eq!(data, vec![1.0, -1.0]);
    }

    #[test]
    fn native_effect_module_processes_pan_deterministically() {
        let mut realtime = NativeEffectModule::new(NativeEffectModuleSpec {
            bypassed: false,
            kind: DspDeviceKind::Pan { pan: 0.5 },
        });
        let mut offline = realtime.clone();
        prepare_stereo(&mut realtime);
        prepare_stereo(&mut offline);

        let mut realtime_data = vec![1.0, 1.0, 0.5, 0.5];
        let mut offline_data = realtime_data.clone();
        realtime
            .process_in_place(&mut realtime_data)
            .expect("realtime process");
        offline
            .process_in_place(&mut offline_data)
            .expect("offline process");

        assert_eq!(realtime_data, offline_data);
        assert_eq!(realtime_data, vec![0.5, 1.0, 0.25, 0.5]);
    }
}
