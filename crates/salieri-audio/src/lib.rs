use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use salieri_core::Song;
use salieri_sampler::PreviewBuffer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_frames: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderChainPlan {
    pub schema_version: u32,
    pub source: RenderSource,
    pub format: RenderFormat,
    pub tracks: Vec<RenderTrackPlan>,
    pub master: RenderMasterPlan,
    pub targets: Vec<RenderTarget>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSource {
    pub project_path: Option<String>,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderTrackPlan {
    pub track_id: u32,
    pub name: String,
    pub midi_channel: u8,
    pub source_type: RenderSourceType,
    pub instrument: Option<RenderInstrument>,
    pub effects: Vec<RenderEffect>,
    pub mix: RenderMixDefaults,
    pub output_stem_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderSourceType {
    TrackerMidi,
    ExternalStem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderInstrument {
    pub kind: String,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderEffect {
    pub kind: String,
    pub bypassed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderMixDefaults {
    pub gain_db: f32,
    pub pan: f32,
    pub muted: bool,
    pub solo: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderMasterPlan {
    pub effects: Vec<RenderEffect>,
    pub mix: RenderMixDefaults,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderTarget {
    pub kind: String,
    pub path: String,
}

#[must_use]
pub fn render_chain_from_song(
    song: &Song,
    project_path: Option<&str>,
    sample_rate: u32,
    channels: u16,
    bit_depth: u16,
) -> RenderChainPlan {
    let tracks = song
        .tracks
        .iter()
        .map(|track| {
            let source_type = if track.stem.is_some() {
                RenderSourceType::ExternalStem
            } else {
                RenderSourceType::TrackerMidi
            };
            RenderTrackPlan {
                track_id: track.id.0,
                name: track.name.clone(),
                midi_channel: track.midi_channel,
                source_type,
                instrument: Some(RenderInstrument {
                    kind: "external-or-future-engine".to_string(),
                    reference: track.stem.as_ref().map(|stem| stem.entry_id.clone()),
                }),
                effects: Vec::new(),
                mix: RenderMixDefaults {
                    gain_db: 0.0,
                    pan: 0.0,
                    muted: track.muted,
                    solo: track.solo,
                },
                output_stem_path: Some(format!(
                    "stems/{:02}-{}.wav",
                    track.id.0,
                    slug(&track.name)
                )),
            }
        })
        .collect::<Vec<_>>();

    RenderChainPlan {
        schema_version: 1,
        source: RenderSource {
            project_path: project_path.map(ToOwned::to_owned),
            title: song.metadata.title.clone(),
        },
        format: RenderFormat {
            sample_rate,
            channels,
            bit_depth,
        },
        tracks,
        master: RenderMasterPlan {
            effects: Vec::new(),
            mix: RenderMixDefaults {
                gain_db: 0.0,
                pan: 0.0,
                muted: false,
                solo: false,
            },
        },
        targets: vec![RenderTarget {
            kind: "stereo-mix".to_string(),
            path: "mix/master.wav".to_string(),
        }],
    }
}

fn slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    slug.trim_matches('-').to_string()
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

#[derive(Debug, thiserror::Error)]
pub enum AudioExportError {
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

    #[test]
    fn render_chain_plan_represents_tracker_and_stem_sources() {
        let mut song = Song::empty();
        song.tracks[1].stem = Some(salieri_core::StemTrackReference {
            entry_id: "stem_001_bass".to_string(),
        });
        song.tracks[1].muted = true;

        let plan = render_chain_from_song(&song, Some("song.salieri"), 48_000, 2, 24);

        assert_eq!(plan.schema_version, 1);
        assert_eq!(plan.source.project_path.as_deref(), Some("song.salieri"));
        assert_eq!(plan.format.sample_rate, 48_000);
        assert_eq!(plan.tracks[0].source_type, RenderSourceType::TrackerMidi);
        assert_eq!(plan.tracks[1].source_type, RenderSourceType::ExternalStem);
        assert_eq!(
            plan.tracks[1]
                .instrument
                .as_ref()
                .and_then(|instrument| instrument.reference.as_deref()),
            Some("stem_001_bass")
        );
        assert!(plan.tracks[1].mix.muted);
        assert_eq!(plan.targets[0].kind, "stereo-mix");
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
}
