use super::{DspDriveMode, MAX_CHANNELS};

pub(super) fn apply_drive_frame(
    frame: &mut [f32],
    mode: DspDriveMode,
    drive_db: f32,
    tone: f32,
    bias: f32,
    mix: f32,
    output_db: f32,
) {
    let drive = db_to_gain(drive_db.clamp(0.0, 48.0));
    let tone = tone.clamp(0.0, 1.0);
    let bias = bias.clamp(-1.0, 1.0) * 0.5;
    let mix = mix.clamp(0.0, 1.0);
    let output = db_to_gain(output_db.clamp(-60.0, 12.0));
    for sample in frame {
        let dry = *sample;
        let driven = dry.mul_add(drive, bias);
        let shaped = match mode {
            DspDriveMode::Overdrive => (driven * 0.75).tanh() * 1.25,
            DspDriveMode::Saturation => driven / (1.0 + driven.abs()),
            DspDriveMode::HardClip => driven.clamp(-1.0, 1.0),
            DspDriveMode::SoftClip => driven.tanh(),
        } - bias * 0.5;
        let dark = shaped * 0.65 + dry * 0.35;
        let wet = dark.mul_add(1.0 - tone, shaped * tone);
        *sample = dry.mul_add(1.0 - mix, wet * mix) * output;
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct BitcrusherState {
    hold_counter: usize,
    held: [f32; MAX_CHANNELS],
    dither_seed: u32,
}

impl BitcrusherState {
    pub(super) fn prepare(&mut self, channels: usize) {
        if self.dither_seed == 0 {
            self.dither_seed = 0x1234_abcd;
        }
        for sample in self.held.iter_mut().skip(channels.min(MAX_CHANNELS)) {
            *sample = 0.0;
        }
    }

    fn next_dither(&mut self) -> f32 {
        self.dither_seed = self
            .dither_seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let normalized = (self.dither_seed >> 8) as f32 / 16_777_216.0;
        normalized - 0.5
    }
}

pub(super) fn apply_bitcrusher_frame(
    frame: &mut [f32],
    bit_depth: u8,
    reduction_ratio: f32,
    dither: bool,
    mix: f32,
    output_db: f32,
    state: &mut BitcrusherState,
) {
    let channels = frame.len().min(MAX_CHANNELS);
    state.prepare(channels);
    let hold_frames = reduction_ratio.round().clamp(1.0, 64.0) as usize;
    let refresh = state.hold_counter == 0;
    let bit_depth = bit_depth.clamp(1, 24);
    let levels = ((1_u32 << u32::from(bit_depth)) - 1).max(1) as f32;
    let mix = mix.clamp(0.0, 1.0);
    let output = db_to_gain(output_db.clamp(-60.0, 12.0));
    for (channel, sample) in frame.iter_mut().enumerate().take(channels) {
        let dry = *sample;
        if refresh {
            let noise = if dither {
                state.next_dither() / levels
            } else {
                0.0
            };
            state.held[channel] =
                ((dry + noise).clamp(-1.0, 1.0).mul_add(0.5, 0.5) * levels).round() / levels * 2.0
                    - 1.0;
        }
        *sample = dry.mul_add(1.0 - mix, state.held[channel] * mix) * output;
    }
    state.hold_counter = (state.hold_counter + 1) % hold_frames;
}

fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}
