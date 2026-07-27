//! Host-side WebAssembly DSP ABI contract.
//!
//! This module defines and tests the buffer/parameter contract trk would use
//! for future generated WebAssembly audio modules. It intentionally does not
//! embed a desktop WASM runtime; native Rust/C wrappers remain the realtime path
//! until scheduling and overhead are measured.

pub const WASM_DSP_ABI_VERSION: u32 = 1;
pub const WASM_DSP_MAX_CHANNELS: u16 = 2;
pub const WASM_DSP_MAX_BLOCK_FRAMES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WasmDspAbiSpec {
    pub abi_version: u32,
    pub sample_rate: u32,
    pub channels: u16,
    pub block_frames: usize,
    pub parameter_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WasmDspParameterValue {
    pub index: usize,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WasmDspAbiError {
    #[error("unsupported WASM DSP ABI version {0}")]
    UnsupportedAbiVersion(u32),
    #[error("unsupported WASM DSP sample rate {0}")]
    UnsupportedSampleRate(u32),
    #[error("unsupported WASM DSP channel count {0}")]
    UnsupportedChannelCount(u16),
    #[error("unsupported WASM DSP block frame count {0}")]
    UnsupportedBlockFrames(usize),
    #[error("WASM DSP input has {actual} samples, expected {expected}")]
    InputLengthMismatch { expected: usize, actual: usize },
    #[error("WASM DSP output has {actual} samples, expected {expected}")]
    OutputLengthMismatch { expected: usize, actual: usize },
    #[error("WASM DSP parameter count mismatch: expected {expected}, got {actual}")]
    ParameterCountMismatch { expected: usize, actual: usize },
    #[error("WASM DSP parameter index mismatch at slot {slot}: expected {expected}, got {actual}")]
    ParameterIndexMismatch {
        slot: usize,
        expected: usize,
        actual: usize,
    },
    #[error("WASM DSP non-finite parameter at index {index}")]
    NonFiniteParameter { index: usize },
    #[error("WASM DSP non-finite sample at index {index}")]
    NonFiniteSample { index: usize },
}

impl Default for WasmDspAbiSpec {
    fn default() -> Self {
        Self {
            abi_version: WASM_DSP_ABI_VERSION,
            sample_rate: 48_000,
            channels: 2,
            block_frames: 128,
            parameter_count: 1,
        }
    }
}

pub fn validate_wasm_dsp_render_request(
    spec: WasmDspAbiSpec,
    input: &[f32],
    output: &[f32],
    parameters: &[WasmDspParameterValue],
) -> Result<(), WasmDspAbiError> {
    validate_wasm_dsp_abi_spec(spec)?;
    let expected_samples = spec.block_frames.saturating_mul(usize::from(spec.channels));
    if input.len() != expected_samples {
        return Err(WasmDspAbiError::InputLengthMismatch {
            expected: expected_samples,
            actual: input.len(),
        });
    }
    if output.len() != expected_samples {
        return Err(WasmDspAbiError::OutputLengthMismatch {
            expected: expected_samples,
            actual: output.len(),
        });
    }
    validate_wasm_dsp_parameters(spec.parameter_count, parameters)?;
    validate_finite_samples(input)?;
    validate_finite_samples(output)?;
    Ok(())
}

pub fn render_wasm_dsp_gain_fixture(
    spec: WasmDspAbiSpec,
    input: &[f32],
    output: &mut [f32],
    parameters: &[WasmDspParameterValue],
) -> Result<(), WasmDspAbiError> {
    validate_wasm_dsp_render_request(spec, input, output, parameters)?;
    let gain = parameters.first().map_or(1.0, |parameter| parameter.value);
    for (source, target) in input.iter().zip(output.iter_mut()) {
        *target = *source * gain;
        if !target.is_finite() {
            return Err(WasmDspAbiError::NonFiniteSample { index: 0 });
        }
    }
    Ok(())
}

fn validate_wasm_dsp_abi_spec(spec: WasmDspAbiSpec) -> Result<(), WasmDspAbiError> {
    if spec.abi_version != WASM_DSP_ABI_VERSION {
        return Err(WasmDspAbiError::UnsupportedAbiVersion(spec.abi_version));
    }
    if spec.sample_rate == 0 {
        return Err(WasmDspAbiError::UnsupportedSampleRate(spec.sample_rate));
    }
    if spec.channels == 0 || spec.channels > WASM_DSP_MAX_CHANNELS {
        return Err(WasmDspAbiError::UnsupportedChannelCount(spec.channels));
    }
    if spec.block_frames == 0 || spec.block_frames > WASM_DSP_MAX_BLOCK_FRAMES {
        return Err(WasmDspAbiError::UnsupportedBlockFrames(spec.block_frames));
    }
    Ok(())
}

fn validate_wasm_dsp_parameters(
    expected_count: usize,
    parameters: &[WasmDspParameterValue],
) -> Result<(), WasmDspAbiError> {
    if parameters.len() != expected_count {
        return Err(WasmDspAbiError::ParameterCountMismatch {
            expected: expected_count,
            actual: parameters.len(),
        });
    }
    for (slot, parameter) in parameters.iter().enumerate() {
        if parameter.index != slot {
            return Err(WasmDspAbiError::ParameterIndexMismatch {
                slot,
                expected: slot,
                actual: parameter.index,
            });
        }
        if !parameter.value.is_finite() {
            return Err(WasmDspAbiError::NonFiniteParameter {
                index: parameter.index,
            });
        }
    }
    Ok(())
}

fn validate_finite_samples(samples: &[f32]) -> Result<(), WasmDspAbiError> {
    for (index, sample) in samples.iter().enumerate() {
        if !sample.is_finite() {
            return Err(WasmDspAbiError::NonFiniteSample { index });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> WasmDspAbiSpec {
        WasmDspAbiSpec {
            block_frames: 4,
            ..WasmDspAbiSpec::default()
        }
    }

    fn parameters() -> [WasmDspParameterValue; 1] {
        [WasmDspParameterValue {
            index: 0,
            value: 0.5,
        }]
    }

    #[test]
    fn wasm_dsp_gain_fixture_renders_deterministic_offline_buffers() {
        let spec = spec();
        let input = [1.0, -1.0, 0.5, -0.5, 0.25, -0.25, 0.0, 0.75];
        let mut left = [0.0; 8];
        let mut right = [0.0; 8];

        render_wasm_dsp_gain_fixture(spec, &input, &mut left, &parameters()).expect("render left");
        render_wasm_dsp_gain_fixture(spec, &input, &mut right, &parameters())
            .expect("render right");

        assert_eq!(left, right);
        assert_eq!(left, [0.5, -0.5, 0.25, -0.25, 0.125, -0.125, 0.0, 0.375]);
    }

    #[test]
    fn wasm_dsp_abi_rejects_unsupported_channel_counts_and_block_sizes() {
        let input = [0.0; 8];
        let output = [0.0; 8];
        assert!(matches!(
            validate_wasm_dsp_render_request(
                WasmDspAbiSpec {
                    channels: 3,
                    ..spec()
                },
                &input,
                &output,
                &parameters()
            ),
            Err(WasmDspAbiError::UnsupportedChannelCount(3))
        ));
        assert!(matches!(
            validate_wasm_dsp_render_request(
                WasmDspAbiSpec {
                    block_frames: WASM_DSP_MAX_BLOCK_FRAMES + 1,
                    ..spec()
                },
                &input,
                &output,
                &parameters()
            ),
            Err(WasmDspAbiError::UnsupportedBlockFrames(_))
        ));
    }

    #[test]
    fn wasm_dsp_abi_rejects_non_finite_samples_and_parameters() {
        let mut input = [0.0; 8];
        let output = [0.0; 8];
        input[3] = f32::NAN;
        assert!(matches!(
            validate_wasm_dsp_render_request(spec(), &input, &output, &parameters()),
            Err(WasmDspAbiError::NonFiniteSample { index: 3 })
        ));

        assert!(matches!(
            validate_wasm_dsp_render_request(
                spec(),
                &[0.0; 8],
                &output,
                &[WasmDspParameterValue {
                    index: 0,
                    value: f32::INFINITY,
                }]
            ),
            Err(WasmDspAbiError::NonFiniteParameter { index: 0 })
        ));
    }

    #[test]
    fn wasm_dsp_abi_rejects_parameter_mismatches_and_buffer_shapes() {
        let input = [0.0; 8];
        let output = [0.0; 8];
        assert!(matches!(
            validate_wasm_dsp_render_request(spec(), &input, &output, &[]),
            Err(WasmDspAbiError::ParameterCountMismatch {
                expected: 1,
                actual: 0
            })
        ));
        assert!(matches!(
            validate_wasm_dsp_render_request(
                spec(),
                &input,
                &output,
                &[WasmDspParameterValue {
                    index: 1,
                    value: 0.5,
                }]
            ),
            Err(WasmDspAbiError::ParameterIndexMismatch {
                slot: 0,
                expected: 0,
                actual: 1
            })
        ));
        assert!(matches!(
            validate_wasm_dsp_render_request(spec(), &[0.0; 7], &output, &parameters()),
            Err(WasmDspAbiError::InputLengthMismatch {
                expected: 8,
                actual: 7
            })
        ));
    }
}
