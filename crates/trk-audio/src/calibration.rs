use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    Arc,
};

const LOW_CROSSOVER_HZ: f32 = 250.0;
const HIGH_CROSSOVER_HZ: f32 = 4_000.0;
const AGC_TARGET_PEAK: f32 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrationSettings {
    pub target_track_id: Option<u32>,
    pub master_gain: f32,
    pub track_gain: f32,
    pub low_gain: f32,
    pub mid_gain: f32,
    pub high_gain: f32,
    pub gate_threshold: f32,
    pub meter_decay: f32,
    pub auto_gain: bool,
}

impl Default for CalibrationSettings {
    fn default() -> Self {
        Self {
            target_track_id: None,
            master_gain: 1.0,
            track_gain: 1.0,
            low_gain: 1.0,
            mid_gain: 1.0,
            high_gain: 1.0,
            gate_threshold: 0.0,
            meter_decay: 0.30,
            auto_gain: false,
        }
    }
}

impl CalibrationSettings {
    pub fn validate(self) -> Result<Self, CalibrationError> {
        for (name, value) in [
            ("master gain", self.master_gain),
            ("track gain", self.track_gain),
            ("low gain", self.low_gain),
            ("mid gain", self.mid_gain),
            ("high gain", self.high_gain),
        ] {
            if !value.is_finite() || !(0.1..=4.0).contains(&value) {
                return Err(CalibrationError::InvalidControl { name, value });
            }
        }
        if !self.gate_threshold.is_finite() || !(0.0..=0.5).contains(&self.gate_threshold) {
            return Err(CalibrationError::InvalidControl {
                name: "gate threshold",
                value: self.gate_threshold,
            });
        }
        if !self.meter_decay.is_finite() || !(0.0..=0.95).contains(&self.meter_decay) {
            return Err(CalibrationError::InvalidControl {
                name: "meter decay",
                value: self.meter_decay,
            });
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CalibrationMeters {
    pub low: f32,
    pub mid: f32,
    pub high: f32,
    pub rms: f32,
    pub peak: f32,
}

#[derive(Debug, Clone)]
pub struct CalibrationControl {
    shared: Arc<CalibrationShared>,
}

impl Default for CalibrationControl {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationControl {
    #[must_use]
    pub fn new() -> Self {
        let control = Self {
            shared: Arc::new(CalibrationShared::default()),
        };
        control
            .store(CalibrationSettings::default())
            .expect("default calibration is valid");
        control
    }

    pub fn store(&self, settings: CalibrationSettings) -> Result<(), CalibrationError> {
        let settings = settings.validate()?;
        self.shared.target_track.store(
            settings.target_track_id.map_or(0, |id| u64::from(id) + 1),
            Ordering::Release,
        );
        store_f32(&self.shared.master_gain, settings.master_gain);
        store_f32(&self.shared.track_gain, settings.track_gain);
        store_f32(&self.shared.low_gain, settings.low_gain);
        store_f32(&self.shared.mid_gain, settings.mid_gain);
        store_f32(&self.shared.high_gain, settings.high_gain);
        store_f32(&self.shared.gate_threshold, settings.gate_threshold);
        store_f32(&self.shared.meter_decay, settings.meter_decay);
        self.shared
            .auto_gain
            .store(settings.auto_gain, Ordering::Release);
        Ok(())
    }

    #[must_use]
    pub fn settings(&self) -> CalibrationSettings {
        let target = self.shared.target_track.load(Ordering::Acquire);
        CalibrationSettings {
            target_track_id: (target > 0).then(|| (target - 1) as u32),
            master_gain: load_f32(&self.shared.master_gain),
            track_gain: load_f32(&self.shared.track_gain),
            low_gain: load_f32(&self.shared.low_gain),
            mid_gain: load_f32(&self.shared.mid_gain),
            high_gain: load_f32(&self.shared.high_gain),
            gate_threshold: load_f32(&self.shared.gate_threshold),
            meter_decay: load_f32(&self.shared.meter_decay),
            auto_gain: self.shared.auto_gain.load(Ordering::Acquire),
        }
    }

    #[must_use]
    pub fn meters(&self) -> CalibrationMeters {
        CalibrationMeters {
            low: load_f32(&self.shared.meter_low),
            mid: load_f32(&self.shared.meter_mid),
            high: load_f32(&self.shared.meter_high),
            rms: load_f32(&self.shared.meter_rms),
            peak: load_f32(&self.shared.meter_peak),
        }
    }

    pub fn clear_meters(&self) {
        self.publish(CalibrationMeters::default());
    }

    fn publish(&self, meters: CalibrationMeters) {
        store_f32(&self.shared.meter_low, meters.low);
        store_f32(&self.shared.meter_mid, meters.mid);
        store_f32(&self.shared.meter_high, meters.high);
        store_f32(&self.shared.meter_rms, meters.rms);
        store_f32(&self.shared.meter_peak, meters.peak);
    }
}

#[derive(Debug, Default)]
struct CalibrationShared {
    target_track: AtomicU64,
    master_gain: AtomicU32,
    track_gain: AtomicU32,
    low_gain: AtomicU32,
    mid_gain: AtomicU32,
    high_gain: AtomicU32,
    gate_threshold: AtomicU32,
    meter_decay: AtomicU32,
    auto_gain: AtomicBool,
    meter_low: AtomicU32,
    meter_mid: AtomicU32,
    meter_high: AtomicU32,
    meter_rms: AtomicU32,
    meter_peak: AtomicU32,
}

fn store_f32(target: &AtomicU32, value: f32) {
    target.store(value.to_bits(), Ordering::Release);
}

fn load_f32(source: &AtomicU32) -> f32 {
    f32::from_bits(source.load(Ordering::Acquire))
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CalibrationError {
    #[error("invalid {name} calibration value {value}")]
    InvalidControl { name: &'static str, value: f32 },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CalibrationProcessor {
    sample_rate: u32,
    channels: usize,
    low_state: Vec<f32>,
    high_state: Vec<f32>,
    gate_envelope: f32,
    agc_gain: f32,
    meters: CalibrationMeters,
}

impl CalibrationProcessor {
    pub(crate) fn new(sample_rate: u32, channels: usize) -> Self {
        Self {
            sample_rate,
            channels,
            low_state: vec![0.0; channels],
            high_state: vec![0.0; channels],
            gate_envelope: 1.0,
            agc_gain: 1.0,
            meters: CalibrationMeters::default(),
        }
    }

    pub(crate) fn process(
        &mut self,
        data: &mut [f32],
        settings: CalibrationSettings,
        control: &CalibrationControl,
    ) {
        if self.channels == 0 || data.is_empty() {
            self.meters = smooth_meters(
                self.meters,
                CalibrationMeters::default(),
                settings.meter_decay,
            );
            control.publish(self.meters);
            return;
        }
        let low_alpha = one_pole_alpha(LOW_CROSSOVER_HZ, self.sample_rate);
        let high_alpha = one_pole_alpha(HIGH_CROSSOVER_HZ, self.sample_rate);
        let mut low_sum = 0.0;
        let mut mid_sum = 0.0;
        let mut high_sum = 0.0;
        let mut pre_gain_peak = 0.0_f32;

        for frame in data.chunks_mut(self.channels) {
            let mut frame_level = 0.0_f32;
            let mut frame_low_sum = 0.0_f32;
            let mut frame_mid_sum = 0.0_f32;
            let mut frame_high_sum = 0.0_f32;
            for (channel, sample) in frame.iter_mut().enumerate() {
                let input = finite_or_zero(*sample);
                self.low_state[channel] += low_alpha * (input - self.low_state[channel]);
                self.high_state[channel] += high_alpha * (input - self.high_state[channel]);
                let low = self.low_state[channel];
                let mid = self.high_state[channel] - low;
                let high = input - self.high_state[channel];
                let low = low * settings.low_gain;
                let mid = mid * settings.mid_gain;
                let high = high * settings.high_gain;
                frame_low_sum += low * low;
                frame_mid_sum += mid * mid;
                frame_high_sum += high * high;
                *sample = low + mid + high;
                frame_level = frame_level.max(sample.abs());
            }
            self.gate_envelope = if frame_level > settings.gate_threshold {
                1.0
            } else {
                self.gate_envelope * settings.meter_decay
            };
            let gate_power = self.gate_envelope * self.gate_envelope;
            low_sum += frame_low_sum * gate_power;
            mid_sum += frame_mid_sum * gate_power;
            high_sum += frame_high_sum * gate_power;
            for sample in frame {
                *sample *= self.gate_envelope;
                pre_gain_peak = pre_gain_peak.max(sample.abs());
            }
        }

        let mastered_peak = pre_gain_peak * settings.master_gain;
        let desired_agc = if settings.auto_gain && mastered_peak > 1.0e-6 {
            (AGC_TARGET_PEAK / mastered_peak).clamp(0.1, 4.0)
        } else {
            1.0
        };
        self.agc_gain = if desired_agc < self.agc_gain {
            desired_agc
        } else {
            self.agc_gain * settings.meter_decay + desired_agc * (1.0 - settings.meter_decay)
        };
        let output_gain = settings.master_gain * self.agc_gain;
        let mut peak = 0.0_f32;
        let mut sum = 0.0_f32;
        for sample in data.iter_mut() {
            *sample = finite_or_zero(*sample * output_gain);
            peak = peak.max(sample.abs());
            sum += *sample * *sample;
        }
        let sample_count = data.len().max(1) as f32;
        let band_scale = output_gain / sample_count.sqrt();
        let current = CalibrationMeters {
            low: (low_sum.sqrt() * band_scale).clamp(0.0, 1.0),
            mid: (mid_sum.sqrt() * band_scale).clamp(0.0, 1.0),
            high: (high_sum.sqrt() * band_scale).clamp(0.0, 1.0),
            rms: (sum / sample_count).sqrt().clamp(0.0, 1.0),
            peak: peak.clamp(0.0, 1.0),
        };
        self.meters = smooth_meters(self.meters, current, settings.meter_decay);
        control.publish(self.meters);
    }
}

fn one_pole_alpha(cutoff: f32, sample_rate: u32) -> f32 {
    if sample_rate == 0 {
        return 1.0;
    }
    1.0 - (-2.0 * std::f32::consts::PI * cutoff / sample_rate as f32).exp()
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

fn smooth_meters(
    previous: CalibrationMeters,
    current: CalibrationMeters,
    decay: f32,
) -> CalibrationMeters {
    let smooth = |before: f32, now: f32| {
        if now >= before {
            now
        } else {
            before * decay + now * (1.0 - decay)
        }
        .clamp(0.0, 1.0)
    };
    CalibrationMeters {
        low: smooth(previous.low, current.low),
        mid: smooth(previous.mid, current.mid),
        high: smooth(previous.high, current.high),
        rms: smooth(previous.rms, current.rms),
        peak: smooth(previous.peak, current.peak),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_validate_and_control_round_trips_without_locks() {
        let control = CalibrationControl::new();
        let settings = CalibrationSettings {
            target_track_id: Some(42),
            master_gain: 1.5,
            track_gain: 0.5,
            low_gain: 2.0,
            auto_gain: true,
            ..CalibrationSettings::default()
        };
        control.store(settings).expect("valid settings");
        assert_eq!(control.settings(), settings);
        assert!(control
            .store(CalibrationSettings {
                master_gain: f32::NAN,
                ..settings
            })
            .is_err());
        assert_eq!(control.settings(), settings);
    }

    #[test]
    fn processor_applies_gain_gate_agc_and_publishes_finite_decaying_meters() {
        let control = CalibrationControl::new();
        let settings = CalibrationSettings {
            master_gain: 2.0,
            low_gain: 1.5,
            mid_gain: 0.8,
            high_gain: 1.2,
            gate_threshold: 0.01,
            meter_decay: 0.5,
            auto_gain: true,
            ..CalibrationSettings::default()
        };
        control.store(settings).expect("settings");
        let mut processor = CalibrationProcessor::new(48_000, 2);
        let mut signal = (0..512)
            .flat_map(|index| {
                let value = (index as f32 * 0.1).sin() * 0.2;
                [value, value]
            })
            .collect::<Vec<_>>();
        processor.process(&mut signal, settings, &control);
        assert!(signal.iter().all(|sample| sample.is_finite()));
        let active = control.meters();
        assert!(active.peak > 0.0 && active.peak <= 1.0);
        assert!(active.rms > 0.0 && active.rms <= active.peak);

        let mut silence = vec![0.0; signal.len()];
        processor.process(&mut silence, settings, &control);
        let decayed = control.meters();
        assert!(decayed.peak < active.peak);
        assert!(decayed.peak > 0.0);
        assert!(silence.iter().all(|sample| sample.is_finite()));
        assert!(silence.iter().all(|sample| sample.abs() < active.peak));

        control.clear_meters();
        assert_eq!(control.meters(), CalibrationMeters::default());
    }

    #[test]
    fn gate_envelope_closes_on_sub_threshold_audio_and_reopens_immediately() {
        let control = CalibrationControl::new();
        let settings = CalibrationSettings {
            gate_threshold: 0.1,
            meter_decay: 0.5,
            ..CalibrationSettings::default()
        };
        let mut processor = CalibrationProcessor::new(48_000, 1);
        let mut quiet = vec![0.05; 4];
        processor.process(&mut quiet, settings, &control);
        assert!(quiet.windows(2).all(|pair| pair[1] < pair[0]));

        let mut transient = vec![0.5];
        processor.process(&mut transient, settings, &control);
        assert_approx(transient[0], 0.5);
    }

    #[test]
    fn auto_gain_reduces_transients_immediately_and_releases_smoothly() {
        let control = CalibrationControl::new();
        let settings = CalibrationSettings {
            master_gain: 2.0,
            meter_decay: 0.5,
            auto_gain: true,
            ..CalibrationSettings::default()
        };
        let mut processor = CalibrationProcessor::new(48_000, 1);
        let mut transient = vec![1.0];
        processor.process(&mut transient, settings, &control);
        assert_approx(transient[0], AGC_TARGET_PEAK);

        let mut quieter = vec![0.25];
        processor.process(&mut quieter, settings, &control);
        assert!(quieter[0] > 0.25);
        assert!(quieter[0] < AGC_TARGET_PEAK);
    }

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "expected {expected}, got {actual}"
        );
    }
}
