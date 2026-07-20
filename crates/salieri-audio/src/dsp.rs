use crate::errors::AudioExportError;

mod degradation;
mod delay;
mod dynamics;
mod kind;
mod modulation;
mod reverb;

use degradation::{apply_bitcrusher_frame, apply_drive_frame, BitcrusherState};
use delay::{apply_delay_frame, DelayFrameSpec, DelayLineState};
use dynamics::{apply_dynamics_frame, dynamics_frame_spec, DynamicsState};
use modulation::{apply_modulation_frame, modulation_frame_spec, ModulationState};
use reverb::{apply_reverb_frame, ReverbFrameSpec, ReverbState};

pub use kind::{DspDeviceKind, DspDriveMode, DspDynamicsDetector, DspFilterMode};

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
            DspDeviceKind::Delay {
                time_left_ms,
                time_right_ms,
                feedback,
                filter_low_cut_hz,
                filter_high_cut_hz,
                mod_rate_hz,
                mod_depth,
                mix,
                output_db,
                ..
            } if time_left_ms.is_finite()
                && (1.0..=4_000.0).contains(&time_left_ms)
                && time_right_ms.is_finite()
                && (1.0..=4_000.0).contains(&time_right_ms)
                && feedback.is_finite()
                && (0.0..=0.95).contains(&feedback)
                && filter_low_cut_hz.is_finite()
                && (20.0..=20_000.0).contains(&filter_low_cut_hz)
                && filter_high_cut_hz.is_finite()
                && (20.0..=20_000.0).contains(&filter_high_cut_hz)
                && filter_low_cut_hz <= filter_high_cut_hz
                && mod_rate_hz.is_finite()
                && (0.0..=20.0).contains(&mod_rate_hz)
                && mod_depth.is_finite()
                && (0.0..=1.0).contains(&mod_depth)
                && mix.is_finite()
                && (0.0..=1.0).contains(&mix)
                && output_db.is_finite()
                && (-60.0..=12.0).contains(&output_db) => {}
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
            } if size.is_finite()
                && (0.0..=1.0).contains(&size)
                && predelay_ms.is_finite()
                && (0.0..=250.0).contains(&predelay_ms)
                && decay_s.is_finite()
                && (0.1..=30.0).contains(&decay_s)
                && damping.is_finite()
                && (0.0..=1.0).contains(&damping)
                && low_cut_hz.is_finite()
                && (20.0..=2_000.0).contains(&low_cut_hz)
                && high_cut_hz.is_finite()
                && (1_000.0..=20_000.0).contains(&high_cut_hz)
                && low_cut_hz <= high_cut_hz
                && diffusion.is_finite()
                && (0.0..=1.0).contains(&diffusion)
                && width.is_finite()
                && (0.0..=2.0).contains(&width)
                && early_reflections.is_finite()
                && (0.0..=1.0).contains(&early_reflections)
                && mix.is_finite()
                && (0.0..=1.0).contains(&mix)
                && output_db.is_finite()
                && (-60.0..=12.0).contains(&output_db) => {}
            DspDeviceKind::Drive {
                drive_db,
                tone,
                bias,
                mix,
                output_db,
                ..
            } if drive_db.is_finite()
                && (0.0..=48.0).contains(&drive_db)
                && tone.is_finite()
                && (0.0..=1.0).contains(&tone)
                && bias.is_finite()
                && (-1.0..=1.0).contains(&bias)
                && mix.is_finite()
                && (0.0..=1.0).contains(&mix)
                && output_db.is_finite()
                && (-60.0..=12.0).contains(&output_db) => {}
            DspDeviceKind::Bitcrusher {
                bit_depth,
                reduction_ratio,
                mix,
                output_db,
                ..
            } if (1..=24).contains(&bit_depth)
                && reduction_ratio.is_finite()
                && (1.0..=64.0).contains(&reduction_ratio)
                && mix.is_finite()
                && (0.0..=1.0).contains(&mix)
                && output_db.is_finite()
                && (-60.0..=12.0).contains(&output_db) => {}
            DspDeviceKind::Chorus {
                rate_hz,
                depth,
                delay_ms,
                voices,
                spread,
                feedback,
                mix,
                output_db,
                ..
            } if valid_rate(rate_hz)
                && valid_unit(depth)
                && delay_ms.is_finite()
                && (1.0..=40.0).contains(&delay_ms)
                && (1..=4).contains(&voices)
                && valid_unit(spread)
                && feedback.is_finite()
                && (0.0..=0.95).contains(&feedback)
                && valid_unit(mix)
                && valid_output(output_db) => {}
            DspDeviceKind::Flanger {
                rate_hz,
                depth,
                manual,
                delay_ms,
                feedback,
                stereo_phase,
                mix,
                output_db,
                ..
            } if valid_rate(rate_hz)
                && valid_unit(depth)
                && valid_unit(manual)
                && delay_ms.is_finite()
                && (0.1..=20.0).contains(&delay_ms)
                && feedback.is_finite()
                && (-0.95..=0.95).contains(&feedback)
                && valid_unit(stereo_phase)
                && valid_unit(mix)
                && valid_output(output_db) => {}
            DspDeviceKind::Phaser {
                rate_hz,
                depth,
                center_hz,
                stages,
                feedback,
                stereo_phase,
                mix,
                output_db,
                ..
            } if valid_rate(rate_hz)
                && valid_unit(depth)
                && center_hz.is_finite()
                && (200.0..=8_000.0).contains(&center_hz)
                && (2..=12).contains(&stages)
                && feedback.is_finite()
                && (-0.95..=0.95).contains(&feedback)
                && valid_unit(stereo_phase)
                && valid_unit(mix)
                && valid_output(output_db) => {}
            DspDeviceKind::Compressor {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                knee_db,
                makeup_db,
                stereo_link,
                mix,
                ..
            } if valid_db(threshold_db, -80.0, 0.0)
                && ratio.is_finite()
                && (1.0..=20.0).contains(&ratio)
                && valid_ms(attack_ms, 0.01, 500.0)
                && valid_ms(release_ms, 1.0, 5_000.0)
                && valid_db(knee_db, 0.0, 24.0)
                && valid_db(makeup_db, -24.0, 24.0)
                && valid_unit(stereo_link)
                && valid_unit(mix) => {}
            DspDeviceKind::Gate {
                threshold_db,
                hysteresis_db,
                attack_ms,
                hold_ms,
                release_ms,
                range_db,
                stereo_link,
                ..
            } if valid_db(threshold_db, -80.0, 0.0)
                && valid_db(hysteresis_db, 0.0, 24.0)
                && valid_ms(attack_ms, 0.01, 500.0)
                && valid_ms(hold_ms, 0.0, 1_000.0)
                && valid_ms(release_ms, 1.0, 5_000.0)
                && valid_db(range_db, 0.0, 80.0)
                && valid_unit(stereo_link) => {}
            DspDeviceKind::Limiter {
                ceiling_db,
                input_gain_db,
                release_ms,
                lookahead_ms,
                stereo_link,
                ..
            } if valid_db(ceiling_db, -24.0, 0.0)
                && valid_db(input_gain_db, -24.0, 24.0)
                && valid_ms(release_ms, 1.0, 1_000.0)
                && valid_ms(lookahead_ms, 0.0, 20.0)
                && valid_unit(stereo_link) => {}
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
    processor.prepare(sample_rate, channels, devices);
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
const MAX_DELAYS: usize = 8;
const MAX_REVERBS: usize = 8;
const MAX_BITCRUSHERS: usize = 8;
const MAX_MODULATORS: usize = 8;
const MAX_DYNAMICS: usize = 8;
const MAX_CHANNELS: usize = 8;
const MAX_DELAY_SECONDS: f32 = 4.0;

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DspFrameProcessor {
    filters: [[SvfState; MAX_CHANNELS]; MAX_FILTERS],
    delays: Vec<DelayLineState>,
    reverbs: Vec<ReverbState>,
    bitcrushers: Vec<BitcrusherState>,
    modulators: Vec<ModulationState>,
    dynamics: Vec<DynamicsState>,
}

