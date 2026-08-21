use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use trk_audio::{
    scale_sampler_playback_frames, AudioBackend, AudioConfig, CalibrationControl, CpalAudioBackend,
    DspDeviceKind as AudioDspDeviceKind, DspDeviceSpec, DspDriveMode as AudioDspDriveMode,
    DspDynamicsDetector as AudioDspDynamicsDetector, DspFilterMode as AudioDspFilterMode,
    DspGraphSpec, RealtimeAudioCommand, SendDspBusSpec, TrackDspChainSpec, TrackSendSpec,
};
use trk_core::{
    DriveMode, DynamicsDetector, EffectDevice, EffectDeviceKind, FilterMode, ParameterLockAction,
    ParameterLockTarget, Pattern, Song, MIXER_SEND_GAIN_PARAMETER_ID,
};

use super::{sample_preload::load_realtime_samples, transport::PlaybackUpdate};

pub(super) enum PlaybackAudioOutput {
    Disabled {
        sample_rate: u32,
    },
    Cpal {
        backend: CpalAudioBackend,
        sample_rate: u32,
        config: AudioConfig,
        sample_base_dir: Option<PathBuf>,
        sample_frame_scales: HashMap<u32, f64>,
        dsp_graphs: Box<PlaybackDspGraphs>,
    },
    #[cfg(test)]
    Recording {
        command_tx: Sender<RealtimeAudioCommand>,
        sample_rate: u32,
    },
}

pub(super) struct PlaybackDspGraphs {
    base: DspGraphSpec,
    last: DspGraphSpec,
}

impl PlaybackAudioOutput {
    pub(super) fn disabled(sample_rate: u32) -> Self {
        Self::Disabled { sample_rate }
    }

    pub(super) fn for_song_with_calibration(
        song: &Song,
        config: AudioConfig,
        update_tx: &Sender<PlaybackUpdate>,
        sample_base_dir: Option<&Path>,
        calibration: CalibrationControl,
    ) -> Self {
        let samples = load_realtime_samples(song, config, update_tx, sample_base_dir);
        if samples.is_empty() {
            return Self::disabled(config.sample_rate);
        }

        let mut backend = CpalAudioBackend::with_calibration(calibration);
        if let Err(error) = AudioBackend::start(&mut backend, config) {
            let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
            return Self::disabled(config.sample_rate);
        }

        let sample_frame_scales = samples
            .iter()
            .map(|(sample_id, _, scale)| (*sample_id, *scale))
            .collect();
        for (sample_id, buffer, _) in samples {
            if let Err(error) = backend.register_sample(sample_id, buffer) {
                let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
            }
        }
        let dsp_graph = audio_dsp_graph(song);
        if let Err(error) = backend.set_dsp_graph(dsp_graph.clone()) {
            let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
        }

        Self::Cpal {
            backend,
            sample_rate: config.sample_rate,
            config,
            sample_base_dir: sample_base_dir.map(Path::to_path_buf),
            sample_frame_scales,
            dsp_graphs: Box::new(PlaybackDspGraphs {
                base: dsp_graph.clone(),
                last: dsp_graph,
            }),
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
            Self::Cpal {
                backend,
                sample_frame_scales,
                ..
            } => {
                let _ = backend.send_realtime_command(scale_sample_command_frames(
                    command,
                    sample_frame_scales,
                ));
            }
            #[cfg(test)]
            Self::Recording { command_tx, .. } => {
                let _ = command_tx.send(command);
            }
        }
    }

    pub(super) fn sync_dsp_graph(&mut self, song: &Song, update_tx: &Sender<PlaybackUpdate>) {
        let Self::Cpal {
            backend,
            dsp_graphs,
            ..
        } = self
        else {
            return;
        };
        let graph = audio_dsp_graph(song);
        dsp_graphs.base = graph.clone();
        set_dsp_graph_if_changed(backend, &mut dsp_graphs.last, graph, update_tx);
    }

