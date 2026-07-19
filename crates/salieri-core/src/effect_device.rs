use crate::{EffectDevice, EffectDeviceKind};

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
}
