use super::{db_to_gain, DspDeviceKind, DspDynamicsDetector, MAX_CHANNELS};

#[derive(Debug, Clone, Copy, PartialEq)]
enum DynamicsKind {
    Compressor,
    Gate,
    Limiter,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DynamicsFrameSpec {
    kind: DynamicsKind,
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
    knee_db: f32,
    makeup_db: f32,
    auto_makeup: bool,
    hold_ms: f32,
    range_db: f32,
    ceiling_db: f32,
    input_gain_db: f32,
    lookahead_ms: f32,
    detector: DspDynamicsDetector,
    stereo_link: f32,
    mix: f32,
}

pub(super) fn dynamics_frame_spec(kind: DspDeviceKind) -> Option<DynamicsFrameSpec> {
    match kind {
        DspDeviceKind::Compressor {
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            knee_db,
            makeup_db,
            auto_makeup,
            detector,
            stereo_link,
            mix,
        } if finite(&[
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            knee_db,
            makeup_db,
            stereo_link,
            mix,
        ]) =>
        {
            Some(DynamicsFrameSpec {
                kind: DynamicsKind::Compressor,
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                knee_db,
                makeup_db,
                auto_makeup,
                hold_ms: 0.0,
                range_db: 0.0,
                ceiling_db: 0.0,
                input_gain_db: 0.0,
                lookahead_ms: 0.0,
                detector,
                stereo_link,
                mix,
            })
        }
        DspDeviceKind::Gate {
            threshold_db,
            hysteresis_db,
            attack_ms,
            hold_ms,
            release_ms,
            range_db,
            detector,
            stereo_link,
        } if finite(&[
            threshold_db,
            hysteresis_db,
            attack_ms,
            hold_ms,
            release_ms,
            range_db,
            stereo_link,
        ]) =>
        {
            Some(DynamicsFrameSpec {
                kind: DynamicsKind::Gate,
                threshold_db,
                ratio: 1.0,
                attack_ms,
                release_ms,
                knee_db: hysteresis_db,
                makeup_db: 0.0,
                auto_makeup: false,
                hold_ms,
                range_db,
                ceiling_db: 0.0,
                input_gain_db: 0.0,
                lookahead_ms: 0.0,
                detector,
                stereo_link,
                mix: 1.0,
            })
        }
        DspDeviceKind::Limiter {
            ceiling_db,
            input_gain_db,
            release_ms,
            lookahead_ms,
            stereo_link,
            ..
        } if finite(&[
            ceiling_db,
            input_gain_db,
            release_ms,
            lookahead_ms,
            stereo_link,
        ]) =>
        {
            Some(DynamicsFrameSpec {
                kind: DynamicsKind::Limiter,
                threshold_db: ceiling_db,
                ratio: 20.0,
                attack_ms: 0.01,
                release_ms,
                knee_db: 0.0,
                makeup_db: 0.0,
                auto_makeup: false,
                hold_ms: 0.0,
                range_db: 0.0,
                ceiling_db,
                input_gain_db,
                lookahead_ms,
                detector: DspDynamicsDetector::Peak,
                stereo_link,
                mix: 1.0,
            })
        }
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct DynamicsState {
    sample_rate: u32,
    channels: usize,
    envelope: [f32; MAX_CHANNELS],
    gain: [f32; MAX_CHANNELS],
    gate_open: [bool; MAX_CHANNELS],
    hold_remaining: [u32; MAX_CHANNELS],
    lookahead: Vec<f32>,
    write_index: usize,
}

impl Default for DynamicsState {
    fn default() -> Self {
        Self {
            sample_rate: 0,
            channels: 0,
            envelope: [0.0; MAX_CHANNELS],
            gain: [1.0; MAX_CHANNELS],
            gate_open: [false; MAX_CHANNELS],
            hold_remaining: [0; MAX_CHANNELS],
            lookahead: Vec::new(),
            write_index: 0,
        }
    }
}

impl DynamicsState {
    pub(super) fn prepare(&mut self, sample_rate: u32, channels: usize) {
        let sample_rate = sample_rate.max(1);
        let channels = channels.clamp(1, MAX_CHANNELS);
        let frames = ((sample_rate as f32) * 0.02).ceil() as usize + 1;
        let needed = frames * channels;
        if self.sample_rate != sample_rate
            || self.channels != channels
            || self.lookahead.len() != needed
        {
            self.sample_rate = sample_rate;
            self.channels = channels;
            self.envelope = [0.0; MAX_CHANNELS];
            self.gain = [1.0; MAX_CHANNELS];
            self.gate_open = [false; MAX_CHANNELS];
            self.hold_remaining = [0; MAX_CHANNELS];
            self.lookahead.clear();
            self.lookahead.resize(needed, 0.0);
            self.write_index = 0;
        }
    }
}

pub(super) fn apply_dynamics_frame(
    frame: &mut [f32],
    sample_rate: u32,
    spec: DynamicsFrameSpec,
    state: &mut DynamicsState,
) {
    let channels = frame.len().min(MAX_CHANNELS);
    if channels == 0 {
        return;
    }
    state.prepare(sample_rate, channels);
    match spec.kind {
        DynamicsKind::Compressor => apply_compressor(frame, spec, state),
        DynamicsKind::Gate => apply_gate(frame, spec, state),
        DynamicsKind::Limiter => apply_limiter(frame, spec, state),
    }
}

fn apply_compressor(frame: &mut [f32], spec: DynamicsFrameSpec, state: &mut DynamicsState) {
    let channels = frame.len().min(MAX_CHANNELS);
    let linked = linked_detector(frame, channels, spec.detector, spec.stereo_link);
    for (channel, sample) in frame.iter_mut().enumerate().take(channels) {
        let dry = *sample;
        let detected = blend_detector(dry, linked, spec.detector, spec.stereo_link);
        let env = follow(
            state.envelope[channel],
            detected,
            spec.attack_ms,
            spec.release_ms,
            state.sample_rate,
        );
        state.envelope[channel] = env;
        let gain_db = compressor_gain_db(amp_to_db(env), spec);
        let auto = if spec.auto_makeup {
            (-spec.threshold_db * (1.0 - 1.0 / spec.ratio.max(1.0))).clamp(0.0, 24.0)
        } else {
            0.0
        };
        let wet = dry * db_to_gain(gain_db + spec.makeup_db + auto);
        *sample = dry.mul_add(
            1.0 - spec.mix.clamp(0.0, 1.0),
            wet * spec.mix.clamp(0.0, 1.0),
        );
    }
}

fn apply_gate(frame: &mut [f32], spec: DynamicsFrameSpec, state: &mut DynamicsState) {
    let channels = frame.len().min(MAX_CHANNELS);
    let linked = linked_detector(frame, channels, spec.detector, spec.stereo_link);
    let hold_samples = (spec.hold_ms.max(0.0) * state.sample_rate as f32 / 1_000.0) as u32;
    for (channel, sample) in frame.iter_mut().enumerate().take(channels) {
        let detected = blend_detector(*sample, linked, spec.detector, spec.stereo_link);
        let env = follow(
            state.envelope[channel],
            detected,
            spec.attack_ms,
            spec.release_ms,
            state.sample_rate,
        );
        state.envelope[channel] = env;
        let level = amp_to_db(env);
        if level >= spec.threshold_db {
            state.gate_open[channel] = true;
            state.hold_remaining[channel] = hold_samples;
        } else if level <= spec.threshold_db - spec.knee_db.max(0.0) {
            if state.hold_remaining[channel] == 0 {
                state.gate_open[channel] = false;
            } else {
                state.hold_remaining[channel] -= 1;
            }
        }
        let target = if state.gate_open[channel] {
            1.0
        } else {
            db_to_gain(-spec.range_db.abs())
        };
        state.gain[channel] = follow_gain(
            state.gain[channel],
            target,
            spec.attack_ms,
            spec.release_ms,
            state.sample_rate,
        );
        *sample *= state.gain[channel];
    }
}

fn apply_limiter(frame: &mut [f32], spec: DynamicsFrameSpec, state: &mut DynamicsState) {
    let channels = frame.len().min(MAX_CHANNELS);
    let delay = ((spec.lookahead_ms.max(0.0) * state.sample_rate as f32 / 1_000.0) as usize)
        .min(state.lookahead.len() / channels - 1);
    let input_gain = db_to_gain(spec.input_gain_db);
    let mut delayed = [0.0; MAX_CHANNELS];
    for channel in 0..channels {
        let frames = state.lookahead.len() / channels;
        let read = (state.write_index + frames - delay) % frames;
        delayed[channel] = state.lookahead[read * channels + channel];
        state.lookahead[state.write_index * channels + channel] = frame[channel] * input_gain;
        frame[channel] *= input_gain;
    }
    let linked = linked_detector(frame, channels, DspDynamicsDetector::Peak, spec.stereo_link);
    for channel in 0..channels {
        let level = amp_to_db(blend_detector(
            frame[channel],
            linked,
            DspDynamicsDetector::Peak,
            spec.stereo_link,
        ));
        let gain_db = (spec.ceiling_db - level).min(0.0);
        let target = db_to_gain(gain_db);
        state.gain[channel] = follow_gain(
            state.gain[channel],
            target,
            0.01,
            spec.release_ms,
            state.sample_rate,
        );
        let sample = if delay == 0 {
            frame[channel]
        } else {
            delayed[channel]
        };
        frame[channel] = (sample * state.gain[channel])
            .clamp(-db_to_gain(spec.ceiling_db), db_to_gain(spec.ceiling_db));
    }
    if !state.lookahead.is_empty() {
        state.write_index = (state.write_index + 1) % (state.lookahead.len() / channels);
    }
}

fn compressor_gain_db(level_db: f32, spec: DynamicsFrameSpec) -> f32 {
    let ratio = spec.ratio.max(1.0);
    let over = level_db - spec.threshold_db;
    let knee = spec.knee_db.max(0.0);
    if knee <= f32::EPSILON {
        return if over > 0.0 {
            over * (1.0 / ratio - 1.0)
        } else {
            0.0
        };
    }
    if over <= -knee * 0.5 {
        0.0
    } else if over >= knee * 0.5 {
        over * (1.0 / ratio - 1.0)
    } else {
        (1.0 / ratio - 1.0) * (over + knee * 0.5).powi(2) / (2.0 * knee)
    }
}

fn linked_detector(
    frame: &[f32],
    channels: usize,
    detector: DspDynamicsDetector,
    stereo_link: f32,
) -> f32 {
    if channels < 2 || stereo_link <= 0.0 {
        return 0.0;
    }
    frame
        .iter()
        .take(channels)
        .map(|sample| detect(*sample, detector))
        .fold(0.0, f32::max)
}

fn blend_detector(
    sample: f32,
    linked: f32,
    detector: DspDynamicsDetector,
    stereo_link: f32,
) -> f32 {
    let local = detect(sample, detector);
    local.mul_add(
        1.0 - stereo_link.clamp(0.0, 1.0),
        linked * stereo_link.clamp(0.0, 1.0),
    )
}

fn detect(sample: f32, detector: DspDynamicsDetector) -> f32 {
    match detector {
        DspDynamicsDetector::Peak => sample.abs(),
        DspDynamicsDetector::Rms => (sample * sample).sqrt(),
    }
}

fn follow(previous: f32, target: f32, attack_ms: f32, release_ms: f32, sample_rate: u32) -> f32 {
    let time_ms = if target > previous {
        attack_ms
    } else {
        release_ms
    };
    let coeff = envelope_coeff(time_ms, sample_rate);
    previous + (target - previous) * coeff
}

fn follow_gain(
    previous: f32,
    target: f32,
    attack_ms: f32,
    release_ms: f32,
    sample_rate: u32,
) -> f32 {
    follow(previous, target, attack_ms, release_ms, sample_rate)
}

fn envelope_coeff(ms: f32, sample_rate: u32) -> f32 {
    if ms <= 0.0 {
        return 1.0;
    }
    1.0 - (-1.0 / (ms * sample_rate.max(1) as f32 / 1_000.0)).exp()
}

fn amp_to_db(value: f32) -> f32 {
    20.0 * value.abs().max(0.000_001).log10()
}

fn finite(values: &[f32]) -> bool {
    values.iter().all(|value| value.is_finite())
}
