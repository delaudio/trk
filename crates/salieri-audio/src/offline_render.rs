use std::collections::HashMap;

use salieri_sampler::PreviewBuffer;

use crate::{
    backend::AudioConfig,
    dsp::{
        apply_dsp_chain_to_buffer, apply_dsp_chain_to_frame, apply_dsp_gain_to_aux_sample,
        pan_gain, send_bus, track_dsp_chain, track_send_levels, validate_dsp_chain, DspDeviceSpec,
        DspFrameProcessor, DspGraphSpec, MixParams,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioSamplerPlaybackMode {
    #[default]
    OneShot,
    ForwardLoop,
    BackwardLoop,
    PingPongLoop,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioSamplerPlaybackSettings {
    pub mode: AudioSamplerPlaybackMode,
    pub loop_start_frame: Option<usize>,
    pub loop_end_frame: Option<usize>,
}

impl Default for AudioSamplerPlaybackSettings {
    fn default() -> Self {
        Self {
            mode: AudioSamplerPlaybackMode::OneShot,
            loop_start_frame: None,
            loop_end_frame: None,
        }
    }
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
    pub playback: AudioSamplerPlaybackSettings,
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
    let mut send_buffers = dsp_graph
        .sends
        .iter()
        .map(|send| (send.send_id, vec![0.0; frames.saturating_mul(channels)]))
        .collect::<HashMap<_, _>>();
    for send in &dsp_graph.sends {
        validate_dsp_chain(&send.devices)?;
    }

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
        let params = MixParams {
            pitch_ratio,
            level,
            pan,
        };
        let track_devices = track_dsp_chain(event.track_id, dsp_graph);
        validate_dsp_chain(track_devices)?;
        let context = OfflineMixContext {
            output_frames: frames,
            channels,
            output_start_frame: output_frame,
            sample_rate: spec.sample_rate,
        };
        mix_sample_event(
            &mut data,
            context,
            sample,
            params,
            event.playback,
            track_devices,
        );
        for track_send in track_send_levels(event.track_id, dsp_graph) {
            let Some(bus) = send_bus(track_send.send_id, dsp_graph) else {
                continue;
            };
            let Some(send_output) = send_buffers.get_mut(&track_send.send_id) else {
                continue;
            };
            if !track_send.gain.is_finite() {
                continue;
            }
            let send_params = MixParams {
                level: params.level * track_send.gain.max(0.0),
                ..params
            };
            let tap_devices = if bus.pre_fader { &[] } else { track_devices };
            mix_sample_event(
                send_output,
                context,
                sample,
                send_params,
                event.playback,
                tap_devices,
            );
        }
    }

    for send in &dsp_graph.sends {
        if let Some(send_output) = send_buffers.get_mut(&send.send_id) {
            apply_dsp_chain_to_buffer(send_output, channels, spec.sample_rate, &send.devices);
            sum_buffer_into(&mut data, send_output);
        }
    }
    apply_dsp_chain_to_buffer(&mut data, channels, spec.sample_rate, &dsp_graph.master);

    Ok(RenderedAudio {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        frames,
        data,
    })
}

fn sum_buffer_into(output: &mut [f32], source: &[f32]) {
    for (output, source) in output.iter_mut().zip(source.iter()) {
        *output += *source;
    }
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
        let rendered_frames = inferred_event_frames(sample, event.playback, pitch_ratio);
        Ok(frames.max(start.saturating_add(rendered_frames)))
    })
}

fn inferred_event_frames(
    sample: &PreviewBuffer,
    playback: AudioSamplerPlaybackSettings,
    pitch_ratio: f32,
) -> usize {
    match playback.mode {
        AudioSamplerPlaybackMode::ForwardLoop
        | AudioSamplerPlaybackMode::BackwardLoop
        | AudioSamplerPlaybackMode::PingPongLoop
            if valid_loop_window(sample, playback).is_some() =>
        {
            sample.frames
        }
        _ => ((sample.frames as f32) / pitch_ratio).ceil() as usize,
    }
}

#[derive(Debug, Clone, Copy)]
struct OfflineMixContext {
    output_frames: usize,
    channels: usize,
    output_start_frame: usize,
    sample_rate: u32,
}

