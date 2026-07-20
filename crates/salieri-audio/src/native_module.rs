use crate::{
    dsp::{DspDeviceKind, DspDeviceSpec, DspFilterMode, DspFrameProcessor},
    errors::AudioExportError,
};

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
    Balance(f32),
    StereoWidth(f32),
    PhaseInvertLeft(bool),
    PhaseInvertRight(bool),
    FilterMode(DspFilterMode),
    FilterCutoffHz(f32),
    FilterResonance(f32),
    FilterDriveDb(f32),
    FilterKeyTrack(f32),
    FilterEnvAmount(f32),
    FilterMix(f32),
    DelaySync(bool),
    DelayTimeLeftMs(f32),
    DelayTimeRightMs(f32),
    DelayLinkTimes(bool),
    DelayFeedback(f32),
    DelayPingPong(bool),
    DelayFilterLowCutHz(f32),
    DelayFilterHighCutHz(f32),
    DelayModRateHz(f32),
    DelayModDepth(f32),
    DelayMix(f32),
    DelayOutputDb(f32),
    ReverbSize(f32),
    ReverbPredelayMs(f32),
    ReverbDecayS(f32),
    ReverbDamping(f32),
    ReverbLowCutHz(f32),
    ReverbHighCutHz(f32),
    ReverbDiffusion(f32),
    ReverbWidth(f32),
    ReverbEarlyReflections(f32),
    ReverbMix(f32),
    ReverbOutputDb(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeEffectModule {
    spec: NativeEffectModuleSpec,
    default: NativeEffectModuleSpec,
    smoothed_kind: DspDeviceKind,
    processor: DspFrameProcessor,
    prepared: Option<NativeModulePrepareSpec>,
}

impl NativeEffectModule {
    #[must_use]
    pub fn new(spec: NativeEffectModuleSpec) -> Self {
        Self {
            spec,
            default: spec,
            smoothed_kind: spec.kind,
            processor: DspFrameProcessor::default(),
            prepared: None,
        }
    }

    pub fn prepare(&mut self, spec: NativeModulePrepareSpec) -> Result<(), AudioExportError> {
        if spec.sample_rate == 0 || spec.channels == 0 || spec.max_block_frames == 0 {
            return Err(AudioExportError::InvalidDspParameter);
        }
        let device = DspDeviceSpec {
            bypassed: self.spec.bypassed,
            kind: self.spec.kind,
        };
        self.processor
            .prepare(spec.sample_rate, usize::from(spec.channels), &[device]);
        self.prepared = Some(spec);
        Ok(())
    }

    pub fn reset(&mut self) {
        self.spec = self.default;
        self.smoothed_kind = self.default.kind;
        self.processor = DspFrameProcessor::default();
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
            (DspDeviceKind::Balance { balance }, NativeEffectParameterValue::Balance(value)) => {
                if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *balance = value;
                Ok(())
            }
            (
                DspDeviceKind::StereoWidth { width },
                NativeEffectParameterValue::StereoWidth(value),
            ) => {
                if !value.is_finite() || !(0.0..=2.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *width = value;
                Ok(())
            }
            (
                DspDeviceKind::PhaseInvert { invert_left, .. },
                NativeEffectParameterValue::PhaseInvertLeft(value),
            ) => {
                *invert_left = value;
                Ok(())
            }
            (
                DspDeviceKind::PhaseInvert { invert_right, .. },
                NativeEffectParameterValue::PhaseInvertRight(value),
            ) => {
                *invert_right = value;
                Ok(())
            }
            (DspDeviceKind::Filter { mode, .. }, NativeEffectParameterValue::FilterMode(value)) => {
                *mode = value;
                Ok(())
            }
            (
                DspDeviceKind::Filter { cutoff_hz, .. },
                NativeEffectParameterValue::FilterCutoffHz(value),
            ) => {
                if !value.is_finite() || !(20.0..=24_000.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *cutoff_hz = value;
                Ok(())
            }
            (
                DspDeviceKind::Filter { resonance, .. },
                NativeEffectParameterValue::FilterResonance(value),
            ) => {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *resonance = value;
                Ok(())
            }
            (
                DspDeviceKind::Filter { drive_db, .. },
                NativeEffectParameterValue::FilterDriveDb(value),
            ) => {
                if !value.is_finite() || !(0.0..=24.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *drive_db = value;
                Ok(())
            }
            (
                DspDeviceKind::Filter { key_track, .. },
                NativeEffectParameterValue::FilterKeyTrack(value),
            ) => {
                if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *key_track = value;
                Ok(())
            }
            (
                DspDeviceKind::Filter { env_amount, .. },
                NativeEffectParameterValue::FilterEnvAmount(value),
            ) => {
                if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *env_amount = value;
                Ok(())
            }
            (DspDeviceKind::Filter { mix, .. }, NativeEffectParameterValue::FilterMix(value)) => {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *mix = value;
                Ok(())
            }
            (DspDeviceKind::Delay { sync, .. }, NativeEffectParameterValue::DelaySync(value)) => {
                *sync = value;
                Ok(())
            }
            (
                DspDeviceKind::Delay { time_left_ms, .. },
                NativeEffectParameterValue::DelayTimeLeftMs(value),
            ) => {
                if !value.is_finite() || !(1.0..=4_000.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *time_left_ms = value;
                Ok(())
            }
            (
                DspDeviceKind::Delay { time_right_ms, .. },
                NativeEffectParameterValue::DelayTimeRightMs(value),
            ) => {
                if !value.is_finite() || !(1.0..=4_000.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *time_right_ms = value;
                Ok(())
            }
            (
                DspDeviceKind::Delay { link_times, .. },
                NativeEffectParameterValue::DelayLinkTimes(value),
            ) => {
                *link_times = value;
                Ok(())
            }
            (
                DspDeviceKind::Delay { feedback, .. },
                NativeEffectParameterValue::DelayFeedback(value),
            ) => {
                if !value.is_finite() || !(0.0..=0.95).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *feedback = value;
                Ok(())
            }
            (
                DspDeviceKind::Delay { ping_pong, .. },
                NativeEffectParameterValue::DelayPingPong(value),
            ) => {
                *ping_pong = value;
                Ok(())
            }
            (
                DspDeviceKind::Delay {
                    filter_low_cut_hz, ..
                },
                NativeEffectParameterValue::DelayFilterLowCutHz(value),
            ) => {
                if !value.is_finite() || !(20.0..=20_000.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *filter_low_cut_hz = value;
                Ok(())
            }
            (
                DspDeviceKind::Delay {
                    filter_high_cut_hz, ..
                },
                NativeEffectParameterValue::DelayFilterHighCutHz(value),
            ) => {
                if !value.is_finite() || !(20.0..=20_000.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *filter_high_cut_hz = value;
                Ok(())
            }
            (
                DspDeviceKind::Delay { mod_rate_hz, .. },
                NativeEffectParameterValue::DelayModRateHz(value),
            ) => {
                if !value.is_finite() || !(0.0..=20.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *mod_rate_hz = value;
                Ok(())
            }
            (
                DspDeviceKind::Delay { mod_depth, .. },
                NativeEffectParameterValue::DelayModDepth(value),
            ) => {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *mod_depth = value;
                Ok(())
            }
            (DspDeviceKind::Delay { mix, .. }, NativeEffectParameterValue::DelayMix(value)) => {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *mix = value;
                Ok(())
            }
            (
                DspDeviceKind::Delay { output_db, .. },
                NativeEffectParameterValue::DelayOutputDb(value),
            ) => {
                if !value.is_finite() || !(-60.0..=12.0).contains(&value) {
                    return Err(AudioExportError::InvalidDspParameter);
                }
                *output_db = value;
                Ok(())
            }
            (DspDeviceKind::Reverb { size, .. }, NativeEffectParameterValue::ReverbSize(value)) => {
                set_reverb_percentage(size, value, 1.0)
            }
            (
                DspDeviceKind::Reverb { predelay_ms, .. },
                NativeEffectParameterValue::ReverbPredelayMs(value),
            ) => set_reverb_range(predelay_ms, value, 0.0, 250.0),
            (
                DspDeviceKind::Reverb { decay_s, .. },
                NativeEffectParameterValue::ReverbDecayS(value),
            ) => set_reverb_range(decay_s, value, 0.1, 30.0),
            (
                DspDeviceKind::Reverb { damping, .. },
                NativeEffectParameterValue::ReverbDamping(value),
            ) => set_reverb_percentage(damping, value, 1.0),
            (
                DspDeviceKind::Reverb { low_cut_hz, .. },
                NativeEffectParameterValue::ReverbLowCutHz(value),
            ) => set_reverb_range(low_cut_hz, value, 20.0, 2_000.0),
            (
                DspDeviceKind::Reverb { high_cut_hz, .. },
                NativeEffectParameterValue::ReverbHighCutHz(value),
            ) => set_reverb_range(high_cut_hz, value, 1_000.0, 20_000.0),
            (
                DspDeviceKind::Reverb { diffusion, .. },
                NativeEffectParameterValue::ReverbDiffusion(value),
            ) => set_reverb_percentage(diffusion, value, 1.0),
            (
                DspDeviceKind::Reverb { width, .. },
                NativeEffectParameterValue::ReverbWidth(value),
            ) => set_reverb_percentage(width, value, 2.0),
            (
                DspDeviceKind::Reverb {
                    early_reflections, ..
                },
                NativeEffectParameterValue::ReverbEarlyReflections(value),
            ) => set_reverb_percentage(early_reflections, value, 1.0),
            (DspDeviceKind::Reverb { mix, .. }, NativeEffectParameterValue::ReverbMix(value)) => {
                set_reverb_percentage(mix, value, 1.0)
            }
            (
                DspDeviceKind::Reverb { output_db, .. },
                NativeEffectParameterValue::ReverbOutputDb(value),
            ) => set_reverb_range(output_db, value, -60.0, 12.0),
            _ => Err(AudioExportError::InvalidDspParameter),
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
        if self.spec.bypassed {
            return Ok(());
        }
        let sample_rate = prepared.sample_rate;
        for frame in data.chunks_exact_mut(channels) {
            self.smoothed_kind = smooth_kind(self.smoothed_kind, self.spec.kind);
            let device = DspDeviceSpec {
                bypassed: false,
                kind: self.smoothed_kind,
            };
            self.processor.process_frame(frame, sample_rate, &[device]);
        }
        Ok(())
    }
}

fn smooth_kind(current: DspDeviceKind, target: DspDeviceKind) -> DspDeviceKind {
    const SMOOTHING: f32 = 0.01;
    match (current, target) {
        (DspDeviceKind::Gain { gain }, DspDeviceKind::Gain { gain: target }) => {
            DspDeviceKind::Gain {
                gain: smooth_value(gain, target, SMOOTHING),
            }
        }
        (DspDeviceKind::Pan { pan }, DspDeviceKind::Pan { pan: target }) => DspDeviceKind::Pan {
            pan: smooth_value(pan, target, SMOOTHING),
        },
        (DspDeviceKind::Balance { balance }, DspDeviceKind::Balance { balance: target }) => {
            DspDeviceKind::Balance {
                balance: smooth_value(balance, target, SMOOTHING),
            }
        }
        (DspDeviceKind::StereoWidth { width }, DspDeviceKind::StereoWidth { width: target }) => {
            DspDeviceKind::StereoWidth {
                width: smooth_value(width, target, SMOOTHING),
            }
        }
        (
            DspDeviceKind::Filter {
                mode: _,
                cutoff_hz,
                resonance,
                drive_db,
                key_track,
                env_amount,
                mix,
            },
            DspDeviceKind::Filter {
                mode: target_mode,
                cutoff_hz: target_cutoff,
                resonance: target_resonance,
                drive_db: target_drive,
                key_track: target_key_track,
                env_amount: target_env_amount,
                mix: target_mix,
            },
        ) => DspDeviceKind::Filter {
            mode: target_mode,
            cutoff_hz: smooth_value(cutoff_hz, target_cutoff, SMOOTHING),
            resonance: smooth_value(resonance, target_resonance, SMOOTHING),
            drive_db: smooth_value(drive_db, target_drive, SMOOTHING),
            key_track: smooth_value(key_track, target_key_track, SMOOTHING),
            env_amount: smooth_value(env_amount, target_env_amount, SMOOTHING),
            mix: smooth_value(mix, target_mix, SMOOTHING),
        },
        (
            DspDeviceKind::Delay {
                sync: _,
                time_left_ms,
                time_right_ms,
                link_times: _,
                feedback,
                ping_pong: _,
                filter_low_cut_hz,
                filter_high_cut_hz,
                mod_rate_hz,
                mod_depth,
                mix,
                output_db,
            },
            DspDeviceKind::Delay {
                sync: target_sync,
                time_left_ms: target_time_left,
                time_right_ms: target_time_right,
                link_times: target_link_times,
                feedback: target_feedback,
                ping_pong: target_ping_pong,
                filter_low_cut_hz: target_low_cut,
                filter_high_cut_hz: target_high_cut,
                mod_rate_hz: target_mod_rate,
                mod_depth: target_mod_depth,
                mix: target_mix,
                output_db: target_output,
            },
        ) => DspDeviceKind::Delay {
            sync: target_sync,
            time_left_ms: smooth_value(time_left_ms, target_time_left, SMOOTHING),
            time_right_ms: smooth_value(time_right_ms, target_time_right, SMOOTHING),
            link_times: target_link_times,
            feedback: smooth_value(feedback, target_feedback, SMOOTHING),
            ping_pong: target_ping_pong,
            filter_low_cut_hz: smooth_value(filter_low_cut_hz, target_low_cut, SMOOTHING),
            filter_high_cut_hz: smooth_value(filter_high_cut_hz, target_high_cut, SMOOTHING),
            mod_rate_hz: smooth_value(mod_rate_hz, target_mod_rate, SMOOTHING),
            mod_depth: smooth_value(mod_depth, target_mod_depth, SMOOTHING),
            mix: smooth_value(mix, target_mix, SMOOTHING),
            output_db: smooth_value(output_db, target_output, SMOOTHING),
        },
        (
            DspDeviceKind::Reverb {
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
            },
            DspDeviceKind::Reverb {
                size: target_size,
                predelay_ms: target_predelay,
                decay_s: target_decay,
                damping: target_damping,
                low_cut_hz: target_low_cut,
                high_cut_hz: target_high_cut,
                diffusion: target_diffusion,
                width: target_width,
                early_reflections: target_early,
                mix: target_mix,
                output_db: target_output,
            },
        ) => DspDeviceKind::Reverb {
            size: smooth_value(size, target_size, SMOOTHING),
            predelay_ms: smooth_value(predelay_ms, target_predelay, SMOOTHING),
            decay_s: smooth_value(decay_s, target_decay, SMOOTHING),
            damping: smooth_value(damping, target_damping, SMOOTHING),
            low_cut_hz: smooth_value(low_cut_hz, target_low_cut, SMOOTHING),
            high_cut_hz: smooth_value(high_cut_hz, target_high_cut, SMOOTHING),
            diffusion: smooth_value(diffusion, target_diffusion, SMOOTHING),
            width: smooth_value(width, target_width, SMOOTHING),
            early_reflections: smooth_value(early_reflections, target_early, SMOOTHING),
            mix: smooth_value(mix, target_mix, SMOOTHING),
            output_db: smooth_value(output_db, target_output, SMOOTHING),
        },
        _ => target,
    }
}

fn set_reverb_percentage(target: &mut f32, value: f32, max: f32) -> Result<(), AudioExportError> {
    set_reverb_range(target, value, 0.0, max)
}

fn set_reverb_range(
    target: &mut f32,
    value: f32,
    min: f32,
    max: f32,
) -> Result<(), AudioExportError> {
    if !value.is_finite() || !(min..=max).contains(&value) {
        return Err(AudioExportError::InvalidDspParameter);
    }
    *target = value;
    Ok(())
}

fn smooth_value(current: f32, target: f32, amount: f32) -> f32 {
    current + (target - current) * amount
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

    #[test]
    fn native_effect_module_processes_balance_width_and_phase() {
        let mut balance = NativeEffectModule::new(NativeEffectModuleSpec {
            bypassed: false,
            kind: DspDeviceKind::Balance { balance: -0.5 },
        });
        prepare_stereo(&mut balance);
        let mut balance_data = vec![1.0, 1.0, 0.5, 0.5];
        balance
            .process_in_place(&mut balance_data)
            .expect("process balance");
        assert_eq!(balance_data, vec![1.0, 0.5, 0.5, 0.25]);

        let mut width = NativeEffectModule::new(NativeEffectModuleSpec {
            bypassed: false,
            kind: DspDeviceKind::StereoWidth { width: 0.0 },
        });
        prepare_stereo(&mut width);
        let mut width_data = vec![1.0, -1.0, 0.25, 0.75];
        width
            .process_in_place(&mut width_data)
            .expect("collapse width");
        assert_eq!(width_data, vec![0.0, 0.0, 0.5, 0.5]);

        let mut width = NativeEffectModule::new(NativeEffectModuleSpec {
            bypassed: false,
            kind: DspDeviceKind::StereoWidth { width: 2.0 },
        });
        prepare_stereo(&mut width);
        let mut wide_data = vec![0.75, 0.25];
        width
            .process_in_place(&mut wide_data)
            .expect("expand width");
        assert_eq!(wide_data, vec![1.0, 0.0]);

        let mut phase = NativeEffectModule::new(NativeEffectModuleSpec {
            bypassed: false,
            kind: DspDeviceKind::PhaseInvert {
                invert_left: true,
                invert_right: false,
            },
        });
        prepare_stereo(&mut phase);
        let mut phase_data = vec![1.0, -0.5, -0.25, 0.75];
        phase
            .process_in_place(&mut phase_data)
            .expect("process phase");
        assert_eq!(phase_data, vec![-1.0, -0.5, 0.25, 0.75]);
    }

    #[test]
    fn native_effect_module_utility_devices_handle_mono_silence_and_invalid_values() {
        let mut width = NativeEffectModule::new(NativeEffectModuleSpec {
            bypassed: false,
            kind: DspDeviceKind::StereoWidth { width: 2.0 },
        });
        width
            .prepare(NativeModulePrepareSpec {
                sample_rate: 48_000,
                channels: 1,
                max_block_frames: 16,
            })
            .expect("prepare mono");
        let mut mono = vec![0.25, -0.5, 0.0];
        width.process_in_place(&mut mono).expect("mono width no-op");
        assert_eq!(mono, vec![0.25, -0.5, 0.0]);

        let mut phase = NativeEffectModule::new(NativeEffectModuleSpec {
            bypassed: false,
            kind: DspDeviceKind::PhaseInvert {
                invert_left: true,
                invert_right: true,
            },
        });
        prepare_stereo(&mut phase);
        let mut silence = vec![0.0, 0.0, 0.0, 0.0];
        phase.process_in_place(&mut silence).expect("phase silence");
        assert_eq!(silence, vec![0.0, -0.0, 0.0, -0.0]);

        assert!(matches!(
            width.set_parameter(NativeEffectParameterValue::StereoWidth(2.1)),
            Err(AudioExportError::InvalidDspParameter)
        ));
    }

    #[test]
    fn native_effect_module_processes_multimode_filter_stably() {
        for mode in [
            DspFilterMode::LowPass,
            DspFilterMode::HighPass,
            DspFilterMode::BandPass,
            DspFilterMode::Notch,
        ] {
            let mut module = NativeEffectModule::new(NativeEffectModuleSpec {
                bypassed: false,
                kind: DspDeviceKind::Filter {
                    mode,
                    cutoff_hz: 1_000.0,
                    resonance: 0.5,
                    drive_db: 6.0,
                    key_track: 0.0,
                    env_amount: 0.0,
                    mix: 1.0,
                },
            });
            prepare_stereo(&mut module);

            let mut data = vec![1.0, -1.0, 0.5, -0.5, 0.25, -0.25, 0.0, 0.0];
            module.process_in_place(&mut data).expect("process filter");

            assert!(data.iter().all(|sample| sample.is_finite()));
            assert_ne!(data, vec![1.0, -1.0, 0.5, -0.5, 0.25, -0.25, 0.0, 0.0]);
        }
    }

    #[test]
    fn native_effect_module_smooths_filter_parameter_changes() {
        let mut module = NativeEffectModule::new(NativeEffectModuleSpec {
            bypassed: false,
            kind: DspDeviceKind::Filter {
                mode: DspFilterMode::LowPass,
                cutoff_hz: 200.0,
                resonance: 0.0,
                drive_db: 0.0,
                key_track: 0.0,
                env_amount: 0.0,
                mix: 1.0,
            },
        });
        prepare_stereo(&mut module);

        module
            .set_parameter(NativeEffectParameterValue::FilterCutoffHz(20_000.0))
            .expect("set cutoff");
        module
            .set_parameter(NativeEffectParameterValue::FilterResonance(1.0))
            .expect("set resonance");

        let mut data = vec![1.0; 16];
        module
            .process_in_place(&mut data)
            .expect("process smoothed");

        assert!(data.iter().all(|sample| sample.is_finite()));
        assert!(matches!(
            module.set_parameter(NativeEffectParameterValue::FilterMix(1.5)),
            Err(AudioExportError::InvalidDspParameter)
        ));
    }
}
#[cfg(test)]
#[path = "native_module_stateful_tests.rs"]
mod stateful_tests;