impl DspFrameProcessor {
    pub(crate) fn prepare(&mut self, sample_rate: u32, channels: usize, devices: &[DspDeviceSpec]) {
        let mut delay_index = 0;
        let mut reverb_index = 0;
        let mut bitcrusher_index = 0;
        let mut modulation_index = 0;
        let mut dynamics_index = 0;
        for device in devices {
            if device.bypassed {
                continue;
            }
            match device.kind {
                DspDeviceKind::Delay { .. } => {
                    let delay_slot = delay_index.min(MAX_DELAYS - 1);
                    delay_index = delay_index.saturating_add(1);
                    self.ensure_delay_slot(delay_slot);
                    self.delays[delay_slot].prepare(sample_rate, channels);
                }
                DspDeviceKind::Reverb { .. } => {
                    let reverb_slot = reverb_index.min(MAX_REVERBS - 1);
                    reverb_index = reverb_index.saturating_add(1);
                    self.ensure_reverb_slot(reverb_slot);
                    self.reverbs[reverb_slot].prepare(sample_rate, channels);
                }
                DspDeviceKind::Bitcrusher { .. } => {
                    let slot = bitcrusher_index.min(MAX_BITCRUSHERS - 1);
                    bitcrusher_index = bitcrusher_index.saturating_add(1);
                    self.ensure_bitcrusher_slot(slot);
                    self.bitcrushers[slot].prepare(channels);
                }
                DspDeviceKind::Chorus { .. }
                | DspDeviceKind::Flanger { .. }
                | DspDeviceKind::Phaser { .. } => {
                    let slot = modulation_index.min(MAX_MODULATORS - 1);
                    modulation_index = modulation_index.saturating_add(1);
                    self.ensure_modulation_slot(slot);
                    self.modulators[slot].prepare(sample_rate, channels);
                }
                DspDeviceKind::Compressor { .. }
                | DspDeviceKind::Gate { .. }
                | DspDeviceKind::Limiter { .. } => {
                    let slot = dynamics_index.min(MAX_DYNAMICS - 1);
                    dynamics_index = dynamics_index.saturating_add(1);
                    self.ensure_dynamics_slot(slot);
                    self.dynamics[slot].prepare(sample_rate, channels);
                }
                _ => {}
            }
        }
    }

