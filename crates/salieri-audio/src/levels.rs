use crate::offline_render::RenderedAudio;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelMeter {
    pub peak: f32,
    pub rms: f32,
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