    pub(super) fn sync_samples(&mut self, song: &Song, update_tx: &Sender<PlaybackUpdate>) -> bool {
        let Self::Cpal {
            backend,
            config,
            sample_base_dir,
            sample_frame_scales,
            ..
        } = self
        else {
            return true;
        };
        let samples = load_realtime_samples(song, *config, update_tx, sample_base_dir.as_deref());
        if !samples.complete {
            return false;
        }
        if let Err(error) = backend.clear_samples() {
            let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
            return false;
        }
        sample_frame_scales.clear();
        let mut complete = true;
        for (sample_id, buffer, frame_scale) in samples {
            if let Err(error) = backend.register_sample(sample_id, buffer) {
                let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
                complete = false;
            } else {
                sample_frame_scales.insert(sample_id, frame_scale);
            }
        }
        complete
    }

    pub(super) fn sync_dsp_graph_at_row(
        &mut self,
        song: &Song,
        pattern: &Pattern,
        row: usize,
        update_tx: &Sender<PlaybackUpdate>,
    ) {
        let Self::Cpal {
            backend,
            dsp_graphs,
            ..
        } = self
        else {
            return;
        };
        let mut graph = dsp_graphs.base.clone();
        apply_row_locks_to_dsp_graph(&mut graph, song, pattern, row);
        set_dsp_graph_if_changed(backend, &mut dsp_graphs.last, graph, update_tx);
    }
}

fn scale_sample_command_frames(
    command: RealtimeAudioCommand,
    sample_frame_scales: &HashMap<u32, f64>,
) -> RealtimeAudioCommand {
    let RealtimeAudioCommand::TriggerSample {
        track_id,
        sample_id,
        frame,
        gain,
        pan,
        pitch_ratio,
        playback,
    } = command
    else {
        return command;
    };
    let scale = sample_frame_scales.get(&sample_id).copied().unwrap_or(1.0);
    RealtimeAudioCommand::TriggerSample {
        track_id,
        sample_id,
        frame,
        gain,
        pan,
        pitch_ratio,
        playback: scale_sampler_playback_frames(playback, scale),
    }
}

fn set_dsp_graph_if_changed(
    backend: &mut CpalAudioBackend,
    last_dsp_graph: &mut DspGraphSpec,
    graph: DspGraphSpec,
    update_tx: &Sender<PlaybackUpdate>,
) {
    if graph == *last_dsp_graph {
        return;
    }
    // The CPAL backend only enqueues this graph for its audio worker; graph replacement
    // and DSP processing never run on the scheduler thread.
    match backend.set_dsp_graph(graph.clone()) {
        Ok(()) => *last_dsp_graph = graph,
        Err(error) => {
            let _ = update_tx.send(PlaybackUpdate::AudioError(error.to_string()));
        }
    }
}

