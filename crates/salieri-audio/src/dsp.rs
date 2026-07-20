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
    Delay {
        sync: bool,
        time_left_ms: f32,
        time_right_ms: f32,
        link_times: bool,
        feedback: f32,
        ping_pong: bool,
        filter_low_cut_hz: f32,
        filter_high_cut_hz: f32,
        mod_rate_hz: f32,
        mod_depth: f32,
        mix: f32,
        output_db: f32,
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
const MAX_CHANNELS: usize = 8;
const MAX_DELAY_SECONDS: f32 = 4.0;

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DspFrameProcessor {
    filters: [[SvfState; MAX_CHANNELS]; MAX_FILTERS],
    delays: Vec<DelayLineState>,
}

impl DspFrameProcessor {
    pub(crate) fn prepare(&mut self, sample_rate: u32, channels: usize, devices: &[DspDeviceSpec]) {
        let mut delay_index = 0;
        for device in devices {
            if device.bypassed {
                continue;
            }
            if let DspDeviceKind::Delay { .. } = device.kind {
                let delay_slot = delay_index.min(MAX_DELAYS - 1);
                delay_index = delay_index.saturating_add(1);
                self.ensure_delay_slot(delay_slot);
                self.delays[delay_slot].prepare(sample_rate, channels);
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
                _ => {}
            }
        }
    }

    fn ensure_delay_slot(&mut self, slot: usize) {
        while self.delays.len() <= slot {
            self.delays.push(DelayLineState::default());
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

#[derive(Debug, Clone, Copy)]
struct DelayFrameSpec {
    sync: bool,
    time_left_ms: f32,
    time_right_ms: f32,
    link_times: bool,
    feedback: f32,
    ping_pong: bool,
    filter_low_cut_hz: f32,
    filter_high_cut_hz: f32,
    mod_rate_hz: f32,
    mod_depth: f32,
    mix: f32,
    output_db: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct DelayLineState {
    sample_rate: u32,
    channels: usize,
    write_index: usize,
    modulation_phase: f32,
    buffer: Vec<f32>,
    low_cut_state: [f32; MAX_CHANNELS],
    high_cut_state: [f32; MAX_CHANNELS],
}

impl DelayLineState {
    fn prepare(&mut self, sample_rate: u32, channels: usize) {
        let sample_rate = sample_rate.max(1);
        let channels = channels.clamp(1, MAX_CHANNELS);
        let frames = max_delay_frames(sample_rate);
        let needed = frames.saturating_mul(channels);
        if self.sample_rate != sample_rate
            || self.channels != channels
            || self.buffer.len() != needed
        {
            self.sample_rate = sample_rate;
            self.channels = channels;
            self.write_index = 0;
            self.modulation_phase = 0.0;
            self.buffer.clear();
            self.buffer.resize(needed, 0.0);
            self.low_cut_state = [0.0; MAX_CHANNELS];
            self.high_cut_state = [0.0; MAX_CHANNELS];
        }
    }

    fn read(&self, channel: usize, delay_samples: usize) -> f32 {
        if self.buffer.is_empty() || self.channels == 0 {
            return 0.0;
        }
        let channel = channel.min(self.channels - 1);
        let frames = self.buffer.len() / self.channels;
        let read_index = (self.write_index + frames - delay_samples.min(frames - 1)) % frames;
        self.buffer[read_index * self.channels + channel]
    }

    fn write(&mut self, channel: usize, value: f32) {
        if self.buffer.is_empty() || self.channels == 0 {
            return;
        }
        let channel = channel.min(self.channels - 1);
        self.buffer[self.write_index * self.channels + channel] = value;
    }

    fn advance(&mut self) {
        if self.buffer.is_empty() || self.channels == 0 {
            return;
        }
        let frames = self.buffer.len() / self.channels;
        self.write_index = (self.write_index + 1) % frames;
    }
}

fn apply_delay_frame(
    frame: &mut [f32],
    sample_rate: u32,
    spec: DelayFrameSpec,
    state: &mut DelayLineState,
) {
    if frame.is_empty() {
        return;
    }
    let channels = frame.len().min(MAX_CHANNELS);
    state.prepare(sample_rate, channels);
    let modulation = delay_modulation_samples(spec, state);
    let left_time = resolved_delay_time_ms(spec.time_left_ms, spec.sync);
    let left_delay = delay_samples(left_time, sample_rate, modulation);
    let right_time = if spec.link_times {
        left_time
    } else if spec.sync {
        resolved_delay_time_ms(spec.time_right_ms, true)
    } else {
        spec.time_right_ms
    };
    let right_delay = delay_samples(right_time, sample_rate, -modulation);
    let dry_left = frame[0];
    let dry_right = if channels > 1 { frame[1] } else { dry_left };
    let wet_left = state.read(0, left_delay);
    let wet_right = if channels > 1 {
        state.read(1, right_delay)
    } else {
        wet_left
    };
    let feedback = spec.feedback.clamp(0.0, 0.95);
    let feedback_left = if spec.ping_pong {
        wet_right * feedback
    } else {
        wet_left * feedback
    };
    let feedback_right = if spec.ping_pong {
        wet_left * feedback
    } else {
        wet_right * feedback
    };
    let write_left = dry_left + filter_feedback(feedback_left, 0, spec, state);
    let write_right = dry_right + filter_feedback(feedback_right, 1, spec, state);
    state.write(0, write_left);
    if channels > 1 {
        state.write(1, write_right);
    }

    let mix = spec.mix.clamp(0.0, 1.0);
    let output_gain = db_to_gain(spec.output_db.clamp(-60.0, 12.0));
    frame[0] = dry_left.mul_add(1.0 - mix, wet_left * mix) * output_gain;
    if channels > 1 {
        frame[1] = dry_right.mul_add(1.0 - mix, wet_right * mix) * output_gain;
    }
    for (channel, sample) in frame.iter_mut().enumerate().take(channels).skip(2) {
        let dry = *sample;
        let wet = state.read(channel, left_delay);
        *sample = dry.mul_add(1.0 - mix, wet * mix) * output_gain;
        let feedback_sample = filter_feedback(wet * feedback, channel, spec, state);
        state.write(channel, dry + feedback_sample);
    }
    state.advance();
}

fn max_delay_frames(sample_rate: u32) -> usize {
    ((sample_rate.max(1) as f32) * MAX_DELAY_SECONDS).ceil() as usize + 1
}

fn delay_samples(time_ms: f32, sample_rate: u32, modulation: f32) -> usize {
    let samples = time_ms.clamp(1.0, 4_000.0) * sample_rate.max(1) as f32 / 1_000.0 + modulation;
    samples
        .round()
        .clamp(1.0, max_delay_frames(sample_rate) as f32 - 1.0) as usize
}

fn resolved_delay_time_ms(time_ms: f32, sync: bool) -> f32 {
    if !sync {
        return time_ms;
    }
    const SYNC_DIVISIONS_MS: [f32; 10] = [
        125.0, 166.66667, 250.0, 333.33334, 500.0, 666.6667, 1_000.0, 1_500.0, 2_000.0, 4_000.0,
    ];
    SYNC_DIVISIONS_MS
        .into_iter()
        .min_by(|left, right| (time_ms - *left).abs().total_cmp(&(time_ms - *right).abs()))
        .unwrap_or(500.0)
}

fn delay_modulation_samples(spec: DelayFrameSpec, state: &mut DelayLineState) -> f32 {
    if spec.mod_rate_hz <= 0.0 || spec.mod_depth <= 0.0 || state.sample_rate == 0 {
        return 0.0;
    }
    let phase = state.modulation_phase;
    let depth_ms = 10.0 * spec.mod_depth.clamp(0.0, 1.0);
    let modulation = phase.sin() * depth_ms * state.sample_rate as f32 / 1_000.0;
    state.modulation_phase = (phase
        + std::f32::consts::TAU * spec.mod_rate_hz / state.sample_rate as f32)
        .rem_euclid(std::f32::consts::TAU);
    modulation
}

fn filter_feedback(
    sample: f32,
    channel: usize,
    spec: DelayFrameSpec,
    state: &mut DelayLineState,
) -> f32 {
    let channel = channel.min(MAX_CHANNELS - 1);
    let sample_rate = state.sample_rate.max(1) as f32;
    let low_cut = spec.filter_low_cut_hz.clamp(20.0, 20_000.0);
    let high_cut = spec.filter_high_cut_hz.clamp(low_cut, 20_000.0);
    let low_alpha = one_pole_alpha(low_cut, sample_rate);
    state.low_cut_state[channel] += low_alpha * (sample - state.low_cut_state[channel]);
    let high_passed = sample - state.low_cut_state[channel];
    let high_alpha = one_pole_alpha(high_cut, sample_rate);
    state.high_cut_state[channel] += high_alpha * (high_passed - state.high_cut_state[channel]);
    state.high_cut_state[channel]
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
