use std::{path::Path, sync::mpsc::Sender};

use salieri_audio::{
    AudioBackend, AudioConfig, CpalAudioBackend, DspDeviceKind as AudioDspDeviceKind,
    DspDeviceSpec, DspDriveMode as AudioDspDriveMode, DspFilterMode as AudioDspFilterMode,
    DspGraphSpec, RealtimeAudioCommand, TrackDspChainSpec,
};
use salieri_core::{DriveMode, EffectDevice, EffectDeviceKind, FilterMode, Song};

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
            EffectDeviceKind::Balance { balance } => AudioDspDeviceKind::Balance { balance },
            EffectDeviceKind::StereoWidth { width } => AudioDspDeviceKind::StereoWidth { width },
            EffectDeviceKind::PhaseInvert {
                invert_left,
                invert_right,
            } => AudioDspDeviceKind::PhaseInvert {
                invert_left,
                invert_right,
            },
            EffectDeviceKind::Filter {
                mode,
                cutoff_hz,
                resonance,
                drive_db,
                key_track,
                env_amount,
                mix,
            } => AudioDspDeviceKind::Filter {
                mode: audio_filter_mode(mode),
                cutoff_hz,
                resonance,
                drive_db,
                key_track,
                env_amount,
                mix,
            },
            EffectDeviceKind::Delay {
                sync,
                time_left_ms,
                time_right_ms,
                link_times,
                feedback,
                ping_pong,
                filter_low_cut_hz,
                filter_high_cut_hz,
                mod_rate_hz,
                mod_depth,
                mix,
                output_db,
            } => AudioDspDeviceKind::Delay {
                sync,
                time_left_ms,
                time_right_ms,
                link_times,
                feedback,
                ping_pong,
                filter_low_cut_hz,
                filter_high_cut_hz,
                mod_rate_hz,
                mod_depth,
                mix,
                output_db,
            },
            EffectDeviceKind::Reverb {
                size,
                predelay_ms,
                decay_s,
                damping,
                low_cut_hz,
                high_cut_hz,
                diffusion,
                width,
                early_reflections,
                mix,
                output_db,
            } => AudioDspDeviceKind::Reverb {
                size,
                predelay_ms,
                decay_s,
                damping,
                low_cut_hz,
                high_cut_hz,
                diffusion,
                width,
                early_reflections,
                mix,
                output_db,
            },
            EffectDeviceKind::Drive {
                mode,
                drive_db,
                tone,
                bias,
                mix,
                output_db,
            } => AudioDspDeviceKind::Drive {
                mode: audio_drive_mode(mode),
                drive_db,
                tone,
                bias,
                mix,
                output_db,
            },
            EffectDeviceKind::Bitcrusher {
                bit_depth,
                reduction_ratio,
                dither,
                mix,
                output_db,
            } => AudioDspDeviceKind::Bitcrusher {
                bit_depth,
                reduction_ratio,
                dither,
                mix,
                output_db,
            },
        },
    }
}

fn audio_filter_mode(mode: FilterMode) -> AudioDspFilterMode {
    match mode {
        FilterMode::LowPass => AudioDspFilterMode::LowPass,
        FilterMode::HighPass => AudioDspFilterMode::HighPass,
        FilterMode::BandPass => AudioDspFilterMode::BandPass,
        FilterMode::Notch => AudioDspFilterMode::Notch,
    }
}

fn audio_drive_mode(mode: DriveMode) -> AudioDspDriveMode {
    match mode {
        DriveMode::Overdrive => AudioDspDriveMode::Overdrive,
        DriveMode::Saturation => AudioDspDriveMode::Saturation,
        DriveMode::HardClip => AudioDspDriveMode::HardClip,
        DriveMode::SoftClip => AudioDspDriveMode::SoftClip,
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
