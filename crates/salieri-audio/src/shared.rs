use salieri_sampler::PreviewBuffer;

use crate::{errors::AudioExportError, offline_render::OfflineRenderSpec};

pub(crate) fn validate_sampler_render_sample(
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

pub(crate) fn validated_pitch_ratio(pitch_ratio: f32) -> Result<f32, AudioExportError> {
    if pitch_ratio.is_finite() && pitch_ratio > 0.0 {
        Ok(pitch_ratio)
    } else {
        Err(AudioExportError::InvalidPitchRatio { pitch_ratio })
    }
}

pub(crate) fn interpolated_sample(
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

pub(crate) fn converted_channel_sample(
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