fn mix_sample_event(
    output: &mut [f32],
    context: OfflineMixContext,
    sample: &PreviewBuffer,
    params: MixParams,
    playback: AudioSamplerPlaybackSettings,
    track_devices: &[DspDeviceSpec],
) {
    let mut source_frame = 0.0_f32;
    let mut output_frame = context.output_start_frame;
    let mut processor = DspFrameProcessor::default();
    processor.prepare(context.sample_rate, context.channels, track_devices);
    while output_frame < context.output_frames {
        let Some(resolved_source_frame) =
            resolve_playback_source_frame(sample, playback, source_frame)
        else {
            break;
        };
        let output_offset = output_frame * context.channels;
        if context.channels == 1 {
            let mut frame =
                [
                    interpolated_sample(sample, resolved_source_frame, 0, context.channels)
                        * params.level,
                ];
            apply_dsp_chain_to_frame(
                &mut processor,
                &mut frame,
                context.sample_rate,
                track_devices,
            );
            output[output_offset] += frame[0];
        } else {
            let mut frame = [
                interpolated_sample(sample, resolved_source_frame, 0, context.channels)
                    * params.level
                    * pan_gain(params.pan, 0, context.channels),
                interpolated_sample(sample, resolved_source_frame, 1, context.channels)
                    * params.level
                    * pan_gain(params.pan, 1, context.channels),
            ];
            apply_dsp_chain_to_frame(
                &mut processor,
                &mut frame,
                context.sample_rate,
                track_devices,
            );
            output[output_offset] += frame[0];
            output[output_offset + 1] += frame[1];
            for channel in 2..context.channels {
                let sample_value =
                    interpolated_sample(sample, resolved_source_frame, channel, context.channels)
                        * params.level
                        * pan_gain(params.pan, channel, context.channels);
                output[output_offset + channel] +=
                    apply_dsp_gain_to_aux_sample(sample_value, track_devices);
            }
        }
        source_frame += params.pitch_ratio;
        output_frame += 1;
    }
}

pub(crate) fn valid_loop_window(
    sample: &PreviewBuffer,
    playback: AudioSamplerPlaybackSettings,
) -> Option<(usize, usize)> {
    let start = playback.loop_start_frame?;
    let end = playback.loop_end_frame?;
    if start < end && end <= sample.frames {
        Some((start, end))
    } else {
        None
    }
}

pub(crate) fn resolve_playback_source_frame(
    sample: &PreviewBuffer,
    playback: AudioSamplerPlaybackSettings,
    source_frame: f32,
) -> Option<f32> {
    if sample.frames == 0 || !source_frame.is_finite() || source_frame < 0.0 {
        return None;
    }
    match playback.mode {
        AudioSamplerPlaybackMode::OneShot => {
            (source_frame < sample.frames as f32).then_some(source_frame)
        }
        AudioSamplerPlaybackMode::Reverse => {
            if source_frame >= sample.frames as f32 {
                None
            } else {
                Some((sample.frames - 1) as f32 - source_frame)
            }
        }
        AudioSamplerPlaybackMode::ForwardLoop => {
            let (start, end) = valid_loop_window(sample, playback)?;
            Some(resolve_forward_loop_frame(source_frame, start, end))
        }
        AudioSamplerPlaybackMode::BackwardLoop => {
            let (start, end) = valid_loop_window(sample, playback)?;
            Some(resolve_backward_loop_frame(source_frame, start, end))
        }
        AudioSamplerPlaybackMode::PingPongLoop => {
            let (start, end) = valid_loop_window(sample, playback)?;
            Some(resolve_ping_pong_loop_frame(source_frame, start, end))
        }
    }
}

fn resolve_forward_loop_frame(source_frame: f32, start: usize, end: usize) -> f32 {
    if source_frame < end as f32 {
        return source_frame;
    }
    let width = (end - start) as f32;
    start as f32 + (source_frame - start as f32).rem_euclid(width)
}

fn resolve_backward_loop_frame(source_frame: f32, start: usize, end: usize) -> f32 {
    if source_frame < end as f32 {
        return source_frame;
    }
    let width = (end - start) as f32;
    (end - 1) as f32 - (source_frame - end as f32).rem_euclid(width)
}

fn resolve_ping_pong_loop_frame(source_frame: f32, start: usize, end: usize) -> f32 {
    if source_frame < end as f32 {
        return source_frame;
    }
    let width = end - start;
    if width <= 1 {
        return start as f32;
    }
    let period = ((width - 1) * 2) as f32;
    let position = (source_frame - end as f32).rem_euclid(period);
    if position < (width - 1) as f32 {
        (end - 2) as f32 - position
    } else {
        start as f32 + (position - (width - 1) as f32 + 1.0)
    }
}
