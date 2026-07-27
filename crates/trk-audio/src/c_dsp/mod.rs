//! Controlled C/C++ DSP integration boundary.
//!
//! This module is intentionally feature-gated and only wraps trk-owned or
//! reviewed native DSP code. It is not a dynamic third-party binary host.

use crate::{errors::AudioExportError, NativeModulePrepareSpec};

#[cfg(test)]
mod tests;

pub const C_NATIVE_GAIN_MODULE_ID: &str = "native.effect.cGainPoc";
pub const C_NATIVE_GAIN_GAIN_PARAMETER_ID: &str = "native.cGainPoc.gain";

#[derive(Debug, Clone, PartialEq)]
pub struct CNativeGainDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub parameters: Vec<CNativeGainParameterDescriptor>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CNativeGainParameterDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub unit: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CNativeGainState {
    pub gain: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CNativeGainSpec {
    pub bypassed: bool,
    pub state: CNativeGainState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CNativeGainParameterValue {
    Gain(f32),
}

pub struct CNativeGainModule {
    spec: CNativeGainSpec,
    default: CNativeGainSpec,
    ffi_state: ffi::TrkCGainState,
    prepared: Option<NativeModulePrepareSpec>,
}

impl CNativeGainDescriptor {
    #[must_use]
    pub fn poc_descriptor() -> Self {
        Self {
            id: C_NATIVE_GAIN_MODULE_ID,
            display_name: "C Gain PoC",
            parameters: vec![CNativeGainParameterDescriptor {
                id: C_NATIVE_GAIN_GAIN_PARAMETER_ID,
                display_name: "Gain",
                min: 0.0,
                max: 2.0,
                default: 1.0,
                unit: "gain",
            }],
        }
    }
}

impl Default for CNativeGainState {
    fn default() -> Self {
        Self { gain: 1.0 }
    }
}

impl CNativeGainModule {
    #[must_use]
    pub fn new(spec: CNativeGainSpec) -> Self {
        let mut module = Self {
            spec,
            default: spec,
            ffi_state: ffi::TrkCGainState::default(),
            prepared: None,
        };
        module.reset_ffi_state();
        module
    }

    pub fn prepare(&mut self, spec: NativeModulePrepareSpec) -> Result<(), AudioExportError> {
        if spec.sample_rate == 0 || spec.channels == 0 || spec.max_block_frames == 0 {
            return Err(AudioExportError::InvalidDspParameter);
        }
        let status = unsafe {
            ffi::trk_c_gain_prepare(
                &mut self.ffi_state,
                spec.sample_rate,
                spec.channels,
                spec.max_block_frames,
            )
        };
        if status != ffi::TRK_SUCCESS {
            return Err(AudioExportError::InvalidDspParameter);
        }
        self.apply_state_to_ffi()?;
        self.prepared = Some(spec);
        Ok(())
    }

    pub fn reset(&mut self) {
        self.spec = self.default;
        self.prepared = None;
        self.reset_ffi_state();
    }

    #[must_use]
    pub fn descriptor() -> CNativeGainDescriptor {
        CNativeGainDescriptor::poc_descriptor()
    }

    #[must_use]
    pub fn state(&self) -> CNativeGainState {
        self.spec.state
    }

    #[must_use]
    pub fn spec(&self) -> CNativeGainSpec {
        self.spec
    }

    #[must_use]
    pub fn is_prepared(&self) -> bool {
        self.prepared.is_some()
    }

    pub fn set_bypassed(&mut self, bypassed: bool) {
        self.spec.bypassed = bypassed;
    }

    pub fn set_parameter(
        &mut self,
        value: CNativeGainParameterValue,
    ) -> Result<(), AudioExportError> {
        match value {
            CNativeGainParameterValue::Gain(gain) => {
                validate_gain(gain)?;
                self.spec.state.gain = gain;
                self.apply_state_to_ffi()
            }
        }
    }

    pub fn process_in_place(&mut self, data: &mut [f32]) -> Result<(), AudioExportError> {
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
        if self.spec.bypassed || data.is_empty() {
            return Ok(());
        }
        let status = unsafe {
            ffi::trk_c_gain_process(
                &mut self.ffi_state,
                data.as_mut_ptr(),
                frames,
                prepared.channels,
            )
        };
        match status {
            ffi::TRK_SUCCESS => Ok(()),
            ffi::TRK_ERR_FRAME_OR_CHANNEL_MISMATCH => Err(AudioExportError::InvalidBufferLength {
                expected: prepared.max_block_frames.saturating_mul(channels),
                actual: data.len(),
            }),
            _ => Err(AudioExportError::InvalidDspParameter),
        }
    }

    fn reset_ffi_state(&mut self) {
        unsafe {
            ffi::trk_c_gain_reset(&mut self.ffi_state);
        }
        let _ = self.apply_state_to_ffi();
    }

    fn apply_state_to_ffi(&mut self) -> Result<(), AudioExportError> {
        validate_gain(self.spec.state.gain)?;
        let status = unsafe { ffi::trk_c_gain_set_gain(&mut self.ffi_state, self.spec.state.gain) };
        if status == ffi::TRK_SUCCESS {
            Ok(())
        } else {
            Err(AudioExportError::InvalidDspParameter)
        }
    }
}

fn validate_gain(gain: f32) -> Result<(), AudioExportError> {
    if gain.is_finite() && (0.0..=2.0).contains(&gain) {
        Ok(())
    } else {
        Err(AudioExportError::InvalidDspParameter)
    }
}

mod ffi {
    use std::os::raw::{c_float, c_int, c_uint};

    pub const TRK_SUCCESS: c_int = 0;
    pub const TRK_ERR_FRAME_OR_CHANNEL_MISMATCH: c_int = -3;

    #[repr(C)]
    pub struct TrkCGainState {
        gain: c_float,
        sample_rate: c_uint,
        channels: u16,
        max_block_frames: usize,
    }

    impl Default for TrkCGainState {
        fn default() -> Self {
            Self {
                gain: 1.0,
                sample_rate: 0,
                channels: 0,
                max_block_frames: 0,
            }
        }
    }

    unsafe extern "C" {
        pub fn trk_c_gain_reset(state: *mut TrkCGainState);
        pub fn trk_c_gain_prepare(
            state: *mut TrkCGainState,
            sample_rate: c_uint,
            channels: u16,
            max_block_frames: usize,
        ) -> c_int;
        pub fn trk_c_gain_set_gain(state: *mut TrkCGainState, gain: c_float) -> c_int;
        pub fn trk_c_gain_process(
            state: *mut TrkCGainState,
            interleaved: *mut c_float,
            frames: usize,
            channels: u16,
        ) -> c_int;
    }
}
