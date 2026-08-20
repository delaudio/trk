use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use trk_audio::{
    prepare_realtime_sample, AudioConfig, AudioSamplerPlaybackMode, AudioSamplerPlaybackSettings,
};
use trk_core::{SamplePlaybackMode, SamplePlaybackSettings, Song};
use trk_sampler::{PreviewSettings, Sample};

use super::transport::PlaybackUpdate;

pub(super) struct RealtimeSampleLoad {
    samples: Vec<(u32, trk_sampler::PreviewBuffer, f64)>,
    pub(super) complete: bool,
}

impl std::ops::Deref for RealtimeSampleLoad {
    type Target = [(u32, trk_sampler::PreviewBuffer, f64)];

    fn deref(&self) -> &Self::Target {
        &self.samples
    }
}

impl IntoIterator for RealtimeSampleLoad {
    type Item = (u32, trk_sampler::PreviewBuffer, f64);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.samples.into_iter()
    }
}

pub(super) fn load_realtime_samples(
    song: &Song,
    config: AudioConfig,
    update_tx: &Sender<PlaybackUpdate>,
    sample_base_dir: Option<&Path>,
) -> RealtimeSampleLoad {
    let assigned_samples = song
        .sample_assignments
        .iter()
        .map(|assignment| assignment.sample)
        .chain(
            song.track_instrument_assignments
                .iter()
                .filter_map(|assignment| song.instrument_for_id(assignment.instrument))
                .flat_map(|instrument| instrument.sample_ids()),
        )
        .chain(
            song.patterns
                .iter()
                .flat_map(|pattern| &pattern.rows)
                .flat_map(|row| &row.cells)
                .filter_map(|cell| cell.instrument.and_then(|id| song.instrument_for_id(id)))
                .flat_map(|instrument| instrument.sample_ids()),
        )
        .collect::<HashSet<_>>();
    if assigned_samples.is_empty() {
        return RealtimeSampleLoad {
            samples: Vec::new(),
            complete: true,
        };
    }

    let samples = song
        .samples
        .iter()
        .filter(|sample| assigned_samples.contains(&sample.id))
        .filter_map(|reference| {
            let path = resolve_sample_path(&reference.path, sample_base_dir);
            match Sample::load_wav(&path) {
                Ok(sample) => {
                    let preview = sample.preview(PreviewSettings::default());
                    let frame_scale = if preview.sample_rate == 0 {
                        1.0
                    } else {
                        f64::from(config.sample_rate) / f64::from(preview.sample_rate)
                    };
                    Some((
                        reference.id.0,
                        prepare_realtime_sample(&preview, config.sample_rate, config.channels),
                        frame_scale,
                    ))
                }
                Err(error) => {
                    let _ = update_tx.send(PlaybackUpdate::AudioError(format!(
                        "Sample audio load failed for {}: {error}",
                        path.display()
                    )));
                    None
                }
            }
        })
        .collect::<Vec<_>>();
    let loaded_sample_ids = samples
        .iter()
        .map(|(sample_id, _, _)| trk_core::SampleId(*sample_id))
        .collect::<HashSet<_>>();
    RealtimeSampleLoad {
        complete: loaded_sample_ids == assigned_samples,
        samples,
    }
}

pub(crate) fn resolve_sample_path(
    path: impl AsRef<Path>,
    sample_base_dir: Option<&Path>,
) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        return path.to_path_buf();
    }
    sample_base_dir.map_or_else(|| path.to_path_buf(), |base_dir| base_dir.join(path))
}

pub(crate) fn audio_sampler_playback_settings(
    settings: SamplePlaybackSettings,
) -> AudioSamplerPlaybackSettings {
    AudioSamplerPlaybackSettings {
        mode: match settings.mode {
            SamplePlaybackMode::OneShot => AudioSamplerPlaybackMode::OneShot,
            SamplePlaybackMode::Loop | SamplePlaybackMode::ForwardLoop => {
                AudioSamplerPlaybackMode::ForwardLoop
            }
            SamplePlaybackMode::BackwardLoop => AudioSamplerPlaybackMode::BackwardLoop,
            SamplePlaybackMode::PingPongLoop => AudioSamplerPlaybackMode::PingPongLoop,
            SamplePlaybackMode::Reverse => AudioSamplerPlaybackMode::Reverse,
        },
        start_frame: settings.start_frame,
        end_frame: settings.end_frame,
        loop_start_frame: settings.loop_start_frame,
        loop_end_frame: settings.loop_end_frame,
        attack_seconds: settings.envelope.attack_seconds,
        decay_seconds: settings.envelope.decay_seconds,
        sustain: settings.envelope.sustain,
        release_seconds: settings.envelope.release_seconds,
    }
}
