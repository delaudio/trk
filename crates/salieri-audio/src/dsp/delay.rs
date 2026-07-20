use super::{db_to_gain, MAX_CHANNELS, MAX_DELAY_SECONDS};

#[derive(Debug, Clone, Copy)]
pub(super) struct DelayFrameSpec {
    pub(super) sync: bool,
    pub(super) time_left_ms: f32,
    pub(super) time_right_ms: f32,
    pub(super) link_times: bool,
    pub(super) feedback: f32,
    pub(super) ping_pong: bool,
    pub(super) filter_low_cut_hz: f32,
    pub(super) filter_high_cut_hz: f32,
    pub(super) mod_rate_hz: f32,
    pub(super) mod_depth: f32,
    pub(super) mix: f32,
    pub(super) output_db: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct DelayLineState {
    sample_rate: u32,
    channels: usize,
    write_index: usize,
    modulation_phase: f32,
    buffer: Vec<f32>,
    low_cut_state: [f32; MAX_CHANNELS],
    high_cut_state: [f32; MAX_CHANNELS],
}

impl DelayLineState {
    pub(super) fn prepare(&mut self, sample_rate: u32, channels: usize) {
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

    pub(super) fn read(&self, channel: usize, delay_samples: usize) -> f32 {
        if self.buffer.is_empty() || self.channels == 0 {
            return 0.0;
        }
        let channel = channel.min(self.channels - 1);
        let frames = self.buffer.len() / self.channels;
        let read_index = (self.write_index + frames - delay_samples.min(frames - 1)) % frames;
        self.buffer[read_index * self.channels + channel]
    }

    pub(super) fn write(&mut self, channel: usize, value: f32) {
        if self.buffer.is_empty() || self.channels == 0 {
            return;
        }
        let channel = channel.min(self.channels - 1);
        self.buffer[self.write_index * self.channels + channel] = value;
    }

    pub(super) fn advance(&mut self) {
        if self.buffer.is_empty() || self.channels == 0 {
            return;
        }
        let frames = self.buffer.len() / self.channels;
        self.write_index = (self.write_index + 1) % frames;
    }
}

pub(super) fn apply_delay_frame(
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
