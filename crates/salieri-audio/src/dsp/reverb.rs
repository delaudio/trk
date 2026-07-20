use super::{db_to_gain, one_pole_alpha, DelayLineState, MAX_CHANNELS};

#[derive(Debug, Clone, Copy)]
pub(super) struct ReverbFrameSpec {
    pub(super) size: f32,
    pub(super) predelay_ms: f32,
    pub(super) decay_s: f32,
    pub(super) damping: f32,
    pub(super) low_cut_hz: f32,
    pub(super) high_cut_hz: f32,
    pub(super) diffusion: f32,
    pub(super) width: f32,
    pub(super) early_reflections: f32,
    pub(super) mix: f32,
    pub(super) output_db: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct ReverbState {
    sample_rate: u32,
    channels: usize,
    predelay: DelayLineState,
    tank_lines: Vec<MonoDelayState>,
    damping_state: [f32; REVERB_LINE_COUNT],
    low_cut_state: [f32; MAX_CHANNELS],
    high_cut_state: [f32; MAX_CHANNELS],
}

const MAX_REVERB_PREDELAY_SECONDS: f32 = 0.25;
const MAX_REVERB_LINE_SECONDS: f32 = 0.5;
const REVERB_LINE_COUNT: usize = 8;
const REVERB_BASE_DELAYS_MS: [f32; REVERB_LINE_COUNT] =
    [29.7, 37.1, 41.1, 43.7, 53.9, 61.7, 68.3, 73.9];

impl ReverbState {
    pub(super) fn prepare(&mut self, sample_rate: u32, channels: usize) {
        let sample_rate = sample_rate.max(1);
        let channels = channels.clamp(1, MAX_CHANNELS);
        if self.sample_rate != sample_rate || self.channels != channels {
            self.sample_rate = sample_rate;
            self.channels = channels;
            self.damping_state = [0.0; REVERB_LINE_COUNT];
            self.low_cut_state = [0.0; MAX_CHANNELS];
            self.high_cut_state = [0.0; MAX_CHANNELS];
        }
        self.predelay.prepare(sample_rate, channels);
        while self.tank_lines.len() < REVERB_LINE_COUNT {
            self.tank_lines.push(MonoDelayState::default());
        }
        let frames = max_reverb_line_frames(sample_rate);
        for line in &mut self.tank_lines {
            line.prepare(frames);
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct MonoDelayState {
    write_index: usize,
    buffer: Vec<f32>,
}

impl MonoDelayState {
    fn prepare(&mut self, frames: usize) {
        if self.buffer.len() != frames {
            self.write_index = 0;
            self.buffer.clear();
            self.buffer.resize(frames, 0.0);
        }
    }

    fn read(&self, delay_samples: usize) -> f32 {
        if self.buffer.is_empty() {
            return 0.0;
        }
        let frames = self.buffer.len();
        let read_index = (self.write_index + frames - delay_samples.min(frames - 1)) % frames;
        self.buffer[read_index]
    }

    fn write(&mut self, value: f32) {
        if !self.buffer.is_empty() {
            self.buffer[self.write_index] = value;
        }
    }

    fn advance(&mut self) {
        if !self.buffer.is_empty() {
            self.write_index = (self.write_index + 1) % self.buffer.len();
        }
    }
}

pub(super) fn apply_reverb_frame(
    frame: &mut [f32],
    sample_rate: u32,
    spec: ReverbFrameSpec,
    state: &mut ReverbState,
) {
    if frame.is_empty() {
        return;
    }
    let channels = frame.len().min(MAX_CHANNELS);
    state.prepare(sample_rate, channels);
    let mut dry = [0.0; MAX_CHANNELS];
    for (channel, sample) in frame.iter().copied().enumerate().take(channels) {
        dry[channel] = sample;
    }

    let predelay_samples = reverb_predelay_samples(spec.predelay_ms, sample_rate);
    let pre_left = state.predelay.read(0, predelay_samples);
    let pre_right = if channels > 1 {
        state.predelay.read(1, predelay_samples)
    } else {
        pre_left
    };
    for (channel, sample) in dry.iter().copied().enumerate().take(channels) {
        state.predelay.write(channel, sample);
    }
    state.predelay.advance();

    let input = (pre_left + pre_right) * 0.5;
    let (mut wet_left, mut wet_right) = reverb_tank_output(input, sample_rate, spec, state);
    let early = spec.early_reflections.clamp(0.0, 1.0);
    wet_left += (pre_left * 0.65 + pre_right * 0.20) * early;
    wet_right += (pre_right * 0.65 + pre_left * 0.20) * early;
    (wet_left, wet_right) = apply_wet_width(wet_left, wet_right, spec.width);
    wet_left = reverb_tone(wet_left, 0, sample_rate, spec, state);
    wet_right = reverb_tone(wet_right, 1, sample_rate, spec, state);

    let mix = spec.mix.clamp(0.0, 1.0);
    let output_gain = db_to_gain(spec.output_db.clamp(-60.0, 12.0));
    frame[0] = dry[0].mul_add(1.0 - mix, wet_left * mix) * output_gain;
    if channels > 1 {
        frame[1] = dry[1].mul_add(1.0 - mix, wet_right * mix) * output_gain;
    }
    for channel in 2..channels {
        let wet = reverb_tone(
            (wet_left + wet_right) * 0.5,
            channel,
            sample_rate,
            spec,
            state,
        );
        frame[channel] = dry[channel].mul_add(1.0 - mix, wet * mix) * output_gain;
    }
}

fn reverb_tank_output(
    input: f32,
    sample_rate: u32,
    spec: ReverbFrameSpec,
    state: &mut ReverbState,
) -> (f32, f32) {
    let mut wet_left = 0.0;
    let mut wet_right = 0.0;
    let sample_rate_f32 = sample_rate.max(1) as f32;
    let size_scale = spec.size.clamp(0.0, 1.0).mul_add(1.5, 0.5);
    let diffusion = spec.diffusion.clamp(0.0, 1.0);
    let damping_cutoff = spec.damping.clamp(0.0, 1.0).mul_add(-15_000.0, 18_000.0);
    let damping_alpha = one_pole_alpha(damping_cutoff.max(1_000.0), sample_rate_f32);
    for (index, base_ms) in REVERB_BASE_DELAYS_MS.iter().copied().enumerate() {
        let delay_samples = reverb_line_samples(base_ms * size_scale, sample_rate);
        let output = state.tank_lines[index].read(delay_samples);
        state.damping_state[index] += damping_alpha * (output - state.damping_state[index]);
        let delay_seconds = delay_samples as f32 / sample_rate_f32;
        let feedback = reverb_feedback(delay_seconds, spec.decay_s, diffusion);
        let polarity = if index % 2 == 0 { 1.0 } else { -1.0 };
        state.tank_lines[index].write(
            input * (0.25 + diffusion * 0.25) + state.damping_state[index] * feedback * polarity,
        );
        state.tank_lines[index].advance();
        if index % 2 == 0 {
            wet_left += output;
        } else {
            wet_right += output;
        }
    }
    (wet_left * 0.25, wet_right * 0.25)
}

fn reverb_tone(
    sample: f32,
    channel: usize,
    sample_rate: u32,
    spec: ReverbFrameSpec,
    state: &mut ReverbState,
) -> f32 {
    let channel = channel.min(MAX_CHANNELS - 1);
    let sample_rate = sample_rate.max(1) as f32;
    let low_cut = spec.low_cut_hz.clamp(20.0, 2_000.0);
    let high_cut = spec.high_cut_hz.clamp(low_cut.max(1_000.0), 20_000.0);
    let low_alpha = one_pole_alpha(low_cut, sample_rate);
    state.low_cut_state[channel] += low_alpha * (sample - state.low_cut_state[channel]);
    let high_passed = sample - state.low_cut_state[channel];
    let high_alpha = one_pole_alpha(high_cut, sample_rate);
    state.high_cut_state[channel] += high_alpha * (high_passed - state.high_cut_state[channel]);
    state.high_cut_state[channel]
}

fn apply_wet_width(left: f32, right: f32, width: f32) -> (f32, f32) {
    let mid = (left + right) * 0.5;
    let side = (left - right) * 0.5 * width.clamp(0.0, 2.0);
    (mid + side, mid - side)
}

fn reverb_feedback(delay_seconds: f32, decay_s: f32, diffusion: f32) -> f32 {
    let rt60_feedback = 10.0_f32.powf(-3.0 * delay_seconds / decay_s.clamp(0.1, 30.0));
    (rt60_feedback * diffusion.mul_add(0.35, 0.55)).clamp(0.0, 0.98)
}

fn reverb_predelay_samples(time_ms: f32, sample_rate: u32) -> usize {
    let max_frames = max_reverb_predelay_frames(sample_rate);
    (time_ms.clamp(0.0, 250.0) * sample_rate.max(1) as f32 / 1_000.0)
        .round()
        .clamp(1.0, max_frames as f32 - 1.0) as usize
}

fn reverb_line_samples(time_ms: f32, sample_rate: u32) -> usize {
    let max_frames = max_reverb_line_frames(sample_rate);
    (time_ms.clamp(1.0, MAX_REVERB_LINE_SECONDS * 1_000.0) * sample_rate.max(1) as f32 / 1_000.0)
        .round()
        .clamp(1.0, max_frames as f32 - 1.0) as usize
}

fn max_reverb_predelay_frames(sample_rate: u32) -> usize {
    ((sample_rate.max(1) as f32) * MAX_REVERB_PREDELAY_SECONDS).ceil() as usize + 1
}

fn max_reverb_line_frames(sample_rate: u32) -> usize {
    ((sample_rate.max(1) as f32) * MAX_REVERB_LINE_SECONDS).ceil() as usize + 1
}
