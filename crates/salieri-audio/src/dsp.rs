use crate::errors::AudioExportError;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DspGraphSpec {
    pub track_chains: Vec<TrackDspChainSpec>,
    pub master: Vec<DspDeviceSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackDspChainSpec {
    pub track_id: u32,
    pub devices: Vec<DspDeviceSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DspDeviceSpec {
    pub bypassed: bool,
    pub kind: DspDeviceKind,
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
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MixParams {
    pub(crate) pitch_ratio: f32,
    pub(crate) level: f32,
    pub(crate) pan: f32,
}

pub(crate) fn track_dsp_chain(track_id: u32, graph: &DspGraphSpec) -> &[DspDeviceSpec] {
    graph
        .track_chains
        .iter()
        .find(|chain| chain.track_id == track_id)
        .map_or(&[], |chain| chain.devices.as_slice())
}

pub(crate) fn validate_dsp_chain(devices: &[DspDeviceSpec]) -> Result<(), AudioExportError> {
    for device in devices {
        if device.bypassed {
            continue;
        }
        match device.kind {
            DspDeviceKind::Gain { gain } if gain.is_finite() && gain >= 0.0 => {}
            DspDeviceKind::Pan { pan } if pan.is_finite() && (-1.0..=1.0).contains(&pan) => {}
            DspDeviceKind::Balance { balance }
                if balance.is_finite() && (-1.0..=1.0).contains(&balance) => {}
            DspDeviceKind::StereoWidth { width }
                if width.is_finite() && (0.0..=2.0).contains(&width) => {}
            DspDeviceKind::PhaseInvert { .. } => {}
            _ => return Err(AudioExportError::InvalidDspParameter),
        }
    }
    Ok(())
}

pub(crate) fn apply_dsp_chain_to_buffer(
    data: &mut [f32],
    channels: usize,
    devices: &[DspDeviceSpec],
) {
    for device in devices {
        if device.bypassed {
            continue;
        }
        match device.kind {
            DspDeviceKind::Gain { gain } if gain.is_finite() && gain >= 0.0 => {
                for sample in data.iter_mut() {
                    *sample *= gain;
                }
            }
            DspDeviceKind::Pan { pan }
                if pan.is_finite() && (-1.0..=1.0).contains(&pan) && channels > 0 =>
            {
                for frame in data.chunks_exact_mut(channels) {
                    for (channel, sample) in frame.iter_mut().enumerate() {
                        *sample *= pan_gain(pan, channel, channels);
                    }
                }
            }
            DspDeviceKind::Balance { balance }
                if balance.is_finite() && (-1.0..=1.0).contains(&balance) && channels > 0 =>
            {
                for frame in data.chunks_exact_mut(channels) {
                    for (channel, sample) in frame.iter_mut().enumerate() {
                        *sample *= pan_gain(balance, channel, channels);
                    }
                }
            }
            DspDeviceKind::StereoWidth { width }
                if width.is_finite() && (0.0..=2.0).contains(&width) =>
            {
                apply_stereo_width(data, channels, width);
            }
            DspDeviceKind::PhaseInvert {
                invert_left,
                invert_right,
            } => apply_phase_invert(data, channels, invert_left, invert_right),
            _ => {}
        }
    }
}

pub(crate) fn apply_dsp_chain_to_frame(frame: &mut [f32], devices: &[DspDeviceSpec]) {
    apply_dsp_chain_to_buffer(frame, frame.len(), devices);
}

pub(crate) fn apply_dsp_gain_to_aux_sample(sample: f32, devices: &[DspDeviceSpec]) -> f32 {
    devices.iter().fold(sample, |sample, device| {
        if device.bypassed {
            return sample;
        }
        match device.kind {
            DspDeviceKind::Gain { gain } if gain.is_finite() && gain >= 0.0 => sample * gain,
            _ => sample,
        }
    })
}

fn apply_stereo_width(data: &mut [f32], channels: usize, width: f32) {
    if channels < 2 {
        return;
    }
    for frame in data.chunks_exact_mut(channels) {
        let mid = (frame[0] + frame[1]) * 0.5;
        let side = (frame[0] - frame[1]) * 0.5 * width;
        frame[0] = mid + side;
        frame[1] = mid - side;
    }
}

fn apply_phase_invert(data: &mut [f32], channels: usize, invert_left: bool, invert_right: bool) {
    if channels == 0 {
        return;
    }
    for frame in data.chunks_exact_mut(channels) {
        if invert_left {
            frame[0] = -frame[0];
        }
        if invert_right && channels > 1 {
            frame[1] = -frame[1];
        }
    }
}

pub(crate) fn pan_gain(pan: f32, channel: usize, channels: usize) -> f32 {
    if channels < 2 {
        return 1.0;
    }
    match channel {
        0 if pan > 0.0 => 1.0 - pan,
        1 if pan < 0.0 => 1.0 + pan,
        _ => 1.0,
    }
}
