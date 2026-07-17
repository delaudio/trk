use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use salieri_sampler::PreviewBuffer;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
};

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
    #[error("audio backend command failed: {0}")]
    Command(String),
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

#[derive(Default)]
pub struct CpalAudioBackend {
    worker: Option<CpalStreamWorker>,
}

impl CpalAudioBackend {
    #[must_use]
    pub const fn new() -> Self {
        Self { worker: None }
    }

    #[must_use]
    pub fn is_started(&self) -> bool {
        self.worker.is_some()
    }

    pub fn register_sample(&self, sample_id: u32, buffer: PreviewBuffer) -> Result<(), AudioError> {
        self.send_stream_command(CpalStreamCommand::RegisterSample { sample_id, buffer })
    }

    pub fn send_realtime_command(&self, command: RealtimeAudioCommand) -> Result<(), AudioError> {
        self.send_stream_command(CpalStreamCommand::Realtime(command))
    }

    pub fn clear_samples(&self) -> Result<(), AudioError> {
        self.send_stream_command(CpalStreamCommand::ClearSamples)
    }

    fn send_stream_command(&self, command: CpalStreamCommand) -> Result<(), AudioError> {
        let Some(worker) = &self.worker else {
            return Err(AudioError::Command(
                "CPAL stream is not started".to_string(),
            ));
        };
        worker
            .command_tx
            .send(command)
            .map_err(|error| AudioError::Command(format!("CPAL stream command failed: {error}")))
    }
}

impl AudioBackend for CpalAudioBackend {
    fn start(&mut self, config: AudioConfig) -> Result<(), AudioError> {
        if self.worker.is_some() {
            return Ok(());
        }

        let (command_tx, command_rx) = mpsc::channel();
        let (startup_tx, startup_rx) = mpsc::channel();
        let handle = thread::spawn(move || cpal_stream_thread(config, command_rx, startup_tx));

        match startup_rx.recv() {
            Ok(Ok(())) => {
                self.worker = Some(CpalStreamWorker { command_tx, handle });
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = handle.join();
                Err(error)
            }
            Err(error) => {
                let _ = handle.join();
                Err(AudioError::Start(format!(
                    "cpal stream thread failed before startup: {error}"
                )))
            }
        }
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        if let Some(worker) = self.worker.take() {
            let _ = worker.command_tx.send(CpalStreamCommand::Stop);
            worker
                .handle
                .join()
                .map_err(|_| AudioError::Stop("cpal stream thread panicked".to_string()))?;
        }
        Ok(())
    }
}

impl Drop for CpalAudioBackend {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

struct CpalStreamWorker {
    command_tx: Sender<CpalStreamCommand>,
    handle: JoinHandle<()>,
}

enum CpalStreamCommand {
    RegisterSample {
        sample_id: u32,
        buffer: PreviewBuffer,
    },
    ClearSamples,
    Realtime(RealtimeAudioCommand),
    Stop,
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

fn cpal_stream_thread(
    config: AudioConfig,
    command_rx: Receiver<CpalStreamCommand>,
    startup_tx: Sender<Result<(), AudioError>>,
) {
    let (realtime_tx, realtime_rx) = mpsc::channel();
    match start_realtime_cpal_stream(config, realtime_rx) {
        Ok(stream) => {
            let _ = startup_tx.send(Ok(()));
            while let Ok(command) = command_rx.recv() {
                if matches!(command, CpalStreamCommand::Stop) {
                    break;
                }
                let _ = realtime_tx.send(command);
            }
            let _ = stream.pause();
        }
        Err(error) => {
            let _ = startup_tx.send(Err(error));
        }
    }
}

fn start_realtime_cpal_stream(
    config: AudioConfig,
    command_rx: Receiver<CpalStreamCommand>,
) -> Result<Stream, AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| AudioError::Start("no default output device".to_string()))?;
    let default_config = device
        .default_output_config()
        .map_err(|error| AudioError::Start(format!("default output config failed: {error}")))?;
    let sample_format = default_config.sample_format();
    let stream_config = StreamConfig {
        channels: config.channels,
        sample_rate: cpal::SampleRate(config.sample_rate),
        buffer_size: cpal::BufferSize::Fixed(u32::from(config.buffer_frames)),
    };

