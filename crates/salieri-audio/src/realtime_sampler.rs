use std::collections::HashMap;

use salieri_sampler::PreviewBuffer;

use crate::{
    backend::AudioConfig,
    dsp::{
        apply_dsp_chain_to_buffer, apply_dsp_chain_to_frame, apply_dsp_gain_to_aux_sample,
        pan_gain, track_dsp_chain, DspDeviceSpec, DspGraphSpec, MixParams,
    },
    errors::AudioExportError,
    offline_render::{OfflineRenderSpec, RenderedAudio},
    shared::{
        converted_channel_sample, interpolated_sample, validate_sampler_render_sample,
        validated_pitch_ratio,
    },
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub enum RealtimeAudioCommand {
    TriggerSample {
        track_id: u32,
        sample_id: u32,
        frame: u64,
        gain: f32,
        pan: f32,
        pitch_ratio: f32,
    },
    StopVoice {
        voice_id: u64,
        frame: u64,
    },
    AllNotesOff {
        frame: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealtimeSamplerConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub max_voices: usize,
}

impl Default for RealtimeSamplerConfig {
    fn default() -> Self {
        Self {
            sample_rate: AudioConfig::default().sample_rate,
            channels: AudioConfig::default().channels,
            max_voices: 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RealtimeSamplerVoice {
    voice_id: u64,
    track_id: u32,
    sample_id: u32,
    start_frame: u64,
    gain: f32,
    pan: f32,
    pitch_ratio: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RealtimeSampler {
    config: RealtimeSamplerConfig,
    samples: HashMap<u32, PreviewBuffer>,
    voices: Vec<RealtimeSamplerVoice>,
    dsp_graph: DspGraphSpec,
    next_voice_id: u64,
    current_frame: u64,
}

impl RealtimeSampler {
    #[must_use]
    pub fn new(config: RealtimeSamplerConfig) -> Self {
        Self {
            config,
            samples: HashMap::new(),
            voices: Vec::with_capacity(config.max_voices),
            dsp_graph: DspGraphSpec::default(),
            next_voice_id: 1,
            current_frame: 0,
        }
    }

    pub fn register_sample(
        &mut self,
        sample_id: u32,
        buffer: PreviewBuffer,
    ) -> Result<(), AudioExportError> {
        validate_sampler_render_sample(
            &buffer,
            OfflineRenderSpec {
                sample_rate: self.config.sample_rate,
                channels: self.config.channels,
                frames: 0,
            },
        )?;
        self.samples.insert(sample_id, buffer);
        Ok(())
    }

    pub fn remove_sample(&mut self, sample_id: u32) {
        self.samples.remove(&sample_id);
        self.voices.retain(|voice| voice.sample_id != sample_id);
    }

    pub fn clear_samples(&mut self) {
        self.samples.clear();
        self.voices.clear();
    }

    pub fn set_dsp_graph(&mut self, graph: DspGraphSpec) {
        self.dsp_graph = graph;
    }

    #[must_use]
    pub fn active_voice_count(&self) -> usize {
        self.voices.len()
    }

    pub fn handle_command(
        &mut self,
        command: RealtimeAudioCommand,
    ) -> Result<Option<u64>, AudioExportError> {
        match command {
            RealtimeAudioCommand::TriggerSample {
                track_id,
                sample_id,
                frame,
                gain,
                pan,
                pitch_ratio,
            } => {
                if !self.samples.contains_key(&sample_id) {
                    return Err(AudioExportError::MissingSample { sample_id });
                }
                let pitch_ratio = validated_pitch_ratio(pitch_ratio)?;
                if self.config.max_voices == 0 {
                    return Ok(None);
                }
                if self.voices.len() >= self.config.max_voices {
                    self.voices.remove(0);
                }
                let voice_id = self.next_voice_id;
                self.next_voice_id = self.next_voice_id.saturating_add(1);
                self.voices.push(RealtimeSamplerVoice {
                    voice_id,
                    track_id,
                    sample_id,
                    start_frame: frame,
                    gain: gain.max(0.0),
                    pan: pan.clamp(-1.0, 1.0),
                    pitch_ratio,
                });
                Ok(Some(voice_id))
            }
            RealtimeAudioCommand::StopVoice { voice_id, .. } => {
                self.voices.retain(|voice| voice.voice_id != voice_id);
                Ok(None)
            }
            RealtimeAudioCommand::AllNotesOff { .. } => {
                self.voices.clear();
                Ok(None)
            }
        }
    }

    pub fn handle_command_now(
        &mut self,
        command: RealtimeAudioCommand,
    ) -> Result<Option<u64>, AudioExportError> {
        let command = match command {
            RealtimeAudioCommand::TriggerSample {
                track_id,
                sample_id,
                gain,
                pan,
                pitch_ratio,
                ..
            } => RealtimeAudioCommand::TriggerSample {
                track_id,
                sample_id,
                frame: self.current_frame,
                gain,
                pan,
                pitch_ratio,
            },
            RealtimeAudioCommand::StopVoice { voice_id, .. } => RealtimeAudioCommand::StopVoice {
                voice_id,
                frame: self.current_frame,
            },
            RealtimeAudioCommand::AllNotesOff { .. } => RealtimeAudioCommand::AllNotesOff {
                frame: self.current_frame,
            },
        };
        self.handle_command(command)
    }

    pub fn render(&mut self, frames: usize) -> RenderedAudio {
        let channels = usize::from(self.config.channels);
        let mut data = vec![0.0; frames.saturating_mul(channels)];
        self.render_into(&mut data);

        RenderedAudio {
            sample_rate: self.config.sample_rate,
            channels: self.config.channels,
            frames,
            data,
        }
    }

    pub fn render_into(&mut self, data: &mut [f32]) {
        data.fill(0.0);
        let channels = usize::from(self.config.channels);
        if channels == 0 {
            return;
        }
        let frames = data.len() / channels;
        let render_start = self.current_frame;
        let render_end = render_start.saturating_add(frames as u64);

        for voice in &self.voices {
            let Some(sample) = self.samples.get(&voice.sample_id) else {
                continue;
            };
            mix_realtime_voice(
                data,
                channels,
                sample,
                voice,
                render_start,
                render_end,
                &self.dsp_graph,
            );
        }

        apply_dsp_chain_to_buffer(data, channels, &self.dsp_graph.master);

        self.current_frame = render_end;
        let current_frame = self.current_frame;
        let samples = &self.samples;
        self.voices.retain(|voice| {
            samples.get(&voice.sample_id).is_some_and(|sample| {
                match voice_end_frame(voice, sample) {
                    Some(end_frame) => end_frame > current_frame,
                    None => true,
                }
            })
        });
    }
}

#[must_use]
pub fn prepare_realtime_sample(
    preview: &PreviewBuffer,
    target_sample_rate: u32,
    target_channels: u16,
) -> PreviewBuffer {
    if preview.sample_rate == target_sample_rate && preview.channels == target_channels {
        return preview.clone();
    }

    let target_channels_usize = usize::from(target_channels).max(1);
    let target_frames = if preview.sample_rate == 0 || target_sample_rate == 0 {
        preview.frames
    } else {
        ((preview.frames as f64) * f64::from(target_sample_rate) / f64::from(preview.sample_rate))
            .ceil() as usize
    };
    let frame_ratio = if target_sample_rate == 0 {
        1.0
    } else {
        preview.sample_rate as f32 / target_sample_rate as f32
    };
    let mut data = vec![0.0; target_frames.saturating_mul(target_channels_usize)];

    for target_frame in 0..target_frames {
        let source_frame = target_frame as f32 * frame_ratio;
        let output_offset = target_frame * target_channels_usize;
        for target_channel in 0..target_channels_usize {
            data[output_offset + target_channel] = converted_channel_sample(
                preview,
                source_frame,
                target_channel,
                target_channels_usize,
            );
        }
    }

    PreviewBuffer {
        sample_rate: target_sample_rate,
        channels: target_channels,
        frames: target_frames,
        data,
    }
}

#[must_use]
pub fn slice_preview_buffer(
    preview: &PreviewBuffer,
    start_frame: Option<usize>,
    end_frame: Option<usize>,
) -> PreviewBuffer {
    let channels = usize::from(preview.channels).max(1);
    let start = start_frame.unwrap_or(0).min(preview.frames);
    let end = end_frame
        .unwrap_or(preview.frames)
        .min(preview.frames)
        .max(start);
    let start_offset = start.saturating_mul(channels);
    let end_offset = end.saturating_mul(channels).min(preview.data.len());

    PreviewBuffer {
        sample_rate: preview.sample_rate,
        channels: preview.channels,
        frames: end.saturating_sub(start),
        data: preview.data[start_offset..end_offset].to_vec(),
    }
}

#[must_use]
pub fn apply_preview_envelope(
    preview: &PreviewBuffer,
    attack_frames: usize,
    decay_frames: usize,
    sustain: f32,
    release_frames: usize,
) -> PreviewBuffer {
    let channels = usize::from(preview.channels).max(1);
    let attack_frames = attack_frames.min(preview.frames);
    let decay_frames = decay_frames.min(preview.frames.saturating_sub(attack_frames));
    let release_frames = release_frames.min(preview.frames);
    let sustain = sustain.clamp(0.0, 1.0);
    let release_start = preview.frames.saturating_sub(release_frames);
    let mut data = preview.data.clone();

    for frame in 0..preview.frames {
        let mut gain = if attack_frames > 0 && frame < attack_frames {
            frame as f32 / attack_frames as f32
        } else if decay_frames > 0 && frame < attack_frames.saturating_add(decay_frames) {
            let decay_position = frame.saturating_sub(attack_frames) as f32 / decay_frames as f32;
            1.0 - decay_position * (1.0 - sustain)
        } else {
            sustain
        };
        if release_frames > 0 && frame >= release_start {
            let release_position =
                frame.saturating_sub(release_start) as f32 / release_frames as f32;
            gain *= 1.0 - release_position;
        }

        let offset = frame.saturating_mul(channels);
        for sample in data.iter_mut().skip(offset).take(channels) {
            *sample *= gain;
        }
    }

    PreviewBuffer {
        sample_rate: preview.sample_rate,
        channels: preview.channels,
        frames: preview.frames,
        data,
    }
}

fn mix_realtime_voice(
    output: &mut [f32],
    channels: usize,
    sample: &PreviewBuffer,
    voice: &RealtimeSamplerVoice,
    render_start: u64,
    render_end: u64,
    dsp_graph: &DspGraphSpec,
) {
    let voice_end = voice_end_frame(voice, sample).unwrap_or(u64::MAX);
    let mix_start = render_start.max(voice.start_frame);
    let mix_end = render_end.min(voice_end);
    if mix_start >= mix_end {
        return;
    }

    let params = MixParams {
        pitch_ratio: voice.pitch_ratio,
        level: voice.gain,
        pan: voice.pan,
    };
    let track_devices = track_dsp_chain(voice.track_id, dsp_graph);

    for absolute_frame in mix_start..mix_end {
        let output_frame = (absolute_frame - render_start) as usize;
        let source_frame = (absolute_frame - voice.start_frame) as f32 * voice.pitch_ratio;
        let output_offset = output_frame * channels;
        mix_realtime_frame(
            output,
            output_offset,
            channels,
            sample,
            source_frame,
            params,
            track_devices,
        );
    }
}

fn mix_realtime_frame(
    output: &mut [f32],
    output_offset: usize,
    channels: usize,
    sample: &PreviewBuffer,
    source_frame: f32,
    params: MixParams,
    track_devices: &[DspDeviceSpec],
) {
    if channels == 1 {
        let mut frame = [interpolated_sample(sample, source_frame, 0, channels) * params.level];
        apply_dsp_chain_to_frame(&mut frame, track_devices);
        output[output_offset] += frame[0];
        return;
    }

    let mut frame = [
        interpolated_sample(sample, source_frame, 0, channels)
            * params.level
            * pan_gain(params.pan, 0, channels),
        interpolated_sample(sample, source_frame, 1, channels)
            * params.level
            * pan_gain(params.pan, 1, channels),
    ];
    apply_dsp_chain_to_frame(&mut frame, track_devices);
    output[output_offset] += frame[0];
    output[output_offset + 1] += frame[1];
    for channel in 2..channels {
        let sample_value = interpolated_sample(sample, source_frame, channel, channels)
            * params.level
            * pan_gain(params.pan, channel, channels);
        output[output_offset + channel] +=
            apply_dsp_gain_to_aux_sample(sample_value, track_devices);
    }
}

fn voice_end_frame(voice: &RealtimeSamplerVoice, sample: &PreviewBuffer) -> Option<u64> {
    if sample.frames == 0 {
        return Some(voice.start_frame);
    }
    let rendered_frames = ((sample.frames as f32) / voice.pitch_ratio).ceil();
    if !rendered_frames.is_finite() || rendered_frames <= 0.0 {
        return None;
    }
    Some(voice.start_frame.saturating_add(rendered_frames as u64))
}
