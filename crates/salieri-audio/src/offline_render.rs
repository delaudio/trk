use std::collections::HashMap;

use salieri_sampler::PreviewBuffer;

use crate::{
    backend::AudioConfig,
    dsp::{
        apply_dsp_chain_to_buffer, apply_track_dsp_to_mix_params, pan_gain, DspGraphSpec, MixParams,
    },
    errors::AudioExportError,
    shared::{interpolated_sample, validate_sampler_render_sample, validated_pitch_ratio},
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfflineRenderSpec {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: usize,
}

impl Default for OfflineRenderSpec {
    fn default() -> Self {
        Self {
            sample_rate: AudioConfig::default().sample_rate,
            channels: AudioConfig::default().channels,
            frames: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedAudio {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: usize,
    pub data: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OfflineSamplerSample {
    pub sample_id: u32,
    pub buffer: PreviewBuffer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OfflineSamplerEvent {
    pub track_id: u32,
    pub sample_id: u32,
    pub frame: u64,
    pub gain: f32,
    pub pan: f32,
    pub pitch_ratio: f32,
    pub velocity: u8,
}

pub fn render_sampler_preview(
    preview: &PreviewBuffer,
    spec: OfflineRenderSpec,
) -> Result<RenderedAudio, AudioExportError> {
    if preview.sample_rate != spec.sample_rate {
        return Err(AudioExportError::UnsupportedSampleRateConversion {
            source_sample_rate: preview.sample_rate,
            target_sample_rate: spec.sample_rate,
        });
    }
    if preview.channels != spec.channels {
        return Err(AudioExportError::UnsupportedChannelConversion {
            source_channels: preview.channels,
            target_channels: spec.channels,
        });
    }

    let channels = usize::from(spec.channels);
    let frames = if spec.frames == 0 {
        preview.frames
    } else {
        spec.frames
    };
    let expected = preview.frames.saturating_mul(channels);
    if preview.data.len() != expected {
        return Err(AudioExportError::InvalidBufferLength {
            expected,
            actual: preview.data.len(),
        });
    }

    let mut data = vec![0.0; frames.saturating_mul(channels)];
    let copy_len = data.len().min(preview.data.len());
    data[..copy_len].copy_from_slice(&preview.data[..copy_len]);

    Ok(RenderedAudio {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        frames,
        data,
    })
}

pub fn render_sampler_events(
    samples: &[OfflineSamplerSample],
    events: &[OfflineSamplerEvent],
    spec: OfflineRenderSpec,
) -> Result<RenderedAudio, AudioExportError> {
    render_sampler_events_with_dsp(samples, events, spec, &DspGraphSpec::default())
}

pub fn render_sampler_events_with_dsp(
    samples: &[OfflineSamplerSample],
    events: &[OfflineSamplerEvent],
    spec: OfflineRenderSpec,
    dsp_graph: &DspGraphSpec,
) -> Result<RenderedAudio, AudioExportError> {
    let sample_lookup = samples
        .iter()
        .map(|sample| (sample.sample_id, &sample.buffer))
        .collect::<HashMap<_, _>>();
    let channels = usize::from(spec.channels);
    let frames = if spec.frames == 0 {
        infer_sampler_render_frames(&sample_lookup, events)?
    } else {
        spec.frames
    };
    let mut data = vec![0.0; frames.saturating_mul(channels)];

    for event in events {
        let sample =
            sample_lookup
                .get(&event.sample_id)
                .ok_or(AudioExportError::MissingSample {
                    sample_id: event.sample_id,
                })?;
        validate_sampler_render_sample(sample, spec)?;
        let pitch_ratio = validated_pitch_ratio(event.pitch_ratio)?;
        let output_frame =
            usize::try_from(event.frame).map_err(|_| AudioExportError::WavTooLarge)?;
        if output_frame >= frames {
            continue;
        }

        let level = event.gain.max(0.0) * (f32::from(event.velocity.min(0x7f)) / 127.0);
        let pan = event.pan.clamp(-1.0, 1.0);
        let params = apply_track_dsp_to_mix_params(
            MixParams {
                pitch_ratio,
                level,
                pan,
            },
            event.track_id,
            dsp_graph,
        )?;
        mix_sample_event(&mut data, frames, channels, sample, output_frame, params);
    }

    apply_dsp_chain_to_buffer(&mut data, channels, &dsp_graph.master);

    Ok(RenderedAudio {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        frames,
        data,
    })
}

fn infer_sampler_render_frames(
    samples: &HashMap<u32, &PreviewBuffer>,
    events: &[OfflineSamplerEvent],
) -> Result<usize, AudioExportError> {
    events.iter().try_fold(0_usize, |frames, event| {
        let sample = samples
            .get(&event.sample_id)
            .ok_or(AudioExportError::MissingSample {
                sample_id: event.sample_id,
            })?;
        let pitch_ratio = validated_pitch_ratio(event.pitch_ratio)?;
        let start = usize::try_from(event.frame).map_err(|_| AudioExportError::WavTooLarge)?;
        let rendered_frames = ((sample.frames as f32) / pitch_ratio).ceil() as usize;
        Ok(frames.max(start.saturating_add(rendered_frames)))
    })
}

fn mix_sample_event(
    output: &mut [f32],
    output_frames: usize,
    channels: usize,
    sample: &PreviewBuffer,
    output_start_frame: usize,
    params: MixParams,
) {
    let mut source_frame = 0.0_f32;
    let mut output_frame = output_start_frame;
    while output_frame < output_frames && source_frame < sample.frames as f32 {
        let output_offset = output_frame * channels;
        for channel in 0..channels {
            output[output_offset + channel] +=
                interpolated_sample(sample, source_frame, channel, channels)
                    * params.level
                    * pan_gain(params.pan, channel, channels);
        }
        source_frame += params.pitch_ratio;
        output_frame += 1;
    }
}