    let stream = build_realtime_output_stream(&device, &stream_config, sample_format, command_rx)?;
    stream
        .play()
        .map_err(|error| AudioError::Start(format!("failed to play stream: {error}")))?;
    Ok(stream)
}

fn build_realtime_output_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    command_rx: Receiver<CpalStreamCommand>,
) -> Result<Stream, AudioError> {
    match sample_format {
        SampleFormat::F32 => build_realtime_output_stream_for::<f32>(device, config, command_rx),
        SampleFormat::I16 => build_realtime_output_stream_for::<i16>(device, config, command_rx),
        SampleFormat::U16 => build_realtime_output_stream_for::<u16>(device, config, command_rx),
        sample_format => Err(AudioError::Start(format!(
            "unsupported output sample format {sample_format}"
        ))),
    }
}

fn build_realtime_output_stream_for<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    command_rx: Receiver<CpalStreamCommand>,
) -> Result<Stream, AudioError>
where
    T: Sample + SizedSample + FromSample<f32> + 'static,
{
    let sampler_config = RealtimeSamplerConfig {
        sample_rate: config.sample_rate.0,
        channels: config.channels,
        max_voices: RealtimeSamplerConfig::default().max_voices,
    };
    let mut sampler = RealtimeSampler::new(sampler_config);
    let mut scratch = vec![0.0; usize::from(config.channels) * config.buffer_size_frame_hint()];

    device
        .build_output_stream(
            config,
            move |output, _| {
                write_realtime_output::<T>(output, &mut scratch, &mut sampler, &command_rx);
            },
            |error| {
                let _ = error;
            },
            None,
        )
        .map_err(|error| AudioError::Start(format!("failed to build output stream: {error}")))
}

trait StreamConfigFrameHint {
    fn buffer_size_frame_hint(&self) -> usize;
}

impl StreamConfigFrameHint for StreamConfig {
    fn buffer_size_frame_hint(&self) -> usize {
        match self.buffer_size {
            cpal::BufferSize::Fixed(frames) => frames as usize,
            cpal::BufferSize::Default => 512,
        }
    }
}