    pub(crate) fn process_frame(
        &mut self,
        frame: &mut [f32],
        sample_rate: u32,
        devices: &[DspDeviceSpec],
    ) {
        let mut filter_index = 0;
        let mut delay_index = 0;
        let mut reverb_index = 0;
        let mut bitcrusher_index = 0;
        let mut modulation_index = 0;
        let mut dynamics_index = 0;
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
                DspDeviceKind::Delay {
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
                } if time_left_ms.is_finite()
                    && time_right_ms.is_finite()
                    && feedback.is_finite()
                    && filter_low_cut_hz.is_finite()
                    && filter_high_cut_hz.is_finite()
                    && mod_rate_hz.is_finite()
                    && mod_depth.is_finite()
                    && mix.is_finite()
                    && output_db.is_finite() =>
                {
                    let delay_slot = delay_index.min(MAX_DELAYS - 1);
                    delay_index = delay_index.saturating_add(1);
                    self.ensure_delay_slot(delay_slot);
                    let spec = DelayFrameSpec {
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
                    };
                    apply_delay_frame(frame, sample_rate, spec, &mut self.delays[delay_slot]);
                }
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
                } if size.is_finite()
                    && predelay_ms.is_finite()
                    && decay_s.is_finite()
                    && damping.is_finite()
                    && low_cut_hz.is_finite()
                    && high_cut_hz.is_finite()
                    && diffusion.is_finite()
                    && width.is_finite()
                    && early_reflections.is_finite()
                    && mix.is_finite()
                    && output_db.is_finite() =>
                {
                    let reverb_slot = reverb_index.min(MAX_REVERBS - 1);
                    reverb_index = reverb_index.saturating_add(1);
                    self.ensure_reverb_slot(reverb_slot);
                    let spec = ReverbFrameSpec {
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
                    };
                    apply_reverb_frame(frame, sample_rate, spec, &mut self.reverbs[reverb_slot]);
                }
                DspDeviceKind::Drive {
                    mode,
                    drive_db,
                    tone,
                    bias,
                    mix,
                    output_db,
                } if drive_db.is_finite()
                    && tone.is_finite()
                    && bias.is_finite()
                    && mix.is_finite()
                    && output_db.is_finite() =>
                {
                    apply_drive_frame(frame, mode, drive_db, tone, bias, mix, output_db);
                }
                DspDeviceKind::Bitcrusher {
                    bit_depth,
                    reduction_ratio,
                    dither,
                    mix,
                    output_db,
                } if reduction_ratio.is_finite() && mix.is_finite() && output_db.is_finite() => {
                    let slot = bitcrusher_index.min(MAX_BITCRUSHERS - 1);
                    bitcrusher_index = bitcrusher_index.saturating_add(1);
                    self.ensure_bitcrusher_slot(slot);
                    apply_bitcrusher_frame(
                        frame,
                        bit_depth,
                        reduction_ratio,
                        dither,
                        mix,
                        output_db,
                        &mut self.bitcrushers[slot],
                    );
                }
                kind @ (DspDeviceKind::Chorus { .. }
                | DspDeviceKind::Flanger { .. }
                | DspDeviceKind::Phaser { .. }) => {
                    if let Some(spec) = modulation_frame_spec(kind) {
                        let slot = modulation_index.min(MAX_MODULATORS - 1);
                        modulation_index = modulation_index.saturating_add(1);
                        self.ensure_modulation_slot(slot);
                        apply_modulation_frame(
                            frame,
                            sample_rate,
                            spec,
                            &mut self.modulators[slot],
                        );
                    }
                }
                kind @ (DspDeviceKind::Compressor { .. }
                | DspDeviceKind::Gate { .. }
                | DspDeviceKind::Limiter { .. }) => {
                    if let Some(spec) = dynamics_frame_spec(kind) {
                        let slot = dynamics_index.min(MAX_DYNAMICS - 1);
                        dynamics_index = dynamics_index.saturating_add(1);
                        self.ensure_dynamics_slot(slot);
                        apply_dynamics_frame(frame, sample_rate, spec, &mut self.dynamics[slot]);
                    }
                }
                _ => {}
            }
        }
    }

    fn ensure_delay_slot(&mut self, slot: usize) {
        while self.delays.len() <= slot {
            self.delays.push(DelayLineState::default());
        }
    }

    fn ensure_reverb_slot(&mut self, slot: usize) {
        while self.reverbs.len() <= slot {
            self.reverbs.push(ReverbState::default());
        }
    }

    fn ensure_bitcrusher_slot(&mut self, slot: usize) {
        while self.bitcrushers.len() <= slot {
            self.bitcrushers.push(BitcrusherState::default());
        }
    }

    fn ensure_modulation_slot(&mut self, slot: usize) {
        while self.modulators.len() <= slot {
            self.modulators.push(ModulationState::default());
        }
    }

    fn ensure_dynamics_slot(&mut self, slot: usize) {
        while self.dynamics.len() <= slot {
            self.dynamics.push(DynamicsState::default());
        }
    }
}

fn valid_rate(value: f32) -> bool {
    value.is_finite() && (0.01..=20.0).contains(&value)
}

fn valid_unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn valid_output(value: f32) -> bool {
    value.is_finite() && (-60.0..=12.0).contains(&value)
}

fn valid_db(value: f32, min: f32, max: f32) -> bool {
    value.is_finite() && (min..=max).contains(&value)
}

fn valid_ms(value: f32, min: f32, max: f32) -> bool {
    value.is_finite() && (min..=max).contains(&value)
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

fn one_pole_alpha(cutoff_hz: f32, sample_rate: f32) -> f32 {
    let rc = 1.0 / (std::f32::consts::TAU * cutoff_hz.max(1.0));
    let dt = 1.0 / sample_rate.max(1.0);
    dt / (rc + dt)
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
