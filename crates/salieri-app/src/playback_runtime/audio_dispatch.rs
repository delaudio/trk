use std::{path::Path, sync::mpsc::Sender};

use salieri_audio::{
    AudioBackend, AudioConfig, CpalAudioBackend, DspDeviceKind as AudioDspDeviceKind,
    DspDeviceSpec, DspGraphSpec, RealtimeAudioCommand, TrackDspChainSpec,
};
use salieri_core::{EffectDevice, EffectDeviceKind, Song};

use super::{sample_preload::load_realtime_samples, transport::PlaybackUpdate};

pub(super) enum PlaybackAudioOutput {
    Disabled {
        sample_rate: u32,
    },
    Cpal {
        backend: CpalAudioBackend,
        sample_rate: u32,
    },
    #[cfg(test)]
    Recording {
        command_tx: Sender<RealtimeAudioCommand>,
        sample_rate: u32,
    },
}

impl PlaybackAudioOutput {
    pub(super) fn disabled(sample_rate: u32) -> Self {
        Self::Disabled { sample_rate }
    }

    pub(super) fn for_song(
        song: &Song,
        config: AudioConfig,
        update_tx: &Sender<PlaybackUpdate>,
        sample_base_dir: Option<&Path>,
    ) -> Self {
        let samples = load_realtime_samples(song, config, update_tx, sample_base_dir);
        if samples.is_empty() {
            return Self::disabled(config.sample_rate);
        }

        let mut backend = CpalAudioBackend::new();
        if let Err(error) = AudioBackend::start(&mut backend, config) {
            let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
            return Self::disabled(config.sample_rate);
        }

        for (sample_id, buffer) in samples {
            if let Err(error) = backend.register_sample(sample_id, buffer) {
                let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
            }
        }
        if let Err(error) = backend.set_dsp_graph(audio_dsp_graph(song)) {
            let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
        }

        Self::Cpal {
            backend,
            sample_rate: config.sample_rate,
        }
    }

    #[cfg(test)]
    pub(super) fn recording(command_tx: Sender<RealtimeAudioCommand>, sample_rate: u32) -> Self {
        Self::Recording {
            command_tx,
            sample_rate,
        }
    }

    pub(super) fn sample_rate(&self) -> u32 {
        match self {
            Self::Disabled { sample_rate } | Self::Cpal { sample_rate, .. } => *sample_rate,
            #[cfg(test)]
            Self::Recording { sample_rate, .. } => *sample_rate,
        }
    }

    pub(super) fn send(&mut self, command: RealtimeAudioCommand) {
        match self {
            Self::Disabled { .. } => {}
            Self::Cpal { backend, .. } => {
                let _ = backend.send_realtime_command(command);
            }
            #[cfg(test)]
            Self::Recording { command_tx, .. } => {
                let _ = command_tx.send(command);
            }
        }
    }
}

fn audio_dsp_graph(song: &Song) -> DspGraphSpec {
    DspGraphSpec {
        track_chains: song
            .mixer
            .tracks
            .iter()
            .filter(|track| !track.effects.is_empty())
            .map(|track| TrackDspChainSpec {
                track_id: track.track.0,
                devices: track.effects.iter().map(audio_dsp_device).collect(),
            })
            .collect(),
        master: song
            .mixer
            .master_effects
            .iter()
            .map(audio_dsp_device)
            .collect(),
    }
}

fn audio_dsp_device(device: &EffectDevice) -> DspDeviceSpec {
    DspDeviceSpec {
        bypassed: device.bypassed,
        kind: match device.kind {
            EffectDeviceKind::Gain { gain } => AudioDspDeviceKind::Gain { gain },
            EffectDeviceKind::Pan { pan } => AudioDspDeviceKind::Pan { pan },
        },
    }
}

pub(super) fn send_all_audio_notes_off(audio_output: &mut PlaybackAudioOutput) {
    send_audio_command(audio_output, RealtimeAudioCommand::AllNotesOff { frame: 0 })
}

pub(super) fn send_audio_command(
    audio_output: &mut PlaybackAudioOutput,
    command: RealtimeAudioCommand,
) {
    audio_output.send(command);
}