fn write_realtime_output<T>(
    output: &mut [T],
    scratch: &mut Vec<f32>,
    sampler: &mut RealtimeSampler,
    command_rx: &Receiver<CpalStreamCommand>,
) where
    T: Sample + FromSample<f32>,
{
    while let Ok(command) = command_rx.try_recv() {
        match command {
            CpalStreamCommand::RegisterSample { sample_id, buffer } => {
                let _ = sampler.register_sample(sample_id, buffer);
            }
            CpalStreamCommand::ClearSamples => {
                sampler.clear_samples();
            }
            CpalStreamCommand::Realtime(command) => {
                let _ = sampler.handle_command_now(command);
            }
            CpalStreamCommand::Stop => {}
        }
    }

    if scratch.len() != output.len() {
        scratch.resize(output.len(), 0.0);
    }
    sampler.render_into(scratch);

    for (output, sample) in output.iter_mut().zip(scratch.iter()) {
        *output = T::from_sample((*sample).clamp(-1.0, 1.0));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RealtimeAudioCommand {
    TriggerSample {
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelMeter {
    pub peak: f32,
    pub rms: f32,
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
    pub pan: f32,
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
                sample_id,
                gain,
                pan,
                pitch_ratio,
                ..
            } => RealtimeAudioCommand::TriggerSample {
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
            mix_realtime_voice(data, channels, sample, voice, render_start, render_end);
        }

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
pub const fn supported_audio_export_formats() -> &'static [AudioExportFormat] {
    &[AudioExportFormat::WavPcm16]
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

#[must_use]
pub fn measure_levels(audio: &RenderedAudio) -> Vec<LevelMeter> {
    let channels = usize::from(audio.channels).max(1);
    let mut peaks = vec![0.0_f32; channels];
    let mut sums = vec![0.0_f32; channels];
    for frame in 0..audio.frames {
        let offset = frame.saturating_mul(channels);
        for channel in 0..channels {
            let value = audio
                .data
                .get(offset + channel)
                .copied()
                .unwrap_or_default();
            let abs = value.abs();
            peaks[channel] = peaks[channel].max(abs);
            sums[channel] += value * value;
        }
    }
    let frames = audio.frames.max(1) as f32;
    peaks
        .into_iter()
        .zip(sums)
        .map(|(peak, sum)| LevelMeter {
            peak,
            rms: (sum / frames).sqrt(),
        })
        .collect()
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
        let pan = event.pan.clamp(-1.0, 1.0);
        mix_sample_event(
            &mut data,
            frames,
            channels,
            sample,
            output_frame,
            MixParams {
                pitch_ratio,
                level,
                pan,
            },
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

#[derive(Debug, Clone, Copy)]
struct MixParams {
    pitch_ratio: f32,
    level: f32,
    pan: f32,
}

fn mix_realtime_voice(
    output: &mut [f32],
    channels: usize,
    sample: &PreviewBuffer,
    voice: &RealtimeSamplerVoice,
    render_start: u64,
    render_end: u64,
) {
    let voice_end = voice_end_frame(voice, sample).unwrap_or(u64::MAX);
    let mix_start = render_start.max(voice.start_frame);
    let mix_end = render_end.min(voice_end);
    if mix_start >= mix_end {
        return;
    }

    for absolute_frame in mix_start..mix_end {
        let output_frame = (absolute_frame - render_start) as usize;
        let source_frame = (absolute_frame - voice.start_frame) as f32 * voice.pitch_ratio;
        let output_offset = output_frame * channels;
        for channel in 0..channels {
            output[output_offset + channel] +=
                interpolated_sample(sample, source_frame, channel, channels)
                    * voice.gain
                    * pan_gain(voice.pan, channel, channels);
        }
    }
}

fn pan_gain(pan: f32, channel: usize, channels: usize) -> f32 {
    if channels < 2 {
        return 1.0;
    }
    match channel {
        0 if pan > 0.0 => 1.0 - pan,
        1 if pan < 0.0 => 1.0 + pan,
        _ => 1.0,
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

fn interpolated_sample(
    sample: &PreviewBuffer,
    source_frame: f32,
    channel: usize,
    _channels: usize,
) -> f32 {
    let channels = usize::from(sample.channels).max(1);
    let base_frame = source_frame.floor() as usize;
    let next_frame = (base_frame + 1).min(sample.frames.saturating_sub(1));
    let fractional = source_frame - base_frame as f32;
    let channel = channel.min(channels.saturating_sub(1));
    let current = sample.data[base_frame * channels + channel];
    let next = sample.data[next_frame * channels + channel];
    current + ((next - current) * fractional)
}

fn converted_channel_sample(
    sample: &PreviewBuffer,
    source_frame: f32,
    target_channel: usize,
    target_channels: usize,
) -> f32 {
    let source_channels = usize::from(sample.channels).max(1);
    if source_channels == 1 {
        return interpolated_sample(sample, source_frame, 0, source_channels);
    }
    if target_channels == 1 {
        return downmixed_sample(sample, source_frame, source_channels);
    }
    if target_channel < source_channels {
        return interpolated_sample(sample, source_frame, target_channel, source_channels);
    }

    downmixed_sample(sample, source_frame, source_channels)
}

fn downmixed_sample(sample: &PreviewBuffer, source_frame: f32, source_channels: usize) -> f32 {
    let sum = (0..source_channels)
        .map(|channel| interpolated_sample(sample, source_frame, channel, source_channels))
        .sum::<f32>();
    sum / source_channels as f32
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
    fn cpal_backend_starts_unopened() {
        let backend = CpalAudioBackend::new();

        assert!(!backend.is_started());
    }

    #[test]
    fn realtime_commands_are_plain_data_messages() {
        let command = RealtimeAudioCommand::TriggerSample {
            sample_id: 1,
            frame: 128,
            gain: 0.5,
            pan: 0.0,
            pitch_ratio: 2.0,
        };

        assert_eq!(
            command,
            RealtimeAudioCommand::TriggerSample {
                sample_id: 1,
                frame: 128,
                gain: 0.5,
                pan: 0.0,
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
    fn prepares_realtime_samples_for_output_config() {
        let preview = PreviewBuffer {
            sample_rate: 2,
            channels: 1,
            frames: 2,
            data: vec![0.25, 0.75],
        };

        let prepared = prepare_realtime_sample(&preview, 4, 2);

        assert_eq!(prepared.sample_rate, 4);
        assert_eq!(prepared.channels, 2);
        assert_eq!(prepared.frames, 4);
        assert_eq!(prepared.data[0], 0.25);
        assert_eq!(prepared.data[1], 0.25);
        assert_approx_eq(prepared.data[2], 0.5);
        assert_approx_eq(prepared.data[3], 0.5);
    }

    #[test]
    fn slices_preview_buffers_by_frame_window() {
        let preview = PreviewBuffer {
            sample_rate: 48_000,
            channels: 2,
            frames: 4,
            data: vec![0.0, 0.1, 1.0, 1.1, 2.0, 2.1, 3.0, 3.1],
        };

        let sliced = slice_preview_buffer(&preview, Some(1), Some(3));

        assert_eq!(sliced.sample_rate, 48_000);
        assert_eq!(sliced.channels, 2);
        assert_eq!(sliced.frames, 2);
        assert_eq!(sliced.data, vec![1.0, 1.1, 2.0, 2.1]);
    }

    #[test]
    fn applies_preview_envelope_to_each_frame() {
        let preview = PreviewBuffer {
            sample_rate: 4,
            channels: 1,
            frames: 4,
            data: vec![1.0, 1.0, 1.0, 1.0],
        };

        let enveloped = apply_preview_envelope(&preview, 2, 0, 1.0, 2);

        assert_approx_eq(enveloped.data[0], 0.0);
        assert_approx_eq(enveloped.data[1], 0.5);
        assert_approx_eq(enveloped.data[2], 1.0);
        assert_approx_eq(enveloped.data[3], 0.5);
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
            pan: 0.0,
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
    fn renders_sampler_events_with_linear_stereo_pan() {
        let samples = vec![OfflineSamplerSample {
            sample_id: 7,
            buffer: PreviewBuffer {
                sample_rate: 48_000,
                channels: 2,
                frames: 1,
                data: vec![1.0, 1.0],
            },
        }];
        let events = vec![OfflineSamplerEvent {
            sample_id: 7,
            frame: 0,
            gain: 1.0,
            pan: 0.75,
            pitch_ratio: 1.0,
            velocity: 127,
        }];

        let rendered = render_sampler_events(
            &samples,
            &events,
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 2,
                frames: 1,
            },
        )
        .expect("render");

        assert_approx_eq(rendered.data[0], 0.25);
        assert_approx_eq(rendered.data[1], 1.0);
    }

    #[test]
    fn measures_rendered_audio_levels() {
        let audio = RenderedAudio {
            sample_rate: 48_000,
            channels: 2,
            frames: 2,
            data: vec![1.0, 0.5, -1.0, 0.0],
        };

        let levels = measure_levels(&audio);

        assert_eq!(levels.len(), 2);
        assert_approx_eq(levels[0].peak, 1.0);
        assert_approx_eq(levels[0].rms, 1.0);
        assert_approx_eq(levels[1].peak, 0.5);
        assert_approx_eq(levels[1].rms, (0.125_f32).sqrt());
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
            pan: 0.0,
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
                    pan: 0.0,
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
                    pan: 0.0,
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
                pan: 0.0,
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
    fn realtime_sampler_renders_triggered_voices() {
        let mut sampler = RealtimeSampler::new(RealtimeSamplerConfig {
            sample_rate: 48_000,
            channels: 1,
            max_voices: 4,
        });
        sampler
            .register_sample(1, mono_sample(vec![0.25, 0.5, 0.75, 1.0]))
            .expect("register sample");

        sampler
            .handle_command(RealtimeAudioCommand::TriggerSample {
                sample_id: 1,
                frame: 1,
                gain: 0.5,
                pan: 0.0,
                pitch_ratio: 2.0,
            })
            .expect("trigger");

        let rendered = sampler.render(4);

        assert_eq!(rendered.data[0], 0.0);
        assert_eq!(rendered.data[1], 0.125);
        assert_eq!(rendered.data[2], 0.375);
        assert_eq!(rendered.data[3], 0.0);
        assert_eq!(sampler.active_voice_count(), 0);
    }

    #[test]
    fn realtime_sampler_can_trigger_at_current_callback_frame() {
        let mut sampler = RealtimeSampler::new(RealtimeSamplerConfig {
            sample_rate: 48_000,
            channels: 1,
            max_voices: 4,
        });
        sampler
            .register_sample(1, mono_sample(vec![0.25, 0.5]))
            .expect("register sample");
        let preroll = sampler.render(8);
        assert_eq!(preroll.data, vec![0.0; 8]);

        sampler
            .handle_command_now(RealtimeAudioCommand::TriggerSample {
                sample_id: 1,
                frame: 0,
                gain: 1.0,
                pan: 0.0,
                pitch_ratio: 1.0,
            })
            .expect("trigger now");

        let rendered = sampler.render(2);
        assert_eq!(rendered.data, vec![0.25, 0.5]);
    }

    #[test]
    fn realtime_sampler_bounds_and_clears_voices() {
        let mut sampler = RealtimeSampler::new(RealtimeSamplerConfig {
            sample_rate: 48_000,
            channels: 1,
            max_voices: 1,
        });
        sampler
            .register_sample(1, mono_sample(vec![1.0, 1.0]))
            .expect("register sample");
        let first_voice = sampler
            .handle_command(RealtimeAudioCommand::TriggerSample {
                sample_id: 1,
                frame: 0,
                gain: 1.0,
                pan: 0.0,
                pitch_ratio: 1.0,
            })
            .expect("trigger first")
            .expect("first voice id");
        let second_voice = sampler
            .handle_command(RealtimeAudioCommand::TriggerSample {
                sample_id: 1,
                frame: 0,
                gain: 1.0,
                pan: 0.0,
                pitch_ratio: 1.0,
            })
            .expect("trigger second")
            .expect("second voice id");

        assert_ne!(first_voice, second_voice);
        assert_eq!(sampler.active_voice_count(), 1);

        sampler
            .handle_command(RealtimeAudioCommand::StopVoice {
                voice_id: second_voice,
                frame: 0,
            })
            .expect("stop voice");
        assert_eq!(sampler.active_voice_count(), 0);

        sampler
            .handle_command(RealtimeAudioCommand::TriggerSample {
                sample_id: 1,
                frame: 0,
                gain: 1.0,
                pan: 0.0,
                pitch_ratio: 1.0,
            })
            .expect("trigger third");
        sampler
            .handle_command(RealtimeAudioCommand::AllNotesOff { frame: 0 })
            .expect("all notes off");
        assert_eq!(sampler.active_voice_count(), 0);
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
