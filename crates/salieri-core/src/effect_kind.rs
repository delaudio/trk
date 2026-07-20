use serde::{Deserialize, Serialize};

use crate::{DriveMode, FilterMode};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EffectDeviceKind {
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
        mode: FilterMode,
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
    Reverb {
        size: f32,
        predelay_ms: f32,
        decay_s: f32,
        damping: f32,
        low_cut_hz: f32,
        high_cut_hz: f32,
        diffusion: f32,
        width: f32,
        early_reflections: f32,
        mix: f32,
        output_db: f32,
    },
    Drive {
        mode: DriveMode,
        drive_db: f32,
        tone: f32,
        bias: f32,
        mix: f32,
        output_db: f32,
    },
    Bitcrusher {
        bit_depth: u8,
        reduction_ratio: f32,
        dither: bool,
        mix: f32,
        output_db: f32,
    },
}
