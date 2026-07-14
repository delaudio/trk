use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use salieri_sampler::PreviewBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_frames: u16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
            buffer_frames: 256,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioCommand {
    Start,
    Stop,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioUpdate {
    Started(AudioConfig),
    Stopped,
    Shutdown,
    Error(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("audio backend failed to start: {0}")]
    Start(String),
    #[error("audio backend failed to stop: {0}")]
    Stop(String),
}

pub trait AudioBackend: Send + 'static {
    fn start(&mut self, config: AudioConfig) -> Result<(), AudioError>;
    fn stop(&mut self) -> Result<(), AudioError>;
}

#[derive(Debug, Default)]
pub struct NullAudioBackend {
    started: bool,
}

impl NullAudioBackend {
    #[must_use]
    pub const fn is_started(&self) -> bool {
        self.started
    }
}

impl AudioBackend for NullAudioBackend {
    fn start(&mut self, _config: AudioConfig) -> Result<(), AudioError> {
        self.started = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        self.started = false;
        Ok(())
    }
}

#[derive(Debug)]
pub struct AudioRuntime {
    command_tx: Sender<AudioCommand>,
    update_rx: Receiver<AudioUpdate>,
    handle: Option<JoinHandle<()>>,
}

impl AudioRuntime {
    #[must_use]
    pub fn spawn<B>(config: AudioConfig, backend: B) -> Self
    where
        B: AudioBackend,
    {
        let (command_tx, command_rx) = mpsc::channel();
        let (update_tx, update_rx) = mpsc::channel();
        let handle = thread::spawn(move || audio_thread(config, backend, command_rx, update_tx));

        Self {
            command_tx,
            update_rx,
            handle: Some(handle),
        }
    }

    pub fn start(&self) {
        let _ = self.command_tx.send(AudioCommand::Start);
    }

    pub fn stop(&self) {
        let _ = self.command_tx.send(AudioCommand::Stop);
    }

    pub fn shutdown(&self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
    }

    #[must_use]
    pub fn try_recv(&self) -> Option<AudioUpdate> {
        self.update_rx.try_recv().ok()
    }
}

