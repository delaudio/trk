#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DspFilterMode {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DspDriveMode {
    Overdrive,
    Saturation,
    HardClip,
    SoftClip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DspDynamicsDetector {
    Peak,
    Rms,
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
        mode: DspDriveMode,
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
    Chorus {
        rate_hz: f32,
        sync: bool,
        depth: f32,
        delay_ms: f32,
        voices: u8,
        spread: f32,
        feedback: f32,
        mix: f32,
        output_db: f32,
    },
    Flanger {
        rate_hz: f32,
        sync: bool,
        depth: f32,
        manual: f32,
        delay_ms: f32,
        feedback: f32,
        stereo_phase: f32,
        mix: f32,
        output_db: f32,
    },
    Phaser {
        rate_hz: f32,
        sync: bool,
        depth: f32,
        center_hz: f32,
        stages: u8,
        feedback: f32,
        stereo_phase: f32,
        mix: f32,
        output_db: f32,
    },
    Compressor {
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        knee_db: f32,
        makeup_db: f32,
        auto_makeup: bool,
        detector: DspDynamicsDetector,
        stereo_link: f32,
        mix: f32,
    },
    Gate {
        threshold_db: f32,
        hysteresis_db: f32,
        attack_ms: f32,
        hold_ms: f32,
        release_ms: f32,
        range_db: f32,
        detector: DspDynamicsDetector,
        stereo_link: f32,
    },
    Limiter {
        ceiling_db: f32,
        input_gain_db: f32,
        release_ms: f32,
        lookahead_ms: f32,
        stereo_link: f32,
        true_peak: bool,
    },
}
