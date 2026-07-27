use super::{db_to_gain, DspDeviceKind, MAX_CHANNELS};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ModulationKind {
    Chorus,
    Flanger,
    Phaser,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ModulationFrameSpec {
    kind: ModulationKind,
    rate_hz: f32,
    sync: bool,
    depth: f32,
    delay_ms: f32,
    manual: f32,
    voices: u8,
    spread: f32,
    feedback: f32,
    stereo_phase: f32,
    center_hz: f32,
    stages: u8,
    mix: f32,
    output_db: f32,
}

pub(super) fn modulation_frame_spec(kind: DspDeviceKind) -> Option<ModulationFrameSpec> {
    match kind {
        DspDeviceKind::Chorus {
            rate_hz,
            sync,
            depth,
            delay_ms,
            voices,
            spread,
            feedback,
            mix,
            output_db,
        } if rate_hz.is_finite()
            && depth.is_finite()
            && delay_ms.is_finite()
            && spread.is_finite()
            && feedback.is_finite()
            && mix.is_finite()
            && output_db.is_finite() =>
        {
            Some(ModulationFrameSpec {
                kind: ModulationKind::Chorus,
                rate_hz,
                sync,
                depth,
                delay_ms,
                manual: 0.5,
                voices,
                spread,
                feedback,
                stereo_phase: spread,
                center_hz: 1_000.0,
                stages: 4,
                mix,
                output_db,
            })
        }
        DspDeviceKind::Flanger {
            rate_hz,
            sync,
            depth,
            manual,
            delay_ms,
            feedback,
            stereo_phase,
            mix,
            output_db,
        } if rate_hz.is_finite()
            && depth.is_finite()
            && manual.is_finite()
            && delay_ms.is_finite()
            && feedback.is_finite()
            && stereo_phase.is_finite()
            && mix.is_finite()
            && output_db.is_finite() =>
        {
            Some(ModulationFrameSpec {
                kind: ModulationKind::Flanger,
                rate_hz,
                sync,
                depth,
                delay_ms,
                manual,
                voices: 1,
                spread: 0.0,
                feedback,
                stereo_phase,
                center_hz: 1_000.0,
                stages: 4,
                mix,
                output_db,
            })
        }
        DspDeviceKind::Phaser {
            rate_hz,
            sync,
            depth,
            center_hz,
            stages,
            feedback,
            stereo_phase,
            mix,
            output_db,
        } if rate_hz.is_finite()
            && depth.is_finite()
            && center_hz.is_finite()
            && feedback.is_finite()
            && stereo_phase.is_finite()
            && mix.is_finite()
            && output_db.is_finite() =>
        {
            Some(ModulationFrameSpec {
                kind: ModulationKind::Phaser,
                rate_hz,
                sync,
                depth,
                delay_ms: 0.0,
                manual: 0.5,
                voices: 1,
                spread: 0.0,
                feedback,
                stereo_phase,
                center_hz,
                stages,
                mix,
                output_db,
            })
        }
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ModulationState {
    sample_rate: u32,
    channels: usize,
    write_index: usize,
    phase: f32,
    buffer: Vec<f32>,
    feedback: [f32; MAX_CHANNELS],
    allpass: [[AllpassState; MAX_PHASER_STAGES]; MAX_CHANNELS],
}

impl Default for ModulationState {
    fn default() -> Self {
        Self {
            sample_rate: 0,
            channels: 0,
            write_index: 0,
            phase: 0.0,
            buffer: Vec::new(),
            feedback: [0.0; MAX_CHANNELS],
            allpass: [[AllpassState::default(); MAX_PHASER_STAGES]; MAX_CHANNELS],
        }
    }
}

impl ModulationState {
    pub(super) fn prepare(&mut self, sample_rate: u32, channels: usize) {
        let sample_rate = sample_rate.max(1);
        let channels = channels.clamp(1, MAX_CHANNELS);
        let frames = ((sample_rate as f32) * 0.1).ceil() as usize + 2;
        let needed = frames * channels;
        if self.sample_rate != sample_rate
            || self.channels != channels
            || self.buffer.len() != needed
        {
            self.sample_rate = sample_rate;
            self.channels = channels;
            self.write_index = 0;
            self.phase = 0.0;
            self.buffer.clear();
            self.buffer.resize(needed, 0.0);
            self.feedback = [0.0; MAX_CHANNELS];
            self.allpass = [[AllpassState::default(); MAX_PHASER_STAGES]; MAX_CHANNELS];
        }
    }

    fn read_delay(&self, channel: usize, delay_samples: usize) -> f32 {
        if self.buffer.is_empty() || self.channels == 0 {
            return 0.0;
        }
        let channel = channel.min(self.channels - 1);
        let frames = self.buffer.len() / self.channels;
        let read = (self.write_index + frames - delay_samples.min(frames - 1)) % frames;
        self.buffer[read * self.channels + channel]
    }

    fn write_delay(&mut self, channel: usize, value: f32) {
        let channel = channel.min(self.channels.saturating_sub(1));
        if !self.buffer.is_empty() && self.channels > 0 {
            self.buffer[self.write_index * self.channels + channel] = value;
        }
    }

    fn advance(&mut self) {
        if !self.buffer.is_empty() && self.channels > 0 {
            self.write_index = (self.write_index + 1) % (self.buffer.len() / self.channels);
        }
    }
}

pub(super) fn apply_modulation_frame(
    frame: &mut [f32],
    sample_rate: u32,
    spec: ModulationFrameSpec,
    state: &mut ModulationState,
) {
    let channels = frame.len().min(MAX_CHANNELS);
    if channels == 0 {
        return;
    }
    state.prepare(sample_rate, channels);
    match spec.kind {
        ModulationKind::Chorus | ModulationKind::Flanger => {
            apply_modulated_delay(frame, spec, state)
        }
        ModulationKind::Phaser => apply_phaser(frame, spec, state),
    }
    advance_lfo(spec, state);
}

fn apply_modulated_delay(
    frame: &mut [f32],
    spec: ModulationFrameSpec,
    state: &mut ModulationState,
) {
    let channels = frame.len().min(MAX_CHANNELS);
    let mix = spec.mix.clamp(0.0, 1.0);
    let output = db_to_gain(spec.output_db.clamp(-60.0, 12.0));
    let base_ms = spec.delay_ms.clamp(0.1, 40.0);
    let depth_ms = base_ms * spec.depth.clamp(0.0, 1.0);
    let voices = spec.voices.clamp(1, 4);
    for (channel, sample) in frame.iter_mut().enumerate().take(channels) {
        let dry = *sample;
        let mut wet = 0.0;
        for voice in 0..voices {
            let offset =
                (voice as f32 / voices as f32 + channel_phase(channel, channels, spec)).fract();
            let lfo = (state.phase + std::f32::consts::TAU * offset).sin();
            let manual = if spec.kind == ModulationKind::Flanger {
                spec.manual.clamp(0.0, 1.0)
            } else {
                0.5
            };
            let delay_ms = (base_ms * manual).max(0.1) + depth_ms * (lfo * 0.5 + 0.5);
            wet += state.read_delay(channel, samples(delay_ms, state.sample_rate));
        }
        wet /= voices as f32;
        let feedback = spec.feedback.clamp(-0.95, 0.95);
        state.feedback[channel] = wet;
        state.write_delay(channel, dry + wet * feedback);
        *sample = dry.mul_add(1.0 - mix, wet * mix) * output;
    }
    state.advance();
}

fn apply_phaser(frame: &mut [f32], spec: ModulationFrameSpec, state: &mut ModulationState) {
    let channels = frame.len().min(MAX_CHANNELS);
    let mix = spec.mix.clamp(0.0, 1.0);
    let output = db_to_gain(spec.output_db.clamp(-60.0, 12.0));
    let stages = usize::from(spec.stages.clamp(2, MAX_PHASER_STAGES as u8));
    for (channel, sample) in frame.iter_mut().enumerate().take(channels) {
        let dry = *sample;
        let lfo =
            (state.phase + std::f32::consts::TAU * channel_phase(channel, channels, spec)).sin();
        let sweep = 2.0_f32.powf((lfo * spec.depth.clamp(0.0, 1.0)) * 2.0);
        let freq = (spec.center_hz * sweep).clamp(80.0, state.sample_rate as f32 * 0.45);
        let coefficient = allpass_coefficient(freq, state.sample_rate);
        let mut wet = dry + state.feedback[channel] * spec.feedback.clamp(-0.95, 0.95);
        for stage in 0..stages {
            wet = state.allpass[channel][stage].process(wet, coefficient);
        }
        state.feedback[channel] = wet;
        *sample = dry.mul_add(1.0 - mix, wet * mix) * output;
    }
}

fn advance_lfo(spec: ModulationFrameSpec, state: &mut ModulationState) {
    let rate = if spec.sync {
        synced_rate(spec.rate_hz)
    } else {
        spec.rate_hz
    };
    state.phase = (state.phase + std::f32::consts::TAU * rate / state.sample_rate.max(1) as f32)
        .rem_euclid(std::f32::consts::TAU);
}

fn channel_phase(channel: usize, channels: usize, spec: ModulationFrameSpec) -> f32 {
    if channels < 2 {
        return 0.0;
    }
    match spec.kind {
        ModulationKind::Chorus => {
            spec.spread.clamp(0.0, 1.0) * channel as f32 / (channels - 1) as f32
        }
        _ => spec.stereo_phase.clamp(0.0, 1.0) * channel as f32 / (channels - 1) as f32,
    }
}

fn samples(ms: f32, sample_rate: u32) -> usize {
    (ms * sample_rate.max(1) as f32 / 1_000.0).round().max(1.0) as usize
}

fn synced_rate(rate_hz: f32) -> f32 {
    const RATES: [f32; 8] = [0.125, 0.25, 0.333_333, 0.5, 1.0, 2.0, 4.0, 8.0];
    RATES
        .into_iter()
        .min_by(|a, b| (rate_hz - *a).abs().total_cmp(&(rate_hz - *b).abs()))
        .unwrap_or(0.5)
}

fn allpass_coefficient(freq: f32, sample_rate: u32) -> f32 {
    let t = (std::f32::consts::PI * freq / sample_rate.max(1) as f32).tan();
    (1.0 - t) / (1.0 + t)
}

const MAX_PHASER_STAGES: usize = 12;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct AllpassState {
    x1: f32,
    y1: f32,
}

impl AllpassState {
    fn process(&mut self, input: f32, coefficient: f32) -> f32 {
        let output = coefficient.mul_add(input - self.y1, self.x1);
        self.x1 = input;
        self.y1 = output;
        output
    }
}