fn apply_row_locks_to_dsp_graph(
    graph: &mut DspGraphSpec,
    song: &Song,
    pattern: &Pattern,
    row: usize,
) {
    let Some(row) = pattern.rows.get(row) else {
        return;
    };
    let locks = row
        .cells
        .iter()
        .flat_map(|cell| cell.parameter_locks.iter())
        .collect::<Vec<_>>();

    for mixer in &song.mixer.tracks {
        for (device_index, source) in mixer.effects.iter().enumerate() {
            let mut resolved = source.clone();
            let mut changed = false;
            for lock in &locks {
                if lock.target
                    != (ParameterLockTarget::TrackEffect {
                        track: mixer.track,
                        device: source.id,
                    })
                {
                    continue;
                }
                let ParameterLockAction::Set { value } = &lock.action else {
                    continue;
                };
                changed |= resolved
                    .set_parameter_value(&lock.parameter, value.clone())
                    .is_ok();
            }
            if changed {
                if let Some(device) = graph
                    .track_chains
                    .iter_mut()
                    .find(|chain| chain.track_id == mixer.track.0)
                    .and_then(|chain| chain.devices.get_mut(device_index))
                {
                    *device = audio_dsp_device(&resolved);
                }
            }
        }
    }

    for (device_index, source) in song.mixer.master_effects.iter().enumerate() {
        let mut resolved = source.clone();
        let mut changed = false;
        for lock in &locks {
            if lock.target != (ParameterLockTarget::MasterEffect { device: source.id }) {
                continue;
            }
            let ParameterLockAction::Set { value } = &lock.action else {
                continue;
            };
            changed |= resolved
                .set_parameter_value(&lock.parameter, value.clone())
                .is_ok();
        }
        if changed {
            if let Some(device) = graph.master.get_mut(device_index) {
                *device = audio_dsp_device(&resolved);
            }
        }
    }

    for send in &mut graph.track_sends {
        if let Some(value) = locks.iter().rev().find_map(|lock| {
            (lock.target
                == (ParameterLockTarget::TrackSend {
                    track: trk_core::TrackId(send.track_id),
                    send: send.send_id,
                })
                && lock.parameter.as_str() == MIXER_SEND_GAIN_PARAMETER_ID)
                .then_some(&lock.action)
                .and_then(|action| match action {
                    ParameterLockAction::Set { value } => value.as_f32(),
                    ParameterLockAction::Reset => None,
                })
        }) {
            send.gain = value;
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
        sends: song
            .mixer
            .sends
            .iter()
            .map(|send| SendDspBusSpec {
                send_id: send.id,
                pre_fader: send.pre_fader,
                devices: send.effects.iter().map(audio_dsp_device).collect(),
            })
            .collect(),
        track_sends: song
            .mixer
            .tracks
            .iter()
            .flat_map(|track| {
                track.sends.iter().map(move |send| TrackSendSpec {
                    track_id: track.track.0,
                    send_id: send.send,
                    gain: send.gain,
                })
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
            EffectDeviceKind::Chorus {
                rate_hz,
                sync,
                depth,
                delay_ms,
                voices,
                spread,
                feedback,
                mix,
                output_db,
            } => AudioDspDeviceKind::Chorus {
                rate_hz,
                sync,
                depth,
                delay_ms,
                voices,
                spread,
                feedback,
                mix,
                output_db,
            },
            EffectDeviceKind::Flanger {
                rate_hz,
                sync,
                depth,
                manual,
                delay_ms,
                feedback,
                stereo_phase,
                mix,
                output_db,
            } => AudioDspDeviceKind::Flanger {
                rate_hz,
                sync,
                depth,
                manual,
                delay_ms,
                feedback,
                stereo_phase,
                mix,
                output_db,
            },
            EffectDeviceKind::Phaser {
                rate_hz,
                sync,
                depth,
                center_hz,
                stages,
                feedback,
                stereo_phase,
                mix,
                output_db,
            } => AudioDspDeviceKind::Phaser {
                rate_hz,
                sync,
                depth,
                center_hz,
                stages,
                feedback,
                stereo_phase,
                mix,
                output_db,
            },
            EffectDeviceKind::Compressor {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                knee_db,
                makeup_db,
                auto_makeup,
                detector,
                stereo_link,
                mix,
            } => AudioDspDeviceKind::Compressor {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                knee_db,
                makeup_db,
                auto_makeup,
                detector: audio_dynamics_detector(detector),
                stereo_link,
                mix,
            },
            EffectDeviceKind::Gate {
                threshold_db,
                hysteresis_db,
                attack_ms,
                hold_ms,
                release_ms,
                range_db,
                detector,
                stereo_link,
            } => AudioDspDeviceKind::Gate {
                threshold_db,
                hysteresis_db,
                attack_ms,
                hold_ms,
                release_ms,
                range_db,
                detector: audio_dynamics_detector(detector),
                stereo_link,
            },
            EffectDeviceKind::Limiter {
                ceiling_db,
                input_gain_db,
                release_ms,
                lookahead_ms,
                stereo_link,
                true_peak,
            } => AudioDspDeviceKind::Limiter {
                ceiling_db,
                input_gain_db,
                release_ms,
                lookahead_ms,
                stereo_link,
                true_peak,
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

fn audio_dynamics_detector(detector: DynamicsDetector) -> AudioDspDynamicsDetector {
    match detector {
        DynamicsDetector::Peak => AudioDspDynamicsDetector::Peak,
        DynamicsDetector::Rms => AudioDspDynamicsDetector::Rms,
    }
}

#[cfg(test)]
#[path = "audio_dispatch/tests.rs"]
mod tests;

pub(super) fn send_all_audio_notes_off(audio_output: &mut PlaybackAudioOutput) {
    send_audio_command(audio_output, RealtimeAudioCommand::AllNotesOff { frame: 0 })
}

pub(super) fn send_audio_command(
    audio_output: &mut PlaybackAudioOutput,
    command: RealtimeAudioCommand,
) {
    audio_output.send(command);
}
