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
    Gain { gain: f32 },
    Pan { pan: f32 },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MixParams {
    pub(crate) pitch_ratio: f32,
    pub(crate) level: f32,
    pub(crate) pan: f32,
}

pub(crate) fn apply_track_dsp_to_mix_params(
    params: MixParams,
    track_id: u32,
    graph: &DspGraphSpec,
) -> Result<MixParams, AudioExportError> {
    let Some(chain) = graph
        .track_chains
        .iter()
        .find(|chain| chain.track_id == track_id)
    else {
        return Ok(params);
    };

    chain
        .devices
        .iter()
        .try_fold(params, apply_device_to_mix_params)
}

pub(crate) fn apply_track_dsp_to_mix_params_lossy(
    params: MixParams,
    track_id: u32,
    graph: &DspGraphSpec,
) -> MixParams {
    apply_track_dsp_to_mix_params(params, track_id, graph).unwrap_or(params)
}

fn apply_device_to_mix_params(
    params: MixParams,
    device: &DspDeviceSpec,
) -> Result<MixParams, AudioExportError> {
    if device.bypassed {
        return Ok(params);
    }
    match device.kind {
        DspDeviceKind::Gain { gain } => {
            if !gain.is_finite() || gain < 0.0 {
                return Err(AudioExportError::InvalidDspParameter);
            }
            Ok(MixParams {
                level: params.level * gain,
                ..params
            })
        }
        DspDeviceKind::Pan { pan } => {
            if !pan.is_finite() || !(-1.0..=1.0).contains(&pan) {
                return Err(AudioExportError::InvalidDspParameter);
            }
            Ok(MixParams {
                pan: (params.pan + pan).clamp(-1.0, 1.0),
                ..params
            })
        }
    }
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
            _ => {}
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
