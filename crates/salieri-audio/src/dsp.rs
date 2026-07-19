use crate::errors::AudioExportError;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DspGraphSpec {
    pub track_chains: Vec<TrackDspChainSpec>,
    pub master: Vec<DspDeviceSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackDspChainSpec {
    pub track_id: u32,
    pub devices: Vec<DspDeviceSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DspDeviceSpec {
    pub bypassed: bool,
    pub kind: DspDeviceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DspFilterMode {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DspDeviceKind {
    Gain {
        gain: f32,
    },
    Pan {
        pan: f32,
    },
    Balance {
        balance: f32,
    },
    StereoWidth {
        width: f32,
    },
    PhaseInvert {
        invert_left: bool,
        invert_right: bool,
    },
    Filter {
        mode: DspFilterMode,
        cutoff_hz: f32,
        resonance: f32,
        drive_db: f32,
        key_track: f32,
        env_amount: f32,
        mix: f32,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MixParams {
    pub(crate) pitch_ratio: f32,
    pub(crate) level: f32,
    pub(crate) pan: f32,
}

pub(crate) fn track_dsp_chain(track_id: u32, graph: &DspGraphSpec) -> &[DspDeviceSpec] {
    graph
        .track_chains
        .iter()
        .find(|chain| chain.track_id == track_id)
        .map_or(&[], |chain| chain.devices.as_slice())
}

pub(crate) fn validate_dsp_chain(devices: &[DspDeviceSpec]) -> Result<(), AudioExportError> {
    for device in devices {
        if device.bypassed {
            continue;
        }
        match device.kind {
            DspDeviceKind::Gain { gain } if gain.is_finite() && gain >= 0.0 => {}
            DspDeviceKind::Pan { pan } if pan.is_finite() && (-1.0..=1.0).contains(&pan) => {}
            DspDeviceKind::Balance { balance }
                if balance.is_finite() && (-1.0..=1.0).contains(&balance) => {}
            DspDeviceKind::StereoWidth { width }
                if width.is_finite() && (0.0..=2.0).contains(&width) => {}
            DspDeviceKind::PhaseInvert { .. } => {}
            DspDeviceKind::Filter {
                cutoff_hz,
                resonance,
                drive_db,
                key_track,
                env_amount,
                mix,
                ..
            } if cutoff_hz.is_finite()
                && (20.0..=24_000.0).contains(&cutoff_hz)
                && resonance.is_finite()
                && (0.0..=1.0).contains(&resonance)
                && drive_db.is_finite()
                && (0.0..=24.0).contains(&drive_db)
                && key_track.is_finite()
                && (-1.0..=1.0).contains(&key_track)
                && env_amount.is_finite()
                && (-1.0..=1.0).contains(&env_amount)
                && mix.is_finite()
                && (0.0..=1.0).contains(&mix) => {}
            _ => return Err(AudioExportError::InvalidDspParameter),
        }
    }
    Ok(())
}

pub(crate) fn apply_dsp_chain_to_buffer(
    data: &mut [f32],
    channels: usize,
    sample_rate: u32,
    devices: &[DspDeviceSpec],
) {
    if channels == 0 {
        return;
    }
    let mut processor = DspFrameProcessor::default();
    for frame in data.chunks_exact_mut(channels) {
        processor.process_frame(frame, sample_rate, devices);
    }
}

pub(crate) fn apply_dsp_chain_to_frame(
    processor: &mut DspFrameProcessor,
    frame: &mut [f32],
    sample_rate: u32,
    devices: &[DspDeviceSpec],
) {
    processor.process_frame(frame, sample_rate, devices);
}

pub(crate) fn apply_dsp_gain_to_aux_sample(sample: f32, devices: &[DspDeviceSpec]) -> f32 {
    devices.iter().fold(sample, |sample, device| {
        if device.bypassed {
            return sample;
        }
        match device.kind {
            DspDeviceKind::Gain { gain } if gain.is_finite() && gain >= 0.0 => sample * gain,
            _ => sample,
        }
    })
}

const MAX_FILTERS: usize = 8;
const MAX_CHANNELS: usize = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct DspFrameProcessor {
    filters: [[SvfState; MAX_CHANNELS]; MAX_FILTERS],
}

impl DspFrameProcessor {
    pub(crate) fn process_frame(
        &mut self,
        frame: &mut [f32],
        sample_rate: u32,
        devices: &[DspDeviceSpec],
    ) {
        let mut filter_index = 0;
        for device in devices {
            if device.bypassed {
                continue;
            }
            match device.kind {
                DspDeviceKind::Gain { gain } if gain.is_finite() && gain >= 0.0 => {
                    for sample in frame.iter_mut() {
                        *sample *= gain;
                    }
                }
                DspDeviceKind::Pan { pan } if pan.is_finite() && (-1.0..=1.0).contains(&pan) => {
                    apply_pan_frame(frame, pan);
                }
                DspDeviceKind::Balance { balance }
                    if balance.is_finite() && (-1.0..=1.0).contains(&balance) =>
                {
                    apply_pan_frame(frame, balance);
                }
                DspDeviceKind::StereoWidth { width }
                    if width.is_finite() && (0.0..=2.0).contains(&width) =>
                {
                    apply_stereo_width_frame(frame, width);
                }
                DspDeviceKind::PhaseInvert {
                    invert_left,
                    invert_right,
                } => apply_phase_invert_frame(frame, invert_left, invert_right),
                DspDeviceKind::Filter {
                    mode,
                    cutoff_hz,
                    resonance,
                    drive_db,
                    mix,
                    ..
                } if cutoff_hz.is_finite()
                    && resonance.is_finite()
                    && drive_db.is_finite()
                    && mix.is_finite() =>
                {
                    let filter_slot = filter_index.min(MAX_FILTERS - 1);
                    filter_index = filter_index.saturating_add(1);
                    let spec = FilterFrameSpec {
                        mode,
                        cutoff_hz,
                        resonance,
                        drive_db,
                        mix,
                    };
                    apply_filter_frame(frame, sample_rate, spec, &mut self.filters[filter_slot]);
                }
                _ => {}
            }
        }
    }
}

fn apply_pan_frame(frame: &mut [f32], pan: f32) {
    let channels = frame.len();
    for (channel, sample) in frame.iter_mut().enumerate() {
        *sample *= pan_gain(pan, channel, channels);
    }
}

fn apply_stereo_width_frame(frame: &mut [f32], width: f32) {
    if frame.len() >= 2 {
        let mid = (frame[0] + frame[1]) * 0.5;
        let side = (frame[0] - frame[1]) * 0.5 * width;
        frame[0] = mid + side;
        frame[1] = mid - side;
    }
}

fn apply_phase_invert_frame(frame: &mut [f32], invert_left: bool, invert_right: bool) {
    if invert_left && !frame.is_empty() {
        frame[0] = -frame[0];
    }
    if invert_right && frame.len() > 1 {
        frame[1] = -frame[1];
    }
}

#[derive(Debug, Clone, Copy)]
struct FilterFrameSpec {
    mode: DspFilterMode,
    cutoff_hz: f32,
    resonance: f32,
    drive_db: f32,
    mix: f32,
}

fn apply_filter_frame(
    frame: &mut [f32],
    sample_rate: u32,
    spec: FilterFrameSpec,
    states: &mut [SvfState; MAX_CHANNELS],
) {
    let sample_rate = sample_rate.max(1) as f32;
    let cutoff_hz = spec
        .cutoff_hz
        .clamp(20.0, sample_rate.mul_add(0.45, -1.0).max(20.0));
    let g = (std::f32::consts::PI * cutoff_hz / sample_rate).tan();
    let damping = 2.0 - spec.resonance.clamp(0.0, 1.0) * 1.9;
    let drive = db_to_gain(spec.drive_db.clamp(0.0, 24.0));
    let mix = spec.mix.clamp(0.0, 1.0);
    for (channel, sample) in frame.iter_mut().enumerate() {
        let state = &mut states[channel.min(MAX_CHANNELS - 1)];
        let dry = *sample;
        let driven = soft_clip(dry * drive);
        let wet = state.process(driven, g, damping, spec.mode);
        *sample = dry.mul_add(1.0 - mix, wet * mix);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct SvfState {
    ic1eq: f32,
    ic2eq: f32,
}

impl SvfState {
    fn process(&mut self, input: f32, g: f32, damping: f32, mode: DspFilterMode) -> f32 {
        let h = 1.0 / (1.0 + g * (g + damping));
        let high = (input - damping * self.ic1eq - self.ic2eq) * h;
        let band = g.mul_add(high, self.ic1eq);
        let low = g.mul_add(band, self.ic2eq);
        self.ic1eq = g.mul_add(high, band);
        self.ic2eq = g.mul_add(band, low);
        match mode {
            DspFilterMode::LowPass => low,
            DspFilterMode::HighPass => high,
            DspFilterMode::BandPass => band,
            DspFilterMode::Notch => low + high,
        }
    }
}

fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

fn soft_clip(sample: f32) -> f32 {
    sample.tanh()
}

pub(crate) fn pan_gain(pan: f32, channel: usize, channels: usize) -> f32 {
    if channels < 2 {
        return 1.0;
    }
    match channel {
        0 if pan > 0.0 => 1.0 - pan,
        1 if pan < 0.0 => 1.0 + pan,
        _ => 1.0,
    }
}