impl Drop for AudioRuntime {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn audio_thread<B>(
    config: AudioConfig,
    mut backend: B,
    command_rx: Receiver<AudioCommand>,
    update_tx: Sender<AudioUpdate>,
) where
    B: AudioBackend,
{
    let mut running = false;
    while let Ok(command) = command_rx.recv() {
        match command {
            AudioCommand::Start => match backend.start(config) {
                Ok(()) => {
                    running = true;
                    let _ = update_tx.send(AudioUpdate::Started(config));
                }
                Err(error) => {
                    let _ = update_tx.send(AudioUpdate::Error(error.to_string()));
                }
            },
            AudioCommand::Stop => {
                if running {
                    match backend.stop() {
                        Ok(()) => {
                            running = false;
                            let _ = update_tx.send(AudioUpdate::Stopped);
                        }
                        Err(error) => {
                            let _ = update_tx.send(AudioUpdate::Error(error.to_string()));
                        }
                    }
                } else {
                    let _ = update_tx.send(AudioUpdate::Stopped);
                }
            }
            AudioCommand::Shutdown => {
                if running {
                    let _ = backend.stop();
                }
                let _ = update_tx.send(AudioUpdate::Shutdown);
                break;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RealtimeAudioCommand {
    TriggerSample {
        sample_id: u32,
        frame: u64,
        gain: f32,
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
pub enum AudioExportFormat {
    WavPcm16,
}

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
    pub sample_id: u32,
    pub frame: u64,
    pub gain: f32,
    pub pitch_ratio: f32,
    pub velocity: u8,
}

#[derive(Debug, thiserror::Error)]
pub enum AudioExportError {
    #[error("sample {sample_id} is missing from the offline render set")]
    MissingSample { sample_id: u32 },
    #[error("unsupported sample-rate conversion from {source_sample_rate} Hz to {target_sample_rate} Hz")]
    UnsupportedSampleRateConversion {
        source_sample_rate: u32,
        target_sample_rate: u32,
    },
    #[error("unsupported channel conversion from {source_channels} to {target_channels}")]
    UnsupportedChannelConversion {
        source_channels: u16,
        target_channels: u16,
    },
    #[error("invalid sampler pitch ratio {pitch_ratio}")]
    InvalidPitchRatio { pitch_ratio: f32 },
    #[error("rendered audio has {actual} samples, expected {expected}")]
    InvalidBufferLength { expected: usize, actual: usize },
    #[error("rendered audio is too large for a RIFF/WAV file")]
    WavTooLarge,
}

#[must_use]
pub const fn supported_audio_export_formats() -> &'static [AudioExportFormat] {
    &[AudioExportFormat::WavPcm16]
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
        mix_sample_event(
            &mut data,
            frames,
            channels,
            sample,
            output_frame,
            pitch_ratio,
            level,
        );
    }

    Ok(RenderedAudio {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        frames,
        data,
    })
}

pub fn encode_audio(
    audio: &RenderedAudio,
    format: AudioExportFormat,
) -> Result<Vec<u8>, AudioExportError> {
    match format {
        AudioExportFormat::WavPcm16 => encode_wav_pcm16(audio),
    }
}

fn encode_wav_pcm16(audio: &RenderedAudio) -> Result<Vec<u8>, AudioExportError> {
    let channels = usize::from(audio.channels);
    let expected = audio.frames.saturating_mul(channels);
    if audio.data.len() != expected {
        return Err(AudioExportError::InvalidBufferLength {
            expected,
            actual: audio.data.len(),
        });
    }

    let data_bytes = audio
        .data
        .len()
        .checked_mul(2)
        .ok_or(AudioExportError::WavTooLarge)?;
    let riff_size = 36_usize
        .checked_add(data_bytes)
        .ok_or(AudioExportError::WavTooLarge)?;
    let data_bytes = u32::try_from(data_bytes).map_err(|_| AudioExportError::WavTooLarge)?;
    let riff_size = u32::try_from(riff_size).map_err(|_| AudioExportError::WavTooLarge)?;
    let byte_rate = audio
        .sample_rate
        .checked_mul(u32::from(audio.channels))
        .and_then(|value| value.checked_mul(2))
        .ok_or(AudioExportError::WavTooLarge)?;
    let block_align = audio
        .channels
        .checked_mul(2)
        .ok_or(AudioExportError::WavTooLarge)?;

    let mut bytes = Vec::with_capacity(44 + audio.data.len() * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&audio.channels.to_le_bytes());
    bytes.extend_from_slice(&audio.sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in &audio.data {
        let sample = sample.clamp(-1.0, 1.0);
        let quantized = if sample >= 0.0 {
            (sample * f32::from(i16::MAX)).round() as i16
        } else {
            (sample * 32768.0).round() as i16
        };
        bytes.extend_from_slice(&quantized.to_le_bytes());
    }
    Ok(bytes)
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

fn validate_sampler_render_sample(
    sample: &PreviewBuffer,
    spec: OfflineRenderSpec,
) -> Result<(), AudioExportError> {
    if sample.sample_rate != spec.sample_rate {
        return Err(AudioExportError::UnsupportedSampleRateConversion {
            source_sample_rate: sample.sample_rate,
            target_sample_rate: spec.sample_rate,
        });
    }
    if sample.channels != spec.channels {
        return Err(AudioExportError::UnsupportedChannelConversion {
            source_channels: sample.channels,
            target_channels: spec.channels,
        });
    }

    let expected = sample.frames.saturating_mul(usize::from(sample.channels));
    if sample.data.len() != expected {
        return Err(AudioExportError::InvalidBufferLength {
            expected,
            actual: sample.data.len(),
        });
    }
    Ok(())
}

fn validated_pitch_ratio(pitch_ratio: f32) -> Result<f32, AudioExportError> {
    if pitch_ratio.is_finite() && pitch_ratio > 0.0 {
        Ok(pitch_ratio)
    } else {
        Err(AudioExportError::InvalidPitchRatio { pitch_ratio })
    }
}

fn mix_sample_event(
    output: &mut [f32],
    output_frames: usize,
    channels: usize,
    sample: &PreviewBuffer,
    output_start_frame: usize,
    pitch_ratio: f32,
    level: f32,
) {
    let mut source_frame = 0.0_f32;
    let mut output_frame = output_start_frame;
    while output_frame < output_frames && source_frame < sample.frames as f32 {
        let output_offset = output_frame * channels;
        for channel in 0..channels {
            output[output_offset + channel] +=
                interpolated_sample(sample, source_frame, channel, channels) * level;
        }
        source_frame += pitch_ratio;
        output_frame += 1;
    }
}

fn interpolated_sample(
    sample: &PreviewBuffer,
    source_frame: f32,
    channel: usize,
    channels: usize,
) -> f32 {
    let base_frame = source_frame.floor() as usize;
    let next_frame = (base_frame + 1).min(sample.frames.saturating_sub(1));
    let fractional = source_frame - base_frame as f32;
    let current = sample.data[base_frame * channels + channel];
    let next = sample.data[next_frame * channels + channel];
    current + ((next - current) * fractional)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn audio_runtime_starts_stops_and_shuts_down_backend() {
        let runtime = AudioRuntime::spawn(AudioConfig::default(), NullAudioBackend::default());

        runtime.start();
        assert_eq!(
            recv_update(&runtime),
            Some(AudioUpdate::Started(AudioConfig::default()))
        );

        runtime.stop();
        assert_eq!(recv_update(&runtime), Some(AudioUpdate::Stopped));

        runtime.shutdown();
        assert_eq!(recv_update(&runtime), Some(AudioUpdate::Shutdown));
    }

    #[test]
    fn stop_is_idempotent_when_audio_is_not_running() {
        let runtime = AudioRuntime::spawn(AudioConfig::default(), NullAudioBackend::default());

        runtime.stop();

        assert_eq!(recv_update(&runtime), Some(AudioUpdate::Stopped));
    }

    #[test]
    fn realtime_commands_are_plain_data_messages() {
        let command = RealtimeAudioCommand::TriggerSample {
            sample_id: 1,
            frame: 128,
            gain: 0.5,
            pitch_ratio: 2.0,
        };

        assert_eq!(
            command,
            RealtimeAudioCommand::TriggerSample {
                sample_id: 1,
                frame: 128,
                gain: 0.5,
                pitch_ratio: 2.0,
            }
        );
    }

    #[test]
    fn supported_export_formats_are_explicit() {
        assert_eq!(
            supported_audio_export_formats(),
            &[AudioExportFormat::WavPcm16]
        );
    }

    #[test]
    fn renders_sampler_preview_deterministically() {
        let preview = PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 2,
            data: vec![0.25, -0.25, 0.5, -0.5],
        };
        let spec = OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 2,
            frames: 4,
        };

        let first = render_sampler_preview(&preview, spec).expect("render");
        let second = render_sampler_preview(&preview, spec).expect("render");

        assert_eq!(first, second);
        assert_eq!(first.frames, 4);
        assert_eq!(first.data, vec![0.25, -0.25, 0.5, -0.5, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn renders_sampler_events_with_timing_gain_and_velocity() {
        let samples = vec![OfflineSamplerSample {
            sample_id: 7,
            buffer: mono_sample(vec![1.0, 0.5]),
        }];
        let events = vec![OfflineSamplerEvent {
            sample_id: 7,
            frame: 2,
            gain: 0.5,
            pitch_ratio: 1.0,
            velocity: 64,
        }];

        let rendered = render_sampler_events(
            &samples,
            &events,
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 1,
                frames: 5,
            },
        )
        .expect("render");

        assert_eq!(rendered.frames, 5);
        assert_eq!(rendered.data[0], 0.0);
        assert_eq!(rendered.data[1], 0.0);
        assert_approx_eq(rendered.data[2], 0.5 * (64.0 / 127.0));
        assert_approx_eq(rendered.data[3], 0.25 * (64.0 / 127.0));
        assert_eq!(rendered.data[4], 0.0);
    }

    #[test]
    fn renders_sampler_events_with_pitch_ratio() {
        let samples = vec![OfflineSamplerSample {
            sample_id: 1,
            buffer: mono_sample(vec![0.25, 0.5, 0.75, 1.0]),
        }];
        let events = vec![OfflineSamplerEvent {
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pitch_ratio: 2.0,
            velocity: 127,
        }];

        let rendered = render_sampler_events(
            &samples,
            &events,
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 1,
                frames: 0,
            },
        )
        .expect("render");

        assert_eq!(rendered.frames, 2);
        assert_eq!(rendered.data, vec![0.25, 0.75]);
    }

    #[test]
    fn renders_sampler_events_as_silence_without_events() {
        let rendered = render_sampler_events(
            &[],
            &[],
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 2,
                frames: 3,
            },
        )
        .expect("render");

        assert_eq!(rendered.frames, 3);
        assert_eq!(rendered.data, vec![0.0; 6]);
    }

    #[test]
    fn sampler_event_render_failures_are_clear() {
        let samples = vec![OfflineSamplerSample {
            sample_id: 1,
            buffer: mono_sample(vec![0.25]),
        }];

        assert!(matches!(
            render_sampler_events(
                &samples,
                &[OfflineSamplerEvent {
                    sample_id: 99,
                    frame: 0,
                    gain: 1.0,
                    pitch_ratio: 1.0,
                    velocity: 127,
                }],
                OfflineRenderSpec {
                    sample_rate: 48_000,
                    channels: 1,
                    frames: 1,
                },
            ),
            Err(AudioExportError::MissingSample { sample_id: 99 })
        ));

        assert!(matches!(
            render_sampler_events(
                &samples,
                &[OfflineSamplerEvent {
                    sample_id: 1,
                    frame: 0,
                    gain: 1.0,
                    pitch_ratio: 0.0,
                    velocity: 127,
                }],
                OfflineRenderSpec {
                    sample_rate: 48_000,
                    channels: 1,
                    frames: 1,
                },
            ),
            Err(AudioExportError::InvalidPitchRatio { pitch_ratio: 0.0 })
        ));
    }

    #[test]
    fn rendered_sampler_events_can_be_encoded_as_wav() {
        let samples = vec![OfflineSamplerSample {
            sample_id: 1,
            buffer: mono_sample(vec![0.5, -0.5]),
        }];
        let rendered = render_sampler_events(
            &samples,
            &[OfflineSamplerEvent {
                sample_id: 1,
                frame: 0,
                gain: 1.0,
                pitch_ratio: 1.0,
                velocity: 127,
            }],
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 1,
                frames: 2,
            },
        )
        .expect("render");

        let bytes = encode_audio(&rendered, AudioExportFormat::WavPcm16).expect("encode");

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
            4
        );
        assert_eq!(i16::from_le_bytes([bytes[44], bytes[45]]), 16_384);
        assert_eq!(i16::from_le_bytes([bytes[46], bytes[47]]), -16_384);
    }

    #[test]
    fn encodes_wav_pcm16_without_filesystem_side_effects() {
        let audio = RenderedAudio {
            sample_rate: 48_000,
            channels: 1,
            frames: 3,
            data: vec![-1.0, 0.0, 1.0],
        };

        let bytes = encode_audio(&audio, AudioExportFormat::WavPcm16).expect("encode");

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
            6
        );
        assert_eq!(i16::from_le_bytes([bytes[44], bytes[45]]), i16::MIN);
        assert_eq!(i16::from_le_bytes([bytes[46], bytes[47]]), 0);
        assert_eq!(i16::from_le_bytes([bytes[48], bytes[49]]), i16::MAX);
    }

    #[test]
    fn render_export_failures_are_clear() {
        let preview = PreviewBuffer {
            sample_rate: 44_100,
            channels: 2,
            frames: 1,
            data: vec![0.0, 0.0],
        };

        assert!(matches!(
            render_sampler_preview(
                &preview,
                OfflineRenderSpec {
                    sample_rate: 48_000,
                    channels: 2,
                    frames: 1,
                }
            ),
            Err(AudioExportError::UnsupportedSampleRateConversion {
                source_sample_rate: 44_100,
                target_sample_rate: 48_000
            })
        ));
    }

    fn recv_update(runtime: &AudioRuntime) -> Option<AudioUpdate> {
        let deadline = Instant::now() + Duration::from_millis(250);
        while Instant::now() < deadline {
            if let Some(update) = runtime.try_recv() {
                return Some(update);
            }
            thread::sleep(Duration::from_millis(1));
        }
        None
    }

    fn mono_sample(data: Vec<f32>) -> PreviewBuffer {
        PreviewBuffer {
            sample_rate: 48_000,
            channels: 1,
            frames: data.len(),
            data,
        }
    }

    fn assert_approx_eq(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "expected {expected}, got {actual}"
        );
    }
}
