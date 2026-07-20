use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use salieri_audio::{
    apply_preview_envelope, prepare_realtime_sample, slice_preview_buffer, AudioConfig,
};
use salieri_core::{SamplePlaybackSettings, Song};
use salieri_sampler::{PreviewBuffer, PreviewSettings, Sample};

use super::transport::PlaybackUpdate;

pub(super) fn load_realtime_samples(
    song: &Song,
    config: AudioConfig,
    update_tx: &Sender<PlaybackUpdate>,
    sample_base_dir: Option<&Path>,
) -> Vec<(u32, salieri_sampler::PreviewBuffer)> {
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
        return Vec::new();
    }

    song.samples
        .iter()
        .filter(|sample| assigned_samples.contains(&sample.id))
        .filter_map(|reference| {
            let path = resolve_sample_path(&reference.path, sample_base_dir);
            match Sample::load_wav(&path) {
                Ok(sample) => {
                    let preview = apply_sample_playback_settings(
                        &sample.preview(PreviewSettings::default()),
                        reference.playback,
                    );
                    Some((
                        reference.id.0,
                        prepare_realtime_sample(&preview, config.sample_rate, config.channels),
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
        .collect()
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

pub(crate) fn apply_sample_playback_settings(
    preview: &PreviewBuffer,
    settings: SamplePlaybackSettings,
) -> PreviewBuffer {
    let sliced = slice_preview_buffer(preview, settings.start_frame, settings.end_frame);
    let sample_rate = sliced.sample_rate as f32;
    let envelope = settings.envelope;
    apply_preview_envelope(
        &sliced,
        seconds_to_frames(envelope.attack_seconds, sample_rate),
        seconds_to_frames(envelope.decay_seconds, sample_rate),
        envelope.sustain,
        seconds_to_frames(envelope.release_seconds, sample_rate),
    )
}

fn seconds_to_frames(seconds: f32, sample_rate: f32) -> usize {
    if !seconds.is_finite() || seconds <= 0.0 || !sample_rate.is_finite() || sample_rate <= 0.0 {
        0
    } else {
        (seconds * sample_rate).round() as usize
    }
}
