use serde::{Deserialize, Serialize};

use crate::{EffectDevice, EffectDeviceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterMode {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DriveMode {
    Overdrive,
    Saturation,
    HardClip,
    SoftClip,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterSpec {
    pub mode: FilterMode,
    pub cutoff_hz: f32,
    pub resonance: f32,
    pub drive_db: f32,
    pub key_track: f32,
    pub env_amount: f32,
    pub mix: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DelaySpec {
    pub sync: bool,
    pub time_left_ms: f32,
    pub time_right_ms: f32,
    pub link_times: bool,
    pub feedback: f32,
    pub ping_pong: bool,
    pub filter_low_cut_hz: f32,
    pub filter_high_cut_hz: f32,
    pub mod_rate_hz: f32,
    pub mod_depth: f32,
    pub mix: f32,
    pub output_db: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverbSpec {
    pub size: f32,
    pub predelay_ms: f32,
    pub decay_s: f32,
    pub damping: f32,
    pub low_cut_hz: f32,
    pub high_cut_hz: f32,
    pub diffusion: f32,
    pub width: f32,
    pub early_reflections: f32,
    pub mix: f32,
    pub output_db: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriveSpec {
    pub mode: DriveMode,
    pub drive_db: f32,
    pub tone: f32,
    pub bias: f32,
    pub mix: f32,
    pub output_db: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BitcrusherSpec {
    pub bit_depth: u8,
    pub reduction_ratio: f32,
    pub dither: bool,
    pub mix: f32,
    pub output_db: f32,
}

impl Default for DriveSpec {
    fn default() -> Self {
        Self {
            mode: DriveMode::Overdrive,
            drive_db: 12.0,
            tone: 0.5,
            bias: 0.0,
            mix: 1.0,
            output_db: 0.0,
        }
    }
}

impl Default for BitcrusherSpec {
    fn default() -> Self {
        Self {
            bit_depth: 12,
            reduction_ratio: 1.0,
            dither: false,
            mix: 1.0,
            output_db: 0.0,
        }
    }
}

impl Default for ReverbSpec {
    fn default() -> Self {
        Self {
            size: 0.5,
            predelay_ms: 20.0,
            decay_s: 2.5,
            damping: 0.5,
            low_cut_hz: 100.0,
            high_cut_hz: 16_000.0,
            diffusion: 0.75,
            width: 1.0,
            early_reflections: 0.5,
            mix: 0.25,
            output_db: 0.0,
        }
    }
}

impl Default for DelaySpec {
    fn default() -> Self {
        Self {
            sync: true,
            time_left_ms: 500.0,
            time_right_ms: 500.0,
            link_times: true,
            feedback: 0.35,
            ping_pong: false,
            filter_low_cut_hz: 20.0,
            filter_high_cut_hz: 20_000.0,
            mod_rate_hz: 0.0,
            mod_depth: 0.0,
            mix: 0.25,
            output_db: 0.0,
        }
    }
}

impl Default for FilterSpec {
    fn default() -> Self {
        Self {
            mode: FilterMode::LowPass,
            cutoff_hz: 12_000.0,
            resonance: 0.25,
            drive_db: 0.0,
            key_track: 0.0,
            env_amount: 0.0,
            mix: 1.0,
        }
    }
}

impl FilterMode {
    #[must_use]
    pub fn parameter_id(self) -> &'static str {
        match self {
            Self::LowPass => "lowPass",
            Self::HighPass => "highPass",
            Self::BandPass => "bandPass",
            Self::Notch => "notch",
        }
    }

    #[must_use]
    pub fn from_parameter_id(value: &str) -> Option<Self> {
        match value {
            "lowPass" => Some(Self::LowPass),
            "highPass" => Some(Self::HighPass),
            "bandPass" => Some(Self::BandPass),
            "notch" => Some(Self::Notch),
            _ => None,
        }
    }
}

impl DriveMode {
    #[must_use]
    pub fn parameter_id(self) -> &'static str {
        match self {
            Self::Overdrive => "overdrive",
            Self::Saturation => "saturation",
            Self::HardClip => "hardClip",
            Self::SoftClip => "softClip",
        }
    }

    #[must_use]
    pub fn from_parameter_id(value: &str) -> Option<Self> {
        match value {
            "overdrive" => Some(Self::Overdrive),
            "saturation" => Some(Self::Saturation),
            "hardClip" => Some(Self::HardClip),
            "softClip" => Some(Self::SoftClip),
            _ => None,
        }
    }
}

impl EffectDevice {
    fn new(id: u32, name: &str, kind: EffectDeviceKind) -> Self {
        Self {
            id,
            name: name.to_string(),
            bypassed: false,
            kind,
        }
    }

    #[must_use]
    pub fn gain(id: u32, gain: f32) -> Self {
        Self::new(id, "Gain", EffectDeviceKind::Gain { gain })
    }

    #[must_use]
    pub fn pan(id: u32, pan: f32) -> Self {
        Self::new(id, "Pan", EffectDeviceKind::Pan { pan })
    }

    #[must_use]
    pub fn balance(id: u32, balance: f32) -> Self {
        Self::new(id, "Balance", EffectDeviceKind::Balance { balance })
    }

    #[must_use]
    pub fn stereo_width(id: u32, width: f32) -> Self {
        Self::new(id, "Stereo Width", EffectDeviceKind::StereoWidth { width })
    }

    #[must_use]
    pub fn phase_invert(id: u32, invert_left: bool, invert_right: bool) -> Self {
        Self::new(
            id,
            "Phase",
            EffectDeviceKind::PhaseInvert {
                invert_left,
                invert_right,
            },
        )
    }

    #[must_use]
    pub fn filter(id: u32, spec: FilterSpec) -> Self {
        Self::new(
            id,
            "Filter",
            EffectDeviceKind::Filter {
                mode: spec.mode,
                cutoff_hz: spec.cutoff_hz,
                resonance: spec.resonance,
                drive_db: spec.drive_db,
                key_track: spec.key_track,
                env_amount: spec.env_amount,
                mix: spec.mix,
            },
        )
    }

    #[must_use]
    pub fn delay(id: u32, spec: DelaySpec) -> Self {
        Self::new(
            id,
            "Delay",
            EffectDeviceKind::Delay {
                sync: spec.sync,
                time_left_ms: spec.time_left_ms,
                time_right_ms: spec.time_right_ms,
                link_times: spec.link_times,
                feedback: spec.feedback,
                ping_pong: spec.ping_pong,
                filter_low_cut_hz: spec.filter_low_cut_hz,
                filter_high_cut_hz: spec.filter_high_cut_hz,
                mod_rate_hz: spec.mod_rate_hz,
                mod_depth: spec.mod_depth,
                mix: spec.mix,
                output_db: spec.output_db,
            },
        )
    }

    #[must_use]
    pub fn reverb(id: u32, spec: ReverbSpec) -> Self {
        Self::new(
            id,
            "Reverb",
            EffectDeviceKind::Reverb {
                size: spec.size,
                predelay_ms: spec.predelay_ms,
                decay_s: spec.decay_s,
                damping: spec.damping,
                low_cut_hz: spec.low_cut_hz,
                high_cut_hz: spec.high_cut_hz,
                diffusion: spec.diffusion,
                width: spec.width,
                early_reflections: spec.early_reflections,
                mix: spec.mix,
                output_db: spec.output_db,
            },
        )
    }

    #[must_use]
    pub fn drive(id: u32, spec: DriveSpec) -> Self {
        Self::new(
            id,
            "Drive",
            EffectDeviceKind::Drive {
                mode: spec.mode,
                drive_db: spec.drive_db,
                tone: spec.tone,
                bias: spec.bias,
                mix: spec.mix,
                output_db: spec.output_db,
            },
        )
    }

    #[must_use]
    pub fn bitcrusher(id: u32, spec: BitcrusherSpec) -> Self {
        Self::new(
            id,
            "Bitcrusher",
            EffectDeviceKind::Bitcrusher {
                bit_depth: spec.bit_depth,
                reduction_ratio: spec.reduction_ratio,
                dither: spec.dither,
                mix: spec.mix,
                output_db: spec.output_db,
            },
        )
    }
}
