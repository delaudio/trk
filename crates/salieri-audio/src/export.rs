use crate::{errors::AudioExportError, offline_render::RenderedAudio};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioExportFormat {
    WavPcm16,
}

#[must_use]
pub const fn supported_audio_export_formats() -> &'static [AudioExportFormat] {
    &[AudioExportFormat::WavPcm16]
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
