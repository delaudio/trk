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
}
